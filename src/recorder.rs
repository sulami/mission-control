use anyhow::{Context, Result};
use csv::Writer;
use time::{macros::format_description, OffsetDateTime};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::{telemetry::Frame, Command, Message};

/// At some point we'll run out of memory, so flush to disk every now
/// and then.
const MAX_FRAMES: usize = 90_000;

pub struct Recorder {
    frames: Vec<Frame>,
}

impl Recorder {
    pub fn new() -> Self {
        Self { frames: vec![] }
    }

    pub async fn run(&mut self, mut rx: Receiver<Message>, tx: Sender<Message>) {
        loop {
            if let Ok(msg) = rx.recv().await {
                match msg {
                    Message::Telemetry(frame) => {
                        self.frames.push(frame);
                        if self.frames.len() >= MAX_FRAMES {
                            match self.export() {
                                Ok(_) => {
                                    let _ = tx.send(Message::Log(
                                        "[SYSTEM] Auto-exported data".to_string(),
                                    ));
                                    self.reset();
                                }
                                Err(e) => {
                                    let _ = tx.send(Message::Log(format!(
                                        "[SYSTEM] Failed to auto-export data: {e}"
                                    )));
                                }
                            }
                        }
                    }
                    Message::Command(cmd) => match cmd {
                        Command::Export => match self.export() {
                            Ok(path) => {
                                let _ = tx.send(Message::Log(format!(
                                    "[SYSTEM] Exported data to {path}"
                                )));
                            }
                            Err(e) => {
                                let _ = tx.send(Message::Log(format!(
                                    "[SYSTEM] Failed to export data: {e}"
                                )));
                            }
                        },
                        Command::Reset => {
                            self.reset();
                        }
                        Command::Exit => {
                            return;
                        }
                        _ => {}
                    },
                    Message::Log(log) => {
                        println!("{log}");
                    }
                }
            }
        }
    }

    fn reset(&mut self) {
        self.frames.clear();
    }

    fn export(&self) -> Result<String> {
        let path = format!(
            "mctl-{}.csv",
            OffsetDateTime::now_local()
                .unwrap()
                .format(&format_description!(
                    "[year]-[month]-[day]T[hour]:[minute]:[second]"
                ))
                .unwrap()
        );
        let mut wtr =
            Writer::from_path(&path).with_context(|| format!("Failed to open file: {}", path))?;

        let mut headers = vec!["timestamp".to_string()];
        for frame in &self.frames {
            for data_point in &frame.data_points {
                if !headers.contains(&data_point.name) {
                    headers.push(data_point.name.clone());
                }
            }
        }
        wtr.write_record(&headers)?;

        for frame in &self.frames {
            let mut record = vec![frame
                .timestamp
                .format(&format_description!(
                    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]"
                ))
                .unwrap()];
            for data_point in &frame.data_points {
                let index = headers.iter().position(|h| h == &data_point.name).unwrap();
                while record.len() <= index {
                    record.push("".to_string());
                }
                record[index] = data_point.value.to_string();
            }
            wtr.write_record(record)?;
        }

        wtr.flush()?;

        Ok(path)
    }
}
