//! Turning text into a colored ASCII banner.

use figlet_rs::FIGfont;

use crate::color::{ColorMode, Rgb};
use crate::gradient::{Direction, Gradient};

/// Horizontal placement of the banner within the target width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    pub fn parse(s: &str) -> Result<Align, String> {
        match s.to_ascii_lowercase().as_str() {
            "left" | "l" => Ok(Align::Left),
            "center" | "centre" | "c" => Ok(Align::Center),
            "right" | "r" => Ok(Align::Right),
            _ => Err(format!("unknown alignment: {s} (left|center|right)")),
        }
    }
}

/// A laid-out banner: glyph rows padded to a common width, no color yet.
#[derive(Clone, Debug)]
pub struct Banner {
    pub lines: Vec<String>,
    pub width: usize,
}

impl Banner {
    /// Render `text` through `font` into padded glyph rows.
    pub fn layout(font: &FIGfont, text: &str) -> Result<Banner, String> {
        let figure = font
            .convert(text)
            .ok_or_else(|| format!("could not render {text:?} with this font"))?;
        let raw = figure.to_string();

        let mut lines: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
        // Trim blank lines top and bottom; FIGlet output is often padded.
        while lines.first().is_some_and(|l| l.trim().is_empty()) {
            lines.remove(0);
        }
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(String::new());
        }

        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        for l in &mut lines {
            let pad = width - l.chars().count();
            if pad > 0 {
                l.push_str(&" ".repeat(pad));
            }
        }
        Ok(Banner { lines, width })
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }
}

/// Everything needed to paint a banner to an ANSI string.
pub struct RenderOptions {
    pub gradient: Gradient,
    pub direction: Direction,
    pub align: Align,
    pub mode: ColorMode,
    /// Column count to align within (e.g. terminal width).
    pub target_width: usize,
    /// Extra blank lines above and below the banner.
    pub margin_y: usize,
}

/// Paint `banner` into a printable string with ANSI color escapes.
pub fn paint(banner: &Banner, opts: &RenderOptions) -> String {
    let rows = banner.height();
    let cols = banner.width;

    let slack = opts.target_width.saturating_sub(cols);
    let indent = match opts.align {
        Align::Left => 0,
        Align::Center => slack / 2,
        Align::Right => slack,
    };
    let pad = " ".repeat(indent);

    let mut out = String::new();
    for _ in 0..opts.margin_y {
        out.push('\n');
    }
    for (r, line) in banner.lines.iter().enumerate() {
        out.push_str(&pad);
        let mut last: Option<Rgb> = None;
        for (c, ch) in line.chars().enumerate() {
            if ch == ' ' {
                out.push(' ');
                continue;
            }
            let t = opts.direction.t(r, c, rows, cols);
            let color = opts.gradient.sample(t);
            if opts.mode != ColorMode::None && last != Some(color) {
                out.push_str(&opts.mode.fg(color));
                last = Some(color);
            }
            out.push(ch);
        }
        out.push_str(opts.mode.reset());
        out.push('\n');
    }
    for _ in 0..opts.margin_y {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> FIGfont {
        FIGfont::standard().unwrap()
    }

    #[test]
    fn layout_pads_to_common_width() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        assert!(b.height() > 1);
        assert!(b.lines.iter().all(|l| l.chars().count() == b.width));
    }

    #[test]
    fn paint_no_color_has_no_escapes() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let opts = RenderOptions {
            gradient: Gradient::preset("ocean").unwrap(),
            direction: Direction::Horizontal,
            align: Align::Left,
            mode: ColorMode::None,
            target_width: 80,
            margin_y: 0,
        };
        let out = paint(&b, &opts);
        assert!(!out.contains('\x1b'));
    }

    #[test]
    fn paint_truecolor_emits_escapes() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let opts = RenderOptions {
            gradient: Gradient::preset("ocean").unwrap(),
            direction: Direction::Horizontal,
            align: Align::Center,
            mode: ColorMode::True,
            target_width: 80,
            margin_y: 0,
        };
        let out = paint(&b, &opts);
        assert!(out.contains("\x1b[38;2;"));
        assert!(out.contains("\x1b[0m"));
    }
}
