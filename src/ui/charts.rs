//! Custom-painted charts: the temperature/fan history and the curve editor.

use std::collections::VecDeque;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2,
};

use super::theme::{self, ACCENT, Palette};
use crate::controller::{HISTORY_LEN, Sample};
use crate::curve::{self, CurvePoint, MAX_POINTS, MIN_POINTS, TEMP_MAX, TEMP_MIN};

const AXIS_W: f32 = 34.0;
const AXIS_H: f32 = 20.0;

/// Line chart of CPU/GPU temperature (30–100 °C) and fan duty (0–100 %).
pub fn history(ui: &mut Ui, pal: &Palette, samples: &VecDeque<Sample>, height: f32) {
    let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(outer);
    let plot = Rect::from_min_max(
        pos2(outer.left() + AXIS_W, outer.top() + 6.0),
        pos2(outer.right() - AXIS_W, outer.bottom() - AXIS_H),
    );
    let small = FontId::proportional(11.0);

    // Grid: temperatures on the left, percent on the right.
    for i in 0..=7 {
        let temp = 30.0 + i as f32 * 10.0;
        let y = egui::remap(temp, 30.0..=100.0, plot.bottom()..=plot.top());
        painter.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], Stroke::new(1.0, pal.grid));
        if i % 2 == 1 {
            painter.text(
                pos2(plot.left() - 6.0, y),
                Align2::RIGHT_CENTER,
                format!("{temp:.0}°"),
                small.clone(),
                pal.weak,
            );
            let pct = egui::remap(temp, 30.0..=100.0, 0.0..=100.0);
            painter.text(
                pos2(plot.right() + 6.0, y),
                Align2::LEFT_CENTER,
                format!("{pct:.0}%"),
                small.clone(),
                pal.weak,
            );
        }
    }
    for (label, frac) in [("-5m", 0.0), ("-2.5m", 0.5), ("now", 1.0)] {
        let x = egui::lerp(plot.left()..=plot.right(), frac);
        let align = match frac {
            0.0 => Align2::LEFT_TOP,
            1.0 => Align2::RIGHT_TOP,
            _ => Align2::CENTER_TOP,
        };
        painter.text(pos2(x, plot.bottom() + 4.0), align, label, small.clone(), pal.weak);
    }

    let Some(last) = samples.back() else { return };
    let span = HISTORY_LEN as f32; // seconds shown
    let to_x = |t: f32| egui::remap_clamp(t, (last.t - span)..=last.t, plot.left()..=plot.right());
    let temp_y = |v: f32| egui::remap_clamp(v, 30.0..=100.0, plot.bottom()..=plot.top());
    let pct_y = |v: f32| egui::remap_clamp(v, 0.0..=100.0, plot.bottom()..=plot.top());

    let series = |get: &dyn Fn(&Sample) -> Option<f32>, y: &dyn Fn(f32) -> f32, color: Color32| {
        // Split into runs so gaps (e.g. dGPU off) are not bridged.
        let mut run: Vec<Pos2> = Vec::new();
        let flush = |run: &mut Vec<Pos2>| {
            if run.len() >= 2 {
                painter.add(Shape::line(std::mem::take(run), Stroke::new(2.0, color)));
            }
            run.clear();
        };
        for s in samples {
            match get(s) {
                Some(v) => run.push(pos2(to_x(s.t), y(v))),
                None => flush(&mut run),
            }
        }
        flush(&mut run);
    };
    series(&|s| s.duty_percent[0], &pct_y, theme::FAN);
    series(&|s| s.gpu.map(f32::from), &temp_y, theme::GPU);
    series(&|s| s.cpu.map(f32::from), &temp_y, ACCENT);
}

pub fn legend(ui: &mut Ui, pal: &Palette, items: &[(&str, Color32)]) {
    ui.horizontal(|ui| {
        for (label, color) in items {
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
            ui.painter().rect_filled(
                Rect::from_center_size(r.center(), vec2(12.0, 3.0)),
                CornerRadius::same(2),
                *color,
            );
            ui.label(egui::RichText::new(*label).color(pal.weak).size(12.0));
            ui.add_space(8.0);
        }
    });
}

/// Interactive curve editor. `drag` keeps the index of the point being dragged
/// across frames; `live` is the current temperature and, if the tool is driving
/// the fans, the applied speed. Returns true when the curve changed.
pub fn curve_editor(
    ui: &mut Ui,
    pal: &Palette,
    points: &mut Vec<CurvePoint>,
    drag: &mut Option<usize>,
    live: Option<(u8, Option<f32>)>,
    now_label: &str,
    height: f32,
) -> bool {
    let (outer, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
    let painter = ui.painter_at(outer);
    let plot = Rect::from_min_max(
        pos2(outer.left() + AXIS_W + 6.0, outer.top() + 10.0),
        pos2(outer.right() - 22.0, outer.bottom() - AXIS_H - 6.0),
    );
    let small = FontId::proportional(11.5);

    let to_screen = |p: CurvePoint| {
        pos2(
            egui::remap(p.temp, TEMP_MIN..=TEMP_MAX, plot.left()..=plot.right()),
            egui::remap(p.percent, 0.0..=100.0, plot.bottom()..=plot.top()),
        )
    };
    let from_screen = |pos: Pos2| {
        CurvePoint::new(
            egui::remap_clamp(pos.x, plot.left()..=plot.right(), TEMP_MIN..=TEMP_MAX).round(),
            egui::remap_clamp(pos.y, plot.bottom()..=plot.top(), 0.0..=100.0).round(),
        )
    };

    painter.rect_filled(plot, CornerRadius::same(6), ui.visuals().extreme_bg_color);
    let mut t = TEMP_MIN;
    while t <= TEMP_MAX + 0.1 {
        let x = to_screen(CurvePoint::new(t, 0.0)).x;
        painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(1.0, pal.grid));
        painter.text(pos2(x, plot.bottom() + 6.0), Align2::CENTER_TOP, format!("{t:.0}°C"), small.clone(), pal.weak);
        t += 10.0;
    }
    for pct in (0..=100).step_by(20) {
        let y = to_screen(CurvePoint::new(TEMP_MIN, pct as f32)).y;
        painter.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], Stroke::new(1.0, pal.grid));
        painter.text(pos2(plot.left() - 8.0, y), Align2::RIGHT_CENTER, format!("{pct}%"), small.clone(), pal.weak);
    }

    // ---- interaction
    let hit_radius = 14.0;
    let nearest = |pos: Pos2, pts: &[CurvePoint]| {
        pts.iter()
            .enumerate()
            .map(|(i, p)| (i, to_screen(*p).distance(pos)))
            .filter(|(_, d)| *d <= hit_radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    };
    let mut changed = false;
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());

    if resp.drag_started() {
        *drag = pointer.and_then(|p| nearest(p, points));
    }
    if let (Some(i), true, Some(pos)) = (*drag, resp.dragged(), pointer)
        && i < points.len()
    {
        let mut np = from_screen(pos);
        // Keep points ordered: stay between the neighbours.
        let lo = if i > 0 { points[i - 1].temp + 1.0 } else { TEMP_MIN };
        let hi = if i + 1 < points.len() { points[i + 1].temp - 1.0 } else { TEMP_MAX };
        np.temp = np.temp.clamp(lo, hi.max(lo));
        if points[i] != np {
            points[i] = np;
            changed = true;
        }
    }
    if resp.drag_stopped() {
        *drag = None;
        curve::sanitize(points);
        changed = true;
    }
    if resp.double_clicked()
        && points.len() < MAX_POINTS
        && let Some(pos) = pointer
        && nearest(pos, points).is_none()
    {
        let np = from_screen(pos);
        if !points.iter().any(|p| (p.temp - np.temp).abs() < 1.0) {
            points.push(np);
            curve::sanitize(points);
            changed = true;
        }
    }
    if resp.secondary_clicked()
        && points.len() > MIN_POINTS
        && let Some(i) = pointer.and_then(|p| nearest(p, points))
    {
        points.remove(i);
        changed = true;
    }
    let hovered = resp.hover_pos().and_then(|p| nearest(p, points));
    if hovered.is_some() || drag.is_some() {
        ui.ctx().set_cursor_icon(if drag.is_some() { CursorIcon::Grabbing } else { CursorIcon::Grab });
    }

    // ---- curve: area, line, points
    let screen: Vec<Pos2> = {
        let mut v = Vec::with_capacity(points.len() + 2);
        if let Some(first) = points.first() {
            v.push(to_screen(CurvePoint::new(TEMP_MIN, first.percent)));
        }
        v.extend(points.iter().map(|p| to_screen(*p)));
        if let Some(last) = points.last() {
            v.push(to_screen(CurvePoint::new(TEMP_MAX, last.percent)));
        }
        v
    };
    let fill = ACCENT.gamma_multiply(0.16);
    for w in screen.windows(2) {
        let quad = vec![w[0], w[1], pos2(w[1].x, plot.bottom()), pos2(w[0].x, plot.bottom())];
        painter.add(Shape::convex_polygon(quad, fill, Stroke::NONE));
    }
    painter.add(Shape::line(screen, Stroke::new(2.5, ACCENT)));

    // Current operating point.
    if let Some((temp, current_percent)) = live {
        let x = to_screen(CurvePoint::new(f32::from(temp).clamp(TEMP_MIN, TEMP_MAX), 0.0)).x;
        painter.add(Shape::dashed_line(
            &[pos2(x, plot.top()), pos2(x, plot.bottom())],
            Stroke::new(1.5, theme::FAN),
            5.0,
            4.0,
        ));
        let pct = current_percent.unwrap_or_else(|| curve::evaluate(points, f32::from(temp)));
        let p = to_screen(CurvePoint::new(f32::from(temp).clamp(TEMP_MIN, TEMP_MAX), pct));
        painter.circle_filled(p, 5.0, theme::FAN);
        let text = format!("{now_label}  {temp}°C · {pct:.0}%");
        let anchor = if p.x > plot.center().x { Align2::RIGHT_BOTTOM } else { Align2::LEFT_BOTTOM };
        let offset = if p.x > plot.center().x { vec2(-8.0, -8.0) } else { vec2(8.0, -8.0) };
        painter.text(p + offset, anchor, text, FontId::proportional(12.0), theme::FAN);
    }

    for (i, p) in points.iter().enumerate() {
        let c = to_screen(*p);
        let active = Some(i) == *drag || Some(i) == hovered;
        let r = if active { 8.0 } else { 6.0 };
        painter.circle(c, r, ui.visuals().panel_fill, Stroke::new(2.5, ACCENT));
        if active {
            let label = format!("{:.0}°C → {:.0}%", p.temp, p.percent);
            let rect =
                painter.text(c + vec2(0.0, -16.0), Align2::CENTER_BOTTOM, &label, FontId::proportional(12.5), pal.text);
            painter.rect_filled(rect.expand(4.0), CornerRadius::same(4), pal.card.gamma_multiply(0.92));
            painter.text(c + vec2(0.0, -16.0), Align2::CENTER_BOTTOM, label, FontId::proportional(12.5), pal.text);
        }
    }

    changed
}
