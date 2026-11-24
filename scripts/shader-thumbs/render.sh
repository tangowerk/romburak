#!/usr/bin/env bash
# Renders a thumbnail of every slang shader preset into shader-thumbs/ (+ crates/core/data/shader-thumbs.json).
#
# Each preset is applied to the synthetic test picture (testimage.py) shown by a tiny libretro core
# (still.c) in RetroArch, inside a headless Wayland compositor (cage) on the real GPU: nothing
# appears on the desktop. Resumable: rendered PNGs stay in $WORK and are skipped next time.
#
# Usage: scripts/shader-thumbs/render.sh <retroarch binary> <shaders_slang dir> [jobs]
# Needs: cage, cc, python3 with Pillow (webp), curl. Env: WORK (default ~/.cache/romburak-shader-thumbs),
# FRAMES (default 10), ONLY (substring filter on preset paths, for tests).
set -euo pipefail

RA=$(realpath "$1")
SH=$(realpath "$2")
JOBS=${3:-4}
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
WORK=${WORK:-$HOME/.cache/romburak-shader-thumbs}
FRAMES=${FRAMES:-10}
mkdir -p "$WORK/png" "$WORK/log" "$WORK/wrap" "$WORK/pl"

[[ -f $WORK/libretro.h ]] || curl -sfL -o "$WORK/libretro.h" \
  https://raw.githubusercontent.com/libretro/libretro-common/master/include/libretro.h
cc -O2 -shared -fPIC -I"$WORK" "$HERE/still.c" -o "$WORK/still_libretro.so"
python3 -I "$HERE/testimage.py" "$WORK/test.png"
python3 -I -c 'import sys; from PIL import Image
open(sys.argv[2], "wb").write(Image.open(sys.argv[1]).convert("RGB").tobytes("raw", "BGRX"))' \
  "$WORK/test.png" "$WORK/test.rgb"

cat > "$WORK/ra.cfg" <<EOF
video_driver = "glcore"
audio_driver = "null"
audio_enable = "false"
video_fullscreen = "false"
video_scale = "3.000000"
video_shader_enable = "true"
auto_shaders_enable = "false"
config_save_on_exit = "false"
pause_nonactive = "false"
video_font_enable = "false"
video_msg_bg_enable = "false"
video_gpu_screenshot = "true"
history_list_enable = "false"
playlist_directory = "$WORK/pl"
content_history_path = "$WORK/pl/history.lpl"
screenshot_directory = "$WORK"
EOF

render_one() {
  local preset=$1 out="$WORK/png/${1%.slangp}.png" wrap="$WORK/wrap/${1//\//__}"
  [[ -s $out ]] && return 0
  mkdir -p "$(dirname "$out")"
  # wrapper: Mega Bezel's intro would leave the screen black for seconds
  printf '#reference "%s"\nHSM_INTRO_WHEN_TO_SHOW = "0"\n' "$SH/$preset" > "$wrap"
  env -u WAYLAND_DISPLAY -u DISPLAY WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 \
    timeout 120 cage -- "$RA" --config "$WORK/ra.cfg" -L "$WORK/still_libretro.so" --set-shader "$wrap" \
    --max-frames="$FRAMES" --max-frames-ss --max-frames-ss-path="$out" "$WORK/test.rgb" \
    > "$WORK/log/${1//\//__}.log" 2>&1 || echo "failed: $preset" >&2
}
export -f render_one
export RA SH WORK FRAMES

cd "$SH"
find . -name '*.slangp' -printf '%P\n' | sort \
  | grep -vE '^(test|stereoscopic-3d|hdr|subframe-bfi)/' | grep -F -- "${ONLY:-}" \
  > "$WORK/presets.txt"
echo "$(wc -l < "$WORK/presets.txt") presets, $JOBS jobs → $WORK/png"
xargs -a "$WORK/presets.txt" -P "$JOBS" -I{} bash -c 'render_one "$1"' _ {}
python3 -I "$HERE/pack.py" "$WORK/png" "$WORK/presets.txt" "$REPO/shader-thumbs" "$REPO/crates/core/data/shader-thumbs.json"
