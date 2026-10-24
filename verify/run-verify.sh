#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
# Written for this crate (not ported from upstream S2).
#
# Runs the Kani harnesses of verify/ under a memory guard and prints one line per
# harness (contracts/verify-commands.md; research R7, R9, R10, R15): a timestamped progress
# line as each finishes, with the counts so far (also in $VERIFY_RUN_DIR/progress.log), then
# the sorted report.
#
#   ./run-verify.sh                                # every harness
#   ./run-verify.sh --exact --harness crash::c_parent
#
# Environment:
#   VERIFY_MEM_PER_HARNESS  per-process memory ceiling (default 4G; accepts G/M suffixes)
#   VERIFY_JOBS             parallel harnesses (default: what the machine's available
#                           memory allows at 7/8 of the ceiling per job, i.e. 3.5 GiB at
#                           the default ceiling, capped at nproc)
#   VERIFY_PLAYBACK_MEM     memory ceiling for the concrete-playback re-run of a failed
#                           harness (default 3 × the per-harness ceiling, at most
#                           MemAvailable − 2 GiB; it runs alone, after the main run)
#   VERIFY_HARNESS_TIMEOUT  per-harness time limit (default 12m, the R15 hard limit on CI;
#                           use 8m on a workstation as the CI proxy). Uses Kani's unstable
#                           --harness-timeout.
#   VERIFY_PLAYBACK         0 skips the concrete-playback re-run of failed harnesses
#                           (run-mutants.sh sets it: a kill needs only the FAILED status, and
#                           the re-run can take as long as the time limit)
#
# Statuses: SUCCESSFUL, FAILED, OOM (hit the ceiling, in the solver or before it: CBMC's
# exit status 6), TIMEOUT (hit the time limit), ERROR (the tool failed without a failed
# check), KNOWN-DEFECT D-NNN (listed in known-defects.txt), KNOWN-TIMEOUT (a TIMEOUT listed
# in known-timeouts.txt, a recorded tool limit). The run fails on any FAILED/OOM/ERROR and
# unlisted TIMEOUT, on any unsatisfied cover (Kani 0.68.0 itself exits 0 then), on a stale
# known-defect or known-timeout entry, and when the memory backstop is hit. Failed harnesses
# are re-run with concrete playback, so the output shows a concrete counterexample.
set -euo pipefail
cd "$(dirname "$0")"

# Logs go to VERIFY_RUN_DIR (default target/run-verify). Give concurrent runs separate
# directories, and separate CARGO_TARGET_DIRs, so they do not overwrite each other.
log_dir=${VERIFY_RUN_DIR:-target/run-verify}
mkdir -p "$log_dir"
log="$log_dir/kani.log"

# ---- memory ceiling and parallelism (research R15) ------------------------------------
to_kib() {   # to_kib <value> <variable name, for the error message>
    case "$1" in
        *[Gg]) echo $(( ${1%[Gg]} * 1024 * 1024 )) ;;
        *[Mm]) echo $(( ${1%[Mm]} * 1024 )) ;;
        *) echo "run-verify: $2 must end in G or M (got '$1')" >&2; exit 2 ;;
    esac
}
ceiling_kib=$(to_kib "${VERIFY_MEM_PER_HARNESS:-4G}" VERIFY_MEM_PER_HARNESS)
time_limit=(-Z unstable-options --harness-timeout "${VERIFY_HARNESS_TIMEOUT:-12m}")
playback_kib=$(( 3 * ceiling_kib ))
[ -n "${VERIFY_PLAYBACK_MEM:-}" ] && playback_kib=$(to_kib "$VERIFY_PLAYBACK_MEM" VERIFY_PLAYBACK_MEM)

is_linux=false
[ "$(uname -s)" = Linux ] && is_linux=true
avail_kib=0
if $is_linux; then
    avail_kib=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo)
fi

if [ -n "${VERIFY_JOBS:-}" ]; then
    jobs=$VERIFY_JOBS
else
    jobs=$(nproc 2>/dev/null || echo 1)
    if $is_linux; then
        # MemAvailable / (7/8 of the ceiling): 3.5 GiB per job at the default 4G ceiling.
        # Most harnesses stay well under the ceiling; the cgroup backstop covers the rest.
        by_mem=$(( avail_kib * 8 / (7 * ceiling_kib) ))
        [ "$by_mem" -lt "$jobs" ] && jobs=$by_mem
    fi
    [ "$jobs" -lt 1 ] && jobs=1
fi

# The caller's build options (for example `--features slow`), without the harness
# selection, for the single-harness re-runs below.
build_args=()
skip_next=false
for a in "$@"; do
    if $skip_next; then skip_next=false; continue; fi
    case "$a" in
        --harness) skip_next=true ;;
        --harness=*|--exact) ;;
        *) build_args+=("$a") ;;
    esac
done

# The playback re-run (one process, after the main run) may use more memory: Kani's driver
# holds the whole counterexample trace, and on the neighbour harnesses it ran out of memory
# at the 4 GiB per-harness ceiling. Keep it 2 GiB below what is available.
if $is_linux; then
    play_cap=$(( avail_kib - 2 * 1024 * 1024 ))
    [ "$play_cap" -lt "$playback_kib" ] && playback_kib=$play_cap
    [ "$playback_kib" -lt "$ceiling_kib" ] && playback_kib=$ceiling_kib
fi

# ---- step 1: build once, without a memory limit ---------------------------------------
echo "run-verify: building harnesses (cargo kani --only-codegen)"
if ! cargo kani --only-codegen "$@" > "$log_dir/codegen.log" 2>&1; then
    tail -n 40 "$log_dir/codegen.log"
    echo "run-verify: codegen failed (full log: verify/$log_dir/codegen.log)" >&2
    exit 1
fi

# ---- the memory guard ------------------------------------------------------------------
backstop=()
unit=""
if $is_linux && systemd-run --user --scope --quiet true > /dev/null 2>&1; then
    unit="s2rst-verify-$$-$RANDOM"
    # The per-harness ceilings, plus 2 GiB for Kani's driver and cargo (001's T003 design).
    # Without the headroom, one harness near its ceiling (the CellUnion 25-byte decode proof
    # uses 7.8 of 8 GiB) tripped the backstop in a one-job run.
    backstop_kib=$(( jobs * ceiling_kib + 2 * 1024 * 1024 ))
    cap_kib=$(( avail_kib - 1024 * 1024 ))
    [ "$cap_kib" -gt 0 ] && [ "$cap_kib" -lt "$backstop_kib" ] && backstop_kib=$cap_kib
    backstop=(systemd-run --user --scope --quiet "--unit=$unit"
              -p "MemoryMax=${backstop_kib}K" -p MemorySwapMax=0)
elif ! $is_linux; then
    echo "run-verify: WARNING: not Linux, no memory guard is active; lower VERIFY_JOBS" >&2
fi

# Runs "$@" under `ulimit -v` (and the cgroup backstop where available). The ceiling is
# $guard_kib when set (the playback re-run, which runs alone), else the per-harness ceiling.
guarded() {
    local limit=${guard_kib:-$ceiling_kib} u=()
    if [ -n "$unit" ]; then
        u=("${backstop[@]}")
        u[4]="--unit=$unit-$1"
        # One process under a raised ceiling needs a backstop at least as large.
        [ "$limit" -gt "$backstop_kib" ] && u[6]="MemoryMax=${limit}K"
    fi
    shift
    "${u[@]}" bash -c 'ulimit -v "$0"; exec "$@"' "$limit" "$@"
}

backstop_hit() {   # $1 = unit suffix
    [ -n "$unit" ] || return 1
    local r
    r=$(systemctl --user show -p Result --value "$unit-$1.scope" 2>/dev/null || true)
    systemctl --user reset-failed "$unit-$1.scope" > /dev/null 2>&1 || true
    [ "$r" = oom-kill ]
}

# ---- the parser --------------------------------------------------------------------------
# Reads Kani's terse output on stdin while the run goes on. Prints a progress line for each
# harness as its result arrives (also to progress.log, for `tail -f` from another terminal),
# and writes results.tsv at the end. With several jobs, each result block follows a
# "Thread N:" line; with one job there are none, and the block follows the harness's
# "Checking harness" line (a TIMEOUT block has no "VERIFICATION RESULT:" line).
IFS= read -r -d '' parse_py <<'PY' || true
import re, sys, time
out, progress_path, known_path, timeouts_path = sys.argv[1:5]
def entries(path):
    d = {}
    for line in open(path):
        f = line.split()
        if f and not f[0].startswith('#'):
            d[f[0]] = f[1] if len(f) > 1 else ''
    return d
known, known_timeouts = entries(known_path), entries(timeouts_path)
check = re.compile(r'^(?:Thread (\d+): )?Checking harness (\S+)\.\.\.\s*$')
thread_hdr = re.compile(r'^Thread (\d+):\s*$')
by_thread, cur, last, res, started, shown = {}, None, None, {}, [], set()
n_fail = 0
progress = open(progress_path, 'w')

def new():
    return {'covers': None, 'failed': None, 'status': None, 'time': None}

def status(r):
    st = r['status'] or 'ERROR'
    if st == 'FAILED' and not r['failed']:
        st = 'ERROR'               # failed without a failed check: OOM or tool error
    return st

def show(h):
    # The progress line, once per harness. Known defects and covers as in step 3; an ERROR
    # is re-run there to tell OOM apart.
    global n_fail
    if h is None or h in shown or h not in res:
        return
    shown.add(h)
    r = res[h]
    st = status(r)
    if h in known:
        stale = st == 'SUCCESSFUL'
        st = ('SUCCESSFUL, STALE KNOWN-DEFECT ' if stale else 'KNOWN-DEFECT ') + known[h]
    elif h in known_timeouts and st in ('TIMEOUT', 'SUCCESSFUL'):
        st = 'KNOWN-TIMEOUT' if st == 'TIMEOUT' else 'SUCCESSFUL, STALE KNOWN-TIMEOUT'
    c = r['covers']
    if c is not None and c[0] < c[1]:
        st += ', UNSATISFIED COVERS'
    if st != 'SUCCESSFUL' and not st.startswith(('KNOWN-DEFECT', 'KNOWN-TIMEOUT')):
        n_fail += 1
    cov = '-' if c is None else '%d/%d' % c
    t = '-' if r['time'] is None else '%.1f' % r['time']
    line = '[%s] %-44s %-24s covers %-6s %ss  (%d done, %d failing)' % (
        time.strftime('%H:%M:%S'), h, st, cov, t, len(shown), n_fail)
    print(line, flush=True)
    progress.write(line + '\n')
    progress.flush()

for line in sys.stdin:
    line = line.rstrip('\n')
    m = check.match(line)
    if m:
        started.append(m.group(2))
        last = m.group(2)
        if m.group(1) is None:         # one job: this harness's block comes next
            show(cur)
            cur = m.group(2)
        else:
            by_thread[m.group(1)] = m.group(2)
        continue
    m = thread_hdr.match(line)
    if m:
        show(cur)
        cur = by_thread.get(m.group(1))
        continue
    if line.startswith('VERIFICATION RESULT:') and cur is None:
        cur = last
    if cur is None:
        continue
    r = res.setdefault(cur, new())
    if m := re.match(r'^ \*\* (\d+) of (\d+) cover properties satisfied', line):
        r['covers'] = (int(m.group(1)), int(m.group(2)))
    elif m := re.match(r'^ \*\* (\d+) of (\d+) failed', line):
        r['failed'] = int(m.group(1))
    elif m := re.match(r'^VERIFICATION:- (SUCCESSFUL|FAILED)', line):
        r['status'] = m.group(1)
    elif line.startswith('CBMC timed out'):
        r['status'] = 'TIMEOUT'
        show(cur)
        cur = None
    elif m := re.match(r'^Verification Time: ([0-9.]+)s', line):
        r['time'] = float(m.group(1))
        show(cur)
        cur = None
for h in started:                      # started without a result: the run was cut short
    res.setdefault(h, new())
for h in sorted(res):
    show(h)
with open(out, 'w') as f:
    for h, r in sorted(res.items()):
        c = r['covers']
        cov = '-' if c is None else '%d/%d' % c
        t = '-' if r['time'] is None else '%.1f' % r['time']
        f.write('\t'.join((h, status(r), cov, t)) + '\n')
PY

# ---- step 2: the guarded run, parsed as it goes -----------------------------------------
results="$log_dir/results.tsv"
progress="$log_dir/progress.log"
echo "run-verify: jobs=$jobs, ceiling $(( ceiling_kib / 1024 )) MiB and" \
     "${VERIFY_HARNESS_TIMEOUT:-12m} per harness, playback $(( playback_kib / 1024 )) MiB," \
     "cgroup backstop $([ -n "$unit" ] && echo on || echo off)"
echo "run-verify: one line per harness as it finishes (also verify/$progress)"
set +e
guarded main cargo kani -j "$jobs" --output-format terse "${time_limit[@]}" "$@" \
    < /dev/null 2>&1 | tee "$log" \
    | python3 -c "$parse_py" "$results" "$progress" known-defects.txt known-timeouts.txt
rcs=("${PIPESTATUS[@]}")
set -e
kani_rc=${rcs[0]}
if backstop_hit main; then
    echo "run-verify: memory backstop hit (cgroup oom-kill); log: verify/$log" >&2
    exit 1
fi
if [ "${rcs[1]}" -ne 0 ] || [ "${rcs[2]}" -ne 0 ]; then
    echo "run-verify: logging or parsing the run failed (tee ${rcs[1]}, parser ${rcs[2]})" >&2
    exit 1
fi
echo "run-verify: main run done; the report follows (sorted, with OOM and counterexamples)"

declare -A known=()
while read -r h d _; do
    [ -z "${h:-}" ] && continue
    case "$h" in \#*) continue ;; esac
    known[$h]=$d
done < known-defects.txt
declare -A known_timeout=()
while read -r h d _; do
    [ -z "${h:-}" ] && continue
    case "$h" in \#*) continue ;; esac
    known_timeout[$h]=${d:-listed}
done < known-timeouts.txt

# ---- step 3: classify, print counterexamples, report ------------------------------------
oom_pattern='out of memory|CBMC failed with status 6|std::bad_alloc|memory allocation of [0-9]+ bytes failed'
fail=0
n_ok=0 n_fail=0 n_known=0 n_timeout=0
declare -A fam_total=() fam_ok=()
report="$log_dir/report.txt"
: > "$report"
while IFS=$'\t' read -r h st cov t; do
    if [ "$st" = ERROR ]; then          # re-run alone in regular output to tell OOM apart
        set +e
        guarded "rerun$RANDOM" cargo kani "${build_args[@]}" --exact --harness "$h" "${time_limit[@]}" \
            < /dev/null > "$log_dir/rerun.log" 2>&1
        set -e
        # CBMC's own messages: the solver's "ran out of memory", the pre-solver "Out of
        # memory" with exit status 6 (Kani prints "CBMC failed with status 6"), and
        # allocation-failure aborts. Any of them means the harness needs a split or a smaller
        # bound, not a tool fix (research R18).
        if grep -qiE "$oom_pattern" "$log_dir/rerun.log"; then st=OOM; fi
    fi
    if [ -n "${known[$h]:-}" ]; then
        if [ "$st" = SUCCESSFUL ]; then
            echo "run-verify: stale known-defect entry: $h ${known[$h]} now succeeds" >&2
            fail=1
        else
            st="KNOWN-DEFECT ${known[$h]}"
        fi
    elif [ -n "${known_timeout[$h]:-}" ]; then
        if [ "$st" = SUCCESSFUL ]; then
            echo "run-verify: stale known-timeout entry: $h ${known_timeout[$h]} now succeeds;" \
                 "record the proof and remove the line" >&2
            fail=1
        elif [ "$st" = TIMEOUT ]; then
            st=KNOWN-TIMEOUT
        fi
    fi
    if [ "$cov" != - ] && [ "${cov%/*}" -lt "${cov#*/}" ]; then
        st="$st, UNSATISFIED COVERS"
    fi
    case "$st" in
        SUCCESSFUL) n_ok=$((n_ok + 1)) ;;
        KNOWN-DEFECT*) n_known=$((n_known + 1)) ;;
        KNOWN-TIMEOUT) n_timeout=$((n_timeout + 1)) ;;
        *) n_fail=$((n_fail + 1)); fail=1 ;;
    esac
    printf '%-44s %-24s covers %-6s %ss\n' "$h" "$st" "$cov" "$t" | tee -a "$report"
    if [[ "$h" == *__* ]]; then
        fam="${h%__*}__*"
        fam_total[$fam]=$(( ${fam_total[$fam]:-0} + 1 ))
        [ "$st" = SUCCESSFUL ] && fam_ok[$fam]=$(( ${fam_ok[$fam]:-0} + 1 ))
    fi
    if [[ "$st" == FAILED* ]] && [ "${VERIFY_PLAYBACK:-1}" != 0 ]; then
        echo "---- counterexample for $h (concrete playback) ----"
        set +e
        guard_kib=$playback_kib guarded "play$RANDOM" cargo kani -Z concrete-playback \
            --concrete-playback=print "${build_args[@]}" --exact --harness "$h" \
            --output-format terse "${time_limit[@]}" < /dev/null > "$log_dir/playback.log" 2>&1
        set -e
        if grep -q '^Concrete playback unit test for' "$log_dir/playback.log"; then
            awk '/^Concrete playback unit test for/ {p=1} p {print} p && /^```$/ && n++ {exit}' \
                "$log_dir/playback.log"
            grep -E '^Failed Checks:|^ File:' "$log_dir/playback.log" | head -n 6
        else
            echo "run-verify: playback produced no test for $h (ceiling $(( playback_kib / 1024 )) MiB;" \
                "log: verify/$log_dir/playback.log). Last lines:"
            grep -vE '^(Unwinding|Not unwinding|Check [0-9]+:|\s+- )' "$log_dir/playback.log" | tail -n 4
            if grep -qiE 'out of memory|memory allocation of [0-9]+ bytes failed' "$log_dir/playback.log"; then
                echo "run-verify: playback ran out of memory; raise VERIFY_PLAYBACK_MEM"
            fi
        fi
        echo "---- end counterexample for $h ----"
    fi
    if [ "$st" = OOM ] || [ "$st" = ERROR ]; then
        echo "---- $st: $h (tool output) ----"
        grep -iE -m 2 "$oom_pattern" "$log_dir/rerun.log" || true
        grep -vE '^(Unwinding|Not unwinding|aborting|Check [0-9]+:|\s+- )' \
            "$log_dir/rerun.log" | grep -E 'VERIFICATION|failed|error|Error' | tail -n 4
    fi
done < "$results"

for fam in $(printf '%s\n' "${!fam_total[@]}" | sort); do
    printf '%-44s %s/%s SUCCESSFUL\n' "$fam" "${fam_ok[$fam]:-0}" "${fam_total[$fam]}" \
        | tee -a "$report"
done

total=$(wc -l < "$results")
echo "run-verify: $total harnesses: $n_ok SUCCESSFUL, $n_known KNOWN-DEFECT, $n_timeout KNOWN-TIMEOUT," \
     "$n_fail failing" \
     "(kani exit $kani_rc; log verify/$log)"
if [ "$total" -eq 0 ]; then
    echo "run-verify: no harness ran" >&2
    grep -E "error|No proof harnesses" "$log" | head -n 5 >&2 || true
    [ "$kani_rc" -eq 0 ] || fail=1
fi
if [ "$kani_rc" -ne 0 ] && [ "$n_fail" -eq 0 ] && [ "$n_known" -eq 0 ] && [ "$n_timeout" -eq 0 ] \
    && [ "$total" -gt 0 ]; then
    echo "run-verify: kani exited $kani_rc although every harness succeeded" >&2
    fail=1
fi
exit "$fail"
