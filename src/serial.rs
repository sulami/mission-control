use std::io::prelude::*;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use bus::{Bus, BusReader};
use fxhash::FxHashMap;
use postcard::from_bytes_cobs;
use serial_core::BaudRate::{self, *};
use serial_core::SerialPort;
use serial_unix::TTYPort;
use time::OffsetDateTime;

use crate::telemetry::Frame;

pub fn send_command(
    path: PathBuf,
    baud_rate: usize,
    mut message_bus: BusReader<String>,
) -> Result<()> {
    let baud = parse_baud_rate(baud_rate)?;
    let mut tty = TTYPort::open(&path)?;
    tty.reconfigure(&|settings| settings.set_baud_rate(baud))?;
    loop {
        if let Ok(cmd) = message_bus.try_recv() {
            tty.write_all(cmd.as_bytes())?;
        }
    }
}

pub fn listen(path: PathBuf, baud_rate: usize, mut message_bus: Bus<Frame>) -> Result<()> {
    let baud = parse_baud_rate(baud_rate)?;
    let mut buf = [0u8; 1024];
    loop {
        if let Ok(mut tty) = TTYPort::open(&path) {
            tty.reconfigure(&|settings| settings.set_baud_rate(baud))?;

            loop {
                thread::sleep(Duration::from_micros(100));
                buf.fill(0);
                if tty.read(&mut buf).is_ok() {
                    let ts = OffsetDateTime::now_local().unwrap();
                    if let Ok(parsed) = from_bytes_cobs::<FxHashMap<String, f32>>(&mut buf) {
                        let frame = Frame::new(
                            ts,
                            &parsed
                                .iter()
                                .map(|(s, v)| (s.clone(), *v))
                                .collect::<Vec<_>>(),
                        );
                        message_bus.broadcast(frame);
                    } else {
                        println!("[WARN] Got malformed package, ignoring",);
                    }
                }
            }
        } else {
            // Failed to open TTY.
            thread::sleep(Duration::from_secs(1));
            continue;
        }
    }
}

fn parse_baud_rate(b: usize) -> Result<BaudRate> {
    match b {
        110 => Ok(Baud110),
        300 => Ok(Baud300),
        600 => Ok(Baud600),
        1200 => Ok(Baud1200),
        2400 => Ok(Baud2400),
        4800 => Ok(Baud4800),
        9600 => Ok(Baud9600),
        19200 => Ok(Baud19200),
        38400 => Ok(Baud38400),
        57600 => Ok(Baud57600),
        115200 => Ok(Baud115200),
        _ => Err(anyhow!("unsupported baud rate")),
    }
}
