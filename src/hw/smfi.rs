//! System76 EC command interface ("SMFI"), I/O ports 0xE00..0xEFF.
//!
//! Protocol, as implemented by <https://github.com/system76/ec>
//! (`src/app/main/smfi.c`, `tools/system76_ectool`):
//! 1. wait until the command byte (0xE00) reads 0,
//! 2. write the arguments to 0xE02..,
//! 3. write the command number to 0xE00; the EC executes it and clears 0xE00,
//! 4. read the result code at 0xE01 (0 = OK) and the returned data at 0xE02...

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::portio::PortIo;

const CMD_BASE: u16 = 0xE00;
const OFF_CMD: u16 = 0x00;
const OFF_RES: u16 = 0x01;
const OFF_DATA: u16 = 0x02;
pub const DATA_SIZE: usize = 0x100 - OFF_DATA as usize;

const RES_OK: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Cmd {
    Probe = 1,
    Board = 2,
    Version = 3,
    FanGetPwm = 7,
    FanSetPwm = 8,
    /// Added to the firmware in 2025 together with the fan mode commands.
    FanGetRpm = 22,
    FanGetMode = 23,
    FanSetMode = 24,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmfiError {
    Busy,
    Timeout(Cmd),
    /// The EC returned a non-zero result; usually means "unknown command" on older firmware.
    Failed(Cmd, u8),
    BadSignature(u8, u8),
}

impl std::fmt::Display for SmfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => write!(f, "EC is busy"),
            Self::Timeout(c) => write!(f, "EC did not answer {c:?}"),
            Self::Failed(c, r) => write!(f, "EC returned error {r} for {c:?}"),
            Self::BadSignature(a, b) => {
                write!(f, "not a System76 EC (signature {a:02X}{b:02X}, expected 76EC)")
            }
        }
    }
}

impl std::error::Error for SmfiError {}

pub struct Smfi {
    io: Arc<PortIo>,
}

impl Smfi {
    pub fn new(io: Arc<PortIo>) -> Self {
        Self { io }
    }

    /// Returns the protocol version if a System76 EC answers.
    pub fn probe(&self) -> Result<u8, SmfiError> {
        let mut data = [0u8; 3];
        self.command(Cmd::Probe, &mut data)?;
        if data[0] != 0x76 || data[1] != 0xEC {
            return Err(SmfiError::BadSignature(data[0], data[1]));
        }
        Ok(data[2])
    }

    pub fn read_string(&self, cmd: Cmd) -> Result<String, SmfiError> {
        let mut data = [0u8; DATA_SIZE];
        self.command(cmd, &mut data)?;
        let len = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        Ok(String::from_utf8_lossy(&data[..len]).into_owned())
    }

    pub fn fan_get_pwm(&self, fan: u8) -> Result<u8, SmfiError> {
        let mut data = [fan, 0];
        self.command(Cmd::FanGetPwm, &mut data)?;
        Ok(data[1])
    }

    pub fn fan_set_pwm(&self, fan: u8, duty: u8) -> Result<(), SmfiError> {
        self.command(Cmd::FanSetPwm, &mut [fan, duty])
    }

    pub fn fan_get_rpm(&self, fan: u8) -> Result<u16, SmfiError> {
        let mut data = [fan, 0, 0];
        self.command(Cmd::FanGetRpm, &mut data)?;
        Ok(u16::from_le_bytes([data[1], data[2]]))
    }

    /// 0 = automatic (EC curve), 1 = PWM set by the host.
    pub fn fan_get_mode(&self) -> Result<u8, SmfiError> {
        let mut data = [0u8; 1];
        self.command(Cmd::FanGetMode, &mut data)?;
        Ok(data[0])
    }

    pub fn fan_set_mode(&self, mode: u8) -> Result<(), SmfiError> {
        self.command(Cmd::FanSetMode, &mut [mode])
    }

    fn command(&self, cmd: Cmd, data: &mut [u8]) -> Result<(), SmfiError> {
        debug_assert!(data.len() <= DATA_SIZE);

        // The previous command must have completed.
        if !self.wait_idle(Duration::from_millis(50)) {
            return Err(SmfiError::Busy);
        }
        for (i, b) in data.iter().enumerate() {
            self.io.outb(CMD_BASE + OFF_DATA + i as u16, *b);
        }
        self.io.outb(CMD_BASE + OFF_CMD, cmd as u8);

        if !self.wait_idle(Duration::from_millis(200)) {
            return Err(SmfiError::Timeout(cmd));
        }
        for (i, b) in data.iter_mut().enumerate() {
            *b = self.io.inb(CMD_BASE + OFF_DATA + i as u16);
        }
        match self.io.inb(CMD_BASE + OFF_RES) {
            RES_OK => Ok(()),
            res => Err(SmfiError::Failed(cmd, res)),
        }
    }

    fn wait_idle(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while self.io.inb(CMD_BASE + OFF_CMD) != 0 {
            if start.elapsed() > timeout {
                return false;
            }
            std::hint::spin_loop();
        }
        true
    }
}
