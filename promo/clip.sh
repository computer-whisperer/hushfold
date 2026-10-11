#!/bin/bash
# Record a clip script's clips (promo/clips/; `record name` ... `stop` in a script)
# and put each one's frames and sound together for the store: <name>.mp4 (H.264,
# AAC at 48 kHz), beside the near-lossless <name>.mkv and <name>.wav it was made from.
#
#   promo/clip.sh [-s WIDTHxHEIGHT] [-l hall|great] <script.txt> <output dir>
set -u -o pipefail
HERE=$(dirname "$(realpath "$0")")
OUT=$(realpath -m "${@: -1}")
mkdir -p "$OUT"
START="$OUT/.clip-start"
touch "$START"
"$HERE/shoot.sh" "$@" || exit $?
STATUS=0
for video in $(find "$OUT" -maxdepth 1 -name '*.mkv' -newer "$START" | sort); do
    sound=${video%.mkv}.wav
    [ -f "$sound" ] || continue
    ffmpeg -y -loglevel error -i "$video" -i "$sound" -map 0:v -map 1:a \
        -c:v libx264 -preset slow -crf 16 -pix_fmt yuv420p \
        -c:a aac -b:a 256k -ar 48000 -movflags +faststart -shortest "${video%.mkv}.mp4" &&
        echo "clip.sh: ${video%.mkv}.mp4" || STATUS=1
done
rm -f "$START"
exit $STATUS
