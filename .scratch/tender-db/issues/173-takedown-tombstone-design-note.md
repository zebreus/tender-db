# 173 — takedown/redaction tombstone design note

Status: CLOSED (2026-08-25, owner) — the tombstone design note is written as
ADR-0012 (`docs/adr/0012-takedown-tombstones-dormant-design.md`), a dormant
design deliberately not built: tombstones table as order-of-record, parsed-layer
delete + `tombstoned` ledger stamp, targeted re-fold via the issue-99 epoch,
append-only change log untouched (`removed` rows only — change rows carry no
content), one-time package repack with the D4-must-consult-tombstones coupling,
content-hash re-arrival guard, snapshot/backup checklist. Was: open — only the
tombstone design note remains; D4/D5 landed 2026-08-23
Role: run-driver

2026-08-23 (Lennart, direct): the lawyer confirmed all stored information is
public; data-protection law is not a concern for this system and is CLOSED as
a topic — the standing rule ("never re-introduce privacy-driven design")
stands, with this note as its dated basis. What remains of this issue is
unrelated to that: the architecture assumes sources never retract published
notices, and if a takedown obligation ever arrives (source-side redaction, a
court order), there is no mechanism to remove one notice from an append-only
versioned store + tar archive + forever change log. Cheap insurance: a
tombstone design note, not machinery.

2026-08-09 (orchestrator): gap 8b half answered by the DR study
(dr-premise-2026-08.md §6): D4 (package re-hash probes) and D5 (BT-198
reveal recheck) are NOT implemented and NOT scheduled — no job kind, no
timer; D5's script exists only as a research artifact on the VPS outside
the repo (a D7 fragility). D4 is also a DR input (re-fetchability drift is
unmeasured and the original hashes die with the DB). Both are small
scheduled-job features; fold into the next ops batch.

### D4 landed in code (2026-08-23, owner) — the immutability probe is a weekly job

`rehash-probe` job kind: pages DISTINCT packages from the fetch registry by a stored cursor
(`registry_page` + the `rehash-cursor` report row), re-downloads each with `refetch:true`, and
lets the existing fetch path's sha256 compare classify — unchanged / DRIFTED (versioned beside
the original, never overwritten) / GONE / error. Findings land in a stored `rehash-probe`
report and the job summary carries an alarm line. Scheduled weekly ×8 on the pre-dawn Sunday
tick behind the data-quality run — cycles today's registry in about a year, and the cadence is
the point: the original hashes this compares against die with the DB (dr-premise C7), so the
probe must accumulate coverage before it is ever needed. Deploys with the next batch; first
live run next Sunday (or manual: `admin.sh enqueue rehash-probe`).

### D5 landed in code (2026-08-23, owner) — the reveal recheck, natively

The lost `51_republication.py` is not coming back; the in-DB form replaces it. `reveal-recheck`
job kind: over `notice_withheld_fields`, count withheld/dated/due, then for the due set (capped
at 20k, cap reported honestly as `checked`) ask whether a LATER version of the same tender no
longer withholds the same BT-195 field — kept promise vs standing reveal debt — plus a due-by-
field breakdown. All drives indexed (`notice_sections_kind`, `tender_versions_notice`). Stored
as a `reveal-recheck` report; weekly on the Sunday tick behind rehash-probe. Fixture test:
one revealed, one debt, one not-yet-due.

Still open here: the tombstone design note.

### D4/D5 deployed and verified on prod (2026-08-24) — D5 hit a perf wall, fixed same hour

Deployed rev 76e33cc. Both probes run live:
- **D4 (rehash-probe)**: verified — re-downloaded 3 real packages, all hashes unchanged
  (`3 probed — 3 unchanged, 0 drifted, 0 gone`).
- **D5 (reveal-recheck)**: the FIRST deployed form aggregated over the
  `notice_withheld_fields` VIEW (per-section correlated subqueries) and pinned a reader
  **>12 min uncancellably** on prod — my regression, caught within the hour. Rewritten
  onto base tables + index seeks, reveal pass hard-bounded to a 2000-row sample; now
  completes in **<1s**. First real numbers: **277,171 withheld fields, 42,932 dated,
  3,729 due; of 2000 sampled due, 182 revealed at head, 1,818 still withheld.**

Caveat for a future refinement (not blocking): "still withheld" currently includes due
fields on tenders that have NO later version at all — those cannot reveal by
construction, so the raw 91% overstates genuine reveal-DEBT. A cleaner metric would
split "no later notice exists" from "later notice exists and still withholds." Filed as
a note here; the job is safe and the signal is directionally real.
