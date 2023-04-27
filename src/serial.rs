use std::io::prelude::*;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use bus::{Bus, BusReader};
use fxhash::FxHashMap;
use postcard::take_from_bytes_cobs;
use serial_core::BaudRate::{self, *};
use serial_core::SerialPort;
use serial_unix::TTYPort;
use time::OffsetDateTime;

use crate::telemetry::Frame;
use crate::Command;

pub fn send_command(
    path: PathBuf,
    baud_rate: usize,
    mut message_bus: BusReader<Command>,
) -> Result<()> {
    let baud = parse_baud_rate(baud_rate)?;
    let mut tty = TTYPort::open(&path)?;
    tty.reconfigure(&|settings| settings.set_baud_rate(baud))?;
    loop {
        if let Ok(Command::SendCommand(cmd)) = message_bus.try_recv() {
            tty.write_all(cmd.as_bytes())?;
        }
    }
}

pub fn listen(path: PathBuf, baud_rate: usize, mut message_bus: Bus<Frame>) -> Result<()> {
    let baud = parse_baud_rate(baud_rate)?;
    let mut read_buf = [0u8; 1024];
    let mut message_bytes: Vec<u8> = vec![];

    // Loop #1: Trying to open the TTY
    loop {
        if let Ok(mut tty) = TTYPort::open(&path) {
            tty.reconfigure(&|settings| settings.set_baud_rate(baud))?;

            // Loop #2: Reading from the TTY
            loop {
                if let Ok(bytes_read) = tty.read(&mut read_buf) {
                    // println!("read {} bytes", bytes_read);
                    // println!("read buf: {:?}", read_buf);
                    message_bytes.extend_from_slice(&read_buf[..bytes_read]);
                    // println!("message buf: {:?}", message_bytes);

                    // Loop #3: Parsing the message buffer
                    loop {
                        match take_from_bytes_cobs::<FxHashMap<String, f32>>(&mut message_bytes) {
                            Ok((parsed, rest)) => {
                                // println!(
                                //     "read {} bytes, got frame: {:?}, {} bytes left in message buffer: {:?}",
                                //     bytes_read,
                                //     parsed.get("ctr"),
                                //     rest.len(), rest,
                                // );
                                let frame = Frame::new(
                                    OffsetDateTime::now_local().unwrap(),
                                    &parsed.into_iter().collect::<Vec<_>>(),
                                );
                                message_bus.broadcast(frame);
                                message_bytes = rest.to_vec();
                            }
                            Err(postcard::Error::DeserializeBadEncoding) => {
                                // Not enough data to get out a COBS frame.
                                println!("bad encoding");
                                break;
                            }
                            Err(postcard::Error::DeserializeUnexpectedEnd) => {
                                message_bytes.clear();
                                break;
                            }
                            Err(_) => {
                                println!("[WARN] Got malformed package, ignoring",);
                                // message_bytes.clear();
                                break;
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_micros(500));
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
