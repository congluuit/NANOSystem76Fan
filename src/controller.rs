//! The fan control loop.
//!
//! A dedicated thread owns the hardware. The UI talks to it through a channel
//! ([`Command`]) and reads a shared [`Status`] snapshot.
//!
//! Safety rules, enforced by [`Governor`]:
//! - above `safety_temp` control is handed back to the EC until the temperature
//!   has dropped by [`SAFETY_HYSTERESIS`];
//! - if the CPU temperature cannot be read [`MAX_FAILED_READS`] times in a row,
//!   control is handed back to the EC;
//! - when the thread exits (normally or by panic) the EC is put back in auto.

use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::config::{Config, Mode};
use crate::curve;
use crate::hw::{EcInfo, FAN_COUNT, FanHardware, FanReading, Temps};

pub const TICK: Duration = Duration::from_secs(1);
/// Rewrite interval on firmware without fan mode (the EC overwrites the PWM every 250 ms–1 s).
pub const LEGACY_REFRESH: Duration = Duration::from_millis(40);
pub const MAX_FAILED_READS: u32 = 5;
pub const SAFETY_HYSTERESIS: f32 = 5.0;
pub const HISTORY_LEN: usize = 300;
/// A failed read keeps showing the previous value for this long (display only;
/// the safety logic always sees the raw readings).
pub const HOLD: Duration = Duration::from_secs(5);

/// Keeps the last good value of a reading for [`HOLD`].
#[derive(Debug, Clone, Copy)]
struct Hold<T: Copy> {
    value: Option<T>,
    at: Option<Instant>,
}

impl<T: Copy> Default for Hold<T> {
    fn default() -> Self {
        Self { value: None, at: None }
    }
}

impl<T: Copy> Hold<T> {
    fn update(&mut self, new: Option<T>, now: Instant) -> Option<T> {
        match new {
            Some(v) => {
                self.value = Some(v);
                self.at = Some(now);
                Some(v)
            }
            None if self.at.is_some_and(|t| now.duration_since(t) < HOLD) => self.value,
            None => None,
        }
    }
}

#[derive(Debug, Default)]
struct Display {
    cpu: Hold<u8>,
    gpu: Hold<u8>,
    pwm: [Hold<u8>; FAN_COUNT],
    rpm: [Hold<u16>; FAN_COUNT],
}

impl Display {
    fn apply(&mut self, temps: Temps, fans: [FanReading; FAN_COUNT]) -> (Temps, [FanReading; FAN_COUNT]) {
        let now = Instant::now();
        let temps = Temps { cpu: self.cpu.update(temps.cpu, now), gpu: self.gpu.update(temps.gpu, now) };
        let fans = std::array::from_fn(|i| FanReading {
            pwm: self.pwm[i].update(fans[i].pwm, now),
            rpm: self.rpm[i].update(fans[i].rpm, now),
        });
        (temps, fans)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoReason {
    UserChoice,
    OverTemp(u8),
    NoTemperature,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    /// The EC firmware is in control.
    Auto(AutoReason),
    /// The tool drives the fans at this percentage.
    Manual(f32),
}

/// Pure decision logic, independent of hardware and time source.
#[derive(Debug, Default)]
pub struct Governor {
    current: Option<f32>,
    last_temp: Option<u8>,
    failed_reads: u32,
    tripped: bool,
}

impl Governor {
    pub fn step(&mut self, cfg: &Config, temps: Temps, dt: f32) -> Decision {
        if temps.cpu.is_some() {
            self.failed_reads = 0;
        } else {
            self.failed_reads += 1;
        }
        if let Some(t) = temps.hottest() {
            self.last_temp = Some(t);
        }

        if cfg.mode == Mode::Auto {
            self.current = None;
            self.tripped = false;
            return Decision::Auto(AutoReason::UserChoice);
        }
        if self.failed_reads >= MAX_FAILED_READS {
            self.current = None;
            return Decision::Auto(AutoReason::NoTemperature);
        }

        let Some(temp) = self.last_temp else {
            // Nothing measured yet: be conservative.
            self.current = Some(100.0);
            return Decision::Manual(100.0);
        };
        let tf = f32::from(temp);
        if tf >= cfg.safety_temp {
            self.tripped = true;
        } else if tf < cfg.safety_temp - SAFETY_HYSTERESIS {
            self.tripped = false;
        }
        if self.tripped {
            self.current = None;
            return Decision::Auto(AutoReason::OverTemp(temp));
        }

        let target = match cfg.mode {
            Mode::Fixed => cfg.fixed_percent,
            _ => curve::evaluate(&cfg.curve, tf),
        }
        .max(cfg.min_percent)
        .clamp(0.0, 100.0);

        let next = match (cfg.mode, self.current) {
            (Mode::Curve, Some(cur)) if target < cur => (cur - cfg.ramp_down_per_sec * dt).max(target),
            _ => target,
        };
        self.current = Some(next);
        Decision::Manual(next)
    }
}

pub fn percent_to_duty(percent: f32) -> u8 {
    (percent.clamp(0.0, 100.0) * 255.0 / 100.0).round() as u8
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Sample {
    /// Seconds since the controller started.
    pub t: f32,
    pub cpu: Option<u8>,
    pub gpu: Option<u8>,
    pub duty_percent: [Option<f32>; FAN_COUNT],
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Connecting,
    Running,
    /// Could not reach the EC; the message explains why.
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Status {
    pub phase: Phase,
    pub info: Option<EcInfo>,
    pub temps: Temps,
    pub fans: [FanReading; FAN_COUNT],
    pub decision: Option<Decision>,
    pub last_error: Option<String>,
    pub history: VecDeque<Sample>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            phase: Phase::Connecting,
            info: None,
            temps: Temps::default(),
            fans: [FanReading::default(); FAN_COUNT],
            decision: None,
            last_error: None,
            history: VecDeque::with_capacity(HISTORY_LEN),
        }
    }
}

pub enum Command {
    SetConfig(Box<Config>),
    Retry,
    Shutdown,
}

pub type Opener = dyn Fn() -> Result<Box<dyn FanHardware>, String> + Send;
pub type Notify = Arc<dyn Fn() + Send + Sync>;

pub struct Controller {
    tx: Sender<Command>,
    status: Arc<Mutex<Status>>,
    handle: Option<JoinHandle<()>>,
}

impl Controller {
    pub fn spawn(open: Box<Opener>, cfg: Config, notify: Notify) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(Status::default()));
        let shared = status.clone();
        let handle = std::thread::Builder::new()
            .name("fan-control".into())
            .spawn(move || thread_main(open, cfg, rx, shared, notify))
            .expect("spawn fan control thread");
        Self { tx, status, handle: Some(handle) }
    }

    pub fn status(&self) -> Status {
        lock(&self.status).clone()
    }

    pub fn set_config(&self, cfg: &Config) {
        let _ = self.tx.send(Command::SetConfig(Box::new(cfg.clone())));
    }

    pub fn retry(&self) {
        let _ = self.tx.send(Command::Retry);
    }

    /// Stops the loop and returns the fans to EC control. Blocks until done.
    pub fn shutdown(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn lock(m: &Mutex<Status>) -> std::sync::MutexGuard<'_, Status> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn thread_main(open: Box<Opener>, mut cfg: Config, rx: Receiver<Command>, status: Arc<Mutex<Status>>, notify: Notify) {
    loop {
        lock(&status).phase = Phase::Connecting;
        notify();
        match open() {
            Ok(mut hw) => {
                {
                    let mut s = lock(&status);
                    s.phase = Phase::Running;
                    s.info = Some(hw.info().clone());
                }
                let result = catch_unwind(AssertUnwindSafe(|| run(&mut *hw, &mut cfg, &rx, &status, &notify)));
                // Always leave the machine in a safe state.
                let _ = hw.set_auto();
                if let Err(panic) = result {
                    let msg = panic
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".into());
                    lock(&status).phase = Phase::Failed(format!("internal error: {msg}"));
                    notify();
                } else {
                    return; // shutdown requested
                }
            }
            Err(e) => {
                lock(&status).phase = Phase::Failed(e);
                notify();
            }
        }
        // Wait for the user to retry (or quit).
        loop {
            match rx.recv() {
                Ok(Command::SetConfig(c)) => cfg = *c,
                Ok(Command::Retry) => break,
                Ok(Command::Shutdown) | Err(_) => return,
            }
        }
    }
}

fn run(hw: &mut dyn FanHardware, cfg: &mut Config, rx: &Receiver<Command>, status: &Mutex<Status>, notify: &Notify) {
    let started = Instant::now();
    let legacy = !hw.info().has_fan_mode;
    let mut governor = Governor::default();
    let mut display = Display::default();
    let mut manual = false;
    let mut last_tick: Option<Instant> = None;
    let mut next_tick = Instant::now();

    loop {
        let now = Instant::now();
        if now >= next_tick {
            let dt = last_tick.map_or(1.0, |t| now.duration_since(t).as_secs_f32());
            last_tick = Some(now);
            next_tick = now + TICK;
            manual = tick(hw, cfg, &mut governor, &mut display, dt, started, status);
            notify();
        }

        let until_tick = next_tick.saturating_duration_since(Instant::now());
        let wait = if legacy && manual { until_tick.min(LEGACY_REFRESH) } else { until_tick };
        match rx.recv_timeout(wait) {
            Ok(Command::SetConfig(c)) => {
                *cfg = *c;
                next_tick = Instant::now(); // apply immediately
            }
            Ok(Command::Retry) => {}
            Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {
                if legacy && manual {
                    let _ = hw.refresh();
                }
            }
        }
    }
}

/// One control step. Returns whether the tool is in manual control afterwards.
fn tick(
    hw: &mut dyn FanHardware,
    cfg: &Config,
    governor: &mut Governor,
    display: &mut Display,
    dt: f32,
    started: Instant,
    status: &Mutex<Status>,
) -> bool {
    let temps = hw.read_temps();
    let decision = governor.step(cfg, temps, dt);
    let result = match decision {
        Decision::Auto(_) => hw.set_auto(),
        Decision::Manual(p) => hw.set_duty(percent_to_duty(p)),
    };
    let fans = hw.read_fans();
    let (temps, fans) = display.apply(temps, fans);

    let mut s = lock(status);
    s.temps = temps;
    s.fans = fans;
    s.decision = Some(decision);
    s.last_error = result.err();
    if s.history.len() >= HISTORY_LEN {
        s.history.pop_front();
    }
    s.history.push_back(Sample {
        t: started.elapsed().as_secs_f32(),
        cpu: temps.cpu,
        gpu: temps.gpu,
        duty_percent: fans.map(|f| f.pwm.map(|p| f32::from(p) * 100.0 / 255.0)),
    });
    matches!(decision, Decision::Manual(_)) && s.last_error.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::CurvePoint;

    fn temps(cpu: u8) -> Temps {
        Temps { cpu: Some(cpu), gpu: None }
    }

    fn curve_cfg() -> Config {
        Config {
            mode: Mode::Curve,
            curve: vec![CurvePoint::new(50.0, 20.0), CurvePoint::new(90.0, 100.0)],
            min_percent: 0.0,
            ramp_down_per_sec: 10.0,
            safety_temp: 92.0,
            ..Default::default()
        }
    }

    #[test]
    fn auto_mode_is_respected() {
        let cfg = Config { mode: Mode::Auto, ..Default::default() };
        assert_eq!(Governor::default().step(&cfg, temps(60), 1.0), Decision::Auto(AutoReason::UserChoice));
    }

    #[test]
    fn curve_follows_temperature_up_immediately() {
        let cfg = curve_cfg();
        let mut g = Governor::default();
        assert_eq!(g.step(&cfg, temps(50), 1.0), Decision::Manual(20.0));
        assert_eq!(g.step(&cfg, temps(90), 1.0), Decision::Manual(100.0));
    }

    #[test]
    fn curve_ramps_down_slowly() {
        let cfg = curve_cfg();
        let mut g = Governor::default();
        g.step(&cfg, temps(90), 1.0);
        assert_eq!(g.step(&cfg, temps(50), 1.0), Decision::Manual(90.0));
        assert_eq!(g.step(&cfg, temps(50), 2.0), Decision::Manual(70.0));
    }

    #[test]
    fn fixed_mode_changes_immediately_and_respects_min() {
        let mut cfg = Config { mode: Mode::Fixed, fixed_percent: 80.0, min_percent: 25.0, ..Default::default() };
        let mut g = Governor::default();
        assert_eq!(g.step(&cfg, temps(60), 1.0), Decision::Manual(80.0));
        cfg.fixed_percent = 10.0;
        assert_eq!(g.step(&cfg, temps(60), 1.0), Decision::Manual(25.0));
    }

    #[test]
    fn over_temperature_hands_back_with_hysteresis() {
        let cfg = curve_cfg();
        let mut g = Governor::default();
        assert_eq!(g.step(&cfg, temps(93), 1.0), Decision::Auto(AutoReason::OverTemp(93)));
        // Still within hysteresis band.
        assert_eq!(g.step(&cfg, temps(89), 1.0), Decision::Auto(AutoReason::OverTemp(89)));
        assert!(matches!(g.step(&cfg, temps(86), 1.0), Decision::Manual(_)));
    }

    #[test]
    fn gpu_temperature_counts() {
        let cfg = curve_cfg();
        let mut g = Governor::default();
        let t = Temps { cpu: Some(50), gpu: Some(90) };
        assert_eq!(g.step(&cfg, t, 1.0), Decision::Manual(100.0));
    }

    #[test]
    fn missing_temperature_falls_back_to_auto() {
        let cfg = curve_cfg();
        let mut g = Governor::default();
        g.step(&cfg, temps(60), 1.0);
        let none = Temps::default();
        for _ in 0..MAX_FAILED_READS - 1 {
            assert!(matches!(g.step(&cfg, none, 1.0), Decision::Manual(_)));
        }
        assert_eq!(g.step(&cfg, none, 1.0), Decision::Auto(AutoReason::NoTemperature));
        // Recovers once readings come back.
        assert!(matches!(g.step(&cfg, temps(60), 1.0), Decision::Manual(_)));
    }

    #[test]
    fn no_reading_yet_means_full_speed() {
        let cfg = curve_cfg();
        assert_eq!(Governor::default().step(&cfg, Temps::default(), 1.0), Decision::Manual(100.0));
    }

    #[test]
    fn duty_conversion() {
        assert_eq!(percent_to_duty(0.0), 0);
        assert_eq!(percent_to_duty(100.0), 255);
        assert_eq!(percent_to_duty(50.0), 128);
        assert_eq!(percent_to_duty(150.0), 255);
    }

    /// Fake hardware used to test the threaded loop end to end.
    struct FakeHw {
        info: EcInfo,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl FanHardware for FakeHw {
        fn info(&self) -> &EcInfo {
            &self.info
        }
        fn read_temps(&mut self) -> Temps {
            temps(70)
        }
        fn read_fans(&mut self) -> [FanReading; FAN_COUNT] {
            [FanReading::default(); FAN_COUNT]
        }
        fn set_duty(&mut self, duty: u8) -> Result<(), String> {
            self.log.lock().unwrap().push(format!("duty {duty}"));
            Ok(())
        }
        fn set_auto(&mut self) -> Result<(), String> {
            self.log.lock().unwrap().push("auto".into());
            Ok(())
        }
        fn refresh(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn shutdown_returns_to_auto() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let l2 = log.clone();
        let info = EcInfo {
            board: "system76/test".into(),
            version: "t".into(),
            protocol: 1,
            has_fan_mode: true,
            has_rpm_cmd: true,
        };
        let mut c = Controller::spawn(
            Box::new(move || Ok(Box::new(FakeHw { info: info.clone(), log: l2.clone() }) as Box<dyn FanHardware>)),
            Config { mode: Mode::Fixed, fixed_percent: 50.0, ..Default::default() },
            Arc::new(|| {}),
        );
        std::thread::sleep(Duration::from_millis(100));
        c.shutdown();
        let log = log.lock().unwrap();
        assert_eq!(log.first().map(String::as_str), Some("duty 128"));
        assert_eq!(log.last().map(String::as_str), Some("auto"));
    }
}
