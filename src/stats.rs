//! Statistics and ASCII/Unicode graph rendering.
//! `sparkline_str()` is exported for use in monitor mode.

use crate::battery_info::BatterySnapshot;
use anyhow::Result;
use colored::Colorize;
use terminal_size::{terminal_size, Width};

// ─── Terminal width ───────────────────────────────────────────────────────────

fn term_width() -> usize {
    if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    }
}

// ─── Resampler ───────────────────────────────────────────────────────────────

fn resample(values: &[f32], n: usize) -> Vec<f32> {
    if n == 0 || values.is_empty() {
        return vec![];
    }
    if values.len() == n {
        return values.to_vec();
    }
    (0..n)
        .map(|i| {
            let pos = i as f32 * (values.len() - 1) as f32 / (n - 1).max(1) as f32;
            let lo = pos.floor() as usize;
            let hi = (lo + 1).min(values.len() - 1);
            let frac = pos - lo as f32;
            values[lo] * (1.0 - frac) + values[hi] * frac
        })
        .collect()
}

// ─── Sparkline (public — used by monitor) ────────────────────────────────────

const BLOCKS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Build a Unicode sparkline string from 0–100 values. Exported for monitor mode.
pub fn sparkline_str(values: &[f32], width: usize) -> String {
    if values.is_empty() {
        return " ".repeat(width);
    }
    let sampled = resample(values, width);
    sampled
        .iter()
        .map(|v| {
            let idx = ((v / 100.0) * (BLOCKS.len() - 1) as f32).round() as usize;
            BLOCKS[idx.min(BLOCKS.len() - 1)]
        })
        .collect()
}

fn sparkline_colored(values: &[f32], width: usize) -> colored::ColoredString {
    let s = sparkline_str(values, width);
    let last = values.last().copied().unwrap_or(50.0);
    if last >= 80.0 {
        s.bright_green()
    } else if last >= 40.0 {
        s.bright_yellow()
    } else {
        s.bright_red()
    }
}

// ─── Braille-style bar graph ──────────────────────────────────────────────────

fn braille_graph(values: &[f32], width: usize, rows: usize) -> Vec<String> {
    let sampled = resample(values, width);
    (0..rows)
        .rev()
        .map(|row| {
            let threshold = (row as f32 / (rows as f32 - 1.0)) * 100.0;
            sampled
                .iter()
                .map(|v| {
                    if *v >= threshold {
                        "█"
                    } else if *v + (100.0 / rows as f32) >= threshold {
                        "▄"
                    } else {
                        " "
                    }
                })
                .collect()
        })
        .collect()
}

fn y_axis(rows: usize) -> Vec<String> {
    (0..rows)
        .rev()
        .map(|r| {
            format!(
                "{:>3}%│",
                ((r as f32 / (rows - 1) as f32) * 100.0).round() as u32
            )
        })
        .collect()
}

fn divider(label: &str, width: usize) -> String {
    let ll = label.len() + 2;
    let side = (width.saturating_sub(ll)) / 2;
    format!(
        "{} {} {}",
        "─".repeat(side).dimmed(),
        label.bold().bright_white(),
        "─".repeat(side).dimmed()
    )
}

fn colorize_row(row: &str, avg: f32) -> String {
    if avg >= 75.0 {
        row.bright_green().to_string()
    } else if avg >= 40.0 {
        row.bright_yellow().to_string()
    } else {
        row.bright_red().to_string()
    }
}

fn print_stats(label: &str, vals: &[f32]) {
    if vals.is_empty() {
        return;
    }
    let min = vals.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = vals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let avg = vals.iter().sum::<f32>() / vals.len() as f32;
    let last = vals.last().copied().unwrap_or(0.0);
    println!(
        "  {:>18}  min:{:.2}  max:{:.2}  avg:{:.2}  last:{:.2}",
        format!("{}:", label).bold(),
        min.to_string().bright_cyan(),
        max.to_string().bright_green(),
        avg.to_string().bright_white(),
        last.to_string().bright_yellow()
    );
}

// ─── Public render ────────────────────────────────────────────────────────────

pub fn render(
    batteries: &[BatterySnapshot],
    history: &[BatterySnapshot],
    verbose: bool,
) -> Result<()> {
    let tw = term_width().min(120);
    let graph_w = tw.saturating_sub(10);
    let rows = 6;

    println!();
    println!("  {}", "battnux — Battery Statistics".bright_cyan().bold());
    println!("  {}", "─".repeat(tw.saturating_sub(2)).dimmed());

    for bat in batteries {
        println!();
        println!(
            "  {}",
            divider(
                &format!(
                    "Battery #{} — {}",
                    bat.index,
                    bat.model.as_deref().unwrap_or("Unknown")
                ),
                tw,
            )
        );
        let sc = match bat.state.as_str() {
            "Charging" => bat.state.bright_cyan().bold(),
            "Discharging" => bat.state.bright_yellow(),
            "Full" => bat.state.bright_green().bold(),
            _ => bat.state.white(),
        };
        println!(
            "  {} {:.1}%  │  State: {}  │  Health: {:.1}%  │  Rate: {:.2}W",
            "Charge:".bold(),
            bat.percentage,
            sc,
            bat.health_pct,
            bat.power_rate_w.abs()
        );
        if verbose {
            println!(
                "  {} {:.4}V  │  Cycles: {}  │  Temp: {}",
                "Voltage:".bold(),
                bat.voltage_v,
                bat.cycle_count.map_or("N/A".into(), |c| c.to_string()),
                bat.temperature_c
                    .map_or("N/A".into(), |t| format!("{:.1}°C", t))
            );
        }
    }

    if history.is_empty() {
        println!();
        println!(
            "  {}",
            "No historical data yet. Run battnux a few more times to populate graphs.".dimmed()
        );
        return Ok(());
    }

    // ── Charge % graph
    let pct_vals: Vec<f32> = history.iter().map(|s| s.percentage).collect();
    let avg_pct = pct_vals.iter().sum::<f32>() / pct_vals.len() as f32;
    let points = history.len();

    println!();
    println!(
        "  {}",
        divider(&format!("Charge % History  ({} snapshots)", points), tw)
    );
    println!();

    let chart_rows = braille_graph(&pct_vals, graph_w, rows);
    let y_labels = y_axis(rows);
    for (label, row) in y_labels.iter().zip(chart_rows.iter()) {
        println!("  {}{}", label.dimmed(), colorize_row(row, avg_pct));
    }
    println!("  {}└{}", "   ".dimmed(), "─".repeat(graph_w).dimmed());
    println!(
        "  {}  {:<width$}{}",
        "    ".dimmed(),
        "oldest".dimmed(),
        "newest".dimmed(),
        width = graph_w.saturating_sub(12)
    );

    println!();
    println!(
        "  {} {}",
        "Trend:".bold(),
        sparkline_colored(&pct_vals, graph_w)
    );

    // ── Power rate graph
    let rate_vals: Vec<f32> = history.iter().map(|s| s.power_rate_w.abs()).collect();
    let rate_max = rate_vals.iter().cloned().fold(0.0_f32, f32::max);
    if rate_max > 0.1 {
        let rate_norm: Vec<f32> = rate_vals.iter().map(|v| (v / rate_max) * 100.0).collect();
        println!();
        println!(
            "  {}",
            divider(&format!("Power Rate History  (max {:.2}W)", rate_max), tw)
        );
        println!();
        let power_rows = braille_graph(&rate_norm, graph_w, rows);
        let power_y = y_axis(rows);
        for (label, row) in power_y.iter().zip(power_rows.iter()) {
            println!("  {}{}", label.dimmed(), row.bright_blue());
        }
        println!("  {}└{}", "   ".dimmed(), "─".repeat(graph_w).dimmed());
    }

    // ── Health sparkline
    let health_vals: Vec<f32> = history.iter().map(|s| s.health_pct).collect();
    if health_vals.windows(2).any(|w| (w[0] - w[1]).abs() > 0.01) {
        println!();
        println!(
            "  {} {}",
            "Health:".bold(),
            sparkline_str(&health_vals, graph_w).bright_magenta()
        );
    }

    // ── Summary
    println!();
    println!("  {}", divider("Summary Statistics", tw));
    print_stats("Charge %", &pct_vals);
    print_stats("Power W", &rate_vals);
    if let (Some(first), Some(last)) = (history.first(), history.last()) {
        println!(
            "  {:>18}  {} → {}",
            "Time Range:".bold(),
            first.timestamp.dimmed(),
            last.timestamp.dimmed()
        );
    }
    println!();
    Ok(())
}
