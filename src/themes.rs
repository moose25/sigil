//! Themes: named bundles of coordinated options (font, gradient, border,
//! background, …) applied in one shot with `--theme`.
//!
//! A theme sits between explicit CLI flags and config in precedence:
//! flag > theme > config > built-in default. Every field is optional, so a
//! theme only sets the parts it cares about.

use serde::Deserialize;

/// A coordinated look. All fields optional; unset ones fall through.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub font: Option<String>,
    pub gradient: Option<String>,
    pub colors: Option<String>,
    pub direction: Option<String>,
    pub angle: Option<f32>,
    pub reverse: Option<bool>,
    pub cycle: Option<u32>,
    pub border: Option<String>,
    pub padding: Option<usize>,
    pub border_color: Option<String>,
    pub background: Option<String>,
    pub align: Option<String>,
}

/// A built-in theme by name (case-insensitive), or `None`.
pub fn builtin(name: &str) -> Option<Theme> {
    // Each string is validated downstream when the render resolves.
    let t = |font: &str, gradient: &str, border: &str, background: Option<&str>| Theme {
        font: Some(font.to_string()),
        gradient: Some(gradient.to_string()),
        border: Some(border.to_string()),
        background: background.map(str::to_string),
        ..Theme::default()
    };
    Some(match name.to_ascii_lowercase().as_str() {
        "cyberpunk" => t("ansishadow", "cyberpunk", "heavy", Some("#0d0221")),
        "retro" => t("big", "vaporwave", "double", Some("#160042")),
        "terminal" => t("standard", "matrix", "single", Some("#001000")),
        "fire" => t("slant", "fire", "none", None),
        "ocean" => t("small", "ocean", "round", Some("#031420")),
        "gold" => t("ansishadow", "gold", "double", Some("#1a1400")),
        _ => return None,
    })
}

/// Names of all built-in themes, in display order.
pub fn builtin_names() -> &'static [&'static str] {
    &["cyberpunk", "retro", "terminal", "fire", "ocean", "gold"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_resolve() {
        for name in builtin_names() {
            let th = builtin(name).unwrap_or_else(|| panic!("missing {name}"));
            assert!(th.font.is_some() && th.gradient.is_some());
        }
        assert!(builtin("nope").is_none());
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert!(builtin("CyberPunk").is_some());
    }
}
