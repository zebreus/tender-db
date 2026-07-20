# 18 — published_at should be the publication date, not dispatch

Status: ready-for-agent
Blocked by: legacy-proj merge (project.rs is in flight)

Finding (verify-15): tender_versions.published_at is populated from the
notice's dispatch date (BT-05 / legacy DS), but the name — and the API field —
imply the OJ publication date. TED issue 136 dispatches 07-16, publishes
07-17; the acceptance harness had to switch to id-set-membership because of
the skew.

Fix: populate published_at from the actual publication date where derivable —
for TED the daily package's issue date (fetch period) is authoritative; DÖE
pubDay likewise; fall back to dispatch when no package date exists (direct
notice re-fetches). Keep dispatch as its own column (dispatched_at) since
ordering within a day uses it. API: tender versions + notices expose both,
documented. Re-projection (rebuild) refreshes historical rows — schedule one
after deploy.

Acceptance: verify-15's Search-API day-check can use published_at date
equality again on a sample day; API docs updated; rebuild executed in prod.
