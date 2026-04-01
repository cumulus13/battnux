use crate::battery_info::BatterySnapshot;
use anyhow::Result;
use colored::Colorize;
use terminal_size::{terminal_size, Width};

// ─── Terminal width ──────────────────────────────────────────────────────────

fn term_width() -> usize {
    if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    }
}

// ─── Sparkline / block-bar graph ────────────────────────────────────────────

/// Build a Unicode sparkline from a series of 0.0–100.0 values.
fn sparkline(values: &[f32], width: usize) -> String {
    // Unicode block elements (⠀ to █ — 8 levels)
    const BLOCKS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if values.is_empty() {
        return " ".repeat(width);
    }

    // Down-sample or pad to `width` columns
    let sampled = resample(values, width);

    sampled
        .iter()
        .map(|v| {
            let idx = ((v / 100.0) * (BLOCKS.len() - 1) as f32).round() as usize;
            BLOCKS[idx.min(BLOCKS.len() - 1)]
        })
        .collect()
}

/// Braille-style multi-row bar graph (4 rows high).
fn braille_graph(values: &[f32], width: usize) -> Vec<String> {
    const ROWS: usize = 6;
    let sampled = resample(values, width);

    (0..ROWS)
        .rev()
        .map(|row| {
            let threshold = (row as f32 / (ROWS as f32 - 1.0)) * 100.0;
            sampled
                .iter()
                .map(|v| {
                    if *v >= threshold {
                        "█"
                    } else if *v + (100.0 / ROWS as f32) >= threshold {
                        "▄"
                    } else {
                        " "
                    }
                })
                .collect::<String>()
        })
        .collect()
}

/// Resample `values` to exactly `n` points by linear interpolation.
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

// ─── Y-axis labels ───────────────────────────────────────────────────────────

fn y_axis(rows: usize) -> Vec<String> {
    (0..rows)
        .rev()
        .map(|r| format!("{:>3}%│", ((r as f32 / (rows - 1) as f32) * 100.0).round() as u32))
        .collect()
}

// ─── Section divider ─────────────────────────────────────────────────────────

fn divider(label: &str, width: usize) -> String {
    let label_len = label.len() + 2;
    let side = (width.saturating_sub(label_len)) / 2;
    format!(
        "{} {} {}",
        "─".repeat(side).dimmed(),
        label.bold().bright_white(),
        "─".repeat(side).dimmed()
    )
}

// ─── Main stats render ───────────────────────────────────────────────────────

pub fn render(
    batteries: &[BatterySnapshot],
    history: &[BatterySnapshot],
    verbose: bool,
) -> Result<()> {
    let tw = term_width().min(120);
    let graph_w = tw.saturating_sub(10); // leave room for y-axis labels

    println!();
    println!("  {}", "battnux — Battery Statistics".bright_cyan().bold());
    println!("  {}", "─".repeat(tw.saturating_sub(2)).dimmed());

    // ── Current status summary ──
    for bat in batteries {
        println!();
        println!("  {}", divider(&format!("Battery #{} — {}", bat.index, bat.model.as_deref().unwrap_or("Unknown")), tw));

        // Inline summary
        let state_color = match bat.state.as_str() {
            "Charging" => bat.state.bright_cyan().bold(),
            "Discharging" => bat.state.bright_yellow(),
            "Full" => bat.state.bright_green().bold(),
            _ => bat.state.white(),
        };

        println!(
            "  {} {:.1}%  │  State: {}  │  Health: {:.1}%  │  Rate: {:.2}W",
            "Charge:".bold(),
            bat.percentage,
            state_color,
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
        println!("  {}", "No historical data yet. Run battnux a few more times to populate graphs.".dimmed());
        println!("  {}", "Snapshots are saved automatically each run.".dimmed());
        return Ok(());
    }

    // ── Charge % graph ──
    let pct_vals: Vec<f32> = history.iter().map(|s| s.percentage).collect();
    let points = history.len();

    println!();
    println!("  {}", divider(&format!("Charge % History  ({} snapshots)", points), tw));
    println!();

    let rows = braille_graph(&pct_vals, graph_w);
    let y_labels = y_axis(rows.len());

    for (label, row) in y_labels.iter().zip(rows.iter()) {
        // Color the bar rows
        let colored_row = colorize_row(row, &pct_vals);
        println!("  {}{}", label.dimmed(), colored_row);
    }
    println!("  {}└{}", "   ".dimmed(), "─".repeat(graph_w).dimmed());
    println!("  {}  {:<width$}{}", "    ".dimmed(), "oldest".dimmed(), "newest".dimmed(), width = graph_w.saturating_sub(12));

    // ── Sparkline summary ──
    println!();
    println!("  {} {}", "Trend:".bold(), sparkline_colored(&pct_vals, graph_w));

    // ── Power rate graph (if we have variance) ──
    let rate_vals: Vec<f32> = history.iter().map(|s| s.power_rate_w.abs()).collect();
    let rate_max = rate_vals.iter().cloned().fold(0.0_f32, f32::max);

    if rate_max > 0.1 {
        // Normalize rates to 0-100 for graphing
        let rate_norm: Vec<f32> = rate_vals.iter().map(|v| (v / rate_max) * 100.0).collect();

        println!();
        println!("  {}", divider(&format!("Power Rate History  (max {:.2}W)", rate_max), tw));
        println!();

        let power_rows = braille_graph(&rate_norm, graph_w);
        let power_y = y_axis(power_rows.len());

        for (label, row) in power_y.iter().zip(power_rows.iter()) {
            println!("  {}{}", label.dimmed(), row.bright_blue());
        }
        println!("  {}└{}", "   ".dimmed(), "─".repeat(graph_w).dimmed());
    }

    // ── Health trend ──
    let health_vals: Vec<f32> = history.iter().map(|s| s.health_pct).collect();
    if health_vals.windows(2).any(|w| (w[0] - w[1]).abs() > 0.01) {
        let spark = sparkline(&health_vals, graph_w);
        println!();
        println!("  {} {}", "Health Trend:".bold(), spark.bright_magenta());
    }

    // ── Stats summary table ──
    println!();
    println!("  {}", divider("Summary Statistics", tw));
    print_stats("Charge %", &pct_vals);
    print_stats("Power W", &rate_vals);
    if let Some(first) = history.first() {
        if let Some(last) = history.last() {
            println!(
                "  {:>18}  {} → {}",
                "Time Range:".bold(),
                first.timestamp.dimmed(),
                last.timestamp.dimmed()
            );
        }
    }

    println!();
    Ok(())
}

// ─── Stats table row ─────────────────────────────────────────────────────────

fn print_stats(label: &str, vals: &[f32]) {
    if vals.is_empty() {
        return;
    }
    let min = vals.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = vals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let avg = vals.iter().sum::<f32>() / vals.len() as f32;
    let last = vals.last().copied().unwrap_or(0.0);

    println!(
        "  {:>18}  min: {:.2}  max: {:.2}  avg: {:.2}  last: {:.2}",
        format!("{}:", label).bold(),
        min.to_string().bright_cyan(),
        max.to_string().bright_green(),
        avg.to_string().bright_white(),
        last.to_string().bright_yellow()
    );
}

// ─── Color helpers ───────────────────────────────────────────────────────────

fn colorize_row(row: &str, pct_vals: &[f32]) -> String {
    let avg = pct_vals.iter().sum::<f32>() / pct_vals.len().max(1) as f32;
    if avg >= 75.0 {
        row.bright_green().to_string()
    } else if avg >= 40.0 {
        row.bright_yellow().to_string()
    } else {
        row.bright_red().to_string()
    }
}

fn sparkline_colored(values: &[f32], width: usize) -> colored::ColoredString {
    let s = sparkline(values, width);
    let last = values.last().copied().unwrap_or(50.0);
    if last >= 80.0 {
        s.bright_green()
    } else if last >= 40.0 {
        s.bright_yellow()
    } else {
        s.bright_red()
    }
}
