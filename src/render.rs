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

/// How the gradient parameter is derived across the banner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorBy {
    /// One sweep across the whole banner (follows `--direction`/`--angle`).
    Banner,
    /// Each row runs the full gradient left→right (stacked lines each get it).
    Line,
    /// The palette cycles rapidly per column for a per-glyph, banded look.
    Char,
}

impl ColorBy {
    pub fn parse(s: &str) -> Result<ColorBy, String> {
        match s.to_ascii_lowercase().as_str() {
            "banner" | "all" => Ok(ColorBy::Banner),
            "line" | "row" => Ok(ColorBy::Line),
            "char" | "column" | "col" => Ok(ColorBy::Char),
            _ => Err(format!("unknown color-by: {s} (banner|line|char)")),
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

    /// Render several texts as separate banners stacked vertically (a blank row
    /// between each) into one banner, padded to a common display width. The
    /// result is a single unit: gradients, borders, and animation span it all.
    pub fn layout_multi(font: &FIGfont, texts: &[&str]) -> Result<Banner, String> {
        let mut lines: Vec<String> = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            if i > 0 {
                lines.push(String::new());
            }
            lines.extend(Banner::layout(font, text)?.lines);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
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
    /// Solid background fill behind the whole box; `None` leaves it bare.
    pub background: Option<Rgb>,
    /// How the gradient is mapped across the banner.
    pub color_by: ColorBy,
    /// Drop-shadow color behind the glyphs; `None` disables the shadow.
    pub shadow: Option<Rgb>,
}

/// The banner, padding, and any frame composited into a character grid.
///
/// `chars` holds the glyph at each cell (space where empty); `is_frame` marks
/// cells that belong to the border. Coloring works over grid coordinates, so
/// the gradient flows across the whole framed box.
pub struct Grid {
    pub chars: Vec<Vec<char>>,
    pub is_frame: Vec<Vec<bool>>,
    pub is_shadow: Vec<Vec<bool>>,
    pub width: usize,
    pub height: usize,
}

/// Drop-shadow offset (columns, rows) — down and to the right.
const SHADOW: (usize, usize) = (1, 1);

/// Composite a banner (with padding, optional frame, optional shadow) into a
/// [`Grid`].
pub fn compose(
    banner: &Banner,
    border: Option<Border>,
    padding: (usize, usize),
    shadow: bool,
) -> Grid {
    let (px, py) = padding;
    let edge = if border.is_some() { 1 } else { 0 };
    let (sdx, sdy) = if shadow { SHADOW } else { (0, 0) };
    let width = banner.width + sdx + 2 * px + 2 * edge;
    let height = banner.height() + sdy + 2 * py + 2 * edge;
    let (ox, oy) = (edge + px, edge + py);

    let mut chars = vec![vec![' '; width]; height];
    let mut is_frame = vec![vec![false; width]; height];
    let mut is_shadow = vec![vec![false; width]; height];

    // Place a banner's glyphs at (base_row, base_col); `shadow` marks the layer.
    let place = |chars: &mut Vec<Vec<char>>,
                 is_shadow: &mut Vec<Vec<bool>>,
                 bx: usize,
                 by: usize,
                 mark_shadow: bool| {
        for (r, line) in banner.lines.iter().enumerate() {
            let mut col = bx;
            for ch in line.chars() {
                let w = UnicodeWidthChar::width(ch).unwrap_or(0);
                if w == 0 || col >= width {
                    continue;
                }
                if ch != ' ' {
                    chars[by + r][col] = ch;
                    is_shadow[by + r][col] = mark_shadow;
                    if w == 2 && col + 1 < width {
                        chars[by + r][col + 1] = CONT;
                        is_shadow[by + r][col + 1] = mark_shadow;
                    }
                }
                col += w;
            }
        }
    };

    // Shadow first (offset), then the main glyphs overwrite where they overlap.
    if shadow {
        place(&mut chars, &mut is_shadow, ox + sdx, oy + sdy, true);
    }
    place(&mut chars, &mut is_shadow, ox, oy, false);

    if let Some(b) = border {
        let (top, bot, left, right) = (0, height - 1, 0, width - 1);
        for col in 0..width {
            chars[top][col] = b.h;
            chars[bot][col] = b.h;
            is_frame[top][col] = true;
            is_frame[bot][col] = true;
            is_shadow[top][col] = false;
            is_shadow[bot][col] = false;
        }
        for row in chars.iter_mut() {
            row[left] = b.v;
            row[right] = b.v;
        }
        for r in is_frame.iter_mut() {
            r[left] = true;
            r[right] = true;
        }
        for r in is_shadow.iter_mut() {
            r[left] = false;
            r[right] = false;
        }
        chars[top][left] = b.tl;
        chars[top][right] = b.tr;
        chars[bot][left] = b.bl;
        chars[bot][right] = b.br;
    }
    Grid {
        chars,
        is_frame,
        is_shadow,
        width,
        height,
    }
}

/// The color of the non-space cell at (row, col), honoring a solid frame color,
/// the gradient direction, reverse/cycle, and an animation `phase` shift.
pub fn cell_color(grid: &Grid, opts: &RenderOptions, row: usize, col: usize, phase: f32) -> Rgb {
    if let Some(sc) = opts.shadow {
        if grid.is_shadow[row][col] {
            return sc;
        }
    }
    if let Some(c) = opts.border_color {
        if grid.is_frame[row][col] {
            return c;
        }
    }
    let cols = grid.width;
    let col_frac = if cols <= 1 {
        0.0
    } else {
        col as f32 / (cols - 1) as f32
    };
    let base = match opts.color_by {
        ColorBy::Banner => opts.direction.t(row, col, grid.height, cols),
        ColorBy::Line => col_frac,
        // Cycle the palette every ~6 columns for a per-glyph banded look.
        ColorBy::Char => (col as f32 / 6.0).rem_euclid(1.0),
    };
    let t = crate::gradient::adjust_t(base, opts.reverse, opts.cycle);
    let t = (t - phase).rem_euclid(1.0);
    opts.gradient.sample(t)
}

/// Paint `banner` into a printable string with ANSI color escapes.
pub fn paint(banner: &Banner, opts: &RenderOptions) -> String {
    let grid = compose(banner, opts.border, opts.padding, opts.shadow.is_some());

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
    // Background is applied per row (indent stays outside the fill) and cleared
    // by the reset at each line's end.
    let bg = opts
        .background
        .filter(|_| opts.mode != ColorMode::None)
        .map(|c| opts.mode.bg(c));

    for row in 0..grid.height {
        out.push_str(&pad);
        if let Some(bg) = &bg {
            out.push_str(bg);
        }
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
    svg_impl(banner, opts, background, false)
}

/// Like [`to_svg`], but the gradient shimmers via SMIL animation (a browser /
/// GitHub renders it as a looping animated banner — no gif tooling needed).
pub fn to_svg_animated(banner: &Banner, opts: &RenderOptions, background: Option<Rgb>) -> String {
    svg_impl(banner, opts, background, true)
}

/// Number of gradient samples per animation cycle.
const SVG_ANIM_STEPS: usize = 24;

fn svg_impl(
    banner: &Banner,
    opts: &RenderOptions,
    background: Option<Rgb>,
    animate: bool,
) -> String {
    let grid = compose(banner, opts.border, opts.padding, opts.shadow.is_some());
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
        if animate {
            for col in 0..grid.width {
                let ch = grid.chars[row][col];
                if ch == CONT {
                    continue;
                }
                if ch == ' ' {
                    s.push(' ');
                    continue;
                }
                push_animated_span(&mut s, ch, &grid, opts, row, col);
            }
        } else {
            // Group consecutive cells sharing a fill; spaces extend the current run.
            let mut run = String::new();
            let mut fill: Option<Rgb> = None;
            for col in 0..grid.width {
                let ch = grid.chars[row][col];
                if ch == CONT {
                    continue;
                }
                let cell_fill = if ch == ' ' {
                    fill
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
        }
        s.push_str("</tspan>\n");
    }
    s.push_str("</text>\n</svg>\n");
    s
}

/// Emit an animated `<tspan>` whose fill cycles through the gradient sweep.
fn push_animated_span(
    out: &mut String,
    ch: char,
    grid: &Grid,
    opts: &RenderOptions,
    row: usize,
    col: usize,
) {
    // Sample the color at each phase; the last equals the first for a seamless loop.
    let values: Vec<String> = (0..=SVG_ANIM_STEPS)
        .map(|i| {
            hex(cell_color(
                grid,
                opts,
                row,
                col,
                i as f32 / SVG_ANIM_STEPS as f32,
            ))
        })
        .collect();
    out.push_str(&format!("<tspan fill=\"{}\">", values[0]));
    out.push_str(&format!(
        "<animate attributeName=\"fill\" dur=\"2s\" repeatCount=\"indefinite\" \
         calcMode=\"linear\" values=\"{}\"/>",
        values.join(";")
    ));
    out.push_str(&xml_escape(&ch.to_string()));
    out.push_str("</tspan>");
}

/// Render the banner as a standalone HTML document: a `<pre>` of colored
/// `<span>`s, for embedding in web pages, docs, or HTML email.
pub fn to_html(banner: &Banner, opts: &RenderOptions, background: Option<Rgb>) -> String {
    let grid = compose(banner, opts.border, opts.padding, opts.shadow.is_some());
    let bg = background.unwrap_or(Rgb::new(13, 17, 23));

    let mut s = String::new();
    s.push_str("<!DOCTYPE html>\n<meta charset=\"utf-8\">\n");
    s.push_str(&format!(
        "<pre style=\"font:bold 16px ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;\
         line-height:1.2;background:{};padding:16px;border-radius:8px;\
         display:inline-block;color:#fff\">",
        hex(bg)
    ));
    for row in 0..grid.height {
        let mut run = String::new();
        let mut fill: Option<Rgb> = None;
        for col in 0..grid.width {
            let ch = grid.chars[row][col];
            if ch == CONT {
                continue;
            }
            let cell_fill = if ch == ' ' {
                fill
            } else {
                Some(cell_color(&grid, opts, row, col, 0.0))
            };
            if ch != ' ' && cell_fill != fill && !run.is_empty() {
                push_html_span(&mut s, &run, fill);
                run.clear();
            }
            if ch != ' ' {
                fill = cell_fill;
            }
            run.push(ch);
        }
        push_html_span(&mut s, &run, fill);
        s.push('\n');
    }
    s.push_str("</pre>\n");
    s
}

/// Emit a `<span>` for a run of text with an optional color.
fn push_html_span(out: &mut String, text: &str, fill: Option<Rgb>) {
    if text.is_empty() {
        return;
    }
    match fill {
        Some(c) => out.push_str(&format!(
            "<span style=\"color:{}\">{}</span>",
            hex(c),
            xml_escape(text)
        )),
        None => out.push_str(&xml_escape(text)),
    }
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
            background: None,
            color_by: ColorBy::Banner,
            shadow: None,
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
    fn color_by_parses_and_bands() {
        assert_eq!(ColorBy::parse("line").unwrap(), ColorBy::Line);
        assert_eq!(ColorBy::parse("CHAR").unwrap(), ColorBy::Char);
        assert!(ColorBy::parse("nope").is_err());

        // Char mode repeats the palette, so a wide banner has fewer distinct
        // colors than a smooth banner sweep.
        let b = Banner::layout(&font(), "ABCDEFGH").unwrap();
        let distinct = |cb: ColorBy| {
            let mut o = base_opts(ColorMode::True);
            o.color_by = cb;
            let grid = compose(&b, None, (0, 0), false);
            let mut set = std::collections::HashSet::new();
            for r in 0..grid.height {
                for c in 0..grid.width {
                    if grid.chars[r][c] != ' ' && grid.chars[r][c] != CONT {
                        set.insert(cell_color(&grid, &o, r, c, 0.0));
                    }
                }
            }
            set.len()
        };
        assert!(distinct(ColorBy::Char) < distinct(ColorBy::Banner));
    }

    #[test]
    fn html_has_colored_spans() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let html = to_html(&b, &base_opts(ColorMode::True), None);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<pre"));
        assert!(html.contains("<span style=\"color:#"));
        assert!(html.trim_end().ends_with("</pre>"));
    }

    #[test]
    fn animated_svg_has_animate_elements() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let svg = to_svg_animated(&b, &base_opts(ColorMode::True), None);
        assert!(svg.contains("<animate "));
        assert!(svg.contains("repeatCount=\"indefinite\""));
        assert!(svg.trim_end().ends_with("</svg>"));
        // Static SVG has no animation.
        assert!(!to_svg(&b, &base_opts(ColorMode::True), None).contains("<animate"));
    }

    #[test]
    fn shadow_grows_grid_and_marks_cells() {
        let banner = Banner {
            lines: vec!["A".to_string()],
            width: 1,
        };
        let plain = compose(&banner, None, (0, 0), false);
        let shad = compose(&banner, None, (0, 0), true);
        // Shadow adds one column and one row.
        assert_eq!(shad.width, plain.width + 1);
        assert_eq!(shad.height, plain.height + 1);
        // Some cell is marked as shadow, and the main glyph is not.
        assert!(shad.is_shadow.iter().any(|row| row.iter().any(|&s| s)));
        assert!(!shad.is_shadow[0][0]); // top-left is the main glyph
    }

    #[test]
    fn wide_glyphs_align_by_display_width() {
        let banner = Banner {
            lines: vec!["日x".to_string(), "ab".to_string()],
            width: 3, // 日(2) + x(1)
        };
        let g = compose(&banner, None, (0, 0), false);
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
