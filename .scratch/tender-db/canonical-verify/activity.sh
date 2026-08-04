#!/usr/bin/env bash
# SUSTAINED-ACTIVITY PRIMITIVE — one implementation, shared by every instrument
# that needs to answer "is something still working?".
#
# WHY IT EXISTS. In a few hours on 2026-08-04 the same defect class voided three
# independent instruments:
#   1. run-driver's restart watcher      (instantaneous D|R thread state)
#   2. this suite's Tier A completion signal
#   3. this suite's abort interlock AND its during-window sampling loop
# All three asked an INSTANT of a flickering state and took the answer as durable.
# All three failed PERMISSIVE — they reported "nothing running" and let work
# proceed. A per-instrument re-derivation is how you get a fourth copy and a
# fourth bug, so this is the single home for the answer.
#
# THE TWO TRAPS IT CLOSES, both measured rather than reasoned:
#
#   `systemctl is-active --quiet U` is FALSE while a --service-type=oneshot unit
#   runs. Such a unit sits in ActiveState=activating for the whole of its
#   ExecStart and becomes `active` only once it has EXITED. Used as a loop
#   condition it means the loop never runs; used as an interlock it can only
#   detect a probe that has already finished.
#
#   Bash `$12` is `$1` followed by a literal `2`, so parsing /proc/<pid>/stat with
#   `set -- $rest; t=$(( $12 + $13 ))` yields 0 for every thread and any check
#   built on it reports "idle" unconditionally. Use `${12}`, or awk (below).
#
# USE:  source activity.sh   then call the functions
#       ./activity.sh --self-test    proves each answer can be BOTH values
set -uo pipefail

ACTIVITY_TCK=$(getconf CLK_TCK)

# ---------------------------------------------------------------- unit liveness
# unit_active <unit> -> 0 while the unit is doing anything at all.
# Reads ActiveState directly. NOT `is-active`, per the trap above. Chosen over
# `list-units --state=a,b,c` because it is a single property read with no state
# list that can silently be incomplete.
unit_active() {
  # `--` matters: a unit name may begin with `-` (e.g. the root slice `-.slice`)
  # and would otherwise be parsed as an option. Caught by this file's own
  # self-test on its first run.
  case "$(systemctl show -p ActiveState --value -- "$1" 2>/dev/null)" in
    activating|active|deactivating|reloading) return 0 ;;
    *) return 1 ;;
  esac
}

# --------------------------------------------------------------- cpu, sustained
# stat_ticks <path-to-stat> -> utime+stime, robust to a comm containing spaces
# or parens (a real thread name can be `Thread-1 (spin`). awk also sidesteps the
# bash positional-digit trap entirely.
stat_ticks() {
  local line rest
  line=$(cat "$1" 2>/dev/null) || return 1
  rest=${line##*') '}
  awk -v r="$rest" 'BEGIN{n=split(r,f," "); if(n<13){print ""; exit} print f[12]+f[13]}'
}

# pid_busy_pct <pid> <interval_s> -> percent of ONE core over the interval.
# A DELTA over real time — the artifact — never an instantaneous state.
#
# FAILS LOUD: prints `ERR` and returns 2 if it could not measure. It must NEVER
# return 0 on failure — 0 means "idle", which every caller reads as "go", and that
# is the permissive shape all six of today's instrument bugs shared. "I could not
# measure" and "there is nothing running" are different answers and callers have
# to be able to tell them apart. (Spec point from run-driver; this function had
# exactly the defect it was written to prevent, on four paths.)
pid_busy_pct() {
  local pid="$1" iv="${2:-3}" a b
  a=$(stat_ticks "/proc/$pid/stat") || { echo ERR; return 2; }
  [ -n "$a" ] || { echo ERR; return 2; }
  sleep "$iv"
  b=$(stat_ticks "/proc/$pid/stat") || { echo ERR; return 2; }
  [ -n "$b" ] || { echo ERR; return 2; }
  echo $(( (b - a) * 100 / (iv * ACTIVITY_TCK) ))
}

# threads_busy <pid> <comm> <interval_s> [pct] -> how many threads named <comm>
# exceeded <pct> of one core. Counts DISTINCT TIDS: threads routinely share a
# name (slow-read-exec is four), and keying anything by name inflates by the
# number of concurrently-busy same-named threads.
# FAILS LOUD on the same principle: `ERR`/2 if the process vanished mid-measure,
# rather than reporting the 0 busy threads that a dead process trivially has.
threads_busy() {
  local pid="$1" want="$2" iv="${3:-3}" min="${4:-5}" t tid n v busy=0
  [ -d "/proc/$pid" ] || { echo ERR; return 2; }
  declare -A before
  for t in /proc/"$pid"/task/*; do
    tid=${t##*/}
    n=$(sed -n 's/^[0-9]* (\(.*\)) .*/\1/p' "$t/stat" 2>/dev/null)
    [ "$n" = "$want" ] || continue
    v=$(stat_ticks "$t/stat"); [ -n "$v" ] && before[$tid]=$v
  done
  sleep "$iv"
  [ -d "/proc/$pid" ] || { echo ERR; return 2; }
  for t in /proc/"$pid"/task/*; do
    tid=${t##*/}; [ -n "${before[$tid]:-}" ] || continue
    v=$(stat_ticks "$t/stat"); [ -n "$v" ] || continue
    [ $(( (v - ${before[$tid]}) * 100 / (iv * ACTIVITY_TCK) )) -gt "$min" ] && busy=$((busy+1))
  done
  echo "$busy"
}

# ------------------------------------------------------------------- self-test
# Every function must be shown to return BOTH answers. A predicate only ever
# observed saying "no" is indistinguishable from one hard-wired to say "no" —
# which is precisely how all three instruments above passed review.
_self_test() {
  local fail=0 pid pct busy
  echo "== activity.sh self-test: each predicate must answer BOTH ways =="

  # busy arm
  bash -c 'while :; do :; done' & pid=$!
  pct=$(pid_busy_pct "$pid" 2)
  if [ "$pct" -gt 50 ]; then echo "  PASS pid_busy_pct busy   = ${pct}%"
  else echo "  FAIL pid_busy_pct saw ${pct}% for a spinning process"; fail=1; fi
  kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null

  # idle arm
  sleep 30 & pid=$!
  pct=$(pid_busy_pct "$pid" 2)
  if [ "$pct" -lt 5 ]; then echo "  PASS pid_busy_pct idle   = ${pct}%"
  else echo "  FAIL pid_busy_pct saw ${pct}% for an idle process"; fail=1; fi
  kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null

  # threads_busy both arms, using same-named threads (the inflation case)
  if command -v python3 >/dev/null; then
    python3 -c '
import threading,time,ctypes,ctypes.util
libc=ctypes.CDLL(ctypes.util.find_library("c"))
def spin():
    libc.prctl(15,b"act-spin",0,0,0)
    t=time.time()
    while time.time()-t<8: pass
for _ in range(3): threading.Thread(target=spin,daemon=True).start()
time.sleep(10)' & pid=$!
    sleep 1
    busy=$(threads_busy "$pid" act-spin 2 1)
    if [ "$busy" -eq 3 ]; then echo "  PASS threads_busy busy   = $busy (3 same-named threads, counted by tid)"
    else echo "  FAIL threads_busy saw $busy, expected 3"; fail=1; fi
    kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
    sleep 20 & pid=$!
    busy=$(threads_busy "$pid" act-spin 2 1)
    if [ "$busy" -eq 0 ]; then echo "  PASS threads_busy idle   = 0"
    else echo "  FAIL threads_busy saw $busy on an idle process"; fail=1; fi
    kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
  else
    echo "  SKIP threads_busy (no python3 to build same-named threads)"; fail=1
  fi

  # ERR arms: "could not measure" must be distinguishable from "idle". A dead pid
  # is the cheapest way to produce an unmeasurable case.
  sleep 5 & pid=$!; kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
  pct=$(pid_busy_pct "$pid" 1); if [ "$pct" = ERR ]; then echo "  PASS pid_busy_pct ERR    (dead pid, not reported as 0/idle)"
  else echo "  FAIL pid_busy_pct returned '$pct' for a dead pid — permissive"; fail=1; fi
  busy=$(threads_busy "$pid" anything 1); if [ "$busy" = ERR ]; then echo "  PASS threads_busy ERR    (dead pid, not reported as 0 busy)"
  else echo "  FAIL threads_busy returned '$busy' for a dead pid — permissive"; fail=1; fi

  # unit_active both arms — only meaningful where systemd is present
  if command -v systemctl >/dev/null && systemctl show -p ActiveState -- '-.slice' >/dev/null 2>&1; then
    if unit_active "-.slice"; then echo "  PASS unit_active true    (-.slice is always active)"
    else echo "  FAIL unit_active false for -.slice"; fail=1; fi
    if unit_active "definitely-not-a-unit-$$.service"; then
      echo "  FAIL unit_active true for a nonexistent unit"; fail=1
    else echo "  PASS unit_active false   (nonexistent unit)"; fi
    # the oneshot trap itself, if we can run a transient unit
    # --no-block here for the SAME reason the probe needs it: without it
    # systemd-run waits for the oneshot to finish, the unit is already gone by the
    # time we look, and this arm reports the trap open when it is closed. This
    # self-test had that bug and failed on the box because of it, not because
    # unit_active was wrong.
    if systemd-run --quiet --no-block --unit="act-selftest-$$" --service-type=oneshot --collect \
         /bin/sh -c 'sleep 6' >/dev/null 2>&1; then
      sleep 1
      if unit_active "act-selftest-$$.service"; then
        echo "  PASS unit_active true DURING a oneshot (the trap is-active fails)"
      else echo "  FAIL unit_active false during a running oneshot — the trap is NOT closed"; fail=1; fi
      systemctl stop "act-selftest-$$.service" >/dev/null 2>&1
    else
      echo "  SKIP oneshot arm (cannot start a transient unit here)"
    fi
  else
    echo "  SKIP unit_active (no usable systemd here)"
  fi

  echo
  [ "$fail" -eq 0 ] && echo "== all predicates answered both ways ==" \
                    || { echo "== SELF-TEST FAILED =="; return 1; }
}

case "${1:-}" in --self-test) _self_test;; esac
