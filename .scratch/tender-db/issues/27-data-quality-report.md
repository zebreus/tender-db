# 27 — Data-quality report: semantic completeness per era

Status: CLOSED-SUPERSEDED-DELIVERED (2026-08-22, owner) — issue 230's server-side weekly measurement IS this report at today's scale; see closing note at the bottom. Previous state: REOPENED-AS-ROTTED (2026-08-18, owner). Verification ATTEMPTED against prod rev `cc0ef20` and
the acceptance does NOT hold today: "report runs green against prod" fails — all 11 queries time out on
the 10s `/v1/sql` cap at full-corpus scale (14.15M tender_versions, 40.9M organization_mentions), so
every section prints empty. The other two clauses DO hold: the baseline is documented in
docs/research/data-quality.md, and the anomalies it found became issues (100, 101, 188 among them).

Nothing regressed in the code — the queries were bounded for the mid-backfill corpus this was written
against, and the corpus outgrew them. (Correction to this note's first version: the tool DOES exit
non-zero on a degraded run and names each failed query on stderr — I had inferred otherwise from its
docstring without reading the exit path. The rot hid because nothing RUNS the tool, not because it lied.) Filed as issue 230 with the measured
output, the reason not to simply raise the cap, and the recommended fix (compute it server-side on the
dashboard refresher's cadence, which already runs comparable aggregates at this scale).

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

### 2026-07-21 — live prod leads folded in (via public /api/dashboard)

Team-lead leads analysed against real prod data from the **public**
`/api/dashboard` (single cached GET; no token, no touching the locked DB) —
see docs/research/data-quality.md §"Live prod observations". Headlines:
(1) `lot_results`=12,600 / `bids`=23,987 / `contracts`=14,767 → results
materialise (issue 22 fear resolved on rev bad8dda); (2) eForms award
linkage reads 96–99 % unchained, but the canonical layer is pre-project-
rerun and near-only 2026 eForms days, so it's the expected missing-referent
artifact — watch-item with rule "re-check post-backfill+project; >90 %
unchained at full data ⇒ reference-resolution defect, file it"; (3)
quarantine headline (1.21 M) is mostly benign — 96.7 % text coverage
coexists with the two big text-era buckets → filed **issue 30** (split the
metric benign-vs-actionable + field-code top-N, needs token; two concrete
r2.0.8/text parser gaps captured there).

### CLOSED — superseded and delivered by the 230 line (2026-08-22, owner)

What this issue asked for exists and RUNS, just not through the transport it originally specified.
The rot note above (2026-08-18) became issue 230, and 230's line delivered the whole acceptance
server-side, where corpus-scale aggregates are actually computable:

- **"report runs green against prod"** — the weekly `data-quality` supervisor job measures 23 eras
  over 32 windows (last run: job 307, 4,528s, `0 label(s) unmeasured`), windowed so no query rides
  the 10s `/v1/sql` cap. Sections 1-8 render server-side; reports are STORED (`put_report` /
  `latest_report`), the dashboard Quality panel serves them, `/metrics` exports ~130 per-era gauge
  series, and a presence step-change alarm watches run-over-run deltas (issues 260/265/266/267).
- **"documented baseline"** — docs/research/data-quality.md holds the 2026-07 baseline as history;
  the stored `data-quality-headlines` history (12 runs) is the living baseline now.
- **"anomalies filed as issues"** — the original run filed 100/101/188 among others; the new
  machinery's first reads filed and resolved 263/264 (value-rate and age-confound investigations,
  both archive-verified as publication reality, caveats now printed in the report itself).

The `crates/ingest/src/bin/data-quality` CLI stays, deliberately: `--print-sql` is the one honest
way to read/diff the exact statements the job runs (including the instantiated windowed form), and
the transport still works against small instances. Its corpus-scale /v1/sql mode remains what the
2026-08-18 note measured — timing out by design of the cap — and nothing should "fix" that: the
server-side job is the fix.
