use std::path::PathBuf;

use anyhow::Result;
use bus::Bus;
use clap::Parser;

mod config;
mod gui;
mod serial;
mod telemetry;

use telemetry::Frame;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config file path
    #[clap(short, long, default_value = "mctl.toml")]
    config: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut bus = Bus::<Frame>::new(1024);
    let rx = bus.add_rx();

    let config = config::load_config(&args.config)?;

    let second_config = config.clone();

    std::thread::spawn(move || {
        serial::listen(
            second_config.serial.path.clone().into(),
            second_config.serial.baud,
            bus,
        )
    });

    gui::run(config, rx);

    Ok(())
}

// TODO: Figure out a good way to keep track of time that works with the graphs.
// Need to keep all original timestamps for persisting to disk
// But need a continuous stream of data for the graphs
