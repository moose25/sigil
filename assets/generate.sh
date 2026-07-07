#!/usr/bin/env bash
# Regenerate the "spectrum" showcase assets used in the README — a progression
# from a single flat color up to a fully-dressed, heavily-themed banner, plus a
# composite that mixes small and large text in one image.
#
# Run from the repo root:  bash assets/generate.sh
set -euo pipefail

# Prefer a release build if present, else fall back to debug, else build.
if   [ -x target/release/sigil ]; then SIGIL=target/release/sigil
elif [ -x target/debug/sigil   ]; then SIGIL=target/debug/sigil
else cargo build -q && SIGIL=target/debug/sigil
fi

A=assets
svg() { local out="$1"; shift; "$SIGIL" "$@" -F svg -o "$A/$out"; }

# --- The spectrum: same word, dialed from plain to fully dressed --------------
svg spectrum-1-plain.svg    "sigil" -f small -c "#8ab4f8"                               # one flat color, small
svg spectrum-2-color.svg    "sigil" -c "#f7768e"                                        # one flat color, standard font
svg spectrum-3-gradient.svg "sigil" -g ocean                                            # a simple gradient
svg spectrum-4-framed.svg   "sigil" -f small -g mint -b round                           # gradient + rounded frame
svg spectrum-5-themed.svg   "sigil" --theme cyberpunk                                   # a curated theme
svg spectrum-6-max.svg      "sigil" -f big -g sunset -b double --shadow --title sigil   # the works

# --- One image, small + large together ---------------------------------------
# The same word at three tiers — a bold, fully-styled headline down to a plain
# small mark — rendered separately then stacked onto a shared backdrop. This is
# how you get small and large text (and simple + themed) in a single picture.
tmp=$(mktemp -d)
"$SIGIL" "sigil" -f big      -g sunset --shadow -F svg -o "$tmp/1-big.svg"    # large + styled
"$SIGIL" "sigil" -f standard -g ocean           -F svg -o "$tmp/2-mid.svg"    # medium + simple gradient
"$SIGIL" "sigil" -f small    -c "#8b949e"        -F svg -o "$tmp/3-small.svg" # small + one plain color
python assets/compose.py "$A/showcase-stack.svg" "$tmp/1-big.svg" "$tmp/2-mid.svg" "$tmp/3-small.svg"
rm -rf "$tmp"

echo "done — spectrum-*.svg and showcase-stack.svg regenerated"
