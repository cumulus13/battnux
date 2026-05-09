//! Configuration file support.
//! Reads/writes `~/.config/battnux/config.toml` (or XDG_CONFIG_HOME).
//! All keys have sensible defaults so the file is entirely optional.

use anyhow::{Context, Result};
use dirs::config_dir;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

// ─── Threshold block ────────────────────────────────────────────────────────

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
fn default_low() -> f32 {
    20.0
}
fn default_high() -> f32 {
    95.0
}
fn default_temp() -> f32 {
    55.0
}
fn default_health() -> f32 {
    50.0
}

// ─── Growl / GNTP block ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowlConfig {
    /// Enable Growl notifications (default true)
    #[serde(default = "bool_true")]
    pub enabled: bool,
    /// Growl server host (default "localhost")
    #[serde(default = "default_host")]
    pub host: String,
    /// Growl server port (default 23053)
    #[serde(default = "default_port")]
    pub port: u16,
    /// Optional Growl password
    #[serde(default)]
    pub password: Option<String>,
    /// Path to notification icon (default "battnux.png" beside binary)
    #[serde(default = "default_icon")]
    pub icon: String,
    /// Sticky notifications (stay until dismissed)
    #[serde(default)]
    pub sticky: bool,
    /// Re-notify cooldown in seconds (prevent spam, default 60)
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
fn default_host() -> String {
    "localhost".into()
}
fn default_port() -> u16 {
    23053
}
fn default_icon() -> String {
    "battnux.png".into()
}
fn default_cooldown() -> u64 {
    60
}

// ─── Audio alert block ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Enable audio alerts (default true)
    #[serde(default = "bool_true")]
    pub enabled: bool,
    /// Path to audio file played on low/high threshold breach
    #[serde(default)]
    pub alert_file: Option<String>,
    /// Volume 0.0–1.0 (default 1.0)
    #[serde(default = "default_volume")]
    pub volume: f32,
}
impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            alert_file: None,
            volume: default_volume(),
        }
    }
}
fn default_volume() -> f32 {
    1.0
}

// ─── Monitor mode block ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Refresh interval in seconds (default 5)
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    /// Show verbose output in monitor mode
    #[serde(default)]
    pub verbose: bool,
}
impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_interval(),
            verbose: false,
        }
    }
}
fn default_interval() -> u64 {
    5
}

// ─── Web dashboard block ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Enable web dashboard (default false)
    #[serde(default)]
    pub enabled: bool,
    /// Listen address (default "127.0.0.1")
    #[serde(default = "default_web_host")]
    pub host: String,
    /// Listen port (default 7878)
    #[serde(default = "default_web_port")]
    pub port: u16,
}
impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_web_host(),
            port: default_web_port(),
        }
    }
}
fn default_web_host() -> String {
    "127.0.0.1".into()
}
fn default_web_port() -> u16 {
    7878
}

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

// ─── I/O ─────────────────────────────────────────────────────────────────────

fn config_path() -> Result<PathBuf> {
    let base = config_dir().unwrap_or_else(|| PathBuf::from("."));
    Ok(base.join("battnux").join("config.toml"))
}

/// Load config from disk, creating a default file if none exists.
pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        debug!("No config found at {}; using defaults", path.display());
        // Write the default so the user has a template to edit
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let default_cfg = Config::default();
        let toml_str = toml::to_string_pretty(&default_cfg).context("Serialise default config")?;
        let header = "# battnux configuration file\n# https://github.com/cumulus13/battnux\n\n";
        fs::write(&path, format!("{}{}", header, toml_str)).ok();
        return Ok(default_cfg);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("Read config: {}", path.display()))?;
    let cfg: Config =
        toml::from_str(&raw).with_context(|| format!("Parse config: {}", path.display()))?;
    debug!("Loaded config from {}", path.display());
    Ok(cfg)
}

/// Print the config file path and contents to stdout (for --show-config).
pub fn show() -> Result<()> {
    let path = config_path()?;
    println!(
        "{}",
        colored::Colorize::bold(colored::Colorize::bright_cyan(
            format!("Config file: {}", path.display()).as_str()
        ))
    );
    if path.exists() {
        println!("{}", fs::read_to_string(&path)?);
    } else {
        println!(
            "{}",
            colored::Colorize::dimmed(
                "(no config file — defaults in use; will be created on next run)"
            )
        );
    }
    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────
fn bool_true() -> bool {
    true
}
