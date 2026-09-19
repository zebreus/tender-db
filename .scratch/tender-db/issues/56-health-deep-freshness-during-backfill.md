# 56 — /health/deep freshness false-alarm during a long backfill job

Status: DORMANT (verified 2026-08-16, owner sweep). The backfill is done and daily jobs complete in minutes, so the false-alarm window is gone in steady state; issue 226 meanwhile narrowed the freshness clock to the ingest kinds. The defect CLASS remains for any future multi-day single job (a full reprocess never records a completion until it ends): if issue 28's reprocess is scheduled, either accept the known 503 or teach freshness to count a RUNNING ingest job's progress as liveness. Re-open then.
Severity: LOW (spurious alert; service is fine)

Found 2026-07-23: during the multi-day historical backfill, /health/deep
returns 503 because `ingest_freshness` checks the last SUCCESSFUL job_log
completion, and job 1 (the marathon `process ted monthly (all)`) runs for
days without recording a completion — so freshness reads ~61h stale even
though ingestion is actively progressing. database.ok=true, disk.ok=true;
only the freshness check trips.

Consequence: the 4-hourly cloud pinger (issue 24) sees 503 and sends
Lennart a spurious "production down" alert while the backfill is healthy.

Fix: make ingest_freshness account for an ACTIVELY-RUNNING process/project
job — e.g. treat "a heavy job is in progress and making progress
(members_done advancing / notices climbing)" as fresh, or measure
freshness against the running job's last progress tick rather than only
the last completed run. Keep the real staleness signal for when NOTHING
is running/progressing (the case the check exists for).

Acceptance: /health/deep stays 200 while a backfill job is actively
progressing; still flips 503 when ingestion is genuinely stalled/stale.

## Verify

    curl -s --max-time 30 https://tenders.zebreus.click/health/deep | python3 -c 'import sys,json; d=json.load(sys.stdin)["checks"]["ingest_freshness"]; print(d["ok"], d["age_secs"], d["threshold_secs"])'

- **done** (dormant holds): `True <age> 93600` with no multi-day single job running (read 2026-09-19: `True 83464 93600`)
- **open** (reopen): `False …` WHILE a multi-day single job is mid-flight — the false alarm this issue describes, back for a job that never records a completion
