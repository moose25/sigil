//! sigil - modern gradient ASCII banners for your projects.
//!
//! The library is split into small pieces:
//! - [`color`]: sRGB/Oklab math and ANSI escape generation
//! - [`gradient`]: named presets and Oklab sampling
//! - [`fonts`]: bundled FIGlet fonts, embedded in the binary
//! - [`render`]: FIGlet layout and painting glyphs with a gradient
//! - [`export`]: output formats (terminal, raw, and code snippets)
//! - [`animate`]: terminal reveal animations (sweep, typewriter)

pub mod animate;
pub mod color;
pub mod config;
pub mod export;
pub mod fonts;
pub mod gradient;
pub mod mark;
pub mod render;
pub mod text;
pub mod themes;

pub use color::{ColorMode, Rgb};
pub use gradient::{Direction, Gradient};
pub use render::{paint, Align, Banner, Border, RenderOptions};
