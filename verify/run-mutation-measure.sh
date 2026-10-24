#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
#
# The tool-generated mutation measurement of spec 003 (FR-018, SC-012; research R14):
# cargo-mutants over the decoding layer's source files, against a fixed test selection.
#
#   ./run-mutation-measure.sh before   # on the unmodified library: tests only
#   ./run-mutation-measure.sh after    # on the final tree, then the proof classification
#
# Hours of CPU. Run it alone (never next to a Kani or Verus job), on a workstation, not in CI.
# Environment: MUTANTS_JOBS (default 2), MUTANTS_ROOT (the tree to measure; default this
# repository), MUTANTS_ITERATE=1 (resume an interrupted run on the same tree: cargo-mutants'
# `--iterate` keeps the mutants already caught or unviable and tests the rest). The raw cargo-mutants output goes to verify/target/mutants-<phase>/, the
# outcome lists to verify/decoding/mutants-<phase>/, and the summary to stdout.
set -euo pipefail

phase=${1:-}
case "$phase" in
    before|after) ;;
    *) echo "usage: $0 before|after" >&2; exit 2 ;;
esac
here=$(cd "$(dirname "$0")" && pwd)
root=${MUTANTS_ROOT:-$(cd "$here/.." && pwd)}
cd "$root"

# The in-scope files (spec 003 FR-001). `--exclude-re` applies to every file, so it names
# only what is out of scope:
#   - the compact point-vector encoder and the functions only it calls (FR-002, excluded by
#     design): encode_compact, choose_best_level, convert_cells_to_values, choose_base,
#     can_encode, get_block_code, interleave_bit_pairs (\b keeps deinterleave_bit_pairs, which
#     the decoder uses);
#   - encoding.rs's geometry and CellUnion codecs (FR-003, spec 002).
files=(
    core/src/s2/encoded_uint_vector.rs
    core/src/s2/encoded_s2cell_id_vector.rs
    core/src/s2/encoded_string_vector.rs
    core/src/s2/encoded_s2point_vector.rs
    core/src/s2/encoding.rs
)
exclude='encode_compact|choose_best_level|convert_cells_to_values|choose_base|can_encode|get_block_code|\binterleave_bit_pairs|Polygon|polygon|Loop|loop_|LaxPolyline|[Pp]olyline|CellUnion|PointVector|io::Error>::from'

# The test selection. `after` adds the differential tests that spec 003 writes (T021).
tests=(--lib --test decoder_robustness --test property_tests)
[ "$phase" = after ] && tests+=(--test cpp_coding_diff)

out="$here/target/mutants-$phase"
mkdir -p "$out"
args=(-p s2rst --jobs "${MUTANTS_JOBS:-2}" --output "$out" --exclude-re "$exclude")
for f in "${files[@]}"; do args+=(--file "$f"); done
run_args=("${args[@]}")
if [ "${MUTANTS_ITERATE:-0}" = 1 ]; then
    run_args+=(--iterate)
    # Keep the interrupted run's lists: they are merged into this run's below.
    rm -rf "$out/previous"; mkdir -p "$out/previous"
    for k in caught missed timeout unviable; do
        cp "$out/mutants.out/$k.txt" "$out/previous/$k.txt" 2>/dev/null || : > "$out/previous/$k.txt"
    done
fi

echo "run-mutation-measure: $(cargo mutants --version), phase $phase, tree $root"
echo "run-mutation-measure: cargo mutants ${run_args[*]} -- ${tests[*]}"
start=$(date +%s)
# Exit status 2 (missed mutants) and 3 (timeouts) are results, not failures.
set +e
nice -n 19 cargo mutants "${run_args[@]}" -- "${tests[@]}"
rc=$?
set -e
end=$(date +%s)
case "$rc" in
    0|2|3) ;;
    *) echo "run-mutation-measure: cargo mutants failed (exit $rc); see $out/mutants.out/debug.log" >&2; exit "$rc" ;;
esac

dest="$here/decoding/mutants-$phase"
mkdir -p "$dest"
for k in caught missed timeout unviable; do
    if [ -f "$out/mutants.out/$k.txt" ]; then cp "$out/mutants.out/$k.txt" "$dest/$k.txt"; else : > "$dest/$k.txt"; fi
done
if [ -d "$out/previous" ] && [ "${MUTANTS_ITERATE:-0}" = 1 ]; then
    # A resumed run: this run's outcome wins; the earlier run's caught and unviable mutants,
    # which --iterate skipped, carry forward.
    python3 - "$out/previous" "$dest" <<'PY'
import sys
prev, dest = sys.argv[1], sys.argv[2]
kinds = ("caught", "missed", "timeout", "unviable")
read = lambda d, k: [l.rstrip("\n") for l in open(f"{d}/{k}.txt") if l.strip()]
new = {k: read(dest, k) for k in kinds}
seen = {n for k in kinds for n in new[k]}
for k in ("caught", "unviable"):
    new[k] += [n for n in read(prev, k) if n not in seen]
for k in kinds:
    open(f"{dest}/{k}.txt", "w").write("".join(n + "\n" for n in sorted(set(new[k]))))
PY
fi
count() { grep -c . "$dest/$1.txt" || true; }
echo "run-mutation-measure: phase $phase done in $(( (end - start) / 60 )) min"
printf '| Phase | Caught | Missed | Timeout | Unviable | Total |\n|---|---|---|---|---|---|\n'
c=$(count caught); m=$(count missed); t=$(count timeout); u=$(count unviable)
printf '| %s | %s | %s | %s | %s | %s |\n' "$phase" "$c" "$m" "$t" "$u" $(( c + m + t + u ))

# ---- the proof classification (after only; T065) -----------------------------------------
# Every `missed` or `timeout` mutant becomes a patch (its diff from `cargo mutants --list
# --json --diff`) in verify/target/mutation-after/. Each patch is applied in a throwaway
# worktree, and the per-change harnesses that cover its file run through run-verify.sh, under
# the memory guard. A failing harness gives `killed by <harness>`; otherwise the mutant is
# `undetected`, to be reviewed by hand (equivalent, or beyond the proofs' bounds). The result
# table goes to verify/decoding/mutants-after/proofs.tsv.
[ "$phase" = after ] || exit 0
patches="$here/target/mutation-after"
rm -rf "$patches"; mkdir -p "$patches"
cargo mutants "${args[@]}" --list --json --diff > "$patches/list.json"
python3 - "$patches" "$dest" <<'PY'
import json, re, sys
patches, dest = sys.argv[1], sys.argv[2]
listed = {m["name"]: m for m in json.load(open(f"{patches}/list.json"))}
names = []
for k in ("missed", "timeout"):
    names += [(k, l.strip()) for l in open(f"{dest}/{k}.txt") if l.strip()]
with open(f"{patches}/index.tsv", "w") as idx:
    for i, (k, name) in enumerate(names, 1):
        m = listed.get(name)
        if m is None:
            print(f"run-mutation-measure: no diff for {name}", file=sys.stderr)
            continue
        f = m["file"]
        diff = re.sub(r"^--- .*$", f"--- a/{f}", m["diff"], count=1, flags=re.M)
        diff = re.sub(r"^\+\+\+ .*$", f"+++ b/{f}", diff, count=1, flags=re.M)
        pid = f"A-{i:03d}"
        open(f"{patches}/{pid}.patch", "w").write(f"diff --git a/{f} b/{f}\n{diff}")
        idx.write(f"{pid}\t{k}\t{f}\t{name}\n")
PY
# The per-change harness groups that cover each file.
group() {
    case "$1" in
        *encoded_uint_vector.rs) echo "--harness cd_fw:: --harness cd_uv:: --harness cd_agree::a_uv --harness cd_agree::a_fw" ;;
        *encoded_s2cell_id_vector.rs) echo "--harness cd_cv:: --harness cd_agree::a_cv" ;;
        *encoded_string_vector.rs) echo "--harness cd_sv:: --harness cd_agree::a_sv" ;;
        *encoded_s2point_vector.rs) echo "--harness cd_pv:: --harness cd_crash::c_decode_s2point_vector" ;;
        *encoding.rs) echo "--harness cd_fx:: --harness cd_crash:: --harness cd_uv::" ;;
    esac
}
repo=$(git -C "$here" rev-parse --show-toplevel)
wt="$here/target/mutation-after-wt"
: > "$dest/proofs.tsv"
while IFS=$'\t' read -r pid kind file name; do
    rm -rf "$wt"; git -C "$repo" worktree prune
    git -C "$repo" worktree add -q --detach "$wt" HEAD
    rsync -a --delete --exclude target "$here/" "$wt/verify/"
    if ! git -C "$wt" apply "$patches/$pid.patch" 2> "$patches/$pid.apply.log"; then
        printf '%s\t%s\t%s\terror (the patch does not apply)\n' "$pid" "$kind" "$name" >> "$dest/proofs.tsv"
    else
        # shellcheck disable=SC2046
        (cd "$wt/verify" && VERIFY_JOBS=${VERIFY_JOBS:-3} VERIFY_HARNESS_TIMEOUT=${VERIFY_HARNESS_TIMEOUT:-5m} \
            VERIFY_RUN_DIR="target/$pid" ./run-verify.sh $(group "$file") < /dev/null) > "$patches/$pid.log" 2>&1 || true
        r="$wt/verify/target/$pid/results.tsv"
        failed=$(awk -F'\t' '$2 ~ /^FAILED/ {print $1; exit}' "$r" 2>/dev/null || true)
        other=$(awk -F'\t' '$2 !~ /^(FAILED|SUCCESSFUL|KNOWN-DEFECT)/ {printf "%s%s %s", sep, $1, $2; sep=", "}' "$r" 2>/dev/null || true)
        if [ -n "$failed" ]; then res="killed by $failed"
        elif [ ! -s "$r" ]; then res="error (no results; see $patches/$pid.log)"
        elif [ -n "$other" ]; then res="undetected ($other)"
        else res="undetected"; fi
        printf '%s\t%s\t%s\t%s\n' "$pid" "$kind" "$name" "$res" >> "$dest/proofs.tsv"
        echo "run-mutation-measure: $pid $res"
    fi
    git -C "$repo" worktree remove --force "$wt"
done < "$patches/index.tsv"
echo "run-mutation-measure: proof classification in verify/decoding/mutants-after/proofs.tsv"
