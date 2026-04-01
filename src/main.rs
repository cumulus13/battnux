mod battery_info;
mod cli;
mod display;
mod logger;
mod stats;

use anyhow::Result;
use cli::Cli;
use clap::Parser;
use log::{debug, info};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logger based on verbosity flags
    logger::init(cli.debug, cli.verbose)?;

    debug!("battnux starting, args parsed");
    info!("Collecting battery information");

    // Gather battery data
    let batteries = battery_info::collect()?;

    if batteries.is_empty() {
        eprintln!("{}", colored::Colorize::red("No battery detected on this system."));
        std::process::exit(1);
    }

    debug!("Found {} battery/batteries", batteries.len());

    // Persist snapshot to log file (always, for stats history)
    logger::persist_snapshot(&batteries)?;

    if cli.stats {
        // Show terminal graph statistics
        let history = logger::load_history(cli.history_limit)?;
        stats::render(&batteries, &history, cli.verbose)?;
    } else {
        // Default: colored textual info
        display::render(&batteries, cli.verbose, cli.json)?;
    }

    Ok(())
}
