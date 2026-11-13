"""Generates assets/icon.ico: a tiny Sankey diagram on a light rounded square.

Usage: python tools/make_icon.py   (requires Pillow)
"""

from pathlib import Path

from PIL import Image, ImageDraw

S = 1024  # supersampled canvas, downscaled for the final sizes
OUT = Path(__file__).resolve().parent.parent / "assets" / "icon.ico"


def smooth(t: float) -> float:
    return t * t * (3 - 2 * t)


def band(layer: Image.Image, x0, x1, left, right, color, alpha):
    """Fills a ribbon between two vertical spans using a smoothstep curve."""
    steps = 120
    top, bottom = [], []
    for i in range(steps + 1):
        t = i / steps
        x = x0 + (x1 - x0) * t
        k = smooth(t)
        top.append((x, left[0] + (right[0] - left[0]) * k))
        bottom.append((x, left[1] + (right[1] - left[1]) * k))
    poly = top + bottom[::-1]
    ribbon = Image.new("RGBA", layer.size, (0, 0, 0, 0))
    ImageDraw.Draw(ribbon).polygon(poly, fill=color + (alpha,))
    layer.alpha_composite(ribbon)


def main() -> None:
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))

    # Background: vertical gradient clipped to a rounded square.
    bg = Image.new("RGBA", (S, S))
    px = ImageDraw.Draw(bg)
    for y in range(S):
        t = y / (S - 1)
        c = (int(255 + (233 - 255) * t), int(255 + (231 - 255) * t), int(254 + (228 - 254) * t), 255)
        px.line([(0, y), (S, y)], fill=c)
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=230, fill=255)
    img.paste(bg, (0, 0), mask)

    green, blue, orange, teal = (76, 175, 80), (92, 107, 192), (255, 152, 0), (0, 150, 136)
    xl0, xl1 = 170, 214  # source node
    xr0, xr1 = 810, 854  # target nodes

    # (left span, right span, color)
    flows = [
        ((290, 490), (190, 390), blue),
        ((490, 600), (480, 590), orange),
        ((600, 734), (690, 824), teal),
    ]
    for left, right, color in flows:
        band(img, xl1, xr0, left, right, color, 200)

    d = ImageDraw.Draw(img)
    d.rectangle([xl0, 290, xl1, 734], fill=green + (255,))
    for _, (y0, y1), color in flows:
        d.rectangle([xr0, y0, xr1, y1], fill=color + (255,))

    OUT.parent.mkdir(parents=True, exist_ok=True)
    sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (24, 24), (16, 16)]
    img.resize((256, 256), Image.LANCZOS).save(OUT, format="ICO", sizes=sizes)
    img.resize((512, 512), Image.LANCZOS).save(OUT.with_suffix(".png"))
    print("wrote", OUT)


if __name__ == "__main__":
    main()
