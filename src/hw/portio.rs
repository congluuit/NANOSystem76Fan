//! Raw x86 port I/O through the InpOut driver (`inpoutx64.dll`).
//!
//! Windows does not allow user-mode programs to execute `in`/`out` instructions,
//! so a kernel driver is required. InpOut (MIT licensed, by Phil Gibbons /
//! highrez.co.uk) ships a small signed driver embedded in its DLL and installs it
//! on first use when the process runs elevated.

use std::fmt;
use std::path::PathBuf;

use libloading::Library;

pub const DLL_NAME: &str = "inpoutx64.dll";

type IsDriverOpenFn = unsafe extern "system" fn() -> i32;
type Out32Fn = unsafe extern "system" fn(port: u16, data: u16);
type Inp32Fn = unsafe extern "system" fn(port: u16) -> u16;

#[derive(Debug, Clone)]
pub enum PortIoError {
    DllMissing(PathBuf),
    LoadFailed(String),
    DriverNotOpen,
}

impl fmt::Display for PortIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DllMissing(p) => write!(f, "{DLL_NAME} not found at {}", p.display()),
            Self::LoadFailed(e) => write!(f, "failed to load {DLL_NAME}: {e}"),
            Self::DriverNotOpen => write!(
                f,
                "the InpOut kernel driver could not be opened (run as administrator; \
                 Memory Integrity / the vulnerable driver blocklist may block it)"
            ),
        }
    }
}

impl std::error::Error for PortIoError {}

/// A loaded InpOut driver. Cheap to share between threads behind an `Arc`.
pub struct PortIo {
    out32: Out32Fn,
    inp32: Inp32Fn,
    // Keep the library alive as long as the function pointers are used.
    _lib: Library,
}

impl PortIo {
    /// Loads `inpoutx64.dll` from the executable's directory and opens the driver.
    pub fn open() -> Result<Self, PortIoError> {
        let path = dll_path();
        if !path.is_file() {
            return Err(PortIoError::DllMissing(path));
        }
        // SAFETY: loading a known DLL by absolute path; the symbols below have
        // the documented InpOut signatures.
        unsafe {
            let lib = Library::new(&path).map_err(|e| PortIoError::LoadFailed(e.to_string()))?;
            let is_open: IsDriverOpenFn = *lib
                .get::<IsDriverOpenFn>(b"IsInpOutDriverOpen\0")
                .map_err(|e| PortIoError::LoadFailed(e.to_string()))?;
            let out32: Out32Fn = *lib.get::<Out32Fn>(b"Out32\0").map_err(|e| PortIoError::LoadFailed(e.to_string()))?;
            let inp32: Inp32Fn = *lib.get::<Inp32Fn>(b"Inp32\0").map_err(|e| PortIoError::LoadFailed(e.to_string()))?;

            if is_open() == 0 {
                return Err(PortIoError::DriverNotOpen);
            }
            Ok(Self { out32, inp32, _lib: lib })
        }
    }

    #[inline]
    pub fn inb(&self, port: u16) -> u8 {
        // SAFETY: the driver is open; reading an I/O port has no memory-safety impact.
        unsafe { ((self.inp32)(port) & 0xFF) as u8 }
    }

    #[inline]
    pub fn outb(&self, port: u16, value: u8) {
        // SAFETY: see `inb`.
        unsafe { (self.out32)(port, u16::from(value)) }
    }
}

pub fn dll_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(DLL_NAME)))
        .unwrap_or_else(|| PathBuf::from(DLL_NAME))
}
