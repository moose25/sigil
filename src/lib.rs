//! sigil — modern gradient ASCII banners for your projects.
//!
//! The library is split into small pieces:
//! - [`color`]: sRGB/Oklab math and ANSI escape generation
//! - [`gradient`]: named presets and Oklab sampling
//! - [`fonts`]: bundled FIGlet fonts, embedded in the binary
//! - [`render`]: FIGlet layout and painting glyphs with a gradient
//! - [`export`]: output formats (terminal, raw, and code snippets)

pub mod color;
pub mod export;
pub mod fonts;
pub mod gradient;
pub mod render;

pub use color::{ColorMode, Rgb};
pub use gradient::{Direction, Gradient};
pub use render::{paint, Align, Banner, RenderOptions};
