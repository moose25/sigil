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
    pub shadow: Option<bool>,
    pub shadow_color: Option<String>,
    pub outline: Option<bool>,
    pub outline_color: Option<String>,
}

/// A built-in theme by name (case-insensitive), or `None`.
pub fn builtin(name: &str) -> Option<Theme> {
    // Strings are validated downstream when the render resolves.
    let base = |font: &str, gradient: &str, border: &str, background: Option<&str>| Theme {
        font: Some(font.to_string()),
        gradient: Some(gradient.to_string()),
        border: Some(border.to_string()),
        background: background.map(str::to_string),
        ..Theme::default()
    };
    Some(match name.to_ascii_lowercase().as_str() {
        "cyberpunk" => base("ansishadow", "cyberpunk", "heavy", Some("#0d0221")),
        "retro" => base("big", "vaporwave", "double", Some("#160042")),
        "terminal" => base("standard", "matrix", "single", Some("#001000")),
        "fire" => base("slant", "fire", "none", None),
        "ocean" => base("small", "ocean", "round", Some("#031420")),
        "gold" => base("ansishadow", "gold", "double", Some("#1a1400")),
        "mono" => Theme {
            outline: Some(true),
            ..base("ansiregular", "mono", "single", Some("#0a0a0a"))
        },
        "sunset" => Theme {
            shadow: Some(true),
            ..base("slant", "sunset", "none", Some("#1a0a12"))
        },
        "forest" => Theme {
            colors: Some("#0b3d2e,#2e8b57,#a8d08d".to_string()),
            ..base("doom", "mint", "heavy", Some("#03150e"))
        },
        "candy" => Theme {
            outline: Some(true),
            outline_color: Some("#3a0d2a".to_string()),
            ..base("bloody", "flamingo", "round", Some("#1a0410"))
        },
        "midnight" => Theme {
            shadow: Some(true),
            ..base("ansishadow", "ice", "double", Some("#05060f"))
        },
        "synthwave" => Theme {
            shadow: Some(true),
            ..base("ansishadow", "vaporwave", "heavy", Some("#1a0033"))
        },
        "arctic" => Theme {
            outline: Some(true),
            outline_color: Some("#062033".to_string()),
            ..base("small", "glacier", "round", Some("#04141f"))
        },
        "sepia" => Theme {
            colors: Some("#5b3a1a,#a67b5b,#e0c9a6".to_string()),
            ..base("slant", "gold", "single", Some("#140f0a"))
        },
        _ => return None,
    })
}

/// Names of all built-in themes, in display order.
pub fn builtin_names() -> &'static [&'static str] {
    &[
        "cyberpunk",
        "retro",
        "terminal",
        "fire",
        "ocean",
        "gold",
        "mono",
        "sunset",
        "forest",
        "candy",
        "midnight",
        "synthwave",
        "arctic",
        "sepia",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_resolve() {
        for name in builtin_names() {
            let th = builtin(name).unwrap_or_else(|| panic!("missing {name}"));
            assert!(th.font.is_some() && th.gradient.is_some());
            // Each theme must name a real gradient preset (unless it overrides
            // with explicit colors), so a typo can't ship a broken theme.
            if th.colors.is_none() {
                let g = th.gradient.as_deref().unwrap();
                assert!(
                    crate::gradient::Gradient::preset(g).is_some(),
                    "theme {name} references unknown gradient {g}"
                );
            }
        }
        assert!(builtin("nope").is_none());
        // Effect-bearing themes set their effects.
        assert_eq!(builtin("midnight").unwrap().shadow, Some(true));
        assert_eq!(builtin("mono").unwrap().outline, Some(true));
        assert_eq!(builtin("synthwave").unwrap().shadow, Some(true));
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert!(builtin("CyberPunk").is_some());
    }
}
