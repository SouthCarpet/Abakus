"""Renders src-tauri/icons/icon.ico from the shapes in assets/icon.svg.

Pillow has no SVG parser, so this draws the same primitives (rounded
background, stroked rounded square, two bars, three dots) at high
resolution and downsamples, instead of parsing the SVG file. Keep the
colours and coordinates in sync with assets/icon.svg by hand.

Run: py scripts/generate-icon.py
"""
from PIL import Image, ImageDraw

BG = "#d9f4d9"
FG = "#186a23"
SCALE = 4  # supersample factor for anti-aliasing
BASE = 256


def draw_icon(size: int) -> Image.Image:
    s = size * SCALE
    k = s / BASE  # scale factor from the 256-unit SVG viewBox to this canvas
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, s - 1, s - 1], radius=56 * k, fill=BG)
    stroke_w = max(1, round(20 * k))
    d.rounded_rectangle(
        [52 * k, 52 * k, 204 * k, 204 * k], radius=16 * k, outline=FG, width=stroke_w
    )
    line_w = max(1, round(20 * k))
    d.line([52 * k, 104 * k, 204 * k, 104 * k], fill=FG, width=line_w)
    d.line([52 * k, 152 * k, 204 * k, 152 * k], fill=FG, width=line_w)
    for cx, cy in ((96, 104), (136, 104), (160, 152)):
        r = 18 * k
        d.ellipse([cx * k - r, cy * k - r, cx * k + r, cy * k + r], fill=FG)
    return img.resize((size, size), Image.LANCZOS)


def main() -> None:
    sizes = [16, 32, 48, 64, 128, 256]
    frames = [draw_icon(sz) for sz in sizes]
    out = "src-tauri/icons/icon.ico"
    frames[-1].save(out, format="ICO", sizes=[(f.width, f.height) for f in frames])
    print(f"wrote {out} with sizes {[f.size for f in frames]}")


if __name__ == "__main__":
    main()
