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
            "sunset", "ocean", "fire", "mint", "grape", "cyberpunk", "gold", "ice",
            "vaporwave", "rainbow", "matrix", "flamingo", "mono",
        ]
    }
}

/// The axis along which the gradient sweeps across the banner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
    Diagonal,
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
        }
    }

    pub fn parse(s: &str) -> Result<Direction, String> {
        match s.to_ascii_lowercase().as_str() {
            "horizontal" | "h" => Ok(Direction::Horizontal),
            "vertical" | "v" => Ok(Direction::Vertical),
            "diagonal" | "d" => Ok(Direction::Diagonal),
            _ => Err(format!("unknown direction: {s} (horizontal|vertical|diagonal)")),
        }
    }
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
}
