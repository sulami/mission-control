use std::path::PathBuf;

use anyhow::Result;
use bus::Bus;
use clap::Parser;

mod config;
mod gui;
mod recorder;
mod serial;
mod telemetry;

use recorder::Recorder;
use telemetry::Frame;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config file path
    #[clap(short, long, default_value = "mctl.toml")]
    config: PathBuf,
}

#[derive(Clone, Debug)]
pub enum Command {
    SendCommand(String),
    Export,
    Reset,
    Exit,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::load_config(&args.config)?;

    let mut telemetry_bus = Bus::<Frame>::new(1024);
    let recorder_telemetry_rx = telemetry_bus.add_rx();
    let gui_telemetry_rx = telemetry_bus.add_rx();

    let mut command_bus = Bus::<Command>::new(16);
    let command_rx = command_bus.add_rx();

    let mut recorder = Recorder::new();
    std::thread::spawn(move || recorder.run(recorder_telemetry_rx, command_rx));

    let baud_rate = config.serial.baud;
    let command_rx = command_bus.add_rx();

    let serial_path = config.serial.path.clone();
    std::thread::spawn(move || serial::send_command(serial_path.into(), baud_rate, command_rx));

    let serial_path = config.serial.path.clone();
    std::thread::spawn(move || serial::listen(serial_path.into(), baud_rate, telemetry_bus));

    gui::run(config, gui_telemetry_rx, command_bus);

    Ok(())
}

// TODO: Format x-axes on plots.
