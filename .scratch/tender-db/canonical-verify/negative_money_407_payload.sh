#!/usr/bin/env bash
# Payload for the confined 407-residue triage: negative_money_407_triage.sql against
# ONE pinned snapshot, inside ONE cgroup.
#
#   GATE=/opt/tender-db/canonical-verify/negative_money_407_payload.sh \
#   SNAPSHOT=<pinned> ./phase1_confined_probe.sh --release
#
# ARGUMENT CONTRACT — FAIL CLOSED, and this is the whole reason the file exists
# rather than passing sqlite3 to the probe directly. Anything the probe is pointed
# at via GATE= is invoked BOTH as `$GATE --self-test` (a PRECONDITION, which runs
# BEFORE systemd-run and therefore OUTSIDE the confinement) and as `$GATE` (the
# expensive confined payload). A wrapper that ignores an argument it does not
# understand and then does the expensive thing turned the self-test precondition
# into a 455 GB UNCONFINED scan of the production box for thirteen minutes. So:
# --self-test is cheap and local and touches no snapshot, and any other argument is
# refused rather than guessed at.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TRIAGE="${TRIAGE:-$HERE/negative_money_407_triage.sql}"
SQLITE="${SQLITE:-sqlite3}"

case "${1:-}" in
  --self-test)
    # Cheap, local, snapshot-free BY CONSTRUCTION. Checks only that the SQL exists
    # and is non-trivial; it must never open the database, because this runs
    # unconfined.
    [ -r "$TRIAGE" ] || { echo "self-test FAIL: triage sql unreadable: $TRIAGE" >&2; exit 1; }
    [ "$(wc -l < "$TRIAGE")" -gt 50 ] || { echo "self-test FAIL: triage sql implausibly short" >&2; exit 1; }
    echo "self-test ok: $TRIAGE present ($(wc -l < "$TRIAGE") lines), no snapshot touched"
    exit 0 ;;
  "") ;;
  *)  echo "refusing unknown argument '$1' — this script runs a full snapshot scan and will not guess" >&2; exit 2 ;;
esac

: "${SNAPSHOT:?SNAPSHOT must be pinned by the caller — this run does not resolve 'newest'}"
[ -r "$SNAPSHOT" ] || { echo "snapshot not readable: $SNAPSHOT" >&2; exit 2; }
[ -r "$TRIAGE" ]   || { echo "triage sql not readable: $TRIAGE" >&2; exit 2; }

# COMPLETENESS, same O(1) header check the gate now applies. A snapshot still being
# written is a TIMING fault; reading one would produce either a malformed-image
# error or, on a large file, an answer computed from a partial database. Either way
# the numbers would be about a file that is not yet the database.
_ps=$(od -An -tu2 -j16 -N2 -v --endian=big "$SNAPSHOT" | tr -d ' ')
_pg=$(od -An -tu4 -j28 -N4 -v --endian=big "$SNAPSHOT" | tr -d ' ')
_sz=$(stat -c %s "$SNAPSHOT")
[ "$_ps" = 1 ] && _ps=65536
if [ -z "$_ps" ] || [ -z "$_pg" ] || [ "${_pg:-0}" -eq 0 ] 2>/dev/null; then
  echo "cannot read sqlite header of $SNAPSHOT — refusing rather than guessing it is complete" >&2; exit 2
fi
if [ "$_sz" -lt $(( _ps * _pg )) ]; then
  echo "snapshot INCOMPLETE: $_sz B vs header's $(( _ps * _pg )) B — still being written" >&2; exit 2
fi

echo "=== 407-residue triage — pinned snapshot, confined ==="
echo "=== snapshot: $SNAPSHOT ($_sz bytes, header-complete)"
echo "=== triage:   $TRIAGE"
echo

err=$(mktemp)
"$SQLITE" -readonly "file:$SNAPSHOT?mode=ro" < "$TRIAGE" 2> "$err"
rc=$?
if [ -s "$err" ]; then
  echo
  echo "!!! TRIAGE ERRORS — the queries below did NOT run, and their results are ABSENT"
  echo "!!! above rather than zero. Do not read this triage as complete:"
  sed 's/^/!!!   /' "$err"
fi
rm -f "$err"
echo
echo "PAYLOAD_DONE triage=$rc"
exit "$rc"
