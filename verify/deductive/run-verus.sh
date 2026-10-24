#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
#
# Verifies the restated codecs in src/ with Verus, one module at a time, under a memory guard
# (spec 003 FR-010, FR-012; contracts/verify-commands.md, `make verify-deductive`).
#
#   ./run-verus.sh                  # every module
#   MUTANTS=1 ./run-verus.sh        # the V-NNN mutants in mutants/ instead (each must fail)
#
# A mutant counts as killed only if Verus reports a verification error for it. One that
# Verus cannot check (it does not compile, or Verus fails before a result) is an ERROR,
# and fails the run like a survivor.
#
# Environment:
#   VERUS          path to the verus binary (default: `verus` on PATH)
#   VERUS_VERSION  the required release (default 0.2026.09.20.aef82ed)
#   VERUS_MEM      address-space ceiling per Verus run (default 4G; G or M suffix)
#   VERUS_RLIMIT   Verus's per-query resource limit (default 60)
#
# Output: one line per module, `<module>  <n> verified, <e> errors  <seconds> s  <MiB> MiB`,
# then a total. Exit status 0 only if every module verifies with 0 errors.
set -euo pipefail
cd "$(dirname "$0")"

modules=(varint fixed_width uint_vector cellid_vector string_vector)
verus=${VERUS:-verus}
want=${VERUS_VERSION:-0.2026.09.20.aef82ed}
rlimit=${VERUS_RLIMIT:-60}
log_dir=${VERUS_RUN_DIR:-target/run-verus}
mkdir -p "$log_dir"

# ---- the pinned release ---------------------------------------------------------------
if ! command -v "$verus" > /dev/null 2>&1; then
    echo "run-verus: no Verus binary at '$verus'. Install release $want from" >&2
    echo "  https://github.com/verus-lang/verus/releases and set VERUS=<path>/verus" >&2
    exit 2
fi
have=$("$verus" --version 2>&1 | awk '/Version:/ {print $2}')
if [ "$have" != "$want" ]; then
    echo "run-verus: $verus reports version '$have', but $want is required" >&2
    exit 2
fi

# ---- memory guard -----------------------------------------------------------------------
to_kib() {
    case "$1" in
        *[Gg]) echo $(( ${1%[Gg]} * 1024 * 1024 )) ;;
        *[Mm]) echo $(( ${1%[Mm]} * 1024 )) ;;
        *) echo "run-verus: VERUS_MEM must end in G or M (got '$1')" >&2; exit 2 ;;
    esac
}
ceiling_kib=$(to_kib "${VERUS_MEM:-4G}")
backstop=()
if [ "$(uname -s)" = Linux ] && systemd-run --user --scope --quiet true > /dev/null 2>&1; then
    backstop=(systemd-run --user --scope --quiet "--unit=s2rst-verus-$$-$RANDOM"
              -p "MemoryMax=$(( ceiling_kib + 1024 * 1024 ))K" -p MemorySwapMax=0)
fi

# Runs Verus on the crate with the given extra arguments; its output goes to $1.
run_verus() {
    local out=$1; shift
    "${backstop[@]}" bash -c 'ulimit -v "$1"; shift; exec /usr/bin/time -f "time=%e maxrss_kb=%M" "$@"' \
        _ "$ceiling_kib" "$verus" --crate-type=lib src/lib.rs --rlimit "$rlimit" \
        --triggers-mode silent "$@" > "$out" 2>&1
}

# ---- mutants mode -----------------------------------------------------------------------
if [ "${MUTANTS:-0}" = 1 ]; then
    shopt -s nullglob
    patches=(mutants/V-*.patch)
    if [ ${#patches[@]} -eq 0 ]; then
        echo "run-verus: no V-NNN mutants in mutants/"
        exit 0
    fi
    survived=0
    for p in "${patches[@]}"; do
        id=$(basename "$p" .patch)
        work=$(mktemp -d)
        trap 'rm -rf "$work"' EXIT
        cp -r src Cargo.toml "$work/"
        if ! patch -s -p3 -d "$work" < "$p"; then
            echo "$id  ERROR (patch does not apply)"; survived=1; continue
        fi
        if (cd "$work" && run_verus "$OLDPWD/$log_dir/$id.log"); then rc=0; else rc=$?; fi
        # Killed only if Verus ran to the end and reported a verification error. Without a
        # result line it stopped before that (the patch does not compile, the memory ceiling,
        # a crash), and the mutant says nothing about the proofs.
        line=$(grep -E '^verification results::' "$log_dir/$id.log" | tail -1 || true)
        err=$(sed -nE 's/.*, ([0-9]+) errors.*/\1/p' <<< "$line")
        if [ -z "$line" ] || [ -z "$err" ]; then
            echo "$id  ERROR (no verification result, exit status $rc; see $log_dir/$id.log)"
            survived=1
        elif [ "$err" -gt 0 ]; then
            echo "$id  killed"
        else
            echo "$id  SURVIVED"; survived=1
        fi
        rm -rf "$work"
    done
    exit $survived
fi

# ---- normal mode ------------------------------------------------------------------------
status=0
total_ok=0
total_err=0
for m in "${modules[@]}"; do
    out="$log_dir/$m.log"
    if run_verus "$out" --verify-only-module "$m"; then rc=0; else rc=$?; fi
    line=$(grep -E '^verification results::' "$out" | tail -1 || true)
    ok=$(sed -nE 's/.* ([0-9]+) verified.*/\1/p' <<< "$line")
    err=$(sed -nE 's/.*, ([0-9]+) errors.*/\1/p' <<< "$line")
    t=$(sed -nE 's/.*time=([0-9.]+).*/\1/p' "$out" | tail -1)
    kb=$(sed -nE 's/.*maxrss_kb=([0-9]+).*/\1/p' "$out" | tail -1)
    printf '%-14s %4s verified, %s errors  %6s s  %5s MiB\n' "$m" "${ok:-?}" "${err:-?}" \
        "${t:-?}" "$(( ${kb:-0} / 1024 ))"
    if [ "$rc" -ne 0 ] || [ -z "$line" ] || [ "${err:-1}" != 0 ]; then
        status=1
        if [ -z "$line" ]; then tail -20 "$out" >&2; else grep -E '^error' -A12 "$out" | head -60 >&2 || true; fi
    fi
    total_ok=$(( total_ok + ${ok:-0} ))
    total_err=$(( total_err + ${err:-0} ))
done
echo "run-verus: ${#modules[@]} modules: $total_ok verified, $total_err errors (logs in verify/deductive/$log_dir)"
exit $status
