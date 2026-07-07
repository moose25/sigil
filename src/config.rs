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
    pub interpolate: Option<String>,
    pub angle: Option<f32>,
    pub reverse: Option<bool>,
    pub cycle: Option<u32>,
    pub border: Option<String>,
    pub padding: Option<usize>,
    pub pad_x: Option<usize>,
    pub pad_y: Option<usize>,
    pub border_color: Option<String>,
    pub background: Option<String>,
    pub shadow: Option<bool>,
    pub shadow_color: Option<String>,
    pub outline: Option<bool>,
    pub outline_color: Option<String>,
    pub scale: Option<usize>,
    pub title: Option<String>,
    pub margin: Option<usize>,
    pub margin_x: Option<usize>,
    pub min_width: Option<usize>,
    pub wrap: Option<usize>,
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
            interpolate,
            angle,
            reverse,
            cycle,
            border,
            padding,
            pad_x,
            pad_y,
            border_color,
            background,
            shadow,
            shadow_color,
            outline,
            outline_color,
            scale,
            title,
            margin,
            margin_x,
            min_width,
            wrap,
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

/// Path to the user config file (`$XDG_CONFIG_HOME/sigil/config.toml` or
/// `~/.config/sigil/config.toml`), if a home/config location is known.
pub fn user_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("sigil").join("config.toml"))
}

/// A commented starter config, written by `sigil init`.
pub const STARTER: &str = "\
# sigil config — https://github.com/moose25/sigil
# Every option is optional; CLI flags override these. Uncomment to use.

# --- color ---
# gradient    = \"vaporwave\"      # a preset, or define one under [gradients]
# colors      = \"#ff5f6d,#ffc371\" # custom stops (overrides gradient)
# interpolate = \"oklab\"          # oklab | rgb | hsl
# color_by    = \"banner\"         # banner | line | char
# direction   = \"horizontal\"     # horizontal | vertical | diagonal
# angle       = 45                # degrees (overrides direction)
# reverse     = false
# cycle       = 1                 # repeat the palette N times

# --- font & layout ---
# font    = \"ansishadow\"
# align   = \"center\"             # left | center | right
# margin  = 0                     # blank lines above/below
# width   = 80                    # alignment width (default: terminal)
# wrap    = 60                    # word-wrap long text to fit N columns

# --- frame & effects ---
# border       = \"round\"         # none | round | single | double | heavy | ascii
# padding      = 1
# border_color = \"#8a2be2\"
# title        = \"my project\"    # caption in the top border
# background   = \"#0d1117\"
# shadow       = false
# shadow_color = \"#1c1c22\"
# outline      = false
# outline_color = \"#0a0a0c\"

# --- output ---
# format  = \"term\"              # term | ansi | raw | rust | go | python | shell | svg | html | png | json
# animate = \"none\"              # none | sweep | type | pulse | scroll
# fps     = 30
# scale   = 1                    # PNG resolution multiplier (1-10)

# Your own named gradients — use with `-g brand`:
# [gradients]
# brand = [\"#0f2027\", \"#203a43\", \"#2c5364\"]

# Your own themes — use with `--theme mine`:
# [themes.mine]
# font = \"slant\"
# gradient = \"gold\"
# border = \"double\"
# shadow = true
";

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
    fn starter_config_is_valid() {
        // The starter is fully commented, so it parses to an empty config.
        let cfg: Config = toml::from_str(STARTER).expect("starter parses");
        assert!(cfg.gradient.is_none() && cfg.font.is_none());
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
