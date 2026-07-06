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
sigil gradients                               # preview all presets
sigil "plain" --no-color                      # respects NO_COLOR too
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-g, --gradient <name>` | Named preset (see `sigil gradients`) | `ocean` |
| `-c, --colors <hex,...>` | Custom gradient stops, e.g. `#ff5f6d,#ffc371` | — |
| `-d, --direction <dir>` | `horizontal` \| `vertical` \| `diagonal` | `horizontal` |
| `-a, --align <align>` | `left` \| `center` \| `right` | `left` |
| `-f, --font <name>` | Font (currently `standard`) | `standard` |
| `-w, --width <cols>` | Target width for alignment | terminal width |
| `-m, --margin <n>` | Blank lines above/below | `0` |
| `--no-color` | Disable color | — |

### Gradients

`sunset`, `ocean`, `fire`, `mint`, `grape`, `cyberpunk`, `gold`, `ice`, `vaporwave`, `rainbow`, `matrix`, `flamingo`, `mono` — or roll your own with `--colors`.

## Color support

`sigil` emits 24-bit truecolor when `COLORTERM` advertises it, falls back to the 256-color palette otherwise, and prints plain glyphs under `NO_COLOR` or `--no-color`.

## Roadmap

Tracked in [issues](../../issues): bundled fonts, animated reveals, an `export`/embed helper for dropping banners into your own tools, config files, borders, packaging, and CI.

## License

MIT
