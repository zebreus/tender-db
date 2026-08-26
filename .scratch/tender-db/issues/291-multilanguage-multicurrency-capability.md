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
