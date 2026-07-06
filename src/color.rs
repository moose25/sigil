//! Color primitives: sRGB <-> Oklab conversion and ANSI escape generation.
//!
//! Gradients are interpolated in Oklab because linear blends between two sRGB
//! colors pass through muddy, desaturated middles. Oklab is perceptually
//! uniform, so a red->blue sweep stays vivid the whole way across.

/// An 8-bit-per-channel sRGB color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    /// Parse `#rrggbb`, `rrggbb`, `#rgb`, or `rgb`.
    pub fn parse(s: &str) -> Result<Rgb, String> {
        let h = s.strip_prefix('#').unwrap_or(s);
        let expand = |c: char| {
            let d = c.to_digit(16).unwrap();
            (d * 16 + d) as u8
        };
        match h.len() {
            6 => {
                let n =
                    u32::from_str_radix(h, 16).map_err(|_| format!("invalid hex color: {s}"))?;
                Ok(Rgb::new((n >> 16) as u8, (n >> 8) as u8, n as u8))
            }
            3 => {
                let mut ch = h.chars();
                let mut next = || {
                    ch.next()
                        .filter(|c| c.is_ascii_hexdigit())
                        .ok_or_else(|| format!("invalid hex color: {s}"))
                };
                Ok(Rgb::new(expand(next()?), expand(next()?), expand(next()?)))
            }
            _ => Err(format!("invalid hex color: {s} (expected #rgb or #rrggbb)")),
        }
    }

    // Coefficients are the canonical Oklab matrices, kept at their published
    // precision for provenance; f32 rounds them to the nearest representable value.
    #[allow(clippy::excessive_precision)]
    pub fn to_oklab(self) -> Oklab {
        let r = srgb_to_linear(self.r);
        let g = srgb_to_linear(self.g);
        let b = srgb_to_linear(self.b);

        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();

        Oklab {
            l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
        }
    }
}

/// A color in the Oklab perceptual color space.
#[derive(Clone, Copy, Debug)]
pub struct Oklab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl Oklab {
    /// Linear interpolation between two Oklab colors.
    pub fn lerp(self, other: Oklab, t: f32) -> Oklab {
        Oklab {
            l: self.l + (other.l - self.l) * t,
            a: self.a + (other.a - self.a) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }

    #[allow(clippy::excessive_precision)]
    pub fn to_rgb(self) -> Rgb {
        let l_ = self.l + 0.3963377774 * self.a + 0.2158037573 * self.b;
        let m_ = self.l - 0.1055613458 * self.a - 0.0638541728 * self.b;
        let s_ = self.l - 0.0894841775 * self.a - 1.2914855480 * self.b;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
        let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
        let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

        Rgb::new(linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
    }
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let v = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0).round() as u8
}

/// How much color a terminal can render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    /// 24-bit truecolor.
    True,
    /// 256-color palette (approximated).
    Ansi256,
    /// No color at all — plain glyphs.
    None,
}

impl ColorMode {
    /// Detect the best supported mode from the environment.
    ///
    /// Respects `NO_COLOR` (https://no-color.org) and `COLORTERM`.
    pub fn detect() -> ColorMode {
        if std::env::var_os("NO_COLOR").is_some() {
            return ColorMode::None;
        }
        match std::env::var("COLORTERM") {
            Ok(v) if v.contains("truecolor") || v.contains("24bit") => ColorMode::True,
            _ => ColorMode::Ansi256,
        }
    }

    /// ANSI SGR sequence that sets the foreground to `c` for this mode.
    pub fn fg(self, c: Rgb) -> String {
        match self {
            ColorMode::True => format!("\x1b[38;2;{};{};{}m", c.r, c.g, c.b),
            ColorMode::Ansi256 => format!("\x1b[38;5;{}m", rgb_to_ansi256(c)),
            ColorMode::None => String::new(),
        }
    }

    pub fn reset(self) -> &'static str {
        match self {
            ColorMode::None => "",
            _ => "\x1b[0m",
        }
    }
}

/// Map a truecolor RGB to the nearest xterm-256 palette index.
fn rgb_to_ansi256(c: Rgb) -> u8 {
    // Grayscale ramp gives smoother results for near-gray colors.
    if c.r == c.g && c.g == c.b {
        if c.r < 8 {
            return 16;
        }
        if c.r > 248 {
            return 231;
        }
        return 232 + ((c.r as u16 - 8) * 24 / 247) as u8;
    }
    let q = |v: u8| -> u16 {
        // 6x6x6 cube steps at 0,95,135,175,215,255.
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            ((v as u16 - 35) / 40).min(5)
        }
    };
    (16 + 36 * q(c.r) + 6 * q(c.g) + q(c.b)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing() {
        assert_eq!(Rgb::parse("#ff8800").unwrap(), Rgb::new(255, 136, 0));
        assert_eq!(Rgb::parse("ff8800").unwrap(), Rgb::new(255, 136, 0));
        assert_eq!(Rgb::parse("#f80").unwrap(), Rgb::new(255, 136, 0));
        assert_eq!(Rgb::parse("abc").unwrap(), Rgb::new(170, 187, 204));
        assert!(Rgb::parse("nope").is_err());
        assert!(Rgb::parse("#12").is_err());
    }

    #[test]
    fn oklab_roundtrip_is_stable() {
        for c in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 128, 255),
            Rgb::new(17, 34, 51),
            Rgb::new(255, 255, 255),
            Rgb::new(0, 0, 0),
        ] {
            let back = c.to_oklab().to_rgb();
            // Allow +/-1 per channel for float rounding.
            assert!((back.r as i16 - c.r as i16).abs() <= 1, "{c:?} -> {back:?}");
            assert!((back.g as i16 - c.g as i16).abs() <= 1, "{c:?} -> {back:?}");
            assert!((back.b as i16 - c.b as i16).abs() <= 1, "{c:?} -> {back:?}");
        }
    }
}
