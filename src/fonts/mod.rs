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

/// Load a font by bundled name/alias, a path to a `.flf` file, or a name in
/// the user fonts directory (`$XDG_CONFIG_HOME/sigil/fonts` or
/// `~/.config/sigil/fonts`).
pub fn load(name: &str) -> Result<FIGfont, String> {
    let q = name.to_ascii_lowercase();
    if let Some(b) = TABLE
        .iter()
        .find(|b| b.canonical == q || b.aliases.contains(&q.as_str()))
    {
        return match b.flf {
            None => FIGfont::standard(),
            Some(content) => parse_flf(content),
        };
    }

    // An explicit path to a .flf file.
    let path = std::path::Path::new(name);
    if name.ends_with(".flf") || path.is_file() {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("cannot read font {name}: {e}"))?;
        return parse_flf(&content);
    }

    // A named font in the user fonts directory.
    if let Some(dir) = user_fonts_dir() {
        let p = dir.join(format!("{name}.flf"));
        if p.is_file() {
            let content = std::fs::read_to_string(&p)
                .map_err(|e| format!("cannot read font {}: {e}", p.display()))?;
            return parse_flf(&content);
        }
    }

    Err(format!(
        "unknown font: {name}. Available: {} (or pass a path to a .flf file)",
        available()
    ))
}

/// Parse `.flf` content, first trimming it to the 102 required characters so
/// that fonts carrying code-tagged extra glyphs still parse.
fn parse_flf(content: &str) -> Result<FIGfont, String> {
    FIGfont::from_content(&trim_to_required(content))
        .map_err(|e| format!("invalid figlet font: {e}"))
}

/// Keep only the header, comment lines, and the 102 required characters
/// (95 printable ASCII + 7 German), dropping any code-tagged glyphs that some
/// `.flf` files append and that the parser can choke on.
fn trim_to_required(content: &str) -> String {
    const REQUIRED: usize = 102;
    let lines: Vec<&str> = content.lines().collect();
    let header: Vec<&str> = match lines.first() {
        Some(h) => h.split_whitespace().collect(),
        None => return content.to_string(),
    };
    let height = header.get(1).and_then(|s| s.parse::<usize>().ok());
    let comments = header.get(5).and_then(|s| s.parse::<usize>().ok());
    match (height, comments) {
        (Some(h), Some(c)) => {
            let keep = (1 + c + REQUIRED * h).min(lines.len());
            let mut out = lines[..keep].join("\n");
            out.push('\n');
            out
        }
        // Malformed header: let the parser report the real error.
        _ => content.to_string(),
    }
}

/// The user fonts directory, if a home/config location can be determined.
fn user_fonts_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("sigil").join("fonts"))
}

/// Names (file stems) of any `.flf` fonts in the user fonts directory, sorted.
pub fn user_font_names() -> Vec<String> {
    let Some(dir) = user_fonts_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "flf"))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
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

    #[test]
    fn trim_drops_codetag_glyphs() {
        // height=1, comments=0 => keep header + 102 required lines.
        let mut content = String::from("flf2a$ 1 1 5 0 0\n");
        for i in 0..102 {
            content.push_str(&format!("row{i}@@\n"));
        }
        // Extra code-tagged glyphs that must be dropped.
        content.push_str("160  NO-BREAK SPACE\n $@@\n");
        let trimmed = trim_to_required(&content);
        assert_eq!(trimmed.lines().count(), 1 + 102);
        assert!(!trimmed.contains("NO-BREAK SPACE"));
    }

    #[test]
    fn missing_font_path_errors_clearly() {
        let err = load("/no/such/font.flf").unwrap_err();
        assert!(err.contains("cannot read font"));
    }
}
