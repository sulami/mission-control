use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use embedded_imu::telemetry::TelemetryReporter;
use random::Source;
use serial_core::BaudRate::*;
use serial_core::SerialPort;
use serial_unix::TTYPort;

fn main() {
    let path = PathBuf::from("/dev/ttys004");
    let mut rand = random::default(42);
    let mut reporter = TelemetryReporter::<10>::new();
    let mut tty = TTYPort::open(&path).expect("failed to open tty");
    tty.reconfigure(&|settings| settings.set_baud_rate(Baud9600))
        .expect("failed to setup tty");
    let start = Instant::now();

    loop {
        assert!(reporter.record("volt", 3.3 + rand.read::<f32>()));
        assert!(reporter.record("gx", 0.5 - rand.read::<f32>()));
        assert!(reporter.record("gy", 0.5 - rand.read::<f32>()));
        assert!(reporter.record("gz", 9.3 + rand.read::<f32>()));
        assert!(reporter.record("pot", (Instant::now() - start).as_secs_f32()));
        assert!(reporter.record("sin", (Instant::now() - start).as_secs_f32().sin()));
        let mut report = [0u8; 1024];
        assert!(reporter.report(&mut report));
        let _ = tty.write(&report).expect("failed to write telemetry");
        let _ = tty.flush();
        thread::sleep(Duration::from_millis(50));
    }
}
