use std::time::Duration;

use anyhow::Result;
use fxhash::FxHashMap;
use postcard::take_from_bytes_cobs;
use time::OffsetDateTime;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::broadcast::{Receiver, Sender},
    time::sleep,
};
use tokio_serial::SerialPortBuilderExt;

use crate::telemetry::Frame;
use crate::Command;

pub async fn send_commands(
    path: &str,
    baud_rate: u32,
    mut message_bus: Receiver<Command>,
) -> Result<()> {
    loop {
        match tokio_serial::new(path, baud_rate).open() {
            Ok(mut tty) => {
                if let Ok(Command::SendCommand(cmd)) = message_bus.recv().await {
                    println!("[INFO] Sending command: {cmd}");
                    tty.write_all(cmd.as_bytes()).unwrap();
                }
            }
            _ => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn listen(path: &str, baud_rate: u32, message_bus: Sender<Frame>) -> Result<()> {
    loop {
        match tokio_serial::new(path, baud_rate).open_native_async() {
            Ok(mut tty) => {
                let mut message_bytes: Vec<u8> = vec![];
                loop {
                    if let Some(frame) = read_serial(&mut tty, &mut message_bytes).await {
                        if message_bus.send(frame).is_err() {
                            println!("[WARN] Telemetry buffer saturated, losing data")
                        }
                    }
                }
            }
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Reads from the serial port and tries to parse a COBS-encoded frame.
async fn read_serial(
    tty: &mut (impl AsyncRead + std::marker::Unpin),
    message_bytes: &mut Vec<u8>,
) -> Option<Frame> {
    let mut buf = [0u8; 1024];

    if let Ok(n) = tty.read(&mut buf).await {
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
