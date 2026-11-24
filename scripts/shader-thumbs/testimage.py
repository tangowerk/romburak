#!/usr/bin/env python3
"""Draws the synthetic pixel-art test picture (320x224) that every shader thumbnail shows.

Made to expose what shaders do: hard sprite edges, gems and diagonals (smoothing/AA), a dithered sky
(de-dithering, NTSC blending), fine stripes and single-pixel text (scanlines, masks, blur),
saturated primaries (colour bleeding, gamma).

Usage: scripts/shader-thumbs/testimage.py out.png
"""

import sys

from PIL import Image, ImageDraw

W, H = 320, 224
SKY = [(24, 20, 72), (40, 36, 120), (64, 72, 176), (104, 128, 216), (160, 184, 240)]


def main() -> None:
    img = Image.new("RGB", (W, H), SKY[0])
    d = ImageDraw.Draw(img)
    # sky: bands with a checkerboard dither between neighbours
    band = 26
    for i, c in enumerate(SKY):
        y0 = i * band
        d.rectangle([0, y0, W - 1, y0 + band - 1], fill=c)
        if i + 1 < len(SKY):
            for y in range(y0 + band - 6, y0 + band):
                for x in range((y % 2), W, 2):
                    img.putpixel((x, y), SKY[i + 1])
    # sun with a hard circular edge
    d.ellipse([236, 18, 284, 66], fill=(248, 216, 96))
    d.ellipse([244, 26, 276, 58], fill=(255, 244, 176))
    # mountains: long diagonals
    d.polygon([(0, 150), (60, 92), (110, 140), (170, 80), (240, 150)], fill=(56, 88, 72))
    d.polygon([(150, 150), (230, 104), (320, 150)], fill=(40, 64, 56))
    for x0, x1 in [(60, 46), (170, 152)]:
        d.polygon([(x0, 92 if x0 == 60 else 80), (x1, 106 if x0 == 60 else 98), (2 * x0 - x1, 106 if x0 == 60 else 98)], fill=(232, 236, 248))
    # ground: grass edge over dirt with scattered stones
    d.rectangle([0, 150, W - 1, H - 1], fill=(112, 72, 56))
    for y in range(158, H, 12):
        for x in range((y // 12 % 2) * 14, W, 28):
            d.ellipse([x, y, x + 9, y + 5], fill=(148, 132, 124), outline=(72, 56, 52))
            d.point((x + 3, y + 1), fill=(204, 196, 188))
    d.rectangle([0, 150, W - 1, 155], fill=(72, 168, 56))
    d.line([0, 150, W - 1, 150], fill=(144, 224, 96))
    for x in range(0, W, 6):
        d.line([x, 156, x + 2, 158], fill=(72, 168, 56))
    # sprite: 16x16 robot, drawn at 2x
    hero = [
        ".......Y........",
        ".......K........",
        "....KKKKKKK.....",
        "...KTTTTTTTK....",
        "...KTWWTWWTK....",
        "...KTWKTWKTK....",
        "...KTTTTTTTK....",
        "...KTRRRRRTK....",
        "....KKKKKKK.....",
        "..KKGGGGGGGKK...",
        ".KGKGGOOOGGKGK..",
        ".KGKGGOOOGGKGK..",
        "..K.KGGGGGK.K...",
        "....KGK.KGK.....",
        "...KKKK.KKKK....",
        "...KKKK.KKKK....",
    ]
    pal = {"K": (24, 28, 40), "T": (64, 200, 192), "W": (240, 248, 255), "R": (232, 72, 104),
           "G": (156, 164, 184), "O": (248, 152, 32), "Y": (255, 232, 64)}
    for j, row in enumerate(hero):
        for i, ch in enumerate(row):
            if ch in pal:
                d.rectangle([40 + i * 2, 118 + j * 2, 41 + i * 2, 119 + j * 2], fill=pal[ch])
    # gems: diamonds with 1-pixel outlines and a highlight
    for k, c in enumerate([(232, 56, 88), (64, 208, 120), (72, 136, 248), (200, 96, 232), (248, 200, 48)]):
        x = 120 + k * 22
        d.polygon([(x + 6, 112), (x + 12, 120), (x + 6, 128), (x, 120)], fill=c, outline=(32, 24, 48))
        d.line([x + 4, 117, x + 6, 115], fill=(255, 255, 255))
    # test strip: 1-px stripes, primaries, grey ramp
    d.rectangle([0, 0, W - 1, 13], fill=(0, 0, 0))
    for x in range(4, 68, 2):
        d.line([x, 2, x, 11], fill=(255, 255, 255))
    for i, c in enumerate([(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0), (0, 255, 255), (255, 0, 255)]):
        d.rectangle([76 + i * 14, 2, 87 + i * 14, 11], fill=c)
    for i in range(8):
        v = i * 255 // 7
        d.rectangle([168 + i * 10, 2, 177 + i * 10, 11], fill=(v, v, v))
    # "SCORE" in a 3x5 pixel font
    font = {
        "S": ["111", "100", "111", "001", "111"], "C": ["111", "100", "100", "100", "111"],
        "O": ["111", "101", "101", "101", "111"], "R": ["110", "101", "110", "101", "101"],
        "E": ["111", "100", "110", "100", "111"], "0": ["111", "101", "101", "101", "111"],
        "4": ["101", "101", "111", "001", "001"], "2": ["111", "001", "111", "100", "111"],
    }
    for n, ch in enumerate("SCORE 004200"):
        for j, row in enumerate(font.get(ch, ["000"] * 5)):
            for i, bit in enumerate(row):
                if bit == "1":
                    img.putpixel((262 + n * 4 + i, 4 + j), (255, 255, 255))
    img.save(sys.argv[1])


if __name__ == "__main__":
    main()
