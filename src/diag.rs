//! `S76FanControl.exe --diag [--test]`
//!
//! Writes a hardware report to `diag.txt` (next to the config file) and shows it.
//! With `--test` the fans are driven to 30 % and then 100 % for a few seconds
//! each, and handed back to the EC afterwards.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::controller::{LEGACY_REFRESH, percent_to_duty};
use crate::hw::{FanHardware, System76Hardware};
use crate::platform;

pub fn run(test: bool) -> i32 {
    let mut log = String::new();
    let start = Instant::now();
    let mut w = |s: String| {
        let _ = writeln!(log, "[{:6.2}s] {s}", start.elapsed().as_secs_f32());
    };

    let code = match System76Hardware::open() {
        Err(e) => {
            w(format!("ERROR: {e}"));
            1
        }
        Ok(mut hw) => {
            let i = hw.info().clone();
            w(format!(
                "EC board={} version={} protocol={} fan_mode={} rpm_cmd={}",
                i.board, i.version, i.protocol, i.has_fan_mode, i.has_rpm_cmd
            ));
            for _ in 0..3 {
                w(snapshot(&mut hw));
                std::thread::sleep(Duration::from_millis(500));
            }
            if test {
                for pct in [30.0, 100.0] {
                    w(format!("--- {pct} % ---"));
                    let duty = percent_to_duty(pct);
                    let t0 = Instant::now();
                    let mut last_log = Instant::now();
                    while t0.elapsed() < Duration::from_secs(10) {
                        if let Err(e) = hw.set_duty(duty) {
                            w(format!("ERROR: {e}"));
                            break;
                        }
                        if last_log.elapsed() > Duration::from_secs(3) {
                            w(snapshot(&mut hw));
                            last_log = Instant::now();
                        }
                        std::thread::sleep(if i.has_fan_mode { Duration::from_millis(500) } else { LEGACY_REFRESH });
                    }
                    w(snapshot(&mut hw));
                }
                match hw.set_auto() {
                    Ok(()) => w("back to auto".into()),
                    Err(e) => w(format!("ERROR returning to auto: {e}")),
                }
                std::thread::sleep(Duration::from_secs(2));
                w(snapshot(&mut hw));
            }
            0
        }
    };

    let path = Config::path().with_file_name("diag.txt");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, &log);
    platform::message_box("S76 Fan Control – diagnostics", &format!("{log}\nSaved to {}", path.display()), code != 0);
    code
}

fn snapshot(hw: &mut System76Hardware) -> String {
    let t = hw.read_temps();
    let f = hw.read_fans();
    let o = |v: Option<u8>| v.map_or("-".into(), |v| v.to_string());
    let r = |v: Option<u16>| v.map_or("-".into(), |v| v.to_string());
    format!(
        "CPU={}°C GPU={}°C | fan1 pwm={} rpm={} | fan2 pwm={} rpm={}",
        o(t.cpu),
        o(t.gpu),
        o(f[0].pwm),
        r(f[0].rpm),
        o(f[1].pwm),
        r(f[1].rpm)
    )
}
