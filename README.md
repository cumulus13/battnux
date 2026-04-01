# battnux 🔋

**Robust terminal battery monitor for Linux** (with cross-platform support for macOS and Windows).

## Features

- ⚡ **Detailed battery info** — charge %, state, health, voltage, power rate, temperature, cycle count, energy (Wh), technology, vendor, serial number
- 🎨 **Colored output** — green/yellow/red coded for charge, health, temperature
- 📊 **Terminal graph statistics** (`--stats`) — Unicode braille-style bar charts + sparklines for charge %, power rate, and health over time
- 🗂 **Automatic history logging** — every run appends a snapshot to `~/.local/share/battnux/history.jsonl`
- 🔧 **Verbose mode** (`--verbose`) — shows extra fields: voltage, temperature, vendor, serial number
- 🐛 **Debug mode** (`--debug`) — prints debug logs to stderr
- 📄 **JSON output** (`--json`) — pipe-friendly machine-readable output
- 🐧 **Linux-first**, also supports macOS and Windows via the `battery` crate

---

## Installation

### From crates.io (once published)

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

---

## Usage

```
battnux [OPTIONS]

Options:
  -s, --stats              Show battery statistics graph
  -v, --verbose            Verbose output with extra fields
  -d, --debug              Enable debug mode (logs to stderr)
  -j, --json               Output as JSON
  -n, --history <N>        Number of historical snapshots in --stats [default: 60]
  -h, --help               Print help
  -V, --version            Print version
```

### Examples

```bash
# Default: show battery info with colored output
battnux

# Show terminal graph stats with history
battnux --stats

# Extra fields (voltage, temperature, serial, etc.)
battnux --verbose

# Stats with verbose summary
battnux --stats --verbose

# JSON output (pipe to jq, scripts, etc.)
battnux --json | jq '.[] | .percentage'

# Last 120 data points in stats
battnux --stats --history 120

# Debug mode
battnux --debug
```

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

---

## Data Storage

History is stored as newline-delimited JSON at:

| Platform | Path |
|----------|------|
| Linux    | `~/.local/share/battnux/history.jsonl` |
| macOS    | `~/Library/Application Support/battnux/history.jsonl` |
| Windows  | `%APPDATA%\battnux\history.jsonl` |

Each line is a complete `BatterySnapshot` JSON record. You can query it with `jq`:

```bash
# See last 10 charge readings
tail -n 10 ~/.local/share/battnux/history.jsonl | jq '.percentage'
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `colored` | Terminal colors |
| `battery` | Cross-platform battery data |
| `chrono` | Timestamps |
| `serde` / `serde_json` | JSON serialization |
| `dirs` | Platform data directory |
| `terminal_size` | Adaptive graph width |
| `anyhow` / `thiserror` | Error handling |
| `log` / `env_logger` | Logging |

---

## License

MIT © Hadi Cahyadi

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)