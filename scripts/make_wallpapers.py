#!/usr/bin/env python3
"""Draws the Xbox Daijisho wallpaper, which the BB-1 Dark pack doesn't have.

  make_wallpapers.py TEMPLATE SCREENSHOT FONT OUT

TEMPLATE is the pack's default.png (its layout with a placeholder), and
SCREENSHOT is the Second Reality title picture from
https://github.com/kooscode/SecondReality/tree/main/docs/screenshots (Unlicense).
The result keeps the pack's layout: a title on the dark panel, the picture on
the right fading to a color at the edge. Needs Pillow.
"""

import sys

from PIL import Image, ImageDraw, ImageFont

TITLE = "XBOX"
GREEN = (91, 179, 24)


def main():
    template_path, screenshot_path, font_path, out_path = sys.argv[1:5]
    image = Image.open(template_path).convert("RGBA")
    width, height = image.size
    # The pack's panel edge runs from 48% of the width at the top to 36% at
    # the bottom.
    top, bottom = round(width * 0.48), round(width * 0.36)
    region = [(top, 0), (width, 0), (width, height), (bottom, height)]

    shot = Image.open(screenshot_path).convert("RGBA")
    span = width - bottom
    scale = max(span / shot.width, height / shot.height)
    shot = shot.resize((round(shot.width * scale), round(shot.height * scale)), Image.LANCZOS)
    layer = Image.new("RGBA", image.size)
    layer.paste(shot, (bottom + (span - shot.width) // 2, (height - shot.height) // 2))

    fade = Image.new("RGBA", image.size)
    pixels = fade.load()
    for x in range(bottom, width):
        alpha = round(200 * max(0.0, (x - width * 0.65) / (width * 0.35)))
        for y in range(height):
            pixels[x, y] = GREEN + (alpha,)
    layer = Image.alpha_composite(layer, fade)

    mask = Image.new("L", image.size)
    ImageDraw.Draw(mask).polygon(region, fill=255)
    image.paste(layer, (0, 0), mask)

    # Cover the placeholder title with the panel color, then draw ours.
    draw = ImageDraw.Draw(image)
    panel = image.getpixel((20, height // 2))
    draw.rectangle([round(width * 0.08), round(height * 0.3), round(width * 0.36), round(height * 0.52)],
                   fill=panel)
    font = ImageFont.truetype(font_path, round(height * 0.13))
    center = (round(width * 0.24), round(height * 0.45))
    draw.text((center[0] + 6, center[1] + 6), TITLE, font=font, fill=(0, 0, 0), anchor="ms")
    draw.text(center, TITLE, font=font, fill=GREEN, anchor="ms")
    image.convert("RGB").save(out_path, optimize=True)


if __name__ == "__main__":
    main()
