use std::io::prelude::*;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use bus::{Bus, BusReader};
use fxhash::FxHashMap;
use postcard::take_from_bytes_cobs;
use serial_core::BaudRate::{self, *};
use serial_core::SerialPort;
use serial_unix::TTYPort;
use time::OffsetDateTime;
use tokio::time::sleep;

use crate::telemetry::Frame;
use crate::Command;

pub async fn send_commands(
    path: PathBuf,
    baud_rate: usize,
    mut message_bus: BusReader<Command>,
) -> Result<()> {
    let baud = parse_baud_rate(baud_rate)?;
    loop {
        match TTYPort::open(&path) {
            Ok(mut tty) => {
                tty.reconfigure(&|settings| settings.set_baud_rate(baud))
                    .context("failed to configure TTY")?;
                message_bus.iter().for_each(|cmd| {
                    if let Command::SendCommand(cmd) = cmd {
                        println!("[INFO] Sending command: {cmd}");
                        tty.write_all(cmd.as_bytes()).unwrap();
                    }
                })
            }
            _ => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn listen(path: PathBuf, baud_rate: usize, mut message_bus: Bus<Frame>) -> Result<()> {
    let baud = parse_baud_rate(baud_rate)?;

    loop {
        match TTYPort::open(&path) {
            Ok(mut tty) => {
                tty.reconfigure(&|settings| settings.set_baud_rate(baud))
                    .context("failed to configure TTY")?;
                let mut message_bytes: Vec<u8> = vec![];
                loop {
                    if let Some(frame) = read_serial(&mut tty, &mut message_bytes) {
                        message_bus.broadcast(frame);
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            }
            _ => {
                thread::sleep(Duration::from_secs(1));
            }
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

/// Reads from the serial port and tries to parse a COBS-encoded frame.
fn read_serial(tty: &mut impl Read, message_bytes: &mut Vec<u8>) -> Option<Frame> {
    let mut buf = [0u8; 1024];

    if let Ok(n) = tty.read(&mut buf) {
        message_bytes.extend_from_slice(&buf[..n]);
        match take_from_bytes_cobs::<FxHashMap<String, f32>>(message_bytes) {
            Ok((parsed, rest)) => {
                let frame = Frame::new(
                    OffsetDateTime::now_utc(),
                    &parsed.into_iter().collect::<Vec<_>>(),
                );
                *message_bytes = rest.to_vec();
                return Some(frame);
            }
            Err(postcard::Error::DeserializeBadEncoding) => {
                return None;
            }
            Err(postcard::Error::DeserializeUnexpectedEnd) => {
                // NB We hit these because zero bytes are the
                // end-of-package markers in COBS, but serial devices
                // spit out zeroes if there's no actual data. Just
                // skip over the zeroes.
                match message_bytes.iter().position(|&b| b != 0) {
                    Some(idx) => {
                        message_bytes.drain(..idx);
                    }
                    None => {
                        message_bytes.clear();
                    }
                }
                return None;
            }
            Err(_) => {
                return None;
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use postcard::to_stdvec_cobs;

    #[test]
    fn test_read_serial_reads_a_frame() {
        let mut map = FxHashMap::default();
        map.insert("foo".to_string(), 1.0f32);
        map.insert("bar".to_string(), 2.0);
        map.insert("baz".to_string(), 3.0);

        let bytes = to_stdvec_cobs(&map).unwrap();
        let result = read_serial(&mut bytes.as_slice(), &mut vec![]);
        assert!(result.is_some());
    }
}
