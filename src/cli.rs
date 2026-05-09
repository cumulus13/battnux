use clap::Parser;

/// battnux — robust battery monitor for Linux (and cross-platform)
#[derive(Parser, Debug)]
#[command(
    name = "battnux",
    author = "Hadi Cahyadi <cumulus13@gmail.com>",
    version,
    about = "Robust terminal battery monitor with monitor mode, graphs, Growl alerts, and web dashboard",
    long_about = "battnux shows detailed battery info with colors, runs in real-time monitor mode,\n\
                  logs history for statistics, sends Growl notifications on threshold breaches,\n\
                  plays audio alerts, and optionally hosts a live web dashboard.\n\n\
                  Homepage: https://github.com/cumulus13/battnux",
    after_help = "EXAMPLES:\n\
                  battnux                           # One-shot battery info\n\
                  battnux --monitor                 # Real-time refresh (default 5s)\n\
                  battnux --monitor --interval 10   # Custom interval\n\
                  battnux --config c:\\tools\\exe\\config.toml  # Use specific config file\n\
                  battnux --stats                   # Historical graph\n\
                  battnux --web                     # Enable web dashboard\n\
                  battnux --monitor --web           # Monitor + web dashboard\n\
                  battnux --verbose                 # Extra fields\n\
                  battnux --json                    # JSON output\n\
                  battnux --show-config             # Print config file path & contents\n\
                  RUST_LOG=debug battnux            # Override log level"
)]
pub struct Cli {
    /// Real-time monitor mode — refresh every interval seconds
    #[arg(short = 'm', long = "monitor", help = "Enable real-time monitor mode")]
    pub monitor: bool,

    /// Refresh interval in seconds for monitor mode (overrides config)
    #[arg(
        short = 'i',
        long = "interval",
        value_name = "SECS",
        help = "Monitor refresh interval in seconds [overrides config]"
    )]
    pub interval: Option<u64>,

    /// Show terminal ASCII/Unicode graph statistics from logged history
    #[arg(short = 's', long = "stats", help = "Show battery statistics graph")]
    pub stats: bool,

    /// Output extra fields: temperature, voltage, manufacture date, serial number
    #[arg(
        short = 'v',
        long = "verbose",
        help = "Verbose output with extra fields"
    )]
    pub verbose: bool,

    /// Enable debug logging to stderr
    #[arg(
        short = 'd',
        long = "debug",
        help = "Enable debug mode (logs to stderr)"
    )]
    pub debug: bool,

    /// Output data as JSON instead of human-readable text
    #[arg(
        short = 'j',
        long = "json",
        help = "Output as JSON (one-shot mode only)"
    )]
    pub json: bool,

    /// Start the web dashboard (uses config [web] host/port, default http://127.0.0.1:7878)
    #[arg(short = 'w', long = "web", help = "Enable live web dashboard")]
    pub web: bool,

    /// Web server port (overrides config)
    #[arg(
        long = "web-port",
        value_name = "PORT",
        help = "Web dashboard port [overrides config]"
    )]
    pub web_port: Option<u16>,

    /// Number of historical snapshots to include in --stats (default: 60)
    #[arg(
        short = 'n',
        long = "history",
        default_value = "60",
        help = "Number of historical snapshots in --stats"
    )]
    pub history_limit: usize,

    /// Path to a specific config file to use (overrides all automatic search locations)
    #[arg(
        short = 'c',
        long = "config",
        value_name = "PATH",
        help = "Path to config file [default: search beside binary, then platform config dir]"
    )]
    pub config: Option<std::path::PathBuf>,

    /// Print the config file path and current contents, then exit
    #[arg(long = "show-config", help = "Show config file location and contents")]
    pub show_config: bool,
}
