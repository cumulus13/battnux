# battnux 🔋

**Robust, production-ready terminal battery monitor** for Linux (with cross-platform support for macOS and Windows).

```
Author  : Hadi Cahyadi <cumulus13@gmail.com>
Repo    : https://github.com/cumulus13/battnux
License : MIT
Version : 1.0.7
```

[![CI](https://github.com/cumulus13/battnux/actions/workflows/ci.yml/badge.svg)](https://github.com/cumulus13/battnux/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/battnux.svg)](https://crates.io/crates/battnux)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Features

| Feature | Details |
|---|---|
| **Detailed battery info** | Charge %, state, health, voltage, power rate, temperature, cycle count, energy (Wh), vendor, serial |
| **Colored terminal output** | Green/yellow/red coded for charge, health, temperature; cyan for charging |
| **Real-time monitor mode** | `--monitor` — smooth alternate-screen refresh like htop, press `q` to quit |
| **Configurable interval** | `--interval N` or via config file |
| **Threshold warnings** | Low/high charge, high temperature, low health — printed in terminal |
| **Growl/GNTP notifications** | Sends desktop notifications via GNTP protocol with per-kind cooldown |
| **Audio alerts** | Plays a wav/mp3/ogg file when thresholds are breached |
| **History logging** | Every run appends a snapshot to `~/.local/share/battnux/history.jsonl` |
| **Terminal statistics** | `--stats` — Unicode braille bar charts + sparklines for charge %, power rate, health |
| **Live web dashboard** | `--web` — auto-refreshing browser dashboard with SSE push at `http://127.0.0.1:7878` |
| **TOML config file** | `~/.config/battnux/config.toml` — all thresholds, growl, audio, monitor, web settings |
| **JSON output** | `--json` — pipe-friendly machine-readable output |
| **Verbose & debug modes** | `--verbose` / `--debug` |
| **Thread-safe** | Notifier, audio, and web run in isolated threads; shared state protected by `parking_lot::Mutex` |
| **Linux-first** | Uses `/sys/class/power_supply` on Linux; also supports macOS (IOKit) and Windows (WMI) |

---

## Installation

### From crates.io

```bash
cargo install battnux
```

### From source

```bash
git clone https://github.com/cumulus13/battnux
cd battnux
cargo build --release
# Binary at: target/release/battnux
sudo cp target/release/battnux /usr/local/bin/
```

### Linux system dependencies (for audio support)

```bash
# Debian/Ubuntu
sudo apt-get install libasound2-dev pkg-config

# Fedora/RHEL
sudo dnf install alsa-lib-devel pkg-config

# Arch
sudo pacman -S alsa-lib pkg-config
```

---

## Usage

```
battnux [OPTIONS]

Options:
  -m, --monitor              Real-time monitor mode (press q to quit)
  -i, --interval <SECS>      Monitor refresh interval [overrides config]
  -s, --stats                Show historical battery statistics graph
  -v, --verbose              Extra fields (voltage, temperature, serial, vendor)
  -d, --debug                Debug logging to stderr
  -j, --json                 JSON output (one-shot mode)
  -w, --web                  Enable live web dashboard
      --web-port <PORT>      Web dashboard port [overrides config]
  -n, --history <N>          Snapshots shown in --stats [default: 60]
      --show-config          Print config file path and contents
  -h, --help                 Print help
  -V, --version              Print version
```

### Quick examples

```bash
battnux                       # One-shot battery info with colors
battnux --monitor             # Real-time monitor (5s interval, press q to quit)
battnux --monitor -i 2        # Monitor with 2s refresh
battnux --stats               # Historical graphs
battnux --monitor --stats     # Monitor with inline sparkline trend
battnux --verbose             # Include voltage, temperature, serial number
battnux --json                # JSON output — pipe to jq
battnux --web                 # Start web dashboard at http://127.0.0.1:7878
battnux --monitor --web       # Monitor mode + web dashboard simultaneously
battnux --show-config         # Print config file location and contents
battnux --debug               # Full debug logging
RUST_LOG=debug battnux        # Alternative: env-based log level
```

---

## Configuration

battnux auto-creates `~/.config/battnux/config.toml` with defaults on first run. Edit it to tune thresholds, Growl, audio, and web settings.

```toml
# ~/.config/battnux/config.toml

[thresholds]
low         = 20.0    # % — warn when discharging below this
high        = 95.0    # % — warn when charging above this
temperature = 55.0    # °C — warn when temperature exceeds this
health      = 50.0    # % — warn when battery health drops below this

[growl]
enabled       = true
host          = "localhost"
port          = 23053
# password    = "secret"
icon          = "battnux.png"   # place beside binary, or give full path
sticky        = false
cooldown_secs = 60              # minimum seconds between repeat notifications

[audio]
enabled    = true
alert_file = "/path/to/alert.wav"   # wav, mp3, or ogg
volume     = 1.0

[monitor]
interval_secs = 5
verbose       = false

[web]
enabled = false
host    = "127.0.0.1"
port    = 7878
```

All keys are optional — unset keys use the defaults shown above. See `assets/config.example.toml` for the full annotated reference.

---

## Sample Output

### Default (`battnux`)

```
  battnux — Battery Monitor
  ────────────────────────────────────────────────

  ─── Battery #0 — SN03055XL ────────────────────────────────────────────────
           Charge:  ████████████████████░░░░░░░░░░ 68.4%
            State:  Discharging
           Health:  94.2%
        Time Left:  3h 22m
           Energy:  35.12 Wh  /  51.34 Wh  (design: 54.52 Wh)
       Power Rate:  10.42 W
       Technology:  LithiumIon
      Cycle Count:  287
```

### Stats (`battnux --stats`)

```
  battnux — Battery Statistics
  ─── Charge % History (45 snapshots) ──────────────────

  100%│
   75%│██████████████████████████████████████████
   50%│
   25%│
     0%│
        └──────────────────────────────────────────
          oldest                              newest

  Trend: ▆▆▇▇▇▇████████████████████████████▇▇▆▅▄

  ─── Summary Statistics ────────────────────────
         Charge %:  min: 32.10  max: 98.50  avg: 71.30  last: 68.40
          Power W:  min: 0.00   max: 45.20  avg: 12.40  last: 10.42
```


## Growl Notification Setup

battnux uses the **GNTP** (Growl Notification Transport Protocol) via the `gntp` crate.

**Supported clients:**
- [Growl for macOS](http://growl.info/)
- [Growl for Windows](https://www.growlforwindows.com/)
- [Notification Center GNTP](https://github.com/nicklockwood/NotificationCenterGNTP) (iOS/macOS bridge)
- Any GNTP-compatible client

**Setup steps:**
1. Install and start your GNTP client
2. Enable network notifications in client settings
3. (Optional) Set a password and update `config.toml`
4. Place `battnux.png` beside the `battnux` binary (or set `icon` to a full path)
5. Run `battnux` — it registers notification types automatically on first send

**Notification types registered:**
- `low-battery` — charge at or below threshold (default 20%)
- `high-battery` — charge at or above threshold while charging (default 95%)
- `high-temperature` — battery temperature too high (default 55°C)
- `low-health` — battery capacity wear too high (default 50%)

To disable Growl: set `growl.enabled = false` in config.

---

## Audio Alert Setup

1. Obtain or create a `.wav`, `.mp3`, or `.ogg` alert sound
2. Set `audio.alert_file` in config to the full path
3. Optionally adjust `audio.volume` (0.0–1.0)
4. Set `audio.enabled = false` to disable without removing the path

The audio thread applies its own 30-second per-kind cooldown independently of the Growl cooldown.

---

## Web Dashboard

Start with `--web` or set `web.enabled = true` in config:

```bash
battnux --monitor --web
# Open http://127.0.0.1:7878 in your browser
```

**Routes:**

| Route | Description |
|---|---|
| `GET /` | Live HTML dashboard (auto-refreshes via SSE) |
| `GET /api/data` | Current JSON snapshot |
| `GET /events` | Server-Sent Events stream (push every interval) |

To expose on your LAN: set `web.host = "0.0.0.0"` in config.

---

## Data Storage

History is stored as newline-delimited JSON at:

| Platform | Path |
|---|---|
| Linux | `~/.local/share/battnux/history.jsonl` |
| macOS | `~/Library/Application Support/battnux/history.jsonl` |
| Windows | `%APPDATA%\battnux\history.jsonl` |

Each line is a complete JSON snapshot of one battery at one point in time.
Each line is a complete `BatterySnapshot` JSON record. You can query it with `jq`:

```bash
# Query history with jq
tail -n 20 ~/.local/share/battnux/history.jsonl | jq '.percentage'

# Average charge over last 100 readings
tail -n 100 ~/.local/share/battnux/history.jsonl | jq -s '[.[].percentage] | add/length'
```

---

## Thread Architecture

```
main thread
├── battery_info::collect()           — reads /sys/class/power_supply
├── threshold::evaluate()             — checks all thresholds
├── logger::persist_snapshot()        — appends to JSONL log
│
├── notifier thread  (mpsc::channel)  — receives ThresholdEvents, sends GNTP
│   └── GntpClient::notify()          — with per-kind cooldown & auto re-register
│
├── audio thread     (mpsc::channel)  — receives ThresholdEvents, plays sound
│   └── rodio::Sink                   — per-kind 30s cooldown
│
└── tokio task       (async)          — axum web server + SSE push
    └── SharedState  (parking_lot::Mutex<MonitorState>)
        └── updated by monitor loop, read by web handlers
```

All shared state is protected by `parking_lot::Mutex`. Channels are non-blocking (`try_send`-style). The monitor loop uses `crossterm` alternate screen for flicker-free refresh.

---

## GitHub Actions

| Workflow | Trigger | Jobs |
|---|---|---|
| `ci.yml` | Push / PR | fmt, clippy, test (Linux/macOS/Windows), MSRV, security audit |
| `release.yml` | Push `v*` tag | validate, build (5 targets), GitHub release, publish to crates.io |
| `deps.yml` | Weekly / manual | cargo upgrade, auto-PR |

**To publish a release:**

```bash
# 1. Update version in Cargo.toml
# 2. Commit, tag, push
git tag v0.2.0
git push origin v0.2.0
# Release workflow auto-builds binaries and publishes to crates.io
```

**Required secrets:**
- `CRATES_IO_TOKEN` — in the `crates-io` GitHub environment

---

## Building from source (development)

```bash
git clone https://github.com/cumulus13/battnux
cd battnux

# Debug build
cargo build

# Release build (optimized, LTO, stripped)
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt
```

---

## License

MIT © Hadi Cahyadi — see [LICENSE](LICENSE)

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)
