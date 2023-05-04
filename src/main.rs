use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tokio::{sync::broadcast, task};

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
pub enum Message {
    Command(Command),
    Telemetry(Frame),
    Log,
}

#[derive(Clone, Debug)]
pub enum Command {
    SendCommand(String),
    Export,
    Reset,
    Exit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::load_config(&args.config)?;

    let (serial_tx, recorder_rx) = broadcast::channel::<Message>(128);
    let gui_tx = serial_tx.clone();
    let gui_rx = serial_tx.subscribe();
    let serial_rx = serial_tx.subscribe();

    let mut recorder = Recorder::new();
    task::spawn(async move { recorder.run(recorder_rx).await });

    let baud_rate = config.serial.baud;

    let serial_path = config.serial.path.clone();
    task::spawn(async move {
        serial::send_commands(&serial_path, baud_rate, serial_rx)
            .await
            .expect("failed to open serial port for sending commands")
    });

    let serial_path = config.serial.path.clone();
    task::spawn(async move {
        serial::listen(&serial_path, baud_rate, serial_tx)
            .await
            .expect("failed to open serial port for listening")
    });

    gui::run(config, gui_rx, gui_tx)?;

    Ok(())
}

// Use this patch for serial-rs to support fake serial ports on macOS
// #[cfg(any(target_os = "ios", target_os = "macos"))]
// pub fn iossiospeed(fd: RawFd, baud_rate: &libc::speed_t) -> Result<()> {
//     match unsafe { raw::iossiospeed(fd, baud_rate) } {
//         Ok(_) => Ok(()),
//         Err(nix::errno::Errno::ENOTTY) => Ok(()),
//         Err(e) => Err(e.into()),
//     }
// }
