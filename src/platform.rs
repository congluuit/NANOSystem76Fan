//! Small Windows helpers: single instance, autostart, opening links, message
//! boxes and the "devices without driver" scan.

use std::os::windows::process::CommandExt;
use std::process::Command;

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, EVENT_MODIFY_STATE, INFINITE, OpenEventW, SetEvent, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW, SW_SHOWNORMAL,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const INSTANCE_MUTEX: &str = "Local\\S76FanControl-rs.Instance";
const SHOW_EVENT: &str = "Local\\S76FanControl-rs.Show";
const TASK_NAME: &str = "S76FanControl";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Holds the single-instance mutex for the life of the process.
pub struct SingleInstance(HANDLE);

// SAFETY: a mutex handle can be used from any thread.
unsafe impl Send for SingleInstance {}

impl SingleInstance {
    /// Returns `None` if another instance is already running; in that case that
    /// instance is asked to show its window.
    pub fn acquire() -> Option<Self> {
        let name = wide(INSTANCE_MUTEX);
        // SAFETY: valid NUL-terminated name.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
        // SAFETY: trivially safe.
        if handle.is_null() || unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            if !handle.is_null() {
                unsafe { CloseHandle(handle) };
            }
            signal_existing_instance();
            return None;
        }
        Some(Self(handle))
    }

    /// Calls `on_show` (on a background thread) whenever a second instance is launched.
    pub fn listen_for_show(&self, on_show: impl Fn() + Send + 'static) {
        let name = wide(SHOW_EVENT);
        // SAFETY: auto-reset event, valid name.
        let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()) } as usize;
        if event == 0 {
            return;
        }
        std::thread::spawn(move || {
            loop {
                // SAFETY: the event handle lives for the whole process.
                unsafe { WaitForSingleObject(event as HANDLE, INFINITE) };
                on_show();
            }
        });
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: handle created in `acquire`.
        unsafe { CloseHandle(self.0) };
    }
}

fn signal_existing_instance() {
    let name = wide(SHOW_EVENT);
    // SAFETY: valid name; handle closed below.
    unsafe {
        let ev = OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr());
        if !ev.is_null() {
            SetEvent(ev);
            CloseHandle(ev);
        }
    }
}

pub fn open_url(url: &str) {
    let op = wide("open");
    let target = wide(url);
    // SAFETY: valid NUL-terminated strings.
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

pub fn open_path(path: &std::path::Path) {
    open_url(&path.to_string_lossy());
}

pub fn message_box(title: &str, text: &str, error: bool) {
    let t = wide(title);
    let m = wide(text);
    let flags = MB_OK | if error { MB_ICONERROR } else { MB_ICONINFORMATION };
    // SAFETY: valid strings, no owner window.
    unsafe { MessageBoxW(std::ptr::null_mut(), m.as_ptr(), t.as_ptr(), flags) };
}

fn hidden(cmd: &str) -> Command {
    let mut c = Command::new(cmd);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

// ---------------------------------------------------------------- autostart

/// Autostart uses a Task Scheduler logon task with highest privileges, which
/// starts the (administrator-only) program without a UAC prompt.
pub fn autostart_enabled() -> bool {
    hidden("schtasks").args(["/Query", "/TN", TASK_NAME]).output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn set_autostart(enable: bool) -> Result<(), String> {
    let output = if enable {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let action = format!("\"{}\" --minimized", exe.display());
        hidden("schtasks")
            .args(["/Create", "/F", "/TN", TASK_NAME, "/SC", "ONLOGON", "/RL", "HIGHEST", "/DELAY", "0000:10", "/TR"])
            .arg(action)
            .output()
    } else {
        hidden("schtasks").args(["/Delete", "/F", "/TN", TASK_NAME]).output()
    }
    .map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&output.stderr).trim().to_string()) }
}

// ------------------------------------------------------------- device scan

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemDevice {
    pub name: String,
    pub id: String,
    pub error_code: u32,
}

/// Lists PnP devices that report a problem (e.g. code 28 = no driver installed).
pub fn scan_problem_devices() -> Result<Vec<ProblemDevice>, String> {
    // Tab-separated output keeps parsing trivial and avoids a JSON dependency.
    let script = "[Console]::OutputEncoding = [Text.Encoding]::UTF8; Get-CimInstance Win32_PnPEntity | Where-Object { $_.ConfigManagerErrorCode -ne 0 } | \
                  ForEach-Object { \"$($_.Name)`t$($_.PNPDeviceID)`t$($_.ConfigManagerErrorCode)\" }";
    let out = hidden("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.trim_end().split('\t');
            let name = parts.next()?.trim().to_string();
            let id = parts.next()?.trim().to_string();
            let error_code = parts.next()?.trim().parse().ok()?;
            Some(ProblemDevice { name, id, error_code })
        })
        .collect())
}
