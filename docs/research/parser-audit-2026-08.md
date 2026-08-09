# Parser audit — business-term coverage, correctness, speed (2026-08-09)

Commissioned by Lennart mid-session ("make sure that our parser is good and fast and
supports all business terms"). Everything below is checked against the tree at this
commit, the vendored inventories, and tonight's gate run — not remembered.

## 1. Business-term coverage: exhaustive at the parse layer, BY CONSTRUCTION

The eForms parser does not have a hand-maintained list of supported BTs that could
silently fall behind. Coverage is structural (ADR-0002 + ADR-0004):

- **Every field id in every vendored `fields.json` must carry a mapping decision.**
  `every_sdk_field_has_a_mapping_decision` (ingest/tests/eforms.rs) walks all 18
  accepted inventories and fails LISTING the undecided ids. Decisions are computed
  from field *type* + xpath context (`sdk::decide`), so a new field in a newer SDK
  cannot arrive undecided — an unknown `type` makes the harness fail at vendor time.
- **Every decided xpath must be reachable.** `every_sdk_xpath_folds_into_the_match_index`
  builds the match tree for all versions and fails on any xpath shape the grammar
  cannot represent (the issue-74 descendant-axis/boolean-or class).
- **At runtime, nothing goes unclaimed.** The parse is a simultaneous descent of
  document and match tree; content outside the inventory quarantines the notice
  whole (ADR-0004) rather than dropping values. A coverage gap is therefore *visible
  work in the quarantine ledger*, never silent loss — the exact mechanism that
  caught and then recovered the 218K eForms-DE 1.x notices (issues 71/75/85).

Vendored inventory, per accepted CustomizationID (fields / distinct BT-n stems):

| line | versions | fields | distinct BTs |
|---|---|---|---|
| EU eForms SDK | 1.0, 1.3, 1.5–1.15 (13 minors) | 708 → 1,256 | 255 → 286 |
| eForms-DE | 2.0; 2.1@EU-1.13; 2.1@EU-1.14 | 1,235–1,260 | 272–288 |
| eForms-DE 1.x | one merged empirical era file | 460 observed paths | (path-keyed) |
| DÖE sdk-0.1 | one empirical era file | 293 observed paths | (path-keyed) |

Of SDK 1.15's 1,256 fields: 480 are `attributeOf` companions (consumed by their
owning field's handler) and 3 are `OPA-*` virtual views over xpaths another field
owns — the documented exclusions; every remaining field writes into one of the 8
typed fact tables (texts/codes/classifications/amounts/dates/integers/numbers/ids).

**"All business terms" per layer** — the deliberate architecture, not a gap:
- *Parse layer*: every BT of every accepted version, exhaustively (above).
- *Canonical layer*: the curated headline projection (deadlines, values, parties,
  lots, results…). Known projection-side gaps are tracked issues, not parser gaps:
  98 (org reference classes), 100 (award→winner chain), 101 (surfacing), 103, 108.
- *Analyst surface*: the full parsed layer stays queryable via `/v1/sql` (issue 50),
  so no BT is unreachable even before it earns canonical projection.

**New-version arrival** is fail-visible: `sdk::resolve` returns `None` for an
unknown CustomizationID → `unknown-customization` quarantine → dashboard funnel →
vendor the new `fields.json` (the harness then forces decisions) → bulk reclaim
(issue 76 machinery, now with issue-87 attempt stamping). Deadline-driven watch for
the eForms-DE successor is issue 165 (acceptance of 2.1 ends 2026-12-02). The other
profiles (r209/r208/text/DÖE) carry the same checklist mechanism with their own
vendored inventories (`ted-export-inventory.json`, `text-inventory.json`).

## 2. Correctness ("good")

- The completeness harness + reachability test run in the standard gate (green
  tonight at this commit).
- Red-check culture: the r208 deadline mapping (174) and lots fixes (115/116) all
  landed with tests demonstrated failing against the unfixed code.
- Per-era spot checks: tonight's r208 verification (tender 5000000, 2012, deadline
  serving via API). The durable per-era "headline fields project" matrix test is the
  agreed follow-up from 174 — still to build.
- Open, tracked: issue 104 (eForms-DE golden fixture), issue 98 (org
  reference-class audit), issue 100 (DE-1.x winner chain), issue 138 (pre-registered
  criteria for the r209 execute re-spec).

## 3. Speed ("fast")

Design: streaming single pass — tar walk → dispatch → parse (roxmltree, one match-
tree descent, no XPath engine at runtime) → bounded channel (64) → single batched
writer. Memory flat by construction (the channel bound is what let monthly-scale
packages ingest at all).

Measured evidence on record:
- **Steady state (what matters daily): comfortably fast.** The 09:35 tick fetches,
  processes and projects the day within minutes; ingest freshness tonight: 35 min
  after the tick, `/health/deep` green. Daily volume (~2–4K notices) is seconds of
  parse at the measured rates.
- **Bulk rate**: ~70–120 notices/s single-threaded, era-dependent (issue 28, from
  full-archive runs). Reclaim passes run at seek-speed after issue 80's index
  (~45 members/s, DB-write dominated — parse is not the limiter there).
- **The known cost**: a full-corpus reprocess at 14.2M notices ≈ 1–2 days. That is
  issue 28's scope, and its instruction stands: PROFILE FIRST (issue 19's lesson —
  the bottleneck was not where assumed). The likely win is parallel parse/map
  feeding the existing single writer; do not build it without the profile.
- Profiling needs real packages + a release `process` bin; the deployed store path
  ships only `server`, so the profile run is a deliberate step (snapshot box or
  shipping the CLI bin), noted on issue 28.

## 4. Verdict + follow-ups

The parser meets the ask at the parse layer: BT coverage is total per accepted
version and enforced by the gate; unknown content and unknown versions fail
visible and recoverable; steady-state speed is not a concern. The genuinely open
work is (a) projection-side coverage issues 98/100/101, (b) the per-era headline
matrix test from 174, (c) issue 28's bulk-reprocess profile, (d) issue 165's
DE-successor deadline, (e) issue 104's golden fixture.
