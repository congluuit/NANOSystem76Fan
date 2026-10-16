//! The egui application.

mod charts;
mod pages;
mod screenshots;
pub mod theme;
mod widgets;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui, ViewportCommand, vec2};

use crate::config::Config;
use crate::controller::{AutoReason, Controller, Decision, Phase, Status};
use crate::hw::{FanHardware, System76Hardware};
use crate::i18n::{self, Strings};
use crate::platform::{self, ProblemDevice};
use crate::tray::{Tray, TrayAction};
use theme::{ACCENT, Palette, icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Curve,
    Drivers,
    Settings,
    About,
}

pub enum DeviceScan {
    NotRun,
    Running,
    Done(Result<Vec<ProblemDevice>, String>),
}

pub struct FanApp {
    cfg: Config,
    sent: Config,
    saved: Config,
    last_save: Instant,
    controller: Controller,
    status: Status,
    page: Page,
    tray: Option<Tray>,
    tray_rx: Option<Receiver<TrayAction>>,
    show_requested: Arc<AtomicBool>,
    quitting: bool,
    curve_drag: Option<usize>,
    autostart: bool,
    autostart_error: Option<String>,
    devices: Arc<Mutex<DeviceScan>>,
    has_icons: bool,
    logo: Option<egui::TextureHandle>,
    shots: Option<screenshots::Plan>,
}

impl FanApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        show_requested: Arc<AtomicBool>,
        screenshot_dir: Option<std::path::PathBuf>,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        let cfg = Config::load();
        let has_icons = theme::install_fonts(&ctx);
        theme::apply(&ctx, cfg.theme);

        let notify_ctx = ctx.clone();
        let controller = Controller::spawn(
            Box::new(|| System76Hardware::open().map(|h| Box::new(h) as Box<dyn FanHardware>)),
            cfg.clone(),
            Arc::new(move || notify_ctx.request_repaint()),
        );

        let (tray, tray_rx) = match Tray::new(&ctx, i18n::strings(cfg.language), theme::ACCENT_RGB) {
            Some((t, rx)) => (Some(t), Some(rx)),
            None => (None, None),
        };

        Self {
            sent: cfg.clone(),
            saved: cfg.clone(),
            cfg,
            last_save: Instant::now(),
            controller,
            status: Status::default(),
            page: Page::Dashboard,
            tray,
            tray_rx,
            show_requested,
            quitting: false,
            curve_drag: None,
            autostart: platform::autostart_enabled(),
            autostart_error: None,
            devices: Arc::new(Mutex::new(DeviceScan::NotRun)),
            has_icons,
            logo: Some(ctx.load_texture(
                "logo",
                egui::ColorImage::from_rgba_unmultiplied(
                    [128, 128],
                    &crate::icon_art::render_icon(128, theme::ACCENT_RGB),
                ),
                egui::TextureOptions::LINEAR,
            )),
            shots: screenshot_dir.map(screenshots::Plan::new),
        }
    }

    fn s(&self) -> &'static Strings {
        i18n::strings(self.cfg.language)
    }

    fn show_window(ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    /// Pushes config changes to the controller and persists them (debounced).
    fn sync_config(&mut self) {
        if self.cfg != self.sent {
            // Do not sanitize while a curve point is being dragged (it reorders points).
            if self.curve_drag.is_none() {
                self.cfg.sanitize();
            }
            self.controller.set_config(&self.cfg);
            self.sent = self.cfg.clone();
        }
        if self.cfg != self.saved
            && self.curve_drag.is_none()
            && self.shots.is_none()
            && self.last_save.elapsed() > Duration::from_secs(1)
        {
            let _ = self.cfg.save();
            self.saved = self.cfg.clone();
            self.last_save = Instant::now();
        }
    }

    /// Short state text + colour, used by the dashboard pill and the tray.
    pub fn state_summary(&self) -> (Color32, String) {
        let s = self.s();
        match &self.status.phase {
            Phase::Connecting => (Color32::GRAY, s.st_connecting.into()),
            Phase::Failed(e) => (theme::DANGER, i18n::fill(s.st_error, &[e])),
            Phase::Running => match (&self.status.last_error, self.status.decision) {
                (Some(e), _) => (theme::DANGER, i18n::fill(s.st_error, &[e])),
                (None, Some(Decision::Manual(p))) => (ACCENT, i18n::fill(s.st_manual, &[&format!("{p:.0}")])),
                (None, Some(Decision::Auto(AutoReason::OverTemp(t)))) => {
                    (theme::DANGER, i18n::fill(s.st_overtemp, &[&t]))
                }
                (None, Some(Decision::Auto(AutoReason::NoTemperature))) => (theme::WARN, s.st_notemp.into()),
                (None, Some(Decision::Auto(AutoReason::UserChoice))) | (None, None) => (theme::OK, s.st_auto.into()),
            },
        }
    }

    fn update_tray(&mut self) {
        let (color, state) = self.state_summary();
        let t = &self.status.temps;
        let fmt = |v: Option<u8>| v.map_or("–".to_string(), |v| format!("{v}°C"));
        let rpm = self.status.fans[0].rpm.map_or("–".to_string(), |r| format!("{r} RPM"));
        let tooltip = format!("S76 Fan Control\nCPU {} · GPU {} · {rpm}\n{state}", fmt(t.cpu), fmt(t.gpu));
        let rgb = [color.r(), color.g(), color.b()];
        let (s, mode) = (self.s(), self.cfg.mode);
        if let Some(tray) = &mut self.tray {
            tray.update(s, mode, &tooltip, if color == Color32::GRAY { theme::ACCENT_RGB } else { rgb });
        }
    }

    fn sidebar(&mut self, ui: &mut Ui, pal: &Palette) {
        let s = self.s();
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            if let Some(logo) = &self.logo {
                ui.add(egui::Image::new(logo).fit_to_exact_size(vec2(28.0, 28.0)));
            }
            ui.label(RichText::new("S76 Fan Control").font(theme::bold(16.0)));
        });
        ui.add_space(22.0);

        let items = [
            (Page::Dashboard, icon::DASHBOARD, s.nav_dashboard),
            (Page::Curve, icon::CURVE, s.nav_curve),
            (Page::Drivers, icon::DRIVERS, s.nav_drivers),
            (Page::Settings, icon::SETTINGS, s.nav_settings),
            (Page::About, icon::ABOUT, s.nav_about),
        ];
        for (page, glyph, label) in items {
            let selected = self.page == page;
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
            let rect = rect.shrink2(vec2(10.0, 2.0));
            if selected {
                ui.painter().rect_filled(rect, CornerRadius::same(8), ACCENT.gamma_multiply(0.18));
                let bar = egui::Rect::from_min_size(rect.left_top() + vec2(0.0, 9.0), vec2(3.0, rect.height() - 18.0));
                ui.painter().rect_filled(bar, CornerRadius::same(2), ACCENT);
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, CornerRadius::same(8), pal.card);
            }
            let color = if selected { ACCENT } else { pal.text };
            let mut x = rect.left() + 14.0;
            if self.has_icons {
                ui.painter().text(
                    egui::pos2(x, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    glyph,
                    theme::icon_font(15.0),
                    color,
                );
                x += 28.0;
            }
            ui.painter().text(
                egui::pos2(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(14.5),
                color,
            );
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.page = page;
            }
        }

        // Status at the bottom of the sidebar.
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(14.0);
            let (color, _) = self.state_summary();
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 4.0, color);
                let model = self.status.info.as_ref().map_or("—".to_string(), |i| i.model().to_string());
                ui.label(widgets::weak(pal, format!("{model} · v{}", env!("CARGO_PKG_VERSION"))).size(12.0));
            });
        });
    }
}

impl eframe::App for FanApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.status = self.controller.status();

        if self.show_requested.swap(false, Ordering::SeqCst) {
            Self::show_window(ctx);
        }
        let actions: Vec<TrayAction> = self.tray_rx.as_ref().map(|rx| rx.try_iter().collect()).unwrap_or_default();
        for action in actions {
            match action {
                TrayAction::Show => Self::show_window(ctx),
                TrayAction::SetMode(m) => self.cfg.mode = m,
                TrayAction::Quit => self.quit(ctx),
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            if self.cfg.close_to_tray && self.tray.is_some() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            } else {
                self.quitting = true;
            }
        }

        self.sync_config();
        self.update_tray();
        if self.shots.is_some() {
            screenshots::step(self, ctx);
        }
        // The controller wakes us every second; this is only a fallback.
        ctx.request_repaint_after(Duration::from_secs(2));
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let pal = Palette::get(self.cfg.theme);
        let theme_before = self.cfg.theme;

        egui::Panel::left("nav")
            .exact_size(210.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(pal.sidebar).stroke(Stroke::NONE))
            .show(ui, |ui| self.sidebar(ui, &pal));

        egui::CentralPanel::default().frame(Frame::new().fill(pal.bg).inner_margin(Margin::symmetric(28, 22))).show(
            ui,
            |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.page {
                    Page::Dashboard => pages::dashboard(self, ui, &pal),
                    Page::Curve => pages::curve(self, ui, &pal),
                    Page::Drivers => pages::drivers(self, ui, &pal),
                    Page::Settings => pages::settings(self, ui, &pal),
                    Page::About => pages::about(self, ui, &pal),
                });
            },
        );

        if self.cfg.theme != theme_before {
            theme::apply(ui.ctx(), self.cfg.theme);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.shots.is_none() {
            self.cfg.sanitize();
            let _ = self.cfg.save();
        }
        // Hands the fans back to the EC before the process ends.
        self.controller.shutdown();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Palette::get(self.cfg.theme).bg.to_normalized_gamma_f32()
    }
}
