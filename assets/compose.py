#!/usr/bin/env python3
"""Stack several `sigil -F svg` outputs into one image, centered on a shared
background.

sigil renders one font per invocation, so the way to show *small and large*
text together in a single picture is to compose separate renders. Each sigil
SVG is a root <svg> with one background <rect> and one <text> block; we lift the
<text> block out of each, translate it into place, and drop them all onto one
rounded backdrop.

Usage:
    compose.py OUT.svg  IN1.svg  IN2.svg  [IN3.svg ...]
"""

import re
import sys

PAD = 30  # outer padding around the whole stack (px)
GAP = 18  # vertical gap between stacked banners (px)
BG = "#0d1117"  # backdrop fill (matches sigil's default)


def load(path):
    """Return (width, height, text_block) for one sigil SVG."""
    svg = open(path, encoding="utf-8").read()
    dims = re.search(r'<svg[^>]*\bwidth="(\d+)"[^>]*\bheight="(\d+)"', svg)
    if not dims:
        raise SystemExit(f"{path}: could not read svg dimensions")
    text = re.search(r"<text\b.*?</text>", svg, re.S)
    if not text:
        raise SystemExit(f"{path}: no <text> block found")
    return int(dims.group(1)), int(dims.group(2)), text.group(0)


def main(argv):
    if len(argv) < 3:
        raise SystemExit(__doc__)
    out, ins = argv[1], argv[2:]
    parts = [load(p) for p in ins]

    inner_w = max(w for w, _, _ in parts)
    total_w = inner_w + 2 * PAD
    total_h = 2 * PAD + sum(h for _, h, _ in parts) + GAP * (len(parts) - 1)

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" '
        f'width="{total_w}" height="{total_h}" viewBox="0 0 {total_w} {total_h}">',
        f'<rect width="100%" height="100%" rx="12" fill="{BG}"/>',
    ]
    y = PAD
    for w, h, text in parts:
        x = PAD + (inner_w - w) // 2  # center each banner horizontally
        lines.append(f'<g transform="translate({x},{y})">{text}</g>')
        y += h + GAP
    lines.append("</svg>")

    open(out, "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print(f"wrote {out}  ({total_w}x{total_h})")


if __name__ == "__main__":
    main(sys.argv)
