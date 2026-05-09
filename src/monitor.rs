//! Monitor mode: smooth real-time refresh using crossterm alternate screen.
//! Runs until Ctrl-C is pressed. Refreshes every `interval_secs` seconds.
//! All side-effects (Growl, audio) happen via pre-spawned thread handles.

use crate::{
    audio::AudioHandle,
    battery_info::{self, BatterySnapshot},
    config::Config,
    display, logger,
    notifier::NotifierHandle,
    stats,
    threshold::{self, ThresholdKind},
};
use anyhow::Result;
use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{self, ClearType},
};
use log::debug;
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

// ─── Public shared state (used by web dashboard) ─────────────────────────────

pub type SharedState = Arc<Mutex<MonitorState>>;

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct MonitorState {
    pub batteries: Vec<BatterySnapshot>,
    pub warnings: Vec<String>,
    pub refresh_count: u64,
    pub last_refresh: String,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn run(
    cfg: &Config,
    verbose: bool,
    show_stats: bool,
    notifier: Option<NotifierHandle>,
    audio: Option<AudioHandle>,
    shared: Option<SharedState>,
) -> Result<()> {
    let interval = Duration::from_secs(cfg.monitor.interval_secs);
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Ctrl-C handler
    ctrlc_handler(r);

    let mut stdout = io::stdout();

    // Enter alternate screen for clean refresh (like htop/btop)
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide,)?;

    // Track which threshold kinds have been fired this session
    let mut fired_kinds: HashSet<ThresholdKind> = HashSet::new();

    let result = monitor_loop(
        cfg,
        verbose,
        show_stats,
        interval,
        &running,
        &mut stdout,
        notifier,
        audio,
        shared,
        &mut fired_kinds,
    );

    // Always restore terminal, even on error
    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show,);
    let _ = terminal::disable_raw_mode();

    match result {
        Ok(_) => {
            println!("\n{}", "battnux monitor stopped.".dimmed());
        }
        Err(ref e) => {
            eprintln!("\nMonitor error: {}", e);
        }
    }

    result
}

// ─── Main loop ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn monitor_loop(
    cfg: &Config,
    verbose: bool,
    show_stats: bool,
    interval: Duration,
    running: &AtomicBool,
    stdout: &mut io::Stdout,
    notifier: Option<NotifierHandle>,
    audio: Option<AudioHandle>,
    shared: Option<SharedState>,
    fired_kinds: &mut HashSet<ThresholdKind>,
) -> Result<()> {
    let mut refresh_count: u64 = 0;

    // Force first refresh immediately
    let mut next_refresh = Instant::now();

    while running.load(Ordering::SeqCst) {
        // Check for keyboard input (q / Q / Ctrl-C)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match (code, modifiers) {
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Char('Q'), _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Time to refresh?
        if Instant::now() < next_refresh {
            continue;
        }
        next_refresh = Instant::now() + interval;
        refresh_count += 1;

        // ── Collect battery data ──────────────────────────────────────────────
        let batteries = battery_info::collect().unwrap_or_default();
        let warnings = evaluate_and_dispatch(
            &batteries,
            cfg,
            fired_kinds,
            notifier.as_ref(),
            audio.as_ref(),
        );

        // Persist snapshot
        let _ = logger::persist_snapshot(&batteries);

        // Update shared state for web dashboard
        if let Some(ref state) = shared {
            let mut s = state.lock();
            s.batteries = batteries.clone();
            s.warnings = warnings.clone();
            s.refresh_count = refresh_count;
            s.last_refresh = chrono::Local::now().to_rfc3339();
        }

        // Render to alternate screen
        render_frame(
            stdout,
            RenderArgs {
                batteries: &batteries,
                warnings: &warnings,
                verbose,
                show_stats,
                cfg,
                refresh_count,
                interval,
            },
        )?;

        debug!("Monitor refresh #{} complete", refresh_count);
    }

    Ok(())
}

// ─── Threshold dispatch ───────────────────────────────────────────────────────

fn evaluate_and_dispatch(
    batteries: &[BatterySnapshot],
    cfg: &Config,
    fired_kinds: &mut HashSet<ThresholdKind>,
    notifier: Option<&NotifierHandle>,
    audio: Option<&AudioHandle>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut active_kinds: HashSet<ThresholdKind> = HashSet::new();

    for bat in batteries {
        let events = threshold::evaluate(bat, &cfg.thresholds);
        for ev in events {
            warnings.push(ev.message.clone());
            active_kinds.insert(ev.kind.clone());

            // Only dispatch on the leading edge — when a kind becomes active
            // for the first time (or re-enters after having cleared).
            // The notifier/audio threads each apply their own cooldown on top.
            if !fired_kinds.contains(&ev.kind) {
                if let Some(n) = notifier {
                    n.send(ev.clone());
                }
                if let Some(a) = audio {
                    a.send(ev.clone());
                }
                fired_kinds.insert(ev.kind.clone());
            }
        }
    }

    // Clear kinds that are no longer active so they can re-trigger if they
    // return (e.g. battery drains below threshold again after charging).
    fired_kinds.retain(|k| active_kinds.contains(k));

    warnings
}

// ─── Frame rendering ──────────────────────────────────────────────────────────

struct RenderArgs<'a> {
    batteries: &'a [BatterySnapshot],
    warnings: &'a [String],
    verbose: bool,
    show_stats: bool,
    cfg: &'a Config,
    refresh_count: u64,
    interval: Duration,
}

fn render_frame(stdout: &mut io::Stdout, args: RenderArgs<'_>) -> Result<()> {
    let RenderArgs {
        batteries,
        warnings,
        verbose,
        show_stats,
        cfg,
        refresh_count,
        interval,
    } = args;
    // Clear and move to top-left (smooth — no flash)
    queue!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All),
    )?;

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let header = format!(
        "  {}  {}  [refresh #{} • every {}s • q to quit]",
        "battnux monitor".bright_cyan().bold(),
        now.to_string().dimmed(),
        refresh_count.to_string().bright_white(),
        interval.as_secs()
    );
    queue!(stdout, Print(header), Print("\n"))?;
    queue!(stdout, Print(format!("  {}\n", "─".repeat(72).dimmed())))?;

    // Battery panels
    for bat in batteries {
        let lines = display::render_battery_to_lines(bat, verbose);
        for line in &lines {
            queue!(stdout, Print(line), Print("\n"))?;
        }
        queue!(stdout, Print("\n"))?;
    }

    // Warnings
    if !warnings.is_empty() {
        queue!(
            stdout,
            Print(format!("  {}\n", "─── Warnings ".yellow().bold()))
        )?;
        for w in warnings {
            queue!(
                stdout,
                Print(format!("  {} {}\n", "⚠".bright_red(), w.bright_red()))
            )?;
        }
        queue!(stdout, Print("\n"))?;
    }

    // Stats sparkline (compact in monitor mode)
    if show_stats {
        if let Ok(history) = logger::load_history(60) {
            if !history.is_empty() {
                let pct_vals: Vec<f32> = history.iter().map(|s| s.percentage).collect();
                let spark = stats::sparkline_str(&pct_vals, 50);
                queue!(stdout, Print(format!("  {} {}\n", "Trend:".bold(), spark)))?;
            }
        }
    }

    // Footer hints
    let growl_status = if cfg.growl.enabled {
        "on".bright_green()
    } else {
        "off".dimmed()
    };
    let audio_status = if cfg.audio.enabled && cfg.audio.alert_file.is_some() {
        "on".bright_green()
    } else {
        "off".dimmed()
    };
    queue!(
        stdout,
        Print(format!(
            "  {} growl:{} audio:{} low:{}% high:{}% temp:{}°C\n",
            "thresholds →".dimmed(),
            growl_status,
            audio_status,
            cfg.thresholds.low.to_string().bright_yellow(),
            cfg.thresholds.high.to_string().bright_yellow(),
            cfg.thresholds.temperature.to_string().bright_yellow(),
        ))
    )?;

    stdout.flush()?;
    Ok(())
}

// ─── Ctrl-C handler ──────────────────────────────────────────────────────────

fn ctrlc_handler(running: Arc<AtomicBool>) {
    // We set a Ctrl-C handler but crossterm raw mode catches it as a key event too.
    // This is a belt-and-suspenders fallback.
    thread::spawn(move || {
        let _ = ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        });
    });
}
