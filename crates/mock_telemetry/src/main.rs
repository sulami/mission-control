use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use embedded_imu::{
    log::Log,
    telemetry::TelemetryReporter,
    transport::{encode, Package},
};
use random::Source;
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
        reporter.record("ctr", counter as f32).unwrap();
        reporter.record("volt", 3.3 + rand.read::<f32>()).unwrap();
        reporter.record("gx", 0.5 - rand.read::<f32>()).unwrap();
        reporter.record("gy", 0.5 - rand.read::<f32>()).unwrap();
        reporter.record("gz", 9.3 + rand.read::<f32>()).unwrap();
        reporter
            .record("vot", (Instant::now() - start).as_secs_f32())
            .unwrap();
        reporter
            .record("sin", (Instant::now() - start).as_secs_f32().sin())
            .unwrap();
        reporter
            .record("cos", (Instant::now() - start).as_secs_f32().cos())
            .unwrap();
        reporter
            .record("tan", (Instant::now() - start).as_secs_f32().tan())
            .unwrap();
        let report = reporter.report();
        tty.write_all(encode(&Package::Telemetry(report), &mut [0u8; 1024]).unwrap())?;
        tty.write_all(
            encode(
                &Package::Log::<32>(Log::info("sent report")),
                &mut [0u8; 128],
            )
            .unwrap(),
        )?;
        tty.flush().unwrap();
        counter += 1;
        thread::sleep(Duration::from_millis(50));
    }
}
