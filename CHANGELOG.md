# Changelog

All notable changes to sigil are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project aims to follow
[Semantic Versioning](https://semver.org/).

## [Unreleased] — 0.1.0

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
