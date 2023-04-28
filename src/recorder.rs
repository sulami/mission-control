use anyhow::{Context, Result};
use csv::Writer;
use time::{macros::format_description, OffsetDateTime};
use tokio::{select, sync::broadcast::Receiver};

use crate::telemetry::Frame;
use crate::Command;

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

    pub async fn run(
        &mut self,
        mut message_bus: Receiver<Frame>,
        mut command_bus: Receiver<Command>,
    ) {
        loop {
            select! {
                Ok(frame) = message_bus.recv() => {
                    self.frames.push(frame);
                    if self.frames.len() >= MAX_FRAMES {
                        match self.export() {
                            Ok(_) => {
                                println!("[INFO] Auto-exported data");
                                self.reset();
                            }
                            Err(e) => {
                                println!("[WARN] Failed to auto-export data: {}", e);
                            }
                        }
                    }
                }
                Ok(cmd) = command_bus.recv() => {
                    match cmd {
                        Command::Export => {
                            if let Err(e) = self.export() {
                                println!("[WARN] Failed to export data: {}", e);
                            }
                        }
                        Command::Reset => {
                            self.reset();
                        }
                        Command::Exit => {
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn reset(&mut self) {
        self.frames.clear();
    }

    fn export(&self) -> Result<()> {
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
                    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]]"
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

        Ok(())
    }
}
