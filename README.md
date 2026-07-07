# sigil

**Give your CLI a face.** Modern gradient ASCII banners for your projects and command-line tools.

`sigil` turns text into a FIGlet banner and paints it with a smooth, perceptually-uniform gradient (interpolated in [Oklab](https://bottosson.github.io/posts/oklab/), so blends stay vivid instead of passing through muddy middles). Use it as a splash for your own CLI's `--help`, a header in your README, or just to make your terminal a little more fun.

> Status: early v0. A single static binary with no runtime dependencies.

## Quick start

```sh
cargo run -- "My Project" --gradient sunset
```

## Usage

```sh
sigil "Hello" --gradient ocean --direction diagonal --align center
sigil "Ship it" --colors "#ff5f6d,#ffc371"   # custom gradient stops
sigil "Deploy" --font ansishadow             # pick a font
echo "From stdin" | sigil                     # or pipe the text in
sigil "Launching" -g fire --animate sweep     # animated shimmer (TTY only)
sigil "Ready" --animate type --fps 60         # typewriter reveal
sigil "Angle" -g rainbow --angle 60 --cycle 2 # tilted, repeating palette
sigil "Boxed" -g ocean --border round         # frame it in a box
sigil gradients                               # preview all presets
sigil fonts                                   # preview all fonts
sigil "plain" --no-color                      # respects NO_COLOR too
```

### Options

| Flag | Description | Default |
| ---- | ----------- | ------- |
| `-g, --gradient <name>` | Named preset (see `sigil gradients`) | `ocean` |
| `-c, --colors <hex,...>` | Custom gradient stops, e.g. `#ff5f6d,#ffc371` | — |
| `-d, --direction <dir>` | `horizontal` \| `vertical` \| `diagonal` | `horizontal` |
| `--angle <deg>` | Sweep angle in degrees (overrides `--direction`) | — |
| `--reverse` | Flip the gradient direction | — |
| `--cycle <n>` | Repeat the palette N times across the banner | `1` |
| `-b, --border <style>` | `none` \| `round` \| `single` \| `double` \| `heavy` \| `ascii` | `none` |
| `-p, --padding <n>` | Interior padding inside the frame | `1` (with border) |
| `--border-color <hex>` | Solid frame color (default: share the gradient) | — |
| `-a, --align <align>` | `left` \| `center` \| `right` | `left` |
| `-f, --font <name>` | Font (see `sigil fonts`) | `standard` |
| `-w, --width <cols>` | Target width for alignment | terminal width |
| `-m, --margin <n>` | Blank lines above/below | `0` |
| `-F, --format <fmt>` | `term` \| `ansi` \| `raw` \| `rust` \| `go` \| `python` \| `shell` | `term` |
| `-o, --out <file>` | Write to a file instead of stdout | — |
| `--animate <style>` | `none` \| `sweep` \| `type` (terminal only) | `none` |
| `--fps <n>` | Animation speed, 1–120 | `30` |
| `--no-color` | Disable color | — |

### Gradients

`sunset`, `ocean`, `fire`, `mint`, `grape`, `cyberpunk`, `gold`, `ice`, `vaporwave`, `rainbow`, `matrix`, `flamingo`, `mono` — or roll your own with `--colors`.

### Fonts

`standard`, `ansishadow`, `slant`, `big`, `small` (with aliases like `shadow`, `italic`, `mini`). Run `sigil fonts` for a live preview. Bundled fonts are embedded in the binary — see [src/fonts/NOTICE.md](src/fonts/NOTICE.md) for attribution.

**Custom fonts:** pass a path to any FIGlet font — `sigil "Hi" -f ./cool.flf` — or drop `.flf` files in `~/.config/sigil/fonts/` and use them by name (`-f cool`). They show up in `sigil fonts` too. Code-tagged glyphs are trimmed automatically so most fonts "just work."

## Embed in your own tool

Generate a banner once and paste it into your project — a splash for `--help`, a startup logo, a script header. `--format` emits ready-to-use output:

```sh
sigil "Acme" -g sunset -F rust   > src/banner.rs   # pub const BANNER: &str = ...
sigil "Acme" -g sunset -F go     > banner.go        # const Banner = ...
sigil "Acme" -g sunset -F python > banner.py        # BANNER = ...
sigil "Acme" -g sunset -F shell  > banner.sh        # cat <<'…' heredoc that prints it
sigil "Acme" -g sunset -F ansi   > banner.ansi      # raw colored ANSI bytes
```

The `rust`/`go`/`python` snippets define a `BANNER` constant (with a comment showing how to print it); `shell` is a runnable heredoc. Color is baked into every snippet format. Use `-o <file>` instead of a shell redirect if you prefer.

## Config

Set defaults so you don't repeat flags. sigil reads two optional files and merges them, then command-line flags override everything:

**Precedence:** CLI flag > `.sigil.toml` (project, current dir) > `~/.config/sigil/config.toml` (user) > built-in default.

```toml
# ~/.config/sigil/config.toml  or  ./.sigil.toml
gradient = "vaporwave"
font = "ansishadow"
align = "center"
border = "round"
# any of: colors, direction, angle, reverse, cycle, padding,
# border_color, margin, width, animate, fps, format
```

Unknown keys are rejected so typos surface early.

## Color support

`sigil` emits 24-bit truecolor when `COLORTERM` advertises it, falls back to the 256-color palette otherwise, and prints plain glyphs under `NO_COLOR` or `--no-color`.

## Roadmap

Tracked in [issues](../../issues): unicode-width correctness, shell completions, packaging, and a demo gif.

## License

MIT
