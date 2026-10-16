//! Page contents.

use eframe::egui::{self, Align, Color32, Layout, RichText, Ui};

use super::theme::{self, ACCENT, Palette, icon};
use super::widgets::{self, card, card_title, weak};
use super::{DeviceScan, FanApp, Page, charts};
use crate::config::{self, Config, Language, Mode, Theme};
use crate::controller::{Decision, Phase};
use crate::curve::Preset;
use crate::drivers::{self, KnownDevice};
use crate::i18n::{Strings, fill};
use crate::platform;

/// Repository of this project (shown on the About page).
pub const PROJECT_URL: &str = env!("CARGO_PKG_REPOSITORY");

fn header(ui: &mut Ui, pal: &Palette, title: &str, subtitle: Option<&str>, right: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(title);
            if let Some(sub) = subtitle {
                ui.label(weak(pal, sub));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(12.0);
            right(ui);
        });
    });
    ui.add_space(8.0);
}

fn preset_label(s: &Strings, p: Preset) -> &'static str {
    match p {
        Preset::Silent => s.preset_silent,
        Preset::Balanced => s.preset_balanced,
        Preset::Performance => s.preset_performance,
        Preset::Firmware => s.preset_firmware,
    }
}

/// Preset chips; highlights the preset matching the current curve, if any.
fn preset_chips(app: &mut FanApp, ui: &mut Ui, pal: &Palette) {
    let s = app.s();
    let labels: Vec<&str> = Preset::ALL.iter().map(|p| preset_label(s, *p)).collect();
    let selected = Preset::ALL.iter().position(|p| app.cfg.curve == p.points());
    ui.label(weak(pal, s.preset).size(12.5));
    if let Some(i) = widgets::chips(ui, pal, selected, &labels) {
        app.cfg.curve = Preset::ALL[i].points();
    }
}

/// Highest value in the history, e.g. "max 91 °C · 5 min".
fn peak(app: &FanApp, get: impl Fn(&crate::controller::Sample) -> Option<u8>) -> String {
    app.status.history.iter().filter_map(get).max().map_or(String::new(), |m| format!("max {m} °C · 5 min"))
}

fn subtitle(app: &FanApp) -> Option<String> {
    app.status.info.as_ref().map(|i| format!("System76 {} · EC {}", i.model(), i.version))
}

// ------------------------------------------------------------------ dashboard

pub fn dashboard(app: &mut FanApp, ui: &mut Ui, pal: &Palette) {
    let s = app.s();
    let (color, state) = app.state_summary();
    header(ui, pal, s.nav_dashboard, subtitle(app).as_deref(), |ui| widgets::status_pill(ui, pal, color, &state));

    if let Phase::Failed(err) = app.status.phase.clone() {
        failure_card(app, ui, pal, &err);
        return;
    }

    let st = &app.status;
    let fmt_temp = |t: Option<u8>| t.map_or("–".to_string(), |t| format!("{t} °C"));
    let temp_bar = |t: Option<u8>| t.map(|t| ((f32::from(t) - 30.0) / 70.0, widgets::temp_color(Some(t))));
    let (temps, fans) = (st.temps, st.fans);
    let (cpu_peak, gpu_peak) = (peak(app, |s| s.cpu), peak(app, |s| s.gpu));

    ui.columns(4, |cols| {
        widgets::stat_tile(
            &mut cols[0],
            pal,
            icon::THERMO,
            s.cpu,
            &fmt_temp(temps.cpu),
            &cpu_peak,
            temp_bar(temps.cpu).or(Some((0.0, Color32::GRAY))),
        );
        let gpu_value = temps.gpu.map_or(s.gpu_off.to_string(), |t| format!("{t} °C"));
        widgets::stat_tile(
            &mut cols[1],
            pal,
            icon::THERMO,
            s.gpu,
            &gpu_value,
            &gpu_peak,
            temp_bar(temps.gpu).or(Some((0.0, Color32::GRAY))),
        );
        for (i, (col, title)) in cols[2..].iter_mut().zip([s.cpu_fan, s.gpu_fan]).enumerate() {
            let f = fans[i];
            let pct = f.pwm.map(|p| f32::from(p) / 255.0);
            widgets::stat_tile(
                col,
                pal,
                icon::DASHBOARD,
                title,
                &f.rpm.map_or("–".into(), |r| format!("{r} RPM")),
                &pct.map_or("PWM –".into(), |p| format!("PWM {:.0} %", p * 100.0)),
                pct.map(|p| (p, theme::FAN)),
            );
        }
    });
    ui.add_space(4.0);

    ui.columns(2, |cols| {
        mode_card(app, &mut cols[0], pal);
        card(&mut cols[1], pal, |ui| {
            card_title(ui, pal, Some(icon::CURVE), s.history);
            charts::legend(ui, pal, &[(s.legend_cpu, ACCENT), (s.legend_gpu, theme::GPU), (s.legend_fan, theme::FAN)]);
            charts::history(ui, pal, &app.status.history, 210.0);
        });
    });

    if let Some(info) = &app.status.info
        && !info.has_fan_mode
    {
        ui.add_space(4.0);
        widgets::banner(ui, pal, icon::ABOUT, ACCENT, &fill(s.legacy_note, &[&info.version]));
    }
}

fn mode_card(app: &mut FanApp, ui: &mut Ui, pal: &Palette) {
    let s = app.s();
    card(ui, pal, |ui| {
        card_title(ui, pal, Some(icon::SLIDERS), s.control_mode);
        let mut mode = app.cfg.mode;
        widgets::segmented(
            ui,
            pal,
            &mut mode,
            &[(Mode::Auto, s.mode_auto), (Mode::Fixed, s.mode_fixed), (Mode::Curve, s.mode_curve)],
        );
        app.cfg.mode = mode;
        ui.add_space(4.0);

        match mode {
            Mode::Auto => {
                ui.label(weak(pal, s.mode_auto_desc));
            }
            Mode::Fixed => {
                ui.label(weak(pal, s.mode_fixed_desc));
                let min = app.cfg.min_percent;
                app.cfg.fixed_percent = app.cfg.fixed_percent.max(min);
                ui.add(
                    egui::Slider::new(&mut app.cfg.fixed_percent, min..=100.0)
                        .suffix(" %")
                        .step_by(1.0)
                        .fixed_decimals(0),
                );
                if app.cfg.fixed_percent < 35.0 {
                    widgets::banner(ui, pal, icon::WARNING, theme::WARN, s.fixed_low_warning);
                }
            }
            Mode::Curve => {
                ui.label(weak(pal, s.mode_curve_desc));
                preset_chips(app, ui, pal);
                if let Some(Decision::Manual(p)) = app.status.decision {
                    ui.label(RichText::new(format!("→ {p:.0} %")).font(theme::bold(18.0)).color(ACCENT));
                }
                if widgets::icon_button(ui, icon::CURVE, s.edit_curve).clicked() {
                    app.page = Page::Curve;
                }
            }
        }
    });
}

fn failure_card(app: &mut FanApp, ui: &mut Ui, pal: &Palette, err: &str) {
    let s = app.s();
    card(ui, pal, |ui| {
        widgets::icon_text(ui, icon::WARNING, theme::DANGER, RichText::new(s.fail_title).font(theme::bold(17.0)));
        ui.add(egui::Label::new(RichText::new(err).monospace().color(theme::DANGER)).wrap());
        ui.add_space(6.0);
        for hint in [s.fail_hint_dll, s.fail_hint_admin, s.fail_hint_hvci, s.fail_hint_model] {
            ui.label(format!("•  {hint}"));
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if widgets::primary_button(ui, s.retry).clicked() {
                app.controller.retry();
            }
            if widgets::icon_button(ui, icon::LINK, s.download_inpout).clicked() {
                platform::open_url(drivers::INPOUT_URL);
            }
        });
    });
}

// ---------------------------------------------------------------------- curve

pub fn curve(app: &mut FanApp, ui: &mut Ui, pal: &Palette) {
    let s = app.s();
    let (color, state) = app.state_summary();
    header(ui, pal, s.curve_title, Some(s.curve_help), |ui| widgets::status_pill(ui, pal, color, &state));

    if app.cfg.mode != Mode::Curve {
        widgets::banner(ui, pal, icon::ABOUT, ACCENT, &format!("{}: {}", s.control_mode, mode_name(s, app.cfg.mode)));
        if widgets::primary_button(ui, &format!("{} → {}", s.control_mode, s.mode_curve)).clicked() {
            app.cfg.mode = Mode::Curve;
        }
        ui.add_space(4.0);
    }

    card(ui, pal, |ui| {
        preset_chips(app, ui, pal);
        let hottest = app.status.temps.hottest();
        let current = match app.status.decision {
            Some(Decision::Manual(p)) if app.cfg.mode == Mode::Curve => Some(p),
            _ => None,
        };
        let live = hottest.map(|t| (t, current));
        charts::curve_editor(ui, pal, &mut app.cfg.curve, &mut app.curve_drag, live, s.curve_now, 360.0);
    });

    card(ui, pal, |ui| {
        card_title(ui, pal, Some(icon::WARNING), s.safety);
        egui::Grid::new("safety").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| {
            ui.label(s.safety_temp);
            ui.add(
                egui::Slider::new(&mut app.cfg.safety_temp, config::SAFETY_TEMP_RANGE)
                    .suffix(" °C")
                    .step_by(1.0)
                    .fixed_decimals(0),
            );
            ui.end_row();
            ui.label(s.min_speed);
            ui.add(egui::Slider::new(&mut app.cfg.min_percent, 0.0..=60.0).suffix(" %").step_by(1.0).fixed_decimals(0));
            ui.end_row();
            ui.label(s.ramp_down);
            ui.add(
                egui::Slider::new(&mut app.cfg.ramp_down_per_sec, 0.5..=20.0)
                    .suffix(format!(" {}", s.per_sec))
                    .step_by(0.5)
                    .fixed_decimals(1),
            );
            ui.end_row();
        });
        if ui.button(s.reset).clicked() {
            let d = Config::default();
            app.cfg.safety_temp = d.safety_temp;
            app.cfg.min_percent = d.min_percent;
            app.cfg.ramp_down_per_sec = d.ramp_down_per_sec;
        }
    });
}

fn mode_name(s: &Strings, m: Mode) -> &'static str {
    match m {
        Mode::Auto => s.mode_auto,
        Mode::Fixed => s.mode_fixed,
        Mode::Curve => s.mode_curve,
    }
}

// -------------------------------------------------------------------- drivers

pub fn drivers(app: &mut FanApp, ui: &mut Ui, pal: &Palette) {
    let s = app.s();
    header(ui, pal, s.nav_drivers, subtitle(app).as_deref(), |_| {});
    let model = app.status.info.as_ref().map(|i| i.model().to_string());

    card(ui, pal, |ui| {
        card_title(ui, pal, Some(icon::CHIP), s.drv_required);
        let ok = app.status.phase == Phase::Running;
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.drv_inpout).font(theme::bold(14.5)));
            let (c, t) = if ok { (theme::OK, s.drv_ok) } else { (theme::DANGER, s.drv_missing) };
            widgets::status_pill(ui, pal, c, t);
        });
        ui.label(weak(pal, s.drv_inpout_desc));
        if !ok && widgets::icon_button(ui, icon::LINK, s.download_inpout).clicked() {
            platform::open_url(drivers::INPOUT_URL);
        }
    });

    card(ui, pal, |ui| {
        card_title(ui, pal, Some(icon::LAPTOP), s.drv_official);
        let packages = model.as_deref().map(drivers::packages_for).unwrap_or_default();
        match &model {
            Some(m) if drivers::model_target(m).is_some() => {
                ui.label(fill(s.drv_official_desc, &[m]));
            }
            _ => {
                ui.label(s.drv_official_unknown);
            }
        }
        if !packages.is_empty() {
            egui::Grid::new("pkgs").num_columns(3).spacing([28.0, 8.0]).show(ui, |ui| {
                for p in packages {
                    ui.label(RichText::new(p.name).font(theme::bold(14.0)));
                    ui.label(weak(pal, p.version));
                    ui.label(RichText::new(p.installer).monospace().size(12.0).color(pal.weak));
                    ui.end_row();
                }
            });
        }
        ui.label(weak(pal, s.drv_others));
        ui.horizontal(|ui| {
            let url = model.as_deref().map_or_else(|| drivers::REPO_URL.to_string(), drivers::model_url);
            if widgets::primary_button(ui, s.drv_open_page).on_hover_text(&url).clicked() {
                platform::open_url(&url);
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(s.drv_nvidia);
            if widgets::icon_button(ui, icon::LINK, s.drv_nvidia_btn).clicked() {
                platform::open_url(drivers::NVIDIA_URL);
            }
        });
    });

    card(ui, pal, |ui| {
        ui.horizontal(|ui| {
            card_title(ui, pal, Some(icon::WARNING), s.drv_devices);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let running = matches!(*app.devices.lock().unwrap(), DeviceScan::Running);
                let label = if running { s.drv_scanning } else { s.drv_scan };
                if ui.add_enabled(!running, egui::Button::new(label)).clicked() {
                    start_scan(app, ui.ctx());
                }
            });
        });
        match &*app.devices.lock().unwrap() {
            DeviceScan::NotRun | DeviceScan::Running => {}
            DeviceScan::Done(Err(e)) => {
                ui.label(RichText::new(e).color(theme::DANGER));
            }
            DeviceScan::Done(Ok(list)) if list.is_empty() => {
                widgets::icon_text(ui, icon::CHECK, theme::OK, s.drv_none);
            }
            DeviceScan::Done(Ok(list)) => {
                for d in list {
                    let (color, note) = match drivers::classify(&d.id) {
                        KnownDevice::System76Acpi => (theme::OK, s.drv_dev_s76),
                        KnownDevice::Coreboot => (theme::OK, s.drv_dev_coreboot),
                        KnownDevice::Unknown => (theme::WARN, s.drv_dev_unknown),
                    };
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, color);
                        let name = if d.name.is_empty() { d.id.clone() } else { format!("{} ({})", d.name, d.id) };
                        ui.label(RichText::new(name).monospace().size(12.5));
                        ui.label(weak(pal, fill(s.drv_code, &[&d.error_code])).size(12.0));
                    });
                    ui.indent(&d.id, |ui| ui.label(weak(pal, note).size(12.5)));
                }
            }
        }
    });

    card(ui, pal, |ui| {
        card_title(ui, pal, Some(icon::BULB), s.tips);
        for tip in [s.tip_power, s.tip_fn1, s.tip_background] {
            ui.label(format!("•  {tip}"));
        }
        if widgets::icon_button(ui, icon::SETTINGS, s.tip_power_btn).clicked() {
            platform::open_url("powercfg.cpl");
        }
    });
}

pub(super) fn start_scan(app: &FanApp, ctx: &egui::Context) {
    *app.devices.lock().unwrap() = DeviceScan::Running;
    let (devices, ctx) = (app.devices.clone(), ctx.clone());
    std::thread::spawn(move || {
        let result = platform::scan_problem_devices();
        *devices.lock().unwrap() = DeviceScan::Done(result);
        ctx.request_repaint();
    });
}

// ------------------------------------------------------------------- settings

pub fn settings(app: &mut FanApp, ui: &mut Ui, pal: &Palette) {
    let s = app.s();
    header(ui, pal, s.nav_settings, None, |_| {});

    card(ui, pal, |ui| {
        card_title(ui, pal, Some(icon::SETTINGS), s.set_general);
        egui::Grid::new("general").num_columns(2).spacing([24.0, 14.0]).show(ui, |ui| {
            ui.label(s.set_language);
            widgets::segmented(
                ui,
                pal,
                &mut app.cfg.language,
                &[(Language::System, s.set_lang_system), (Language::En, "English"), (Language::Vi, "Tiếng Việt")],
            );
            ui.end_row();
            ui.label(s.set_theme);
            widgets::segmented(ui, pal, &mut app.cfg.theme, &[(Theme::Dark, s.set_dark), (Theme::Light, s.set_light)]);
            ui.end_row();
        });
        ui.add_space(6.0);

        let mut autostart = app.autostart;
        if widgets::toggle(ui, pal, &mut autostart, s.set_autostart).changed() {
            match platform::set_autostart(autostart) {
                Ok(()) => {
                    app.autostart = autostart;
                    app.autostart_error = None;
                }
                Err(e) => app.autostart_error = Some(e),
            }
        }
        if let Some(e) = &app.autostart_error {
            ui.label(RichText::new(fill(s.set_autostart_error, &[e])).color(theme::DANGER));
        }
        widgets::toggle(ui, pal, &mut app.cfg.close_to_tray, s.set_close_to_tray);
    });

    card(ui, pal, |ui| {
        card_title(ui, pal, Some(icon::FOLDER), s.set_config_file);
        let path = Config::path();
        ui.label(RichText::new(path.display().to_string()).monospace().size(12.5));
        if widgets::icon_button(ui, icon::FOLDER, s.set_open_folder).clicked()
            && let Some(dir) = path.parent()
        {
            let _ = std::fs::create_dir_all(dir);
            platform::open_path(dir);
        }
    });
}

// ---------------------------------------------------------------------- about

pub fn about(app: &mut FanApp, ui: &mut Ui, pal: &Palette) {
    let s = app.s();
    let logo = app
        .logo
        .get_or_insert_with(|| {
            let img = egui::ColorImage::from_rgba_unmultiplied(
                [128, 128],
                &crate::icon_art::render_icon(128, theme::ACCENT_RGB),
            );
            ui.ctx().load_texture("logo", img, egui::TextureOptions::LINEAR)
        })
        .clone();

    card(ui, pal, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(&logo).fit_to_exact_size(egui::vec2(72.0, 72.0)));
            ui.vertical(|ui| {
                ui.label(RichText::new("S76 Fan Control").font(theme::bold(24.0)));
                ui.label(weak(pal, format!("v{}", env!("CARGO_PKG_VERSION"))));
                ui.label(s.about_desc);
            });
        });
        ui.add_space(6.0);
        widgets::banner(ui, pal, icon::WARNING, theme::WARN, s.about_disclaimer);
        ui.label(weak(pal, s.about_how));
        if let Some(info) = &app.status.info {
            ui.label(
                RichText::new(format!(
                    "{}: {} · {} · protocol {} · fan mode {} · rpm cmd {}",
                    s.about_ec, info.board, info.version, info.protocol, info.has_fan_mode, info.has_rpm_cmd
                ))
                .monospace()
                .size(12.0),
            );
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if widgets::icon_button(ui, icon::LINK, s.about_source).clicked() {
                platform::open_url(PROJECT_URL);
            }
            if widgets::icon_button(ui, icon::LINK, "system76/ec").clicked() {
                platform::open_url("https://github.com/system76/ec");
            }
            if widgets::icon_button(ui, icon::LINK, "system76/windows-drivers").clicked() {
                platform::open_url(drivers::REPO_URL);
            }
        });
        ui.label(weak(pal, s.about_licenses).size(12.0));
    });
}
