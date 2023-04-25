use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

mod config;
mod gui;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config file path
    #[clap(short, long, default_value = "mctl.toml")]
    config: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config = config::load_config(&args.config)?;

    gui::run(config);

    Ok(())
}

// TODO: Figure out a good way to keep track of time that works with the graphs.
// Need to keep all original timestamps for persisting to disk
// But need a continuous stream of data for the graphs
