#!/usr/bin/env bash
# ============================================================================
# daily_verify.sh — the cycle integration for the standing gate (task #28).
#
# Runs the tiers in COST ORDER against the snapshot the pipeline just wrote, and
# turns their verdicts into an ALERTING DECISION. The gate answers "what is true
# of this snapshot"; this answers "does anyone need to be told".
#
# Design in .scratch/tender-db/design/daily-verify-integration.md. Four decisions
# from it are encoded here and each is load-bearing:
#
# 1. COST ORDER (0, C, A, B). ~63 min of confined I/O daily. If the window closes
#    early — or the interlock defers, or the box is shut down mid-run — a
#    cost-ordered sequence has already delivered the cheap high-value checks. A
#    cost-descending one would have spent the window on B and delivered nothing.
#    Partial completion must degrade to LESS COVERAGE, never NO COVERAGE.
#
# 2. TRANSITION ALERTING, not state alerting. A check red yesterday and red today
#    is the standing condition of the layer: reported, never alerted. Only
#    green->red and red->green are events. This is what makes a known-red baseline
#    (issue 36's ~72k negative-money rows) need no suppression — and it is
#    strictly better than suppressing, because a suppression would also hide
#    35,001 becoming 50,000, which is the most valuable thing the check could say
#    after its first finding.
#
# 3. THE FIRST RUN IS AN INVENTORY, NOT AN INCIDENT — and the reader must meet
#    that framing BEFORE the alert, not in an issue found afterwards. Printed at
#    the top of a first run, where it cannot be missed.
#
# 4. NO CHUNKING of tier B. Ruled out positively, not left unproven: two runs of
#    identical duration under identical confinement gave 222 refaults and 0, and
#    across six confined runs the effect scales with neither tier weight nor
#    duration. Chunking changes both, so it cannot help — and shipping it would be
#    WORSE than omitting it, because an interlock that cannot do its job still
#    looks like a mitigation and would be read as one in a future incident review.
#
# The units themselves (paths, MemoryMax/IOWeight, StateDirectory) belong with
# whoever owns the box; this is the logic they invoke, so it can be read and
# self-tested without one. AS OF 2026-08-05 THEY DO NOT EXIST — not written, not
# merely uninstalled. Do not read this file's existence as implying them.
#
# ---------------------------------------------------------------------------
# TWO MEASURED TRAPS FOR WHOEVER WRITES THOSE UNITS. Both were found the
# expensive way; both are invisible from reading systemd's documentation; and
# both had, until this comment, lived only in inter-agent messages — which is how
# a lesson arrives too late to the one person it was for.
#
# TRAP 1 — `RuntimeMaxSec` IS INERT FOR `Type=oneshot`. The whole ExecStart runs
# with the unit in ActiveState=activating, and RuntimeMaxSec bounds the ACTIVE
# state, so it never applies. MEASURED: RuntimeMaxSec=5 let a 30s sleep run the
# full 30s; TimeoutStartSec=5 killed it at 5s. Use TimeoutStartSec (set both if
# you like, but only one enforces). I reported RuntimeMaxSec upward as "a hard
# bound enforced by systemd" before measuring it. It was decoration, and an
# assert that checked the SETTING'S PRESENCE confirmed it happily.
#
# TRAP 2 — ENV DOES NOT CROSS THE `systemd-run` BOUNDARY. GATE_LABEL, STATE_FILE,
# FAIL_ON_REPEAT and MAX_AGE_H are NOT inherited; they need explicit --setenv (or
# Environment= in the unit). A unit that omits them does not fail — it runs the
# right checks under the DEFAULT label against the DEFAULT state file, so all
# four tiers silently share one state and every repeat-detection is nonsense.
# Caught only because a run under a fresh label reported repeat=yes, which was
# impossible. This one fails permissive in the suite's own idiom: it reports
# confidently, and the number it reports is meaningless.
#
# Corollary for the unit files: `FAIL_ON_REPEAT=1` on all four is decision (2)'s
# other half, and it is inert unless each unit ALSO gets its own STATE_FILE.
# Setting the first without the second yields four units agreeing on one file.
# ---------------------------------------------------------------------------
#
#   daily_verify.sh --snapshot /data/db/snapshots/x.db
#   daily_verify.sh --self-test        # no snapshot, no box
# ============================================================================
set -euo pipefail

GATE="${GATE:-$(dirname "$0")/standing_gate.sh}"
STATE_DIR="${STATE_DIR:-/var/lib/tender-db}"
# Where the previous run's per-check verdicts live, so this run can tell a NEW
# violation from a continuing one. Distinct from the gate's own repeat-detection
# state, which answers a different question (did the INPUT change).
VERDICT_STATE="${VERDICT_STATE:-$STATE_DIR/daily_verify.verdicts}"
TIERS="${TIERS:-0 C A B}"

# --- alerting -------------------------------------------------------------
# A check's condition is one of: ok | red | blind. An ALERT is a transition between
# ok and red. Everything else is state, printed rather than raised.
#
# BLIND IS NOT A THIRD SHADE OF RED, AND IT IS NEVER SUPPRESSED. It means the gate
# could not run at all — no snapshot, unreadable, or (proj-fix's case) a snapshot
# that is INCOMPLETE. Transition alerting is correct for FINDINGS: a check red
# yesterday and red today is the layer's standing condition, and issue 36's ~72k
# rows should not page anyone daily. It is WRONG for an inability to verify.
#
# The compound failure this closes, which none of the three parts causes alone:
# `db.snapshot` writes straight to the final name, so a crash mid-copy leaves a
# partial file at a perfectly valid snapshot name, permanently. `newest` then
# selects that corpse forever. The gate correctly refuses it — red. Transition
# alerting sees red, red, red and goes quiet after the first. Net result: the gate
# verifies NOTHING, every day, in silence, while remaining installed and green in
# the unit list. Three reasonable designs composing into the exact "looks
# installed, does nothing" shape this suite exists to prevent.
#
# So: suppress standing FINDINGS, never suppress standing BLINDNESS. A finding may
# legitimately persist; not knowing must stay loud for exactly as long as it lasts.
classify() {
  local prev_file="$1" now_file="$2" alerts=0 id cond prev
  while IFS='|' read -r id cond; do
    [ -n "$id" ] || continue
    prev=$(grep "^$id|" "$prev_file" 2>/dev/null | head -1 | cut -d'|' -f2 || true)
    if [ "$cond" = "blind" ]; then
      printf 'ALERT    %-28s BLIND — the gate could not run; this verified NOTHING\n' "$id"
      alerts=$((alerts+1))
    elif [ -z "$prev" ]; then
      # First sighting. On a first run this is inventory; on any later run it is a
      # check that did not exist before, which is worth saying either way.
      printf 'NEW      %-28s %s\n' "$id" "$cond"
      [ "$cond" = "red" ] && alerts=$((alerts+1))
    elif [ "$prev" != "$cond" ]; then
      printf 'ALERT    %-28s %s -> %s\n' "$id" "$prev" "$cond"
      alerts=$((alerts+1))
    else
      printf 'state    %-28s %s (unchanged)\n' "$id" "$cond"
    fi
  done < "$now_file"
  return "$alerts"
}

self_test() {
  local d; d=$(mktemp -d); local rc=0
  printf 'a|ok\nb|red\nc|ok\n'  > "$d/prev"
  printf 'a|red\nb|red\nc|ok\nd|red\n' > "$d/now"
  local out; out=$(classify "$d/prev" "$d/now") || rc=$?
  echo "$out"
  # a went green->red: an alert. b stayed red: NOT an alert — the known-red
  # baseline that needs no suppression. c stayed ok. d is new and red.
  grep -q 'ALERT    a  *ok -> red'        <<<"$out" || { echo "FAIL: green->red must alert"; exit 1; }
  grep -q 'state    b  *red (unchanged)'  <<<"$out" || { echo "FAIL: red->red must NOT alert"; exit 1; }
  grep -q 'state    c  *ok (unchanged)'   <<<"$out" || { echo "FAIL: ok->ok must not alert"; exit 1; }
  grep -q 'NEW      d  *red'              <<<"$out" || { echo "FAIL: a new red check must be reported"; exit 1; }
  [ "$rc" -eq 2 ] || { echo "FAIL: expected 2 alerts (a, d), got $rc"; exit 1; }

  # Recovery is an event too — a check going green is worth knowing, and a
  # detector that only ever speaks about failure cannot tell you a fix worked.
  printf 'b|ok\n' > "$d/now2"; printf 'b|red\n' > "$d/prev2"
  out=$(classify "$d/prev2" "$d/now2") || rc=$?
  grep -q 'ALERT    b  *red -> ok' <<<"$out" || { echo "FAIL: red->green must alert"; exit 1; }

  # BLINDNESS ALERTS EVERY RUN, and the blind->blind arm is the one that matters:
  # it is exactly the case transition-suppression would swallow, leaving the gate
  # verifying nothing in silence. Tested from all three prior states so the
  # never-suppressed property is shown, not asserted.
  local rc2=0
  printf 'x|blind\ny|blind\nz|blind\n' > "$d/now3"
  printf 'x|ok\ny|red\nz|blind\n'      > "$d/prev3"
  out=$(classify "$d/prev3" "$d/now3") || rc2=$?
  grep -q 'ALERT    x  *BLIND' <<<"$out" || { echo "FAIL: ok->blind must alert"; exit 1; }
  grep -q 'ALERT    y  *BLIND' <<<"$out" || { echo "FAIL: red->blind must alert"; exit 1; }
  grep -q 'ALERT    z  *BLIND' <<<"$out" || { echo "FAIL: blind->blind must STILL alert — suppressing it is the silent-blindness bug"; exit 1; }
  [ "$rc2" -eq 3 ] || { echo "FAIL: expected 3 blind alerts, got $rc2"; exit 1; }
  # And blindness must not be reachable by accident: a red check stays a plain red.
  printf 'q|red\n' > "$d/now4"; printf 'q|red\n' > "$d/prev4"
  out=$(classify "$d/prev4" "$d/now4") || true
  grep -q 'state    q  *red (unchanged)' <<<"$out" || { echo "FAIL: red->red must remain suppressed"; exit 1; }

  # And the failure this whole shape exists to prevent: a permanently-red check
  # must never accumulate alerts, or the operator learns to ignore the stream.
  local n; n=0
  for _ in 1 2 3 4 5; do
    printf 'b|red\n' > "$d/p"; printf 'b|red\n' > "$d/n"
    classify "$d/p" "$d/n" >/dev/null || n=$((n+$?))
  done
  [ "$n" -eq 0 ] || { echo "FAIL: 5 runs over a standing red raised $n alerts, must be 0"; exit 1; }

  rm -rf "$d"
  echo "self-test: transition alerting ok (green->red, red->green alert; red->red never does)"
}

[ "${1:-}" = "--self-test" ] && { self_test; exit 0; }

SNAPSHOT=""
[ "${1:-}" = "--snapshot" ] && SNAPSHOT="${2:-}"
[ -n "$SNAPSHOT" ] || { echo "usage: $0 --snapshot <path> | --self-test" >&2; exit 2; }

first_run=0
[ -s "$VERDICT_STATE" ] || first_run=1
if [ "$first_run" = 1 ]; then
  # Decision 3: the framing arrives before the findings, not after.
  cat <<'FIRST'
== FIRST RUN — read this as an INVENTORY, not an incident ==
Nothing below is new breakage. This is the first time these checks have been run
on a schedule, so every violation they report is a pre-existing condition being
counted for the first time. Expect the negative-money surfaces to be red: ~72k
rows on two surfaces nothing had ever checked (issue 36). A red verdict today
means "the detector found what was already there", not "last night broke it".
FIRST
fi

now=$(mktemp); trap 'rm -f "$now"' EXIT
for tier in $TIERS; do
  echo "-- tier $tier"
  # Each tier gets its own GATE_LABEL, hence its own repeat-detection state: with
  # a shared file the tiers answer each other's "did the input change" question.
  # FAIL_ON_REPEAT=1 on all four — every tier now runs once per snapshot, so the
  # question is meaningful for each (it was not while tier 0 ran every 5 minutes).
  # Three outcomes, not two. The gate exits 2 when it CANNOT RUN (no snapshot,
  # unreadable, incomplete) and that must not be collapsed into "red" — see the
  # blindness note on classify(). Captured to a file rather than piped, because a
  # pipeline's exit status is the LAST command's: `"$GATE" | grep -q` reports on
  # grep and discards the gate's status entirely, which is how the distinction got
  # lost in the first place.
  gate_out=$(mktemp)
  TIER="$tier" GATE_LABEL="tier$tier" FAIL_ON_REPEAT=1 SNAPSHOT="$SNAPSHOT" \
    "$GATE" >"$gate_out" 2>&1
  gate_rc=$?
  cat "$gate_out" >&2
  if [ "$gate_rc" -eq 2 ]; then
    echo "tier$tier|blind" >> "$now"
  elif grep -q '^VERDICT ok' "$gate_out"; then
    echo "tier$tier|ok" >> "$now"
  else
    echo "tier$tier|red" >> "$now"
  fi
  rm -f "$gate_out"
done

echo
echo "== verdicts =="
alerts=0
classify "$VERDICT_STATE" "$now" || alerts=$?
mkdir -p "$(dirname "$VERDICT_STATE")" && cp "$now" "$VERDICT_STATE"

echo
if [ "$alerts" -eq 0 ]; then
  echo "no transitions — the layer's condition is unchanged since the last run"
else
  echo "$alerts transition(s) above"
fi
exit 0   # a red tier is a finding to report, not a reason to fail the unit
