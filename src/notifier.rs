//! Growl/GNTP notification thread.
//! Spawns a background thread that receives `ThresholdEvent`s via channel
//! and sends Growl notifications with cooldown tracking to prevent spam.

use crate::{
    config::GrowlConfig,
    threshold::{ThresholdEvent, ThresholdKind},
};
use anyhow::Result;
use gntp::{GntpClient, IconMode, NotificationType, Resource};
use log::{debug, error, info, warn};
use std::{
    collections::HashMap,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

// ─── Notification type IDs (must match registered types) ────────────────────
const NOTIF_LOW: &str = "low-battery";
const NOTIF_HIGH: &str = "high-battery";
const NOTIF_TEMP: &str = "high-temperature";
const NOTIF_HEALTH: &str = "low-health";

/// Handle for sending notification events to the background thread.
#[derive(Clone)]
pub struct NotifierHandle {
    tx: Sender<ThresholdEvent>,
}

impl NotifierHandle {
    /// Queue a notification event (non-blocking).
    pub fn send(&self, event: ThresholdEvent) {
        if let Err(e) = self.tx.send(event) {
            warn!("Notifier channel closed: {}", e);
        }
    }
}

/// Spawn the Growl notification background thread.
/// Returns `None` if notifications are disabled in config.
pub fn spawn(cfg: GrowlConfig) -> Option<NotifierHandle> {
    if !cfg.enabled {
        info!("Growl notifications disabled in config");
        return None;
    }

    let (tx, rx): (Sender<ThresholdEvent>, Receiver<ThresholdEvent>) = mpsc::channel();

    thread::Builder::new()
        .name("battnux-notifier".into())
        .spawn(move || {
            notifier_loop(cfg, rx);
        })
        .map_err(|e| error!("Failed to spawn notifier thread: {}", e))
        .ok()?;

    Some(NotifierHandle { tx })
}

// ─── Background loop ─────────────────────────────────────────────────────────

fn notifier_loop(cfg: GrowlConfig, rx: Receiver<ThresholdEvent>) {
    // Track last notification time per kind to enforce cooldown
    let mut last_sent: HashMap<ThresholdKind, Instant> = HashMap::new();
    let cooldown = Duration::from_secs(cfg.cooldown_secs);

    // Build and register the GNTP client
    let mut client = match build_client(&cfg) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to build Growl client: {}", e);
            return;
        }
    };

    if let Err(e) = register_client(&mut client, &cfg) {
        warn!("Growl registration failed (is Growl running?): {}", e);
        // Don't exit — keep the thread alive so future events can retry
    }

    // Process incoming events
    for event in rx {
        // Check cooldown
        if let Some(last) = last_sent.get(&event.kind) {
            if last.elapsed() < cooldown {
                debug!("Skipping {:?} notification (cooldown)", event.kind);
                continue;
            }
        }

        let notif_type = match event.kind {
            ThresholdKind::LowBattery => NOTIF_LOW,
            ThresholdKind::HighBattery => NOTIF_HIGH,
            ThresholdKind::HighTemperature => NOTIF_TEMP,
            ThresholdKind::LowHealth => NOTIF_HEALTH,
        };

        match client.notify(notif_type, &event.title, &event.message) {
            Ok(_) => {
                info!("Growl notification sent: {}", event.title);
                last_sent.insert(event.kind, Instant::now());
            }
            Err(e) => {
                warn!(
                    "Growl notify failed ({}): {}. Attempting re-register…",
                    event.title, e
                );
                // Try to re-register and resend once
                if register_client(&mut client, &cfg).is_ok() {
                    if let Err(e2) = client.notify(notif_type, &event.title, &event.message) {
                        error!("Growl notify still failing after re-register: {}", e2);
                    } else {
                        last_sent.insert(event.kind, Instant::now());
                    }
                }
            }
        }
    }

    debug!("Notifier thread exiting (channel closed)");
}

// ─── Client construction ─────────────────────────────────────────────────────

fn build_client(cfg: &GrowlConfig) -> Result<GntpClient> {
    // DataUrl is the most compatible mode (works with Growl for Windows too)
    let mut builder = GntpClient::new("battnux")
        .with_host(&cfg.host)
        .with_port(cfg.port)
        .with_icon_mode(IconMode::DataUrl);

    if let Some(ref pw) = cfg.password {
        builder = builder.with_password(pw);
    }

    // Set application-level icon if the file exists
    let icon_path = resolve_icon_path(&cfg.icon);
    if let Ok(icon) = Resource::from_file(&icon_path) {
        builder = builder.with_icon(icon);
        debug!("Loaded app icon: {}", icon_path.display());
    } else {
        debug!(
            "App icon not found at {}; using no icon",
            icon_path.display()
        );
    }

    Ok(builder)
}

fn register_client(client: &mut GntpClient, cfg: &GrowlConfig) -> Result<()> {
    let icon_path = resolve_icon_path(&cfg.icon);

    let make_type = |id: &str, display: &str| {
        let mut t = NotificationType::new(id).with_display_name(display);
        if let Ok(icon) = Resource::from_file(&icon_path) {
            t = t.with_icon(icon);
        }
        t
    };

    let types = vec![
        make_type(NOTIF_LOW, "Low Battery Warning"),
        make_type(NOTIF_HIGH, "Battery Full"),
        make_type(NOTIF_TEMP, "High Temperature Warning"),
        make_type(NOTIF_HEALTH, "Low Battery Health"),
    ];

    // register() returns Result<String, GntpError> — discard the Ok value
    client
        .register(types)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{}", e))
}

fn resolve_icon_path(icon: &str) -> std::path::PathBuf {
    let p = Path::new(icon);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    // Look beside the binary first
    if let Ok(exe) = std::env::current_exe() {
        let beside = exe.parent().map(|d| d.join(icon));
        if let Some(ref b) = beside {
            if b.exists() {
                return b.clone();
            }
        }
    }
    // Then XDG data dir
    if let Some(data) = dirs::data_local_dir() {
        let xdg = data.join("battnux").join(icon);
        if xdg.exists() {
            return xdg;
        }
    }
    p.to_path_buf()
}
