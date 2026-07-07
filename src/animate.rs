//! Terminal reveal animations for a rendered banner.
//!
//! Animations only run on a real TTY (the caller checks). Each frame is drawn
//! in place by moving the cursor back up over the banner and repainting, so no
//! alternate screen or raw mode is needed — and nothing to restore if the user
//! hits Ctrl-C.

use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

use crate::color::ColorMode;
use crate::gradient::Gradient;
use crate::render::Banner;

/// A reveal style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anim {
    /// No animation.
    None,
    /// A gradient shimmer that scrolls across the banner, then settles.
    Sweep,
    /// A left-to-right typewriter reveal, column by column.
    Type,
}

impl Anim {
    pub fn parse(s: &str) -> Result<Anim, String> {
        match s.to_ascii_lowercase().as_str() {
            "none" | "off" => Ok(Anim::None),
            "sweep" | "shimmer" => Ok(Anim::Sweep),
            "type" | "typewriter" => Ok(Anim::Type),
            _ => Err(format!("unknown animation: {s} (none|sweep|type)")),
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
    gradient: &Gradient,
    mode: ColorMode,
    style: Anim,
    fps: u32,
) -> io::Result<()> {
    let fps = fps.clamp(1, 120);
    let delay = Duration::from_secs_f32(1.0 / fps as f32);
    let height = banner.height();

    match style {
        Anim::None => {
            out.write_all(sweep_frame(banner, gradient, mode, 0.0).as_bytes())?;
        }
        Anim::Sweep => {
            // Scroll the gradient for ~2 seconds (one cycle per second), then
            // settle on the static banner.
            let frames = fps as usize * 2;
            for i in 0..frames {
                let phase = i as f32 / fps as f32;
                draw(
                    out,
                    &sweep_frame(banner, gradient, mode, phase),
                    height,
                    i == 0,
                )?;
                sleep(delay);
            }
            draw(
                out,
                &sweep_frame(banner, gradient, mode, 0.0),
                height,
                false,
            )?;
        }
        Anim::Type => {
            for reveal in 0..=banner.width {
                draw(
                    out,
                    &type_frame(banner, gradient, mode, reveal),
                    height,
                    reveal == 0,
                )?;
                sleep(delay);
            }
        }
    }
    Ok(())
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

/// Horizontal gradient position of a column, in `[0, 1]`.
fn column_t(col: usize, cols: usize) -> f32 {
    if cols <= 1 {
        0.0
    } else {
        col as f32 / (cols - 1) as f32
    }
}

/// A sweep frame: the horizontal gradient shifted by `phase` and wrapped.
fn sweep_frame(banner: &Banner, gradient: &Gradient, mode: ColorMode, phase: f32) -> String {
    let cols = banner.width;
    let mut out = String::new();
    for line in &banner.lines {
        let mut last = None;
        for (c, ch) in line.chars().enumerate() {
            if ch == ' ' {
                out.push(' ');
                continue;
            }
            let t = (column_t(c, cols) - phase).rem_euclid(1.0);
            let color = gradient.sample(t);
            if mode != ColorMode::None && last != Some(color) {
                out.push_str(&mode.fg(color));
                last = Some(color);
            }
            out.push(ch);
        }
        out.push_str(mode.reset());
        out.push('\n');
    }
    out
}

/// A typewriter frame: static gradient, revealing columns `0..reveal`.
fn type_frame(banner: &Banner, gradient: &Gradient, mode: ColorMode, reveal: usize) -> String {
    let cols = banner.width;
    let mut out = String::new();
    for line in &banner.lines {
        let mut last = None;
        for (c, ch) in line.chars().enumerate() {
            if c >= reveal || ch == ' ' {
                out.push(' ');
                continue;
            }
            let color = gradient.sample(column_t(c, cols));
            if mode != ColorMode::None && last != Some(color) {
                out.push_str(&mode.fg(color));
                last = Some(color);
            }
            out.push(ch);
        }
        out.push_str(mode.reset());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;

    fn banner() -> Banner {
        let font = figlet_rs::FIGfont::standard().unwrap();
        Banner::layout(&font, "Hi").unwrap()
    }

    fn grad() -> Gradient {
        Gradient::new(&[Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)])
    }

    #[test]
    fn parses_styles() {
        assert_eq!(Anim::parse("sweep").unwrap(), Anim::Sweep);
        assert_eq!(Anim::parse("Typewriter").unwrap(), Anim::Type);
        assert_eq!(Anim::parse("off").unwrap(), Anim::None);
        assert!(Anim::parse("wobble").is_err());
    }

    #[test]
    fn type_frame_reveals_progressively() {
        let b = banner();
        let none = type_frame(&b, &grad(), ColorMode::None, 0);
        let full = type_frame(&b, &grad(), ColorMode::None, b.width);
        // Nothing revealed yet: only spaces/newlines.
        assert!(none.chars().all(|c| c == ' ' || c == '\n'));
        // Fully revealed frame has ink.
        assert!(full.chars().any(|c| !c.is_whitespace()));
    }

    #[test]
    fn sweep_frame_has_correct_line_count() {
        let b = banner();
        let f = sweep_frame(&b, &grad(), ColorMode::True, 0.25);
        assert_eq!(f.lines().count(), b.height());
        assert!(f.contains("\x1b[38;2;"));
    }
}
