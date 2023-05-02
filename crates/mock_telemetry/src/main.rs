use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use embedded_imu::{log::Log, telemetry::TelemetryReporter, transport::encode};
use random::Source;
use serde::Serialize;
use serial_core::BaudRate::*;
use serial_core::SerialPort;
use serial_unix::TTYPort;

fn main() -> Result<()> {
    let path = PathBuf::from("/dev/ttys004");
    let mut rand = random::default(42);
    let mut reporter = TelemetryReporter::<10>::new();
    let mut tty = TTYPort::open(&path).expect("failed to open tty");
    tty.reconfigure(&|settings| settings.set_baud_rate(Baud9600))
        .expect("failed to setup tty");
    let start = Instant::now();
    let mut counter = 0;

    loop {
        reporter.record("ctr", counter as f32)?;
        reporter.record("volt", 3.3 + rand.read::<f32>())?;
        reporter.record("gx", 0.5 - rand.read::<f32>())?;
        reporter.record("gy", 0.5 - rand.read::<f32>())?;
        reporter.record("gz", 9.3 + rand.read::<f32>())?;
        reporter.record("vot", (Instant::now() - start).as_secs_f32())?;
        reporter.record("sin", (Instant::now() - start).as_secs_f32().sin())?;
        reporter.record("cos", (Instant::now() - start).as_secs_f32().cos())?;
        reporter.record("tan", (Instant::now() - start).as_secs_f32().tan())?;
        let report = reporter.report();
        tty.write_all(encode(&SerialMessage::Telemetry(report), &mut [0u8; 1024])?)?;
        tty.write_all(encode(
            &SerialMessage::LogMessage(Log::info("sent report")),
            &mut [0u8; 128],
        )?)?;
        tty.flush()?;
        counter += 1;
        thread::sleep(Duration::from_millis(50));
    }
}

#[derive(Debug, Clone, Serialize)]
enum SerialMessage {
    Telemetry(embedded_imu::telemetry::TelemetryFrame<10>),
    LogMessage(Log),
}
