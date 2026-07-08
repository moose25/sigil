# Changelog

All notable changes to sigil are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project aims to follow
[Semantic Versioning](https://semver.org/).

## [0.3.0] - 2026-07-07

A large feature release focused on **branding your project**: deriving looks
from a single color, richer typography, more output targets, and a couple of
signature capabilities.

### Color & gradients

- `--from <hex>` derives a full gradient from one brand color (Oklab tints/shades).
- Positioned gradient stops: `--colors "#000@0,#fff@0.8"`.
- `--gradient-file <file>` imports a palette (hex-per-line or GIMP `.gpl`).
- `--bg-gradient <spec>` fills the background with a gradient in **every**
  renderer (terminal, SVG, PNG, APNG, HTML).
- New directions: `--direction radial | conic`.
- Six new gradients (`coral`, `glacier`, `nebula`, `moss`, `peach`, `twilight`)
  and three new themes (`synthwave`, `arctic`, `sepia`).

### Typography & layout

- `--subtitle <text>` / `--subtitle-font` - a logo with a smaller tagline in one render.
- `--wrap <cols>` word-wraps long text to fit N columns.
- `--fit <cols>` auto-picks the boldest bundled font that fits.
- `--letter-spacing <n>` for an airier look.
- `--icon <glyph>` places an emoji/glyph beside the wordmark.
- `--fill shade` renders glyphs as block-shade `░▒▓█` by brightness, so a
  gradient reads even without color.

### Output & tooling

- Animated PNG (APNG) export: `-F png --animate sweep`.
- New snippet targets: `-F js | ts | c | cpp | ruby`, plus `-F markdown`.
- `--func` emits a `print_banner()` function instead of a bare constant.
- `sigil mark` - a deterministic generative geometric logo from a string.
- `sigil gallery` - a self-contained HTML page of your text in every style.
- `sigil random` - a surprise combo plus the exact flags to reproduce it.
- `sigil config path | show` - inspect config files and effective settings.
- `sigil themes` now previews each theme as a live mini-banner.
- `cargo binstall sigil` support via `[package.metadata.binstall]`.

### Messaging

- Reframed the project around "brand your project in seconds," with a
  simple→elaborate showcase gallery in the README.

## [0.2.0] - 2026-07-07

### Added

- Independent interior padding: `--pad-x` / `--pad-y` (override `--padding` per axis).
- Horizontal margin: `--margin-x` (left indent, complements the vertical `--margin`).
- Minimum width: `--min-width` pads the box out to at least N columns (centered).

## [0.1.0] - 2026-07-07

The initial release. Built up across seven development milestones.

### Rendering

- FIGlet banners painted with smooth, perceptually-uniform **Oklab** gradients.
- 21 gradient presets, custom `--colors` stops, and user-defined gradients in config.
- Interpolation color spaces: `--interpolate oklab | rgb | hsl`.
- Sweep direction / arbitrary `--angle`, `--reverse`, `--cycle`, and
  `--color-by banner | line | char`.
- 10 bundled fonts (embedded, trimmed) plus custom `.flf` files by path or from
  `~/.config/sigil/fonts`.
- Frames (`--border round|single|double|heavy|ascii`) with `--title`, interior
  `--padding`, `--border-color`, and `--background`.
- `--shadow` and `--outline` effects.
- Multi-line stacked banners (`--lines`), `--random` (with `--seed`), and
  colorizing arbitrary art (`--art`).
- Correct display-width layout for wide glyphs; graceful non-ASCII folding.

### Output formats (`-F`)

- `term`, `ansi`, `raw`, code snippets (`rust`, `go`, `python`, `shell`),
  `svg` (+ animated), `html`, `png` (with `--scale`), and `json`.
- `--out` to a file, `--copy` to the clipboard, TTY-aware color, `NO_COLOR`.

### Animations (terminal)

- `sweep`, `type`, `pulse`, and `scroll` with `--fps`.

### Themes & config

- 11 built-in themes and user `[themes.<name>]`.
- TOML config (user + project) with `sigil init`; precedence
  flag > theme > project > user > default.

### Tooling

- Subcommands: `gradients`, `fonts`, `themes`, `demo`, `init`, `completions`,
  `man`.
- Shell completions and a man page generated from the CLI.
- Reads text from stdin; multi-word arguments; SIGPIPE handled like a normal
  Unix filter.
- CI (fmt, clippy, test) and a tag-driven release workflow for prebuilt binaries.
