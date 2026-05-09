//! Configuration file support.
//!
//! Search order for config file:
//!   1. --config <PATH> flag (explicit, always wins)
//!   2. Directory containing the battnux binary (beside the exe)
//!   3. Platform config dir: %APPDATA%\battnux\config.toml (Windows)
//!      ~/.config/battnux/config.toml (Linux/macOS)
//!
//! The config file may contain sections battnux does not know about
//! (e.g. [terminal] from a shared config). Those are silently ignored
//! by parsing into a wrapper that extracts only the battnux sections.
//!
//! All battnux keys are optional — unset keys fall back to built-in defaults.

use anyhow::{Context, Result};
use dirs::config_dir;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

// ─── Threshold block ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    /// Warn when charge drops at or below this % (default 20)
    #[serde(default = "default_low")]
    pub low: f32,
    /// Warn when charge rises at or above this % (default 95)
    #[serde(default = "default_high")]
    pub high: f32,
    /// Warn when temperature exceeds this °C (default 55)
    #[serde(default = "default_temp")]
    pub temperature: f32,
    /// Warn when health drops below this % (default 50)
    #[serde(default = "default_health")]
    pub health: f32,
}
impl Default for Thresholds {
    fn default() -> Self {
        Self {
            low: default_low(),
            high: default_high(),
            temperature: default_temp(),
            health: default_health(),
        }
    }
}
fn default_low()    -> f32 { 20.0 }
fn default_high()   -> f32 { 95.0 }
fn default_temp()   -> f32 { 55.0 }
fn default_health() -> f32 { 50.0 }

// ─── Growl / GNTP block ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowlConfig {
    #[serde(default = "bool_true")]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub sticky: bool,
    #[serde(default = "default_cooldown")]
    pub cooldown_secs: u64,
}
impl Default for GrowlConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_host(),
            port: default_port(),
            password: None,
            icon: default_icon(),
            sticky: false,
            cooldown_secs: default_cooldown(),
        }
    }
}
fn default_host()     -> String { "localhost".into() }
fn default_port()     -> u16   { 23053 }
fn default_icon()     -> String { "battnux.png".into() }
fn default_cooldown() -> u64   { 60 }

// ─── Audio alert block ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default = "bool_true")]
    pub enabled: bool,
    #[serde(default)]
    pub alert_file: Option<String>,
    #[serde(default = "default_volume")]
    pub volume: f32,
}
impl Default for AudioConfig {
    fn default() -> Self {
        Self { enabled: true, alert_file: None, volume: default_volume() }
    }
}
fn default_volume() -> f32 { 1.0 }

// ─── Monitor mode block ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default)]
    pub verbose: bool,
}
impl Default for MonitorConfig {
    fn default() -> Self {
        Self { interval_secs: default_interval(), verbose: false }
    }
}
fn default_interval() -> u64 { 5 }

// ─── Web dashboard block ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_web_host")]
    pub host: String,
    #[serde(default = "default_web_port")]
    pub port: u16,
}
impl Default for WebConfig {
    fn default() -> Self {
        Self { enabled: false, host: default_web_host(), port: default_web_port() }
    }
}
fn default_web_host() -> String { "127.0.0.1".into() }
fn default_web_port() -> u16   { 7878 }

// ─── Root config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub thresholds: Thresholds,
    #[serde(default)]
    pub growl: GrowlConfig,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub monitor: MonitorConfig,
    #[serde(default)]
    pub web: WebConfig,
}

// ─── Wrapper that tolerates unknown top-level sections ───────────────────────
//
// The TOML crate rejects unknown keys when deserializing into a typed struct.
// We first parse into a raw `toml::Table` (which accepts everything), then
// extract only the sections battnux cares about and deserialize those.
// Unknown sections like [terminal], [editor], etc. are silently dropped.

fn extract_from_table(table: toml::Table) -> Result<Config> {
    // Helper: pull one optional section out of the table and deserialize it.
    fn section<T: for<'de> Deserialize<'de> + Default>(
        table: &toml::Table,
        key: &str,
    ) -> Result<T> {
        match table.get(key) {
            None => Ok(T::default()),
            Some(val) => val
                .clone()
                .try_into()
                .with_context(|| format!("Parse [{}] section", key)),
        }
    }

    Ok(Config {
        thresholds: section(&table, "thresholds")?,
        growl:      section(&table, "growl")?,
        audio:      section(&table, "audio")?,
        monitor:    section(&table, "monitor")?,
        web:        section(&table, "web")?,
    })
}

// ─── Config file search ───────────────────────────────────────────────────────

/// Resolve the config file path using the priority search order.
/// Returns (path, found) — `found` is false when we're falling back to default.
pub fn resolve_path(explicit: Option<&Path>) -> (PathBuf, bool) {
    // 1. Explicit --config flag
    if let Some(p) = explicit {
        return (p.to_path_buf(), p.exists());
    }

    // 2. Beside the binary (most useful on Windows where users drop the exe
    //    in a tools directory alongside their own config.toml)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let beside = dir.join("config.toml");
            if beside.exists() {
                return (beside, true);
            }
        }
    }

    // 3. Platform config dir
    let platform = config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("battnux")
        .join("config.toml");

    let found = platform.exists();
    (platform, found)
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Load config. `explicit` comes from the --config CLI flag.
pub fn load(explicit: Option<&Path>) -> Result<Config> {
    let (path, found) = resolve_path(explicit);

    if !found {
        debug!("No config file found; using built-in defaults");
        debug!("(searched beside binary and {})", path.display());

        // Write a default template to the platform config dir so the user
        // has something to edit — but only when no explicit path was given.
        if explicit.is_none() {
            write_default_template(&path);
        }
        return Ok(Config::default());
    }

    info!("Loading config from {}", path.display());

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Cannot read config file: {}", path.display()))?;

    // Parse as a generic TOML table first — tolerates unknown sections.
    let table: toml::Table = toml::from_str(&raw)
        .with_context(|| format!("Invalid TOML in config file: {}", path.display()))?;

    let cfg = extract_from_table(table)
        .with_context(|| format!("Config values invalid in: {}", path.display()))?;

    debug!("Config loaded — thresholds: low={} high={} temp={} health={}",
        cfg.thresholds.low, cfg.thresholds.high,
        cfg.thresholds.temperature, cfg.thresholds.health);

    Ok(cfg)
}

/// Write a commented default config template (silently, best-effort).
fn write_default_template(path: &Path) {
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let default_cfg = Config::default();
    if let Ok(toml_str) = toml::to_string_pretty(&default_cfg) {
        let content = format!(
            "# battnux configuration file\n\
             # https://github.com/cumulus13/battnux\n\
             #\n\
             # This file was auto-created with default values.\n\
             # Edit and save — battnux will pick it up on next run.\n\n\
             {}",
            toml_str
        );
        fs::write(path, content).ok();
        debug!("Wrote default config template to {}", path.display());
    }
}

/// Print the resolved config path and its contents (for --show-config).
pub fn show(explicit: Option<&Path>) -> Result<()> {
    let (path, found) = resolve_path(explicit);

    use colored::Colorize;
    println!("{}", format!("Config file: {}", path.display()).bright_cyan().bold());

    if found {
        println!("{}", fs::read_to_string(&path)?);
    } else {
        println!("{}", "(no config file found — built-in defaults in use)".dimmed());
        println!("{}", format!("  A template will be written to: {}", path.display()).dimmed());
    }
    Ok(())
}

fn bool_true() -> bool { true }