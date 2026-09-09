# 365 — any ≥4-character alphanumeric string containing a digit becomes an Organization merge key: field labels, phone numbers, notice numbers, department names

Status: ready-for-agent — **UNITS 1 AND 2 SHIPPED AND DEPLOYED 2026-09-09 (`6da7320`)**, standing
stock dissolved the same firing; see "Units 1+2 DONE" below. Units 3 (letter-run, now 22,827 rows),
4 (carry the publisher's scheme) and 5 (the nameless class) remain.
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
