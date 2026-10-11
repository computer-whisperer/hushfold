"""The logo: a folded-paper crane (the familiars) alighting on an open book under a
crescent moon, over the name in Cinzel, the face the library letters its signs in.
The name is set as outlines, so the SVGs need no font to be drawn.

  python3 -I promo/logo/make.py <Cinzel[wght].ttf> [<png dir>]

Cinzel[wght].ttf is google/fonts' ofl/cinzel at the revision assets/fonts/subset.py
names (which downloads it into its download dir). Writes, beside this script:
logo.svg (the emblem over the name), wordmark.svg (the name alone) and emblem.svg
(the emblem alone, for icons); with a png dir, renders them there with resvg at the
sizes the store and the game's icon want (see SIZES).
"""
import os
import subprocess
import sys

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

HERE = os.path.dirname(os.path.abspath(__file__))

# The HUD's gold and ink (src/hud.rs), deepened and lightened for a gradient.
GOLD_HI, GOLD, GOLD_LO = "#f6dc9a", "#e8ba60", "#b8833a"
PAPER_HI, PAPER, PAPER_LO = "#fbf3e2", "#ece0c8", "#c9b894"
NIGHT = "#141a33"

# (file, width, height, what): PNGs rendered with a png dir.
SIZES = [
    ("logo.svg", 1280, 720, "the store's library logo (transparent)"),
    ("logo.svg", 2560, 1440, "the same, large"),
    ("wordmark.svg", 1600, 400, "the name alone"),
    ("emblem.svg", 512, 512, "icon"),
    ("emblem.svg", 256, 256, "icon"),
    ("emblem.svg", 184, 184, "the store's community icon"),
    ("emblem.svg", 64, 64, "icon"),
    ("emblem.svg", 32, 32, "icon"),
]


def defs():
    return f"""<defs>
  <linearGradient id="gold" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="{GOLD_HI}"/><stop offset="0.55" stop-color="{GOLD}"/>
    <stop offset="1" stop-color="{GOLD_LO}"/>
  </linearGradient>
  <linearGradient id="paper" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0" stop-color="{PAPER_HI}"/><stop offset="1" stop-color="{PAPER_LO}"/>
  </linearGradient>
</defs>"""


def poly(points, fill, extra=""):
    pts = " ".join(f"{x:g},{y:g}" for x, y in points)
    return f'<polygon points="{pts}" fill="{fill}"{extra}/>'


def emblem():
    """The emblem in a 200 x 200 box: the moon behind, the crane, the book."""
    parts = []
    # The crescent: a disc with a disc beside it masked out.
    parts.append(
        '<mask id="bite"><rect width="200" height="200" fill="white"/>'
        '<circle cx="166" cy="34" r="27" fill="black"/></mask>'
        '<circle cx="150" cy="46" r="30" fill="url(#gold)" mask="url(#bite)"/>'
    )
    # The crane, folded paper: each face a plane of its own shade, wings raised.
    shade = {"hi": PAPER_HI, "mid": PAPER, "lo": PAPER_LO, "dark": "#a8976f"}
    crane = [
        # the far wing, in shadow, mostly behind the near one
        ([(82, 114), (54, 32), (104, 108)], "dark"),
        # the tail, rising behind
        ([(84, 120), (34, 78), (78, 124)], "lo"),
        ([(84, 120), (34, 78), (82, 115)], "mid"),
        # the body, a diamond of two folds
        ([(72, 122), (100, 106), (100, 136)], "mid"),
        ([(100, 106), (130, 122), (100, 136)], "lo"),
        # the neck, rising before, and the head bent down from it
        ([(116, 126), (156, 70), (123, 128)], "hi"),
        ([(116, 126), (156, 70), (113, 119)], "mid"),
        ([(156, 70), (170, 80), (158, 79)], "hi"),
        # the near wing: broad, two faces either side of its fold
        ([(86, 116), (132, 24), (98, 114)], "mid"),
        ([(98, 114), (132, 24), (122, 116)], "hi"),
    ]
    for pts, s in crane:
        parts.append(poly(pts, shade[s], ' stroke="#8a7650" stroke-width="0.6" stroke-linejoin="round"'))
    # The open book: two pages bowing from the spine, a gold edge under them.
    book = (
        '<path fill="url(#gold)" d="M 100 152 C 82 142 56 140 32 146 L 32 157 C 56 151 82 153 100 163 '
        "C 118 153 144 151 168 157 L 168 146 C 144 140 118 142 100 152 Z\"/>"
        '<path fill="url(#paper)" d="M 100 148 C 82 136 58 134 36 140 L 36 149 C 58 143 82 145 100 155 Z"/>'
        '<path fill="url(#paper)" d="M 100 148 C 118 136 142 134 164 140 L 164 149 C 142 143 118 145 100 155 Z"/>'
        '<path fill="none" stroke="#b39f78" stroke-width="0.8" d="M 46 143 C 64 140 84 142 96 148 '
        'M 104 148 C 116 142 136 140 154 143"/>'
    )
    parts.append(book)
    return "\n".join(parts)


def name_paths(font, text, size, tracking):
    """`text` in `font` as SVG paths, `size` units to the em, its letters spaced
    `tracking` ems apart (and kerned); the path data with its baseline at 0, and its
    width."""
    upm = font["head"].unitsPerEm
    scale = size / upm
    cmap = font.getBestCmap()
    gs = font.getGlyphSet()
    hmtx = font["hmtx"]
    kern = {}
    if "kern" in font:
        for t in font["kern"].kernTables:
            kern.update(t.kernTable)
    out, x, prev = [], 0.0, None
    for ch in text:
        if ch == " ":
            x += size * 0.3
            prev = None
            continue
        g = cmap[ord(ch)]
        if prev is not None:
            x += kern.get((prev, g), 0) * scale
        pen = SVGPathPen(gs)
        gs[g].draw(TransformPen(pen, (scale, 0, 0, -scale, x, 0)))
        out.append(pen.getCommands())
        x += hmtx[g][0] * scale + tracking * size
        prev = g
    return " ".join(out), x - tracking * size


def svg(w, h, body, view=None):
    """An SVG of `body`, `w` x `h`, showing `view` of it (a viewBox; the whole of it)."""
    view = view or f"0 0 {w:g} {h:g}"
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view}" width="{w:g}" height="{h:g}">\n'
        f"{defs()}\n{body}\n</svg>\n"
    )


def star(cx, cy, r):
    """A four-pointed star, as the HUD's ✦."""
    k = r * 0.28
    return poly(
        [(cx, cy - r), (cx + k, cy - k), (cx + r, cy), (cx + k, cy + k), (cx, cy + r), (cx - k, cy + k),
         (cx - r, cy), (cx - k, cy - k)],
        "url(#gold)",
    )


def wordmark(font):
    """The name: HUSHFOLD over LIBRARY between rules; (svg body, width, height)."""
    big, bw = name_paths(font, "HUSHFOLD", 120, 0.06)
    small, sw = name_paths(font, "LIBRARY", 46, 0.42)
    w = max(bw, sw + 220) + 20
    h = 210
    bx, sx = (w - bw) / 2, (w - sw) / 2
    rule = 90
    body = (
        f'<g fill="url(#gold)"><path transform="translate({bx:.1f} 112)" d="{big}"/>'
        f'<path transform="translate({sx:.1f} 186)" d="{small}"/></g>'
        f'<g stroke="{GOLD}" stroke-width="2.4" stroke-linecap="round">'
        f'<line x1="{sx - 34 - rule:.1f}" y1="170" x2="{sx - 34:.1f}" y2="170"/>'
        f'<line x1="{sx + sw + 34:.1f}" y1="170" x2="{sx + sw + 34 + rule:.1f}" y2="170"/></g>'
        f"{star(sx - 22, 170, 9)}{star(sx + sw + 22, 170, 9)}"
    )
    return body, w, h


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    font = instancer.instantiateVariableFont(TTFont(sys.argv[1]), {"wght": 700})
    wm, ww, wh = wordmark(font)
    files = {
        "wordmark.svg": svg(ww, wh, wm),
        "emblem.svg": svg(200, 200, emblem(), view="25 13.5 152 152"),
    }
    # The lockup, on a 16:9 canvas (the store's library logo is 1280 x 720): the
    # emblem over the name.
    W, H = 1280, 720
    s = 1.55
    ex, ey = W / 2 - 100 * s, 36
    mx = (W - ww) / 2
    files["logo.svg"] = svg(
        W, H,
        f'<g transform="translate({ex:.1f} {ey}) scale({s})">{emblem()}</g>'
        f'<g transform="translate({mx:.1f} {ey + 172 * s:.1f})">{wm}</g>',
    )
    for name, text in files.items():
        with open(os.path.join(HERE, name), "w") as f:
            f.write(text)
        print(name)
    if len(sys.argv) > 2:
        out = sys.argv[2]
        os.makedirs(out, exist_ok=True)
        for name, w, h, _ in SIZES:
            png = os.path.join(out, f"{os.path.splitext(name)[0]}-{w}x{h}.png")
            subprocess.run(["resvg", "-w", str(w), "-h", str(h), os.path.join(HERE, name), png], check=True)
            print(png)


main()
