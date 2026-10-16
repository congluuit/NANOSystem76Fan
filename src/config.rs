//! User settings, stored as TOML.
//!
//! Location: `%APPDATA%\S76FanControl\config.toml`, or `config.toml` next to the
//! executable if that file exists (portable mode).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::curve::{self, CurvePoint, Preset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The EC firmware controls the fans.
    Auto,
    /// One constant speed.
    Fixed,
    /// Speed follows the user curve.
    #[default]
    Curve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    System,
    En,
    Vi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mode: Mode,
    /// 0..=100 %
    pub fixed_percent: f32,
    pub curve: Vec<CurvePoint>,
    /// Above this temperature control is handed back to the EC.
    pub safety_temp: f32,
    /// Lowest speed the tool will ever request in Fixed/Curve mode.
    pub min_percent: f32,
    /// How fast the speed may drop, in % per second (rising is immediate).
    pub ramp_down_per_sec: f32,
    pub language: Language,
    pub theme: Theme,
    pub close_to_tray: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Curve,
            fixed_percent: 40.0,
            curve: Preset::Balanced.points(),
            safety_temp: 92.0,
            min_percent: 20.0,
            ramp_down_per_sec: 3.0,
            language: Language::System,
            theme: Theme::Dark,
            close_to_tray: true,
        }
    }
}

pub const SAFETY_TEMP_RANGE: std::ops::RangeInclusive<f32> = 75.0..=98.0;

impl Config {
    pub fn sanitize(&mut self) {
        self.fixed_percent = self.fixed_percent.clamp(0.0, 100.0);
        self.min_percent = self.min_percent.clamp(0.0, 60.0);
        self.safety_temp = self.safety_temp.clamp(*SAFETY_TEMP_RANGE.start(), *SAFETY_TEMP_RANGE.end());
        self.ramp_down_per_sec = self.ramp_down_per_sec.clamp(0.5, 50.0);
        curve::sanitize(&mut self.curve);
    }

    pub fn path() -> PathBuf {
        let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
        if let Some(dir) = &exe_dir {
            let portable = dir.join("config.toml");
            if portable.is_file() {
                return portable;
            }
        }
        match std::env::var_os("APPDATA") {
            Some(appdata) => PathBuf::from(appdata).join("S76FanControl").join("config.toml"),
            None => exe_dir.unwrap_or_default().join("config.toml"),
        }
    }

    pub fn load() -> Self {
        let mut cfg = std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| toml::from_str::<Config>(&s).ok())
            .unwrap_or_default();
        cfg.sanitize();
        cfg
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        // Write-then-rename so a crash never leaves a half-written file.
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let cfg = Config::default();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let cfg: Config = toml::from_str("mode = \"fixed\"\nfixed_percent = 55.0\n").unwrap();
        assert_eq!(cfg.mode, Mode::Fixed);
        assert_eq!(cfg.fixed_percent, 55.0);
        assert_eq!(cfg.safety_temp, Config::default().safety_temp);
    }

    #[test]
    fn sanitize_clamps() {
        let mut cfg = Config { fixed_percent: 140.0, safety_temp: 200.0, ..Default::default() };
        cfg.sanitize();
        assert_eq!(cfg.fixed_percent, 100.0);
        assert_eq!(cfg.safety_temp, *SAFETY_TEMP_RANGE.end());
    }
}
