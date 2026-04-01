use crate::battery_info::BatterySnapshot;
use anyhow::Result;
use colored::Colorize;

// ─── helpers ────────────────────────────────────────────────────────────────

/// Return a color-coded string for battery percentage.
fn pct_colored(pct: f32) -> colored::ColoredString {
    let s = format!("{:.1}%", pct);
    if pct >= 80.0 {
        s.bright_green()
    } else if pct >= 40.0 {
        s.bright_yellow()
    } else if pct >= 15.0 {
        s.yellow()
    } else {
        s.bright_red().bold()
    }
}

/// Return a color-coded string for battery health.
fn health_colored(pct: f32) -> colored::ColoredString {
    let s = format!("{:.1}%", pct);
    if pct >= 85.0 {
        s.bright_green()
    } else if pct >= 60.0 {
        s.yellow()
    } else {
        s.bright_red()
    }
}

/// Return a color-coded state label.
fn state_colored(state: &str) -> colored::ColoredString {
    match state {
        "Charging" => state.bright_cyan().bold(),
        "Discharging" => state.bright_yellow(),
        "Full" => state.bright_green().bold(),
        "Empty" => state.bright_red().bold(),
        _ => state.white(),
    }
}

/// Build a Unicode bar-chart gauge (e.g. ████████░░ 80%).
fn bar_gauge(pct: f32, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);
    bar
}

/// Format minutes as a human-readable duration.
fn fmt_duration(mins: f32) -> String {
    let total = mins as u64;
    let h = total / 60;
    let m = total % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else {
        format!("{}m", m)
    }
}

// ─── divider helper ─────────────────────────────────────────────────────────

fn section(label: &str) -> String {
    let line = "─".repeat(50);
    format!("{} {}", label.bold().bright_white(), line.dimmed())
}

// ─── public render ──────────────────────────────────────────────────────────

pub fn render(batteries: &[BatterySnapshot], verbose: bool, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(batteries)?);
        return Ok(());
    }

    println!();
    println!(
        "  {} {}",
        "battnux".bright_cyan().bold(),
        "— Battery Monitor".dimmed()
    );
    println!("  {}", "─".repeat(48).dimmed());

    for bat in batteries {
        render_battery(bat, verbose);
        println!();
    }

    Ok(())
}

fn render_battery(b: &BatterySnapshot, verbose: bool) {
    let title = format!(
        "Battery #{} — {}",
        b.index,
        b.model.as_deref().unwrap_or("Unknown Model")
    );
    println!("  {}", section(&title));

    // Gauge bar
    let gauge = bar_gauge(b.percentage, 30);
    let gauge_colored = if b.percentage >= 80.0 {
        gauge.bright_green()
    } else if b.percentage >= 40.0 {
        gauge.bright_yellow()
    } else {
        gauge.bright_red()
    };
    println!(
        "  {:>16}  {} {}",
        "Charge:".bold(),
        gauge_colored,
        pct_colored(b.percentage)
    );

    println!(
        "  {:>16}  {}",
        "State:".bold(),
        state_colored(&b.state)
    );

    println!(
        "  {:>16}  {}",
        "Health:".bold(),
        health_colored(b.health_pct)
    );

    // Time remaining
    if let Some(mins) = b.time_to_empty_min {
        println!(
            "  {:>16}  {}",
            "Time Left:".bold(),
            fmt_duration(mins).bright_yellow()
        );
    }
    if let Some(mins) = b.time_to_full_min {
        println!(
            "  {:>16}  {}",
            "Time to Full:".bold(),
            fmt_duration(mins).bright_cyan()
        );
    }

    println!(
        "  {:>16}  {:.2} Wh  /  {:.2} Wh  (design: {:.2} Wh)",
        "Energy:".bold(),
        b.energy_wh,
        b.energy_full_wh,
        b.energy_full_design_wh
    );

    let rate_str = format!("{:.2} W", b.power_rate_w.abs());
    let rate_colored = if b.state == "Charging" {
        rate_str.bright_cyan()
    } else {
        rate_str.bright_yellow()
    };
    println!("  {:>16}  {}", "Power Rate:".bold(), rate_colored);

    println!(
        "  {:>16}  {}",
        "Technology:".bold(),
        b.technology.bright_white()
    );

    if let Some(cycles) = b.cycle_count {
        println!("  {:>16}  {}", "Cycle Count:".bold(), cycles.to_string().bright_white());
    }

    // Verbose extras
    if verbose {
        println!();
        println!("  {}", section("  Extended Info"));

        println!(
            "  {:>16}  {:.4} V",
            "Voltage:".bold(),
            b.voltage_v
        );

        if let Some(temp) = b.temperature_c {
            let temp_str = format!("{:.1} °C", temp);
            let temp_colored = if temp > 50.0 {
                temp_str.bright_red()
            } else if temp > 40.0 {
                temp_str.yellow()
            } else {
                temp_str.bright_green()
            };
            println!("  {:>16}  {}", "Temperature:".bold(), temp_colored);
        }

        if let Some(ref vendor) = b.vendor {
            println!("  {:>16}  {}", "Vendor:".bold(), vendor.bright_white());
        }
        if let Some(ref sn) = b.serial_number {
            println!("  {:>16}  {}", "Serial No:".bold(), sn.dimmed());
        }
        println!("  {:>16}  {}", "Snapshot:".bold(), b.timestamp.dimmed());
    }
}
