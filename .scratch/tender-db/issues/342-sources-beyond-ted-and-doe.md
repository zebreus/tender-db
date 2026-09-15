# 342 — sources beyond TED and DÖE ("international"): nothing exists, the entry contract does

Status: **UNIT 2 COMPLETE and MEASURED ON PROD 2026-09-08.** Commits (a) fetcher, (b) profile, (c) the crosswalk GB arm (`db02939`) + parser (`d7264c8`), plus two fixes the live run found: reparse reaching pending notices (`3192851`) and the zone-less date (`18ce0c1`). All deployed; June 2025 reparsed and projected clean — **7,243 / 7,243 parsed, 0 quarantined, 0 failing, 0 islands → 6,239 tenders**. Remaining: the 2021-01 backfill, and the docs (plan step 12). Unit 1 DONE: `docs/research/uk-fts.md`, adversarially re-checked, GO. Plan: `.scratch/tender-db/342-fts-plan.md`.
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

## Unit 2, measured on prod (2026-09-08)

The month the plan gates the backfill on, run end to end on the box:

| | |
| --- | --- |
| members walked | 7,243 |
| notices parsed | **7,243** (100%) |
| parse-quarantined | **0** |
| islands | **0** — every release keyed on its `ocid`, which is the whole design |
| tenders | **6,239** from 7,243 versions, so ~1,000 releases joined an ocid a sibling had already opened |

It took three passes, and both corrections were things only the live run could show:

1. **The first reparse did nothing** — "no packages hold parsed notices of fts:ocds-1.1". `reparse`
   required `parse_state='parsed'`, so the dry-first rung it was meant to complete was invisible to it.
   Fixed in `3192851`; the general lesson is on `Db::reparse_packages`.
2. **13 of 7,243 then failed.** Pulling `033064-2025` out of the archive showed
   `tenderPeriod.endDate = "2025-07-07"` — a bare date with no zone, which the parser refused. FTS
   publishes those; a bare date is a UK civil day. Fixed in `18ce0c1` with `fts::uk_offset`, and the
   release is now a named regression test. Note the 13 were left `pending` rather than quarantined —
   `reparse` will not downgrade a stored row on a failing re-parse, which is what kept them
   diagnosable.

### The GB crosswalk is being fed

`organizations` under country GB now carries **1,074 `GBPPON…`** identifiers and several hundred
`GBCOH…` across the 02/03/04/07/08/10 series — the two registers the arm keys, arriving from real
notices rather than fixtures.

**Filed as noticed, not fixed:** 176 GB rows carry an identifier starting `COMPANY…`, a scheme the
crosswalk deliberately keys as nothing (E0 exact only). Worth a look before the backfill multiplies it
— it may be a `GB-COMPANY-` scheme worth folding onto `GB:coh`, or it may be junk the idgate should
condemn. Not urgent at 176 rows.

### Before the backfill

`bids.statistics` still has no real-data coverage — no member fixture exercises it, and June 2025
evidently did not either (no parse failure came from it). Check it against a month that has bids
before trusting the arm, or the backfill will write untested rows at scale.

## Comments

### 2026-09-15 — API/data-quality review fan-out: `bids.statistics` becomes phantom `STAT-<id>` lot_results on FTS

This is the check the "Before the backfill" section above asks for, run against prod (rev `9e082fd`) on
2026-09-15 — and it fails. The arm that "still has no real-data coverage" does have real data now, and it
turns every OCDS `bids.statistics[]` entry into its own award decision.

Evidence, literally as run:

```
curl -s https://tenders.zebreus.click/v1/tenders/7954620 | python3 -c "import json,sys,collections;d=json.load(sys.stdin);lr=d['lot_results'];print(d['lots'],len(lr),collections.Counter(str(r['decision']) for r in lr));print([r for r in lr if r['key'].startswith('STAT-')][:2]);print(sum(1 for r in lr if r['decision'] and r['statistics']))"
# -> 28 103 Counter({'None': 66, 'selec-w': 33, 'clos-nw': 4})
# -> [{'awarded': None, 'decided': None, 'decision': None, 'key': 'STAT-215', 'lot': None,
#      'notice_id': 30808088, 'reason': None, 'statistics': {'bids': 3},
#      'statistics_withheld': 0, 'winners': []}, {... 'key': 'STAT-216' ...}]
# -> 0    (none of the genuine results carries any statistic)
```

Tender 7954620 (`fts:ocds-1.1`, 5 award+contract releases), by decision:

| lots | lot_results served | `STAT-` rows (decision/lot/winners null) | `selec-w` | `clos-nw` | genuine results carrying a statistic |
|---|---|---|---|---|---|
| 28 | 103 | 66 | 33 | 4 | 0 of 37 |

Per notice on that tender:

| notice | `STAT-` rows | real results |
|---|---|---|
| 30808088 | 10 | 5 `selec-w` |
| 30808648 | 20 | 10 `selec-w` |
| 30808828 | 8 | 2 `clos-nw` + 4 `selec-w` |
| 30808920 | 18 | 1 `clos-nw` + 9 `selec-w` |
| 30809108 | 10 | 1 `clos-nw` + 5 `selec-w` |

Not a one-tender accident — the same endpoint over the adjacent ids:

| window (GET /v1/tenders/{id}) | tenders carrying `STAT-` rows | lot_results rows | of those, `STAT-` | genuine results with a statistic |
|---|---|---|---|---|
| 7954610–7954620 (11 consecutive FTS tenders) | 8 of 11 | 205 | 128 (62%) | 0 of 77 |

Kinds served are the OCDS measure names — `bids`, `electronicBids`, `smeBids`, `foreignBidsFromEU`,
`foreignBidsFromNonEU`, `lowestValidBidValue`, `highestValidBidValue` — not the received-submission-type
codes `crates/store/src/canonical.rs:715` documents for `tender_version_result_stats.kind` (`tenders`,
`t-sme`, `t-eea`, …). And the value measures land in the same integer slot as the counts: on tender
7954611, `{'lowestValidBidValue': 4725722}` / `{'highestValidBidValue': 4725722}` — a monetary amount
served as a bare count, the `Statistic` struct having no currency field and `project.rs:4005-4045`
casting BT-759 `as i64`.

Mechanism, in our code: `crates/ingest/src/fts/parse.rs:305-317` opens a fresh `LotResult` section
`STAT-<id>` parented at `ROOT` per statistic and pushes BT-759/BT-760 onto it; the `Statistic` struct
(`parse.rs:688`) deserializes only `id`/`measure`/`value`, so `relatedLot` — which the real page fixture
`crates/ingest/tests/fixtures/fts/pages/2026-09-03-p002.json` carries on every statistic — is never read
and the lot link is dropped by construction on every FTS release that has bids.

Judge's reasoning for why this is ours, not the publisher's: *Premise verified live (curl
/v1/tenders/7954620): 28 lots, 103 lot_results = 33 selec-w + 4 clos-nw + 66 rows keyed STAT-<n> with
decision/lot/winners/awarded all null and only {"bids": N} or {"electronicBids": N}; 0 of the 37 genuine
RES- rows carry any statistic. This is something THIS system introduced, not a publisher shape: OCDS
bids.statistics[] is a per-lot submission count (relatedLot names the lot, as fixture
pages/2026-09-03-p002.json line ~199 shows), but crates/ingest/src/fts/parse.rs ~line 305 opens a fresh
`LotResult` section `STAT-<id>` parented at ROOT per statistic and pushes BT-759/BT-760 onto it, and the
`Statistic` struct (line 688) deserializes only id/measure/value — `relatedLot` is never read — so the
linkage to the award/lot is dropped on the floor and every stat becomes a phantom award decision. The
plan (.scratch/tender-db/342-fts-plan.md §3, line 155) specified "sub-section of the LotResult", so the
build diverged from its own design. Consequences the maintainer can act on: lot_results/v_lot_results/
v_awards carry ~2x phantom rows per FTS award notice; the served `statistics` kinds are OCDS measure
names (bids, electronicBids) while the schema note on tender_version_result_stats (canonical.rs line 715)
promises the eForms received-submission-type code (tenders, t-esubm, t-sme…), so cross-era consumers
summing statistics.tenders get nothing for FTS; and the DQ `with_awardable` predicate (data_quality.rs
line ~496) counts a NULL-decision result as awardable, so every STAT row inflates the FTS winner-rate
denominator. Not resolved on the board: issue 342 is open (status: unit 2 complete, backfill and docs
remaining) and its final section says verbatim that bids.statistics "still has no real-data coverage…
Check it against a month that has bids before trusting the arm, or the backfill will write untested rows
at scale" — this finding is that check, failing. No other issue covers it (grep for STAT-,
bids.statistics, result_stats hits only 372's eForms withheld-marker work and 342). Attach to 342 rather
than file twice. One correction to the finding's title: "currency and decimals dropped" is not a real
loss here — submission counts have no currency; the count value itself is served intact. Severity medium:
contained to fts:ocds-1.1 (one month today), but the 319k-release backfill 342 is about to run would
write it at scale, and it corrupts three consumer surfaces (lot_results row semantics, the stats kind
vocabulary, the DQ denominator).*

Two parts of the original draft did **not** survive checking, recorded so nobody re-derives them: (1) the
DQ `with_awardable` **denominator is not inflated in this sample** — that counter is per version, not per
row, a NULL-decision row with no winner evaluates NULL under `NOT (decision IN (...) AND NOT EXISTS
winners)` (verified in sqlite), and every notice here that has STAT rows also has a real `selec-w` row;
the judge's paragraph above still states the inflation as a consequence, and on this evidence only the
per-row predicate shape supports it, not the measured FTS denominator. (2) "decimals dropped" is
code-supported (`as i64`) but not data-confirmed. `v_lot_results`/`v_awards` were not checked — box SQL
reads were denied this session.

To close: parent the statistic section on the LotResult its `relatedLot` names (the plan's §3 shape)
instead of at ROOT, deserialize `relatedLot` on `Statistic`, map the OCDS measure names onto the
documented received-submission-type vocabulary (and give the value measures their own non-count slot),
with the p002 page fixture as the regression test — before the 319k-release backfill writes it at scale.
