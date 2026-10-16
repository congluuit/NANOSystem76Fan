//! Reusable widgets: cards, segmented control, toggle switch, stat tiles.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Rect, Response, RichText, Role, Sense, Stroke,
    Ui, WidgetInfo, pos2, vec2,
};

use super::theme::{self, ACCENT, Palette};

/// A rounded panel used for every content block.
pub fn card<R>(ui: &mut Ui, pal: &Palette, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(pal.card)
        .stroke(Stroke::new(1.0, pal.stroke))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Card title with an optional icon.
pub fn card_title(ui: &mut Ui, pal: &Palette, icon: Option<&str>, text: &str) {
    ui.horizontal(|ui| {
        if let Some(i) = icon {
            ui.label(RichText::new(i).font(theme::icon_font(16.0)).color(ACCENT));
        }
        ui.label(RichText::new(text).font(theme::bold(16.0)).color(pal.text));
    });
    ui.add_space(2.0);
}

pub fn weak(pal: &Palette, text: impl Into<String>) -> RichText {
    RichText::new(text).color(pal.weak)
}

/// Text preceded by an icon glyph.
pub fn icon_text(ui: &mut Ui, icon: &str, color: Color32, text: impl Into<RichText>) -> Response {
    let text: RichText = text.into();
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).font(theme::icon_font(15.0)).color(color));
        ui.label(text);
    })
    .response
}

/// Segmented control. Returns true if the value changed.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, pal: &Palette, value: &mut T, options: &[(T, &str)]) -> bool {
    let font = egui::FontId::proportional(14.5);
    let pad = vec2(18.0, 8.0);
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, label)| ui.painter().layout_no_wrap(label.to_string(), font.clone(), pal.text).size().x + pad.x * 2.0)
        .collect();
    let height = 34.0;
    let total: f32 = widths.iter().sum::<f32>() + 8.0;
    let (rect, _) = ui.allocate_exact_size(vec2(total, height), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(9), ui.visuals().extreme_bg_color);

    let mut changed = false;
    let mut x = rect.left() + 4.0;
    for ((opt, label), w) in options.iter().zip(widths) {
        let seg = Rect::from_min_size(pos2(x, rect.top() + 4.0), vec2(w, height - 8.0));
        x += w;
        let id = ui.scope_id().with(("segment", label));
        let resp = ui.interact(seg, id, Sense::click());
        resp.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, *value == *opt, *label));
        let selected = *value == *opt;
        if selected {
            ui.painter().rect_filled(seg, CornerRadius::same(7), ACCENT);
        } else if resp.hovered() {
            ui.painter().rect_filled(seg, CornerRadius::same(7), pal.card_hover);
        }
        let color = if selected { Color32::WHITE } else { pal.text };
        ui.painter().text(seg.center(), Align2::CENTER_CENTER, label, font.clone(), color);
        if resp.clicked() && !selected {
            *value = *opt;
            changed = true;
        }
    }
    changed
}

/// iOS-style toggle switch with a label. Returns the response of the whole row.
pub fn toggle(ui: &mut Ui, pal: &Palette, on: &mut bool, label: &str) -> Response {
    let size = vec2(40.0, 22.0);
    let text = ui.painter().layout(
        label.to_string(),
        egui::FontId::proportional(14.5),
        pal.text,
        ui.available_width() - size.x - 12.0,
    );
    let row = vec2(size.x + 12.0 + text.size().x, size.y.max(text.size().y));
    let (rect, mut resp) = ui.allocate_exact_size(row, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, *on, label));

    let switch = Rect::from_min_size(pos2(rect.left(), rect.center().y - size.y / 2.0), size);
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let off_color = ui.visuals().extreme_bg_color;
    let bg = lerp_color(off_color, ACCENT, t);
    ui.painter().rect(switch, CornerRadius::same(11), bg, Stroke::new(1.0, pal.stroke), egui::StrokeKind::Inside);
    let knob_x = egui::lerp((switch.left() + 11.0)..=(switch.right() - 11.0), t);
    ui.painter().circle_filled(pos2(knob_x, switch.center().y), 8.0, Color32::WHITE);
    ui.painter().galley(pos2(switch.right() + 12.0, rect.center().y - text.size().y / 2.0), text, pal.text);
    resp
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Dashboard tile: title, big value, optional sub-line and a progress bar.
pub fn stat_tile(
    ui: &mut Ui,
    pal: &Palette,
    icon: &str,
    title: &str,
    value: &str,
    sub: &str,
    bar: Option<(f32, Color32)>,
) {
    card(ui, pal, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).font(theme::icon_font(14.0)).color(pal.weak));
            ui.label(weak(pal, title));
        });
        ui.label(RichText::new(value).font(theme::bold(28.0)).color(pal.text));
        ui.label(weak(pal, sub).size(12.5));
        if let Some((frac, color)) = bar {
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(3), ui.visuals().extreme_bg_color);
            let mut fill = rect;
            fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
            ui.painter().rect_filled(fill, CornerRadius::same(3), color);
        }
    });
}

/// Colour for a temperature reading.
pub fn temp_color(t: Option<u8>) -> Color32 {
    match t {
        Some(t) if t >= 90 => theme::DANGER,
        Some(t) if t >= 80 => theme::WARN,
        Some(_) => theme::OK,
        None => Color32::GRAY,
    }
}

/// Rounded status pill with a coloured dot. Sized to its text, so it also
/// works inside right-to-left layouts.
pub fn status_pill(ui: &mut Ui, pal: &Palette, color: Color32, text: &str) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(14.0), pal.text);
    let size = vec2(galley.size().x + 40.0, galley.size().y + 14.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(20),
        color.gamma_multiply(0.14),
        Stroke::new(1.0, color.gamma_multiply(0.5)),
        egui::StrokeKind::Inside,
    );
    p.circle_filled(pos2(rect.left() + 16.0, rect.center().y), 4.0, color);
    p.galley(pos2(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0), galley, pal.text);
}

/// Small coloured banner for notes and warnings.
pub fn banner(ui: &mut Ui, pal: &Palette, icon: &str, color: Color32, text: &str) {
    Frame::new()
        .fill(color.gamma_multiply(0.10))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.4)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.label(RichText::new(icon).font(theme::icon_font(15.0)).color(color));
                ui.add(egui::Label::new(RichText::new(text).color(pal.text).size(13.0)).wrap());
            });
        });
}

/// Row of small selectable chips; `selected` may be `None` (e.g. a custom curve).
/// Returns the index of the clicked chip.
pub fn chips(ui: &mut Ui, pal: &Palette, selected: Option<usize>, labels: &[&str]) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (i, label) in labels.iter().enumerate() {
            let on = selected == Some(i);
            let text = RichText::new(*label).size(13.0).color(if on { Color32::WHITE } else { pal.text });
            let button = egui::Button::new(text)
                .fill(if on { ACCENT } else { ui.visuals().extreme_bg_color })
                .stroke(Stroke::new(1.0, if on { ACCENT } else { pal.stroke }))
                .corner_radius(CornerRadius::same(14));
            if ui.add(button).clicked() {
                clicked = Some(i);
            }
        }
    });
    clicked
}

/// Button styled with the accent colour.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(text).color(Color32::WHITE)).fill(ACCENT))
}

/// Button with an icon glyph in front of the text.
pub fn icon_button(ui: &mut Ui, icon: &str, text: &str) -> Response {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        icon,
        0.0,
        egui::TextFormat {
            font_id: theme::icon_font(13.0),
            color: ui.visuals().text_color(),
            valign: Align::Center,
            ..Default::default()
        },
    );
    job.append(
        text,
        8.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.5),
            color: ui.visuals().text_color(),
            valign: Align::Center,
            ..Default::default()
        },
    );
    ui.button(job)
}
