//! Temperature → fan speed curves.

use serde::{Deserialize, Serialize};

pub const TEMP_MIN: f32 = 30.0;
pub const TEMP_MAX: f32 = 100.0;
pub const MAX_POINTS: usize = 10;
pub const MIN_POINTS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    /// °C
    pub temp: f32,
    /// 0..=100 %
    pub percent: f32,
}

impl CurvePoint {
    pub const fn new(temp: f32, percent: f32) -> Self {
        Self { temp, percent }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Silent,
    Balanced,
    Performance,
    /// The curve built into the addw3 EC firmware.
    Firmware,
}

impl Preset {
    pub const ALL: [Preset; 4] = [Preset::Silent, Preset::Balanced, Preset::Performance, Preset::Firmware];

    pub fn points(self) -> Vec<CurvePoint> {
        let p = CurvePoint::new;
        match self {
            Preset::Silent => vec![p(50.0, 20.0), p(65.0, 25.0), p(75.0, 40.0), p(85.0, 65.0), p(92.0, 100.0)],
            Preset::Balanced => {
                vec![p(50.0, 25.0), p(60.0, 30.0), p(70.0, 45.0), p(78.0, 65.0), p(85.0, 85.0), p(90.0, 100.0)]
            }
            Preset::Performance => vec![p(45.0, 30.0), p(55.0, 45.0), p(65.0, 65.0), p(75.0, 85.0), p(82.0, 100.0)],
            Preset::Firmware => vec![p(60.0, 40.0), p(65.0, 60.0), p(70.0, 75.0), p(75.0, 90.0), p(80.0, 100.0)],
        }
    }
}

/// Linear interpolation; flat before the first and after the last point.
/// `points` must be sorted by temperature (see [`sanitize`]).
pub fn evaluate(points: &[CurvePoint], temp: f32) -> f32 {
    let Some(first) = points.first() else { return 100.0 };
    if temp <= first.temp {
        return first.percent;
    }
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        if temp <= b.temp {
            let span = b.temp - a.temp;
            if span <= f32::EPSILON {
                return b.percent;
            }
            return a.percent + (b.percent - a.percent) * (temp - a.temp) / span;
        }
    }
    points[points.len() - 1].percent
}

/// Clamps values into range, sorts by temperature, enforces a monotonic
/// (non-decreasing) speed and the point-count limits.
pub fn sanitize(points: &mut Vec<CurvePoint>) {
    points.retain(|p| p.temp.is_finite() && p.percent.is_finite());
    for p in points.iter_mut() {
        p.temp = p.temp.clamp(TEMP_MIN, TEMP_MAX);
        p.percent = p.percent.clamp(0.0, 100.0);
    }
    points.sort_by(|a, b| a.temp.total_cmp(&b.temp));
    points.truncate(MAX_POINTS);
    if points.len() < MIN_POINTS {
        *points = Preset::Balanced.points();
    }
    // A hotter point must never ask for less airflow than a cooler one.
    let mut floor = 0.0f32;
    for p in points.iter_mut() {
        p.percent = p.percent.max(floor);
        floor = p.percent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> Vec<CurvePoint> {
        vec![CurvePoint::new(50.0, 20.0), CurvePoint::new(70.0, 60.0), CurvePoint::new(90.0, 100.0)]
    }

    #[test]
    fn flat_outside_range() {
        assert_eq!(evaluate(&curve(), 10.0), 20.0);
        assert_eq!(evaluate(&curve(), 99.0), 100.0);
    }

    #[test]
    fn interpolates() {
        assert_eq!(evaluate(&curve(), 60.0), 40.0);
        assert_eq!(evaluate(&curve(), 80.0), 80.0);
        assert_eq!(evaluate(&curve(), 70.0), 60.0);
    }

    #[test]
    fn empty_curve_is_full_speed() {
        assert_eq!(evaluate(&[], 40.0), 100.0);
    }

    #[test]
    fn sanitize_sorts_clamps_and_is_monotonic() {
        let mut c = vec![
            CurvePoint::new(120.0, 150.0),
            CurvePoint::new(60.0, 50.0),
            CurvePoint::new(70.0, 30.0),
            CurvePoint::new(10.0, -5.0),
        ];
        sanitize(&mut c);
        assert_eq!(c.first().unwrap().temp, TEMP_MIN);
        assert_eq!(c.last().unwrap().temp, TEMP_MAX);
        assert_eq!(c.last().unwrap().percent, 100.0);
        assert!(c.windows(2).all(|w| w[0].temp <= w[1].temp && w[0].percent <= w[1].percent));
    }

    #[test]
    fn sanitize_restores_too_short_curve() {
        let mut c = vec![CurvePoint::new(60.0, 50.0)];
        sanitize(&mut c);
        assert_eq!(c, Preset::Balanced.points());
    }

    #[test]
    fn presets_are_valid() {
        for p in Preset::ALL {
            let mut pts = p.points();
            let orig = pts.clone();
            sanitize(&mut pts);
            assert_eq!(pts, orig, "{p:?}");
        }
    }
}
