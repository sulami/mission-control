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
    let mut telemetry_bus = Bus::<Frame>::new(1024);
    let gui_telemetry_rx = telemetry_bus.add_rx();

    let mut command_bus = Bus::<String>::new(16);
    let command_rx = command_bus.add_rx();

    let config = config::load_config(&args.config)?;
    let baud_rate = config.serial.baud;

    let serial_path = config.serial.path.clone();
    std::thread::spawn(move || serial::send_command(serial_path.into(), baud_rate, command_rx));

    let serial_path = config.serial.path.clone();
    std::thread::spawn(move || serial::listen(serial_path.into(), baud_rate, telemetry_bus));

    gui::run(config, gui_telemetry_rx, command_bus);

    Ok(())
}

// TODO: Format x-axes on plots.

// TODO: Add a recorder that keeps all the data outside of the GUI bits.

// TODO: Add a CSV export from the recorder.
