"""Generates public/og.png (1200x630), the social sharing image.

Usage: python tools/make_og.py   (requires Pillow; uses Segoe UI when available)
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
W, H = 1200, 630
BG, TEXT, MUTED, ACCENT = (250, 250, 249), (28, 25, 23), (87, 83, 78), (15, 118, 110)


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    for name in ("segoeuib.ttf" if bold else "segoeui.ttf", "arialbd.ttf" if bold else "arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def rounded(img: Image.Image, radius: int) -> Image.Image:
    mask = Image.new("L", img.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, *img.size], radius=radius, fill=255)
    out = img.convert("RGBA")
    out.putalpha(mask)
    return out


def window(shot: Image.Image, width: int, radius: int = 16, bar: int = 32) -> Image.Image:
    """The screenshot inside a generic app window (rounded, with a slim title bar)."""
    shot = shot.convert("RGB")
    shot = shot.resize((width, round(width * shot.height / shot.width)), Image.LANCZOS)
    win = Image.new("RGB", (width, shot.height + bar), (245, 245, 244))
    d = ImageDraw.Draw(win)
    for i in range(3):
        cx = 18 + i * 17
        d.ellipse([cx - 5, bar // 2 - 5, cx + 5, bar // 2 + 5], fill=(214, 211, 209))
    d.line([0, bar - 1, width, bar - 1], fill=(231, 229, 228))
    win.paste(shot, (0, bar))
    out = rounded(win, radius)
    ImageDraw.Draw(out).rounded_rectangle([0, 0, out.width - 1, out.height - 1], radius=radius, outline=(214, 211, 209), width=2)
    return out


def main() -> None:
    img = Image.new("RGBA", (W, H), BG + (255,))
    d = ImageDraw.Draw(img)
    left = 72

    icon = Image.open(ROOT / "src" / "assets" / "icon.png").convert("RGBA").resize((80, 80), Image.LANCZOS)
    img.alpha_composite(icon, (left, 64))
    d.text((left + 100, 104), "CashFlow", font=font(48, True), fill=TEXT, anchor="lm")

    title = font(64, True)
    for i, line in enumerate(["See where", "your money", "flows."]):
        d.text((left, 196 + i * 74), line, font=title, fill=TEXT)

    sub = font(27)
    for i, line in enumerate(["Free desktop app for", "Windows, macOS and Linux"]):
        d.text((left, 448 + i * 36), line, font=sub, fill=MUTED)

    # Button with centred label.
    bx0, by0, bx1, by1 = left, 544, left + 220, 600
    d.rounded_rectangle([bx0, by0, bx1, by1], radius=14, fill=ACCENT)
    d.text(((bx0 + bx1) / 2, (by0 + by1) / 2), "Download", font=font(28, True), fill=(255, 255, 255), anchor="mm")

    shot = Image.open(ROOT / "src" / "assets" / "screenshots" / "en-light.png")
    win = window(shot, 640)
    wx, wy = 600, 120
    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle([wx + 6, wy + 14, wx + win.width, wy + win.height + 14], radius=18, fill=(28, 25, 23, 55))
    img.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(14)))
    img.alpha_composite(win, (wx, wy))

    out = ROOT / "public" / "og.png"
    img.convert("RGB").save(out, optimize=True)
    print("wrote", out)


if __name__ == "__main__":
    main()
