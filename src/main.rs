//! Launcher for S76 Fan Control.

#![cfg_attr(not(test), windows_subsystem = "windows")]

use std::sync::Arc;
use std::sync::atomic::Ordering;

use eframe::egui;
use s76_fan_control::{diag, hw, icon_art, platform, ui};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a.eq_ignore_ascii_case(flag));

    if has("--diag") {
        std::process::exit(diag::run(has("--test")));
    }
    // Used by the installer/uninstaller: close a running instance cleanly.
    if has("--quit") {
        let closed = platform::request_quit(std::time::Duration::from_secs(10));
        std::process::exit(if closed { 0 } else { 1 });
    }

    let Some(instance) = platform::SingleInstance::acquire() else {
        return; // the running instance was asked to show itself
    };

    install_panic_hook();

    let signals = Arc::new(ui::Signals::default());
    let start_hidden = has("--minimized");
    // Hidden developer flag: `--screenshot <dir>` captures every page and exits.
    let screenshot_dir =
        args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)).map(std::path::PathBuf::from);

    let icon = egui::IconData { rgba: icon_art::render_icon(64, ui::theme::ACCENT_RGB), width: 64, height: 64 };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("S76 Fan Control")
            .with_app_id("S76FanControl")
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([860.0, 560.0])
            .with_visible(!start_hidden)
            .with_icon(Arc::new(icon)),
        centered: true,
        ..Default::default()
    };

    let result = eframe::run_native(
        "S76 Fan Control",
        options,
        Box::new(move |cc| {
            let (ctx, s) = (cc.egui_ctx.clone(), signals.clone());
            instance.listen_for_show(move || {
                s.show.store(true, Ordering::SeqCst);
                ctx.request_repaint();
            });
            let (ctx, s) = (cc.egui_ctx.clone(), signals.clone());
            instance.listen_for_quit(move || {
                s.quit.store(true, Ordering::SeqCst);
                ctx.request_repaint();
            });
            std::mem::forget(instance); // keep the mutex for the life of the process
            Ok(Box::new(ui::FanApp::new(cc, signals, screenshot_dir)))
        }),
    );
    if let Err(e) = result {
        platform::message_box("S76 Fan Control", &format!("Failed to start the user interface: {e}"), true);
    }
}

/// If the UI thread panics, make a best effort to give the fans back to the
/// EC before the process dies (firmware without fan mode recovers by itself).
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        if std::thread::current().name() != Some("fan-control")
            && let Ok(mut hw) = hw::System76Hardware::open()
        {
            use hw::FanHardware;
            let _ = hw.set_auto();
        }
    }));
}
