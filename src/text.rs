//! Input-text sanitization.
//!
//! FIGlet fonts only cover a small character set (printable ASCII plus a few
//! Latin-1 letters), and the parser errors on anything it can't render. To keep
//! `sigil` friendly, we fold input down to renderable ASCII: accented Latin
//! letters lose their marks (`café` → `cafe`), a handful of ligatures are
//! spelled out (`ß` → `ss`), and anything still unrenderable (CJK, emoji)
//! becomes `?` rather than crashing.

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Fold `text` to a FIGlet-renderable ASCII string.
pub fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.nfkd() {
        if is_combining_mark(c) {
            continue; // dropped accent from a decomposed letter
        }
        if c.is_ascii_graphic() || c == ' ' {
            out.push(c);
        } else if let Some(rep) = spell_out(c) {
            out.push_str(rep);
        } else if c.is_whitespace() {
            out.push(' ');
        } else {
            out.push('?');
        }
    }
    out
}

/// Spell out a few common non-decomposable Latin letters and ligatures.
fn spell_out(c: char) -> Option<&'static str> {
    Some(match c {
        'ß' => "ss",
        'æ' => "ae",
        'Æ' => "AE",
        'œ' => "oe",
        'Œ' => "OE",
        'ø' => "o",
        'Ø' => "O",
        'ł' => "l",
        'Ł' => "L",
        'đ' | 'ð' => "d",
        'Đ' | 'Ð' => "D",
        'þ' => "th",
        'Þ' => "TH",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_passes_through() {
        assert_eq!(sanitize("Hello, World! 123"), "Hello, World! 123");
    }

    #[test]
    fn accents_are_folded() {
        assert_eq!(sanitize("café"), "cafe");
        assert_eq!(sanitize("naïve"), "naive");
        assert_eq!(sanitize("Zürich"), "Zurich");
    }

    #[test]
    fn ligatures_spelled_out() {
        assert_eq!(sanitize("straße"), "strasse");
        assert_eq!(sanitize("æon"), "aeon");
    }

    #[test]
    fn unrenderable_becomes_placeholder() {
        assert_eq!(sanitize("日本"), "??");
        // A wave emoji (written as an escape so no literal emoji lives in source).
        assert_eq!(sanitize("Hi\u{1f44b}"), "Hi?");
    }
}
