# 37 — Dashboard first snapshot after boot takes ~9 min under load

Status: needs-verification
Priority: UPGRADED to bug (2026-07-21 ~16:40, user-reported twice)

Two escalations beyond the original polish framing:
1. **The empty snapshot renders as literal zeros.** Lennart read
   "quarantine 0" on the live dashboard — for that metric, 0 is a
   strong TRUE claim ("the strictness guarantee holds perfectly"),
   which the boot transient asserts falsely. ADR-0008 promised the
   empty default "renders as no data yet"; the UI does not deliver
   that. Pre-first-fill, every data panel must show an explicit
   "measuring since boot…" state — never zeros.
2. ~~Fill time grows~~ CORRECTED (lead, 16:50): the second boot filled
   in ~7.5-10 min (run-driver watch, visible 16:44:46) — same as the
   first boot; the ">25 min" read was the lead's arithmetic error
   (measured from deploy START not service restart). Not wedged, not
   slower. The incremental-sections design is still the right fix —
   7-10 blind minutes per boot stands — but as the original polish
   scope, not an escalation. The quarantine_resolution LIKE-scan audit
   stays as a nice-to-have.

Observed after the b0a5cdb deploy (2026-07-21): the background refresher
(issue 20 part 3) computes the whole dashboard snapshot as one
measure() pass, so after a restart the data panels serve the empty
initial snapshot until the first full pass lands — ~8-9 min while the
re-walk churned IO. Requests are fast throughout (by design) but the
dashboard shows "never"/zeros, which reads as broken.

Fix directions (pick the minimal one that works):
- Fill sections independently: funnel/counts/lag are cheap (sub-second)
  and can land in the snapshot immediately; only the coverage GROUP BY
  is slow. Incremental section fills make the dashboard useful within
  seconds of boot.
- And/or persist the last snapshot (a small JSON blob in the DB, written
  by the refresher) and serve it stale-with-age on boot until the first
  fresh pass replaces it — the snapshot_age field already exists to be
  honest about it.

Acceptance: within ~10s of a restart under ingestion load, the
dashboard shows data (fresh cheap sections, or stale-labelled previous
snapshot) instead of zeros; no request-path scans (part-3 invariant
holds).
