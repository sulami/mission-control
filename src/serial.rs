use std::time::Duration;

use anyhow::Result;
use embedded_imu::transport;
use postcard::take_from_bytes_cobs;
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
    mut message_bus: Receiver<Message>,
) -> Result<()> {
    loop {
        match tokio_serial::new(path, baud_rate).open() {
            Ok(mut tty) => {
                if let Ok(Message::Command(Command::SendCommand(cmd))) = message_bus.recv().await {
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

pub async fn listen(path: &str, baud_rate: u32, message_bus: Sender<Message>) -> Result<()> {
    loop {
        match tokio_serial::new(path, baud_rate).open_native_async() {
            Ok(mut tty) => {
                let mut message_bytes: Vec<u8> = vec![];
                loop {
                    match read_serial(&mut tty, &mut message_bytes).await {
                        Some(transport::Package::Telemetry(frame)) => {
                            let internal_frame = Frame::new(
                                OffsetDateTime::now_utc(),
                                &frame
                                    .into_iter()
                                    .filter_map(|(s, v)| {
                                        if let transport::telemetry::DataPoint::F32(f) = v {
                                            Some((s, f))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            );
                            if message_bus
                                .send(Message::Telemetry(internal_frame))
                                .is_err()
                            {
                                println!("[WARN] Telemetry buffer saturated, losing data")
                            }
                        }
                        Some(transport::Package::Log(_log)) => {}
                        None => {}
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
) -> Option<transport::Package> {
    let mut buf = [0u8; 1024];

    if let Ok(n) = tty.read(&mut buf).await {
        message_bytes.extend_from_slice(&buf[..n]);
        match take_from_bytes_cobs::<transport::Package>(message_bytes) {
            Ok((package, rest)) => {
                *message_bytes = rest.to_vec();
                return Some(package);
            }
            Err(postcard::Error::DeserializeBadEncoding) => {
                println!("[WARN] Received some bad data, skipping");
                message_bytes.clear();
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
            }
            Err(_) => {}
        }
    }
    None
}
