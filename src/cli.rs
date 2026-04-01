use clap::Parser;

/// battnux — robust battery monitor for Linux (and cross-platform)
///
/// Displays detailed battery information with optional terminal statistics graphs.
/// Historical snapshots are automatically logged for trend analysis.
#[derive(Parser, Debug)]
#[command(
    name = "battnux",
    author = "Hadi Cahyadi <cumulus13@gmail.com>",
    version,
    about = "Robust terminal battery monitor with colored output and statistics",
    long_about = "battnux displays detailed battery information including charge level, \
                  health, voltage, temperature, cycle count, and estimated time remaining. \
                  Use --stats to visualize historical trends as ASCII/Unicode graphs.\n\n\
                  Homepage: https://github.com/cumulus13/battnux",
    after_help = "EXAMPLES:\n\
                  battnux              # Show battery info (default)\n\
                  battnux --stats      # Show graphs + history\n\
                  battnux --verbose    # Include extra fields\n\
                  battnux --json       # JSON output (pipe-friendly)\n\
                  battnux --debug      # Debug mode (verbose logging)"
)]
pub struct Cli {
    /// Show terminal ASCII/Unicode graph statistics from logged history
    #[arg(short = 's', long = "stats", help = "Show battery statistics graph")]
    pub stats: bool,

    /// Output extra fields: temperature, voltage, manufacture date, serial number
    #[arg(short = 'v', long = "verbose", help = "Verbose output with extra fields")]
    pub verbose: bool,

    /// Enable debug logging to stderr (implies verbose)
    #[arg(short = 'd', long = "debug", help = "Enable debug mode (logs to stderr)")]
    pub debug: bool,

    /// Output data as JSON instead of human-readable text
    #[arg(short = 'j', long = "json", help = "Output as JSON")]
    pub json: bool,

    /// Number of historical snapshots to include in stats view (default: 60)
    #[arg(
        short = 'n',
        long = "history",
        default_value = "60",
        help = "Number of historical snapshots to display in --stats"
    )]
    pub history_limit: usize,
}
