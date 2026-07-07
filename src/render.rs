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

/// A box-drawing style for framing a banner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Border {
    pub tl: char,
    pub tr: char,
    pub bl: char,
    pub br: char,
    pub h: char,
    pub v: char,
}

impl Border {
    /// Parse a border style name. `none`/`off` returns `Ok(None)`.
    pub fn parse(s: &str) -> Result<Option<Border>, String> {
        let b = |tl, tr, bl, br, h, v| Border {
            tl,
            tr,
            bl,
            br,
            h,
            v,
        };
        Ok(Some(match s.to_ascii_lowercase().as_str() {
            "none" | "off" => return Ok(None),
            "round" | "rounded" => b('╭', '╮', '╰', '╯', '─', '│'),
            "single" | "line" => b('┌', '┐', '└', '┘', '─', '│'),
            "double" => b('╔', '╗', '╚', '╝', '═', '║'),
            "heavy" | "bold" => b('┏', '┓', '┗', '┛', '━', '┃'),
            "ascii" => b('+', '+', '+', '+', '-', '|'),
            other => {
                return Err(format!(
                    "unknown border: {other} (none|round|single|double|heavy|ascii)"
                ))
            }
        }))
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
    /// Flip the gradient's direction.
    pub reverse: bool,
    /// Repeat the palette this many times across the sweep.
    pub cycle: u32,
    /// Optional frame around the banner.
    pub border: Option<Border>,
    /// Interior padding (columns, rows) between the banner and the frame.
    pub padding: (usize, usize),
    /// Solid color for the frame; when `None`, the frame shares the gradient.
    pub border_color: Option<Rgb>,
}

/// Paint `banner` into a printable string with ANSI color escapes.
///
/// The banner (with any padding and frame) is composited into a character
/// grid, then every non-space cell is colored by its position in that grid —
/// so the gradient flows across the whole framed box, frame included.
pub fn paint(banner: &Banner, opts: &RenderOptions) -> String {
    let (px, py) = opts.padding;
    let edge = if opts.border.is_some() { 1 } else { 0 };
    let total_w = banner.width + 2 * px + 2 * edge;
    let total_h = banner.height() + 2 * py + 2 * edge;
    let (ox, oy) = (edge + px, edge + py);

    // Composite the banner glyphs (and frame) into a grid of chars.
    let mut grid = vec![vec![' '; total_w]; total_h];
    let mut is_frame = vec![vec![false; total_w]; total_h];
    for (r, line) in banner.lines.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            grid[oy + r][ox + c] = ch;
        }
    }
    if let Some(b) = opts.border {
        let (top, bot, left, right) = (0, total_h - 1, 0, total_w - 1);
        for col in 0..total_w {
            grid[top][col] = b.h;
            grid[bot][col] = b.h;
            is_frame[top][col] = true;
            is_frame[bot][col] = true;
        }
        for row in grid.iter_mut() {
            row[left] = b.v;
            row[right] = b.v;
        }
        for r in is_frame.iter_mut() {
            r[left] = true;
            r[right] = true;
        }
        grid[top][left] = b.tl;
        grid[top][right] = b.tr;
        grid[bot][left] = b.bl;
        grid[bot][right] = b.br;
    }

    let slack = opts.target_width.saturating_sub(total_w);
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
    for row in 0..total_h {
        out.push_str(&pad);
        let mut last: Option<Rgb> = None;
        for col in 0..total_w {
            let ch = grid[row][col];
            if ch == ' ' {
                out.push(' ');
                continue;
            }
            let color = match opts.border_color {
                Some(c) if is_frame[row][col] => c,
                _ => {
                    let t = opts.direction.t(row, col, total_h, total_w);
                    let t = crate::gradient::adjust_t(t, opts.reverse, opts.cycle);
                    opts.gradient.sample(t)
                }
            };
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

    fn base_opts(mode: ColorMode) -> RenderOptions {
        RenderOptions {
            gradient: Gradient::preset("ocean").unwrap(),
            direction: Direction::Horizontal,
            align: Align::Left,
            mode,
            target_width: 80,
            margin_y: 0,
            reverse: false,
            cycle: 1,
            border: None,
            padding: (0, 0),
            border_color: None,
        }
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
        let out = paint(&b, &base_opts(ColorMode::None));
        assert!(!out.contains('\x1b'));
    }

    #[test]
    fn paint_truecolor_emits_escapes() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let mut opts = base_opts(ColorMode::True);
        opts.align = Align::Center;
        let out = paint(&b, &opts);
        assert!(out.contains("\x1b[38;2;"));
        assert!(out.contains("\x1b[0m"));
    }

    #[test]
    fn border_frames_the_banner() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let mut opts = base_opts(ColorMode::None);
        opts.border = Border::parse("round").unwrap();
        opts.padding = (2, 1);
        let out = paint(&b, &opts);
        let lines: Vec<&str> = out.lines().collect();
        // Two border rows + two padding rows + banner height.
        assert_eq!(lines.len(), b.height() + 2 + 2);
        assert!(lines[0].starts_with('╭') && lines[0].ends_with('╮'));
        assert!(lines.last().unwrap().starts_with('╰'));
        // Every row is the same display width (banner + padding + border).
        let want = b.width + 2 * 2 + 2;
        assert!(lines.iter().all(|l| l.chars().count() == want));
    }
}
