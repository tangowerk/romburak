#!/usr/bin/env python3
"""Builds crates/core/data/shader-index.json from the git history of libretro/slang-shaders.

Per preset (`.slangp`, path below the repo root = below `shaders_slang`): date added, date of
the last change, and the author who added it. Renames carry the history along.

Usage: scripts/shader-index.py <slang-shaders clone with full history> [out.json]
"""

import json
import subprocess
import sys
from pathlib import Path


def main() -> None:
    repo = Path(sys.argv[1])
    out = Path(sys.argv[2] if len(sys.argv) > 2 else "crates/core/data/shader-index.json")
    log = subprocess.run(
        ["git", "-C", str(repo), "log", "--reverse", "-M", "--name-status",
         "--format=\x01%H\t%as\t%aN"],
        check=True, capture_output=True, text=True,
    ).stdout
    meta: dict[str, list[str]] = {}  # path -> [added, changed, author]
    date = author = ""
    for line in log.splitlines():
        if line.startswith("\x01"):
            _, date, author = line[1:].split("\t", 2)
            continue
        if not line:
            continue
        status, *paths = line.split("\t")
        kind = status[0]
        if kind == "R":
            old, new = paths
            rec = meta.pop(old, None)
            if new.endswith(".slangp"):
                meta[new] = [rec[0], date, rec[2]] if rec else [date, date, author]
        elif kind == "C":
            if paths[1].endswith(".slangp"):
                meta[paths[1]] = [date, date, author]
        elif paths[0].endswith(".slangp"):
            if kind == "D":
                meta.pop(paths[0], None)
            elif kind == "A" or paths[0] not in meta:
                meta[paths[0]] = [date, date, author]
            else:
                meta[paths[0]][1] = date
    present = {p.relative_to(repo).as_posix() for p in repo.rglob("*.slangp")}
    head = subprocess.run(
        ["git", "-C", str(repo), "log", "-1", "--format=%H %as"],
        check=True, capture_output=True, text=True,
    ).stdout.split()
    index = {
        "source": f"libretro/slang-shaders@{head[0]}",
        "date": head[1],
        "presets": {p: meta[p] for p in sorted(present) if p in meta},
    }
    missing = len(present) - len(index["presets"])
    out.write_text(json.dumps(index, ensure_ascii=False, separators=(",", ":")) + "\n")
    print(f"{len(index['presets'])} presets → {out} ({out.stat().st_size // 1024} KiB, {missing} without history)")


if __name__ == "__main__":
    main()
