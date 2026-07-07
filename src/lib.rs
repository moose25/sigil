//! sigil — modern gradient ASCII banners for your projects.
//!
//! The library is split into small pieces:
//! - [`color`]: sRGB/Oklab math and ANSI escape generation
//! - [`gradient`]: named presets and Oklab sampling
//! - [`fonts`]: bundled FIGlet fonts, embedded in the binary
//! - [`render`]: FIGlet layout and painting glyphs with a gradient

pub mod color;
pub mod fonts;
pub mod gradient;
pub mod render;

pub use color::{ColorMode, Rgb};
pub use gradient::{Direction, Gradient};
pub use render::{paint, Align, Banner, RenderOptions};
