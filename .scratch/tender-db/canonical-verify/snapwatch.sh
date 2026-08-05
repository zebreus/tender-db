#!/usr/bin/env bash
# Durable snapshot-freshness watch for tender-db (issue 119's cadence half).
#
# Deliberately built in tender-db-diskwatch.sh's idiom (task #23, run-driver):
# read-only, journal-only, detection NOT notification, one greppable tag per
# line, and a distinct ERROR arm for "this watch did not run" so a broken watch
# never renders as a quiet pass.
#
# WHAT THIS WATCHES, and why it is not the gate. The snapshot is produced by the
# app's daily pipeline — `supervisor.rs:enqueue_daily()` ends with
# `Spec::Snapshot`, last so it captures the freshly folded layer. That is a real
# producer, but it is CONDITIONAL ON THE PIPELINE REACHING ITS END: a wedged or
# failed ingest silently produces no snapshot and says nothing. The ring has real
# holes from exactly this (Jul 30 and Jul 31 are missing).
#
# standing_gate.sh already refuses a stale snapshot at the point of USE. That is
# the important half, but it only fires when someone runs the gate, and it says
# "I will not verify this", not "the pipeline stopped". This watch is the other
# half: it notices the ring going stale on its own schedule, whether or not
# anything is verifying that day.
#
# THRESHOLD. Deliberately the SAME 30h as the gate's MAX_AGE_H, so there is one
# definition of "stale" rather than two that can drift apart. 30h = one full
# missed daily, with slack for a long projection (the snapshot is last in the
# pipeline, so a multi-hour fold legitimately pushes it later in the day). A
# tighter 26h would false-fire the first time a fold runs long.
#
# COST. `stat` and a directory listing — bounded metadata, no data pages, safe by
# construction against the live box (the category boundary adopted 2026-08-04).
set -u

SNAPSHOT_DIR="${SNAPSHOT_DIR:-/data/db/snapshots}"
MAX_AGE_H="${MAX_AGE_H:-30}"
MIN_RING="${MIN_RING:-2}"   # TENDER_SNAPSHOT_KEEP default; fewer means pruning or production is wrong

newest=$(ls -1 "$SNAPSHOT_DIR"/tender-db-*.db 2>/dev/null | sort | tail -1)

if [ -z "$newest" ]; then
  echo "TENDERDB_SNAPWATCH ERROR $SNAPSHOT_DIR — no snapshot found at all; the daily pipeline has never produced one here, or the path is wrong. This watch establishes nothing about freshness."
  exit 0
fi
if [ ! -r "$newest" ]; then
  echo "TENDERDB_SNAPWATCH ERROR $newest — exists but is not readable; this watch did NOT run."
  exit 0
fi

# RING DEPTH COUNTS COMPLETE SNAPSHOTS, NOT FILES — and the difference is not
# pedantic. `db.snapshot` writes straight to the final name, so a run that dies
# mid-copy (OOM, kill, power) leaves a PARTIAL file at a perfectly valid snapshot
# name, permanently (proj-fix, 2026-08-05). Counting files would score that corpse
# as ring depth, so a keep=2 ring holding one real snapshot and one corpse reports
# ring=2 and this watch says "ok" — an instrument actively reassuring about the DR
# posture at the moment it is half of what it claims. Exactly the shape this suite
# keeps finding, so the count is of what the ring can actually restore FROM.
#
# O(1) per file: the sqlite header carries page_size @16 and the page count @28, so
# a complete file is at least page_size * pages. Bounded metadata, no data pages.
ring=0; corpses=""; newest_complete=""
for _f in $(ls -1 "$SNAPSHOT_DIR"/tender-db-*.db 2>/dev/null | sort); do
  [ -r "$_f" ] || continue
  _ps=$(od -An -tu2 -j16 -N2 -v --endian=big "$_f" 2>/dev/null | tr -d ' ')
  _pg=$(od -An -tu4 -j28 -N4 -v --endian=big "$_f" 2>/dev/null | tr -d ' ')
  _sz=$(stat -c %s "$_f" 2>/dev/null)
  [ "$_ps" = 1 ] && _ps=65536
  if [ -z "$_ps" ] || [ -z "$_pg" ] || [ -z "$_sz" ] || [ "${_pg:-0}" -eq 0 ] 2>/dev/null; then
    corpses="$corpses ${_f##*/}(unreadable-header)"; continue
  fi
  if [ "$_sz" -lt $(( _ps * _pg )) ]; then
    corpses="$corpses ${_f##*/}($_sz/$(( _ps * _pg )))"
  else
    ring=$((ring+1)); newest_complete="$_f"
  fi
done
if [ -n "$corpses" ]; then
  echo "TENDERDB_SNAPWATCH WARNING $SNAPSHOT_DIR — INCOMPLETE snapshot file(s) present:$corpses. A partial file at a valid snapshot name is never cleaned up and 'newest' selects it forever, so the gate goes permanently blind on it. These are NOT counted in ring depth below."
fi

# FRESHNESS IS REPORTED FROM THE NEWEST *COMPLETE* SNAPSHOT, not the newest file.
# Reporting the file would be the reassurance failure in its worst form: a corpse
# written five minutes ago would make this print "ok age=0h" at the exact moment
# the ring's newest restorable copy is a day old and the gate is blind.
if [ -z "$newest_complete" ]; then
  echo "TENDERDB_SNAPWATCH ERROR $SNAPSHOT_DIR — snapshot files exist but NONE is complete; there is nothing to restore from and nothing to verify against. This watch establishes no freshness."
  exit 0
fi
if [ "$newest_complete" != "$newest" ]; then
  echo "TENDERDB_SNAPWATCH WARNING newest FILE ${newest##*/} is incomplete; freshness below is reported from the newest COMPLETE snapshot ${newest_complete##*/} instead. Anything resolving by glob will pick the incomplete one."
fi
newest="$newest_complete"

mtime=$(stat -c %Y "$newest" 2>/dev/null)
if [ -z "$mtime" ]; then
  echo "TENDERDB_SNAPWATCH ERROR $newest — stat returned nothing; this watch did NOT run."
  exit 0
fi
age_h=$(( ( $(date +%s) - mtime ) / 3600 ))
stamp=$(date -u -d "@$mtime" +%FT%TZ)

if [ "$age_h" -gt "$MAX_AGE_H" ]; then
  echo "TENDERDB_SNAPWATCH WARNING $newest age=${age_h}h (max ${MAX_AGE_H}h) mtime=$stamp ring=$ring — a full daily cycle produced no snapshot. The pipeline is the producer (enqueue_daily -> Spec::Snapshot), so this means the daily did not reach its end. Check /admin/jobs and the journal for a failed or wedged job; standing_gate.sh will refuse this input rather than verify it."
else
  echo "TENDERDB_SNAPWATCH ok $newest age=${age_h}h mtime=$stamp ring=$ring"
fi

# A shrinking ring is a separate fault from a stale one: it means pruning is
# wrong or production stopped a while ago, and it is invisible in the age of the
# newest file.
if [ "$ring" -lt "$MIN_RING" ]; then
  echo "TENDERDB_SNAPWATCH WARNING $SNAPSHOT_DIR ring=$ring below expected ${MIN_RING} (TENDER_SNAPSHOT_KEEP) — the ring is thinner than configured, so a bad snapshot has less behind it than the DR posture assumes."
fi
