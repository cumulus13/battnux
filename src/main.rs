mod audio;
mod battery_info;
mod cli;
mod config;
mod display;
mod logger;
mod monitor;
mod notifier;
mod stats;
mod threshold;
mod web;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use log::{debug, info};
use monitor::SharedState;
use parking_lot::Mutex;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Init logger
    logger::init(cli.debug, cli.verbose)?;
    debug!("battnux v{} starting", env!("CARGO_PKG_VERSION"));

    // Load config — honour --config flag, then search beside binary, then platform dir
    let explicit_config = cli.config.as_deref();
    let mut cfg = config::load(explicit_config)?;

    // CLI flags override config values
    if let Some(interval) = cli.interval {
        cfg.monitor.interval_secs = interval;
    }
    if let Some(port) = cli.web_port {
        cfg.web.port = port;
    }
    if cli.web {
        cfg.web.enabled = true;
    }

    // --show-config: print resolved path and contents, then exit
    if cli.show_config {
        return config::show(explicit_config);
    }

    // ── Spawn background threads ──────────────────────────────────────────────

    // Growl notifier thread (persistent background, dies when handle is dropped)
    let notifier = notifier::spawn(cfg.growl.clone());
    if notifier.is_some() {
        info!("Growl notifier thread started");
    }

    // Audio alert thread
    let audio = audio::spawn(cfg.audio.clone());
    if audio.is_some() {
        info!("Audio alert thread started");
    }

    // Shared state for web dashboard
    let shared: Option<SharedState> = if cfg.web.enabled {
        Some(Arc::new(Mutex::new(monitor::MonitorState::default())))
    } else {
        None
    };

    // Web dashboard (Tokio async task)
    if cfg.web.enabled {
        let s = shared.clone().unwrap();
        let host = cfg.web.host.clone();
        let port = cfg.web.port;
        tokio::spawn(async move {
            web::spawn(s, &host, port).await;
        });
        // Give web server a tick to bind before printing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // ── Monitor mode ──────────────────────────────────────────────────────────

    if cli.monitor {
        info!(
            "Starting monitor mode (interval={}s)",
            cfg.monitor.interval_secs
        );
        return monitor::run(&cfg, cli.verbose, cli.stats, notifier, audio, shared);
    }

    // ── One-shot mode ─────────────────────────────────────────────────────────

    let batteries = battery_info::collect()?;

    if batteries.is_empty() {
        eprintln!(
            "{}",
            colored::Colorize::red("No battery detected on this system.")
        );
        std::process::exit(1);
    }

    debug!("Found {} battery(ies)", batteries.len());

    // Persist snapshot
    logger::persist_snapshot(&batteries)?;

    // Evaluate thresholds and dispatch notifications
    for bat in &batteries {
        let events = threshold::evaluate(bat, &cfg.thresholds);
        for ev in &events {
            if let Some(ref n) = notifier {
                n.send(ev.clone());
            }
            if let Some(ref a) = audio {
                a.send(ev.clone());
            }
        }
        if !events.is_empty() {
            println!();
            threshold::print_warnings(&events);
        }
    }

    if cli.stats {
        let history = logger::load_history(cli.history_limit)?;
        stats::render(&batteries, &history, cli.verbose)?;
    } else {
        display::render(&batteries, cli.verbose, cli.json)?;
    }

    // Drop handles to close the mpsc channels — this signals the background
    // threads to finish their current work and exit cleanly.
    // We then wait long enough for a TCP connect + GNTP round-trip to Growl
    // (default connect timeout on most systems is ~5s; we wait up to 6s).
    let has_notifications = notifier.is_some() || audio.is_some();
    drop(notifier);
    drop(audio);
    if has_notifications {
        tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    }

    Ok(())
}
