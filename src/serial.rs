use std::time::Duration;

use anyhow::{anyhow, Result};
use postcard::take_from_bytes_cobs;
use postcard_telemetry::transport;
use time::OffsetDateTime;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::broadcast::{Receiver, Sender},
    time::sleep,
};
use tokio_serial::SerialPortBuilderExt;

use crate::{telemetry::Frame, Command, Message};

pub async fn send_commands(
    path: &str,
    baud_rate: u32,
    mut rx: Receiver<Message>,
    tx: Sender<Message>,
) -> Result<()> {
    loop {
        match tokio_serial::new(path, baud_rate).open() {
            Ok(mut tty) => {
                if let Ok(Message::Command(Command::SendCommand(cmd))) = rx.recv().await {
                    let _ = match tty.write_all(cmd.as_bytes()) {
                        Ok(_) => tx.send(Message::Log(format!("[SYSTEM] Sent command: {cmd}"))),
                        Err(e) => tx.send(Message::Log(format!(
                            "[SYSTEM] Failed to send command: {cmd} due to {e}"
                        ))),
                    };
                }
            }
            _ => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn listen(path: &str, baud_rate: u32, message_bus: Sender<Message>) -> Result<()> {
    loop {
        match tokio_serial::new(path, baud_rate).open_native_async() {
            Ok(mut tty) => {
                let mut message_bytes: Vec<u8> = vec![];
                loop {
                    match read_serial(&mut tty, &mut message_bytes).await {
                        Ok(Some(transport::Package::Telemetry(frame))) => {
                            let internal_frame = Frame::new(
                                OffsetDateTime::now_local().unwrap(),
                                &frame
                                    .into_iter()
                                    .map(|(s, v)| match v {
                                        transport::telemetry::DataPoint::F32(n) => (s, n),
                                        transport::telemetry::DataPoint::U32(n) => (s, n as f32),
                                        transport::telemetry::DataPoint::I32(n) => (s, n as f32),
                                    })
                                    .collect::<Vec<_>>(),
                            );
                            if message_bus
                                .send(Message::Telemetry(internal_frame))
                                .is_err()
                            {
                                println!("[WARN] Message bus saturated, losing data")
                            }
                        }
                        Ok(Some(transport::Package::Log(log))) => {
                            if message_bus.send(Message::Log(format!("{log}"))).is_err() {
                                println!("[WARN] Message bus saturated, losing data")
                            }
                        }
                        Ok(None) => {
                            // TTY might be closed, return to outer loop.
                            break;
                        }
                        Err(e) => {
                            if message_bus
                                .send(Message::Log(format!("[WARN] {e}")))
                                .is_err()
                            {
                                println!("[WARN] Message bus saturated, losing data")
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = message_bus.send(Message::Log(format!(
                    "[WARN] TTY disconnected, waiting for connection... ({e})"
                )));
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Reads from the serial port and tries to parse a COBS-encoded frame.
async fn read_serial(
    tty: &mut (impl AsyncRead + std::marker::Unpin),
    message_bytes: &mut Vec<u8>,
) -> Result<Option<transport::Package>> {
    let mut buf = [0u8; 2048];

    if let Ok(n) = tty.read(&mut buf).await {
        message_bytes.extend_from_slice(&buf[..n]);
        match take_from_bytes_cobs::<transport::Package>(message_bytes) {
            Ok((package, rest)) => {
                *message_bytes = rest.to_vec();
                return Ok(Some(package));
            }
            Err(postcard::Error::DeserializeBadEncoding) => {
                // This can happen if we have some bytes but not
                // enough to assemble a full message, especially a
                // large one. In that case we should just continue
                // reading. If we encounter a zero byte though, the
                // message should have ended, so we discard the junk.
                if let Some(idx) = message_bytes.iter().position(|&b| b == 0) {
                    println!(
                        "[WARN] Got bad encoding: {}",
                        message_bytes
                            .iter()
                            .take(idx)
                            .map(|b| format!("{b:x?}"))
                            .collect::<String>()
                    );
                    message_bytes.drain(..idx);
                }
            }
            Err(postcard::Error::DeserializeUnexpectedEnd) => {
                // We hit these because zero bytes are the
                // end-of-package markers in COBS, but simulated
                // serial devices spit out zeroes if there's no actual
                // data. Just skip over the zeroes.
                match message_bytes.iter().position(|&b| b != 0) {
                    Some(idx) => {
                        message_bytes.drain(..idx);
                    }
                    None => {
                        message_bytes.clear();
                    }
                }
            }
            Err(e) => {
                return Err(anyhow!("Got an error reading serial device: {e}"));
            }
        }
    }
    Ok(None)
}
