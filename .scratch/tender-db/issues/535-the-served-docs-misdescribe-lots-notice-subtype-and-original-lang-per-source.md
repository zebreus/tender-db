# 535 — the served docs misdescribe `lots`, `notice_subtype` and `original_lang` per source: an empty value is the source's silence, and no surface says so

Status: ready-for-agent — 535 units 1-3 BUILT 2026-10-14 (the three fields rewritten on `/v1/openapi.json`, the `/v1/sql/schema` notes and `/docs`; the retired-claims walk now reads the schema notes; a positive-half test; the 2026-09-15 board rows point here; two-reviewer round applied; focused tests green; see "Unit 1 — built" to "Review round"). Next: gate + deploy with the 522/524 batch, then read the Verify.
Kind: docs (the served contract): `crates/app/data/openapi.json` (`Tender` :969/:970/:974, `TenderDetail.lots` :986,
the `/v1/lots` description :156), the `/v1/sql/schema` column notes (`COLUMN_NOTES`, `crates/app/src/v1/sql.rs:1121-1135`),
and `/docs` "Coverage varies by era" (`crates/app/src/v1/docs.rs:690`). No data change.
Relates to:
- 235 (RESOLVED-DEPLOYED): kept the legacy document types report-side (`DOC_TYPE_MARKERS`, `data_quality.rs:343`)
  instead of projecting them into `notice_subtype`. Moving them stays open only as a simplification inside a corpus
  rebuild. It decided the data, not the served text. Not reopened.
- 344 (done): fixed the DE 1.x language; sdk-0.1 decided 2026-09-07 (owner): no profile default `DEU`, because 93% of
  those notices publish no language element. Covers the data. The OpenAPI text still never says what NULL means.
- 340 (CLOSED 2026-09-03): built the `original_lang` leg. Its SQL note says "NULL where the notice did not say"; the
  OpenAPI text does not.
- 391 (DONE 2026-09-18): made `Tender` declare these three fields. Those are the descriptions corrected here.
- 342 (DONE 2026-10-01): FTS writes its UK form code, or failing that the OCDS release tag, into `OPP-070-notice`
  (`crates/ingest/src/fts/parse.rs:95-102`). The SQL note "eForms notice subtype id" contradicts that.
- 368 (DONE 2026-09-18): the 2026-09-07 review routed the DÖE subtype NULL to its `kind_of` unit. Data, not docs.
- 462, 479 (done): cross-era classification goes through `kind` and procedure type, not `notice_subtype`.
- 370 (DONE 2026-09-18): its detector `no_served_surface_repeats_a_retired_claim` (`docs.rs:1069`) is unit 2's guard.
  It walks `/docs` and the OpenAPI only, not the `/v1/sql/schema` notes.
- `.scratch/tender-db/api-dq-review-2026-09-15.md:235/246/254`: the legacy/sdk-0.1 `notice_subtype` rows and "r208
  undivided contracts project zero Lots" sit on its "Refuted — do not re-file" list with the reason "not recorded".
  They are not decisions. This issue records the reason (unit 3 points them here).
- `.scratch/tender-db/external-review-2026-09-07.md:24`: the doe "language and subtype null" row. Its `original_lang`
  "0 of 1,613 sampled" still reproduces on its own id band (below). It is not stale.

## Verify

    curl -s --max-time 30 https://tenders.zebreus.click/v1/openapi.json https://tenders.zebreus.click/v1/sql/schema | jq -r 'if .components then (.components.schemas.Tender.properties | .notice_subtype.description, .lots.description, .original_lang.description) else (.tables[] | select(.name=="v_tenders") | .columns[] | select(.name=="notice_subtype") | .note) end'

Four lines: OpenAPI `Tender.notice_subtype`, `.lots`, `.original_lang`, then the `v_tenders.notice_subtype` SQL note
(the same `*` note is served on `v_tender_notices` and `tender_versions`). Covers unit 1. Unit 2 is a test and unit 3
a board edit; check both in place.
- **done**: line 1 names who fills `notice_subtype` (eForms OPP-070, DE 1.x via its alias, FTS form code or release
  tag), says it is NULL for pre-eForms TED and DÖE sdk-0.1, and points at where those eras' document type is served.
  Line 2 states the single-lot rule per format (0 for the text era, internal-ojs 2008 without a LOTS section and
  r2.0.8 without a contract-notice lot annex; 1 for r2.0.9 and eForms; FTS 0 until a release lists `tender.lots`). Line 3 says what NULL means. Line 4 no longer reads "eForms notice subtype id".
- **open (2026-10-14)**: `The source's own notice-subtype code for the current version.` / `How many Lots the current
  version publishes; 0 is normal for a single-lot procurement.` / `Language the current version was originally
  published in, uppercase ISO 639-2/T (ADR-0013).` / `eForms notice subtype id, e.g. '16' (contract notice), '29'
  (contract award).`

## Repro

Public GETs, 2026-10-14:

    for id in 3871260 8414257 8414014 8554010 1598198 7954604; do curl -s https://tenders.zebreus.click/v1/tenders/$id | jq -c '[.id, .source, .lots, .notice_subtype, .original_lang]'; done

| tender | format | lots | notice_subtype | original_lang |
| --- | --- | --- | --- | --- |
| 3871260 | internal-ojs 2008 (129171-2008), LOTS sections | 24 | null | ENG |
| 8414257 | r2.0.8 published in 2018 (R2.0.8.S03, notice 19738076) | 0 | null | SPA |
| 8414014 | r2.0.9 standalone F14 corrigendum (093419-2018) | 0 | null | ENG |
| 8554010 | TED eForms, PIN-only subtype 1 | 0 | `1` | ITA |
| 1598198 | DÖE sdk-0.1 | 1 | null | null |
| 7954604 | FTS | 4 | `tenderUpdate` | ENG |

- `/v1/notices/19722698/content` (8414014's notice): PROCEDURE has `TED-FORM` F14 and `TED-TD_DOCUMENT_TYPE` 2.
- `/v1/notices/26668618/content` (1598198's notice): PROCEDURE has `SDK01-NoticeTypeCode` `cn-standard` (list
  `competition`) and no language field.

Bounded `/v1/sql` reads by the verifier and refute (not re-run here). One 1,000-tender PK window per era, tenders ⨝
current `tender_versions` ⨝ `notices`, grouped by profile; columns are tenders / lots=0 / subtype NULL / lang NULL:
- 8155000–8155999: text 1000 / 1000 / 1000 / 0.
- 8289000–8289999: r208 997 / 815 / 997 / 0; r209 3 / 0 / 3 / 0.
- 8414000–8414999: r209 866 / 50 / 866 / 0; r208 134 / 99 / 134 / 0.
- 1598000–1598999: eforms-sdk-0.1 971 / 0 / 971 / 971.
- 1000000–1000999 (the 2026-09-07 review's band): 32 sdk-0.1 current versions, lang NULL 0, subtype NULL 32.
- Systematic PK sample, ids k×20011 for k=1..432 (429 exist): see "What was found".
- 408'd, not retried, do not repeat: `SELECT max(id), min(id) FROM tenders`; a `notice_codes` `field_id IN (...)` join.

Public week reads, lots=0 of 200 tenders: 2005: 200; 2008-05-13..20: 180; 2012: 170; 2015: 184; 2018: 82; TED eForms
2025-06-02: 14, all subtype 1 or 3 (subtype and lang NULL 0); FTS 2025-06-02: 0. `source=doe` weeks with subtype and
lang both NULL: 188/200 (2023-03-01..03; the other 12 are eForms-DE E3/16/E4 with `DEU`) and 198/200 (2025-06-02).

## What was found

In every case the notice publishes nothing to put in the field. The data is correct. The defect is that the served
descriptions do not say which sources leave each field empty, and one of them is wrong. Three outside reviews
(2026-09-07, 2026-09-15, 2026-10-14) each read some of these fields as "missing structure".

**`notice_subtype`.** It is filled from `OPP-070-notice` (`crates/ingest/src/project.rs:1410`), DE 1.x via the alias
at :1251. FTS writes its UK form code or `tag.join("+")` there (`fts/parse.rs:99`): `award+contract` (7954588),
`awardUpdate+contractUpdate` (7954597), `tenderUpdate` (7954604), `tender` (7954674). Nothing aliases a legacy TD
code or the sdk-0.1 type. So it is NULL by construction for the text era, internal-ojs 2008, r2.0.8, r2.0.9 and DÖE
sdk-0.1, and filled for TED eForms, eForms-DE and FTS.
- Corpus share (refute, systematic sample): NULL on 367 of 429 (85.5%): text 141/141, r2.0.9 111/111, r2.0.8 79/79,
  sdk-0.1 36/36. Every eForms and FTS row is filled. No eForms-DE row landed in the sample.
- The era's own document type is served, just not in this column: `TED-TD_DOCUMENT_TYPE` (r2.0.8/r2.0.9), `TXT-TD`
  (text), `TED-NAT_NOTICE` (internal-ojs), `SDK01-NoticeTypeCode` (sdk-0.1), in `/v1/notices/{id}/content` and the
  `notice_codes` table on `/v1/sql`. 235 reads them report-side.
- Served text: the OpenAPI's "the source's own notice-subtype code" makes a reader expect TED's TD code or DÖE's
  `cn-standard` here. The SQL note's "eForms notice subtype id" is wrong for FTS.

**`lots`.** It counts the Lot sections the current version publishes. Whether a single-lot procurement serves 0 or
1 depends on the format, not the date:
- Text era (1993–2010): no lot structure at all (`docs/research/ted-legacy-mapping.md:505`). Always 0.
- internal-ojs 2008: Lots only from a `LOTS` section (`internal_ojs.rs:98`); 20 of 200 in the May 2008 week have one.
- r2.0.8 (`ted-export-r208`): Lots only from the lot annexes (`F02_ANNEX_B`, `F05_ANNEX_B`, the PIN lot annex) and
  the PIN object section (`r209/rules.rs:692-695`). An undivided contract notice has none and serves 0. This format
  is still published in 2018 (8414257).
- r2.0.9: `OBJECT_DESCR` is a Lot section (`r209/rules.rs:339`), published for undivided contracts too, so a
  single-lot tender serves 1. The lot-less ones are standalone F14 corrigenda: refute sampled 10 of the 50 in
  8414000–8414999, all F14 (nine TD 2, one TD 1).
- eForms (TED, DE, DÖE sdk-0.1): a lot section is always published; the lot-less ones are PIN-only subtypes 1 and 3
  (8554010 has no LOT or PAR section).
- FTS: 0 of 200 lot-less in one week.

A lot-less tender has no row on `/v1/lots` by definition. It is on `/v1/tenders` with the same filters. Neither the
`/v1/lots` description nor `TenderDetail.lots` ("Lot count.") says so.

**Why `lots = 0` is not a defect.** The 2026-09-15 refuted row never recorded this. The source publishes no lot
section for these notices. Synthesising one for an undivided r2.0.8 contract would serve a Lot nobody published, and
it would need a corpus re-projection. No ADR or issue had decided against it before; this is the reason, recorded.

**`original_lang`.** It is NULL only where the causing notice states no language. eForms-DE 1.x and 2.x carry `DEU`
(344's fix). The DÖE sdk-0.1 share depends on the id band: 971/971 NULL at 1,598,000–1,598,999, 0/32 at
1,000,000–1,000,999, and 33/36 (about 92%) in the systematic sample. That is consistent with 344's "93% publish no
language element". Nobody has read why the 32 in the 1.0M band are filled. In the systematic sample, 35 of 429 are
NULL, 33 of them sdk-0.1. The SQL note says what NULL means but lists only "TED LG_ORIG, eForms BT-702, text-era OL".
The OpenAPI text says nothing about NULL.

### What the external review got wrong

- **"Missing structure."** In each case the source publishes nothing for the field.
- **"Before 2009, lots = 0 on 100%."** That is true of the text era only. internal-ojs in May 2008 has lots on 20 of
  200 (3871260 has 24).
- **"2009–15, 88%."** The figure is in range (82% in an r2.0.8 window; 85% and 92% in 2012 and 2015 weeks), but the
  cause is r2.0.8's lot-annex rule, and that format runs into 2018.
- **"They never appear in `/v1/lots`."** That is true by definition. These tenders are on `/v1/tenders`.
- **"notice_subtype empty on 83%."** That holds (85.5% in the sample), and it is empty by construction.
- **"`cn-standard` is a notice type the column drops."** It is sdk-0.1's BT-02 notice type, which spans several
  eForms subtypes. Mapping it into `notice_subtype` would be an inference.
- **"original_lang empty on German-portal tenders."** Only DÖE sdk-0.1 is empty, and not uniformly. eForms-DE serves
  `DEU`.

Nor re-file the first verification draft's errors: it called the 2026-09-07 review's "0 of 1,613" stale (it
reproduces on its band); it said a single-lot tender serves "0 before about 2016, 1 after" (the axis is the format);
it said three reviews "each read these three fields" (each read some); and it cited the 2026-09-15 refuted row as a
decision.

## Proposed units

1. **Rewrite the served descriptions (S, docs only).**
   - OpenAPI: `Tender.notice_subtype`, `Tender.lots`, `Tender.original_lang`, `TenderDetail.lots`, and the `/v1/lots`
     description. Lot-less tenders (text era, internal-ojs and r2.0.8 undivided contracts, standalone F14
     corrigenda, eForms PIN-only) have no rows there; find them on `/v1/tenders`.
   - `sql.rs` `COLUMN_NOTES`: the `notice_subtype` note, and add sdk-0.1 to the `original_lang` note's NULL case.
   - `/docs` "Coverage varies by era": one bullet carrying the per-format rules above.
   - The `notice_subtype` text points at the four document-type field ids, `/v1/notices/{id}/content` and
     `notice_codes`, so the docs say where to look, not only that the column is NULL.
   - State rules, not shares. A served percentage is a photograph nothing re-derives (the 370 class).
   - Before writing "NULL means the notice stated no language", read 2–3 of the 32 filled sdk-0.1 tenders in
     1,000,000–1,000,999 through `/v1/notices/{id}/content`. If their language is not the notice's own, the
     version-level text must say where it comes from.
   - Gate with `ops/check.sh` together with unit 2, deploy with the next batch, then read the Verify.
2. **Guard the corrected text (S, same gate as unit 1).**
   - Add `eForms notice subtype id` and `0 is normal for a single-lot procurement` to `RETIRED` in
     `no_served_surface_repeats_a_retired_claim`.
   - That test walks `PAGE` and `SPEC` only. The wrong subtype note lives in `COLUMN_NOTES`, served on
     `/v1/sql/schema`. Extend the walk to the notes, or the new entry passes without checking anything.
   - Add the positive half, in the shape of `the_docs_say_what_provisional_and_a_408_actually_mean`: the subtype text
     names FTS and sdk-0.1, the lots text names r2.0.9, the language text says what NULL means. A rewrite that just
     deletes the sentence must fail.
3. **Point the board record here (XS).** In `api-dq-review-2026-09-15.md`, replace "not recorded" on dq-results-winners
   (:235), dq-lots (:246) and the `notice_subtype` half of dq-text-era (:254) with a pointer to 535. Leave that row's
   `kind` half alone; it is not examined here. Leave `external-review-2026-09-07.md:24` as it is, since it reproduces.

Out of scope (decided, or worth weighing only inside a corpus rebuild): projecting the legacy TD / NAT_NOTICE /
sdk-0.1 types into `notice_subtype` (235) or mapping `cn-standard` into it; synthesising a Lot for undivided
text-era, internal-ojs or r2.0.8 contracts (reason above); stamping `DEU` on sdk-0.1 (344, owner decision).

## Unit 1 — built (2026-10-14)

**The `original_lang` read the unit asked for first.** A bounded read (`sq.sh`, the PK range 1,000,000–1,000,999
joined to the current version and its notice, `LIMIT 40`) returned the 32 sdk-0.1 current versions. All 32 have
`DEU`. Three were read through `/v1/notices/{id}/content`: 26891435 (tender 1000005), 26168402 (1000028) and
26487223 (1000195). Each carries its own `SDK01-NoticeLanguageCode` `DEU` on PROCEDURE. So these are the notice's
own language, and "NULL means the notice states no language" holds. In the code, a version's language is its
causing notice's own (`NoticeState.original_lang` → `TenderVersion`) and is not carried forward. One rule the
issue did not list: FTS writes the release `language`, or `en` when the release has none (`fts/parse.rs:88`), so
an FTS row is never NULL. The text says so.

**What is served now:**
- `Tender.notice_subtype`, the `notice_subtype` note on `/v1/sql/schema` (the `*` note, so also on
  `v_tender_notices` and `tender_versions`) and the `/docs` bullet say the same thing. The column holds the causing
  notice's subtype: the eForms subtype (OPP-070; TED eForms, eForms-DE), or on FTS the UK form code, else the
  release tags joined with `+`. It is NULL for text, internal-ojs, r2.0.8, r2.0.9 and DÖE sdk-0.1, which publish
  no subtype. Those eras' own type is in `/v1/notices/{id}/content` and `notice_codes`, under
  `TED-TD_DOCUMENT_TYPE`, `TXT-TD`, `TED-NAT_NOTICE` or `SDK01-NoticeTypeCode`. Checked: only `fts/parse.rs`, the
  eForms parser and the DE 1.x alias (`project.rs`) produce `OPP-070-notice`.
- `Tender.lots` states the per-format rule. 1 for a single-lot procurement on r2.0.9 (`OBJECT_DESCR` is a Lot,
  `r209/rules.rs`) and on eForms (DÖE sdk-0.1 included). 0 on the text era, and for an undivided contract notice
  or award on internal-ojs without a `LOTS` section or r2.0.8 without a lot annex. FTS serves 0 until a release
  lists a lot (corrected in the review round below). Also lot-less: a standalone r2.0.9 F14 corrigendum, and an
  eForms prior-information notice with no lot or part section. Lot-less Tenders have no row on `/v1/lots` and are
  on `/v1/tenders`. The count is written as what the current version "carries": lots carry forward by key across
  the chain (`project.rs`, `previous.map(|p| p.lots.clone())`), and it includes LotsGroups and Parts (the count is
  over `tender_version_lots`). The eForms PIN case is written as a rule ("no lot or part section"), not as
  subtypes 1 and 3, which are only what was observed. `TenderDetail.lots` ("Lot count.") and the `/v1/lots`
  description name the lot-less cases and point at `/v1/tenders`.
- `Tender.original_lang` and the SQL note say what NULL means ("the notice states no language", many DÖE sdk-0.1
  notices) and how FTS fills it. The SQL note's source list adds the DE 1.x and sdk-0.1 `NoticeLanguageCode`.
- `/docs` → "Coverage varies by era" has one new bullet with all three rules.

No percentage is served anywhere.

## Unit 2 — built (2026-10-14)

- RETIRED in `no_served_surface_repeats_a_retired_claim` gains `eForms notice subtype id` and
  `0 is normal for a single-lot procurement`.
- The walk now reads `/v1/sql/schema` as a third surface, through `sql::served_schema_notes()` (`#[cfg(test)]`):
  every `TABLE_NOTES`, `COLUMN_NOTES`, `EPOCH_NOTE` and `SCHEMA_NOTES` text plus `schema_limit_notes`. Without it
  the new subtype entry would check nothing. `sql::column_note` is now `pub(super)` so the positive half can read
  one column's served note.
- Positive half: `the_docs_say_which_sources_leave_lots_subtype_and_language_empty`. It reads each field's own
  text, because "FTS" and "sdk-0.1" occur elsewhere on every surface. Those texts are the OpenAPI field, the
  `v_tenders` schema note, and the one `/docs` coverage bullet that names `notice_subtype`. The subtype texts must
  name FTS, sdk-0.1, r2.0.9, `notice_codes` and the four field ids. The lots texts must name r2.0.9, r2.0.8,
  internal-ojs, the text era, F14, `/v1/lots` and `/v1/tenders`. `/v1/lots` and `TenderDetail.lots` must name
  r2.0.8 and F14 and say "no row". The language texts must say "null where the/that notice states no language" and
  name sdk-0.1.

## Unit 3 — built (2026-10-14)

In `.scratch/tender-db/api-dq-review-2026-09-15.md`, "not recorded" is replaced with a pointer to 535 on three
rows: dq-results-winners `notice_subtype`, dq-lots "r208 undivided contracts project zero Lots", and the
`notice_subtype` half of dq-text-era PIN-only. That row's `kind` half still reads "not recorded".
`external-review-2026-09-07.md:24` is unchanged.

## Review round (2026-10-14, two reviewers)

The unit 1 text above was corrected where the reviewers found it overclaimed:
- **FTS `lots`:** FTS does not always serve 1. `fts/parse.rs` makes a Lot only from `tender.lots[]`, so an FTS
  Tender serves 0 until one of its releases lists a lot (8781543, 005947-2022, an `award+contract` notice, serves
  0). The texts now say so, and FTS is on the lot-less lists of `Tender.lots`, `TenderDetail.lots` and `/v1/lots`.
- **internal-ojs and r2.0.8 PINs:** a supplies/services prior-information notice's object section is a Lot there
  (`OBJECT_SUPPLY_SERVICE_PRIOR_INFORMATION`, `r209/rules.rs`; 3871629 serves 1). The 0 rule is now stated for
  an undivided contract notice or award only.
- **`notice_subtype`:** an FTS release with neither a form code nor tags writes no `OPP-070-notice`, so NULL is
  possible there too; the texts say so. The legacy NULL is now described as the column's scope (issue 235 keeps
  their document type out of it), not "the source's silence". The `/docs` bullet notes once that r2.0.8 includes
  the R2.0.7 files filed under that profile.
- **`original_lang`:** "uppercase ISO 639-2/T" is qualified on `Tender.original_lang` and the SQL note: a code
  `normalize_lang` does not recognise passes through uppercased (e.g. `FR NL` on some bilingual Belgian TED
  notices). The dq-multilingual board row is still "not recorded".
- **Tests:** the retired walk squashes whitespace on every surface. The positive test pins a phrase per corrected
  sentence (e.g. "on FTS, the UK form code", "which publish no subtype", "sdk-0.1 notices do not") rather than
  tokens another sentence also carries, after dropping `<code>` tags and backticks so the `/docs` and OpenAPI
  spellings match. Deleting any one corrected sentence now fails it. Focused tests green.
