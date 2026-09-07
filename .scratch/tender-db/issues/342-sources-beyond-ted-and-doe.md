# 342 — sources beyond TED and DÖE ("international"): nothing exists, the entry contract does

Status: UNIT 1 DONE 2026-09-07 (owner) — `docs/research/uk-fts.md` written from the live API and re-checked adversarially (8/8 load-bearing claims confirmed, 4 overstatements corrected): GO for unit 2, the FTS fetcher. Unit 2 is ready-for-agent (shape below). Was: ready-for-agent — DECIDED 2026-09-07 (owner): the first market is the UK
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
