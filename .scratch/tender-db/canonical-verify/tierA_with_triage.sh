#!/usr/bin/env bash
# The payload for the confined Phase 1 re-run: Tier A, then the negative_amount
# triage, in ONE cgroup against ONE pinned snapshot.
#
# WHY A WRAPPER. phase1_confined_probe.sh launches a single command inside the
# confinement unit and measures the live service around it. The lead approved
# folding the triage into that authorized run precisely because it inherits the
# memory cap under test — so it must go INSIDE the same unit, not beside it. A
# second confined run would need its own window and its own during-window; a run
# outside the cgroup would be the unconfined read nobody authorized.
#
#   GATE=/opt/tender-db/canonical-verify/tierA_with_triage.sh \
#   SNAPSHOT=<pinned> ./phase1_confined_probe.sh --release
#
# EXIT CODE. The gate exits nonzero when the layer FAILS a check — which it
# currently does (negative_amount). That is a finding, not a runner error, and it
# must not stop the triage that exists to explain it. So the gate's status is
# captured and re-reported at the end rather than allowed to abort the script.
# Getting this wrong would mean the run that found the failure never explains it.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GATE_SH="${GATE_SH:-$HERE/standing_gate.sh}"
TRIAGE="${TRIAGE:-$HERE/negative_amount_triage.sql}"
SQLITE="${SQLITE:-sqlite3}"
TIER="${TIER:-A}"

: "${SNAPSHOT:?SNAPSHOT must be pinned by the caller — this run does not resolve 'newest'}"
[ -r "$SNAPSHOT" ] || { echo "snapshot not readable: $SNAPSHOT" >&2; exit 2; }
[ -x "$GATE_SH" ]  || { echo "gate not executable: $GATE_SH" >&2; exit 2; }
[ -r "$TRIAGE" ]   || { echo "triage sql not readable: $TRIAGE" >&2; exit 2; }

echo "=== payload: Tier $TIER gate + negative_amount triage, one cgroup, one snapshot ==="
echo "=== snapshot: $SNAPSHOT"

TIER="$TIER" SNAPSHOT="$SNAPSHOT" "$GATE_SH"
gate_status=$?
echo "=== gate exited $gate_status (nonzero = a check FAILED; that is a finding) ==="

echo
echo "=== negative_amount triage — same snapshot, same cgroup ==="
# stderr is captured and RE-EMITTED rather than left to scroll past. sqlite3
# continues after a failed statement and merely exits nonzero, so a triage whose
# query 5 errored still prints queries 6-11 and looks complete. An absent result
# in a wall of output is exactly the silent-partial-answer this suite keeps
# finding; naming the failed statement is the difference between a gap you can
# see and one you infer later.
triage_err=$(mktemp)
"$SQLITE" -readonly "file:$SNAPSHOT?mode=ro" < "$TRIAGE" 2> "$triage_err"
triage_status=$?
if [ -s "$triage_err" ]; then
  echo
  echo "!!! TRIAGE ERRORS — the queries below did NOT run, and their results are ABSENT"
  echo "!!! above rather than zero. Do not read this triage as complete:"
  sed 's/^/!!!   /' "$triage_err"
fi
rm -f "$triage_err"
echo "=== triage exited $triage_status ==="

echo
echo "PAYLOAD_DONE gate=$gate_status triage=$triage_status"
# Surface the gate's verdict as this script's, so the probe's own reporting is not
# told the run was clean when a check failed. The triage's status is separate and
# reported above; a triage failure must not masquerade as a layer failure.
exit "$gate_status"
