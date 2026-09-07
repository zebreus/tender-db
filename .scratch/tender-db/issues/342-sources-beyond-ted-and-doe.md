# 342 — sources beyond TED and DÖE ("international"): nothing exists, the entry contract does

Status: ready-for-agent — DECIDED 2026-09-07 (owner): the first market is the UK (Find a Tender Service, OCDS releases, Open Government Licence); unit 1 is the research entry at the bottom. Was: BACKLOG / needs-decision (filed 2026-09-02 on Lennart's "didn't you create
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
