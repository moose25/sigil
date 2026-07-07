//! Terminal reveal animations for a rendered banner.
//!
//! Animations only run on a real TTY (the caller checks). Each frame is drawn
//! in place by moving the cursor back up over the banner and repainting, so no
//! alternate screen or raw mode is needed — and nothing to restore if the user
//! hits Ctrl-C.
//!
//! Frames reuse [`crate::render`]'s grid and per-cell coloring, so borders,
//! gradient direction, and reverse/cycle all animate for free.

use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

use crate::render::{cell_color, compose, Banner, Grid, RenderOptions};

/// A reveal style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anim {
    /// No animation.
    None,
    /// A gradient shimmer that scrolls across the banner, then settles.
    Sweep,
    /// A left-to-right typewriter reveal, column by column.
    Type,
    /// A brightness pulse — the banner breathes between dim and full.
    Pulse,
    /// A marquee that scrolls the banner horizontally.
    Scroll,
}

impl Anim {
    pub fn parse(s: &str) -> Result<Anim, String> {
        match s.to_ascii_lowercase().as_str() {
            "none" | "off" => Ok(Anim::None),
            "sweep" | "shimmer" => Ok(Anim::Sweep),
            "type" | "typewriter" => Ok(Anim::Type),
            "pulse" | "breathe" => Ok(Anim::Pulse),
            "scroll" | "marquee" => Ok(Anim::Scroll),
            _ => Err(format!(
                "unknown animation: {s} (none|sweep|type|pulse|scroll)"
            )),
        }
    }

    pub fn is_animated(self) -> bool {
        self != Anim::None
    }
}

/// Play the animation to `out`, leaving the finished banner on screen.
pub fn play(
    out: &mut impl Write,
    banner: &Banner,
    opts: &RenderOptions,
    style: Anim,
    fps: u32,
) -> io::Result<()> {
    let fps = fps.clamp(1, 120);
    let delay = Duration::from_secs_f32(1.0 / fps as f32);
    let grid = compose(banner, opts);
    let height = grid.height;

    match style {
        Anim::None => {
            out.write_all(frame(&grid, opts, 0.0, None, 1.0).as_bytes())?;
        }
        Anim::Sweep => {
            // Scroll the gradient for ~2 seconds (one cycle per second), then
            // settle on the static banner.
            let frames = fps as usize * 2;
            for i in 0..frames {
                let phase = i as f32 / fps as f32;
                draw(out, &frame(&grid, opts, phase, None, 1.0), height, i == 0)?;
                sleep(delay);
            }
            draw(out, &frame(&grid, opts, 0.0, None, 1.0), height, false)?;
        }
        Anim::Type => {
            for reveal in 0..=grid.width {
                draw(
                    out,
                    &frame(&grid, opts, 0.0, Some(reveal), 1.0),
                    height,
                    reveal == 0,
                )?;
                sleep(delay);
            }
        }
        Anim::Pulse => {
            // Breathe brightness over ~3 seconds (period 1.5s), then settle full.
            let frames = fps as usize * 3;
            for i in 0..frames {
                let t = i as f32 / fps as f32;
                let phase = (std::f32::consts::TAU * t / 1.5).cos();
                let brightness = 0.35 + 0.65 * (0.5 - 0.5 * phase); // 0.35..=1.0
                draw(
                    out,
                    &frame(&grid, opts, 0.0, None, brightness),
                    height,
                    i == 0,
                )?;
                sleep(delay);
            }
            draw(out, &frame(&grid, opts, 0.0, None, 1.0), height, false)?;
        }
        Anim::Scroll => {
            // Marquee across the terminal width; loop twice, then settle at the start.
            let vw = if opts.target_width > 0 {
                opts.target_width
            } else {
                grid.width
            };
            const GAP: usize = 4;
            let period = grid.width + GAP;
            for i in 0..(period * 2) {
                draw(
                    out,
                    &scroll_frame(&grid, opts, i % period, vw),
                    height,
                    i == 0,
                )?;
                sleep(delay);
            }
            draw(out, &scroll_frame(&grid, opts, 0, vw), height, false)?;
        }
    }
    Ok(())
}

/// A marquee frame: a `vw`-wide viewport onto the grid, shifted left by
/// `offset` and wrapping with a gap.
fn scroll_frame(grid: &Grid, opts: &RenderOptions, offset: usize, vw: usize) -> String {
    const GAP: usize = 4;
    let period = grid.width + GAP;
    let bg = opts
        .background
        .filter(|_| opts.mode != crate::color::ColorMode::None)
        .map(|c| opts.mode.bg(c));
    let mut out = String::new();
    for row in 0..grid.height {
        if let Some(bg) = &bg {
            out.push_str(bg);
        }
        let mut last = None;
        for vc in 0..vw {
            let src = (vc + offset) % period;
            let ch = if src < grid.width {
                grid.chars[row][src]
            } else {
                ' ' // the gap between wraps
            };
            if ch == crate::render::CONT || ch == ' ' {
                out.push(' ');
                continue;
            }
            let color = cell_color(grid, opts, row, src, 0.0);
            if opts.mode != crate::color::ColorMode::None && last != Some(color) {
                out.push_str(&opts.mode.fg(color));
                last = Some(color);
            }
            out.push(ch);
        }
        out.push_str(opts.mode.reset());
        out.push('\n');
    }
    out
}

/// Scale an RGB color's brightness by `f` in `[0, 1]`.
fn dim(c: crate::color::Rgb, f: f32) -> crate::color::Rgb {
    let s = |v: u8| (v as f32 * f).round().clamp(0.0, 255.0) as u8;
    crate::color::Rgb::new(s(c.r), s(c.g), s(c.b))
}

/// Render one frame of the grid: `phase` shifts the gradient (sweep), `reveal`
/// hides columns at/after it (typewriter), and `brightness` scales all colors
/// (pulse).
fn frame(
    grid: &Grid,
    opts: &RenderOptions,
    phase: f32,
    reveal: Option<usize>,
    brightness: f32,
) -> String {
    let mut out = String::new();
    let bg = opts
        .background
        .filter(|_| opts.mode != crate::color::ColorMode::None)
        .map(|c| opts.mode.bg(c));
    for row in 0..grid.height {
        if let Some(bg) = &bg {
            out.push_str(bg);
        }
        let mut last = None;
        for col in 0..grid.width {
            let hidden = reveal.is_some_and(|r| col >= r);
            if hidden {
                out.push(' '); // blank column, preserves width
                continue;
            }
            let ch = grid.chars[row][col];
            if ch == crate::render::CONT {
                continue; // second column of a wide glyph; already drawn
            }
            if ch == ' ' {
                out.push(' ');
                continue;
            }
            let mut color = cell_color(grid, opts, row, col, phase);
            if brightness < 1.0 {
                color = dim(color, brightness);
            }
            if opts.mode != crate::color::ColorMode::None && last != Some(color) {
                out.push_str(&opts.mode.fg(color));
                last = Some(color);
            }
            out.push(ch);
        }
        out.push_str(opts.mode.reset());
        out.push('\n');
    }
    out
}

/// Write one frame, repositioning over the previous one when not the first.
fn draw(out: &mut impl Write, content: &str, height: usize, first: bool) -> io::Result<()> {
    if !first {
        write!(out, "\x1b[{height}A")?; // cursor up `height` lines
    }
    for line in content.lines() {
        write!(out, "\r\x1b[2K{line}\n")?; // carriage return, clear line, draw
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{ColorMode, Rgb};
    use crate::gradient::{Direction, Gradient};
    use crate::render::{Align, Border, ColorBy};

    fn banner() -> Banner {
        let font = figlet_rs::FIGfont::standard().unwrap();
        Banner::layout(&font, "Hi").unwrap()
    }

    fn opts(mode: ColorMode, border: Option<Border>) -> RenderOptions {
        RenderOptions {
            gradient: Gradient::new(&[Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)]),
            direction: Direction::Horizontal,
            align: Align::Left,
            mode,
            target_width: 0,
            margin_y: 0,
            margin_x: 0,
            reverse: false,
            cycle: 1,
            border,
            padding: (0, 0),
            border_color: None,
            background: None,
            background_gradient: None,
            shade: false,
            color_by: ColorBy::Banner,
            shadow: None,
            outline: None,
            title: None,
        }
    }

    #[test]
    fn parses_styles() {
        assert_eq!(Anim::parse("sweep").unwrap(), Anim::Sweep);
        assert_eq!(Anim::parse("Typewriter").unwrap(), Anim::Type);
        assert_eq!(Anim::parse("breathe").unwrap(), Anim::Pulse);
        assert_eq!(Anim::parse("off").unwrap(), Anim::None);
        assert!(Anim::parse("wobble").is_err());
    }

    #[test]
    fn dim_scales_brightness() {
        assert_eq!(dim(Rgb::new(200, 100, 50), 0.5), Rgb::new(100, 50, 25));
        assert_eq!(dim(Rgb::new(200, 100, 50), 1.0), Rgb::new(200, 100, 50));
    }

    #[test]
    fn type_reveal_is_progressive() {
        let b = banner();
        let o = opts(ColorMode::None, None);
        let grid = compose(&b, &o);
        let none = frame(&grid, &o, 0.0, Some(0), 1.0);
        let full = frame(&grid, &o, 0.0, Some(grid.width), 1.0);
        assert!(none.chars().all(|c| c == ' ' || c == '\n'));
        assert!(full.chars().any(|c| !c.is_whitespace()));
    }

    #[test]
    fn frame_line_count_and_color() {
        let b = banner();
        let o = opts(ColorMode::True, None);
        let grid = compose(&b, &o);
        let f = frame(&grid, &o, 0.25, None, 1.0);
        assert_eq!(f.lines().count(), grid.height);
        assert!(f.contains("\x1b[38;2;"));
    }

    #[test]
    fn border_shows_during_animation() {
        let b = banner();
        let o = opts(ColorMode::None, Border::parse("round").unwrap());
        let grid = compose(&b, &o);
        // A mid-reveal frame should already include the top-left corner.
        let f = frame(&grid, &o, 0.0, Some(1), 1.0);
        assert!(f.contains('╭'));
    }
}
