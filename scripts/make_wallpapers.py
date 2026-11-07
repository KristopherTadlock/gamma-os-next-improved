#!/usr/bin/env python3
"""Draws the Daijisho platform wallpapers in mods/daijisho-wallpapers/.

  make_wallpapers.py OUT_DIR FONT_BOLD FONT_REGULAR

Each is 1080x720 like GammaOS's own: the platform's name on a dark diagonal
panel, next to a color gradient. They use plain text only, no logos or game
art. Needs Pillow.
"""

import os
import sys

from PIL import Image, ImageDraw, ImageFont

WIDTH, HEIGHT = 1080, 720
PANEL = [(0, 0), (520, 0), (390, HEIGHT), (0, HEIGHT)]

# shortname: (name, caption, gradient start, gradient end)
PLATFORMS = {
    "gc": ("GameCube", "Nintendo · 2001", "#7e57c2", "#311b92"),
    "wii": ("Wii", "Nintendo · 2006", "#4fc3f7", "#01579b"),
    "3ds": ("3DS", "Nintendo · 2011", "#ef5350", "#7f0000"),
    "xbox": ("Xbox", "Microsoft · 2001", "#66bb6a", "#1b5e20"),
    "scummvm": ("ScummVM", "Adventure games", "#ffa726", "#6d2e00"),
    "virtualboy": ("Virtual Boy", "Nintendo · 1995", "#e53935", "#210000"),
    "sega32x": ("32X", "Sega · 1994", "#ffca28", "#b71c1c"),
    "tg16": ("PC Engine", "NEC · 1987", "#ff8a65", "#4e342e"),
    "atari2600": ("Atari 2600", "Atari · 1977", "#bcaaa4", "#3e2723"),
    "atari5200": ("Atari 5200", "Atari · 1982", "#90a4ae", "#212121"),
    "atari7800": ("Atari 7800", "Atari · 1986", "#ef9a9a", "#263238"),
}


def rgb(color):
    return tuple(int(color[i : i + 2], 16) for i in (1, 3, 5))


def gradient(start, end):
    image = Image.new("RGB", (WIDTH, HEIGHT))
    pixels = image.load()
    a, b = rgb(start), rgb(end)
    for y in range(HEIGHT):
        for x in range(WIDTH):
            t = min(1.0, (x / WIDTH * 0.6 + y / HEIGHT * 0.4) * 1.1)
            pixels[x, y] = tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))
    # Faint stripes parallel to the panel's edge.
    stripes = Image.new("RGBA", (WIDTH, HEIGHT))
    draw = ImageDraw.Draw(stripes)
    for offset in range(560, WIDTH + 200, 90):
        draw.polygon([(offset, 0), (offset + 40, 0), (offset - 90, HEIGHT), (offset - 130, HEIGHT)],
                     fill=(255, 255, 255, 18))
    return Image.alpha_composite(image.convert("RGBA"), stripes)


def fitted(text, path, width, size):
    while True:
        font = ImageFont.truetype(path, size)
        if font.getlength(text) <= width or size <= 20:
            return font
        size -= 4


def draw(shortname, out_dir, bold, regular):
    name, caption, start, end = PLATFORMS[shortname]
    image = gradient(start, end)
    draw = ImageDraw.Draw(image)
    draw.polygon(PANEL, fill="#1a1a1a")
    title = fitted(name, bold, 380, 96)
    sub = ImageFont.truetype(regular, 34)
    center = 230
    draw.text((center, 330), name, font=title, fill="white", anchor="ms")
    draw.text((center, 390), caption, font=sub, fill="#b0b0b0", anchor="ms")
    image.convert("RGB").save(os.path.join(out_dir, shortname + ".png"), optimize=True)


def main():
    out_dir, bold, regular = sys.argv[1:4]
    os.makedirs(out_dir, exist_ok=True)
    for shortname in PLATFORMS:
        draw(shortname, out_dir, bold, regular)


if __name__ == "__main__":
    main()
