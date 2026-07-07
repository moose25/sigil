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

    /// Render `text` with `spacing` extra blank columns between each glyph, for
    /// an airier, letter-spaced look.
    ///
    /// Each character is laid out on its own and the rows are concatenated, so
    /// the usual FIGlet kerning/smushing between neighbours is dropped in favour
    /// of even, explicit spacing. `spacing == 0` still works (just no gap added).
    pub fn layout_spaced(font: &FIGfont, text: &str, spacing: usize) -> Result<Banner, String> {
        let text = crate::text::sanitize(text);
        let gap = " ".repeat(spacing);
        let mut rows: Vec<String> = Vec::new();
        for (i, ch) in text.chars().enumerate() {
            let figure = font
                .convert(&ch.to_string())
                .ok_or_else(|| format!("could not render {ch:?} with this font"))?;
            let glyph: Vec<String> = figure.to_string().lines().map(str::to_string).collect();
            // FIGlet glyphs share a fixed height, so every char has the same row
            // count; seed `rows` from the first one.
            if rows.is_empty() {
                rows = vec![String::new(); glyph.len()];
            }
            for (r, line) in glyph.iter().enumerate() {
                if r >= rows.len() {
                    break;
                }
                if i > 0 {
                    rows[r].push_str(&gap);
                }
                rows[r].push_str(line);
            }
        }
        // Trim shared blank rows top and bottom, then pad to a common width.
        while rows.first().is_some_and(|l| l.trim().is_empty()) {
            rows.remove(0);
        }
        while rows.last().is_some_and(|l| l.trim().is_empty()) {
            rows.pop();
        }
        if rows.is_empty() {
            rows.push(String::new());
        }
        let width = rows.iter().map(|l| display_width(l)).max().unwrap_or(0);
        for l in &mut rows {
            let pad = width - display_width(l);
            if pad > 0 {
                l.push_str(&" ".repeat(pad));
            }
        }
        Ok(Banner { lines: rows, width })
    }

    /// Build a banner from existing multi-line art (not a FIGlet font): keep the
    /// characters as-is and pad to a common display width.
    pub fn from_art(content: &str) -> Banner {
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
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
        Banner { lines, width }
    }

    /// Stack `bottom` beneath `top` into one banner, each centered within the
    /// wider of the two, separated by `gap` blank rows.
    ///
    /// The two can come from different fonts — this is how a large headline and
    /// a smaller subtitle become a single unit that shares one gradient, border,
    /// and animation.
    pub fn stacked(top: &Banner, bottom: &Banner, gap: usize) -> Banner {
        let width = top.width.max(bottom.width);
        let centered = |b: &Banner| -> Vec<String> {
            b.lines
                .iter()
                .map(|l| {
                    let pad = width - display_width(l);
                    let left = pad / 2;
                    format!("{}{}{}", " ".repeat(left), l, " ".repeat(pad - left))
                })
                .collect()
        };
        let mut lines = centered(top);
        for _ in 0..gap {
            lines.push(" ".repeat(width));
        }
        lines.extend(centered(bottom));
        Banner { lines, width }
    }

    /// Prepend a small icon (an emoji or Nerd Font glyph) to the left of the
    /// banner, vertically centered, with `gap` blank columns between it and the
    /// text. The glyph renders at the terminal's normal cell size — an icon
    /// beside a wordmark. A zero-width icon is ignored.
    pub fn with_icon(mut self, icon: &str, gap: usize) -> Banner {
        let iw = display_width(icon);
        if iw == 0 {
            return self;
        }
        let mid = self.lines.len() / 2;
        let blank = " ".repeat(iw + gap);
        for (i, line) in self.lines.iter_mut().enumerate() {
            if i == mid {
                *line = format!("{icon}{}{line}", " ".repeat(gap));
            } else {
                *line = format!("{blank}{line}");
            }
        }
        self.width += iw + gap;
        self
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
    /// Extra left indent (columns) added on top of alignment.
    pub margin_x: usize,
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
    /// Gradient background fill behind the whole box (overrides `background`
    /// where a cell is covered); `None` leaves the solid/base background.
    pub background_gradient: Option<Gradient>,
    /// How the gradient is mapped across the banner.
    pub color_by: ColorBy,
    /// Drop-shadow color behind the glyphs; `None` disables the shadow.
    pub shadow: Option<Rgb>,
    /// Outline (halo) color around the glyphs; `None` disables the outline.
    pub outline: Option<Rgb>,
    /// Caption embedded in the top border (only shown when a border is set).
    pub title: Option<String>,
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
    pub is_outline: Vec<Vec<bool>>,
    pub width: usize,
    pub height: usize,
}

/// Drop-shadow offset (columns, rows) — down and to the right.
const SHADOW: (usize, usize) = (1, 1);

/// Composite a banner (with padding, frame, shadow, outline, title) into a
/// [`Grid`], ready for coloring.
pub fn compose(banner: &Banner, opts: &RenderOptions) -> Grid {
    let (px, py) = opts.padding;
    let edge = if opts.border.is_some() { 1 } else { 0 };
    let (sdx, sdy) = if opts.shadow.is_some() {
        SHADOW
    } else {
        (0, 0)
    };
    let om = usize::from(opts.outline.is_some()); // 1-cell halo margin on every side
    let width = banner.width + 2 * om + sdx + 2 * px + 2 * edge;
    let height = banner.height() + 2 * om + sdy + 2 * py + 2 * edge;
    let (ox, oy) = (edge + px + om, edge + py + om);

    let mut chars = vec![vec![' '; width]; height];
    let mut is_frame = vec![vec![false; width]; height];
    let mut is_shadow = vec![vec![false; width]; height];
    let mut is_outline = vec![vec![false; width]; height];

    // Stamp the banner's glyphs at base (by, bx) into `flag`. `only_empty`
    // leaves already-occupied cells alone (used for the outline halo).
    let stamp = |chars: &mut Vec<Vec<char>>,
                 flag: &mut Vec<Vec<bool>>,
                 by: usize,
                 bx: usize,
                 only_empty: bool| {
        for (r, line) in banner.lines.iter().enumerate() {
            let mut col = bx;
            for ch in line.chars() {
                let w = UnicodeWidthChar::width(ch).unwrap_or(0);
                if w == 0 {
                    continue;
                }
                if col >= width {
                    break;
                }
                if ch != ' ' && (!only_empty || chars[by + r][col] == ' ') {
                    chars[by + r][col] = ch;
                    flag[by + r][col] = true;
                    if w == 2 && col + 1 < width {
                        chars[by + r][col + 1] = CONT;
                        flag[by + r][col + 1] = true;
                    }
                }
                col += w;
            }
        }
    };

    // Layers, back to front: shadow (offset), outline (8-way halo), main glyphs.
    if opts.shadow.is_some() {
        stamp(&mut chars, &mut is_shadow, oy + sdy, ox + sdx, false);
    }
    if opts.outline.is_some() {
        for dr in -1_isize..=1 {
            for dc in -1_isize..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let by = (oy as isize + dr) as usize;
                let bx = (ox as isize + dc) as usize;
                stamp(&mut chars, &mut is_outline, by, bx, true);
            }
        }
    }
    // Main glyphs on top, clearing any shadow/outline flags they cover.
    for (r, line) in banner.lines.iter().enumerate() {
        let mut col = ox;
        for ch in line.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if w == 0 {
                continue;
            }
            if col >= width {
                break;
            }
            if ch != ' ' {
                for c in col..(col + w).min(width) {
                    chars[oy + r][c] = if c == col { ch } else { CONT };
                    is_shadow[oy + r][c] = false;
                    is_outline[oy + r][c] = false;
                }
            }
            col += w;
        }
    }

    if let Some(b) = opts.border {
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
        // Frame cells are never shadow/outline.
        for (fr, (sr, or)) in is_frame
            .iter()
            .zip(is_shadow.iter_mut().zip(is_outline.iter_mut()))
        {
            for (c, &f) in fr.iter().enumerate() {
                if f {
                    sr[c] = false;
                    or[c] = false;
                }
            }
        }
        chars[top][left] = b.tl;
        chars[top][right] = b.tr;
        chars[bot][left] = b.bl;
        chars[bot][right] = b.br;

        // Embed a title into the top border: `╭─ title ─────╮`.
        if let Some(t) = opts.title.as_deref().filter(|t| !t.is_empty()) {
            let deco: Vec<char> = format!(" {t} ").chars().collect();
            let start = 2; // after the corner and one horizontal rule
            let room = width.saturating_sub(start + 2); // keep a rule + corner on the right
            for (i, ch) in deco.into_iter().take(room).enumerate() {
                chars[top][start + i] = ch;
            }
        }
    }
    Grid {
        chars,
        is_frame,
        is_shadow,
        is_outline,
        width,
        height,
    }
}

/// The color of the non-space cell at (row, col), honoring a solid frame color,
/// the gradient direction, reverse/cycle, and an animation `phase` shift.
pub fn cell_color(grid: &Grid, opts: &RenderOptions, row: usize, col: usize, phase: f32) -> Rgb {
    if let Some(oc) = opts.outline {
        if grid.is_outline[row][col] {
            return oc;
        }
    }
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

/// Background color for a cell when `background_gradient` is set — sampled along
/// the sweep direction across the whole box, honoring reverse/cycle. Returns
/// `None` when there is no background gradient (callers fall back to the solid
/// background or leave the cell bare).
pub fn bg_cell_color(grid: &Grid, opts: &RenderOptions, row: usize, col: usize) -> Option<Rgb> {
    let g = opts.background_gradient.as_ref()?;
    let base = opts
        .direction
        .t(row, col, grid.height.max(1), grid.width.max(1));
    let t = crate::gradient::adjust_t(base, opts.reverse, opts.cycle);
    Some(g.sample(t))
}

/// Paint `banner` into a printable string with ANSI color escapes.
pub fn paint(banner: &Banner, opts: &RenderOptions) -> String {
    let grid = compose(banner, opts);

    let slack = opts.target_width.saturating_sub(grid.width);
    let indent = match opts.align {
        Align::Left => 0,
        Align::Center => slack / 2,
        Align::Right => slack,
    };
    let pad = " ".repeat(indent + opts.margin_x);

    let mut out = String::new();
    for _ in 0..opts.margin_y {
        out.push('\n');
    }
    // Background: a solid fill is set once per row; a gradient fill is set
    // per cell as it changes. Both are cleared by the reset at each line's end.
    let color_on = opts.mode != ColorMode::None;
    let has_bg_grad = color_on && opts.background_gradient.is_some();
    let solid_bg = opts
        .background
        .filter(|_| color_on && !has_bg_grad)
        .map(|c| opts.mode.bg(c));

    for row in 0..grid.height {
        out.push_str(&pad);
        if let Some(bg) = &solid_bg {
            out.push_str(bg);
        }
        let mut last_fg: Option<Rgb> = None;
        let mut last_bg: Option<Rgb> = None;
        for col in 0..grid.width {
            let ch = grid.chars[row][col];
            if ch == CONT {
                continue; // second column of a wide glyph; already drawn
            }
            if has_bg_grad {
                let bgc = bg_cell_color(&grid, opts, row, col);
                if last_bg != bgc {
                    if let Some(c) = bgc {
                        out.push_str(&opts.mode.bg(c));
                    }
                    last_bg = bgc;
                }
            }
            if ch == ' ' {
                out.push(' ');
                continue;
            }
            let color = cell_color(&grid, opts, row, col, 0.0);
            if color_on && last_fg != Some(color) {
                out.push_str(&opts.mode.fg(color));
                last_fg = Some(color);
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
    let grid = compose(banner, opts);
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
    // A gradient background is drawn as a grid of per-cell rects over the base.
    if opts.background_gradient.is_some() {
        for row in 0..grid.height {
            for col in 0..grid.width {
                if let Some(c) = bg_cell_color(&grid, opts, row, col) {
                    s.push_str(&format!(
                        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>\n",
                        col as f32 * cell_w,
                        row as f32 * line_h,
                        cell_w + 0.5,
                        line_h + 0.5,
                        hex(c)
                    ));
                }
            }
        }
    }
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

/// Render the banner to PNG bytes, one color block per character cell (a crisp
/// pixel-style image that needs no font). Honors gradient, borders, shadow,
/// outline, and background.
pub fn to_png(
    banner: &Banner,
    opts: &RenderOptions,
    background: Option<Rgb>,
    scale: usize,
) -> Result<Vec<u8>, String> {
    // Cell size in pixels, roughly a terminal cell's 1:2 aspect, times `scale`.
    let scale = scale.clamp(1, 10);
    let cw = 14 * scale;
    let ch = 28 * scale;
    let grid = compose(banner, opts);
    let w = grid.width * cw;
    let h = grid.height * ch;
    if w == 0 || h == 0 {
        return Err("nothing to render".into());
    }
    let bg = background.unwrap_or(Rgb::new(13, 17, 23));

    // RGB pixel buffer, initialized to the background.
    let mut px = Vec::with_capacity(w * h * 3);
    for _ in 0..(w * h) {
        px.extend_from_slice(&[bg.r, bg.g, bg.b]);
    }
    let mut fill = |x0: usize, y0: usize, c: Rgb| {
        for y in y0..y0 + ch {
            let base = (y * w + x0) * 3;
            for x in 0..cw {
                let i = base + x * 3;
                px[i] = c.r;
                px[i + 1] = c.g;
                px[i + 2] = c.b;
            }
        }
    };
    // Gradient background: paint each cell's backdrop before the glyphs.
    if opts.background_gradient.is_some() {
        for row in 0..grid.height {
            for col in 0..grid.width {
                if let Some(c) = bg_cell_color(&grid, opts, row, col) {
                    fill(col * cw, row * ch, c);
                }
            }
        }
    }
    for row in 0..grid.height {
        for col in 0..grid.width {
            if grid.chars[row][col] == ' ' {
                continue; // leave the background showing
            }
            let c = cell_color(&grid, opts, row, col, 0.0);
            fill(col * cw, row * ch, c);
        }
    }

    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| format!("png error: {e}"))?;
        writer
            .write_image_data(&px)
            .map_err(|e| format!("png error: {e}"))?;
    }
    Ok(out)
}

/// Render the banner as an animated PNG (APNG) whose gradient sweeps, looping
/// forever at `fps`. `frames` frames span one full pass of the palette.
pub fn to_apng(
    banner: &Banner,
    opts: &RenderOptions,
    background: Option<Rgb>,
    scale: usize,
    frames: usize,
    fps: u32,
) -> Result<Vec<u8>, String> {
    let scale = scale.clamp(1, 10);
    let (cw, ch) = (14 * scale, 28 * scale);
    let grid = compose(banner, opts);
    let (w, h) = (grid.width * cw, grid.height * ch);
    if w == 0 || h == 0 {
        return Err("nothing to render".into());
    }
    let bg = background.unwrap_or(Rgb::new(13, 17, 23));
    let frames = frames.clamp(2, 120) as u32;
    let fps = fps.clamp(1, 120) as u16;

    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_animated(frames, 0)
            .map_err(|e| format!("apng error: {e}"))?; // 0 plays = loop forever
        enc.set_frame_delay(1, fps)
            .map_err(|e| format!("apng error: {e}"))?; // 1/fps seconds per frame
        let mut writer = enc.write_header().map_err(|e| format!("png error: {e}"))?;
        for f in 0..frames {
            let phase = f as f32 / frames as f32;
            let mut px = Vec::with_capacity(w * h * 3);
            for _ in 0..(w * h) {
                px.extend_from_slice(&[bg.r, bg.g, bg.b]);
            }
            let mut put = |x0: usize, y0: usize, c: Rgb| {
                for y in y0..y0 + ch {
                    let base = (y * w + x0) * 3;
                    for x in 0..cw {
                        let i = base + x * 3;
                        px[i] = c.r;
                        px[i + 1] = c.g;
                        px[i + 2] = c.b;
                    }
                }
            };
            // Static gradient backdrop behind the shimmering glyphs.
            if opts.background_gradient.is_some() {
                for row in 0..grid.height {
                    for col in 0..grid.width {
                        if let Some(c) = bg_cell_color(&grid, opts, row, col) {
                            put(col * cw, row * ch, c);
                        }
                    }
                }
            }
            for row in 0..grid.height {
                for col in 0..grid.width {
                    if grid.chars[row][col] == ' ' {
                        continue;
                    }
                    let c = cell_color(&grid, opts, row, col, phase);
                    put(col * cw, row * ch, c);
                }
            }
            writer
                .write_image_data(&px)
                .map_err(|e| format!("apng error: {e}"))?;
        }
    }
    Ok(out)
}

/// Render the banner as JSON: dimensions plus per-cell char and hex color
/// (null for spaces), for programmatic consumers.
pub fn to_json(banner: &Banner, opts: &RenderOptions) -> String {
    let grid = compose(banner, opts);
    let mut rows = Vec::with_capacity(grid.height);
    for row in 0..grid.height {
        let mut cells = Vec::new();
        for col in 0..grid.width {
            let ch = grid.chars[row][col];
            if ch == CONT {
                continue;
            }
            let color = if ch == ' ' {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(hex(cell_color(&grid, opts, row, col, 0.0)))
            };
            cells.push(serde_json::json!({ "char": ch.to_string(), "color": color }));
        }
        rows.push(serde_json::Value::Array(cells));
    }
    let v = serde_json::json!({
        "width": grid.width,
        "height": grid.height,
        "cells": rows,
    });
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".to_string())
}

/// Render the banner as a standalone HTML document: a `<pre>` of colored
/// `<span>`s, for embedding in web pages, docs, or HTML email.
pub fn to_html(banner: &Banner, opts: &RenderOptions, background: Option<Rgb>) -> String {
    let grid = compose(banner, opts);
    let bg = background.unwrap_or(Rgb::new(13, 17, 23));

    let mut s = String::new();
    s.push_str("<!DOCTYPE html>\n<meta charset=\"utf-8\">\n");
    s.push_str(&format!(
        "<pre style=\"font:bold 16px ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;\
         line-height:1.2;background:{};padding:16px;border-radius:8px;\
         display:inline-block;color:#fff\">",
        hex(bg)
    ));
    let has_bg_grad = opts.background_gradient.is_some();
    for row in 0..grid.height {
        if has_bg_grad {
            // Per-cell spans so each cell can carry its own background color.
            for col in 0..grid.width {
                let ch = grid.chars[row][col];
                if ch == CONT {
                    continue;
                }
                let mut style = String::new();
                if let Some(b) = bg_cell_color(&grid, opts, row, col) {
                    style.push_str(&format!("background:{};", hex(b)));
                }
                if ch != ' ' {
                    style.push_str(&format!(
                        "color:{};",
                        hex(cell_color(&grid, opts, row, col, 0.0))
                    ));
                }
                s.push_str(&format!(
                    "<span style=\"{style}\">{}</span>",
                    xml_escape(&ch.to_string())
                ));
            }
            s.push('\n');
            continue;
        }
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
            margin_x: 0,
            reverse: false,
            cycle: 1,
            border: None,
            padding: (0, 0),
            border_color: None,
            background: None,
            background_gradient: None,
            color_by: ColorBy::Banner,
            shadow: None,
            outline: None,
            title: None,
        }
    }

    #[test]
    fn stacked_combines_and_centers() {
        let f = font();
        let top = Banner::layout(&f, "Headline").unwrap();
        let bottom = Banner::layout(&f, "sub").unwrap();
        let s = Banner::stacked(&top, &bottom, 1);
        assert_eq!(s.width, top.width.max(bottom.width));
        assert_eq!(s.height(), top.height() + 1 + bottom.height());
        assert!(s.lines.iter().all(|l| display_width(l) == s.width));
    }

    #[test]
    fn letter_spacing_widens_banner() {
        let f = font();
        let tight = Banner::layout(&f, "AB").unwrap();
        let spaced = Banner::layout_spaced(&f, "AB", 4).unwrap();
        assert!(spaced.width > tight.width);
        assert_eq!(spaced.height(), tight.height());
        assert!(spaced
            .lines
            .iter()
            .all(|l| display_width(l) == spaced.width));
    }

    #[test]
    fn icon_prefixes_and_widens() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let w0 = b.width;
        let iced = b.with_icon("*", 2);
        assert_eq!(iced.width, w0 + 1 + 2); // icon width 1 + gap 2
                                            // The middle row carries the icon; every row grew to the new width.
        let mid = &iced.lines[iced.lines.len() / 2];
        assert!(mid.starts_with('*'));
        assert!(iced.lines.iter().all(|l| display_width(l) == iced.width));
    }

    #[test]
    fn background_gradient_adds_svg_cells() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let plain = to_svg(&b, &base_opts(ColorMode::True), None);
        let mut opts = base_opts(ColorMode::True);
        opts.background_gradient = Some(Gradient::preset("dusk").unwrap());
        let washed = to_svg(&b, &opts, None);
        // The bg gradient adds a per-cell rect grid, so there are more rects.
        assert!(washed.matches("<rect").count() > plain.matches("<rect").count());
        // And bg_cell_color yields a color where a gradient is set (none otherwise).
        let grid = compose(&b, &opts);
        assert!(bg_cell_color(&grid, &opts, 0, 0).is_some());
        assert!(bg_cell_color(&grid, &base_opts(ColorMode::True), 0, 0).is_none());
    }

    #[test]
    fn apng_has_animation_chunks() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let bytes = to_apng(&b, &base_opts(ColorMode::True), None, 1, 6, 30).unwrap();
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
        );
        // acTL marks an animated PNG; one fcTL per frame.
        let count = |needle: &[u8]| bytes.windows(4).filter(|w| *w == needle).count();
        assert!(count(b"acTL") >= 1, "missing acTL (not animated)");
        assert_eq!(count(b"fcTL"), 6, "expected one fcTL per frame");
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
            let grid = compose(&b, &o);
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
    fn json_has_dimensions_and_cells() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let json = to_json(&b, &base_opts(ColorMode::True));
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["width"].as_u64().unwrap() > 0);
        assert!(v["height"].as_u64().unwrap() > 0);
        assert!(v["cells"].as_array().unwrap().len() == v["height"].as_u64().unwrap() as usize);
    }

    #[test]
    fn png_has_valid_signature() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let bytes = to_png(&b, &base_opts(ColorMode::True), None, 1).unwrap();
        // PNG magic number.
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
        );
        assert!(bytes.len() > 100);
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
    fn title_embeds_in_top_border() {
        let b = Banner::layout(&font(), "Hi").unwrap();
        let mut opts = base_opts(ColorMode::None);
        opts.border = Border::parse("round").unwrap();
        opts.padding = (2, 1);
        opts.title = Some("hey".to_string());
        let top = paint(&b, &opts).lines().next().unwrap().to_string();
        assert!(top.contains("hey"));
        assert!(top.starts_with('╭') && top.ends_with('╮'));
    }

    #[test]
    fn shadow_grows_grid_and_marks_cells() {
        let banner = Banner {
            lines: vec!["A".to_string()],
            width: 1,
        };
        let plain = compose(&banner, &base_opts(ColorMode::None));
        let mut o = base_opts(ColorMode::None);
        o.shadow = Some(Rgb::new(20, 20, 20));
        let shad = compose(&banner, &o);
        // Shadow adds one column and one row.
        assert_eq!(shad.width, plain.width + 1);
        assert_eq!(shad.height, plain.height + 1);
        // Some cell is marked as shadow, and the main glyph is not.
        assert!(shad.is_shadow.iter().any(|row| row.iter().any(|&s| s)));
        assert!(!shad.is_shadow[0][0]); // top-left is the main glyph
    }

    #[test]
    fn outline_haloes_the_glyphs() {
        let banner = Banner {
            lines: vec!["A".to_string()],
            width: 1,
        };
        let plain = compose(&banner, &base_opts(ColorMode::None));
        let mut o = base_opts(ColorMode::None);
        o.outline = Some(Rgb::new(200, 200, 200));
        let out = compose(&banner, &o);
        // Outline adds a 1-cell halo on every side.
        assert_eq!(out.width, plain.width + 2);
        assert_eq!(out.height, plain.height + 2);
        assert!(out.is_outline.iter().any(|row| row.iter().any(|&s| s)));
    }

    #[test]
    fn wide_glyphs_align_by_display_width() {
        let banner = Banner {
            lines: vec!["日x".to_string(), "ab".to_string()],
            width: 3, // 日(2) + x(1)
        };
        let g = compose(&banner, &base_opts(ColorMode::None));
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
    fn from_art_pads_and_preserves() {
        let art = " /\\_/\\\n( o.o )\n"; // ragged lines
        let b = Banner::from_art(art);
        assert_eq!(b.height(), 2);
        assert!(b.lines.iter().all(|l| display_width(l) == b.width));
        assert!(b.lines[0].contains('/')); // art kept as-is
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
