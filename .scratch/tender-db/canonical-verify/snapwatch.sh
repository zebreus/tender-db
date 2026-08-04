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

mtime=$(stat -c %Y "$newest" 2>/dev/null)
if [ -z "$mtime" ]; then
  echo "TENDERDB_SNAPWATCH ERROR $newest — stat returned nothing; this watch did NOT run."
  exit 0
fi

age_h=$(( ( $(date +%s) - mtime ) / 3600 ))
ring=$(ls -1 "$SNAPSHOT_DIR"/tender-db-*.db 2>/dev/null | wc -l)
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
