"""Drafts of Steam's capsule art, from the store screenshots and the logo.

  python3 -I promo/capsules.py <shots dir> <out dir>

The shots dir holds the store screenshots (promo/shots/*.txt), best shot at
3840x2160 (`promo/shoot.sh -s 3840x2160 ...`) for the large capsules and the library
hero. The logo comes from promo/logo/logo.svg and wordmark.svg (promo/logo/make.py
writes them), drawn with resvg. Each capsule is a screenshot cut to its shape about
a point of it, darkened behind the logo so the logo reads, and the logo; the library
hero and the page background are art alone, as Steam asks.
"""
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))

# (name, width, height, screenshot, the point of it kept in view (fractions across
# and down), and the logo: which (logo: the emblem over the name; wordmark: the name
# alone), its width as a fraction of the capsule's, and where its middle is.
CAPSULES = [
    ("header", 920, 430, "07-great-observatory", (0.5, 0.42), ("logo", 0.62, (0.5, 0.5))),
    ("small", 462, 174, "07-great-observatory", (0.5, 0.42), ("wordmark", 0.84, (0.5, 0.5))),
    ("main", 1232, 706, "07-great-observatory", (0.5, 0.42), ("logo", 0.56, (0.5, 0.48))),
    ("vertical", 748, 896, "09-great-clockworks", (0.5, 0.45), ("logo", 0.84, (0.5, 0.24))),
    ("library", 600, 900, "09-great-clockworks", (0.5, 0.45), ("logo", 0.86, (0.5, 0.22))),
    ("library-hero", 3840, 1240, "01-hall-overture", (0.5, 0.52), None),
    ("page-background", 1438, 810, "06-great-atrium", (0.5, 0.5), None),
]


def tight(svg, width):
    """The SVG drawn `width` pixels wide by resvg, cropped to what it draws."""
    with tempfile.TemporaryDirectory() as d:
        png = os.path.join(d, "x.png")
        subprocess.run(["resvg", "-w", str(width), svg, png], check=True)
        im = Image.open(png).convert("RGBA")
        im.load()
    return im.crop(im.getchannel("A").getbbox())


def cover(im, w, h, focus):
    """`im` scaled to cover `w` x `h` and cut to it, keeping `focus` in view."""
    s = max(w / im.width, h / im.height)
    im = im.resize((round(im.width * s), round(im.height * s)), Image.LANCZOS)
    x = min(max(round(focus[0] * im.width - w / 2), 0), im.width - w)
    y = min(max(round(focus[1] * im.height - h / 2), 0), im.height - h)
    return im.crop((x, y, x + w, y + h))


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    shots, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    logos = {k: tight(os.path.join(HERE, "logo", f"{k}.svg"), 2560) for k in ("logo", "wordmark")}
    for name, w, h, shot, focus, logo in CAPSULES:
        base = cover(Image.open(os.path.join(shots, f"{shot}.png")).convert("RGB"), w, h, focus)
        if logo:
            which, frac, (cx, cy) = logo
            mark = logos[which]
            lw = round(w * frac)
            mark = mark.resize((lw, round(mark.height * lw / mark.width)), Image.LANCZOS)
            x, y = round(cx * w - mark.width / 2), round(cy * h - mark.height / 2)
            # A soft dark pool under the logo, and the whole a little dimmer.
            pool = Image.new("L", (w, h), 0)
            pad = 0.18 * max(mark.width, mark.height)
            ImageDraw.Draw(pool).ellipse((x - pad, y - pad, x + mark.width + pad, y + mark.height + pad), fill=200)
            pool = pool.filter(ImageFilter.GaussianBlur(pad * 0.6))
            base = Image.composite(Image.new("RGB", (w, h)), base, pool)
            base = Image.blend(base, Image.new("RGB", (w, h)), 0.12)
            base = base.convert("RGBA")
            base.alpha_composite(mark, (x, y))
            base = base.convert("RGB")
        path = os.path.join(out, f"{name}-{w}x{h}.png")
        base.save(path)
        print(path)


main()
