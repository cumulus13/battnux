//! Threshold evaluation and warning display.
//! Returns structured `ThresholdEvent`s that other modules (growl, audio) react to.

use crate::{battery_info::BatterySnapshot, config::Thresholds};
use colored::Colorize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThresholdKind {
    LowBattery,
    HighBattery,
    HighTemperature,
    LowHealth,
}

#[derive(Debug, Clone)]
pub struct ThresholdEvent {
    pub kind: ThresholdKind,
    pub title: String,
    pub message: String,
}

/// Evaluate all thresholds for a battery snapshot. Returns any triggered events.
pub fn evaluate(bat: &BatterySnapshot, cfg: &Thresholds) -> Vec<ThresholdEvent> {
    let mut events = Vec::new();

    if bat.percentage <= cfg.low && bat.state != "Charging" && bat.state != "Full" {
        events.push(ThresholdEvent {
            kind: ThresholdKind::LowBattery,
            title: format!("⚠ Low Battery — {:.0}%", bat.percentage),
            message: format!(
                "Battery #{} is at {:.1}% (threshold: {:.0}%). {}",
                bat.index,
                bat.percentage,
                cfg.low,
                bat.time_to_empty_min
                    .map(|m| format!("~{} remaining.", fmt_mins(m)))
                    .unwrap_or_default()
            ),
        });
    }

    if bat.percentage >= cfg.high && bat.state == "Charging" {
        events.push(ThresholdEvent {
            kind: ThresholdKind::HighBattery,
            title: format!("🔋 Battery Full — {:.0}%", bat.percentage),
            message: format!(
                "Battery #{} reached {:.1}% (threshold: {:.0}%). Consider unplugging.",
                bat.index, bat.percentage, cfg.high
            ),
        });
    }

    if let Some(temp) = bat.temperature_c {
        if temp >= cfg.temperature {
            events.push(ThresholdEvent {
                kind: ThresholdKind::HighTemperature,
                title: format!("🌡 High Temperature — {:.1}°C", temp),
                message: format!(
                    "Battery #{} temperature is {:.1}°C (threshold: {:.0}°C). Check ventilation.",
                    bat.index, temp, cfg.temperature
                ),
            });
        }
    }

    if bat.health_pct < cfg.health {
        events.push(ThresholdEvent {
            kind: ThresholdKind::LowHealth,
            title: format!("💀 Low Battery Health — {:.0}%", bat.health_pct),
            message: format!(
                "Battery #{} health is {:.1}% (threshold: {:.0}%). Consider replacement.",
                bat.index, bat.health_pct, cfg.health
            ),
        });
    }

    events
}

/// Print threshold warnings to the terminal with color.
pub fn print_warnings(events: &[ThresholdEvent]) {
    for ev in events {
        let (icon, color_fn): (&str, fn(&str) -> colored::ColoredString) = match ev.kind {
            ThresholdKind::LowBattery => ("⚠", |s: &str| s.bright_red().bold()),
            ThresholdKind::HighBattery => ("🔋", |s: &str| s.bright_green().bold()),
            ThresholdKind::HighTemperature => ("🌡", |s: &str| s.bright_red().bold()),
            ThresholdKind::LowHealth => ("💀", |s: &str| s.yellow().bold()),
        };
        println!("  {} {}", icon, color_fn(&ev.title));
        println!("    {}", ev.message.dimmed());
    }
}

fn fmt_mins(mins: f32) -> String {
    let total = mins as u64;
    let h = total / 60;
    let m = total % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else {
        format!("{}m", m)
    }
}
