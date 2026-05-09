//! Colored text output renderer.
//! `render()` writes to stdout (one-shot mode).
//! `render_battery_to_lines()` returns Vec<String> for monitor mode frame building.

use crate::battery_info::BatterySnapshot;
use anyhow::Result;
use colored::Colorize;

// ─── Color helpers ───────────────────────────────────────────────────────────

pub fn pct_colored(pct: f32) -> colored::ColoredString {
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

pub fn health_colored(pct: f32) -> colored::ColoredString {
    let s = format!("{:.1}%", pct);
    if pct >= 85.0 {
        s.bright_green()
    } else if pct >= 60.0 {
        s.yellow()
    } else {
        s.bright_red()
    }
}

pub fn state_colored(state: &str) -> colored::ColoredString {
    match state {
        "Charging" => state.bright_cyan().bold(),
        "Discharging" => state.bright_yellow(),
        "Full" => state.bright_green().bold(),
        "Empty" => state.bright_red().bold(),
        _ => state.white(),
    }
}

pub fn bar_gauge(pct: f32, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    "█".repeat(filled) + &"░".repeat(empty)
}

pub fn fmt_duration(mins: f32) -> String {
    let total = mins as u64;
    let h = total / 60;
    let m = total % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else {
        format!("{}m", m)
    }
}

fn section(label: &str) -> String {
    format!(
        "{} {}",
        label.bold().bright_white(),
        "─".repeat(46).dimmed()
    )
}

// ─── One-shot render (writes directly to stdout) ──────────────────────────────

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
    println!("  {}", "─".repeat(50).dimmed());

    for bat in batteries {
        for line in render_battery_to_lines(bat, verbose) {
            println!("{}", line);
        }
        println!();
    }
    Ok(())
}

// ─── Render a battery to a Vec of colored strings ────────────────────────────
// Used both by `render()` and by monitor mode frame builder.

pub fn render_battery_to_lines(b: &BatterySnapshot, verbose: bool) -> Vec<String> {
    let mut lines = Vec::new();

    let title = format!(
        "Battery #{} — {}",
        b.index,
        b.model.as_deref().unwrap_or("Unknown Model")
    );
    lines.push(format!("  {}", section(&title)));

    // Gauge bar
    let gauge = bar_gauge(b.percentage, 28);
    let gauge_colored = if b.percentage >= 80.0 {
        gauge.bright_green()
    } else if b.percentage >= 40.0 {
        gauge.bright_yellow()
    } else {
        gauge.bright_red()
    };
    lines.push(format!(
        "  {:>16}  {} {}",
        "Charge:".bold(),
        gauge_colored,
        pct_colored(b.percentage)
    ));

    lines.push(format!(
        "  {:>16}  {}",
        "State:".bold(),
        state_colored(&b.state)
    ));

    lines.push(format!(
        "  {:>16}  {}",
        "Health:".bold(),
        health_colored(b.health_pct)
    ));

    if let Some(mins) = b.time_to_empty_min {
        lines.push(format!(
            "  {:>16}  {}",
            "Time Left:".bold(),
            fmt_duration(mins).bright_yellow()
        ));
    }
    if let Some(mins) = b.time_to_full_min {
        lines.push(format!(
            "  {:>16}  {}",
            "Time to Full:".bold(),
            fmt_duration(mins).bright_cyan()
        ));
    }

    lines.push(format!(
        "  {:>16}  {:.2} Wh / {:.2} Wh  (design: {:.2} Wh)",
        "Energy:".bold(),
        b.energy_wh,
        b.energy_full_wh,
        b.energy_full_design_wh
    ));

    let rate_str = format!("{:.2} W", b.power_rate_w.abs());
    let rate_colored = if b.state == "Charging" {
        rate_str.bright_cyan()
    } else {
        rate_str.bright_yellow()
    };
    lines.push(format!("  {:>16}  {}", "Power Rate:".bold(), rate_colored));

    lines.push(format!(
        "  {:>16}  {}",
        "Technology:".bold(),
        b.technology.bright_white()
    ));

    if let Some(cycles) = b.cycle_count {
        lines.push(format!(
            "  {:>16}  {}",
            "Cycle Count:".bold(),
            cycles.to_string().bright_white()
        ));
    }

    if verbose {
        lines.push(String::new());
        lines.push(format!("  {}", section("  Extended Info")));
        lines.push(format!("  {:>16}  {:.4} V", "Voltage:".bold(), b.voltage_v));

        if let Some(temp) = b.temperature_c {
            let ts = format!("{:.1} °C", temp);
            let tc = if temp > 50.0 {
                ts.bright_red()
            } else if temp > 40.0 {
                ts.yellow()
            } else {
                ts.bright_green()
            };
            lines.push(format!("  {:>16}  {}", "Temperature:".bold(), tc));
        }

        if let Some(ref vendor) = b.vendor {
            lines.push(format!(
                "  {:>16}  {}",
                "Vendor:".bold(),
                vendor.bright_white()
            ));
        }
        if let Some(ref sn) = b.serial_number {
            lines.push(format!("  {:>16}  {}", "Serial No:".bold(), sn.dimmed()));
        }
        lines.push(format!(
            "  {:>16}  {}",
            "Snapshot:".bold(),
            b.timestamp.dimmed()
        ));
    }

    lines
}
