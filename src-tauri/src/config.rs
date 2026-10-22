//! Persisted settings (`config.toml`).
//!
//! Loading never fails: a missing file yields defaults, a corrupt file is backed up to
//! `config.toml.corrupt` and individual sections that still parse are kept.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const MAX_RECENT: usize = 10;
pub const MIN_FONT_SIZE: u32 = 10;
pub const MAX_FONT_SIZE: u32 = 32;
pub const DEFAULT_FONT_SIZE: u32 = 16;
pub const WIDTHS: [&str; 4] = ["narrow", "medium", "wide", "full"];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiSettings,
    pub window: WindowGeometry,
    pub recent_files: Vec<PathBuf>,
}

/// Settings owned by the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// A `data-theme` name from themes.css, or `"system"`.
    pub theme: String,
    pub body_font: String,
    pub code_font: String,
    pub font_size: u32,
    /// One of [`WIDTHS`].
    pub content_width: String,
    pub toc_open: bool,
}

/// Logical (DPI-independent) window geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowGeometry {
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub maximized: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            body_font: "inter".into(),
            code_font: "jetbrains-mono".into(),
            font_size: DEFAULT_FONT_SIZE,
            content_width: "medium".into(),
            toc_open: true,
        }
    }
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            width: 1100.0,
            height: 800.0,
            x: None,
            y: None,
            maximized: false,
        }
    }
}

impl UiSettings {
    /// Clamp/repair values that would break the UI.
    pub fn sanitized(mut self) -> Self {
        let d = Self::default();
        let ident = |s: &str| {
            !s.is_empty()
                && s.len() <= 64
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        };
        if !ident(&self.theme) {
            self.theme = d.theme;
        }
        if !ident(&self.body_font) {
            self.body_font = d.body_font;
        }
        if !ident(&self.code_font) {
            self.code_font = d.code_font;
        }
        self.font_size = self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        if !WIDTHS.contains(&self.content_width.as_str()) {
            self.content_width = d.content_width;
        }
        self
    }
}

impl WindowGeometry {
    pub fn sanitized(mut self) -> Self {
        let d = Self::default();
        let ok = |v: f64| v.is_finite() && (200.0..=20_000.0).contains(&v);
        if !ok(self.width) || !ok(self.height) {
            self.width = d.width;
            self.height = d.height;
        }
        let pos_ok = |v: Option<f64>| v.is_none_or(|v| v.is_finite() && v.abs() < 100_000.0);
        if !pos_ok(self.x) || !pos_ok(self.y) || self.x.is_some() != self.y.is_some() {
            self.x = None;
            self.y = None;
        }
        self
    }
}

impl Config {
    pub fn sanitized(mut self) -> Self {
        self.ui = self.ui.sanitized();
        self.window = self.window.sanitized();
        let mut seen = Vec::new();
        self.recent_files.retain(|p| {
            let keep = !p.as_os_str().is_empty() && !seen.contains(p);
            seen.push(p.clone());
            keep
        });
        self.recent_files.truncate(MAX_RECENT);
        self
    }

    /// Move `path` to the front of the recent list.
    pub fn push_recent(&mut self, path: &Path) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(MAX_RECENT);
    }

    /// Parse TOML text. Never fails: unparseable sections fall back to their defaults.
    /// Returns the config and whether anything had to be discarded.
    pub fn from_toml_lenient(text: &str) -> (Self, bool) {
        if let Ok(cfg) = toml::from_str::<Config>(text) {
            return (cfg.sanitized(), false);
        }
        // Salvage what we can, section by section.
        let Ok(table) = text.parse::<toml::Table>() else {
            return (Self::default(), true);
        };
        let section = |key: &str| table.get(key).cloned();
        let mut cfg = Self::default();
        if let Some(v) = section("ui") {
            cfg.ui = v.try_into().unwrap_or_default();
        }
        if let Some(v) = section("window") {
            cfg.window = v.try_into().unwrap_or_default();
        }
        if let Some(v) = section("recent_files") {
            cfg.recent_files = v.try_into().unwrap_or_default();
        }
        (cfg.sanitized(), true)
    }

    /// Load from `path`. Missing → defaults. Corrupt → defaults (salvaging valid sections)
    /// and the bad file is copied to `<path>.corrupt` for the user to inspect.
    pub fn load(path: &Path) -> Self {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                eprintln!(
                    "lectern: cannot read {}: {e}; using defaults",
                    path.display()
                );
                return Self::default();
            }
        };
        let (cfg, damaged) = Self::from_toml_lenient(&text);
        if damaged {
            let backup = path.with_extension("toml.corrupt");
            eprintln!(
                "lectern: {} is invalid; using defaults (backup: {})",
                path.display(),
                backup.display()
            );
            let _ = fs::copy(path, &backup);
        }
        cfg
    }

    /// Atomically write to `path` (temp file + rename), creating parent directories.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, path)
    }
}

/// Platform config path: `~/.config/lectern/config.toml` on Linux,
/// `%APPDATA%\lectern\config\config.toml` on Windows.
pub fn default_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "lectern").map(|d| d.config_dir().join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Config::load(&dir.path().join("nope.toml")),
            Config::default()
        );
    }

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("config.toml");
        let mut cfg = Config::default();
        cfg.ui.theme = "nord".into();
        cfg.ui.font_size = 19;
        cfg.window.x = Some(10.0);
        cfg.window.y = Some(20.0);
        cfg.push_recent(Path::new("/a.md"));
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path), cfg);
    }

    #[test]
    fn garbage_falls_back_to_defaults_and_backs_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "this is = = not toml [[[").unwrap();
        assert_eq!(Config::load(&path), Config::default());
        assert!(path.with_extension("toml.corrupt").exists());
    }

    #[test]
    fn wrong_types_salvage_other_sections() {
        let text = r#"
            recent_files = ["/x.md"]
            [ui]
            font_size = "huge"
            [window]
            width = 900.0
            height = 700.0
        "#;
        let (cfg, damaged) = Config::from_toml_lenient(text);
        assert!(damaged);
        assert_eq!(cfg.ui, UiSettings::default());
        assert_eq!(cfg.window.width, 900.0);
        assert_eq!(cfg.recent_files, vec![PathBuf::from("/x.md")]);
    }

    #[test]
    fn partial_file_fills_defaults() {
        let (cfg, damaged) = Config::from_toml_lenient("[ui]\ntheme = \"sepia\"\n");
        assert!(!damaged);
        assert_eq!(cfg.ui.theme, "sepia");
        assert_eq!(cfg.ui.font_size, DEFAULT_FONT_SIZE);
        assert_eq!(cfg.window, WindowGeometry::default());
    }

    #[test]
    fn out_of_range_values_are_repaired() {
        let text = "[ui]\nfont_size = 400\ncontent_width = \"enormous\"\ntheme = \"<script>\"\n\
                    [window]\nwidth = -5.0\nheight = inf\nx = 5.0\n";
        let (cfg, _) = Config::from_toml_lenient(text);
        assert_eq!(cfg.ui.font_size, MAX_FONT_SIZE);
        assert_eq!(cfg.ui.content_width, "medium");
        assert_eq!(cfg.ui.theme, "system");
        assert_eq!(cfg.window.width, 1100.0);
        assert_eq!(cfg.window.x, None, "x without y is dropped");
    }

    #[test]
    fn recent_files_dedup_and_cap() {
        let mut cfg = Config::default();
        for i in 0..15 {
            cfg.push_recent(Path::new(&format!("/f{i}.md")));
        }
        cfg.push_recent(Path::new("/f3.md"));
        assert_eq!(cfg.recent_files.len(), MAX_RECENT);
        assert_eq!(cfg.recent_files[0], PathBuf::from("/f3.md"));
        assert_eq!(
            cfg.recent_files
                .iter()
                .filter(|p| **p == Path::new("/f3.md"))
                .count(),
            1
        );
    }
}
