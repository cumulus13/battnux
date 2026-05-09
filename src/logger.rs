use crate::battery_info::BatterySnapshot;
use anyhow::{Context, Result};
use dirs::data_local_dir;
use log::{debug, info, warn};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

/// Returns the path to the battnux data directory, creating it if needed.
fn data_dir() -> Result<PathBuf> {
    let base = data_local_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    let dir = base.join("battnux");
    fs::create_dir_all(&dir)
        .with_context(|| format!("Cannot create data dir: {}", dir.display()))?;
    Ok(dir)
}

/// Path to the JSONL history log file.
fn log_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("history.jsonl"))
}

/// Append a new set of battery snapshots as one JSON line per battery.
pub fn persist_snapshot(batteries: &[BatterySnapshot]) -> Result<()> {
    let path = log_path()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Cannot open log file: {}", path.display()))?;

    for bat in batteries {
        let line = serde_json::to_string(bat).context("Failed to serialise battery snapshot")?;
        writeln!(file, "{}", line).context("Failed to write to log file")?;
    }

    debug!(
        "Persisted {} snapshots to {}",
        batteries.len(),
        path.display()
    );
    info!("Log: {}", path.display());
    Ok(())
}

/// Load the last `limit` snapshots for battery index 0 (primary battery).
pub fn load_history(limit: usize) -> Result<Vec<BatterySnapshot>> {
    let path = log_path()?;
    if !path.exists() {
        debug!("No history file found at {}", path.display());
        return Ok(vec![]);
    }

    let file =
        fs::File::open(&path).with_context(|| format!("Cannot read log: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut all: Vec<BatterySnapshot> = Vec::new();
    for (line_no, line_result) in reader.lines().enumerate() {
        match line_result {
            Ok(line) if line.trim().is_empty() => {}
            Ok(line) => {
                match serde_json::from_str::<BatterySnapshot>(&line) {
                    Ok(snap) if snap.index == 0 => all.push(snap),
                    Ok(_) => {} // other battery index, skip for primary graph
                    Err(e) => warn!("Line {}: parse error: {}", line_no + 1, e),
                }
            }
            Err(e) => warn!("IO error reading log line {}: {}", line_no + 1, e),
        }
    }

    // Return last `limit` entries
    let start = all.len().saturating_sub(limit);
    Ok(all[start..].to_vec())
}

// ─── Logger initialization ───────────────────────────────────────────────────

/// Initialize env_logger respecting debug/verbose flags.
pub fn init(debug: bool, _verbose: bool) -> Result<()> {
    let level = if debug { "debug" } else { "warn" };

    // Allow RUST_LOG env override, else fall back to our computed level
    let env = env_logger::Env::default().default_filter_or(level);
    env_logger::Builder::from_env(env)
        .format_timestamp_secs()
        .format_target(debug)
        .init();

    Ok(())
}
