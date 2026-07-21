# 37 — Dashboard first snapshot after boot takes ~9 min under load

Status: ready-for-agent
Priority: UPGRADED to bug (2026-07-21 ~16:40, user-reported twice)

Two escalations beyond the original polish framing:
1. **The empty snapshot renders as literal zeros.** Lennart read
   "quarantine 0" on the live dashboard — for that metric, 0 is a
   strong TRUE claim ("the strictness guarantee holds perfectly"),
   which the boot transient asserts falsely. ADR-0008 promised the
   empty default "renders as no data yet"; the UI does not deliver
   that. Pre-first-fill, every data panel must show an explicit
   "measuring since boot…" state — never zeros.
2. **Fill time grows with every metric added.** Second boot took >25
   min to fill (vs ~9 first boot): issue 40's ledger added two more
   1.2M-row quarantine scans (with LIKE) to the single monolithic
   measure() pass, competing with a CPU-saturated parser. The
   incremental-sections design is now necessary, not optional: cheap
   sections (funnel/counts/lag/system) land in seconds, each section
   updates independently, slow scans can't hold the rest hostage.
   Also audit quarantine_resolution's LIKE scans for an indexed
   formulation.

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
