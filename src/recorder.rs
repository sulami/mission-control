use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use bus::BusReader;
use csv::Writer;
use time::{macros::format_description, OffsetDateTime};

use crate::telemetry::Frame;
use crate::Command;

pub struct Recorder {
    frames: Vec<Frame>,
}

impl Recorder {
    pub fn new() -> Self {
        Self { frames: vec![] }
    }

    pub fn run(&mut self, mut message_bus: BusReader<Frame>, mut command_bus: BusReader<Command>) {
        loop {
            if let Ok(frame) = message_bus.try_recv() {
                self.frames.push(frame);
            }
            match command_bus.try_recv() {
                Ok(Command::Export) => {
                    if self.export().is_err() {
                        println!("Failed to export data");
                    }
                }
                Ok(Command::Reset) => {
                    self.frames.clear();
                }
                Ok(Command::Exit) => {
                    break;
                }
                _ => {}
            }

            thread::sleep(Duration::from_millis(10));
        }
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
                    "[year]-[month]-[day]T[hour]:[minute]:[second]"
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
