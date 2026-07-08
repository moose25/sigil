//! Generative "sigil" marks - a deterministic geometric emblem derived from a
//! string. The same input always yields the same mirror-symmetric mark, painted
//! with a gradient; a quick, unique logo when you don't have one.

use crate::color::Rgb;
use crate::gradient::Gradient;

/// A tiny SplitMix64 PRNG (same algorithm the CLI uses for `--random`), so a
/// mark is fully determined by its seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// FNV-1a hash of the (trimmed) input - seeds the mark deterministically.
fn seed_of(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.trim().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Grid is `N`×`N` cells, mirrored left↔right for a symmetric, glyph-like mark.
const N: usize = 5;

/// Render a deterministic mark for `text` as a standalone SVG, painted with
/// `gradient` on a `bg` backdrop (default dark).
pub fn to_svg(text: &str, gradient: &Gradient, bg: Option<Rgb>) -> String {
    const CELL: usize = 48; // px per cell
    const PAD: usize = 40; // outer padding

    let mut rng = Rng(seed_of(text));
    // Fill the left half (including the center column) and mirror it.
    let half = N / 2 + 1;
    let mut on = [[false; N]; N];
    for row in on.iter_mut() {
        for col in 0..half {
            let bit = rng.next() & 1 == 1;
            row[col] = bit;
            row[N - 1 - col] = bit;
        }
    }

    let size = N * CELL + 2 * PAD;
    let bg = bg.unwrap_or(Rgb::new(13, 17, 23));
    let mut s = String::with_capacity(512);
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" \
         viewBox=\"0 0 {size} {size}\">\n"
    ));
    s.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" rx=\"{}\" fill=\"#{:02x}{:02x}{:02x}\"/>\n",
        PAD / 2,
        bg.r,
        bg.g,
        bg.b
    ));
    for (row, cells) in on.iter().enumerate() {
        for (col, &lit) in cells.iter().enumerate() {
            if !lit {
                continue;
            }
            // Color along the diagonal so the gradient reads across the mark.
            let t = (row + col) as f32 / (2 * (N - 1)) as f32;
            let c = gradient.sample(t);
            let (x, y) = (PAD + col * CELL, PAD + row * CELL);
            s.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{CELL}\" height=\"{CELL}\" rx=\"6\" \
                 fill=\"#{:02x}{:02x}{:02x}\"/>\n",
                c.r, c.g, c.b
            ));
        }
    }
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic_and_input_sensitive() {
        let g = Gradient::preset("aurora").unwrap();
        let a1 = to_svg("acme", &g, None);
        let a2 = to_svg("acme", &g, None);
        let b = to_svg("acme2", &g, None);
        assert_eq!(a1, a2, "same input must yield the same mark");
        assert_ne!(a1, b, "different input should yield a different mark");
    }

    #[test]
    fn is_well_formed_svg() {
        let g = Gradient::preset("sunset").unwrap();
        let svg = to_svg("sigil", &g, None);
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("<rect"));
    }
}
