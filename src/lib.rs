//! S76 Fan Control — fan control for System76 Open Firmware laptops on Windows.
//!
//! The binary (`src/main.rs`) is a thin launcher; everything lives here so the
//! unit tests run without the administrator manifest that the .exe carries.

pub mod config;
pub mod controller;
pub mod curve;
pub mod diag;
pub mod drivers;
pub mod hw;
pub mod i18n;
pub mod icon_art;
pub mod platform;
pub mod tray;
pub mod ui;
