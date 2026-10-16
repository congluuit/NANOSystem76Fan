//! Colours, fonts and egui style.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2,
};

use crate::config::Theme;

/// Keep in sync with `ACCENT` in build.rs (the .exe icon).
pub const ACCENT: Color32 = Color32::from_rgb(0x2E, 0xA8, 0xF0);
pub const ACCENT_RGB: [u8; 3] = [0x2E, 0xA8, 0xF0];
pub const OK: Color32 = Color32::from_rgb(0x3D, 0xD6, 0x8C);
pub const WARN: Color32 = Color32::from_rgb(0xFF, 0xB0, 0x3B);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x5C, 0x5C);
pub const GPU: Color32 = Color32::from_rgb(0xB3, 0x88, 0xFF);
pub const FAN: Color32 = Color32::from_rgb(0xFF, 0x9F, 0x43);

pub const ICON_FONT: &str = "icons";

/// Segoe MDL2 Assets code points.
pub mod icon {
    pub const DASHBOARD: &str = "\u{EC4A}";
    pub const CURVE: &str = "\u{E9D2}";
    pub const DRIVERS: &str = "\u{E964}";
    pub const SETTINGS: &str = "\u{E713}";
    pub const ABOUT: &str = "\u{E946}";
    pub const THERMO: &str = "\u{E9CA}";
    pub const WARNING: &str = "\u{E7BA}";
    pub const CHECK: &str = "\u{E930}";
    pub const LINK: &str = "\u{E8A7}";
    pub const FOLDER: &str = "\u{E838}";
    pub const BULB: &str = "\u{EA80}";
    pub const SLIDERS: &str = "\u{E9E9}";
    pub const LAPTOP: &str = "\u{E7F8}";
    pub const CHIP: &str = "\u{E950}";
}

/// Palette for the current theme.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub sidebar: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub stroke: Color32,
    pub text: Color32,
    pub weak: Color32,
    pub grid: Color32,
}

impl Palette {
    pub fn get(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                bg: Color32::from_rgb(0x12, 0x14, 0x19),
                sidebar: Color32::from_rgb(0x0D, 0x0F, 0x13),
                card: Color32::from_rgb(0x1B, 0x1E, 0x26),
                card_hover: Color32::from_rgb(0x23, 0x27, 0x31),
                stroke: Color32::from_rgb(0x2A, 0x2F, 0x3B),
                text: Color32::from_rgb(0xE8, 0xEA, 0xF0),
                weak: Color32::from_rgb(0x8B, 0x93, 0xA7),
                grid: Color32::from_rgb(0x26, 0x2A, 0x35),
            },
            Theme::Light => Self {
                bg: Color32::from_rgb(0xF3, 0xF5, 0xF8),
                sidebar: Color32::from_rgb(0xE8, 0xEC, 0xF1),
                card: Color32::WHITE,
                card_hover: Color32::from_rgb(0xF0, 0xF3, 0xF7),
                stroke: Color32::from_rgb(0xDD, 0xE2, 0xEA),
                text: Color32::from_rgb(0x1A, 0x1D, 0x24),
                weak: Color32::from_rgb(0x67, 0x6F, 0x80),
                grid: Color32::from_rgb(0xE6, 0xEA, 0xF0),
            },
        }
    }
}

/// Loads Segoe UI (+ Semibold and the MDL2 icon font) from the Windows font
/// folder. Falls back to egui's built-in fonts if they are missing.
/// Returns whether the icon font is available.
pub fn install_fonts(ctx: &egui::Context) -> bool {
    let dir =
        std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(|| "C:\\Windows".into()).join("Fonts");
    let load = |f: &str| std::fs::read(dir.join(f)).ok();

    let mut fonts = FontDefinitions::default();
    if let Some(regular) = load("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), Arc::new(FontData::from_owned(regular)));
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "segoe".into());
    }
    if let Some(sym) = load("seguisym.ttf") {
        fonts.font_data.insert("segoe-symbol".into(), Arc::new(FontData::from_owned(sym)));
        fonts.families.entry(FontFamily::Proportional).or_default().push("segoe-symbol".into());
    }
    let mut bold_family = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(semibold) = load("seguisb.ttf") {
        fonts.font_data.insert("segoe-semibold".into(), Arc::new(FontData::from_owned(semibold)));
        bold_family.insert(0, "segoe-semibold".into());
    }
    fonts.families.insert(FontFamily::Name("bold".into()), bold_family);

    let icons = load("segmdl2.ttf").or_else(|| load("SegoeIcons.ttf"));
    let has_icons = icons.is_some();
    let mut icon_family = Vec::new();
    if let Some(icons) = icons {
        fonts.font_data.insert(ICON_FONT.into(), Arc::new(FontData::from_owned(icons)));
        icon_family.push(ICON_FONT.to_string());
    }
    icon_family.extend(fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    fonts.families.insert(FontFamily::Name(ICON_FONT.into()), icon_family);

    ctx.set_fonts(fonts);
    has_icons
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ICON_FONT.into()))
}

pub fn apply(ctx: &egui::Context, theme: Theme) {
    let p = Palette::get(theme);
    let mut v = match theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
    };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.extreme_bg_color = match theme {
        Theme::Dark => Color32::from_rgb(0x0E, 0x10, 0x14),
        Theme::Light => Color32::from_rgb(0xE9, 0xED, 0xF2),
    };
    v.faint_bg_color = p.card_hover;
    v.override_text_color = Some(p.text);
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.slider_trailing_fill = true;
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_stroke = Stroke::new(1.0, p.stroke);

    let r = CornerRadius::same(6);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.card;
    w.noninteractive.weak_bg_fill = p.card;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.stroke);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = r;
    for (wv, fill) in [
        (&mut w.inactive, p.card_hover),
        (&mut w.hovered, p.stroke),
        (&mut w.active, ACCENT.gamma_multiply(0.6)),
        (&mut w.open, p.stroke),
    ] {
        wv.bg_fill = fill;
        wv.weak_bg_fill = fill;
        wv.corner_radius = r;
        wv.bg_stroke = Stroke::new(1.0, p.stroke);
        wv.fg_stroke = Stroke::new(1.0, p.text);
        wv.expansion = 0.0;
    }
    w.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));

    // egui keeps one style per theme and follows the OS theme by default;
    // pin the theme so our visuals are the ones in use.
    let egui_theme = match theme {
        Theme::Dark => egui::Theme::Dark,
        Theme::Light => egui::Theme::Light,
    };
    ctx.set_theme(egui_theme);
    ctx.set_visuals_of(egui_theme, v);

    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = Vec2::new(10.0, 10.0);
        s.spacing.button_padding = Vec2::new(14.0, 7.0);
        s.spacing.slider_width = 260.0;
        s.spacing.interact_size.y = 30.0;
        s.spacing.window_margin = Margin::same(16);
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Body, FontId::proportional(14.5)),
            (TextStyle::Button, FontId::proportional(14.5)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
            (TextStyle::Heading, bold(24.0)),
        ]
        .into();
    });
}
