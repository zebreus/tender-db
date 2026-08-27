# 291 — proper multilanguage + multicurrency support (the pre-portal-expansion capability)

Status: ASSESSED 2026-08-26 (owner, on Lennart's direct question) — full repo survey run
(4-reader workflow + owner spot-verification of every crux). VERDICT: **no data-model
redesign needed — the spine was built multilingual and currency-carrying from day one;
the work is additive schema + read-layer + one contained fold-semantics decision +
refolds.** Sequencing proposal below. Gate: land BEFORE onboarding non-eForms portals.
Kind: capability (product surface)
Relates to: 172 (currency/classification drift — the prerequisite research), 292 (the
lang-tag defect this survey found, filed separately), 232 (text-era winner half), 231
(sdk-0.1 value refold), 165 (eforms-de successor, hard deadline 2026-12-02), 92 (fold
O(chain²) — watch before whole-corpus refolds), C10/C14 in docs/research/SUMMARY.md.

## Why "redesign" is the wrong frame (verified)

The 2026-07-19 representation decisions (CONTEXT.md) already put the multilingual/
multicurrency substrate in the spine:
- Every text fact carries language: `Fact::Text { field, lang, value }`
  (canonical.rs:903), `tender_version_texts.lang`, parse-layer `notice_texts.lang`.
  The tender DETAIL already serves every stored language variant.
- Every amount carries currency: `Fact::Amount { field, cents, currency, tax_basis }`
  (canonical.rs:904) — stored verbatim, plus the issue-251 tax_basis retrofit as the
  proven additive-column + refold playbook.

What is missing is everything ON TOP, none of which changes the spine's shape:

### Language gaps (all additive; one contained semantic decision)
1. **Lang-tag vocabulary is unnormalized — and it already bit us** (issue 292, LIVE
   DEFECT): three dialects coexist (eForms 3-letter `ENG`/`DEU`; r208/r209 2-letter
   `EN`/`DE`; text era `EN`-or-NULL), and every "English wins" pick compares the
   literal `'ENG'` — inert for the whole pre-eForms corpus. Fix = parse-time
   normalization + field-scoped refold. This is ALSO the cautionary tale for portal
   expansion: each new source adds a dialect unless a normalization layer exists.
2. **Reader-selectable language + fallback chain**: pure read/API work — a `?lang=`
   param parameterizing the existing ORDER-BY-rank picks; optionally per-language
   denormalized title columns if list latency demands it.
3. **Multilingual organization names**: one new satellite table
   `(org_id, lang, name, name_norm)` + widened mention capture + refold. Touches the
   provisional-merge keying `(name_norm, country)` — design with issue 168's
   false-merge study in hand.
4. **Coverage beyond "EN + original"**: r208/r209 translation copies live INSIDE the
   already-stored raw XML → parser-policy change + re-parse + refold (machinery
   proven by the DE-1.x/251 campaigns). Text-era non-EN deliveries are dispatch-time
   policy skips → recoverable by re-dispatch. Both are STORAGE decisions, not model
   ones: multilingual text measured ~86% of parsed size; all-language ≈188 GB
   (ted-access-channels.md) — needs a capacity check against the 1 TB volume first.
5. **The one spine-level decision — supersession semantics**: `Fact::key()` replaces
   ALL language variants of a field as one unit, so a monolingual corrigendum erases
   the other languages from the head version (tested behavior). If the product wants
   per-language carry-forward, that is a semantics change in ONE function + a
   whole-corpus refold — decide it in an ADR before widening coverage, because more
   languages multiply the cases. NOT a schema redesign.

### Currency gaps (all additive; the intellectual work is C10)
1. **No rates, no normalized column, currency-blind reads** (verified:
   read.rs:814-823 — `min_value`/`max_value` compare `MAX(a.cents)` numerically
   across currencies; list headline value = raw-cents MAX over mixed currencies; the
   /docs#caveats page already honestly says "No currency normalisation is applied").
2. **C10 (EUR-at-date column + rate source) was never researched OR decided** — it
   fell off the decision register (research-gaps Gap 7 / issue 172). This is the one
   genuinely unstarted piece: rate source (ECB daily reference rates + the fixed
   irrevocable pre-euro conversion rates for DEM/ESP/… which VERIFIED exist in real
   canonical rows), date policy (must be publication-date corpus-wide — every version
   has one; decision-date only exists on award blocks per issue 255 — usable as an
   award-subset refinement later), and a NULLable-by-policy stance for unconvertible
   rows (OP_DATPRO leak, MTL-in-2014, USN — 26 observed currency codes).
3. **Build shape**: an exchange-rates reference table (NOTE: the repo's first
   non-notice ingestion source — small but new machinery class); `eur_cents` beside
   published cents in tender_version_amounts AND the three sibling money loci
   (lot_results/bids/contracts) — additive migration + one whole-corpus refold (~3h
   precedent), NO re-parse (currency survives the parse layer already); a `currency`
   filter + normalized min/max/sort with a materialized index (the current min/max
   already routes to the isolation pool — a normalized indexed column makes it
   first-class); API stays `{cents, currency}` with an optional normalized field.
   Hard product constraint kept: published values are served as published — the
   normalized value layers BESIDE, never replaces.

## Is TED/DÖE stable enough? YES (evidence)

Of recent issues 270-290: only TWO were fold-correctness bugs (278, 279 — both found
by deliberate adversarial review, both fixed + deployed, residues static and bounded);
the rest is read-path/change-feed/operational polish. Campaigns are at documented
terminal states with acceptance numbers (text era 83.2% materialised, winner-named
100.0% where published, fold shortfall 0). Quarantine is terminal at ~306 outstanding
corpus-wide, all diagnosed. Zero TODO/FIXME in crates/ingest/src. The 251/255
campaigns just exercised the exact parse-change → re-parse → refold → acceptance-read
machinery this effort rides. The eforms-de successor watch (165) is armed. Current
churn lives in the serving/change-feed surface — orthogonal to this effort.

Caveats to carry into the plan: issue 92 (fold O(chain²), worst chain 3,282, "on the
clock") — check before scheduling whole-corpus refolds; issue 165's hard external
deadline 2026-12-02 may interleave; 278 track-2's ~45k static ghosts should ride one
of this effort's refolds for free.

## "The only thing from TED we deferred?" — NO

TED-side, still deferred besides language/currency: text-era BODY extraction
(header-only profile is an explicit v1 non-goal; A18 field-code semantics research
never done), text-era WINNER half (232 — blocked on a CO archive study), sdk-1.0
award-chain check (188 note), Reviews/E5 as canonical entities, PDF attachments,
buyer-side previous-publication identity edges (ADR-0011 scope), and the vanished
register decisions C14 (FTS/search) + C15 (SQL dialect promise). DÖE-side: 231's
sdk-0.1 value acceptance refold, two deliberately-unmapped sdk-0.1 amount ids,
eforms-de 2.2/3.0 vendoring (watch armed). Multilanguage/multicurrency is the only
deferred item of PRODUCT-SURFACE breadth — the rest are bounded slices or dormant
designs.

## Proposed sequencing (each step a shippable unit)

1. **Fix 292 now** (lang-tag normalization + field-scoped refold) — a live defect fix
   that IS step one of multilanguage, and the normalization layer portal expansion
   needs anyway.
2. **Research unit: C10/172** — rate source, date policy, pre-euro handling,
   unconvertible-row policy → ADR. (Also settles 172's classification-drift half or
   explicitly splits it off.)
3. **ADR: language semantics** — supersession unit (per-language carry-forward or
   not), fallback chain, org-name model.
4. **Build, additive**: rates table + eur_cents (4 loci) + refold; `?lang=` +
   `?currency=` + normalized min/max; org-name satellite + refold.
5. **Coverage expansions, storage-gated**: r208/r209 translation copies re-parse;
   text-era non-EN re-dispatch — each behind a measured capacity check.
6. THEN portal expansion, with the normalization layers as the entry contract for
   every new source.

## 2026-08-26 — Lennart's follow-up: decisions made, backlog materialized

Lennart (direct, 2026-08-26): file issues for all the smaller deferrals + remaining
quarantine work, and make the ADR-sized decisions with maintainability, scalability
and flexibility in mind. Done:

- **ADR-0013 (language model)**: canonical vocabulary ISO 639-2/T mapped once at the
  fold boundary (292's `normalize_lang`); supersession stays WHOLESALE per field —
  per-language carry-forward rejected because variants are translations of one value
  and a carried-forward old translation would serve stale content as current;
  fallback chain `requested → ENG → original → deterministic`; org names via one
  additive satellite with merge semantics unchanged; coverage widening stays a
  per-era storage decision.
- **ADR-0014 (currency normalization / C10)**: EUR-at-PUBLICATION-date derived
  `eur_cents` beside (never replacing) published values in all four money loci; ECB
  reference rates + fixed irrevocable pre-euro conversions in a `currency_rates`
  reference table (the first non-notice ingestion source); unconvertible → NULL with
  a per-era `eur_convertible_rate` gauge; `?currency=` filter + eur-based min/max.
  One open validation step: the 172 research pass validates pre-1999/accession
  cutovers BEFORE the backfill refold.
- **Backlog filed** (293-301): text-era body extraction, sdk-1.0 chain check,
  Reviews/E5 entities, document attachments, buyer-side previous-pub edge, C14
  search decision, C15 SQL-dialect promise, org fuzzy-matcher design, board-hygiene
  sweep. **Quarantine** (302-303): not-utf8 last-residue naming; terminal-state
  ledger + growth tripwire (gated on 288).

Sequencing update: step 1 (292) DEPLOYED (rev 5036a09, forward-only pending the
sized backfill); steps 2-3 (the ADRs) MADE — remaining before build: the 172/ADR-0014
validation pass, then the additive build units.

## Build progress (2026-08-27, owner)

- **Step 4 currency, units 1+2 SHIPPED + LIVE**: the `currency_rates` reference
  table (notice-layer, survives rebuilds), the 21 irrevocable euro conversion
  rates, the EUR-pivot lookup (7-day daily window, irrevocable exemption, EUR
  identity, honest-absence), and the `fetch-rates` admin job (ECB
  eurofxref-hist.csv through the ordinary fetch registry, chunk-upserted).
  First prod run: job 1300 `ok` — 85,446 daily rows (1999→today) + 21
  irrevocable rates loaded, raw CSV archived (rev d42c3e5).
- Remaining currency units: the ECU 1993-1998 series (sourcing = part of the
  172 validation pass), `eur_cents` migration + projection derivation (4 loci),
  the shared epoch-bump refold (carries 292's lang backfill + 278's ghosts),
  filters + `eur_convertible_rate` gauge + docs. `currency_rates` is NOT in the
  /v1/sql allow-list — public exposure is a 299 (C15 contract) question.
- 231's sdk-0.1 value refold ran (658,646 tenders, 34 min); acceptance number
  rides today's weekly DQ run.

## Build progress (2026-08-27, 03:4x — unit 3 live)

**eur_cents derivation SHIPPED + DEPLOYED (rev f676874)**: the four money loci
carry the nullable derived column (O(1) ALTERs ran at boot); the projection
loads one rates snapshot per run and every version converts at its publication
date through a per-version EurContext; unresolvable → NULL; equivalence/golden
suites green with the derivation on both sides. New daily folds populate
eur_cents from now on; existing rows stay NULL until the epoch-bump refold.

**Remaining currency units**: (a) ECU 1993-1998 series (172 validation pass —
also validates pre-1999 cutovers); (b) THE epoch-bump refold — one deliberately
scheduled run carrying eur_cents backfill + 292's lang normalization + 278's
~45k ghosts; check issue 92 (fold O(chain²)) before scheduling; (c) D5 read
surface (?currency= filter, eur-based min/max + index) + eur_convertible_rate
gauge + /docs. Then the language read-path units (ADR-0013 D3/D4).

## Build progress (2026-08-27, 04:4x — D5's refold-independent half)

Shipped this firing (ADR build order kept — the min/max→eur_cents FLIP and the
head-column materialization deliberately wait until AFTER the backfill refold,
because flipping on a mostly-NULL corpus would break the public filter):

- **`?currency=` published-currency filter** on Tenders/Lots: exact ISO-4217
  match (case-insensitive input, uppercased once at the API layer; junk 400s),
  an EXISTS over `tender_version_amounts` beside the value bounds — classified
  isolating, honoured-params/classification/isolation gates all extended.
- **`eur_convertible_rate` per-era gauge** riding the 265/266 pattern end to
  end: a `convertible` column on the amount-plausibility DQ query → headline
  `[num,den]` pair (serde-default so stored runs keep deserializing) →
  `tender_db_dq_eur_convertible_rate` on /metrics → an "EUR conv" dashboard
  column → an eur-conv rate column in report section 8.
- **Issue 92's longest-chain tripwire** in the same DQ unit (section 9, FLAG at
  ≥ 4,000, headline scalar + /metrics gauge).
- **Docs told the truth everywhere**: /docs caveat rewritten (as-published
  serving + eur_cents visible via /v1/sql + the pending flip announced), filter
  table + applies matrix + OpenAPI param + README; `v_tender_amounts` now
  carries `eur_cents`; /v1/sql column notes for `eur_cents`/`awarded_eur_cents`
  (they were already PRAGMA-visible with no note — gap found by the doc
  survey); README's stale `country` ISO-3 and `limit` max-500 claims fixed.

Remaining: (a) ECU series + 172 validation pass; (b) the epoch-bump refold;
(c') post-refold: min/max flip to eur_cents + materialized head column
(`tenders.current_value_eur_cents` + deferred index, the `current_deadline`
pattern) + the de-isolation MEASURED per the 88d876a rule + the CHANGELOG
surface (none exists yet — the flip entry creates it).

## Build progress (2026-08-27, 05:1x — ECU series loader + the stale-rates gap)

Unit (a)'s load half BUILT (prod run pending deploy): source verified by live
fetches — Eurostat splits the Commission's official daily ECU series across
`ert_h_eur_d` (former national currencies: DEM/FRF/ITL/ATS/…) and
`ert_bil_eur_d` (GBP/DKK/USD/…), both daily back to 1974, OBS_VALUE = national
units per 1 ECU (DEM closes 1998-12-31 on the irrevocable 1.95583 EXACTLY; ITL
1936.27 — the two constants cross-confirm), CC BY 4.0, keyless. New
`fetch-rates-ecu` admin job fetches both through the registry (source
`eurostat`), parses SDMX-CSV (header-driven, CRLF, confidential empty-value
rows skipped — 48 RSD rows measured in the real file), re-filters to
< 1999-01-01 (the ECB series is the authority from there; 1:1 by Council Reg
1103/97), REPLACE-upserts as `eurostat-ecu`. Expected prod load: 56,706 rows.
Real downloaded files validated against the parser's assumptions locally.

**Stale-rates gap found and fixed in the same unit**: nothing scheduled
`fetch-rates`, so with a 7-day daily window every fold more than a week after
the last manual run would have derived NULL eur_cents for non-EUR amounts —
new folds would have silently regressed within days of unit 3 going live. The
daily chain now runs `fetch-rates` before the projection (queued-guarded).

The 172 validation half of (a) — sampling real pre-1999/cutover canonical rows
against the loaded series on a snapshot — remains open, and stays the gate
before the epoch refold.

**172 validation pass RAN (2026-08-27 07:1x, snapshot census + samples): PASS
with one real find** — 1993–1996 publishes TED-legacy currency codes
(LIT/UKL/DKR/…, plus ECU itself), fixed by a `canonical_currency()` alias map
at lookup (published codes stay verbatim; ECU/XEU = EUR identity). Magnitudes
validated against the fetched series. **The ADR's refold gate is OPEN** once
the alias fix deploys; the epoch refold is the next deliberate prod step
(carries eur_cents backfill + 292 lang + 278 ghosts; issue 92 says seconds per
worst chain, precedent ~3h corpus-wide).

**Prod run (2026-08-27 05:53 UTC, rev fb4feb6)**: deploy green; job 1303
`fetch-rates-ecu` ok in 3s — **56,706 rows upserted, exactly the local
prediction** (ert_h_eur_d 29,468 + ert_bil_eur_d 27,238; the 48 confidential
RSD rows skipped as designed); lookup cache now 142,173 rows = 85,446 ECB +
56,706 ECU + 21 irrevocable, reconciling exactly. The rates series is now
CONTINUOUS from 1993-01-04 to today. Same deploy carried the D5
refold-independent half live (currency filter, gauges, docs) and put
fetch-rates on the daily chain — first scheduled run rides today's 07:35
chain. 231 closed both halves off job 392's report (value 0.0% → 0.6%).

## THE epoch refold ENQUEUED (2026-08-27 07:44 UTC, jobs 401 refold + 402 project)

All 23 profiles, expect 14,300,000 (±25% vs 14,328,899 headline versions),
enqueued behind today's completed daily chain on rev 668a399 (alias fix live).
Carries: the corpus-wide eur_cents backfill (rates continuous 1993→today, ECU
aliases in), 292's lang-tag normalization backfill (~100M rows), and 278's
~45k regrouped ghosts via the 279 sweep gates. Expected ~10-14h at the sdk-0.1
fold rate (331 notices/s); the change feed will carry the full rewrite (by
design — the ONE planned whole-corpus refold). The hourly firings monitor;
after it lands: the D5 flip unit (min/max→eur_cents + head column + CHANGELOG)
and the eur_convertible_rate gauge gets its first real numbers on Sunday's DQ
run (or an earlier manual one for the acceptance read).

## Build progress (2026-08-27, 09:2x — ADR-0013 D3 shipped in code)

`?lang=` landed (7f89576): requested → ENG → labelled → unlabelled at every
read-time pick (list all sorts, detail header + lot titles, /v1/lots, SSE
include_data), 639-1/639-2 input normalized through the fold's own map,
selector-not-filter semantics classified and documented, detail endpoint now
validates its query string. ADR-0013 amended: the "original" leg has no
persisted data source — needs an additive column + refold if wanted. Deploys
with the post-refold batch (6da6d96 flip + this). Remaining language units:
org-name satellite (D5), coverage widening (per-era storage decisions).
