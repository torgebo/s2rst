#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
#
# Non-vacuity campaign (FR-008; research R7; contracts/verify-commands.md):
#
#   ./run-mutants.sh          # every verify/mutants/M-*.patch
#   ./run-mutants.sh M-001    # one mutant
#   ./run-mutants.sh M-001 M-016   # several
#   ./run-mutants.sh 'M-5*'   # by pattern: spec 006's (M-0NN is spec 001's, … M-5NN spec 006's)
#
# For each mutant: create a throwaway git worktree at HEAD (under verify/target/, which
# git ignores), copy the caller's verify/ into it (so uncommitted harnesses and patches are
# tested), apply the patch, and run every `Expect-Fail` harness through run-verify.sh, so
# the memory guard and the time limit apply. Outcomes:
#   killed    every expected harness reported FAILED
#   survived  some expected harness still succeeded (the property is vacuous or too weak)
#   no-verdict  no expected harness succeeded, and one ended in KNOWN-TIMEOUT: its unmutated
#             run times out too (known-timeouts.txt), so the campaign cannot tell; not an error
#   error     the patch did not apply, or a harness ended in OOM/TIMEOUT/ERROR or did not run
# Failed harnesses are not re-run for a counterexample (VERIFY_PLAYBACK=0): the kill is the
# FAILED status, and the re-run can take as long as the time limit.
# A `Tier: slow` header (contracts/harness-conventions.md) builds the mutant with
# `--features slow` and the slow-tier time limit (VERIFY_SLOW_HARNESS_TIMEOUT, default 30m).
# A `Memory: 8G` header sets that mutant's per-harness ceiling (VERIFY_MEM_PER_HARNESS), for
# expected harnesses that need more than the default 4G (spec 002's 25-byte decode proofs).
# A `Test: <dir> <cargo test arguments>` header (spec 007; one or more) also runs an ordinary
# test in the worktree, `cd <dir> && cargo test <arguments>`: the mutant is killed by it when the
# output reports `test result: FAILED`, survives when every result is `ok`, and is an error
# otherwise (a build failure). It is for properties Kani cannot check, such as `normalized`
# through Kani's wrong `%` and the enumeration of `tests/angle_enumeration.rs`. Expect-Fail may
# then be empty.
# A `Tier: xlarge` header (spec 003) builds with `--features xlarge,slow`, the xlarge time
# limit (VERIFY_XLARGE_HARNESS_TIMEOUT, default 4h) and a 26G ceiling unless `Memory:` says
# otherwise. Such a mutant needs a 32 GB machine, so it is skipped unless VERIFY_XLARGE=1.
# The caller's working tree is never modified, except verify/MUTANTS.md, which is
# regenerated from the patch headers and the outcomes (earlier outcomes are kept for
# mutants that were not re-run).
set -euo pipefail
cd "$(dirname "$0")"
verify_dir=$(pwd)
repo=$(git rev-parse --show-toplevel)
read -ra filters <<< "$*"     # ids or patterns, split without expanding the patterns
wt_root="$verify_dir/target/mutant-worktrees"
mkdir -p "$wt_root"

cleanup() {
    for wt in "$wt_root"/*; do
        [ -d "$wt" ] && git -C "$repo" worktree remove --force "$wt" > /dev/null 2>&1 || true
    done
    git -C "$repo" worktree prune > /dev/null 2>&1 || true
}
# Ctrl-C or TERM ends the run (cleaning up through the EXIT trap), instead of carrying on
# with the loop in a worktree that cleanup has just removed.
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if [ -n "$(git -C "$repo" status --porcelain core/)" ]; then
    echo "run-mutants: WARNING: core/ has uncommitted changes; mutants apply to HEAD" >&2
fi

selected() {   # whether mutant $1 matches one of the filters (all, without filters)
    local f
    [ "${#filters[@]}" -eq 0 ] && return 0
    for f in "${filters[@]}"; do
        # shellcheck disable=SC2053  # $f is a pattern
        [[ "$1" == $f ]] && return 0
    done
    return 1
}

declare -A outcome=()
n_killed=0 n_survived=0 n_error=0 n_noverdict=0
for patch in mutants/M-*.patch; do
    [ -e "$patch" ] || continue
    id=$(sed -n 's/^Mutant: *//p' "$patch" | head -n 1)
    selected "$id" || continue
    expect=$(sed -n 's/^Expect-Fail: *//p' "$patch" | head -n 1 | tr ',' ' ')
    tier=$(sed -n 's/^Tier: *//p' "$patch" | head -n 1)
    mem=$(sed -n 's/^Memory: *//p' "$patch" | head -n 1)
    mem=${mem:-${VERIFY_MEM_PER_HARNESS:-4G}}
    features=() limit=${VERIFY_HARNESS_TIMEOUT:-12m}
    if [ "$tier" = slow ]; then
        features=(--features slow) limit=${VERIFY_SLOW_HARNESS_TIMEOUT:-30m}
    elif [ "$tier" = xlarge ]; then
        if [ "${VERIFY_XLARGE:-0}" != 1 ]; then
            echo "$id skipped (xlarge tier; set VERIFY_XLARGE=1 on a 32 GB machine)"
            continue
        fi
        features=(--features xlarge,slow) limit=${VERIFY_XLARGE_HARNESS_TIMEOUT:-4h}
        [ -z "$(sed -n 's/^Memory: *//p' "$patch")" ] && mem=26G
    fi
    wt="$wt_root/$id"
    git -C "$repo" worktree add -q --detach "$wt" HEAD
    rsync -a --delete --exclude target "$verify_dir/" "$wt/verify/"
    result=killed
    if ! git -C "$wt" apply "$verify_dir/$patch" 2> "$wt_root/$id.apply.log"; then
        result=error
        echo "run-mutants: $id: patch does not apply" >&2
    else
        while IFS= read -r t; do
            [ -n "$t" ] || continue
            tdir=${t%% *} targs=${t#* }
            tlog="$wt_root/$id.test.${tdir//\//_}.log"
            set +e
            # shellcheck disable=SC2086  # the arguments are split on purpose
            (cd "$wt/$tdir" && cargo test $targs) > "$tlog" 2>&1
            set -e
            if grep -q "test result: FAILED" "$tlog"; then
                :
            elif grep -q "test result: ok" "$tlog"; then
                case "$result" in killed|no-verdict) result=survived ;; esac
            else
                result=error
                echo "run-mutants: $id: the test '$t' did not run (log kept: verify/target/mutant-worktrees/$id.test.${tdir//\//_}.log)" >&2
            fi
        done < <(sed -n 's/^Test: *//p' "$patch")
        for h in $expect; do
            set +e
            (cd "$wt/verify" && VERIFY_HARNESS_TIMEOUT=$limit VERIFY_MEM_PER_HARNESS=$mem VERIFY_PLAYBACK=0 \
                ./run-verify.sh "${features[@]}" --exact --harness "$h") \
                > "$wt_root/$id.$h.log" 2>&1
            set -e
            st=$(awk -v h="$h" '$1 == h {print $2}' "$wt_root/$id.$h.log" | head -n 1)
            case "$st" in
                # A failed check kills the mutant, with or without unsatisfied covers.
                FAILED*) ;;
                SUCCESSFUL*) case "$result" in killed|no-verdict) result=survived ;; esac ;;
                KNOWN-TIMEOUT*) [ "$result" = killed ] && result=no-verdict ;;
                *) result=error
                   echo "run-mutants: $id: $h ended in '${st:-no result}' (log kept: verify/target/mutant-worktrees/$id.$h.log)" >&2 ;;
            esac
        done
    fi
    git -C "$repo" worktree remove --force "$wt"
    outcome[$id]=$result
    case "$result" in
        killed) n_killed=$((n_killed + 1)) ;;
        survived) n_survived=$((n_survived + 1)) ;;
        no-verdict) n_noverdict=$((n_noverdict + 1)) ;;
        *) n_error=$((n_error + 1)) ;;
    esac
    echo "$id $result"
done

# Regenerate MUTANTS.md: one row per patch, keeping earlier outcomes for mutants not re-run.
outcomes_file="$wt_root/outcomes.tsv"
: > "$outcomes_file"
for id in "${!outcome[@]}"; do printf '%s\t%s\n' "$id" "${outcome[$id]}" >> "$outcomes_file"; done
python3 - "$outcomes_file" <<'PY'
import glob, re, sys, datetime
new = dict(line.rstrip("\n").split("\t") for line in open(sys.argv[1]) if line.strip())
old = {}
try:
    for line in open("MUTANTS.md"):
        m = re.match(r"^\| (M-\d+) \|.*\| (\S+) \|$", line.rstrip())
        if m:
            old[m.group(1)] = m.group(2)
except FileNotFoundError:
    pass
rows = []
for path in sorted(glob.glob("mutants/M-*.patch")):
    hdr = {}
    for line in open(path):
        if line.startswith("diff --git"):
            break
        if ":" in line:
            k, v = line.split(":", 1)
            hdr[k.strip()] = v.strip()
    mid = hdr.get("Mutant", path)
    res = new.get(mid, old.get(mid, "not-run"))
    rows.append(f"| {mid} | {hdr.get('Defect', '')} | {hdr.get('Targets', '')} | "
                f"`{hdr.get('Expect-Fail', '')}` | {res} |")
with open("MUTANTS.md", "w") as f:
    f.write("# Mutants: non-vacuity evidence (FR-008, SC-004)\n\n")
    f.write("Each mutant injects one defect into `core/`; every listed harness must fail on it "
            "(`killed`). Generated by `run-mutants.sh` (`make verify-mutants`); last updated "
            f"{datetime.date.today().isoformat()}.\n\n")
    f.write("| Id | Defect | Targets | Expect-Fail | Outcome |\n")
    f.write("|----|--------|---------|-------------|---------|\n")
    f.write("\n".join(rows) + "\n")
PY

echo "run-mutants: $n_killed killed, $n_survived survived, $n_noverdict no-verdict, $n_error error"
[ "$n_survived" -eq 0 ] && [ "$n_error" -eq 0 ]
