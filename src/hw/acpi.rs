//! Read-only access to the EC's ACPI register space through the standard
//! ACPI EC ports 0x62 (data) / 0x66 (command/status).
//!
//! Register map from `src/app/main/acpi.c` of the System76 EC firmware:
//! - `0x07` CPU temperature (PECI, °C)
//! - `0xCD` dGPU temperature (°C, 0 when the dGPU is off)
//! - `0xD0..0xD1` / `0xD2..0xD3` fan 1 / fan 2 tachometer (raw on older
//!   firmware, RPM on newer firmware)
//!
//! Windows' own ACPI EC driver uses the same ports, so transactions are kept
//! short, retried, and guarded by the `Global\Access_EC` mutex that other
//! hardware tools (LibreHardwareMonitor, HWiNFO, ...) also respect.

use std::sync::Arc;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

use super::portio::PortIo;

const DATA_PORT: u16 = 0x62;
const CMD_PORT: u16 = 0x66;
const RD_EC: u8 = 0x80;
const OBF: u8 = 0x01;
const IBF: u8 = 0x02;

pub const REG_CPU_TEMP: u8 = 0x07;
pub const REG_GPU_TEMP: u8 = 0xCD;
pub const REG_FAN1_TACH: u8 = 0xD0;
pub const REG_FAN2_TACH: u8 = 0xD2;

pub struct AcpiEc {
    io: Arc<PortIo>,
    mutex: HANDLE,
}

// SAFETY: the mutex handle is only used through thread-safe Win32 calls.
unsafe impl Send for AcpiEc {}

impl AcpiEc {
    pub fn new(io: Arc<PortIo>) -> Self {
        let name: Vec<u16> = "Global\\Access_EC\0".encode_utf16().collect();
        // SAFETY: valid, NUL-terminated name. A null handle just disables locking.
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        Self { io, mutex }
    }

    pub fn read(&self, addr: u8) -> Option<u8> {
        let _guard = self.lock();
        for _ in 0..3 {
            if let Some(v) = self.try_read(addr) {
                return Some(v);
            }
        }
        None
    }

    pub fn read_u16(&self, addr: u8) -> Option<u16> {
        let lo = self.read(addr)?;
        let hi = self.read(addr.wrapping_add(1))?;
        Some(u16::from_le_bytes([lo, hi]))
    }

    fn try_read(&self, addr: u8) -> Option<u8> {
        self.wait(IBF, false)?;
        // Drop a stale byte left in the output buffer, if any.
        if self.io.inb(CMD_PORT) & OBF != 0 {
            self.io.inb(DATA_PORT);
        }
        self.io.outb(CMD_PORT, RD_EC);
        self.wait(IBF, false)?;
        self.io.outb(DATA_PORT, addr);
        self.wait(OBF, true)?;
        Some(self.io.inb(DATA_PORT))
    }

    fn wait(&self, bit: u8, set: bool) -> Option<()> {
        let start = Instant::now();
        while (self.io.inb(CMD_PORT) & bit != 0) != set {
            if start.elapsed() > Duration::from_millis(20) {
                return None;
            }
            std::hint::spin_loop();
        }
        Some(())
    }

    fn lock(&self) -> MutexGuard {
        if self.mutex.is_null() {
            return MutexGuard(std::ptr::null_mut());
        }
        // SAFETY: valid mutex handle owned by self.
        let r = unsafe { WaitForSingleObject(self.mutex, 100) };
        if r == WAIT_OBJECT_0 || r == WAIT_ABANDONED {
            MutexGuard(self.mutex)
        } else {
            MutexGuard(std::ptr::null_mut())
        }
    }
}

impl Drop for AcpiEc {
    fn drop(&mut self) {
        if !self.mutex.is_null() {
            // SAFETY: handle created in `new`.
            unsafe { CloseHandle(self.mutex) };
        }
    }
}

struct MutexGuard(HANDLE);

impl Drop for MutexGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: we own the mutex at this point.
            unsafe { ReleaseMutex(self.0) };
        }
    }
}
