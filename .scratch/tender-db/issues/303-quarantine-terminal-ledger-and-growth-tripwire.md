# 303 — quarantine terminal-state ledger + growth tripwire

Status: CLOSED 2026-08-27 — built as the curated terminal policy + tripwire
(model::dashboard::quarantine_terminal_policy/exceeded): the live terminal state
measured at exactly two classes (unrepresentable-value 300, ACCEPTED-INFLOW by
design; EOCD-corrupt zips Fixed(8)); every other reason defaults to Fixed(0),
so a NEW reason with rows trips the wire too. Surfaced as a panel verdict line
and the tender_db_quarantine_terminal_exceeded gauge (0 = holds), both computed
from the same cached by_reason counts so they cannot disagree. The narrative
ledger half already existed (ResolvedCategory, issue 40) — this adds the
machine-checked baseline the issue asked for. Deploys with the post-refold
batch.
Kind: observability (keep "quarantine is done" a monitored fact, not a memory)
Relates to: 268 (drained the last fed bucket; "~306 outstanding, all diagnosed"),
288 (make the resolution counters trustworthy FIRST), 289 (zero-stamp tripwire).

The corpus-wide outstanding quarantine is ~306 rows, all diagnosed and held by
design (garbage-class 298 from issue 268 + not-utf8 208-adjacent classes + small
residues). Two gaps: (1) that inventory lives across several issue files, not in
one machine-checked place — add a per-class terminal ledger (reason → count →
explanation → issue ref) served on the dashboard's quarantine panel; (2) nothing
alerts if outstanding GROWS above the documented terminal state (a new-era
regression would accrete silently between weekly DQ reads) — add a step-change
tripwire on the outstanding total per reason (the 265/266 gauge machinery has the
pattern). Depends on 288 landing first so the counters the ledger reads are
disjoint/trustworthy.
