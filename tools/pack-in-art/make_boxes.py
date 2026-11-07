#!/usr/bin/env python3
"""Draws box art and marquees for the pack-in games.

  make_boxes.py ART_DIR CACHE_DIR FONTS_DIR

Each box is laid out the way that console's boxes usually are (shape, colours,
title band, stripes) around the game's own title screen, key art or
screenshot, with no console or publisher logos. A game without a logo of its
own also gets a marquee: its title on a transparent background.

Images come from ART_DIR (capture.py's frames) or are downloaded into CACHE_DIR
and checked against their sha256. FONTS_DIR holds the OFL fonts named in STYLES
(Google Fonts). Writes ART_DIR/<system>/<game> - box.png and - marquee.png.
Needs Pillow.
"""

import hashlib
import math
import os
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

GH = "https://raw.githubusercontent.com/"

# Each game: system, file name without extension, title, credit, the picture
# panel's image, and optionally a logo for the title band.
GAMES = [
    ("gb", "Big2Small", "Big2Small", "Matthew D. Steele",
     GH + "mdsteele/big2small/01d1eaa8c87edd29882ef5b8243cff14cd4338f9/images/screenshot1.png#c09b98501caf1c9576739e702abcdc453ad20b4286b0a9068cf9cfc6d8445ad8",
     GH + "mdsteele/big2small/01d1eaa8c87edd29882ef5b8243cff14cd4338f9/images/title.png#3c7b5159f7200a5d8fe4756f8e359256244469e43f241ad09f66093950b654ea"),
    ("gbc", "Tobu Tobu Girl Deluxe", "Tobu Tobu Girl Deluxe", "Tangram Games", "gbc/Tobu Tobu Girl Deluxe - title.png", None),
    ("gba", "The Hat Chooses the Wizard", "The Hat Chooses the Wizard", "Corwin Kuiper & Gwilym Inzani",
     GH + "agbrs/agb/1c2842c954fe1dd40a4b0ce0fbecf4ab7157745a/examples/the-hat-chooses-the-wizard/gfx/splash.png#d145cb56a1ebb8bf72ba16d0a72911e1f308c18e206107736060cec89e9a7711", None),
    ("nds", "Blimp Chicken DS", "Blimp Chicken DS", "TJ M.", "nds/Blimp Chicken DS - title.png", None),
    ("gc", "Vacpyre", "Vacpyre", "Solarscape Games", None, None),
    ("wii", "chro.mono", "chro.mono", "Thomas Perl",
     GH + "mardy/chromono/a31f70ba92dbc3c5d7f053ca5c798ea940d0c47d/chromono.png#f2c3a898f8eb2e30582867d3b30117e24cf139c94bf3b77ed103f277c843d9c3", None),
    ("n3ds", "Anarch", "Anarch", "drummyfish",
     GH + "blitzdoughnuts/AnarchDEVKIT/b5bd91173c64602f8866bafbfd0a197e2c71a241/3DS/banner/banner.png#71f8c4afff6535c4de66637fb54c6bc4710f7e51d3dd71e7908c59535352aba4", None),
    ("nes", "Lala the Magical", "Lala the Magical", "The Mojon Twins",
     "https://www.mojontwins.com/wp-content/uploads/2016/11/lala-gnrom-20161116a-rc1-201611291114173.png#8bf91c6e2b9836304e599a086535cc8e6eccf4b988427381ec7e87cb7b0fb1a6", None),
    ("snes", "Super Sudoku", "Super Sudoku", "Raphaël Assenat", "snes/Super Sudoku - title.png", None),
    ("virtualboy", "VUEngine Platformer Demo", "VUEngine Platformer Demo", "VUEngine",
     GH + "VUEngine/VUEngine-Platformer-Demo/7c2243571e791dead2d849610ad306ed4640d7e9/screenshot.png#8ea27424835306fc09dc4d26dc06c32f53c82d549319e6ba3b4726f44c93b8f3",
     GH + "VUEngine/VUEngine-Platformer-Demo/7c2243571e791dead2d849610ad306ed4640d7e9/assets/images/Stages/TitleScreenStage/Other/Preview/PlatformerDemoLogo.png#88af501595347287775b1090b00fd26b129f703081f0ac185e409117071145a7"),
    ("mastersystem", "Gravity Beam Master Gaiden", "Gravity Beam: Master Gaiden", "Mathew Carr",
     "https://www.smspower.org/uploads/Homebrew/GravityBeamMasterGaiden-SMS-Title.png#5af3789a13a7afce57386a91ac7328a595193b5bdb4a002ae827723b8391a0fe", None),
    ("gamegear", "Waternet", "Waternet", "Willems Davy", "gamegear/Waternet - title.png", None),
    ("genesis", "Miniplanets REMIX Ver", "Miniplanets REMIX", "Sik", "genesis/Miniplanets REMIX Ver - title.png", None),
    ("sega32x", "Dungeons of Noudar 3D", "Dungeons of Noudar 3D", "Daniel Monteiro & Haroldo O. Pinheiro",
     GH + "haroldo-ok/noudar-32x/792ee829cf55bcbcc284851be65b41c1961f1b48/screenshot.png#c3efe9145fb6a0a188b9a5901fb91e6d72da168e7fa504091291a5b3c2c259a6",
     GH + "haroldo-ok/noudar-32x/792ee829cf55bcbcc284851be65b41c1961f1b48/res/logo.png#0e39d00ca30effb51b2adfbc5f472e098d2a4b0bacf54d766e0a85cc31870785"),
    ("dreamcast", "Demon Bazooka", "Demon Bazooka", "Rich Stokes",
     GH + "richstokes/dreamcast-homebrew/0fe12dbad08be321d3ac224f1d7094f34f9ca622/docs/screenshots/demon-bazooka.png#db9e8f44bb7e55c22c7d18591b697f515f1ff92a086179a23c88341c02c76b79", None),
    ("pcengine", "Worm", "Worm", "Andy McCall", "pcengine/Worm - title.png", None),
    ("ngpc", "Stargunner", "Stargunner", "Tixul", None, None),
    ("psp", "EBOOT", "KłełeAtoms", "Nightwolf-47",
     GH + "Nightwolf-47/KleleAtoms-PSP/778526d027f296ca09700e3db1289f0b894b69d9/KABG.PNG#7075e9091b135433d123f54df6fedf6bffd12a6b7a06d9d3564aeaddcee2fc08", None),
    ("atari5200", "Curse of the Lost Miner", "Curse of the Lost Miner", "BAHA Software",
     "atari5200/Curse of the Lost Miner - screenshot.png", None),
    ("atari7800", "robotfindskitten", "robotfindskitten", "Thomas Mathys",
     "https://sourceforge.net/p/rfk7800/screenshot/32624.jpg#86f02fdc61bd4ba4177ef10a592fa145fd7d3cba6f6bedbc1e28b79af66e3a1c", None),
]

# size: box in pixels. bg: colour, or two for a top-to-bottom gradient. grid: line colour over the
# box (or "band" for the title band only). band: title band height (fraction) and colour (None =
# the background). stripes: (colour, height fraction) under the band. side: a left strip's colours
# (one each, stacked) and width fraction. art: the picture panel (left, top, right, bottom
# fractions) and its frame colour. font: title font. title/credit: text colours.
STYLES = {
    "gb": dict(size=(800, 880), bg="#e6e6df", band=(0.16, None), stripes=[("#20206e", 0.008)],
               art=((0.07, 0.2, 0.93, 0.86), "#1c1c1c"), font="ArchivoBlack-Regular", title="#20206e", credit="#555555"),
    "gbc": dict(size=(800, 880), bg="#ffffff", band=(0.16, None), side=(["#d4145a", "#f7931e", "#fcee21", "#39b54a", "#2e3192"], 0.06),
                art=((0.12, 0.2, 0.94, 0.86), "#1c1c1c"), font="ArchivoBlack-Regular", title="#2e3192", credit="#555555"),
    "gba": dict(size=(820, 880), bg="#1d1b38", band=(0.16, None), side=(["#6a5acd"], 0.08),
                art=((0.13, 0.2, 0.95, 0.88), "#0e0d1c"), font="RussoOne-Regular", title="#ffffff", credit="#b8b4e6"),
    "nds": dict(size=(820, 900), bg="#ffffff", band=(0.15, None), stripes=[("#c8c8c8", 0.004)],
                art=((0.06, 0.18, 0.94, 0.9), "#bdbdbd"), font="Exo2", title="#202020", credit="#707070"),
    "n3ds": dict(size=(820, 930), bg="#ffffff", band=(0.15, None), stripes=[("#d0102a", 0.012)],
                 art=((0.06, 0.18, 0.94, 0.91), "#bdbdbd"), font="Exo2", title="#202020", credit="#707070"),
    "gc": dict(size=(720, 1000), bg="#141414", band=(0.13, "#3b2a7a"), stripes=[("#7d6bd1", 0.006)],
               art=((0.06, 0.17, 0.94, 0.92), "#3b2a7a"), font="RussoOne-Regular", title="#ffffff", credit="#a99fe0"),
    "wii": dict(size=(720, 1000), bg="#ffffff", band=(0.12, "#f0f0f0"), stripes=[("#9a9a9a", 0.003)],
                art=((0.06, 0.16, 0.94, 0.92), "#d6d6d6"), font="Exo2", title="#333333", credit="#777777"),
    "psp": dict(size=(580, 1000), bg="#0a0a0a", band=(0.11, None), stripes=[("#3a3a3a", 0.003)],
                art=((0.06, 0.14, 0.94, 0.93), "#2a2a2a"), font="Exo2", title="#ffffff", credit="#9a9a9a"),
    "dreamcast": dict(size=(900, 900), bg="#ffffff", band=(0.14, None), stripes=[("#f06a1d", 0.01)],
                      art=((0.06, 0.18, 0.94, 0.92), "#d0d0d0"), font="RussoOne-Regular", title="#333333", credit="#777777"),
    "nes": dict(size=(720, 1000), bg="#0c0c0c", grid="#262626", band=(0.2, None),
                art=((0.1, 0.23, 0.9, 0.86), "#e8e8e8"), font="ArchivoBlack-Regular", title="#ffffff", credit="#9a9a9a"),
    "snes": dict(size=(1000, 720), bg="#d8d8dd", band=(0.17, "#2a2a32"), stripes=[("#6d4fb0", 0.012)],
                 art=((0.05, 0.23, 0.95, 0.88), "#2a2a32"), font="ArchivoBlack-Regular", title="#ffffff", credit="#55555f"),
    "virtualboy": dict(size=(720, 1000), bg="#000000", band=(0.16, None), stripes=[("#d00000", 0.006)],
                       art=((0.06, 0.2, 0.94, 0.88), "#900000"), font="BlackOpsOne-Regular", title="#ff2020", credit="#a00000"),
    "mastersystem": dict(size=(720, 1000), bg="#ffffff", grid="#d2d2d2", band=(0.17, None), stripes=[("#c8102e", 0.008)],
                         art=((0.08, 0.22, 0.92, 0.86), "#111111"), font="RussoOne-Regular", title="#c8102e", credit="#555555"),
    "gamegear": dict(size=(740, 1000), bg="#f2f2f2", band=(0.15, "#101820"), stripes=[("#00a3e0", 0.01)],
                     art=((0.07, 0.2, 0.93, 0.87), "#101820"), font="RussoOne-Regular", title="#ffffff", credit="#555555"),
    "genesis": dict(size=(720, 1000), bg="#0b0b0b", grid="band", band=(0.18, None), stripes=[("#c8102e", 0.006)],
                    art=((0.07, 0.22, 0.93, 0.88), "#3a3a3a"), font="RussoOne-Regular", title="#ffffff", credit="#9a9a9a"),
    "sega32x": dict(size=(720, 1000), bg="#0b0b0b", band=(0.15, None), stripes=[("#f6c700", 0.008), ("#d50032", 0.008)],
                    art=((0.07, 0.2, 0.93, 0.88), "#3a3a3a"), font="RussoOne-Regular", title="#ffffff", credit="#9a9a9a"),
    "pcengine": dict(size=(760, 880), bg="#151515", band=(0.16, None), stripes=[("#f28c28", 0.01)],
                     art=((0.07, 0.21, 0.93, 0.88), "#3a3a3a"), font="Bungee-Regular", title="#ffffff", credit="#9a9a9a"),
    "ngpc": dict(size=(760, 940), bg="#ffffff", band=(0.15, None), stripes=[("#e60012", 0.008), ("#ffd400", 0.008), ("#0068b7", 0.008)],
                 art=((0.07, 0.2, 0.93, 0.88), "#222222"), font="RussoOne-Regular", title="#111111", credit="#666666"),
    "atari5200": dict(size=(720, 1000), bg=("#e4e4e4", "#9c9c9c"), band=(0.16, "#111111"),
                      stripes=[("#e2231a", 0.01), ("#f7941d", 0.01), ("#fdd20a", 0.01)],
                      art=((0.08, 0.22, 0.92, 0.87), "#111111"), font="BebasNeue-Regular", title="#ffffff", credit="#333333"),
    "atari7800": dict(size=(720, 1000), bg=("#d9d9d9", "#8f8f8f"), band=(0.16, "#111111"), stripes=[("#d71920", 0.01)],
                      art=((0.08, 0.21, 0.92, 0.87), "#111111"), font="BebasNeue-Regular", title="#ffffff", credit="#333333"),
}


def load(src, art_dir, cache):
    if not src.startswith("https://"):
        return Image.open(os.path.join(art_dir, src)).convert("RGBA")
    url, want = src.split("#")
    path = os.path.join(cache, want)
    if not os.path.exists(path):
        subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", path + ".partial", url], check=True)
        os.replace(path + ".partial", path)
    if hashlib.sha256(open(path, "rb").read()).hexdigest() != want:
        sys.exit(f"{url}: sha256 doesn't match")
    return Image.open(path).convert("RGBA")


def is_pixel_art(img):
    return max(img.size) <= 640 and len(img.convert("RGB").getcolors(4096) or ()) < 4096


def scale_into(img, w, h, cover=False):
    """img scaled to fit w x h (or to fill it, cropped, when cover); pixel art stays sharp."""
    factor = (max if cover else min)(w / img.width, h / img.height)
    size = (max(1, round(img.width * factor)), max(1, round(img.height * factor)))
    img = img.resize(size, Image.NEAREST if is_pixel_art(img) and factor >= 1 else Image.LANCZOS)
    if cover:
        left, top = (img.width - w) // 2, (img.height - h) // 2
        img = img.crop((left, top, left + w, top + h))
    return img


def font(fonts, name, size):
    f = ImageFont.truetype(os.path.join(fonts, name + ".ttf"), size)
    try:
        f.set_variation_by_name("Bold")
    except (OSError, ValueError):
        pass
    return f


def fit_text(draw, text, fonts, name, w, h, max_size):
    """The largest font size (and line split) that fits text in w x h, on one line or two."""
    words = text.split()
    pairs = [[" ".join(words[:i]), " ".join(words[i:])] for i in range(1, len(words))]
    splits = [[text]] + sorted(pairs, key=lambda lines: max(len(line) for line in lines))
    for size in range(max_size, 8, -2):
        f = font(fonts, name, size)
        for lines in splits:
            boxes = [draw.textbbox((0, 0), line, font=f) for line in lines]
            line_h = max(b[3] - b[1] for b in boxes)
            if max(b[2] - b[0] for b in boxes) <= w and line_h * len(lines) * 1.15 <= h:
                return f, lines, line_h
    return font(fonts, name, 10), [text], 10


def draw_text_block(draw, text, fonts, name, box, colour, align="center", max_size=120):
    x0, y0, x1, y1 = box
    f, lines, line_h = fit_text(draw, text, fonts, name, x1 - x0, y1 - y0, max_size)
    total = line_h * len(lines) * 1.15
    y = y0 + (y1 - y0 - total) / 2
    for line in lines:
        b = draw.textbbox((0, 0), line, font=f)
        x = x0 if align == "left" else x0 + (x1 - x0 - (b[2] - b[0])) / 2
        draw.text((x - b[0], y - b[1]), line, font=f, fill=colour)
        y += line_h * 1.15


def make_box(game, style, art_dir, cache, fonts):
    system, base, title, credit, art_src, logo_src = game
    W, H = style["size"]
    bg = style["bg"]
    box = Image.new("RGBA", (W, H))
    if isinstance(bg, tuple):
        top, bottom = (Image.new("RGBA", (1, 1), c).getpixel((0, 0)) for c in bg)
        for y in range(H):
            t = y / (H - 1)
            ImageDraw.Draw(box).line([(0, y), (W, y)], fill=tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)))
    else:
        box.paste(bg, (0, 0, W, H))
    d = ImageDraw.Draw(box)
    band_h = round(style["band"][0] * H)
    if style.get("grid"):
        gy1 = band_h if style["grid"] == "band" else H
        colour = "#3a3a3a" if style["grid"] == "band" else style["grid"]
        step = max(W, H) // 25
        for x in range(0, W, step):
            d.line([(x, 0), (x, gy1)], fill=colour, width=2)
        for y in range(0, gy1, step):
            d.line([(0, y), (W, y)], fill=colour, width=2)
    if style["band"][1]:
        d.rectangle([0, 0, W, band_h], fill=style["band"][1])
    y = band_h
    for colour, frac in style.get("stripes", []):
        t = max(2, round(frac * H))
        d.rectangle([0, y, W, y + t], fill=colour)
        y += t
    side_w = 0
    if "side" in style:
        colours, frac = style["side"]
        side_w = round(frac * W)
        seg = H / len(colours)
        for i, colour in enumerate(colours):
            d.rectangle([0, round(i * seg), side_w, round((i + 1) * seg)], fill=colour)

    pad = round(0.04 * W)
    band_box = (side_w + pad, round(0.12 * band_h), W - pad, band_h - round(0.12 * band_h))
    if logo_src:
        logo = load(logo_src, art_dir, cache)
        lw, lh = band_box[2] - band_box[0], band_box[3] - band_box[1]
        logo = scale_into(logo, lw, lh)
        box.alpha_composite(logo, (band_box[0] + (lw - logo.width) // 2, band_box[1] + (lh - logo.height) // 2))
    else:
        draw_text_block(d, title, fonts, style["font"], band_box, style["title"],
                        align="left" if system in ("nes", "snes", "genesis") else "center")

    (l, t, r, b), frame = style["art"]
    ax0, ay0, ax1, ay1 = round(l * W), round(t * H), round(r * W), round(b * H)
    aw, ah = ax1 - ax0, ay1 - ay0
    if art_src:
        art = load(art_src, art_dir, cache)
        # The whole image, over a blurred and darkened copy filling the rest of the panel.
        back = scale_into(art.convert("RGB"), aw, ah, cover=True).filter(ImageFilter.GaussianBlur(max(aw, ah) / 40))
        panel = Image.blend(back, Image.new("RGB", (aw, ah), "#000000"), 0.45).convert("RGBA")
        img = scale_into(art, aw, ah)
        panel.alpha_composite(img, ((aw - img.width) // 2, (ah - img.height) // 2))
    else:
        # No picture of the game: its title, large, on the box's own colours.
        panel = Image.new("RGBA", (aw, ah), style["band"][1] or frame)
        pd = ImageDraw.Draw(panel)
        for i in range(0, aw + ah, max(aw, ah) // 12):
            pd.line([(i, 0), (i - ah, ah)], fill=style.get("stripes", [(frame, 0)])[0][0], width=max(aw, ah) // 40)
        pd.rectangle([aw * 0.08, ah * 0.36, aw * 0.92, ah * 0.64], fill=style["bg"] if isinstance(style["bg"], str) else style["bg"][0])
        draw_text_block(pd, title, fonts, style["font"], (aw * 0.12, ah * 0.38, aw * 0.88, ah * 0.62), style["title"]
                        if style["bg"] != style["title"] else "#000000", max_size=200)
    box.alpha_composite(panel, (ax0, ay0))
    fw = max(2, W // 200)
    d.rectangle([ax0 - fw, ay0 - fw, ax1 + fw - 1, ay1 + fw - 1], outline=frame, width=fw)
    draw_text_block(d, credit, fonts, "Exo2", (side_w + pad, ay1 + fw * 3, W - pad, H - round(0.015 * H)), style["credit"], max_size=34)
    return box.convert("RGB")


def make_marquee(title, style, fonts):
    """The title in the box's font, white with a dark outline, on a transparent background."""
    img = Image.new("RGBA", (1200, 400), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    f, lines, line_h = fit_text(d, title, fonts, style["font"], 1120, 360, 200)
    stroke = max(4, f.size // 14)
    y = (400 - line_h * len(lines) * 1.15) / 2
    for line in lines:
        b = d.textbbox((0, 0), line, font=f, stroke_width=stroke)
        d.text(((1200 - (b[2] - b[0])) / 2 - b[0], y - b[1]), line, font=f, fill="#ffffff",
               stroke_width=stroke, stroke_fill="#141414")
        y += line_h * 1.15
    return img.crop(img.getbbox())


def main():
    art_dir, cache, fonts = sys.argv[1:4]
    os.makedirs(cache, exist_ok=True)
    for game in GAMES:
        system, base, title = game[:3]
        style = STYLES[system]
        os.makedirs(os.path.join(art_dir, system), exist_ok=True)
        out = os.path.join(art_dir, system, f"{base} - box.png")
        make_box(game, style, art_dir, cache, fonts).save(out, optimize=True)
        print(out)
        if system != "virtualboy":   # VUEngine's own logo is its marquee
            make_marquee(title, style, fonts).save(os.path.join(art_dir, system, f"{base} - marquee.png"), optimize=True)


if __name__ == "__main__":
    main()
