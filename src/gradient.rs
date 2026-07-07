//! Gradients: named presets plus Oklab sampling along a set of color stops.

use crate::color::{Oklab, Rgb};

/// A multi-stop gradient, sampled in Oklab space.
#[derive(Clone, Debug)]
pub struct Gradient {
    stops: Vec<Oklab>,
}

impl Gradient {
    /// Build a gradient from two or more sRGB stops.
    pub fn new(stops: &[Rgb]) -> Gradient {
        assert!(!stops.is_empty(), "gradient needs at least one stop");
        Gradient {
            stops: stops.iter().map(|c| c.to_oklab()).collect(),
        }
    }

    /// Sample the gradient at `t` in `[0, 1]`.
    pub fn sample(&self, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        if self.stops.len() == 1 {
            return self.stops[0].to_rgb();
        }
        let segments = self.stops.len() - 1;
        let scaled = t * segments as f32;
        let idx = (scaled.floor() as usize).min(segments - 1);
        let local = scaled - idx as f32;
        self.stops[idx].lerp(self.stops[idx + 1], local).to_rgb()
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
