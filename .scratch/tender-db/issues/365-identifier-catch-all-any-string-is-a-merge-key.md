# 365 — any ≥4-character alphanumeric string containing a digit becomes an Organization merge key: field labels, phone numbers, notice numbers, department names

Status: ready-for-agent — **UNITS 1 AND 2 SHIPPED, DEPLOYED AND VERIFIED ON PROD 2026-09-09
(`6da7320`)**, standing stock dissolved and re-censused the same firing; every "done when" bullet
for these two units is met. See "Units 1+2 DONE" and "The dissolve, verified". Units 3 (letter-run,
22,796 rows), 4 (carry the publisher's scheme into the normaliser and apply the denial list at E0)
and 5 (the 1.4M nameless class, an owner decision) remain. **UNIT 3 ANSWERED AND SHIPPED
2026-09-09 (`330c7ca`)** — the answer was "do not wire the class"; see "Unit 3 ANSWERED".
Kind: defect (organization layer — identifier admission); units 1-3 are prevention + the
328/345 repair path, unit 5 is a decision
Relates to: 300 (the gate — its exemplar sheet already classifies the phone class
must-CONDEMN and sizes letter-run at 27,781 rows), 312 (the 32-hex class, measured and
deliberately NOT condemned), 328/359/363 (label prefixes), 234/351/261 (the
identifier-less arms), 329 (duplicate identity triples), 357 (whose campaign artifact
already diagnosed the BT-501 class per-row)

## Observed (verified 2026-09-07 on prod)

**A schema field name is a live merge key for six countries.** `SELECT country, identifier_kind, identifier, COUNT(*) FROM organizations WHERE identifier='BT501ORGANIZATIONCOMPANY' GROUP BY …` → exactly six rows, one each in ES, FR, GR, IE, IT, SE, all `kind='national'`, all `provisional=0`: ES 22550176 "Tribunal Català de Contractes del Sector Públic" (13 mentions), IT 22663622 "COMUNE DI PESARO" (12), GR 22786143 "www.eagrwgr.GR" (2), FR 22794601 "France Télévisions" (3), SE 23341692 "Getinge Sverige AB" (1), IE 23561726 "Companies Registration Office" (1) — 32 mentions of six unrelated bodies on one key. The mention row: `organization_id=22786143` → `raw_identifier 'BT-501-Organization-Company'`, `scheme` NULL, and one of its two mentions has *name* `'BT-500-Organization-Company'`. `BT-501-Organization-Company` is this codebase's own `ORG_IDENTIFIER_FIELD` (`crates/ingest/src/project.rs:699`).

**Bare years and round numbers too.** `identifier IN ('2022','1000')` → 12 rows, six countries each, every one `provisional=0`. `1000` keys Universität Stuttgart (DE), Tribunal Administratif de Lille (FR), Macedonian Railways (MK), e-Zamówienia (PL), a Sardinian comune union (IT), Waterford City and County Council (IE). `2022` keys KELAG (AT), Università di Perugia (IT), OTN Implants B.V. (NL), rectorat de Martinique (FR), CALITOM (FR), a Greek prison.

**A department name and two phone numbers, all canonical.** Landeshauptstadt München: org 4310313 (DE, national, `DIREKTORIUMHAIIVERGABESTELLE1ABT4`) with `raw_identifier 'Direktorium - HA II - Vergabestelle 1, Abt. 4'`, scheme `national`, **one mention**, `provisional=0`; orgs 22524465 / 23243086 keyed `T08923324526` / `T089233726026` (`raw_identifier 't:08923324526'`, scheme NULL). The same name-prefix seek shows the class is broad at this one buyer: `08923396211`, `LEITWEGID09162000ZRE100000009` / `LEITEGID…` / `LEITWEGID0916200040` (German e-invoicing ROUTING ids plus a typo), `96162000ZRE100000009` vs `09162000ZRE100000009` (same Leitweg-ID off by a digit), `2023S198622496` (a TED OJS number), `HRB93017` vs `MNCHENHRB93107`.

**Deutsche Bahn AG, four junk classes.** `name_norm='deutsche bahn ag'` → 37 rows, 35 with an identifier, all `provisional=0`: DB procurement references (`14TEI10487`, `15TEI19266`, `16FEI21202`, `17FEI28567LOS2`, `18GEI32660`), TED OJS numbers (`2018S150345644` = OJ 2018/S 150-345644, `2018S025054890`, `2017S204421475`), 32-hex platform GUIDs (`9E044BF7C3874DED9B31EAC475402535` and five more), an LEI, and `HRB50000USTIDNRDE811569869`. Mention 8055796: `raw_identifier '14TEI10487'`, `scheme 'national'`. (At the wider `deutsche bahn a*` scope there are 2,178 rows, of which only 56 carry an identifier — the rest are identifier-less name variants, i.e. unit 5's class, not this one.)

**The other side of the same admission rule.** `SELECT COUNT(*), SUM(name=''), SUM(country IS NULL), SUM(identifier IS NOT NULL) FROM organizations WHERE provisional=1` → **5,703,677 / 1,415,301 / 2,578,274 / 0**. The trailing zero is load-bearing: `provisional` is *exactly* `identifier IS NULL`, with no exceptions in 5.7M rows. Split: nameless and country-less 309,203; nameless with a country 1,106,098; named with no country 2,269,071; named with country 2,019,305.

## Why, exactly

- **The catch-all.** `normalise_identifier_with`'s last arm is unconditional: `let national = || Identifier { country, kind: "national", value }` — `crates/ingest/src/project.rs:4790` — reached by anything surviving four cheap shape filters at `crates/ingest/src/project.rs:4722-4730` (≥4 ASCII alphanumerics, at least one digit, digits not all zero, not one repeated character).
- **The publisher's scheme is captured and then dropped.** `mention.scheme.clone_from(scheme)` at `crates/ingest/src/project.rs:3143-3145`, but the call is `normalise_identifier(raw, m.country.as_deref())` (`crates/ingest/src/project.rs:3161`) and `scheme` is not a parameter of `normalise_identifier_with` (`crates/ingest/src/project.rs:4681`). So `kind` is inferred from the value's own shape, and nothing in the pipeline can express "this scheme is a notice number or a routing id, never a merge key".
- **`condemns` reads four of the eight rules `census` computes.** `crates/ingest/src/idgate.rs:217-223`: `c.lexicon || c.sequence || c.short_vat || (c.checksum == Fail && hard_scheme(...))`. `census` already computes `phone` (`crates/ingest/src/idgate.rs:88`, via `phone_shaped` at :161 — `t`/`T` + optional colon + a 0-led 9-12 digit run), `letter_run` (`crates/ingest/src/idgate.rs:81`, `letter_run_after_prefix(...) >= 4`, defined at :311-327), `hex_hash` and `short_numeric`. `condemns` consults none of them. The struct's own doc comments say why: "Census-only" and "Census-only until the weekly report sizes the class" (`crates/ingest/src/idgate.rs:56-60`).
- **The rule that would have caught the field name exists twice, and both are inert.** `letter_run_after_prefix("BT501ORGANIZATIONCOMPANY")` = 19 ("ORGANIZATIONCOMPANY"), far past the ≥4 threshold — computed on every identifier, never read. And `lexicon_hit` already knows this exact field: `if value == "BT501" { return true }` at `crates/ingest/src/idgate.rs:264` — but as an **equality**, so the bare field id is condemned while the full field *name*, which is what publishers actually paste, walks straight past. A prefix/containment test on the same entry catches all six rows.
- **`2022`/`1000` are condemned by nothing**: the zero-padding rule needs a zero run before a ≤2-digit tail (`crates/ingest/src/idgate.rs:268-273`) and `suspicious_digit_run` needs ≥5 digits (`crates/ingest/src/idgate.rs:286-289`).
- **There is no third state.** `gated()` (`crates/ingest/src/project.rs:4811`) maps a condemned id to `None`; the mention takes the identifier-less path and the row gets `identifier NULL`. "Not condemned" is identically "live merge key", bound at `crates/store/src/canonical.rs:7873` (`org_of.get(&key)`) and written by the `provisional = 0` INSERT at `crates/store/src/canonical.rs:8137-8142`. So refusing a value cannot mean "keep it, don't merge on it" today — that state has to be built or the value goes to NULL.
- **The enforcement decision was already taken and not wired.** `.scratch/tender-db/issues/300-exemplars.md:170` classifies org 660's `t:04131153308` must-CONDEMN because that phone number FUSES Vergabekammer Niedersachsen (~17k mentions) with Die Vergabekammern des Bundes (708) — two different review bodies under one switchboard — and line 303 records the remedy "phone numbers → NULL the id". `300-org-fuzzy-matching-design.md:91` defers letter-run: "rule 4 (letter-run) stays census-only pending composition", class sized 27,781 rows. Gate v2.1 shipped the two FOLDS from that read (Greek confusables, RO sub-unit suffix) and left the two CONDEMN classes unwired. The routing-scheme denial ("GLN/IPA/DIR3/OIN/Leitweg are location/office/routing scoped: never merge keys", design:827-828) is unimplemented and, as written, guards only E1/E2 auto-merge — not the E0 exact-triple bind at canonical.rs:7873.
- **Unit 5's separate mechanism.** Both identifier-less reuse arms require a non-empty name: `let scope = (!name_norm.is_empty()).then_some(()).and(country.clone())` (`crates/store/src/canonical.rs:8197`, issue 234) and `} else if !name_norm.is_empty() && country.is_none()` (`crates/store/src/canonical.rs:8240`, issue 351). A nameless mention therefore falls through to the unconditional mint at `crates/store/src/canonical.rs:8333-8344` and creates a brand-new row *every time, forever* — the pre-234 behaviour, preserved deliberately (234: "nameless rows are distinct unknown parties"), with nothing bounding or ever collapsing the class. Issue 259's nested-org repair only rescues nameless rows with a single named child.


## Units 1+2 DONE (2026-09-09, `6da7320`)

Three classes `census` already computed and `condemns` never read are now condemning:
`phone`, the eForms field-name lexicon, and a new `bare_four_digit`.

### Unit 1's premise was worth re-testing, and the first metric said the wrong thing

The field's own doc argued to keep `phone` census-only: the review chambers publish a
switchboard consistently, so it keys a body more often than it fuses two. That is a real
argument — it is exactly the reasoning that (correctly) spared the hex class in issue 312 — so
it was measured rather than overridden.

**The obvious metric agreed with the doc and was wrong.** Rows-per-distinct-value over
`id <= 3000000` reads 68 rows / 68 values = **1.0**, which is the "harmless, doing the linking"
signature. But that measure detects prevented SPLITS: after a fusion the bad key still holds
exactly one row, so a switchboard is invisible to it. The measure that exposes fusion is name
diversity:

| phone-keyed canonical orgs (`id <= 3000000`) | 68 |
| carrying ≥2 distinct mention names | **47 (69 %)** vs 14.7 % corpus baseline |
| carrying ≥6 | **24 (35 %)** vs 1.1 % baseline |
| most distinct names on one row | **264** |
| mentions riding those 68 rows | 154,671 |

A row with 264 names is a switchboard, not an organization. Same method as 312, opposite
answer — and the difference is entirely which question you ask.

### Unit 2, both halves

**The field NAME, not just the id.** `lexicon_hit` knew `BT501` as an *equality*, so the bare
field id was refused while `BT-501-Organization-Company` — normalised to
`BT501ORGANIZATIONCOMPANY`, and the form publishers actually paste — walked past. Now a narrow
shape: `BT`/`OPT`/`OPP`, at most four digits, then either nothing or a run of ≥4 letters.
`BT93017425` and `OPTIMA2020` stay identifiers.

**Bare four digits.** `2022` and `1000` each key six unrelated bodies in six countries; over
`id <= 3000000`, 93 such orgs with 37 (40 %) carrying ≥2 names against the 14.7 % baseline,
worst 71. The rule does not rest on that sample: 10,000 possible values cannot discriminate
between 5.7M organizations, which is precisely what `short_vat` already says about a short VAT
tail.

### Two things the tests caught that review would not have

1. **Folding the 4-digit rule into `lexicon_hit` silently stole a reported class.**
   `short_numeric` excludes lexicon hits to avoid double-counting, so `8477` stopped being
   short-numeric and the weekly report's count would have shifted with no explanation. A RED
   PRE-EXISTING test caught it. The rule now has its own census field and the two classes are
   disjoint (`short_numeric` is 5-digit DE only) — confirmed by the census below: 2,296 became
   1,725 + 571.
2. **Three of my own negative controls were invalid.** `12345`, `BT12345678` and `HRB12345` each
   contain an ascending digit run that the pre-existing `sequence` rule condemns anyway, so they
   asserted nothing about the new rules. Replaced with non-sequential specimens.

### The dissolve reconciles exactly

Post-deploy census, then the dry plan:

```
gate census: 53 lexicon, 0 sequence, 22827 letter-run, 0 short-vat, 276962 hex-hash,
             3655 phone-id, 1725 short-numeric, 4611 bare-4-digit, 3349 compound
placeholder dissolve DRY RUN: 1108688 identifier-bearing orgs scanned, 8319 condemned,
             8319 dissolved, 0 skipped, 259342 mentions re-resolved
             (27160 fresh provisionals, 232182 reused), 78556 tenders touched
```

**53 + 3,655 + 4,611 = 8,319** — the condemned set is exactly the three wired classes, with
nothing unexpected swept in. That reconciliation is why the wet run was safe to make: a total
alone would not have shown whether a rule over-reached. `letter_run` (22,827) and `hex_hash`
(276,962) stay census-only and untouched, as units 3 and issue 312 respectively require.

## The dissolve, verified (2026-09-09, jobs 835-837)

The wet run matched the dry plan on every count but two, and both differences are dry-run
artifacts rather than surprises:

| | dry | wet |
| --- | --- | --- |
| identifier-bearing orgs scanned | 1,108,688 | 1,108,688 |
| condemned by the gate | 8,319 | 8,319 |
| dissolved / skipped | 8,319 / 0 | 8,319 / 0 |
| mentions re-resolved | 259,342 | 259,342 |
| — of those, fresh provisionals | 27,160 | **4,982** |
| — of those, reused | 232,182 | **254,360** |
| winner-row duplicates removed | 0 | **3** |
| tenders touched | 78,556 | 78,556 |

The fresh/reused split moves because a dry pass writes nothing, so every mention that *would*
mint a provisional row counts as fresh — it cannot see that an earlier mention in the same run
already created the row it would reuse. Wet, those rows exist and get reused, so only **4,982**
new org rows appeared instead of 27,160 (the total is identical either way). The 3 duplicate
winner rows are the same effect: they only become duplicates once the dissolve actually
re-points them. Both differences are in the harmless direction, and worth stating rather than
glossing, because "the wet run differed from its plan" would otherwise read as a problem.

Then `project` (job 836) re-derived exactly the stamped set: *579 notices → 45 tenders, 45
written*.

### The re-census reconciles exactly

```
before: 53 lexicon, 0 sequence, 22827 letter-run, 0 short-vat, 276962 hex-hash,
        3655 phone-id, 1725 short-numeric, 4611 bare-4-digit, 3349 compound
after:   0 lexicon, 0 sequence, 22796 letter-run, 0 short-vat, 276962 hex-hash,
            0 phone-id, 1725 short-numeric,    0 bare-4-digit, 3349 compound
```

All three condemned classes stand at **0**. Identifier-bearing orgs fell 1,108,688 → 1,100,369,
which is **−8,319 exactly**.

The classes that were meant to be left alone were left alone, which is the check that the rules
did not over-reach: **hex-hash is unchanged to the row at 276,962** (issue 312's
deliberately-spared class), `short_numeric` unchanged at 1,725, `compound` unchanged at 3,349.
`letter_run` moved only 22,827 → 22,796; those 31 rows carried a condemned class *as well*, so
they left with the dissolve — the class itself is untouched and unit 3 still owes its
composition read.

### The "done when" bullets, checked individually

- `identifier = 'BT501ORGANIZATIONCOMPANY'` → **0 rows**; `identifier IN ('2022','1000')` → **0
  rows**; the phone shape → 0 (census).
- **The raw values survived on their mentions**, which is what makes this a refusal rather than a
  deletion: `raw_identifier LIKE 'BT-501%'` still returns them, including one publisher's
  `BT-501- PANTRY AND CORKSCREW` — a field id and a company name pasted together, caught because
  the normalised form carries a ≥4-letter tail.
- **Org 660, the switchboard fusion, is gone** — the row no longer exists and holds 0 mentions.
  The bodies it had fused now stand separately (`Vergabekammer des Bundes` id 255, distinct from
  the regional chambers), keyed by their own platform GUIDs rather than by a shared telephone
  number. That is precisely what `300-exemplars.md:170` asked for in classifying this
  must-CONDEMN.

One number moved the *wrong* way and is worth flagging rather than hiding: the worst multi-name
org went from 700 distinct names to **701** (org 2660, already the report's own exemplar), because
a dissolved mention re-resolved onto it. Corpus multi-name counts fell only modestly
(162,266 → 161,591 at ≥2 names), since dissolving mints name-keyed rows that can themselves carry
name variants. So this fixed a fusion class; it is not a general cure for multi-name orgs, which
is 329/351 territory.


## Unit 3 ANSWERED (2026-09-09, `330c7ca`) — the class does not condemn; two prefixes inside it do

Issue 300 parked `letter_run` as "census-only pending composition" with the class sized at
27,781 rows. This is that composition read, and wiring the class would have been wrong in
**both** directions simultaneously.

### The aggregate says "mildly elevated" and is misleading

Over `id <= 3000000`: 513 rows, **25.7 %** carrying ≥2 distinct mention names against the
14.7 % baseline, 6.0 % at ≥6 against 1.1 %. Elevated — but nothing like the phone class's
69 %/35 %, and those 513 rows carry **523,601 mentions**. On the aggregate alone this is closer
to the hex class that issue 312 deliberately spared than to anything worth condemning.

### Reading the worst rows shows it is three different things

| what | example | verdict |
| --- | --- | --- |
| routing / reporting references | `LEITWEGID08A986640` (43 names), `BERICHTSEINHEITID00002636` (47) | **condemn** |
| real registry ids wearing a LABEL | `CVRNR…`, `SIRET…`, `HANDELSREGISTERHRB…`, `REGISTRIERUNGSNUMMER…` | **strip, don't refuse** (issue 374) |
| genuine high-volume keys | org 28 `0204994DOEVD83` — **370,791 mentions** over 15 names | **leave alone** |

Eight of the top 18 by name diversity are Leitweg-IDs and four are Berichtseinheit-IDs, which
is what makes the class look bad in aggregate. Condemning it wholesale would have discarded a
370,791-mention key and thrown away four label-prefixed registry numbers that the 359/363 strip
vocabulary should be *recovering*. So `letter_run` stays census-only — now with the reason
recorded instead of a TODO.

### What justifies the two condemns is the worst row, not the percentage

A Leitweg-ID addresses **where an electronic invoice is delivered**; a Berichtseinheit-ID names
a **statistical reporting bucket**. Neither is a party, and shared-service arrangements put many
bodies behind one of each. Corpus-wide:

| | orgs | ≥2 names | max names | mentions |
| --- | --- | --- | --- | --- |
| `LEITWEGID` | 766 | 221 (28.9 %) | **43** | 43,715 |
| `BERICHTSEINHEITID` | 611 | 193 (31.6 %) | **47** | 28,787 |

28.9 % is unremarkable; **43 distinct organization names on one invoice-routing address** is
not. `300-org-fuzzy-matching-design.md:827` had already ruled these out ("location/office/
routing scoped: never merge keys") — the rule was simply never wired.

Matched as prefix FAMILIES because publishers spell them many ways: `LEITWEG` covers
LEITWEGID/LEITWEGEID/LEITWEGSID/LEITWEGLD/LEITWEG, `BERICHT` covers
BERICHTSEINHEITID/BERICHTEINHEITID/BERICHTSID. That is worth 44 extra rows over exact-string
matching (1,421 condemned vs the 1,377 the two exact globs measure). `LEITID`/`LEITWERTID`
(3 rows) are deliberately left unmatched — stretching the prefix further would be guessing.

### Dissolved and verified the same firing

Wet run (job 839) matched its dry plan (838) on every count but the fresh/reused provisional
split — 863 fresh wet against 11,507 dry, identical 73,021 total — the same dry-run artifact
explained above. Only **863** new org rows, so these mentions almost all landed on rows that
already existed under the bodies' names. `project` (840) re-derived exactly the stamped 6
notices → 2 tenders.

```
before: 0 lexicon, 0 sequence, 22796 letter-run, 276962 hex-hash, 0 phone-id,
        1725 short-numeric, 0 bare-4-digit,    — routing-scope, 3349 compound
after:  0 lexicon, 0 sequence, 21375 letter-run, 276962 hex-hash, 0 phone-id,
        1725 short-numeric, 0 bare-4-digit,    0 routing-scope, 3349 compound
```

`routing_scope` → 0; identifier-bearing orgs 1,100,369 → 1,098,948, **−1,421 exactly**.
`letter_run` fell 22,796 → 21,375, which is the same 1,421 (a routing id also carries a letter
run) — so the 21,375 rows the decision means to keep, including org 28's key and the label-
prefixed registry numbers, are untouched. **hex-hash is unchanged to the row at 276,962 for
the second dissolve running**, which is the standing proof neither rule has crept.

### One pre-existing test moved rather than bent

`a_word_that_starts_with_a_country_code_does_not_mint_that_country` used
`BERICHTSEINHEITID00002636` as a "starts with BE" specimen. This rule gates it away before
country election runs, which is a different question from the one that test asks, so the
fixture was removed with the reason inline — BE stays covered twice by
`BERLINCHARLOTTENBURG93627` and a `BE2A…` GUID. Bending the new rule to keep an incidental
fixture alive would have been the wrong repair.


## Units

1. Wire `c.phone` into `condemns` (the decision is already recorded at 300-exemplars.md:170), re-measure the class, and run the 328/345 repair path so standing rows catch up. Verify org 660 splits back into two review bodies.
2. `lexicon_hit`: turn the `BT501` equality (`crates/ingest/src/idgate.rs:264`) into a prefix/containment test over the eForms BT/OPT/OPP field vocabulary, and add a bare-4-digit rule (a year or round number is not a register number). Both classes are measured above; both are cheap.
3. Size then wire `letter_run` — the "pending composition" read 300 deferred on: top N of the 27,781 rows, classified, then enable or record why not.
4. Carry the publisher's declared scheme into `normalise_identifier_with` (`crates/ingest/src/project.rs:4681`, called at :3161) so a routing/notice-number scheme can be refused as a merge key with the value kept on the mention — and apply the design's denial list at **E0** (canonical.rs:7873), not only at E1/E2.
5. **The nameless class** (independently grabbable): 1,415,301 provisional rows, one per mention by construction, unbounded. Decide (owner's call, on this issue) whether a nameless mention should mint at all, attach to the notice's own party section, or share one sentinel row per notice. 234's evidence argued for minting; nothing has re-derived that against 1.4M rows. Add the count to the weekly report either way.

## Done when

- `BT501ORGANIZATIONCOMPANY`, `t:…` phone ids and `2022`/`1000` are refused as merge keys with the raw values still on their mentions, pinned by tests;
- the repair job has run and the six BT501 rows stand as six identifier-less rows;
- the weekly org report carries the letter-run and nameless-row counts, so neither class can grow unobserved again.

*One issue because:* the field name, the phone numbers, the department name, the Leitweg ids, the OJS numbers and the DB reference numbers are all the same line of code — the `national()` catch-all with the scheme thrown away and `condemns` reading half of what `census` computes. Unit 5 is the same admission rule seen from the other side (no key at all), which is why it lives here rather than under 234/351.
