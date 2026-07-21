# 27 — Data-quality report: semantic completeness per era

Status: needs-verification

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

## Comments

### 2026-07-21 — tool built + validated; prod baseline pending token (needs-verification)

Built `crates/ingest/src/bin/data-quality` (thin transport) over
`crates/ingest/src/data_quality.rs` (measurement logic, unit-tested) — a
**sibling binary**, not a `--report` mode on `verify`: `verify` is a
pass/fail acceptance harness against external ground truth with exit-code
semantics, this is a descriptive per-era measurement (rates/densities, no
pass/fail); folding the two would muddy `verify`'s contract. Metrics:
per-era field completeness (title/buyer/value/CPV/deadline/winner),
award→notice linkage, results materialisation (award-notices →
`lot_results`), TED↔DÖE merge. Unit = tender-version (≈ per notice); era =
the version's notice profile. Queries are bounded satellite-driven
aggregates (safe against a live, backfilling instance). Tests green
(`cargo test -p ingest`), clippy clean (`-D warnings`); validated
end-to-end in `crates/ingest/tests/data_quality.rs` (identical SQL against
a scratch DB projected from real fixtures).

**Token gap (reported to team lead):** `/v1/sql` is account-gated; the
`acceptance-verify` credentials are documented nowhere reachable (repo,
docs/operations.md, VPS /opt/tender-db/, shell history), the admin API
mints only ingestion jobs, and minting needs the account password — not
guessed. Prior token was revoked. So no prod numbers yet; the 33 GB prod
DB is also under active backfill write (refused even a read-only sqlite3
open). First-run snapshot in `docs/research/data-quality.md` is therefore
**fixture validation**, clearly labelled.

**Anomaly filed:** issue 29 — DÖE sdk-0.1 (~40 % of German volume) projects
to empty canonical Tenders (0 % on every field); its `SDK01-*` field stems
are unmapped in the projection. (Results-materialisation gap is already
tracked as issue 22; small-sample r2.0.8 value/deadline gaps noted in the
doc, not yet filed — confirm at scale.)

**To close:** mint a token from the dashboard, run
`data-quality --token …` (and `--json`) against prod **after** the issue-15
backfill + issue-22 results re-projection complete; that full-archive run
is the definitive baseline. Update docs/research/data-quality.md with the
real numbers and file any new era-scale anomalies.
