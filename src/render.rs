//! Turning text into a colored ASCII banner.

use figlet_rs::FIGfont;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::color::{ColorMode, Rgb};
use crate::gradient::{Direction, Gradient};

/// Display width of a string in terminal columns (wide glyphs count as 2).
fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Sentinel marking the second column occupied by a preceding wide glyph.
/// It renders as nothing (the wide glyph already covers both columns).
pub(crate) const CONT: char = '\0';

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
    ///
    /// Input is folded to renderable ASCII first (see [`crate::text::sanitize`])
    /// so non-ASCII text degrades gracefully instead of failing.
    pub fn layout(font: &FIGfont, text: &str) -> Result<Banner, String> {
        let text = crate::text::sanitize(text);
        let figure = font
            .convert(&text)
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

        // Pad to a common *display* width so lines with wide glyphs still align.
        let width = lines.iter().map(|l| display_width(l)).max().unwrap_or(0);
        for l in &mut lines {
            let pad = width - display_width(l);
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

/// The banner, padding, and any frame composited into a character grid.
///
/// `chars` holds the glyph at each cell (space where empty); `is_frame` marks
/// cells that belong to the border. Coloring works over grid coordinates, so
/// the gradient flows across the whole framed box.
pub struct Grid {
    pub chars: Vec<Vec<char>>,
    pub is_frame: Vec<Vec<bool>>,
    pub width: usize,
    pub height: usize,
}

/// Composite a banner (with padding and optional frame) into a [`Grid`].
pub fn compose(banner: &Banner, border: Option<Border>, padding: (usize, usize)) -> Grid {
    let (px, py) = padding;
    let edge = if border.is_some() { 1 } else { 0 };
    let width = banner.width + 2 * px + 2 * edge;
    let height = banner.height() + 2 * py + 2 * edge;
    let (ox, oy) = (edge + px, edge + py);

    let mut chars = vec![vec![' '; width]; height];
    let mut is_frame = vec![vec![false; width]; height];
    for (r, line) in banner.lines.iter().enumerate() {
        // Advance by each glyph's display width so wide glyphs occupy two
        // columns: the glyph itself and a continuation sentinel.
        let mut col = ox;
        for ch in line.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if w == 0 || col >= width {
                continue;
            }
            chars[oy + r][col] = ch;
            if w == 2 && col + 1 < width {
                chars[oy + r][col + 1] = CONT;
            }
            col += w;
        }
    }
    if let Some(b) = border {
        let (top, bot, left, right) = (0, height - 1, 0, width - 1);
        for col in 0..width {
            chars[top][col] = b.h;
            chars[bot][col] = b.h;
            is_frame[top][col] = true;
            is_frame[bot][col] = true;
        }
        for row in chars.iter_mut() {
            row[left] = b.v;
            row[right] = b.v;
        }
        for r in is_frame.iter_mut() {
            r[left] = true;
            r[right] = true;
        }
        chars[top][left] = b.tl;
        chars[top][right] = b.tr;
        chars[bot][left] = b.bl;
        chars[bot][right] = b.br;
    }
    Grid {
        chars,
        is_frame,
        width,
        height,
    }
}

/// The color of the non-space cell at (row, col), honoring a solid frame color,
/// the gradient direction, reverse/cycle, and an animation `phase` shift.
pub fn cell_color(grid: &Grid, opts: &RenderOptions, row: usize, col: usize, phase: f32) -> Rgb {
    if let Some(c) = opts.border_color {
        if grid.is_frame[row][col] {
            return c;
        }
    }
    let base = opts.direction.t(row, col, grid.height, grid.width);
    let t = crate::gradient::adjust_t(base, opts.reverse, opts.cycle);
    let t = (t - phase).rem_euclid(1.0);
    opts.gradient.sample(t)
}

/// Paint `banner` into a printable string with ANSI color escapes.
pub fn paint(banner: &Banner, opts: &RenderOptions) -> String {
    let grid = compose(banner, opts.border, opts.padding);

    let slack = opts.target_width.saturating_sub(grid.width);
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
    for row in 0..grid.height {
        out.push_str(&pad);
        let mut last: Option<Rgb> = None;
        for col in 0..grid.width {
            let ch = grid.chars[row][col];
            if ch == CONT {
                continue; // second column of a wide glyph; already drawn
            }
            if ch == ' ' {
                out.push(' ');
                continue;
            }
            let color = cell_color(&grid, opts, row, col, 0.0);
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

/// Render the banner as a standalone SVG (monospace grid with colored spans),
/// suitable for embedding in a README or docs. `background` fills the canvas;
/// when `None` a dark backdrop is used so light gradients stay readable.
pub fn to_svg(banner: &Banner, opts: &RenderOptions, background: Option<Rgb>) -> String {
    let grid = compose(banner, opts.border, opts.padding);
    let font_size = 24.0_f32;
    let cell_w = font_size * 0.6; // monospace advance width
    let line_h = font_size * 1.2;
    let w = grid.width as f32 * cell_w;
    let h = grid.height as f32 * line_h;
    let bg = background.unwrap_or(Rgb::new(13, 17, 23));

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\">\n"
    ));
    s.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" rx=\"8\" fill=\"{}\"/>\n",
        hex(bg)
    ));
    s.push_str(&format!(
        "<text xml:space=\"preserve\" font-family=\"ui-monospace,SFMono-Regular,Menlo,Consolas,monospace\" \
         font-size=\"{font_size:.0}\" font-weight=\"bold\">\n"
    ));

    for row in 0..grid.height {
        let y = (row as f32 + 0.8) * line_h;
        s.push_str(&format!("<tspan x=\"0\" y=\"{y:.1}\">"));
        // Group consecutive cells sharing a fill; spaces extend the current run.
        let mut run = String::new();
        let mut fill: Option<Rgb> = None;
        for col in 0..grid.width {
            let ch = grid.chars[row][col];
            if ch == CONT {
                continue;
            }
            let cell_fill = if ch == ' ' {
                fill // keep current color under spaces (invisible anyway)
            } else {
                Some(cell_color(&grid, opts, row, col, 0.0))
            };
            if ch != ' ' && cell_fill != fill && !run.is_empty() {
                push_span(&mut s, &run, fill);
                run.clear();
            }
            if ch != ' ' {
                fill = cell_fill;
            }
            run.push(ch);
        }
        push_span(&mut s, &run, fill);
        s.push_str("</tspan>\n");
    }
    s.push_str("</text>\n</svg>\n");
    s
}

/// Emit a `<tspan>` for a run of text with an optional fill color.
fn push_span(out: &mut String, text: &str, fill: Option<Rgb>) {
    if text.is_empty() {
        return;
    }
    match fill {
        Some(c) => out.push_str(&format!(
            "<tspan fill=\"{}\">{}</tspan>",
            hex(c),
            xml_escape(text)
        )),
        None => out.push_str(&xml_escape(text)),
    }
}

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
    fn svg_is_well_formed() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let svg = to_svg(&b, &base_opts(ColorMode::True), None);
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("fill=\"#"));
        assert!(svg.contains("<rect"));
        // Opening and closing tspans balance.
        assert_eq!(
            svg.matches("<tspan").count(),
            svg.matches("</tspan>").count()
        );
    }

    #[test]
    fn xml_special_chars_escaped() {
        assert_eq!(xml_escape("a<b>&c"), "a&lt;b&gt;&amp;c");
    }

    #[test]
    fn wide_glyphs_align_by_display_width() {
        let banner = Banner {
            lines: vec!["日x".to_string(), "ab".to_string()],
            width: 3, // 日(2) + x(1)
        };
        let g = compose(&banner, None, (0, 0));
        assert_eq!(g.width, 3);
        assert_eq!(g.chars[0][0], '日');
        assert_eq!(g.chars[0][1], CONT);
        assert_eq!(g.chars[0][2], 'x');

        let out = paint(&banner, &base_opts(ColorMode::None));
        let first = out.lines().next().unwrap();
        assert_eq!(display_width(first), 3);
        assert!(first.contains('日') && first.contains('x'));
        assert!(!first.contains(CONT));
    }

    #[test]
    fn layout_pads_to_common_width() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        assert!(b.height() > 1);
        assert!(b.lines.iter().all(|l| display_width(l) == b.width));
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
