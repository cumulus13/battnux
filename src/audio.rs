//! Audio alert thread.
//! Plays a sound file when a threshold breach event is received.
//! Runs in its own thread to avoid blocking the main loop.

use crate::{
    config::AudioConfig,
    threshold::{ThresholdEvent, ThresholdKind},
};
use anyhow::Result;
use log::{debug, error, info, warn};
use rodio::{Decoder, OutputStream, Sink};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

/// Handle for sending audio play requests to the background thread.
#[derive(Clone)]
pub struct AudioHandle {
    tx: Sender<ThresholdEvent>,
}

impl AudioHandle {
    pub fn send(&self, event: ThresholdEvent) {
        if let Err(e) = self.tx.send(event) {
            warn!("Audio channel closed: {}", e);
        }
    }
}

/// Spawn the audio alert background thread.
/// Returns `None` if audio is disabled or no alert file is configured.
pub fn spawn(cfg: AudioConfig) -> Option<AudioHandle> {
    if !cfg.enabled {
        info!("Audio alerts disabled in config");
        return None;
    }

    let alert_file = cfg.alert_file.clone()?;
    // Verify file exists before spawning thread
    if !std::path::Path::new(&alert_file).exists() {
        warn!(
            "Audio alert file not found: {}; audio alerts disabled",
            alert_file
        );
        return None;
    }

    let (tx, rx): (Sender<ThresholdEvent>, Receiver<ThresholdEvent>) = mpsc::channel();

    thread::Builder::new()
        .name("battnux-audio".into())
        .spawn(move || {
            audio_loop(cfg, rx);
        })
        .map_err(|e| error!("Failed to spawn audio thread: {}", e))
        .ok()?;

    Some(AudioHandle { tx })
}

// ─── Audio playback loop ─────────────────────────────────────────────────────

fn audio_loop(cfg: AudioConfig, rx: Receiver<ThresholdEvent>) {
    // Cooldown: don't play the same kind repeatedly
    let mut last_played: HashMap<ThresholdKind, Instant> = HashMap::new();
    let cooldown = Duration::from_secs(30);

    let alert_path = match &cfg.alert_file {
        Some(p) => p.clone(),
        None => return,
    };

    for event in rx {
        if let Some(last) = last_played.get(&event.kind) {
            if last.elapsed() < cooldown {
                debug!("Audio cooldown active for {:?}", event.kind);
                continue;
            }
        }

        match play_file(&alert_path, cfg.volume) {
            Ok(_) => {
                info!("Played audio alert: {}", alert_path);
                last_played.insert(event.kind, Instant::now());
            }
            Err(e) => {
                warn!("Audio playback failed: {}", e);
            }
        }
    }

    debug!("Audio thread exiting");
}

fn play_file(path: &str, volume: f32) -> Result<()> {
    let (_stream, stream_handle) =
        OutputStream::try_default().map_err(|e| anyhow::anyhow!("Audio device error: {}", e))?;

    let sink =
        Sink::try_new(&stream_handle).map_err(|e| anyhow::anyhow!("Audio sink error: {}", e))?;

    sink.set_volume(volume.clamp(0.0, 1.0));

    let file =
        File::open(path).map_err(|e| anyhow::anyhow!("Cannot open audio file {}: {}", path, e))?;

    let source =
        Decoder::new(BufReader::new(file)).map_err(|e| anyhow::anyhow!("Decode error: {}", e))?;

    sink.append(source);
    sink.sleep_until_end(); // block this thread until audio finishes
    Ok(())
}
