//! Optional config files that set default options.
//!
//! Two files are read and merged, lowest precedence first:
//! 1. user config: `$XDG_CONFIG_HOME/sigil/config.toml` (or `~/.config/...`)
//! 2. project config: `.sigil.toml` in the current directory
//!
//! Command-line flags then override both. Every field is optional.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Default options loaded from config files. Absent fields stay `None` and fall
/// back to the built-in defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub gradient: Option<String>,
    pub colors: Option<String>,
    pub font: Option<String>,
    pub direction: Option<String>,
    pub align: Option<String>,
    pub color_by: Option<String>,
    pub angle: Option<f32>,
    pub reverse: Option<bool>,
    pub cycle: Option<u32>,
    pub border: Option<String>,
    pub padding: Option<usize>,
    pub border_color: Option<String>,
    pub background: Option<String>,
    pub shadow: Option<bool>,
    pub shadow_color: Option<String>,
    pub title: Option<String>,
    pub margin: Option<usize>,
    pub width: Option<usize>,
    pub animate: Option<String>,
    pub fps: Option<u32>,
    pub format: Option<String>,
    /// User-defined named gradients: name -> list of hex stops.
    pub gradients: HashMap<String, Vec<String>>,
    /// User-defined themes: name -> coordinated option bundle.
    pub themes: HashMap<String, crate::themes::Theme>,
}

impl Config {
    /// Load and merge the user and project config files.
    pub fn load() -> Result<Config, String> {
        let mut cfg = Config::default();
        if let Some(p) = user_config_path() {
            cfg.merge_file(&p)?;
        }
        cfg.merge_file(Path::new(".sigil.toml"))?;
        Ok(cfg)
    }

    /// Merge a single file's settings over the current ones. A missing file is
    /// not an error; a malformed one is.
    fn merge_file(&mut self, path: &Path) -> Result<(), String> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("cannot read config {}: {e}", path.display())),
        };
        let other: Config =
            toml::from_str(&text).map_err(|e| format!("invalid config {}: {e}", path.display()))?;
        self.merge(other);
        Ok(())
    }

    /// Overlay `other` (higher precedence) onto `self`.
    fn merge(&mut self, other: Config) {
        macro_rules! overlay {
            ($($field:ident),* $(,)?) => {
                $( if other.$field.is_some() { self.$field = other.$field; } )*
            };
        }
        overlay!(
            gradient,
            colors,
            font,
            direction,
            align,
            color_by,
            angle,
            reverse,
            cycle,
            border,
            padding,
            border_color,
            background,
            shadow,
            shadow_color,
            title,
            margin,
            width,
            animate,
            fps,
            format,
        );
        // Named gradients and themes merge per-key (project entries win).
        for (name, stops) in other.gradients {
            self.gradients.insert(name, stops);
        }
        for (name, theme) in other.themes {
            self.themes.insert(name, theme);
        }
    }
}

fn user_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("sigil").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_fields() {
        let cfg: Config = toml::from_str("gradient = \"sunset\"\nfont = \"slant\"\ncycle = 3\n")
            .expect("valid toml");
        assert_eq!(cfg.gradient.as_deref(), Some("sunset"));
        assert_eq!(cfg.font.as_deref(), Some("slant"));
        assert_eq!(cfg.cycle, Some(3));
        assert_eq!(cfg.border, None);
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(toml::from_str::<Config>("wobble = true\n").is_err());
    }

    #[test]
    fn parses_user_themes() {
        let cfg: Config =
            toml::from_str("[themes.brandy]\nfont = \"slant\"\ngradient = \"gold\"\n").unwrap();
        let t = cfg.themes.get("brandy").expect("theme present");
        assert_eq!(t.font.as_deref(), Some("slant"));
        assert_eq!(t.gradient.as_deref(), Some("gold"));
        assert_eq!(t.border, None);
    }

    #[test]
    fn parses_custom_gradients() {
        let cfg: Config =
            toml::from_str("[gradients]\nbrand = [\"#0f2027\", \"#2c5364\"]\n").unwrap();
        assert_eq!(
            cfg.gradients.get("brand").map(Vec::as_slice),
            Some(["#0f2027".to_string(), "#2c5364".to_string()].as_slice())
        );
    }

    #[test]
    fn merge_prefers_other() {
        let mut base = Config {
            gradient: Some("ocean".into()),
            font: Some("standard".into()),
            ..Config::default()
        };
        base.merge(Config {
            gradient: Some("fire".into()),
            ..Config::default()
        });
        assert_eq!(base.gradient.as_deref(), Some("fire")); // overridden
        assert_eq!(base.font.as_deref(), Some("standard")); // preserved
    }
}
