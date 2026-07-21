# 27 — Data-quality report: semantic completeness per era

Status: ready-for-agent

Coverage (counts) and quarantine (parse failures) are measured; semantic
quality is not. Nobody knows what fraction of notices per era carry a
title, buyer, value, CPV code, deadline, or winner — or how well awards
link to their notices, or how often a procedure's TED and DÖE readings
actually merged into one Tender (issue 12's promise).

Scope:
- Extend `crates/ingest/src/bin/verify.rs` with a `--report` mode (or a
  sibling bin if cleaner): per-era, per-profile field-completeness rates
  (title/buyer/value/CPV/deadline/winner), award→notice linkage rate,
  lot_results density for CAN-bearing eras, TED↔DÖE merge rate for the
  overlap window (2022-12→). Reads via `/v1/sql` against production
  (needs an API token — the acceptance-verify account; investigate how
  to mint one, report if blocked).
- Human-readable report + `--json`; commit a snapshot of the first full
  run to docs/research/data-quality.md with anomalies called out.
- Each anomaly worth acting on becomes a new issue (parser gap, mapping
  bug, projection gap) — the report is the discovery tool, not the fix.

Note: full-archive numbers only meaningful after the issue-15 backfill
completes; build and validate the tool now against current partial data
(1993→~2011 present), run the definitive report after.

Acceptance: report runs green against prod; documented baseline in
docs/research/; anomalies filed as issues.
