#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "Usage: $0 <seconds>"
    echo "  Runs all fuzz targets sequentially, each for <seconds>/K seconds."
    echo "  Exits non-zero immediately if any target finds a crash."
    exit 1
}

[[ $# -ne 1 || ! "$1" =~ ^[0-9]+$ ]] && usage

TOTAL=$1

TARGETS=()
while IFS= read -r t; do TARGETS+=("$t"); done < <(cargo +nightly fuzz list)

K=${#TARGETS[@]}
PER_TARGET=$(( TOTAL / K ))

if (( PER_TARGET < 1 )); then
    echo "Error: $TOTAL seconds is too short to split across $K targets (need at least $K)." >&2
    exit 1
fi

echo "Running $K fuzz targets, ${PER_TARGET}s each (total budget: ${TOTAL}s)"
echo ""

for target in "${TARGETS[@]}"; do
    echo "==> [$target] fuzzing for ${PER_TARGET}s ..."
    cargo +nightly fuzz run "$target" -- -max_total_time="$PER_TARGET" -rss_limit_mb=8192
    echo "==> [$target] done, no crash found."
    echo ""
done

echo "All $K targets completed without finding a crash."
