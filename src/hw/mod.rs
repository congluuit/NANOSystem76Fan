//! Hardware access: the InpOut port I/O driver, the System76 EC command
//! interface and the EC's ACPI registers.

pub mod acpi;
pub mod portio;
pub mod smfi;

use std::sync::Arc;

use acpi::AcpiEc;
use portio::PortIo;
use smfi::{Cmd, Smfi};

pub const FAN_COUNT: usize = 2;

/// Static facts about the EC, detected once at start-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcInfo {
    /// e.g. `system76/addw3`
    pub board: String,
    /// EC firmware version, e.g. `2023-09-19_16ef69c`
    pub version: String,
    pub protocol: u8,
    /// Firmware has FAN_GET/SET_MODE (2025+). Without it the EC overwrites the
    /// PWM on every fan tick, so the host has to rewrite it continuously.
    pub has_fan_mode: bool,
    /// Firmware has FAN_GET_RPM; otherwise the raw tachometer is read via ACPI.
    pub has_rpm_cmd: bool,
}

impl EcInfo {
    /// Model code such as `addw3`.
    pub fn model(&self) -> &str {
        self.board.rsplit('/').next().unwrap_or(&self.board)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Temps {
    pub cpu: Option<u8>,
    /// `None` while the dGPU is powered off.
    pub gpu: Option<u8>,
}

impl Temps {
    pub fn hottest(&self) -> Option<u8> {
        match (self.cpu, self.gpu) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FanReading {
    /// Current duty, 0..=255.
    pub pwm: Option<u8>,
    pub rpm: Option<u16>,
}

/// Everything the controller needs from the machine. Abstracted so the control
/// logic can be unit-tested without hardware.
pub trait FanHardware: Send {
    fn info(&self) -> &EcInfo;
    fn read_temps(&mut self) -> Temps;
    fn read_fans(&mut self) -> [FanReading; FAN_COUNT];
    /// Applies `duty` (0..=255) to all fans, taking manual control if needed.
    fn set_duty(&mut self, duty: u8) -> Result<(), String>;
    /// Hands control back to the EC's built-in curve.
    fn set_auto(&mut self) -> Result<(), String>;
    /// Re-sends the last duty. Only needed on firmware without fan mode.
    fn refresh(&mut self) -> Result<(), String>;
}

pub struct System76Hardware {
    smfi: Smfi,
    acpi: AcpiEc,
    info: EcInfo,
    manual: bool,
    duty: u8,
}

impl System76Hardware {
    pub fn open() -> Result<Self, String> {
        let io = Arc::new(PortIo::open().map_err(|e| e.to_string())?);
        let smfi = Smfi::new(io.clone());
        let protocol = smfi.probe().map_err(|e| e.to_string())?;
        let info = EcInfo {
            board: smfi.read_string(Cmd::Board).unwrap_or_else(|_| "?".into()),
            version: smfi.read_string(Cmd::Version).unwrap_or_else(|_| "?".into()),
            protocol,
            has_fan_mode: smfi.fan_get_mode().is_ok(),
            has_rpm_cmd: smfi.fan_get_rpm(0).is_ok(),
        };
        Ok(Self { smfi, acpi: AcpiEc::new(io), info, manual: false, duty: 255 })
    }

    fn rpm(&self, fan: u8) -> Option<u16> {
        if self.info.has_rpm_cmd {
            return self.smfi.fan_get_rpm(fan).ok();
        }
        let reg = if fan == 0 { acpi::REG_FAN1_TACH } else { acpi::REG_FAN2_TACH };
        self.acpi.read_u16(reg).map(tach_to_rpm)
    }

    fn write_all(&self) -> Result<(), String> {
        for fan in 0..FAN_COUNT as u8 {
            self.smfi.fan_set_pwm(fan, self.duty).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// Converts the raw ITE tachometer count to RPM, matching `TACH_TO_RPM` in the
/// firmware: `60 * f_EC / 128 / 2 / count` with f_EC = 9.2 MHz.
pub fn tach_to_rpm(count: u16) -> u16 {
    if count == 0 {
        return 0;
    }
    (60u64 * 9_200_000 / 128 / 2 / u64::from(count)).min(u64::from(u16::MAX)) as u16
}

fn valid_temp(v: Option<u8>) -> Option<u8> {
    v.filter(|t| (1..120).contains(t))
}

impl FanHardware for System76Hardware {
    fn info(&self) -> &EcInfo {
        &self.info
    }

    fn read_temps(&mut self) -> Temps {
        Temps {
            cpu: valid_temp(self.acpi.read(acpi::REG_CPU_TEMP)),
            gpu: valid_temp(self.acpi.read(acpi::REG_GPU_TEMP)),
        }
    }

    fn read_fans(&mut self) -> [FanReading; FAN_COUNT] {
        std::array::from_fn(|i| FanReading { pwm: self.smfi.fan_get_pwm(i as u8).ok(), rpm: self.rpm(i as u8) })
    }

    fn set_duty(&mut self, duty: u8) -> Result<(), String> {
        self.duty = duty;
        if self.info.has_fan_mode {
            // Re-checked every time: the EC may reset to auto after sleep.
            if self.smfi.fan_get_mode().map_err(|e| e.to_string())? != 1 {
                self.smfi.fan_set_mode(1).map_err(|e| e.to_string())?;
            }
        }
        self.manual = true;
        self.write_all()
    }

    fn set_auto(&mut self) -> Result<(), String> {
        if self.info.has_fan_mode {
            self.smfi.fan_set_mode(0).map_err(|e| e.to_string())?;
        }
        // Older firmware: simply stop writing, the EC takes over on its next tick.
        self.manual = false;
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), String> {
        if self.manual { self.write_all() } else { Ok(()) }
    }
}
