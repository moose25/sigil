//! Gradients: named presets sampled along a set of color stops, in a choice of
//! color space (Oklab by default, or RGB / HSL).

use crate::color::Rgb;

/// The color space a gradient blends in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Interp {
    /// Perceptually-uniform Oklab (default): vivid, even blends.
    #[default]
    Oklab,
    /// Straight sRGB component interpolation.
    Rgb,
    /// HSL — rotates hue for a rainbow-ish transition.
    Hsl,
}

impl Interp {
    pub fn parse(s: &str) -> Result<Interp, String> {
        match s.to_ascii_lowercase().as_str() {
            "oklab" | "lab" => Ok(Interp::Oklab),
            "rgb" | "srgb" => Ok(Interp::Rgb),
            "hsl" => Ok(Interp::Hsl),
            _ => Err(format!("unknown interpolation: {s} (oklab|rgb|hsl)")),
        }
    }
}

/// A multi-stop gradient over sRGB stops, blended in a chosen [`Interp`] space.
#[derive(Clone, Debug)]
pub struct Gradient {
    stops: Vec<Rgb>,
    interp: Interp,
}

impl Gradient {
    /// Build a gradient from one or more sRGB stops (Oklab blending by default).
    pub fn new(stops: &[Rgb]) -> Gradient {
        assert!(!stops.is_empty(), "gradient needs at least one stop");
        Gradient {
            stops: stops.to_vec(),
            interp: Interp::default(),
        }
    }

    /// Set the interpolation color space.
    pub fn with_interp(mut self, interp: Interp) -> Gradient {
        self.interp = interp;
        self
    }

    /// Sample the gradient at `t` in `[0, 1]`.
    pub fn sample(&self, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        if self.stops.len() == 1 {
            return self.stops[0];
        }
        let segments = self.stops.len() - 1;
        let scaled = t * segments as f32;
        let idx = (scaled.floor() as usize).min(segments - 1);
        let local = scaled - idx as f32;
        let (a, b) = (self.stops[idx], self.stops[idx + 1]);
        match self.interp {
            Interp::Oklab => a.to_oklab().lerp(b.to_oklab(), local).to_rgb(),
            Interp::Rgb => lerp_rgb(a, b, local),
            Interp::Hsl => lerp_hsl(a, b, local),
        }
    }

    /// Look up a named preset (case-insensitive), or `None`.
    pub fn preset(name: &str) -> Option<Gradient> {
        let stops: &[u32] = match name.to_ascii_lowercase().as_str() {
            "sunset" => &[0xff5f6d, 0xffc371],
            "ocean" => &[0x2193b0, 0x6dd5ed],
            "fire" => &[0xf12711, 0xf5af19],
            "mint" => &[0x00b09b, 0x96c93d],
            "grape" => &[0x8e2de2, 0x4a00e0],
            "cyberpunk" => &[0xf0f, 0x0ff],
            "gold" => &[0xf7971e, 0xffd200],
            "ice" => &[0x83a4d4, 0xb6fbff],
            "vaporwave" => &[0xff6ad5, 0x8a2be2, 0x26c6da],
            "rainbow" => &[0xff0000, 0xff8800, 0xffee00, 0x00cc44, 0x0088ff, 0x8800ff],
            "matrix" => &[0x003b00, 0x00ff41],
            "flamingo" => &[0xf093fb, 0xf5576c],
            "mono" => &[0xffffff, 0x888888],
            "aurora" => &[0x00c9ff, 0x92fe9d],
            "lava" => &[0xff512f, 0xdd2476],
            "neon" => &[0x39ff14, 0x00e5ff],
            "pastel" => &[0xa8edea, 0xfed6e3],
            "dusk" => &[0x2c3e50, 0xfd746c],
            "berry" => &[0x8a2387, 0xe94057, 0xf27121],
            "steel" => &[0xbdc3c7, 0x2c3e50],
            "forest" => &[0x134e5e, 0x71b280],
            _ => return None,
        };
        Some(Gradient::new(
            &stops
                .iter()
                .map(|&n| Rgb::new((n >> 16) as u8, (n >> 8) as u8, n as u8))
                .collect::<Vec<_>>(),
        ))
    }

    /// Names of all built-in presets, in display order.
    pub fn preset_names() -> &'static [&'static str] {
        &[
            "sunset",
            "ocean",
            "fire",
            "mint",
            "grape",
            "cyberpunk",
            "gold",
            "ice",
            "vaporwave",
            "rainbow",
            "matrix",
            "flamingo",
            "mono",
            "aurora",
            "lava",
            "neon",
            "pastel",
            "dusk",
            "berry",
            "steel",
            "forest",
        ]
    }
}

/// The axis along which the gradient sweeps across the banner.
///
/// `Angle` is measured in degrees: 0° = left→right, 90° = top→bottom,
/// 45° = diagonal — so the named variants are just convenient angles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
    Diagonal,
    Angle(f32),
}

impl Direction {
    /// Compute the gradient parameter `t` for a cell at (row, col) in a grid.
    pub fn t(self, row: usize, col: usize, rows: usize, cols: usize) -> f32 {
        let fx = frac(col, cols);
        let fy = frac(row, rows);
        match self {
            Direction::Horizontal => fx,
            Direction::Vertical => fy,
            Direction::Diagonal => (fx + fy) * 0.5,
            Direction::Angle(deg) => project(fx, fy, deg),
        }
    }

    pub fn parse(s: &str) -> Result<Direction, String> {
        match s.to_ascii_lowercase().as_str() {
            "horizontal" | "h" => Ok(Direction::Horizontal),
            "vertical" | "v" => Ok(Direction::Vertical),
            "diagonal" | "d" => Ok(Direction::Diagonal),
            _ => Err(format!(
                "unknown direction: {s} (horizontal|vertical|diagonal)"
            )),
        }
    }
}

/// Project a point in the unit square onto the direction at `deg` degrees,
/// normalized to `[0, 1]` across the square.
fn project(fx: f32, fy: f32, deg: f32) -> f32 {
    let (s, c) = deg.to_radians().sin_cos();
    let raw = fx * c + fy * s;
    let min = c.min(0.0) + s.min(0.0);
    let max = c.max(0.0) + s.max(0.0);
    let span = max - min;
    if span.abs() < 1e-6 {
        0.0
    } else {
        (raw - min) / span
    }
}

/// Apply `reverse` and `cycle` to a base parameter `t` in `[0, 1]`.
///
/// `cycle` repeats the palette that many times across the sweep; `reverse`
/// flips its direction.
pub fn adjust_t(t: f32, reverse: bool, cycle: u32) -> f32 {
    let t = if reverse { 1.0 - t } else { t };
    let cycle = cycle.max(1) as f32;
    (t * cycle).rem_euclid(1.0)
}

/// Position within a span of `n` cells, in `[0, 1]`; 0.0 when there is one cell.
fn frac(i: usize, n: usize) -> f32 {
    if n <= 1 {
        0.0
    } else {
        i as f32 / (n - 1) as f32
    }
}

fn lerp8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// Straight sRGB component interpolation.
fn lerp_rgb(a: Rgb, b: Rgb, t: f32) -> Rgb {
    Rgb::new(lerp8(a.r, b.r, t), lerp8(a.g, b.g, t), lerp8(a.b, b.b, t))
}

/// Interpolate in HSL, taking the shortest path around the hue circle.
fn lerp_hsl(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let (h1, s1, l1) = rgb_to_hsl(a);
    let (h2, s2, l2) = rgb_to_hsl(b);
    let mut dh = h2 - h1;
    if dh > 180.0 {
        dh -= 360.0;
    } else if dh < -180.0 {
        dh += 360.0;
    }
    let h = (h1 + dh * t).rem_euclid(360.0);
    let s = s1 + (s2 - s1) * t;
    let l = l1 + (l2 - l1) * t;
    hsl_to_rgb(h, s, l)
}

fn rgb_to_hsl(c: Rgb) -> (f32, f32, f32) {
    let (r, g, b) = (c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h.rem_euclid(360.0), s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to8 = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(to8(r1), to8(g1), to8(b1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_match_stops() {
        let g = Gradient::new(&[Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)]);
        assert_eq!(g.sample(0.0), Rgb::new(255, 0, 0));
        assert_eq!(g.sample(1.0), Rgb::new(0, 0, 255));
    }

    #[test]
    fn interp_modes_differ_but_share_endpoints() {
        let stops = [Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)];
        let modes = [Interp::Oklab, Interp::Rgb, Interp::Hsl];
        for m in modes {
            let g = Gradient::new(&stops).with_interp(m);
            assert_eq!(g.sample(0.0), stops[0]);
            assert_eq!(g.sample(1.0), stops[1]);
        }
        // RGB midpoint is the plain average; Oklab differs.
        let rgb = Gradient::new(&stops).with_interp(Interp::Rgb);
        assert_eq!(rgb.sample(0.5), Rgb::new(128, 0, 128));
        let oklab = Gradient::new(&stops).with_interp(Interp::Oklab);
        assert_ne!(oklab.sample(0.5), rgb.sample(0.5));
        assert_eq!(Interp::parse("hsl").unwrap(), Interp::Hsl);
        assert!(Interp::parse("nope").is_err());
    }

    #[test]
    fn all_presets_resolve() {
        for name in Gradient::preset_names() {
            assert!(Gradient::preset(name).is_some(), "missing preset {name}");
        }
        assert!(Gradient::preset("nonsense").is_none());
    }

    #[test]
    fn direction_spans_full_range() {
        assert_eq!(Direction::Horizontal.t(0, 0, 3, 5), 0.0);
        assert_eq!(Direction::Horizontal.t(0, 4, 3, 5), 1.0);
        assert_eq!(Direction::Vertical.t(2, 0, 3, 5), 1.0);
    }

    #[test]
    fn angle_matches_named_directions() {
        // 0° behaves like horizontal, 90° like vertical.
        for (r, c) in [(0, 0), (1, 2), (2, 4)] {
            let h = Direction::Horizontal.t(r, c, 3, 5);
            let a0 = Direction::Angle(0.0).t(r, c, 3, 5);
            assert!((h - a0).abs() < 1e-5, "{h} vs {a0}");
            let v = Direction::Vertical.t(r, c, 3, 5);
            let a90 = Direction::Angle(90.0).t(r, c, 3, 5);
            assert!((v - a90).abs() < 1e-5, "{v} vs {a90}");
        }
    }

    #[test]
    fn adjust_reverse_and_cycle() {
        assert_eq!(adjust_t(0.25, false, 1), 0.25);
        assert_eq!(adjust_t(0.25, true, 1), 0.75);
        // cycle=2 doubles the parameter and wraps.
        assert!((adjust_t(0.3, false, 2) - 0.6).abs() < 1e-6);
        assert!((adjust_t(0.6, false, 2) - 0.2).abs() < 1e-6);
        // cycle=0 is treated as 1.
        assert_eq!(adjust_t(0.4, false, 0), 0.4);
    }
}
