#!/usr/bin/env python3
"""Turns rendered shader screenshots into the thumbnails shipped in the repo.

Per preset two WebPs, named by a hash of the picture so presets with the same output share them:
`<id>.webp` (whole picture, 448x336: curvature, bezels) and `<id>-detail.webp` (448x336 crop at
1:1: scanlines, masks). Black or missing renders get no thumbnail. Writes the preset → id map
and removes WebPs no preset uses any more.

Usage: pack.py <png dir> <presets.txt> <out dir> <map.json>
"""

import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageStat

SIZE = (448, 336)
# robot, gems and ground of the test picture at 3x (viewport 960x720)
DETAIL = (96 / 960, 216 / 720)


def main() -> None:
    png_dir, presets, out, map_path = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4])
    out.mkdir(parents=True, exist_ok=True)
    mapping: dict[str, str] = {}
    failed = []
    for preset in presets.read_text().split():
        png = png_dir / (preset.removesuffix(".slangp") + ".png")
        try:
            img = Image.open(png).convert("RGB")
        except OSError:
            failed.append(preset)
            continue
        if max(ImageStat.Stat(img).mean) < 4:  # black: shader failed or needs more than we give
            failed.append(preset)
            continue
        ident = hashlib.sha1(img.tobytes()).hexdigest()[:12]
        mapping[preset] = ident
        full, detail = out / f"{ident}.webp", out / f"{ident}-detail.webp"
        if not full.exists():
            img.resize(SIZE, Image.Resampling.LANCZOS).save(full, quality=80, method=6)
        if not detail.exists():
            x, y = int(DETAIL[0] * img.width), int(DETAIL[1] * img.height)
            img.crop((x, y, x + SIZE[0], y + SIZE[1])).save(detail, quality=80, method=6)
    used = set(mapping.values())
    for f in out.glob("*.webp"):
        if f.name.split(".")[0].removesuffix("-detail") not in used:
            f.unlink()
    map_path.write_text(json.dumps(mapping, separators=(",", ":"), sort_keys=True) + "\n")
    size = sum(f.stat().st_size for f in out.glob("*.webp")) >> 20
    print(f"{len(mapping)} presets, {len(used)} distinct pictures, {size} MiB; no thumbnail: {len(failed)}")
    for p in failed:
        print(f"  {p}")


if __name__ == "__main__":
    main()
