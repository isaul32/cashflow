"""Generates public/og.png (1200x630), the social sharing image.

Usage: python tools/make_og.py   (requires Pillow; uses Segoe UI when available)
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

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


def main() -> None:
    img = Image.new("RGBA", (W, H), BG + (255,))
    d = ImageDraw.Draw(img)

    icon = Image.open(ROOT / "src" / "assets" / "icon.png").convert("RGBA").resize((88, 88), Image.LANCZOS)
    img.alpha_composite(icon, (72, 86))
    d.text((176, 100), "CashFlow", font=font(52, True), fill=TEXT)

    d.text((72, 220), "See where", font=font(84, True), fill=TEXT)
    d.text((72, 316), "your money flows.", font=font(84, True), fill=TEXT)
    d.text((72, 448), "Free desktop app for Windows, macOS and Linux", font=font(30), fill=MUTED)
    d.rounded_rectangle([72, 510, 332, 566], radius=14, fill=ACCENT)
    d.text((104, 522), "Download", font=font(30, True), fill=(255, 255, 255))

    shot = Image.open(ROOT / "src" / "assets" / "screenshots" / "en-light.png").convert("RGB")
    shot = shot.resize((760, int(760 * shot.height / shot.width)), Image.LANCZOS)
    shot = rounded(shot, 18)
    # Drop shadow, then the screenshot bleeding off the right edge.
    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle([548, 168, 548 + 760, 168 + shot.height], radius=18, fill=(28, 25, 23, 50))
    img.alpha_composite(shadow)
    img.alpha_composite(shot, (520, 150))

    out = ROOT / "public" / "og.png"
    img.convert("RGB").save(out, optimize=True)
    print("wrote", out)


if __name__ == "__main__":
    main()
