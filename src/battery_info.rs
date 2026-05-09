use anyhow::{Context, Result};
use battery::{Battery, Manager, State};
use chrono::Local;
use log::debug;
use serde::{Deserialize, Serialize};

/// Snapshot of a single battery at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatterySnapshot {
    /// Index among batteries found
    pub index: usize,
    /// Battery model string, if available
    pub model: Option<String>,
    /// Battery vendor/manufacturer, if available
    pub vendor: Option<String>,
    /// Serial number, if available
    pub serial_number: Option<String>,
    /// Charge percentage 0.0–100.0
    pub percentage: f32,
    /// Charge state (Charging / Discharging / Full / Unknown)
    pub state: String,
    /// Energy currently stored, in watt-hours
    pub energy_wh: f32,
    /// Full design energy capacity, in watt-hours
    pub energy_full_wh: f32,
    /// Design capacity (original), in watt-hours
    pub energy_full_design_wh: f32,
    /// Health percentage: energy_full / energy_full_design * 100
    pub health_pct: f32,
    /// Current voltage in volts
    pub voltage_v: f32,
    /// Current charge/discharge rate in watts (positive = charging)
    pub power_rate_w: f32,
    /// Temperature in Celsius, if sensor available
    pub temperature_c: Option<f32>,
    /// Estimated time to empty in minutes (only when discharging)
    pub time_to_empty_min: Option<f32>,
    /// Estimated time to full in minutes (only when charging)
    pub time_to_full_min: Option<f32>,
    /// Battery cycle count, if available
    pub cycle_count: Option<u32>,
    /// Technology string (Li-ion, LiPo, etc.)
    pub technology: String,
    /// Timestamp of this snapshot (RFC3339)
    pub timestamp: String,
}

impl BatterySnapshot {
    fn from_battery(index: usize, bat: &Battery) -> Self {
        let pct = bat.state_of_charge().value * 100.0;
        let state = format!("{:?}", bat.state());

        let energy_wh = bat.energy().value / 3600.0; // Joules → Wh
        let energy_full_wh = bat.energy_full().value / 3600.0;
        let energy_full_design_wh = bat.energy_full_design().value / 3600.0;
        let health_pct = if energy_full_design_wh > 0.0 {
            (energy_full_wh / energy_full_design_wh * 100.0).min(100.0)
        } else {
            0.0
        };

        let voltage_v = bat.voltage().value;
        let power_rate_w = bat.energy_rate().value;

        let temperature_c = bat.temperature().map(|t| t.value - 273.15); // K → °C

        let time_to_empty_min = if bat.state() == State::Discharging {
            bat.time_to_empty().map(|t| t.value / 60.0)
        } else {
            None
        };
        let time_to_full_min = if bat.state() == State::Charging {
            bat.time_to_full().map(|t| t.value / 60.0)
        } else {
            None
        };

        let cycle_count = bat.cycle_count();
        let technology = format!("{:?}", bat.technology());
        let model = bat.model().map(str::to_owned);
        let vendor = bat.vendor().map(str::to_owned);
        let serial_number = bat.serial_number().map(str::to_owned);
        let timestamp = Local::now().to_rfc3339();

        debug!(
            "Battery[{}]: {}% {} @ {:.2}V {:.2}W",
            index, pct, state, voltage_v, power_rate_w
        );

        BatterySnapshot {
            index,
            model,
            vendor,
            serial_number,
            percentage: pct,
            state,
            energy_wh,
            energy_full_wh,
            energy_full_design_wh,
            health_pct,
            voltage_v,
            power_rate_w,
            temperature_c,
            time_to_empty_min,
            time_to_full_min,
            cycle_count,
            technology,
            timestamp,
        }
    }
}

/// Collect info from all available batteries.
pub fn collect() -> Result<Vec<BatterySnapshot>> {
    let manager = Manager::new().context("Failed to initialise battery manager")?;
    let mut snapshots = Vec::new();

    for (idx, result) in manager
        .batteries()
        .context("Failed to enumerate batteries")?
        .enumerate()
    {
        match result {
            Ok(bat) => {
                snapshots.push(BatterySnapshot::from_battery(idx, &bat));
            }
            Err(e) => {
                log::warn!("Could not read battery {}: {}", idx, e);
            }
        }
    }

    Ok(snapshots)
}
