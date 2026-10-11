#!/bin/bash
# Run a script (see README, "Scripted runs") in a headless sway of its own: no window
# on the desktop, saves and settings in a directory of its own, and no sound played
# (a scripted run makes its sound only for its clips). Pictures and clips go to the
# output directory.
#
#   promo/shoot.sh [-s WIDTHxHEIGHT] [-l hall|great] <script.txt> <output dir>
#
# The size is the window's (default 1920x1080). HUSHFOLD_BIN names the game's binary
# (default target/release/hushfold, built with `cargo build --release`).
set -u -o pipefail
SIZE=1920x1080
LIB=""
while getopts "s:l:" o; do
    case $o in
        s) SIZE=$OPTARG ;;
        l) LIB=$OPTARG ;;
        *) sed -n '2,10p' "$0"; exit 2 ;;
    esac
done
shift $((OPTIND - 1))
[ $# -eq 2 ] || { sed -n '2,10p' "$0"; exit 2; }
HERE=$(dirname "$(realpath "$0")")
BIN=$(realpath "${HUSHFOLD_BIN:-$HERE/../target/release/hushfold}")
[ -f "$1" ] || { echo "shoot.sh: no script $1"; exit 2; }
OUT=$(realpath -m "$2")
mkdir -p "$OUT"
[ "$LIB" = great ] || LIB=""
# Short: the Wayland socket's path must fit in a Unix socket address.
BASE=${XDG_RUNTIME_DIR:-/tmp}
R=$(mktemp -d "$BASE/hf.XXXXXX")
chmod 700 "$R"
mkdir -p "$R/data"
SWAY=""
cleanup() {
    [ -n "$SWAY" ] && kill "$SWAY" 2>/dev/null && wait "$SWAY" 2>/dev/null
    rm -rf "$R"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
# The script, ending the run when it is done with it.
SCRIPT="$R/script.txt"
{ cat "$1"; echo; echo quit; } > "$SCRIPT"
cat > "$R/sway.conf" <<CONF
output HEADLESS-1 resolution $SIZE
default_border none
for_window [app_id="hushfold"] fullscreen enable
CONF
env -u WAYLAND_DISPLAY -u DISPLAY XDG_RUNTIME_DIR="$R" WLR_BACKENDS=headless WLR_RENDERER=vulkan \
    WLR_LIBINPUT_NO_DEVICES=1 sway -c "$R/sway.conf" > "$R/sway.log" 2>&1 &
SWAY=$!
for _ in $(seq 50); do
    SOCK=$(ls "$R" 2>/dev/null | grep -m1 '^wayland-[0-9]*$') && break
    sleep 0.1
done
if [ -z "${SOCK:-}" ]; then
    echo "shoot.sh: headless sway did not start:"; cat "$R/sway.log"
    exit 1
fi
(cd "$OUT" && env -u DISPLAY -u HUSHFOLD_AUDIO XDG_RUNTIME_DIR="$R" XDG_DATA_HOME="$R/data" \
    WAYLAND_DISPLAY="$SOCK" HUSHFOLD_LIBRARY="$LIB" HUSHFOLD_UNPACED=1 HUSHFOLD_SCRIPT="$SCRIPT" \
    HUSHFOLD_SCRIPT_DATA="$R/data/script" HUSHFOLD_SHOT_DIR="$OUT" \
    timeout 1800 "$BIN" 2>&1 | { grep -v "^ALSA\|^$" || true; })
exit $?
