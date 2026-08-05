#!/usr/bin/env bash
# PHASE 1 of task #28: does cgroup confinement actually protect the live service
# when the standing gate scans a 455 GB snapshot on the prod box?
#
# This script exists because the confinement is a CLAIM, not a fact. cgroup v2
# charges page cache to the cgroup that faults it in, so a `MemoryMax`-limited
# scan should reclaim ITS OWN cache rather than evict the live service's ~4.8 GB.
# Should. Issue 17 sat "resolved on construction" and unverified under load for a
# week; this is the same shape, so it gets measured before it gets believed.
#
# TWO-PHASE BY DESIGN. Default is STAGE: preconditions only, no gate, no load —
# safe to run any time. `--release` runs the confined measurement, and is the
# authorized action. Staging first means a late hold can physically land
# (prod-load-safety, the #16 deploy lesson).
#
#   ./phase1_confined_probe.sh              # stage: check preconditions, print the plan
#   ./phase1_confined_probe.sh --release    # the authorized run (Tier A only)
#
# RUNS ON THE PROD BOX. That is the whole point — the read cannot leave it (455 GB
# at ~100 kB/s is ~53 days). A prod-box read gates on HOST, not size, which is why
# it needs the lead's word and why it is confined and instrumented rather than bare.
#
# ONLY THE CONFINED ARM RUNS. The unconfined control is deliberately the harmful
# case; we judge against the pre-run baseline instead of running it.
#
# SELF-ABORTING. If live latency degrades past the threshold for 3 consecutive
# samples, the gate is killed. Worst case is a few seconds of degradation, not a
# run we notice afterwards in a graph.
set -uo pipefail

GATE="${GATE:-/opt/tender-db/canonical-verify/standing_gate.sh}"
TIER="${TIER:-A}"                 # Tier A only until its measurement gates B and C
MEM_MAX="${MEM_MAX:-512M}"        # the confinement under test
IO_WEIGHT="${IO_WEIGHT:-10}"      # default 100
CPU_WEIGHT="${CPU_WEIGHT:-10}"    # default 100
NICE="${NICE:-19}"
BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
HOT_READ="${HOT_READ:-/v1/tenders?limit=5}"
BASELINE_S="${BASELINE_S:-60}"    # measure normal operation before adding any load
SETTLE_S="${SETTLE_S:-60}"        # and after, to see recovery
INTERVAL_S="${INTERVAL_S:-2}"
ABORT_MS="${ABORT_MS:-250}"       # abort if the hot read exceeds this...
ABORT_MULT="${ABORT_MULT:-20}"    # ...or this multiple of baseline p95, whichever is larger
# REFAULT TRIPWIRE — the abort that watches the right variable.
#
# Tier B moved `refault` (239 pages) while latency did not move at all, so the
# latency abort could only ever have said "fine" about the one effect we now know
# a heavier tier can produce. Latency stays as the backstop; this is the primary
# guard for Tier C.
#
# Calibrated to SUSTAINED pressure, not Tier B's harmless burst. B's worst single
# sample was 172 pages over 2s (86/s) and it had at most 3 consecutive non-zero
# samples, only one of them above 50/s. Requiring 5 CONSECUTIVE samples above 50/s
# means ~500 pages over 10s sustained — an order above what B did, and a shape B
# never produced. A threshold that B would have tripped would abort on a known-
# harmless event; one that nothing could trip would be the permissive pattern on
# the new guard.
REFAULT_RATE_MAX="${REFAULT_RATE_MAX:-50}"   # pages/sec — ABSOLUTE FLOOR only
REFAULT_BASELINE_MULT="${REFAULT_BASELINE_MULT:-3}"  # abort above max(floor, MULT x measured baseline)
REFAULT_BASELINE_RATE="${REFAULT_BASELINE_RATE:-0}"  # filled in from the baseline window
REFAULT_SUSTAIN="${REFAULT_SUSTAIN:-5}"      # consecutive samples above it
UNIT="${UNIT:-tdb-standing-gate-probe}"
# HARD REMOTE BOUND on the confined work. Applied as TimeoutStartSec, not
# RuntimeMaxSec: the latter is silently ignored for Type=oneshot (measured).
#
# VERIFIED APPLIED, 2026-08-05: with MAX_RUN_S=20 the probe's own in-flight
# assertion read `TimeoutStartUSec = 20s` from the live unit. That was the claim
# outstanding — that the probe applies the bound, not merely that the setting
# works in isolation.
#
# AND THE REASON TWO EARLIER ATTEMPTS "FAILED": I checked the unit with
# `systemctl show` from OUTSIDE, after `--collect` had already reaped it — and
# systemd answers for an unknown unit with DEFAULTS (TimeoutStartUSec=1min 30s,
# MemoryMax=infinity, Type=), not with an error. I read those defaults as the
# probe's settings and twice concluded the fix had not applied. A query that
# answers plausibly for a thing that does not exist is the permissive-read shape:
# the honest answer is "no such unit", and systemd's is a full set of numbers.
# Read the unit while it is ALIVE, or read the probe's own assertion, which does. run-driver orphaned a snapshot read on
# prod for 90 minutes today because a LOCAL `timeout` around `ssh` killed the
# client, not the remote process — the guard bounded their VIEW of the work, not
# the work. TimeoutStartSec puts the bound inside systemd, which is the thing
# actually running it, so the gate dies on schedule whatever happens to my
# transport, my shell, or me. NOT RuntimeMaxSec, which this comment used to name:
# it is silently ignored for Type=oneshot (see the measurement below). Both are
# set on the unit; only one of them enforces, and naming the wrong one here is how
# a bound gets believed rather than checked. Tier A measured 28m50s; 1h is a generous backstop
# that still cannot become an unbounded read.
MAX_RUN_S="${MAX_RUN_S:-3600}"
# Resolve the input ONCE, here, and pass it to the gate PINNED. The lead requires
# SNAPSHOT= rather than "newest" for this run, and the reason is proj-fix's: age
# bounds how stale an input may be, it cannot establish WHICH run produced it. It
# also matters for the measurement itself — preconditions must report on exactly
# the file the confined run will read, or the two could diverge if the daily lands
# a new snapshot between the check and the launch.
SNAP="${SNAPSHOT:-$(ls -1 /data/db/snapshots/tender-db-*.db 2>/dev/null | sort | tail -1)}"
OUT="${OUT:-/tmp/phase1-$(date -u +%Y%m%dT%H%M%SZ)}"

ms() { awk -v s="$1" 'BEGIN{printf "%.1f", s*1000}'; }

# unit_running <unit> — true while the unit exists in ANY live state.
# NOT `systemctl is-active --quiet`: a --service-type=oneshot unit sits in
# ActiveState=activating for the WHOLE of its ExecStart and only becomes active
# once it has EXITED, so is-active is false exactly while the work runs. Using it
# as a loop condition meant the sampling loop never executed and the auto-abort —
# which lives inside that loop — never armed. Found by run-driver against a live
# run of this probe, 2026-08-04; the first of the day's permissive failures to
# disable a safety mechanism rather than merely misreport.
unit_running() {
  case "$(systemctl show "$1" -p ActiveState --value 2>/dev/null)" in
    activating|active|deactivating|reloading) return 0;; *) return 1;;
  esac
}

# THE EVICTION SIGNAL is the load-bearing measurement, not latency. At ~7% CPU
# duty the interference was never contention — it is PAGE-CACHE EVICTION: pulling
# 455 GB through the cache displaces the live service's working set, and MemoryMax
# is load-bearing precisely because it charges that cache to the gate's cgroup and
# reclaims it there instead of from the service. So we sample what MemoryMax
# CLAIMS TO PREVENT, per sample, from the live service's own cgroup:
#
#   file                     file-backed cache charged to the service. A DROP is
#                            its working set being taken.
#   workingset_refault_file  pages evicted and then READ BACK — the canonical
#                            eviction counter. A rising delta is the failure, even
#                            if latency p95 looks fine.
#   pgmajfault               major faults; the same story from the fault side.
#
# A latency p95 can look healthy on average while a brief eviction spike during
# the fill climb hurts a handful of requests — and that transient is exactly what
# an aggregate, or peak-equals-cap, cannot see. (team-lead, from proj-fix's
# I/O-bound analysis, 2026-08-04.)
# Configurable so the refault tripwire can be proven against a REAL cgroup whose
# refault counter is genuinely rising, on hardware that is not production. The
# tripwire is the only guard against a worse instance of whatever produced Tier B
# run 1's 222-refault burst, and it was proven only synthetically — a guard whose
# firing has never been observed against a real counter is the day's own pattern.
LIVE_CG="${LIVE_CG:-/sys/fs/cgroup/system.slice/tender-db.service}"

probe() { # -> epoch health_ms hot_ms live_bytes gate_bytes cached_kb file refault majflt
  local h r
  h=$(curl -o /dev/null -s -w '%{time_total}' --max-time 10 "$BASE_URL/health" 2>/dev/null || echo 9.999)
  r=$(curl -o /dev/null -s -w '%{time_total}' --max-time 30 "$BASE_URL$HOT_READ" 2>/dev/null || echo 29.999)
  local live gate cached lfile lref lmaj
  live=$(systemctl show tender-db.service -p MemoryCurrent --value 2>/dev/null)
  gate=$(cat "/sys/fs/cgroup/system.slice/$UNIT.service/memory.current" 2>/dev/null || echo 0)
  cached=$(awk '/^Cached:/{print $2}' /proc/meminfo)
  lfile=$(awk '/^file /{print $2}'                   "$LIVE_CG/memory.stat" 2>/dev/null)
  lref=$(awk '/^workingset_refault_file /{print $2}' "$LIVE_CG/memory.stat" 2>/dev/null)
  lmaj=$(awk '/^pgmajfault /{print $2}'              "$LIVE_CG/memory.stat" 2>/dev/null)
  echo "$(date +%s) $(ms "$h") $(ms "$r") ${live:-0} ${gate:-0} $cached ${lfile:-0} ${lref:-0} ${lmaj:-0}"
}

# delta <file> <col> -> last minus first. Cumulative counters must be differenced
# across the window; their absolute values say nothing.
delta() {
  [ -s "$1" ] || { echo "n/a"; return; }
  awk -v c="$2" 'NR==1{f=$c} {l=$c} END{if(NR==0){print "n/a";exit} print l-f}' "$1"
}

# refault_verdict <prev_total> <cur_total> <interval_s> <run_length>
# -> prints the new consecutive-breach count.
#
# PURE: no globals, no I/O. That is deliberate — it lets the self-test drive it
# with a synthetic series instead of trying to provoke real memory pressure on a
# live box, so both arms can be shown cheaply. A tripwire that has only ever been
# observed saying "fine" is the permissive pattern on the newest guard, and this
# one guards the heaviest tier.
# THE THRESHOLD IS RELATIVE TO THE MEASURED BASELINE, with an absolute floor —
# mirroring the latency guard beside it, which has always been max(250ms, 20x
# baseline p95). This one was a bare constant, and the inconsistency was not
# harmless: on 2026-08-05 it aborted a run after 10 s while the live service's
# OWN idle refault rate was 51-121/s, measured with nothing else running. The
# service was still refilling after a restart 45 min earlier, so the box sat
# permanently above a 50/s absolute bar and ANY payload would have aborted. The
# guard was not detecting my payload; it was detecting the weather.
#
# REFAULT_BASELINE_RATE is set from the baseline window before the payload starts.
# Zero/unset falls back to the absolute floor alone, which is the old behaviour and
# is correct when no baseline was measured.
refault_verdict() {
  local prev="$1" cur="$2" iv="$3" run="$4" rate limit
  [ "${iv:-0}" -gt 0 ] 2>/dev/null || { echo "$run"; return; }
  rate=$(( (cur - prev) / iv ))
  limit="$REFAULT_RATE_MAX"
  if [ "${REFAULT_BASELINE_RATE:-0}" -gt 0 ] 2>/dev/null; then
    local rel=$(( REFAULT_BASELINE_RATE * REFAULT_BASELINE_MULT ))
    [ "$rel" -gt "$limit" ] && limit="$rel"
  fi
  if [ "$rate" -gt "$limit" ]; then echo $(( run + 1 )); else echo 0; fi
}

# _test_refault_abort — both arms, against series taken from the real Tier B run
# and a synthetic sustained one. Run with --test-refault-abort.
_test_refault_abort() {
  local fail=0 run series label expect got prev
  echo "== refault tripwire: must fire on sustained pressure, NOT on Tier B's burst =="
  # Tier B, measured: per-sample refault deltas 5, 45, 172, then zeros, then 17.
  # Cumulative series at 2s intervals.
  for spec in "tierB-burst:0 5 50 222 222 222 239:0" \
              "sustained:0 200 400 600 800 1000 1200:1"; do
    label=${spec%%:*}; rest=${spec#*:}; series=${rest%:*}; expect=${rest##*:}
    run=0; prev=""; got=0
    for v in $series; do
      [ -n "$prev" ] && run=$(refault_verdict "$prev" "$v" 2 "$run")
      [ "$run" -ge "$REFAULT_SUSTAIN" ] && got=1
      prev=$v
    done
    if [ "$got" = "$expect" ]; then
      printf '  PASS %-14s fired=%s (expected %s)\n' "$label" "$got" "$expect"
    else
      printf '  FAIL %-14s fired=%s (expected %s)\n' "$label" "$got" "$expect"; fail=1
    fi
  done
  echo "  threshold (no baseline): >${REFAULT_RATE_MAX} pages/s for ${REFAULT_SUSTAIN} consecutive samples"

  # RELATIVE ARMS — the behaviour added 2026-08-05, driven by the real numbers that
  # exposed the need. On that day the live service idled at 51-121/s while still
  # refilling after a restart, so an absolute 50/s bar aborted a payload in 10 s for
  # doing nothing unusual. Both arms use a 57/s baseline (the measured figure) at
  # 2 s intervals: 114 pages per sample is exactly the idle weather, 400 is real.
  echo "== relative threshold: must ignore the box's own weather, still catch real pressure =="
  local saved="${REFAULT_BASELINE_RATE:-0}"
  REFAULT_BASELINE_RATE=57   # -> limit = max(50, 3x57) = 171/s
  for spec in "idle-weather-75/s:0 150 300 450 600 750 900:0" \
              "real-pressure-400/s:0 800 1600 2400 3200 4000 4800:1"; do
    label=${spec%%:*}; rest=${spec#*:}; series=${rest%:*}; expect=${rest##*:}
    run=0; prev=""; got=0
    for v in $series; do
      [ -n "$prev" ] && run=$(refault_verdict "$prev" "$v" 2 "$run")
      [ "$run" -ge "$REFAULT_SUSTAIN" ] && got=1
      prev=$v
    done
    if [ "$got" = "$expect" ]; then
      printf '  PASS %-20s fired=%s (expected %s)\n' "$label" "$got" "$expect"
    else
      printf '  FAIL %-20s fired=%s (expected %s)\n' "$label" "$got" "$expect"; fail=1
    fi
  done
  echo "  with baseline 57/s and mult ${REFAULT_BASELINE_MULT} -> abort above $((57*REFAULT_BASELINE_MULT))/s (arms above state what fired)"
  REFAULT_BASELINE_RATE="$saved"

  [ "$fail" -eq 0 ] && echo "== all four arms correct ==" || { echo "== TRIPWIRE SELF-TEST FAILED =="; return 1; }
}

pct() { # pct <file> <col> <percentile> — portable (no gawk asort)
  [ -s "$1" ] || { echo "n/a"; return; }
  sort -k"$2","$2" -g "$1" | awk -v c="$2" -v p="$3" \
    '{v[NR]=$c} END{if(NR==0){print "n/a";exit} i=int(p/100*NR); if(i<1)i=1; print v[i]}'
}

# AFTER A SERVICE RESTART, DO NOT MEASURE UNTIL THE CACHE STOPS FILLING — and do
# not wait a fixed number of minutes for it. There is no measured warm-up curve,
# and inventing an interval is the guessed-number habit this suite exists to
# remove. Watch the ARTIFACT: sample the live service's `file` (or MemoryCurrent)
# and start when it STOPS CLIMBING. A stabilised working set is observable;
# "twenty minutes should do it" is a belief.
#
# The target level is not fixed either — the live service sat at 2.4 GB during one
# run and 3.24 GB earlier the same day — which is the argument for watching the
# DERIVATIVE rather than waiting for a number. (run-driver, 2026-08-05. Recorded
# here because it had lived only in messages, and the next person to measure after
# a restart is exactly who needs it.)
#
# NOT YET A CHECK. Every measurement so far ran against a warm ~1.72 GB cache, so
# this has never gated anything. It should become a precondition; it is a comment
# today because adding an untested gate mid-deploy would be worse than the gap.
preconditions() {
  local ok=0
  echo "== preconditions =="
  [ -x "$GATE" ] && echo "  ok   gate present: $GATE" || { echo "  FAIL gate not executable at $GATE"; ok=1; }
  # The gate IS sqlite3; without it every check would ERROR and the run would
  # measure nothing while still reading the disk.
  command -v "${SQLITE:-sqlite3}" >/dev/null && echo "  ok   sqlite3: $(command -v "${SQLITE:-sqlite3}")" \
    || { echo "  FAIL no sqlite3 on PATH — the gate would error on every check"; ok=1; }
  # Self-test needs no snapshot and no box state; if the detectors are broken
  # here, the measured run would be measuring a broken gate.
  if "$GATE" --self-test >/dev/null 2>&1; then echo "  ok   gate self-test passes on this host"
  else echo "  FAIL gate self-test FAILS on this host — fix before measuring anything"; ok=1; fi
  [ "$(stat -fc %T /sys/fs/cgroup)" = cgroup2fs ] && echo "  ok   cgroup v2 (MemoryMax covers page cache)" \
    || { echo "  FAIL not cgroup v2 — MemoryMax would not bound page cache, the whole mechanism"; ok=1; }
  systemctl is-active --quiet tender-db.service && echo "  ok   live service active (there is something to protect)" \
    || { echo "  FAIL tender-db.service not active"; ok=1; }
  local snap="$SNAP" snap_age
  if [ -n "$snap" ] && [ -r "$snap" ]; then
    snap_age=$(( ($(date +%s) - $(stat -c %Y "$snap")) / 3600 ))
    echo "  ok   snapshot PINNED: $snap (${snap_age}h old, $(stat -c %s "$snap") bytes)"
    [ "$snap_age" -le 30 ] || { echo "  FAIL snapshot is ${snap_age}h old — the gate would refuse it anyway"; ok=1; }
  else
    echo "  FAIL no readable snapshot to pin"; ok=1
  fi
  unit_running "$UNIT.service" && { echo "  FAIL $UNIT.service already present (running or stale)"; ok=1; } \
    || echo "  ok   no probe unit present (none stale, none concurrent)"
  # A gate run must not race the daily pipeline. /admin/jobs needs an operator
  # secret we do not hold, and a check that always returns 403 is decorative — it
  # would "pass" identically whether a job were running or not. So use two signals
  # that actually work unauthenticated:
  #   1. the snapshot's own age. Spec::Snapshot is the LAST step of enqueue_daily,
  #      so a snapshot from today means the pipeline already finished.
  #   2. the live service's CPU. A project/process job is a sustained burner.
  local age_h; age_h=$(( ($(date +%s) - $(stat -c %Y "$snap")) / 3600 ))
  [ "$age_h" -lt 20 ] && echo "  ok   daily pipeline finished (its last step, the snapshot, is ${age_h}h old)" \
    || { echo "  WARN snapshot ${age_h}h old — the daily may not have completed; check before releasing"; }
  local cpu1 cpu2 busy
  # `awk '{print $14+$15}'` is the comm-with-spaces trap rejected in thread_cpu.sh:
  # /proc/<pid>/stat's comm is parenthesised and MAY contain spaces, which shifts
  # every positional field. It works here only because this target's comm is
  # `server` — right by luck, not construction, which is the same thing I flagged
  # in my own early one-liners. Everything after the LAST ') ' is field 3 onward,
  # so utime/stime are fields 12/13 of that remainder. (run-driver's review.)
  local _st
  _st=$(cat /proc/"$(systemctl show tender-db.service -p MainPID --value)"/stat 2>/dev/null)
  cpu1=$(awk -v r="${_st##*') '}" 'BEGIN{n=split(r,f," "); print (n<13)?0:f[12]+f[13]}')
  sleep 3
  _st=$(cat /proc/"$(systemctl show tender-db.service -p MainPID --value)"/stat 2>/dev/null)
  cpu2=$(awk -v r="${_st##*') '}" 'BEGIN{n=split(r,f," "); print (n<13)?0:f[12]+f[13]}')
  busy=$(( (cpu2 - cpu1) * 100 / (3 * $(getconf CLK_TCK)) ))
  [ "$busy" -lt 50 ] && echo "  ok   live service near-idle over 3s (${busy}% of one core) — no job mid-flight" \
    || { echo "  FAIL live service busy (${busy}% of one core) — a job is running; do not add a scan"; ok=1; }

  # CLASS B SLOTS — a stricter and more specific gate than aggregate CPU, and the
  # one that actually matters here. An abandoned Class B read can sit at ~24% of a
  # core (well under the threshold above) while continuously scanning and evicting
  # the live service's page cache. Page cache is THE VARIABLE THIS PROBE MEASURES,
  # so such a read is not background noise, it is an uncontrolled instance of the
  # thing under test — and worse, it can END mid-run, which would look exactly like
  # the confinement working. Aggregate CPU cannot see it; the thread can.
  #
  # This is the standard run-driver stood their watcher down to give me, so it is
  # enforced here rather than left to my judgement at the keyboard.
  local pid tid n v0 v1 slots=0
  pid=$(systemctl show tender-db.service -p MainPID --value)
  declare -A before
  for t in /proc/"$pid"/task/*; do
    tid=${t##*/}; n=$(sed -n 's/^[0-9]* (\(.*\)) .*/\1/p' "$t/stat" 2>/dev/null)
    [ "$n" = slow-read-exec ] || continue
    v0=$(awk '{r=$0; sub(/^.*\) /,"",r); split(r,f," "); print f[12]+f[13]}' "$t/stat" 2>/dev/null)
    [ -n "$v0" ] && before[$tid]=$v0
  done
  sleep 3
  for t in /proc/"$pid"/task/*; do
    tid=${t##*/}; [ -n "${before[$tid]:-}" ] || continue
    v1=$(awk '{r=$0; sub(/^.*\) /,"",r); split(r,f," "); print f[12]+f[13]}' "$t/stat" 2>/dev/null)
    [ -n "$v1" ] || continue
    [ $(( (v1 - ${before[$tid]}) * 100 / 300 )) -gt 5 ] && slots=$((slots+1))
  done
  [ "$slots" -eq 0 ] && echo "  ok   no Class B slot occupied — page cache is not being churned by a stray scan" \
    || { echo "  FAIL ${slots} Class B slot(s) still burning — an abandoned scan is churning the page cache this probe measures"; ok=1; }
  return $ok
}

# The tripwire self-test is pure and touches nothing — dispatch before the plan
# banner and before preconditions, so it can be run anywhere including off-box.
case "${1:-}" in
  --test-refault-abort) _test_refault_abort; exit $?;;
esac

echo "== #28 phase 1: confined-read impact probe =="
echo "-- plan: TIER=$TIER under MemoryMax=$MEM_MAX IOWeight=$IO_WEIGHT CPUWeight=$CPU_WEIGHT Nice=$NICE"
echo "--       baseline ${BASELINE_S}s -> gate -> settle ${SETTLE_S}s, sampling every ${INTERVAL_S}s"
echo "--       abort if hot read > max(${ABORT_MS}ms, ${ABORT_MULT}x baseline p95) for 3 consecutive samples"
echo "--       input:  $SNAP  (pinned, not newest-at-launch)"
echo "--       hard bound: TimeoutStartSec=${MAX_RUN_S}s enforced by systemd, not by my transport"
echo "--                   (RuntimeMaxSec is also set and is INERT for Type=oneshot — measured)"
echo "--       output: $OUT.{baseline,during,after}.tsv"
preconditions || { echo; echo "PRECONDITIONS FAILED — not staged."; exit 2; }

if [ "${1:-}" != "--release" ]; then
  echo
  echo "STAGED ONLY. Nothing was run and no load was added."
  echo "Re-run with --release to execute the authorized measurement."
  exit 0
fi

mkdir -p "$(dirname "$OUT")"
echo
echo "-- baseline (${BASELINE_S}s, live service undisturbed)"
: > "$OUT.baseline.tsv"
end=$(( $(date +%s) + BASELINE_S ))
while [ "$(date +%s)" -lt "$end" ]; do probe >> "$OUT.baseline.tsv"; sleep "$INTERVAL_S"; done
base_h=$(pct "$OUT.baseline.tsv" 2 95); base_r=$(pct "$OUT.baseline.tsv" 3 95)
echo "   baseline p95: health ${base_h}ms, hot read ${base_r}ms"

thresh=$(awk -v a="$ABORT_MS" -v b="$base_r" -v m="$ABORT_MULT" 'BEGIN{t=b*m; print (t>a)?t:a}')
echo "   abort threshold: ${thresh}ms sustained over 3 samples"

# THE REFAULT BASELINE, measured in the same window, so the eviction guard is
# judged against this box today rather than against a constant chosen on a
# different day. Column 8 is workingset_refault_file. Without this the guard is
# absolute and fires on the live service's own weather -- which is exactly what it
# did on 2026-08-05, aborting after 10 s while the idle rate was already 51-121/s.
_base_rf=$(delta "$OUT.baseline.tsv" 8)
case "$_base_rf" in ''|*[!0-9-]*) _base_rf=0;; esac
[ "$_base_rf" -lt 0 ] 2>/dev/null && _base_rf=0
REFAULT_BASELINE_RATE=$(( _base_rf / BASELINE_S ))
_rf_limit=$(( REFAULT_BASELINE_RATE * REFAULT_BASELINE_MULT ))
[ "$_rf_limit" -lt "$REFAULT_RATE_MAX" ] && _rf_limit="$REFAULT_RATE_MAX"
echo "   refault: baseline ${REFAULT_BASELINE_RATE}/s -> abort above ${_rf_limit}/s (max of ${REFAULT_RATE_MAX}/s floor, ${REFAULT_BASELINE_MULT}x baseline) sustained over 5"
if [ "$REFAULT_BASELINE_RATE" -gt "$REFAULT_RATE_MAX" ]; then
  echo "   NOTE: the live service is ALREADY refaulting above the absolute floor with"
  echo "         nothing running — its cache is still settling (a restart, or a job that"
  echo "         just churned it). The relative threshold is what makes this run"
  echo "         interpretable; a measurement taken now still says less than one taken"
  echo "         on a settled box. Prefer waiting for the rate to STOP FALLING."
fi

echo "-- gate (confined, Tier $TIER)"
# ENV DOES NOT CROSS systemd-run BY DEFAULT. Only what is --setenv'd reaches the
# unit. This forwarded TIER and SNAPSHOT only, so GATE_LABEL was accepted on the
# command line, appeared to be in use, and EVAPORATED at the boundary — the gate
# inside fell back to its tier$TIER default and wrote another tier's state file.
# Caught by a repeat=yes that should have been repeat=unknown on a fresh label.
# No error, no warning: the variable is simply not there, which is the silent-drop
# shape rather than a failure. Forwarded explicitly, and only when set, so an
# unset variable still takes the gate's own default rather than an empty string.
#
# --no-block is REQUIRED, not tidiness. Without it systemd-run waits for the
# START JOB to complete, and a Type=oneshot job completes only when ExecStart
# EXITS — so systemd-run blocks for the entire gate run, the script never reaches
# the sampling loop, during.tsv is never created and the abort never arms.
# Measured on the box: a oneshot sleeping 6s made systemd-run return after 6s;
# with --no-block it returned in 0s and the unit read `activating`.
systemd-run --no-block --unit="$UNIT" --service-type=oneshot --collect \
  -p MemoryMax="$MEM_MAX" -p MemorySwapMax=0 \
  -p IOWeight="$IO_WEIGHT" -p CPUWeight="$CPU_WEIGHT" -p Nice="$NICE" \
  -p TimeoutStartSec="$MAX_RUN_S" -p RuntimeMaxSec="$MAX_RUN_S" \
  --setenv=TIER="$TIER" --setenv=SNAPSHOT="$SNAP" \
  ${GATE_LABEL:+--setenv=GATE_LABEL="$GATE_LABEL"} \
  ${STATE_FILE:+--setenv=STATE_FILE="$STATE_FILE"} \
  ${FAIL_ON_REPEAT:+--setenv=FAIL_ON_REPEAT="$FAIL_ON_REPEAT"} \
  ${MAX_AGE_H:+--setenv=MAX_AGE_H="$MAX_AGE_H"} \
  ${SNAPSHOT_DIR:+--setenv=SNAPSHOT_DIR="$SNAPSHOT_DIR"} \
  /bin/bash "$GATE" >/dev/null 2>&1 || { echo "   FAILED to launch confined unit"; exit 2; }

# PROVE THE PAYLOAD IS ACTUALLY CONFINED, from the box, for THIS run — then fail
# closed. Today I ran a 455GB scan on prod completely outside the cgroup because a
# PRECONDITION invoked the payload, and preconditions run BEFORE systemd-run, i.e.
# outside the sandbox BY DEFINITION. The wrapper is fixed, but "I fixed the
# wrapper" is a claim about code; this is a claim about the running box.
#
# The load-bearing assertion is the LAST one: any sqlite3 reading the snapshot
# that is NOT in this unit's cgroup is an unconfined read, which is exactly what
# happened. It catches the accident even if every earlier check passes.
assert_confined() {
  local cg="/sys/fs/cgroup/system.slice/$UNIT.service" fail=0 procs
  for _ in $(seq 1 15); do
    [ -s "$cg/cgroup.procs" ] && break
    sleep 1
  done
  echo "-- confinement proof (this run, from the box)"

  procs=$(tr '\n' ' ' < "$cg/cgroup.procs" 2>/dev/null)
  if [ -n "${procs// /}" ]; then echo "   ok   cgroup has processes: $procs"
  else echo "   FAIL cgroup.procs is empty — the payload is not in the unit"; fail=1; fi

  local mm exp; mm=$(cat "$cg/memory.max" 2>/dev/null)
  exp=$(numfmt --from=iec "$MEM_MAX" 2>/dev/null)
  if [ -n "$mm" ] && [ "$mm" = "$exp" ]; then echo "   ok   memory.max = $mm ($MEM_MAX)"
  else echo "   FAIL memory.max is '$mm', expected '$exp' ($MEM_MAX) — cap not applied"; fail=1; fi

  local iow; iow=$(systemctl show -p IOWeight --value -- "$UNIT.service" 2>/dev/null)
  if [ "$iow" = "$IO_WEIGHT" ]; then echo "   ok   IOWeight = $iow"
  else echo "   FAIL IOWeight is '$iow', expected $IO_WEIGHT"; fail=1; fi

  # THE BOUND THAT ACTUALLY BINDS. RuntimeMaxSec is IGNORED for Type=oneshot —
  # systemd says so in the journal ("has no effect in combination with
  # Type=oneshot") and measurement agrees: RuntimeMaxSec=5 let a 30s sleep run the
  # full 30s, while TimeoutStartSec=5 killed it at 5. A oneshot is "starting" for
  # its whole life, so the start timeout is its runtime bound.
  #
  # This check previously read RuntimeMaxUSec and passed on it being set — verifying
  # the PRESENCE of a setting that has no EFFECT. A decorative check on a decorative
  # setting, inside the confinement proof, reported upward as a hard bound.
  local ts; ts=$(systemctl show -p TimeoutStartUSec --value -- "$UNIT.service" 2>/dev/null)
  if [ -n "$ts" ] && [ "$ts" != infinity ]; then echo "   ok   TimeoutStartUSec = $ts (the bound that binds a oneshot)"
  else echo "   FAIL TimeoutStartUSec is '$ts' — this run has NO effective time bound"; fail=1; fi

  # THE ONE THAT CATCHES TODAY'S ACCIDENT — and it now asks the KERNEL who holds
  # the file open, rather than who spelled a command line a particular way.
  #
  # This was `pgrep -f "sqlite3 -readonly file:$SNAP"`. run-driver's review: that is
  # select-by-MATCHING, the antipattern that failed four separate times today
  # (pkill -f self-match twice, is-active on an evaporating state, --state=active
  # missing activating). It sees a reader only if that reader spells the path the
  # way I spelled it — a symlink, a relative path, an extra flag between `sqlite3`
  # and `-readonly`, or a different tool entirely are all invisible. And since the
  # other four assertions check a unit the runaway was never IN, the single arm
  # that can catch a repeat was built on the suite's weakest primitive.
  #
  # lsof on the file is the artifact: an entry in the kernel's open-file table
  # cannot be spelled differently. It is also what actually established the box was
  # clear this morning, while pgrep was busy matching its own command line.
  local stray="" p holders rc=1
  if command -v lsof >/dev/null 2>&1; then
    holders=$(lsof -t -- "$SNAP" 2>/dev/null); rc=0
  elif [ -r /proc/self/fd ]; then
    local canon; canon=$(readlink -f "$SNAP")
    holders=$(for p in /proc/[0-9]*; do
                for f in "$p"/fd/*; do
                  [ "$(readlink -f "$f" 2>/dev/null)" = "$canon" ] && { echo "${p##*/}"; break; }
                done
              done 2>/dev/null); rc=0
  fi
  if [ "$rc" -ne 0 ]; then
    echo "   FAIL cannot enumerate holders of the snapshot (no lsof, no /proc) — confinement unprovable"; fail=1
  else
    for p in $holders; do
      grep -qx "$p" "$cg/cgroup.procs" 2>/dev/null || stray="$stray $p"
    done
    if [ -z "$stray" ]; then echo "   ok   every process holding this snapshot open is inside the cgroup"
    else echo "   FAIL holder(s) OUTSIDE the cgroup:$stray — unconfined read, aborting"; fail=1; fi
  fi

  if [ "$fail" -ne 0 ]; then
    echo "   CONFINEMENT NOT PROVEN — stopping the unit and refusing to measure."
    systemctl stop "$UNIT.service" 2>/dev/null
    echo "VERDICT confinement_unproven tier=$TIER snapshot=$SNAP"
    exit 2
  fi
  echo "   confinement proven for this run."
}
assert_confined
: > "$OUT.during.tsv"; breaches=0; refault_run=0; prev_refault=""; aborted=no; abort_why=""
gate_start=$(date +%s)
while unit_running "$UNIT.service"; do
  s=$(probe); echo "$s" >> "$OUT.during.tsv"
  hot=$(echo "$s" | awk '{print $3}')
  cur_refault=$(echo "$s" | awk '{print $8}')

  # PRIMARY GUARD: sustained refault. Tier B moved refault while latency did not
  # move at all, so latency alone would have said "fine" about the only effect a
  # heavier tier is known to produce.
  if [ -n "$prev_refault" ]; then
    refault_run=$(refault_verdict "$prev_refault" "$cur_refault" "$INTERVAL_S" "$refault_run")
    if [ "$refault_run" -gt 0 ]; then
      echo "   !! refault > ${REFAULT_RATE_MAX}/s (sustained $refault_run/${REFAULT_SUSTAIN})"
    fi
    if [ "$refault_run" -ge "$REFAULT_SUSTAIN" ]; then
      echo "   ABORTING — sustained page-cache eviction of the live service"
      systemctl stop "$UNIT.service" 2>/dev/null; aborted=yes; abort_why=refault; break
    fi
  fi
  prev_refault=$cur_refault

  # BACKSTOP: latency. Kept, but no longer the only guard.
  if awk -v h="$hot" -v t="$thresh" 'BEGIN{exit !(h>t)}'; then
    breaches=$((breaches+1))
    echo "   !! hot read ${hot}ms > ${thresh}ms (breach $breaches/3)"
    if [ "$breaches" -ge 3 ]; then
      echo "   ABORTING — confinement is not protecting the live service"
      systemctl stop "$UNIT.service" 2>/dev/null; aborted=yes; abort_why=latency; break
    fi
  else breaches=0; fi
  sleep "$INTERVAL_S"
done
gate_secs=$(( $(date +%s) - gate_start ))
peak_gate=$(awk '{if($5>m)m=$5} END{print m+0}' "$OUT.during.tsv")

echo "-- settle (${SETTLE_S}s)"
: > "$OUT.after.tsv"
end=$(( $(date +%s) + SETTLE_S ))
while [ "$(date +%s)" -lt "$end" ]; do probe >> "$OUT.after.tsv"; sleep "$INTERVAL_S"; done

echo
echo "== result =="
printf '  %-10s %-12s %-12s\n' window health_p95 hotread_p95
for w in baseline during after; do
  printf '  %-10s %-12s %-12s\n' "$w" "$(pct "$OUT.$w.tsv" 2 95)" "$(pct "$OUT.$w.tsv" 3 95)"
done
echo "  gate ran ${gate_secs}s, aborted=$aborted"
echo
echo "  -- EVICTION SIGNAL (what MemoryMax claims to prevent) --"
printf '  %-10s %-18s %-16s %-14s\n' window cache_file_delta refault_delta majflt_delta
for w in baseline during after; do
  printf '  %-10s %-18s %-16s %-14s\n' "$w" "$(delta "$OUT.$w.tsv" 7)" "$(delta "$OUT.$w.tsv" 8)" "$(delta "$OUT.$w.tsv" 9)"
done
echo "  cache_file_delta NEGATIVE during = the live service's cache was taken."
echo "  refault_delta RISING vs baseline = evicted pages being read back — the"
echo "  failure MemoryMax claims to prevent, and invisible in latency p95."
echo
echo "  gate cgroup peak: $peak_gate bytes (MemoryMax=$MEM_MAX) — if this pinned at the"
echo "    limit and live latency held, the confinement did the work it claims to."
echo "  live service MemoryCurrent, first->last during: $(head -1 "$OUT.during.tsv" | awk '{print $4}') -> $(tail -1 "$OUT.during.tsv" | awk '{print $4}')"
echo "    (a large DROP here is the live cache being evicted — the failure mode under test)"
echo "  raw: $OUT.{baseline,during,after}.tsv"
echo
echo "VERDICT_INPUTS aborted=$aborted${abort_why:+ abort_why=$abort_why} gate_secs=$gate_secs peak_gate_bytes=$peak_gate refault_guard=>${REFAULT_RATE_MAX}/s x${REFAULT_SUSTAIN}"
echo "Read the numbers before declaring the confinement sound. A short run that never"
echo "touched the disk proves nothing about a run that does."
