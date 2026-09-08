# 342 — sources beyond TED and DÖE ("international"): nothing exists, the entry contract does

Status: UNIT 2 — commits (a) fetcher, (b) profile and the raw-reader fix are BUILT, DEPLOYED (`5a48d88`) and MEASURED. **Commit (c) is BUILT and GATED 2026-09-08**: the crosswalk GB arm (`db02939`) and the parser (`d7264c8`), `ops/check.sh` GATE-EXIT=0 both times. Not yet deployed, and the 7,243 June-2025 releases are still `parse_state='pending'` on the box until a Reparse runs. Remaining: deploy, Reparse 2025-06 → project → measure, then the 2021-01 backfill and the docs (plan step 12). Unit 1 DONE: `docs/research/uk-fts.md`, adversarially re-checked, GO. Plan: `.scratch/tender-db/342-fts-plan.md`.
issues for that"). No non-TED/DÖE source has ever been researched for onboarding;
the first step is a market choice, which is Lennart's.
Kind: capability (sources) — the product-breadth half of "full internationalization"
Relates to: 291 (its "portal expansion" gate — ADR-0013/0014 were sequenced to
land BEFORE any new source, and have), docs/research/german-portals.md (the only
source research so far, German-only), 12 (how DÖE was onboarded), 292
(`normalize_lang`), ADR-0014 (`canonical_currency`)

## Where it stands

- Live sources: **TED** (EU-wide, every era 1993→) and **DÖE**
  (oeffentlichevergabe.de, Germany, eForms-DE). That is the whole list.
- Researched but not onboarded: the German below-threshold portals
  (docs/research/german-portals.md, 2026-07-19: ~30 Vergabeplattformen, two federal
  mirrors; candidates ranked on no-sign-in, history, machine access, reuse terms).
- Not researched at all: anything outside Germany that is not already in TED.
  Note that TED already carries every EU/EEA above-threshold notice, so
  "international" in the EU sense is covered; a new source adds BELOW-threshold
  national notices or NON-EU procurement (UK Contracts Finder / Find a Tender,
  Swiss SIMAP, Norwegian Doffin, US SAM.gov, …), each with its own format,
  identifiers, currencies and terms.

## What a new source needs — and already has

The 291 assessment made the normalisation layers the entry contract, and they
exist: language tags through `normalize_lang` (ISO 639-2/T at the fold boundary),
currency codes through `canonical_currency` (published verbatim, converted beside),
country/NUTS handling, the profile dispatcher, the fetch registry with archives,
the identity model (ADR-0001) and the dry-first job ladder. Onboarding is a
parser plus a fetcher plus two map reviews, not a model change — DÖE's onboarding
(issue 12) is the template and took a parser, a probe job and a daily reconcile.

## What a decision needs

Pick a market, then a research unit answers: bulk access and terms, history depth,
notice format (eForms-based or bespoke), identifier schemes (issue 300's
crosswalk), currency, and expected volume — sized against the 1.7 TB volume
(702 GB free before the 304 campaign; ~+100 GB after it). That research is one
firing per candidate; the build is a multi-day unit per source.

## Not this issue

Below-threshold German coverage via DÖE's own feeds (already the DÖE source's
scope) and the text-era/r209 language work (304). This is only "a source that is
not TED or DÖE".

## Decision (2026-09-07, owner)

First market: the **UK** — Find a Tender Service (above-threshold, post-Brexit) with
Contracts Finder beneath it. Reasons: the largest procurement market not in TED since
2021, English, an open OCDS release feed with a documented API and an open licence,
company identifiers from one register (Companies House), one currency (GBP, already in
the rate series). Swiss SIMAP and Norwegian Doffin are the next candidates by the same
criteria; SAM.gov is a different scale and format and comes last.

Unit 1 (research, one firing): the OCDS release schema as FTS publishes it, the
identifier schemes on parties, the licence text, the daily volume and the history
depth available, the fetch channel (bulk vs API paging) — written up as
`docs/research/uk-fts.md` with a go/no-go for the fetcher. Unit 2: fetcher + profile +
parser through the existing entry contract (`normalize_lang`, `canonical_currency`, the
profile dispatcher, the fetch registry), dry-first.

## Unit 1 result (2026-09-07): GO

`docs/research/uk-fts.md`. The short form: FTS is an unauthenticated, documented OCDS 1.1.5
release API under OGL v3 (attribution "Contains public sector information licensed under the
Open Government Licence v3.0."), English only, one publisher prefix (`ocds-h6vhtk-`), one
notice per release, GBP for 98.9 % of amounts (AED 1.0 %, USD 0.1 %).

| measured | value |
|---|---|
| releases per weekday (1–4 Sep 2026) | 457 / 371 / 436 / 471; weekends 7 and 1 |
| bytes per release | 14,317 (uncompressed JSON) |
| notices per year (year-end ids) | 32,542 (2021) · 36,737 · 38,048 · 41,642 · 86,608 (2025) · ≈123k pace (2026) |
| full backfill | 319,742 releases ≈ 4.6 GB raw JSON, ≈ 3,200–4,700 requests, ~14 GB of DB growth |
| history | FTS from 2 Jan 2021; Contracts Finder OCDS from 26 Feb 2015; UK notices before 2021 are in TED |
| parties without any identifier | ~95 % before 2025, 9.2 % now; GB-PPON dominant (`AAAA-1111-AAAA`), GB-COH second |

Three risks the fetcher design has to carry: (1) an opaque, variable 429 limiter (`Retry-After:
120`) — persist (window, cursor) progress, honour every Retry-After, backfill as a resumable
one-to-two-day job; the data.gov.uk daily XML zips are a limit-free cross-check for counts;
(2) weak organization identity — the PPON is not in org-id.guide, Companies House numbers arrive
unpadded and prefixed, so the crosswalk needs a GB arm (COH normalised to 8 characters, PPON as
its own series, COH↔PPON pairs from `additionalIdentifiers` as E2 evidence) and the matcher must
not expect TED-grade identifier rates for 2021–2024; (3) two regimes (PCR 2015 CELEX and
Procurement Act UKPGA notices coexist), amendment-bearing releases carry only deltas (merge per
ocid in the projection), and Contracts Finder duplicates FTS 2021–Feb 2025 with no machine link
— so unit 2 is FTS-only and Contracts Finder is a later, deduplicated unit.

## Unit 2 (ready-for-agent): the FTS fetcher, profile and parser

Through the existing entry contract, dry-first, in three commits: (a) a `fts` source in the fetch
registry — daily poll of the previous UK-local day with a 2-hour overlap on `updatedFrom`,
idempotent on release id, `Retry-After` honoured, (window, cursor) progress persisted, a
backfill mode by 1-day windows from 2021-01-01; (b) a profile `fts:ocds-1.1` dispatched on the
package header, the release stored as the notice body with `original_lang = ENG`, `publication_id`
= the release id (`nnnnnn-yyyy`), `procedure_key` = the ocid, the `tag` set and
`documents[].noticeType` (UK1–UK17) as the subtype; (c) the parser onto the notice model —
parties → mentions with country `GB`, `scheme` kept, `identifier_kind = national`; lots, values
(`amount`/`amountGross`, GBP), periods, awards via `relatedLots`, suppliers; plus the crosswalk's
GB arm. Measure on one month of 2025 before the backfill.

## Unit 2 commit (a) — the fetch registry gains `fts` (2026-09-07)

Built to `.scratch/tender-db/342-fts-plan.md` §1 by a subagent, then reviewed by four adversarial
lenses (behaviour parity for TED/DÖE, fetcher correctness, operational hazard, do the tests pin
what they claim) with three refuters per finding. 18 candidates; the corrections applied are
recorded in the plan's new §4b. The sharpest — found independently by all four lenses — was that
the daily chain's `fts daily (all)` process pass would quarantine every OCDS release as
`unparsable-xml` until commit (b) exists; only the probe ships in (a).

What landed: `ingest::fts` (window/URL builders, UK DST, `member_bytes` — a deterministic
single-release package minus the page-specific header fields, `PROBE_DAY_CAP`); `fetch_fts`
(staging dir + `cursor.json`, resume mid-window, zip assembly with fixed entry timestamps so an
unchanged re-walk hashes equal, 0-member zip for an empty window), `probe_fts_daily` (capped
walk-forward), `latest_fts_day`, `get_bytes`/`land`/`retrying` extracted so the paged and plain
fetchers share the immutability and retry rules, 503 in the throttled class; the supervisor's
`ProbeFts`, `Fetch` routing, `build_target`/`fetch_parts`/`enqueue_backfill` arms and the
`RehashProbe` exemption (bodies moved into methods that box internally — the 62-arm `run_spec`
future was 24 KB from the stack limit); the CLI's `fts` source. Tests: 13 in `tests/fetch.rs`
(including the throttle-exhaustion arm, the `_noid/` archive, the staging-debris discard and the
per-tick cap), the register-archive extension, and unit tests on the builders and DST edges.

Next: commit (b) the profile arm (`fts:ocds-1.1`, publication_id = release id, procedure_key =
ocid) with the process pushes, then commit (c) the parser and the crosswalk GB arm. First deploy
is after (b); first measurement is `2025-06` (the plan's step 7).

## Unit 2, first live measurement (2026-09-07, plan step 7)

Deployed `5a48d88`. `POST /admin/jobs {kind:"fetch", source:"fts", package_kind:"monthly",
period:"2025-06"}` (job 796) walked the month as 30 contiguous one-day windows against the live
API at the paced cadence and landed **one 17.3 MB package**, `fts/monthly/2025-06.zip`, with the
staging directory removed — the first proof the fetcher works outside its fixtures. `process`
(job 797): **7,243 members → 7,243 notices, 0 parsed, 0 quarantined, 0 unrecognised, 0
duplicates**.

What the rows say (bounded read by primary-key range):

- all 7,243 under profile `fts:ocds-1.1`, all `parse_state = 'pending'` — the identity-only rung
  commit (b) intends, since the field parser is commit (c);
- `published_at` is NULL, not the unix epoch — the honest absence issue 367 is about;
- **0 members under the `_noid/` prefix**: no release in the month lacked a usable id;
- publication ids run **028961-2025 … 036267-2025**, contiguous FTS notice numbers.

**Completeness.** FTS numbers are a per-year sequence, so the captured range spans 7,307 numbers
and we hold 7,243 of them — **99.1%**, with 64 in-range numbers absent. That is expected rather
than lost: the API filters on a release's LAST-UPDATE instant, so a June notice amended in July
appears in July's window, not June's. It matches the plan's ±1% acceptance bar against the
independent data.gov.uk daily XML counts (332 files for 2025-06-11 against our ~345/weekday), and
the daily zips remain the limit-free cross-check when the backfill runs.

Nothing else in the corpus moved: these notices are pending, so the projection ignores them and
no Tender rows exist for FTS yet — which is the dry-first rung the plan asked for.

## Commit (c), as built (2026-09-08)

`crates/ingest/src/fts/parse.rs` (935 lines incl. 9 tests) + `pub mod parse;` in `fts/mod.rs` + the
`fts:` arm in `process.rs`; the crosswalk GB arm and its 3 tests plus the `normalise_identifier` test
went first, in `db02939`.

Built against the fixtures rather than the plan's table — §3b of the plan records the six places they
disagreed. Departures from §3 worth knowing:

- **Subtype** is the UK form code (`UK1`…`UK15`) off the single `noticeType`-bearing document, searched
  across `tender.documents`, `planning.documents`, `awards[].documents` and `contracts[].documents`,
  since which branch carries it depends on the archetype. `tag.join("+")` is the fallback.
- **`BT-536/537-Procedure` are not emitted** — `tender.contractPeriod` does not exist in this source.
  Lot-scoped contract periods are.
- **CPVs and NUTS come off `tender.items[]`**, scoped to the item's `relatedLot` when it names one that
  exists, else procedure-wide. `tender.classification`, when present, is `BT-262-Procedure`.
- **A gross-only value** is emitted with `TED-VAL_TOTAL_TAX_BASIS = incl`; a net one with `excl`.
- **Delta awards emit nothing**: `is_delta()` is "no status, no value, no suppliers", which is exactly
  UK15's 34 `{id, amendments}` skeletons.
- **`bids.statistics`** land on a `STAT-<id>` LotResult section. No member fixture covers them (only
  page p002 does), so that arm is written but untested against real data — the first thing to check
  after the Reparse.

Quarantine reasons the parser can raise: `unparsable-json`, `ocds-release-count`, `missing-ocid`,
`unrepresentable-value`. The last one is the `1e9999` guard, now at the value layer as well as the page
layer: an exponent amount refuses rather than rounds.

### Next, in order

1. Deploy (queue idle).
2. `Reparse` scoped to the fts 2025-06 fetch, then `project`. Both are small — 7,243 notices.
3. Measure: notices parsed vs quarantined by reason, tenders/versions created, buyers resolved, and
   whether any GB identifier reached `organizations` with a `GB:coh`/`GB:ppon` canonical key.
4. Only then the 2021-01 backfill, and the docs (plan step 12).
