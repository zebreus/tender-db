#!/usr/bin/env bash
# Per-thread CPU sampler — a server-side signal for how long work actually runs.
#
# WHY THIS EXISTS. A stopwatch on an HTTP response measures when the RESPONSE
# finished, not when the WORK did. `/v1/tenders?include_data=true` returned 200
# with 63 KB in seconds and then burned a Class B slot for over ten minutes
# (2026-08-04). Any client-side clock records that read as fast. This reads the
# server's own threads instead, so a read that outlives its client is visible.
#
# Built for issue 120/#31 (abandoned queries terminate but with UNBOUNDED
# duration — the #5 shed bounds concurrency, not duration) and for #30b, which
# needs exactly this to avoid recording ten-minute reads as fast.
#
#   ./thread_cpu.sh tender-db.service            # 1 sample over 5s
#   ./thread_cpu.sh tender-db.service 10 60      # 60 samples, 10s apart, + summary
#   ./thread_cpu.sh 12345 5 12                   # by PID
#
# COST. Reads /proc/<pid>/task/*/stat. Bounded metadata, no data pages, no page
# cache touched — safe by construction against the live box under the category
# boundary adopted 2026-08-04. Adds no query load of its own.
#
# OCCUPANCY, which is the number #31 wants. With samples > 1 it reports, per
# thread, how many samples it was busy — so a slot held for 26 minutes shows as
# sustained occupancy rather than being inferred from load average decaying.
set -u

TARGET="${1:?usage: $0 <unit|pid> [interval_s] [samples]}"
INTERVAL="${2:-5}"
SAMPLES="${3:-1}"
BUSY_PCT="${BUSY_PCT:-5}"   # a thread counts as busy above this % of one core
TCK=$(getconf CLK_TCK)

if [[ "$TARGET" =~ ^[0-9]+$ ]]; then PID="$TARGET"
else PID=$(systemctl show "$TARGET" -p MainPID --value 2>/dev/null); fi
[ -n "${PID:-}" ] && [ "$PID" != 0 ] && [ -d "/proc/$PID" ] \
  || { echo "no such process for target '$TARGET'" >&2; exit 2; }

# ticks <statfile> -> utime+stime, parsed robustly.
# /proc/<pid>/stat's comm field is parenthesised and MAY CONTAIN SPACES, which
# breaks the obvious `awk '{print $14+$15}'`. Everything after the LAST ')' is
# field 3 onward, so utime/stime are fields 12/13 of that remainder.
ticks() {
  local line rest
  line=$(cat "$1" 2>/dev/null) || return 1
  rest=${line##*') '}
  awk -v r="$rest" 'BEGIN{n=split(r,f," "); if(n<13){print ""; exit} print f[12]+f[13]}'
}
tname() { local line; line=$(cat "$1" 2>/dev/null) || return 1; line=${line#*(}; echo "${line%%)*}"; }

declare -A occupied total_pct
busy_seen=0   # counted separately: ${#assoc[@]} on an empty array trips `set -u`
echo "== per-thread CPU  pid=$PID  target=$TARGET  ${SAMPLES}x${INTERVAL}s  busy>${BUSY_PCT}% =="

for _ in $(seq 1 "$SAMPLES"); do
  declare -A t0 names
  for t in /proc/"$PID"/task/*; do
    tid=${t##*/}; v=$(ticks "$t/stat") || continue
    [ -n "$v" ] && { t0[$tid]=$v; names[$tid]=$(tname "$t/stat"); }
  done
  sleep "$INTERVAL"

  line=""; proc_total=0
  for t in /proc/"$PID"/task/*; do
    tid=${t##*/}; v=$(ticks "$t/stat") || continue
    [ -n "$v" ] && [ -n "${t0[$tid]:-}" ] || continue
    # NOTE the braces+$ are load-bearing. In arithmetic context an ASSOCIATIVE
    # array subscript is evaluated as an arithmetic expression, so `t0[tid]`
    # silently resolves to 0 and d becomes cumulative-since-process-start rather
    # than a delta. That renders as percentages that CLIMB every sample — which
    # looks exactly like a query getting slower, i.e. it corroborates the thing
    # you are usually investigating. Caught by a smoke test, not by reading it.
    d=$(( v - ${t0[$tid]} )); pct=$(( d * 100 / (INTERVAL * TCK) ))
    proc_total=$(( proc_total + pct ))
    if [ "$pct" -gt "$BUSY_PCT" ]; then
      n=${names[$tid]}
      line+=" ${n}=${pct}%"
      [ -z "${occupied[$n]:-}" ] && busy_seen=$(( busy_seen + 1 ))
      occupied[$n]=$(( ${occupied[$n]:-0} + 1 ))
      total_pct[$n]=$(( ${total_pct[$n]:-0} + pct ))
    fi
  done
  printf '%s total=%d%%%s\n' "$(date -u +%H:%M:%SZ)" "$proc_total" "${line:-  (no thread above threshold)}"
  unset t0 names
done

if [ "$SAMPLES" -gt 1 ]; then
  echo "-- occupancy over $SAMPLES samples (${INTERVAL}s each = $((SAMPLES*INTERVAL))s window)"
  if [ "$busy_seen" -eq 0 ]; then
    echo "   no thread was busy in any sample"
  else
    for n in "${!occupied[@]}"; do
      printf '   %-22s busy in %d/%d samples (~%ds), mean %d%% of one core while busy\n' \
        "$n" "${occupied[$n]}" "$SAMPLES" "$(( occupied[$n] * INTERVAL ))" \
        "$(( total_pct[$n] / occupied[$n] ))"
    done
  fi
  echo "   NOTE occupancy is a LOWER BOUND on duration: work already running when"
  echo "        sampling started, or still running when it stopped, is undercounted."
fi
