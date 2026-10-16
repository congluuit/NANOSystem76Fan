//! Developer helper: `--screenshot <dir>` renders every page, saves each one as
//! a BMP into `<dir>` and quits. Used to produce the README screenshots.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, ScreenshotCallback, ViewportCommand};

use super::{FanApp, Page, theme};
use crate::config::Theme;

const SHOTS: &[(Page, Theme, &str)] = &[
    (Page::Dashboard, Theme::Dark, "dashboard"),
    (Page::Curve, Theme::Dark, "curve"),
    (Page::Drivers, Theme::Dark, "drivers"),
    (Page::Settings, Theme::Dark, "settings"),
    (Page::About, Theme::Dark, "about"),
    (Page::Dashboard, Theme::Light, "dashboard-light"),
];

/// Time for data to arrive and animations to settle before a capture.
const SETTLE: Duration = Duration::from_millis(2500);

pub struct Plan {
    dir: PathBuf,
    index: usize,
    shown_at: Option<Instant>,
    pending: Arc<AtomicBool>,
}

impl Plan {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        Self { dir, index: 0, shown_at: None, pending: Arc::new(AtomicBool::new(false)) }
    }
}

pub fn step(app: &mut FanApp, ctx: &egui::Context) {
    ctx.request_repaint();
    let Some(plan) = app.shots.as_mut() else { return };
    if plan.pending.load(Ordering::SeqCst) {
        return;
    }
    let Some(&(page, theme_choice, name)) = SHOTS.get(plan.index) else {
        app.quit(ctx);
        return;
    };
    match plan.shown_at {
        None => {
            plan.shown_at = Some(Instant::now());
            app.page = page;
            if app.cfg.theme != theme_choice {
                app.cfg.theme = theme_choice;
                theme::apply(ctx, theme_choice);
            }
            if page == Page::Drivers {
                super::pages::start_scan(app, ctx);
            }
        }
        Some(t) if t.elapsed() >= SETTLE => {
            plan.pending.store(true, Ordering::SeqCst);
            let pending = plan.pending.clone();
            let path = plan.dir.join(format!("{name}.bmp"));
            let repaint = ctx.clone();
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(ScreenshotCallback::new(move |img| {
                let _ = save_bmp(&path, &img);
                pending.store(false, Ordering::SeqCst);
                repaint.request_repaint();
            })));
            plan.index += 1;
            plan.shown_at = None;
        }
        Some(_) => {}
    }
}

fn save_bmp(path: &std::path::Path, img: &ColorImage) -> std::io::Result<()> {
    let [w, h] = img.size;
    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut v = Vec::with_capacity(size);
    v.extend_from_slice(b"BM");
    v.extend_from_slice(&(size as u32).to_le_bytes());
    v.extend_from_slice(&[0; 4]);
    v.extend_from_slice(&54u32.to_le_bytes());
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&(w as i32).to_le_bytes());
    v.extend_from_slice(&(h as i32).to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() {
        for x in 0..w {
            let c = img.pixels[y * w + x];
            v.extend_from_slice(&[c.b(), c.g(), c.r()]);
        }
        v.extend(std::iter::repeat_n(0u8, row - w * 3));
    }
    std::fs::write(path, v)
}
