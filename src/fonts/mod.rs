//! Bundled FIGlet fonts, embedded directly in the binary.
//!
//! Each font is `include_str!`'d so `sigil` stays a single self-contained
//! executable with no runtime font files to install.

use figlet_rs::FIGfont;

/// A bundled font: its canonical name, alternate spellings, a short
/// description, and the embedded `.flf` source (or `None` for the built-in
/// standard font that ships inside `figlet-rs`).
struct Bundled {
    canonical: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    flf: Option<&'static str>,
}

const TABLE: &[Bundled] = &[
    Bundled {
        canonical: "standard",
        aliases: &["default"],
        description: "the classic FIGlet font",
        flf: None,
    },
    Bundled {
        canonical: "ansishadow",
        aliases: &["ansi-shadow", "shadow"],
        description: "bold block letters with a drop shadow",
        flf: Some(include_str!("ANSI_Shadow.flf")),
    },
    Bundled {
        canonical: "slant",
        aliases: &["italic"],
        description: "slanted, italic-style strokes",
        flf: Some(include_str!("Slant.flf")),
    },
    Bundled {
        canonical: "big",
        aliases: &[],
        description: "tall and chunky",
        flf: Some(include_str!("Big.flf")),
    },
    Bundled {
        canonical: "small",
        aliases: &["mini"],
        description: "compact, space-saving",
        flf: Some(include_str!("Small.flf")),
    },
];

/// A listable font: canonical name and one-line description.
pub struct FontInfo {
    pub name: &'static str,
    pub description: &'static str,
}

/// All bundled fonts, in display order.
pub fn catalog() -> impl Iterator<Item = FontInfo> {
    TABLE.iter().map(|b| FontInfo {
        name: b.canonical,
        description: b.description,
    })
}

/// Comma-separated list of canonical font names, for error messages.
pub fn available() -> String {
    TABLE
        .iter()
        .map(|b| b.canonical)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Load a font by canonical name or alias (case-insensitive).
pub fn load(name: &str) -> Result<FIGfont, String> {
    let q = name.to_ascii_lowercase();
    let entry = TABLE
        .iter()
        .find(|b| b.canonical == q || b.aliases.contains(&q.as_str()));
    match entry {
        Some(b) => match b.flf {
            None => FIGfont::standard(),
            Some(content) => FIGfont::from_content(content),
        },
        None => Err(format!(
            "unknown font: {name}. Available: {}. (custom .flf files are on the roadmap)",
            available()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_font_parses_and_renders() {
        for info in catalog() {
            let font = load(info.name).unwrap_or_else(|e| panic!("{}: {e}", info.name));
            let fig = font
                .convert("Ab1")
                .unwrap_or_else(|| panic!("{} produced no output", info.name));
            assert!(!fig.to_string().trim().is_empty(), "{} empty", info.name);
        }
    }

    #[test]
    fn aliases_resolve() {
        assert!(load("shadow").is_ok());
        assert!(load("ANSI-Shadow").is_ok());
        assert!(load("default").is_ok());
        assert!(load("nope").is_err());
    }
}
