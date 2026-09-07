# Organization identifiers and value domains — bounded first pass (issues 168 + 171)

Research date: 2026-08-09 (02:00–02:50 UTC, low-traffic window). Closes the *first-pass* halves of research gaps #2 and #6 (`docs/research/research-gaps-2026-08.md`); the full studies need the snapshot machine (§6). Repo at `e46ee0cf4dbd1fefc79bb723f395e72af821093b`.

Confidence markers: **[verified aggregate]** = whole-table result via an index-bounded aggregation on prod; **[verified sample]** = LIMITed or id-window sample on prod (window stated, shares are of that window only); **[code-verified]** = read from the repo at the rev above; **[known]** = existing issue/doc record, not re-derived here.

## 0. Method

All reads went through the bounded `/v1/sql` endpoint (one bare SELECT, 10 s cap, allow-list per `crates/app/src/v1/sql.rs::ALLOWED`), per `docs/agents/prod-box-reads.md`, on the team lead's word (the task brief). ~100 calls total; typical server-side runtime 0.5–2 s (2–4 s including SSH round-trip). The per-token limiter (300/h) was hit mid-study; all subsequent calls were paced at ≥14 s.

Query shapes used, cheapest-first — this list is the reusable part of the method:

| shape | example | outcome |
|---|---|---|
| PK endpoint seek | `SELECT MAX(id) FROM organizations` | ~0.2 s server, safe |
| PK id-window aggregate (10–100 k rows) | `SELECT COUNT(*), SUM(identifier IS NULL) FROM organizations WHERE id BETWEEN 8000000 AND 8099999` | 1–2 s, safe; the workhorse |
| index distinct-walk | `SELECT MIN(profile) FROM notices WHERE profile > '<prev>'` iterated | ~0.2 s/step; enumerated all 23 profiles in 23 calls |
| identity-index range probe | `SELECT COUNT(*) FROM organizations WHERE country='HR' AND identifier_kind='vat' AND identifier LIKE 'HRB%'` | <1 s even for 10⁴-row slices; several combined per statement as scalar subqueries |
| per-org mention aggregate | `(SELECT COUNT(DISTINCT m.name) FROM organization_mentions m WHERE m.organization_id=o.id)` via `organization_mentions_org` | <1 s |
| indexed ORDER BY … LIMIT | `SELECT published_at FROM tender_versions ORDER BY published_at LIMIT 12` | <1 s both directions |
| window join | `notice_dates d JOIN notices n ON n.id=d.notice_id WHERE d.notice_id BETWEEN …` (10 k notices) | 1–2 s |

**Dead shapes** (408'd once, never retried, work abandoned server-side — each is a standing cost, see below):

1. `SELECT COUNT(*) FROM (SELECT 1 FROM organizations WHERE country IS NULL AND identifier IS NOT NULL LIMIT 300000)` — `IS NULL` does not seek the identity index; scans.
2. `SELECT (SELECT COUNT(*) FROM notices WHERE id BETWEEN 100000 AND 109999 AND published_at IS NULL AND parse_state='parsed') AS w100k, …` (three 10 k windows in one statement) — three cold wide-row range scans in one 10 s budget.
3. `SELECT LENGTH(code), COUNT(*) FROM notice_classifications WHERE notice_id BETWEEN 22000000 AND 22009999 AND scheme='cpv' GROUP BY 1` — that region of `notice_classifications` is colder/denser than its `notice_dates` sibling (which ran in 2 s).
4–6. Three final calls (`LENGTH` over a `tender_version_classifications` window; a two-subquery `(scheme,code)` point-count; a 100 k org-window `GROUP BY country`) 408'd *in a row* on shapes of a class that had been running in 2–3 s — read as **box degradation from the earlier stacked 408 scans**, not as evidence about the shapes. Querying was stopped at that point; `/health` answered 200 in 0.5 ms throughout and load stayed under 2 on 4 vCPUs (the issue-17 isolation held). Lesson for the next pass, stated bluntly: even disciplined never-retry accumulates uninterruptible server-side work; budget ~3 expendable shape-probes per session, not 6.

Era anchoring: `organizations.id`, `tenders.id` interleave eras (a rebuild groups by procedure — a 20 k-tender window at id 4 M spans 2008–2020 **[verified sample]**), but `notices.id` maps to eras cleanly. All per-era statements below anchor on notice-id windows: 2 M/3 M/4 M = text (2002/2007/2010), 18 M = ted-export-r208 (2014), 20 M = r209+r208 (2018), 22 M = r209 (2021), 24 M = eForms sdk-1.6…1.11 mix (2024), 26 M = DÖE sdk-0.1 backfill (2022), 27.0–27.6 M = live 2025–26 (eforms-de-2.1 + sdk-0.1), ids ≤ 10 k = earliest live 2026 (sdk-1.12/13/14) **[verified sample]**. Scale context: MAX(id) = 25,316,065 organizations / 27,625,886 notices / 8,140,205 tenders **[verified aggregate]**.

---

## 1. Deliverable A — organization identifier landscape (gap 2 / issue 168)

### 1.1 Identifier presence per era (mention level)

One 10 k-notice window per era; `organization_mentions(raw_identifier, scheme, country)` **[verified sample]**:

| era (window) | mentions | with raw id | with scheme attr | NULL country |
|---|---|---|---|---|
| text 2002 (2.00 M) | **0** | — | — | — |
| text 2010 (4.00 M) | **0** | — | — | — |
| r208 2014 (18.00 M) | 36,289 | 3,606 (9.9 %) | 3,606 (`national`, parser-assigned) | 5,906 (16.3 %) |
| r209 2018 (20.00 M) | 41,848 | 10,030 (24.0 %) | same as raw | 442 (1.1 %) |
| r209 2021 (22.00 M) | 48,767 | 23,366 (47.9 %) | same | 703 (1.4 %) |
| eForms 2024 (24.00 M) | 36,484 | 36,329 (**99.6 %**) | 7,550 (20.7 %) | 0 |
| DÖE sdk-0.1 2022 (26.00 M) | 4,276 | **0** | 0 | **4,276 (100 %)** |
| live 2026 (ids ≤ 10 k) | 29,387 | 29,356 (99.9 %) | 4,388 (14.9 %) | 0 |
| live mixed (27.60 M) | 2,641 | 2,169 (82.1 %) | 108 | 430 |

Refinements over ted-legacy-mapping §6's 2019-package numbers: the id fill **climbs steadily through the r209 era** (10 % → 24 % → 48 % between 2014 and 2021); eForms BT-501 is effectively always present (but see 1.4 — presence ≠ quality); **the text era contributes zero mentions** (the text parser emits no organization sections — buyer names live in `notice_texts` only), and **DÖE sdk-0.1 contributes zero identifiers and zero countries** (the sdk-0.1 party mapping deliberately carries no official id — `project.rs` `sdk01` arm, [code-verified]).

### 1.2 What the organizations table is actually made of

Fifteen 100 k-row PK windows across the id space (1.5 M rows sampled of 25.3 M): **provisional (NULL-identifier) share 96.5 %**, per-window range 85.1–100 % (lowest at the id-space edges — 86.7 % at ids 1–100 k, 85.1 % at 25.2 M; a pure-provisional plateau of 99–100 % through the 1 M–24 M middle) **[verified sample]**. Identified orgs split roughly 1:6 vat:national in most windows.

Read this with the grain in mind: provisional profiles are one-per-mention *by design* (never merged), identified profiles absorb many mentions each — so 96.5 % of **rows** is not 96.5 % of **mentions**, and is not itself a defect. It does mean the Organizations feature's headline surface is dominated by single-mention shards, and any `v_organizations` listing is ~24 M provisional rows deep.

### 1.3 Scheme attributes (eForms BT-501 `schemeName`)

2024 window + live window GROUP BY, ~10 k schemed mentions **[verified sample]**: `002` (5,415 + 2,026 — dominant, undocumented in-corpus; appears on LTU/NOR/FIN/SWE mentions), `eu`/`EU` (914+255 / 1,625+107), `ID_PLATAFORMA` (506/359 — Spanish platform-internal id), `NIF` (417/261), `national` (25), `OTROS` (12), `ID_UTE_TEMP_PLATAFORMA` (6/4 — Spanish *temporary consortium* platform id: an identifier that by construction names a per-procedure entity, poison for cross-procedure merging), `CompanyId` (1). The legacy eras have no published scheme at all (the `national` value there is parser-assigned, [code-verified]).

So even where a scheme attribute exists, it is a small minority (≈15–21 %), its vocabulary is publisher-idiosyncratic, and at least two of its values name platform-scoped rather than register-scoped identity. The current normalizer ignores it entirely ([code-verified]).

### 1.4 Format landscape, from raw samples

80-row DISTINCT samples per era window **[verified sample]**; the recurring format families:

- **DE**: `HRB 22388`, `HRB  740925` (double space), `HRB9263`, `HRB 132229 B` (court suffix); VAT `DE254473301`, `DE 268660541`; Behörden ids `992-80317-72`, `09-0991208-91`; platform UUIDs (`ac82707a-…`, scheme `EU`); junk `unbekannt`, `t:084193460`, placeholder `DE123456789`; DÖE routing ids `0204:994-DOEVD-83` (scheme `002`).
- **FR**: SIRET with spaces `219 200 227 00011`, bare SIREN `443928874`.
- **PL**: `NIP 5252465327`, formatted `815-16-33-492`, `KRS 0000445351`, `REGON: 01751157500050`, and **compound ids**: `REGON: 271578690, NIP: 6340132519` — two registers in one field; NUTS code as id (`PL823`), bare `PL`.
- **FI**: the same entity population publishes both Y-tunnus (`0103288-0`) and VAT (`FI26426691`) forms — one register, two lexical schemes, guaranteed false splits.
- **BE**: `0500.927.497_518607` (KBO + establishment suffix).
- **RO**: `RO42283735` vs bare `4394927` vs suffixed `4394927_3`.
- **NO**: org numbers with spaces `974 761 076`, one truncated 8-digit (`999 601 39`), and a plain name (`Norad`) under scheme `002`.
- **ES**: NIF `A28517308`, and a court's full name as its identifier (`Tribunal Català de Contractes del Sector Públic`).
- Legacy NULL-country text junk: `Deutschland` as a DE national id; Bulgarian municipalities (`Obshtina Kameno`) under `country=DE` with Bulgarian registry numbers; Cyrillic transliterations of German companies carrying `рег. № HRB 7032` inline.

### 1.5 The merge key's measured failure classes

Merge key ([code-verified], `project.rs::normalise_identifier` + `canonical.rs::resolve_one_mention`): strip non-alphanumerics, uppercase; reject <4 chars / no digit / all-zero digits / single-repeated-char; then **if the first two chars are letters and a digit follows → `kind='vat'`, `country` := those two letters**; else `kind='national'`, `country` := mention country as published. Merge on exact `(country, identifier_kind, identifier)`.

**(a) Register prefixes minted as countries — issue 86 is a class, not a bug.** Every alphabetic-prefixed register string becomes a "VAT" id whose country is the register prefix. Index-range counts over the whole table **[verified aggregate]**:

| stored (country, pattern) | real meaning | org rows |
|---|---|---|
| HR, `HRB%` | German Handelsregister B | **11,238** |
| HR, `HRA%` | German Handelsregister A | 2,104 |
| NI, `NIP%` | Polish tax id | **18,093** |
| RE, `REGON%` | Polish statistical register | 9,389 |
| US, `UST%` | German USt-IdNr spelled out | 4,858 |
| FN, any | Austrian Firmenbuch (FN is not ISO) | 1,999 |
| KR, `KRS%` | Polish court register | 1,023 |
| SI, `SIRET%`/`SIREN%` | French registers | 587 |
| CV, `CVR%` | Danish CVR | 360 |
| VR, any | German Vereinsregister (VR not ISO) | 332 |
| CI, `CIF%` | Spanish CIF | 258 |
| NI, `NIF%` | ES/PT/FR tax id | 252 |
| OI, `OIB%` | Croatian OIB | 152 |
| CU, `CUI%` | Romanian CUI | 19 |
| IC/DI, `ICO%`/`DIC%` | Czech registers | 4 |

Enumerated classes alone: **≈50,700 org rows** carrying a false country — and this enumeration is a *lower bound* (only prefixes I thought to probe; the full inventory needs the snapshot machine). Sharpest single number: `country='HR' AND identifier_kind='vat'` totals 13,812 rows, of which 13,342 = **96.6 % are German HRB/HRA companies** — the Croatian org register is, as stored, mostly Germany **[verified aggregate]**.

**(b) Cross-register collisions inside a fake country.** HRB numbers are unique per *Amtsgericht*, not nationally; the key `(HR, vat, HRB<n>)` merges across courts. Sample of 15 four-digit-HRB orgs: `HRB0270` = 9 mentions, **3 distinct company names**; `HRB0558` = 3/2 **[verified sample]**.

**(c) Placeholder VATs pass the plausibility gate and merge strangers.** `(DE, vat, DE123456789)` is one canonical org with **282 mentions across 138 distinct names** **[verified aggregate]** — currently named "Land Baden-Württemberg, vertreten durch das Ministerium für Kultus, Jugend und Sport". Also live: `DE999999999` (34 mentions/20 names), `DE111111111` (10/6), `DE987654321` (4/2), `DE000000001` (4/2). The gate's filler checks (all-zero digits, repeated single char) do not catch ascending/descending runs or checksum-invalid ids.

**(d) NULL-country national ids merge globally.** [code-verified]: `normalise_identifier`'s own comment says an unknown-country national id "stays separate", but the code returns `country: None` and the resolver's HashMap key `(None, "national", value)` is one bucket — all countryless mentions sharing a digit string worldwide collapse into one org. With 16 % of r208 mentions countryless, this is a real population; its size could not be measured (the `IS NULL` count is dead shape #1) — snapshot item.

**(e) False splits.** Same entity, many keys: leading-zero variants (`0000968` and `00000968` are two DE org rows both named "Magistrat der Stadt Rüsselsheim am Main" **[verified sample]**); FI Y-tunnus vs FI-VAT forms; RO bare vs `RO`-prefixed CUI; spaced vs unspaced SIRET; and the alpha-2/alpha-3 country split (an id seen under `DE` in 2021 and under `DEU` in 2024 never merges — see 1.6).

**(f) Procurement-reference junk as national ids.** The low end of the `(DE, national)` index range is file references, not org ids: `0004DLG20160323`, `00120192018WIN007`, `00375515099KRANUNDTORTECHNIK` (Fraunhofer appears under ≥3 such "ids") **[verified sample]**.

### 1.6 Country coding (extends issue 48)

Mention-level, one window per era **[verified sample]**: r208 2014 is alpha-2 **with TED's non-ISO `UK`** (1,902) and `GR` (447), 16 % NULL, plus a long third-country tail (`RE`, `GP`, `GF`, `MQ`, `US`, `ZM`, `NC`, `SM`…). eForms 2024 is **alpha-3 dominant** (`DEU` 8,654, `FRA` 4,336 …) **with an alpha-2 minority in the same window** (`LU` 33, `BE` 26, `FR` 12, `DE` 5, `UK` 4) — the mix is publisher-level, not just era-level. On the organizations table, bounded counts **[verified aggregate, lower bounds]**: `DEU` ≥ 150,000, `FRA` ≥ 150,000, `ESP` 69,263, `ROU` 10,032, `GBR` 3,298 — three orders of magnitude beyond issue 48's July snapshot (DEU 1,173). Country is thus a three-vocabulary column (alpha-2 incl. `UK`/`GR`, alpha-3, fake register prefixes) plus NULL.

---

## 2. Deliverable B — value-domain first pass (gap 6 / issue 171)

### 2.1 Amounts

**Storage precondition** [known, issues 144/131]: integer cents, >2 fraction digits quarantined never rounded (so no sub-cent values exist *in* the DB — the sub-cent question is a quarantine-composition question, ~40–50 % of the 3,571 eForms residue); text era emits no amounts. **[verified sample]** confirms both: text and sdk-0.1 windows contain zero `notice_amounts` rows.

**Currency inventory** (window GROUP BYs; global DISTINCT is not index-servable — lower bound): 26 currencies observed. Per era **[verified sample]**:

- r208 2014: EUR, PLN, RON, GBP, CZK, BGN, **HRK**, MKD, NOK, **LTL**, HUF, DKK, SEK, CHF, USD, ISK, **MTL** — pre-euro-transition currencies live where they belong (LTL 2014 ✓, HRK ≤2023 ✓), **except MTL (Maltese lira, retired 2008) in a 2014 window ×4** — either old-contract references or source junk.
- r209 2021: + **CNY, MMK, AED, ALL**, MDL — third-country currencies enter via TED's external notices.
- eForms 2024: + KRW, and **`OP_DATPRO` ×3 (zero cents)** — not ISO 4217 but the Publications Office *placeholder codelist value* leaking into a currency attribute.
- live 2026: EUR + **USN** (ISO "US dollar, next day" funds code — valid but absurd in procurement; also 3× in the earliest live window).

**Negatives** [known: 17,738 canonical `cents<0`, all source-published, re-specced to allow `result_value` €1–100; 51 negative ceilings open in issue 132; 407 ≥€10k rows in bids/awarded, issue 136]. New, sample-level: in the 2024 window **all 99 negative `notice_amounts` rows are exactly −100 cents (−€1.00)**, spread over BT-720-Tender (35), BT-710/711-LotResult (29/23), BT-161 (7), BT-709/660/118/1118 (≤2 each) **[verified sample]** — direct support for issue 132's sentinel hypothesis (question 2): **−1.00 is a publisher sentinel, not a value**. Legacy windows (2014/2021) contain zero negatives — negatives are an eForms-era phenomenon at the parse layer.

**Zeros** — the bigger and previously unnamed population: EUR zero-cent amounts are 2.7 % (2014), 3.4 % (2024), and **8.9 % (live 2026)** of EUR rows in their windows **[verified sample]**. Zero is being used as "no value" — anyone averaging awarded values gets silently dragged down. Not covered by any existing issue.

**Magnitudes**: window maxima look like fat-tail-plausible-to-absurd: EUR max €69.5 bn (2021 window), GBP max £80 bn (2021), CZK 10 bn CZK, canonical window max €33.3 bn **[verified sample]**. Nothing above €100 bn in sampled windows; the −€323 M record-holder [known, issue 136] remains the documented extreme. A per-currency absolute ceiling plus issue 134's ratio rule would have flagged every specimen seen here.

### 2.2 Dates

**Canonical publication axis is clean at the extremes**: `tender_versions.published_at` spans 1993-01-02 (`55278-1992` — 1992-numbered notices in the first 1993 OJ issues, correct) to 2026-08-07, both ends genuine **[verified aggregate]** (indexed ORDER BY, both directions).

**Deadline before publication** (deadline field vs causing notice's `published_at`, per era window) **[verified sample]**:

| era | deadlines | before publication |
|---|---|---|
| text 2007 | 4,781 | 15 (0.31 %) |
| r208 2014 (`TED-DT_DATE_FOR_SUBMISSION`/`RECEIPT_LIMIT_DATE`) | 9,104 | 16 (0.18 %) |
| r209 2021 (`TED-DATE_RECEIPT_TENDERS`) | 3,921 | 8 (0.20 %) |
| eForms 2024 (`BT-131(d)-Lot/-Part/-Procedure`) | 17,380 | 44 (0.25 %) |

A stable ~0.2–0.3 % across 20 years — a source-side background rate, not an era defect. Zero deadlines beyond publication+10 y in any window.

> **Re-taken 2026-09-07 (issue 370).** Both halves of that line are window artefacts. The 0.2–0.3 % is a WITHIN-NOTICE rate; compared at ROW level (head `published_at` against a deadline carried forward from an earlier version) it is 37.6 %, and after an award notice that ordering is expected rather than noise. And the corpus does hold deadlines beyond publication+10 y — 3005-07-06, 2999-12-31 twice, 2924-04-15, 2205-11-18 — so "zero in any window" was green because of the windows chosen. Re-take with the queries in issue 366, not by re-reading this line.

**Placeholder/impossible instants** (eForms 2024 window, 99,558 date rows): min = **year 0000** (−62,135,600,400 s), max = **2100-01-01**; 55 rows before 1980, 32 rows ≥ 2038 **[verified sample]**. Specimens: `OPT-999` = 1969-12-31 ×36 and 0000-12-31 ×7 (epoch-zero and year-zero), `BT-132(t)-Lot` 1970-01-01, `BT-536-Lot` **1899-12-31** ×2 (spreadsheet-epoch), `BT-537-Lot` 2099-12-30 ×6 / 2050-12-30 ×5 / a 2038–2045 sprinkle (some genuine long frameworks, some junk). Legacy 2021 window is far cleaner: min 2008, max 2048, 2 rows ≥ 2038, none before 1980.

**Offsets** (eForms 2024 window): dominated by 0/+60/+120/+180 as expected, but **−600 ×159, −630 ×159 (UTC−10:30!), −720 ×1** on European notices **[verified sample]** — either genuine overseas-territory publishers or offset junk; small, worth one specimen read on the snapshot machine.

**Dispatch vs publication**: `dispatched_at > published_at` = 0 of 10,000 in the 2024 window (the 22 NULLs are its quarantined rows) **[verified sample]** — issue 18's ordering holds.

**`notices.published_at` NULL on parsed rows**: 7,417 of the first 10,000 notices (earliest live-2026 ingests) are `parse_state='parsed'` with `published_at IS NULL` **[verified sample]** — rows processed before the issue-18 stamp and never re-stamped. The canonical layer is unaffected (`tender_versions.published_at` is NOT NULL and fed at fold time), but every public-SQL query that filters `notices.published_at` silently drops these. Extent unknown (probe was dead shape #2) — snapshot item.

### 2.3 Coded values

- **CPV**: essentially clean — exactly **one** non-digit-leading code in the whole canonical layer (a single `'O'`) **[verified aggregate]** (index range `code >= 'A'`). Low end of the range: `01000000` — **CPV-2003 division codes live alongside CPV-2008** (divisions 01/02 were removed in 2008), so cross-era CPV aggregation crosses a vocabulary boundary (feeds gap 7 / issue 172).
- **NUTS**: both ends of the `(scheme,code)` index are junk classes: numeric `'00'` at the bottom, country codes as pseudo-NUTS at the top (`'ZW'`) **[verified sample of the range ends]** — the legacy third-country convention flows straight into the canonical layer. Counting them 408'd during the degradation window; snapshot item.
- **Currency codes**: all sampled values are 3-letter ISO **except** `OP_DATPRO` (2.1) — one deny-list entry suffices today.
- **Country codes**: see 1.6 — three vocabularies plus NULL plus minted register prefixes.

### 2.4 Side finding (canonical mapping, not data): r208 deadlines may never project

The r208 2014 window carries its submission deadlines as `TED-DT_DATE_FOR_SUBMISSION` (262) and `TED-RECEIPT_LIMIT_DATE` (203) and has **zero** `TED-DATE_RECEIPT_TENDERS` rows — but `project.rs::DATES` maps only the latter to `submission_deadline` ([code-verified]; `canonical_name` is exact/stem match). If nothing else re-routes those field ids, r208-era Tenders have parsed-but-unprojected deadlines and any "deadline fill per era" metric under-reports 2011–2016. One bounded canonical-side check would confirm; filing-worthy either way.

---

## 3. Validation-rule catalog seed

For ingestion (I = at parse/projection) and for the standing gate (G = measured invariant). Parameters marked ⚙ need corpus-wide numbers from the snapshot machine before enforcement.

**Identifiers / organizations (feeds B8):**
1. (I) **Register-prefix table before VAT sniffing**: strip/classify known prefixes (HRB, HRA, VR, GnR, PR, FN, KRS, NIP, REGON, CUI, CIF, NIF, OIB, CVR, IČO, DIČ, SIREN, SIRET, UST/USt) into `(register_scheme, number, register_scope?)`; only then treat a leading ISO-3166 alpha-2 + digits as VAT. A two-letter prefix that is not ISO alpha-2 (or `EL`/`XI`) must never mint a country.
2. (I) **Country vocabulary**: one canonical coding (alpha-2), mapping alpha-3 and TED `UK`→`GB`, `GR`→`GR`-canonical; reject/flag anything outside the mapped set. (Issue 48's fix, now with measured scale: ≥300 k alpha-3 rows.)
3. (I) **Placeholder-id gate**: reject ids whose digit run is a repeated/ascending/descending sequence (`123456789`, `987654321`, `000000001`); checksum-validate where the scheme has one (DE VAT, SIREN/SIRET Luhn, PL NIP, FI Y-tunnus, CZ IČO, ES NIF letter) — checksum failure ⇒ provisional, never a merge key.
4. (I) **Compound-field splitter**: `REGON: X, NIP: Y` shapes yield two candidate ids, not one 20-digit hash; ids containing ≥4 consecutive letters after normalization (`…KRANUNDTORTECHNIK`) ⇒ provisional.
5. (I) **Unknown-country national ids never merge** — make the code do what its comment already claims; `(NULL, national, v)` must not be a shared bucket.
6. (G) ⚙ **Merge-key hygiene invariants**: zero orgs with non-ISO `country`; distinct-name count per org ≤ ⚙ (the DE123456789 class, today's max known: 138); per-country VAT-checksum failure rate ≤ ⚙.
7. (I, policy) Platform-scoped schemes (`ID_PLATAFORMA`, `ID_UTE_TEMP_PLATAFORMA`, bare UUIDs) merge only within their platform scope — a UTE (consortium) id must never merge across procedures.

**Amounts:**
8. (I) **−100-cents sentinel**: any amount of exactly −1.00 (any currency) ⇒ store as published, flag `sentinel`, exclude from canonical value columns (resolves issue 132 Q2 for the −100 stratum; the €420 k/€11.8 M negatives among the 51 stay real suspects).
9. (G) `estimated_value ≥ 0`, `framework_maximum ≥ 0` (already agreed via 132) + ⚙ per-currency absolute ceiling (nothing sampled exceeds €100 bn; the ratio invariant of issue 134 sits in Tier B above this floor).
10. (G) ⚙ **Zero-amount rate per field per era** as a *reported* (not alerting) metric; docs/API note that 3–9 % of EUR amounts are zero-as-no-value.
11. (I) **Currency ∈ ISO 4217 alpha** minus deny-list {`OP_DATPRO`}; flag funds codes (`USN`) and currencies retired before the notice's publication date (MTL post-2008, HRK post-2023, LTL post-2015).

**Dates:**
12. (I) **Placeholder-instant null-out list**: {year ≤ 1900, 1969-12-31, 1970-01-01, 1899-12-31, year-0000} on any date field ⇒ absent, flag `placeholder` (the 55-per-100 k class).
13. (G) Deadline ≥ causing notice's publication, as a *reported rate* with the measured ~0.2–0.3 % source background as baseline — alert only on excursion, not existence.
14. (G) Date plausibility window: deadlines/openings within [publication, publication+10 y] (0 violations in 4 eras — cheap and currently vacuously green); `duration_end` exempt but flagged > 2060.
15. (G) `dispatched_at ≤ published_at` (holds at 100 % in sample).

**Codes:**
16. (G) CPV `^\d{8}$` (one known violation corpus-wide — enforceable now); era-tag CPV-2003 vs 2008 divisions rather than validating against one list (issue 172).
17. (I) NUTS: `^[A-Z]{2}[0-9A-Z]{0,3}$`; classify `00` and bare third-country codes as `pseudo-nuts`, excluded from region aggregation.

## 4. Requires the snapshot machine (no compliant bounded path)

1. **Full fake-country inventory**: `GROUP BY country, identifier_kind` over all 25.3 M organizations — my §1.5(a) table enumerates only guessed prefixes; the tail (and totals for `DEU`/`FRA` beyond the 150 k probe caps) is unmeasured.
2. **NULL-country identified orgs**: count + mention-name diversity (dead shape #1). This is the quantification of failure class (d).
3. **False-merge/false-split rates** — the issue-168 headline numbers: distribution of distinct mention-names per identified org (false-merge league table needs a global aggregate+sort); duplicate-name clusters among provisional orgs and across lexical id variants (false splits).
4. **Global currency inventory** (`DISTINCT currency` on three amount tables — no index; the 26 observed are a floor), and corpus-wide zero/negative/ceiling counts per field × era × currency.
5. **`notices.published_at IS NULL` extent** over the parsed corpus (dead shape #2), plus its region map.
6. **NUTS/CPV vocabulary census** with per-code counts per era (the `(scheme,code)` index supports walking, but count-per-code shapes were 408-class by session end) — feeds issue 172 directly.
7. **Offset-junk specimens** (the −630 ×159 class) and BT-537 far-future split (genuine frameworks vs junk) — needs row reads with notice context.
8. **Canonical r208 deadline coverage** (§2.4) — one join over era-scoped versions; bounded in principle but the box owes no more 10 s slots to this session.
9. Anything shaped like `GROUP BY` over `organization_mentions` (40.9 M) or `tender_version_amounts` without an id window.

## 5. What this pass does and does not establish

It establishes the *shape catalogue* — which pathologies exist, with at least one verified specimen or window rate each, and which merge-key failure classes are live at what minimum scale (≈50.7 k fake-country rows, 138-name placeholder merges, three-vocabulary country column, −1.00 sentinels, 3–9 % zero amounts, 0.2–0.3 % impossible deadlines, year-0000/2100 placeholder dates). It does **not** establish corpus totals for any sampled rate (windows are ingestion-order samples, not random), does not rank the failure classes by mention-weighted impact, and does not settle issue 132's 51 ceilings (the −100 sentinel finding covers the sentinel stratum only). Era attribution rides on the notice-id map in §0, which is itself windowed — the sparse 5–16 M id region was probed at five points, not walked.

---

*Prod contact: ~100 bounded `/v1/sql` SELECTs (6 hit the 10 s cap and were abandoned, never retried; 4 429s), one `/health` probe, no writes, no admin ops. Query pacing ≥14 s after the hourly limiter engaged. Session ended deliberately after three consecutive 408s on previously-fast shapes — treat stacked-408 degradation as a first-class cost in the next bounded pass.*
