# TED legacy-era mapping — R2.0.8 / R2.0.9 into the eForms-shaped canonical model

Research date: 2026-07-19. Companion to `ted-access-channels.md` (era ladder)
and `eforms-data-model.md` (the canonical target shape).

Method: hands-on dissection of real daily packages on the VPS
(`root@zebreus.click`, samples under `/opt/tender-db/samples/x/`), using a
small element-tree dumper (`/opt/tender-db/samples/tree.py`) plus grep/python
sweeps over whole packages; official XSDs and mapping material mirrored to
`/opt/tender-db/ted-xsd/` from the Publications Office archive page
(<https://op.europa.eu/en/web/eu-vocabularies/e-procurement/tedschemas>).
Packages dissected: 2011-01-04, 2014-01-02, 2015-01-02, 2016-01-02,
2017-01-03, 2019-01-02 (primary R2.0.9 corpus, 1 529 notices), 2024-06-28
(eForms side of the era boundary), plus text-era 1993/2005/2008 files.

Confidence markers: **[measured]** = computed from real packages (commands
reproducible on the VPS); **[docs]** = official documentation; **[inferred]**
= my conclusion from evidence.

---

## Summary

- **Linkage verdict: yes, legacy notices chain into Tender aggregates**, via
  OJ notice numbers instead of eForms UUIDs: every notice carries exactly one
  optional `REF_NOTICE/NO_DOC_OJS` back-reference in its OP-curated coded
  section, present in 48 % of all notices (2019 package) and ~100 % of
  corrigenda/modifications. Award notices link explicitly in 71–74 %
  [measured]; roughly a third of the unlinked rest are legitimately
  reference-free (award without prior publication), leaving a genuine ~15–20 %
  linkage gap on awards. Chains cross era boundaries *backwards* (2011 notices
  reference 2009/S text-era numbers) but break *forward* at the eForms
  boundary: eForms notices reference by UUID, and legacy OJS numbers appear
  only in free text (2 of 400 sampled 2024 award notices) [measured].
- **Coverage**: the whole canonical core (buyer, title, CPV, NUTS, values,
  deadlines, procedure type, lots, winners, award values, contract dates,
  bid statistics, award criteria, review bodies) is structurally present in
  R2.0.9 with high fill rates (deadline/CPV 100 %, award value 91 %, winner
  91 %). R2.0.8 covers the same core slightly thinner (estimated value 44 %,
  no structured corrigenda). Of the 357 eForms BT ids, roughly half have a
  legacy R2.0.9 source (**unverified estimate** — §5.2); the rest are
  legitimately absent from the legacy schemas. Lossiness also runs the other
  way: OP documents ~23 legacy elements with **no eForms BT at all** (incl.
  `REFERENCE_NUMBER`), so the legacy profiles need a small set of canonical
  fields outside the eForms shape (§5.2).
- **Era policy**: define per-era mapping profiles keyed on the era's own
  schema — "era completeness" = every element/attribute of that era's XSD set
  is mapped-or-ignored — so ADR-0004 quarantine stays exhaustive *within* an
  era without demanding eForms-era semantics of 2012 notices. Concrete
  profiles and checklist sizes in §8.

---

## 1. The legacy container: `TED_EXPORT` (shared by both XML eras)

Every 2011–2024 legacy notice is one XML file, root `TED_EXPORT`, with four
sections [measured on all packages]:

| Section | Content | Mapping stance |
|---|---|---|
| `TECHNICAL_SECTION` | `RECEPTION_ID` (eSender submission id, `18-583998-001`), `DELETION_DATE`, `FORM_LG_LIST`, `COMMENTS` (often "From Convertor") | keep RECEPTION_ID as lineage, ignore rest |
| `LINKS_SECTION` | boilerplate xlink URLs (all point at ted.europa.eu) | explicit ignore rule |
| `CODED_DATA_SECTION` | **OP-curated codes** — the stable backbone, see below | map fully |
| `TRANSLATION_SECTION` | `ML_TITLES/ML_TI_DOC` (title + country + town in all 23–24 EU languages, every era) and `ML_AA_NAMES/AA_NAME` (buyer name, translated) | map — free multilingual titles for search |
| `FORM_SECTION` | the actual standard form, repeated once per language with `CATEGORY="ORIGINAL"` / `"TRANSLATION"` | map ORIGINAL fully; TRANSLATION = same tree, store as language variants |

**Identifiers** [measured]: root attributes `DOC_ID="000001-2019"` (equals
the package filename `{number}_{year}.xml`), `EDITION="2019001"` (OJ S
issue), plus `CODED_DATA_SECTION/NOTICE_DATA/NO_DOC_OJS` in OJ display form
(`2019/S 001-000001`). **Normalization gotcha**: the display form is not
stable across eras — 2011 writes `2011/S 1-000181` (unpadded issue), 2019
writes `2019/S 001-000001`, and `REF_NOTICE` values follow the era of the
*referencing* notice (`2013/S 54-087792`). The canonical join key must be
`(year, publication_number)` parsed out of the string, never the raw string.

**Version/namespace ladder** [measured]:

| Package | Root namespace | `VERSION` attr (on forms) |
|---|---|---|
| 2011-01 | `http://publications.europa.eu/TED_schema/Export` | `R2.0.7.S03.E01` |
| 2014/2015/2016-01 | same | `R2.0.8.S02.E01` |
| 2016-01 already +12 files, 2016-11→2017 | `http://formex.publications.europa.eu/ted/schema/export/R2.0.9.S01.E01` | `R2.0.9.S01.E01` |
| 2018→2024 | `http://publications.europa.eu/resource/schema/ted/R2.0.9/publication` (+ `…/ted/2016/nuts`) | `R2.0.9.S03…S05` |
| Defence forms F16–F19, any year through 2024 | `…/resource/schema/ted/R2.0.8/publication` | `R2.0.8.S04+` |

Three consequences [measured]:

1. **2011–2013 files are actually R2.0.7**, not R2.0.8, and *no R2.0.7 XSD
   is on the OP archive page* (earliest offered: R2.0.8.S03, 2017). Element
   vocabulary is nearly identical (544 shared names of ~610 per package;
   67 names appear in 2011 but not 2014), so the R2.0.8 profile can absorb
   R2.0.7 with a small delta — but the delta must be discovered empirically.
2. **Defence-directive notices never migrated**: a 2019 package contains
   `CONTRACT_AWARD_DEFENCE` files in the R2.0.8 namespace with
   `FORM="18"` side by side with R2.0.9 forms. Era dispatch is per file *and
   per namespace*, not per year.
3. The per-notice URL endpoint (`…/notice/{n}-{y}/xml`) serves yet another
   namespace spelling (`xmlns="ted/R2.0.9.S02/publication"`, relative!)
   [measured]. Parsers must match by local element name within a detected
   profile, not by exact namespace URI.

---

## 2. R2.0.9 hands-on dissection (2016-11 → 2024)

Form mix in the 2019-01-02 package (1 529 notices, file-level counts)
[measured]: F03 558, F02 448, F14 143, F06 49, F05 40, F20 40, F21 38,
F01 20, F15 13, F08 11, F25 3, F12 2, F24/F23/F13 1 each, 10 defence-form
files. So **F02+F03 ≈ 66 %** of the era's volume, and **F14+F20 ≈ 12 %** —
the notices OP's own converter skips (see §5) are not a fringe.

### 2.1 F02 contract notice (dissected: `000001_2019.xml`)

Element tree (ORIGINAL form): `LEGAL_BASIS@VALUE` (CELEX, e.g. `32014L0024`)
→ `CONTRACTING_BODY` (`ADDRESS_CONTRACTING_BODY` with OFFICIALNAME,
NATIONALID?, ADDRESS/TOWN/POSTAL_CODE/COUNTRY@VALUE/NUTS@CODE,
CONTACT_POINT/PHONE/E_MAIL, URL_GENERAL/URL_BUYER; `CA_TYPE@VALUE`,
`CA_ACTIVITY@VALUE`; document-access and participation addresses, often via
`ADDRESS_FURTHER_INFO_IDEM`-style "same as above" flags) → `OBJECT_CONTRACT`
(TITLE/P, REFERENCE_NUMBER, CPV_MAIN/CPV_CODE@CODE, TYPE_CONTRACT@CTYPE,
SHORT_DESCR/P, VAL_ESTIMATED_TOTAL@CURRENCY, LOT_DIVISION|NO_LOT_DIVISION,
one `OBJECT_DESCR@ITEM` per lot with LOT_NO, CPV_ADDITIONAL, NUTS@CODE,
MAIN_SITE, SHORT_DESCR, award criteria `AC/AC_QUALITY/AC_CRITERION+
AC_WEIGHTING`, `AC_PRICE`, or `AC_PROCUREMENT_DOC`, DURATION@TYPE=MONTH|DAY,
renewal/options/variants/EU-funds indicator elements) → `LEFTI` (selection
criteria, largely free text `P`) → `PROCEDURE` (`PT_OPEN`/`PT_RESTRICTED`/…
as *empty marker elements*, GPA indicator, DATE_RECEIPT_TENDERS +
TIME_RECEIPT_TENDERS, DURATION_TENDER_VALID, OPENING_CONDITION) →
`COMPLEMENTARY_INFO` (review body address, dispatch date).

Codes vs free text: codes are attributes (`@CODE`, `@VALUE`, `@CTYPE`,
`@CURRENCY`) whose element *text* is a human-readable label in the form's
language — parse attributes only, labels are redundant [measured]. Booleans
are element-presence pairs (`NO_LOT_DIVISION` vs `LOT_DIVISION`,
`NO_ACCEPTED_VARIANTS`…), procedure type is element choice — both map
cleanly to enums. Free text is `<P>` paragraph sequences.

Dates in R2.0.9 forms are ISO (`2019-02-07`) with separate `HH:MM` time
elements; the coded section duplicates the deadline as
`DT_DATE_FOR_SUBMISSION` (`20190207 11:00`). Values are machine-readable
decimals with `@CURRENCY` (`VAL_TOTAL CURRENCY="EUR">1590482.50`).

### 2.2 F03 award notice (dissected: `000003_2019.xml`, `000492_2019.xml`)

Adds to the F02 shape: `OBJECT_CONTRACT/VAL_TOTAL` (procedure total),
per-lot `OBJECT_DESCR@ITEM`, and one `AWARD_CONTRACT@ITEM` block per award:
CONTRACT_NO, LOT_NO, TITLE, then either `AWARDED_CONTRACT` —
DATE_CONCLUSION_CONTRACT, `TENDERS/NB_TENDERS_RECEIVED` (+ `_SME`,
`_OTHER_EU`, `_NON_EU`, `_EMEANS` variants), `CONTRACTORS/CONTRACTOR/
ADDRESS_CONTRACTOR` (name/NATIONALID?/address/NUTS/SME-flag; repeated for
joint winners under AWARDED_TO_GROUP), `VALUES/VAL_ESTIMATED_TOTAL` +
`VAL_TOTAL` (or VAL_RANGE_TOTAL), subcontracting value/percentage — or
`NO_AWARDED_CONTRACT` with a reason (99 of 558 F03 files have an unawarded
lot [measured]).

Lot linkage inside the notice is positional-by-number: `AWARD_CONTRACT@ITEM`
↔ `LOT_NO` ↔ `OBJECT_DESCR/LOT_NO` (verified aligned on a 7-lot notice
[measured]). There are **no notice-internal entity ids** — no ORG-/LOT-/RES-
registry as in eForms; the importer synthesizes Lot identity from LOT_NO and
OrganizationMentions from inline address blocks (§6).

### 2.3 F14 corrigendum (dissected: `000119_2019.xml`)

Slim header (buyer address block, OBJECT_CONTRACT title/CPV/type) +
`COMPLEMENTARY_INFO/NOTICE_NUMBER_OJ` (the corrected notice — present in
143/143 F14s [measured]) + structured `CHANGES/CHANGE` list: `WHERE`
(SECTION "IV.2.7", LABEL, LOT_NO?) and **typed** OLD_VALUE/NEW_VALUE pairs —
`DATE` (210), `TEXT` (225), `NOTHING` (4), CPV variants also allowed by the
XSD [measured 2019 package]. This is genuinely machine-applicable: the
canonical versioned layer can apply DATE changes (e.g. deadline moves)
mechanically; TEXT changes at minimum version the affected section.
`CHANGE@PUBLICATION` / `NOTHING` mark unpublished-content cases.

### 2.4 F20 modification (dissected: `000128_2019.xml`)

Buyer + object header; `PROCEDURE/NOTICE_NUMBER_OJ` → the original award
publication (37/40 have coded REF_NOTICE [measured]); `AWARD_CONTRACT`
repeating the contract as awarded (CONTRACT_NO, conclusion date, contractors
with NATIONALID, VAL_TOTAL); `MODIFICATIONS_CONTRACT` with
`DESCRIPTION_PROCUREMENT` (CPV/NUTS/duration/values/contractors after
modification) and `INFO_MODIFICATIONS` (SHORT_DESCR, reason element
`UNFORESEEN_CIRCUMSTANCE` | `ADDITIONAL_NEED`, `VALUES/VAL_TOTAL_BEFORE` +
`VAL_TOTAL_AFTER`). Maps directly onto the eForms ContractModification
entity (BT-200/201/202, BT-1501) [inferred, high confidence].

### 2.5 Defence forms (dissected: `001417_2019.xml`, F18)

R2.0.8-style: deeply nested wrapper names
(`FD_CONTRACT_AWARD_DEFENCE/CONTRACTING_AUTHORITY_INFORMATION_CONTRACT_AWARD_DEFENCE/
NAME_ADDRESSES_CONTACT_CONTRACT_AWARD/CA_CE_CONCESSIONAIRE_PROFILE/ORGANISATION/…`),
values as locale-formatted text with the machine value in an attribute
(`<VALUE_COST FMTVAL="1381073714.86">1 381 073 714,86<`), dates as
DAY/MONTH/YEAR child elements, procedure-type + negotiated-without-
publication justifications as nested marker elements (`ANNEX_D/…`). Content
core (buyer, title, CPV, NUTS, total value, winner, previous-publication
block) is all present — same canonical mapping, different element grammar.

### 2.6 Multilingual handling [measured]

- Titles: every notice, every era, carries `ML_TITLES` in all 23–24 EU
  languages (plus TI_CY country, TI_TOWN).
- Forms: exactly **one `CATEGORY="ORIGINAL"` form** per notice (language =
  `NOTICE_DATA/LG_ORIG`); 2019 package has 805 TRANSLATION copies across
  1 529 notices — EU-institution notices are translated into all 24, national
  notices usually ORIGINAL-only.
- The TRANSLATION forms replicate the full element tree with translated
  text; codes identical. Canonical layer: store text fields as
  `(lang, text)` satellites exactly as planned for eForms
  `text-multilingual`, with ORIGINAL flagged authoritative.

---

## 3. Cross-notice linkage — can legacy notices form Tender chains?

**Mechanism** [measured]: two parallel reference carriers:

1. `CODED_DATA_SECTION/NOTICE_DATA/REF_NOTICE/NO_DOC_OJS` — OP-curated,
   always exactly one per notice when present (0 or 1, never more; 727
   files in the 2019 package, zero multi-ref cases), format 100 % canonical
   `yyyy/S nnn-nnnnnn` in 2019.
2. In-form `NOTICE_NUMBER_OJ` (R2.0.9 `PROCEDURE` section IV.2.1 /
   `COMPLEMENTARY_INFO` VII; R2.0.8
   `ADMINISTRATIVE_INFORMATION_*/PREVIOUS_PUBLICATION_INFORMATION_*/
   CNT_NOTICE_INFORMATION/NOTICE_NUMBER_OJ` + split DATE_OJ).
   Agreement with the coded value: **709/711 files (99.7 %)**; both
   mismatches were F20s (form cites the CAN, coded section another link in
   the chain) [measured]. Use the coded REF_NOTICE as primary, in-form as
   corroboration.

**Presence by form type** (files with REF_NOTICE / total) [measured]:

| Form | 2019 (R2.0.9) | 2014 (R2.0.8) |
|---|---|---|
| F03 / CONTRACT_AWARD | 411/558 (74 %) | 382/536 (71 %) |
| F14 corrigendum | 143/143 (100 %) | — (no F14; see OTH_NOT) |
| OTH_NOT (R2.0.8 corrigenda) | — | 157/162 (97 %) |
| F20 modification | 37/40 (93 %) | — |
| F02/F05 contract notices | 20/448 (4 %; PIN refs) | — |
| F21 social | 14/38 | — |

Of the 147 unlinked 2019 F03s, 50 are `PT_AWARD_CONTRACT_WITHOUT_CALL`
(legitimately chainless single-notice procedures), but **92 are PT_OPEN** —
open-procedure awards whose eSender simply left IV.2.1 empty. Net effect:
**~17 % of awards that should chain, don't** [measured]. Fallback evidence
exists: `OBJECT_CONTRACT/REFERENCE_NUMBER` (buyer's file number) is filled
in 66 % of all notices and 76 % of the unlinked F03s [measured] — usable for
*provisional* buyer+refnum matching, but it is free text (not exact-id
grade; under ADR-0003 discipline it must not auto-merge).

**Chain topology** [inferred from mechanism, high confidence]: each notice
points to *one* earlier notice → procedures form reference trees (CN ← many
F14s, ← many F03s; F03 ← F20s; CN ← PIN via the rare F02 refs). Tender
aggregation = union-find over the notice→notice edges, exactly the ADR-0001
"Tender documented by Notices" model. This replaces eForms' shared BT-04
procedure UUID (every notice carries the procedure id) with a **transitive
chain**: one missing link splits a procedure into two Tender aggregates
(silent under-merge, no wrong merges). eForms linkage is strictly stronger.

**Era-boundary behaviour** [measured]:

- Backwards: chains cross into the text era — 2011 notices reference
  `2009/S`/`2010/S` numbers; text-era notices themselves carry `RN:`
  references from at least 1993 (105 records in the 1993-01-02 file). If the
  text era is ingested even minimally (header-only Notices), XML-era chains
  terminate cleanly instead of dangling.
- Forwards (**the hard break**): eForms notices reference previous notices
  by eForms UUID (`notice-id-ref`), and legacy notices have no UUID. In 400
  sampled 2024 eForms CANs, exactly 2 mentioned a `yyyy/S` number — inside
  free-text `cbc:Description`, not machine-readable [measured]. Procedures
  that started under legacy forms (CN ≤ 2023) and ended under eForms
  (CAN ≥ 2024) will generally appear as **two unlinked Tenders**. No
  general fix exists; national procedure ids in free text could patch some
  German/Hungarian etc. cases later (out of scope for v1).

**Verdict**: legacy Tender aggregates + the versioned canonical layer are
buildable and sound. Expect: complete chains for corrigenda/modifications
(97–100 %), ~3/4 of awards chained to their contract notice, a long tail of
singleton Tenders (direct awards — real; and link-dropped awards — a
measurable data-quality metric for the dashboard), and a systematic split at
the 2023/24 eForms boundary.

---

## 4. R2.0.8 (and R2.0.7) vs R2.0.9 — what actually differs

Same container (§1); the differences are concentrated in the FORM_SECTION
grammar [measured, 2011 + 2014 vs 2019 packages]:

| Aspect | R2.0.7/R2.0.8 (2011–2016) | R2.0.9 (2016–2024) |
|---|---|---|
| Form roots | `CONTRACT`, `CONTRACT_AWARD`, `CONTRACT_UTILITIES`, `PRIOR_INFORMATION`, `OTH_NOT`, … + `FD_*` wrapper, `FORM="2"` numeric | `F02_2014`, `F03_2014`, … `FORM="F02"` |
| Directive regime | 2004/17/EC, 2004/18/EC (+2009/81 defence) | 2014/23/24/25/EU (`LEGAL_BASIS` CELEX) |
| Dates | `<DAY><MONTH><YEAR>` split elements | ISO `2019-11-26` + separate `HH:MM` |
| Money | display text + `@FMTVAL` machine value; coded VALUES_LIST formatted (`91 045`) | machine decimal + `@CURRENCY` |
| Corrigenda | `OTH_NOT` — semi-structured numbered free-text marks (`MARK_LIST/MLI_OCCUR/NO_MARK+TI_MARK+TXT_MARK`), 10–14 % of packages, TD code 2 | structured F14 with typed OLD/NEW values |
| Contract modifications | not a notice type | F20 |
| Lots | annex-style lot blocks; awards via repeated `AWARD_OF_CONTRACT` with LOT_NUMBER | `OBJECT_DESCR@ITEM`/`AWARD_CONTRACT@ITEM` + LOT_NO |
| Buyer/org ids | `ORGANISATION/NATIONALID` exists from R2.0.8 (198/1 156 files in 2014, 0 in 2011) | NATIONALID common (§6) |
| "Same as above" | `IDEM` marker elements inside contact blocks | `ADDRESS_*_IDEM` empty elements |
| Bid statistics | OFFERS_RECEIVED_NUMBER (+ _MEANING for e-offers) | NB_TENDERS_RECEIVED + SME/EU/e-means breakdown |
| Element vocabulary (whole daily package) | 609 distinct names (2014), 611 (2011); overlap 544 | 755 incl. defence (2019) |
| Declared XSD surface | R2.0.8.S03 full form set: **1 024** element names; R2.0.8.S05 (defence-only remnant): 686 | R2.0.9.S05 (F01–F25 + MOVE): **420** element names |

R2.0.7 (2011–2013): same namespace and near-same grammar as R2.0.8;
differences are ~65 element names each way per package sample — absorb into
the R2.0.8 profile and let the completeness checklist surface the delta
[measured/inferred].

`OTH_NOT` deserves emphasis: it is the R2.0.8 era's catch-all (corrigenda,
calls for expression of interest, EU-institution notices) — 177/1 817 (2011)
and 162/1 156 (2014) of files [measured]. Its body is numbered free-text
sections with occasional `ADDRESS_NOT_STRUCT` islands. It cannot populate
structured canonical fields beyond the coded header; it must be modelled the
same way as text-era notices: coded header + text blob, chained via its 97 %
REF_NOTICE coverage (a corrigendum arrives as a *version event* whose diff
is prose).

**The constant backbone**: `CODED_DATA_SECTION` is structurally stable
2011→2024 (REF_OJS; NOTICE_DATA: NO_DOC_OJS, URI_LIST, LG_ORIG, ISO_COUNTRY,
ORIGINAL_CPV, NUTS (as `ORIGINAL_NUTS` pre-2016, `PERFORMANCE_NUTS`/
`CA_CE_NUTS` after), REF_NOTICE, VALUES_LIST (R2.0.8); CODIF_DATA:
DS_DATE_DISPATCH, DT_DATE_FOR_SUBMISSION (2019), AA_AUTHORITY_TYPE,
TD_DOCUMENT_TYPE, NC_CONTRACT_NATURE, PR_PROC, RP_REGULATION, TY_TYPE_BID,
AC_AWARD_CRIT, MA_MAIN_ACTIVITIES, DIRECTIVE/HEADING/INITIATOR) [measured].
The TD/NC/PR/AA/AC code lists are also exactly the text-era header codes
(§7) — one small code-driven mapper covers 1993–2024 for the ~15 fields it
carries, independent of form grammar.

---

## 5. Field coverage against the canonical core and the 357 BTs

### 5.1 Canonical core: legacy source elements and measured fill rates

R2.0.9 = 2019 package; R2.0.8 = 2014 package [measured]:

| Canonical field | R2.0.9 source | fill | R2.0.8 source | fill |
|---|---|---|---|---|
| Buyer name/address/contact | `ADDRESS_CONTRACTING_BODY` | 100 % | `CA_CE_CONCESSIONAIRE_PROFILE/ORGANISATION` | 100 % |
| Buyer type / activity | `CA_TYPE`, `CA_ACTIVITY` + coded AA/MA | ~100 % | `TYPE_OF_CONTRACTING_AUTHORITY`/`TYPE_OF_ACTIVITY` + coded | ~100 % |
| Title | `OBJECT_CONTRACT/TITLE` (+24-lang ML_TITLES) | 100 % | `TITLE_CONTRACT` (+ML_TITLES) | 100 % |
| CPV main/additional | `CPV_MAIN`/`CPV_ADDITIONAL` + coded ORIGINAL_CPV | 100 % | same | 100 % |
| NUTS (performance) | lot `NUTS@CODE` (n2016) + coded | 100 % of F02 | `LOCATION_NUTS/NUTS` + coded | 73 % of CONTRACT |
| Estimated value | `VAL_ESTIMATED_TOTAL@CURRENCY` | 64 % of F02 | `COSTS_RANGE_AND_CURRENCY@FMTVAL` | 44 % of CONTRACT |
| Submission deadline | `DATE_RECEIPT_TENDERS`+`TIME_…` + coded DT | 100 % of F02 | `RECEIPT_LIMIT_DATE` (D/M/Y) | 100 % of CONTRACT |
| Procedure type | `PT_*` marker + coded PR_PROC | 100 % | `TYPE_OF_PROCEDURE_*` + coded | 100 % |
| Lots | `LOT_DIVISION`/`OBJECT_DESCR@ITEM`+LOT_NO | 100 % | lot blocks / `LOT_NUMBER` | 100 % |
| Winner(s) | `CONTRACTORS/CONTRACTOR/ADDRESS_CONTRACTOR` | 91 % of F03 | `ECONOMIC_OPERATOR_NAME_ADDRESS` | 100 % of CONTRACT_AWARD |
| Award value | `AWARDED_CONTRACT/VALUES/VAL_TOTAL` | 91 % of F03 | `CONTRACT_VALUE_INFORMATION/COSTS_RANGE_AND_CURRENCY` | 83 % |
| Contract conclusion date | `DATE_CONCLUSION_CONTRACT` | 91 % of F03 | `CONTRACT_AWARD_DATE` (D/M/Y) | 96 % |
| Bids received | `NB_TENDERS_RECEIVED` (+breakdowns) | high | `OFFERS_RECEIVED_NUMBER` | high |
| Award criteria | `AC_QUALITY/AC_CRITERION+AC_WEIGHTING`, `AC_PRICE` | when MEAT | `AWARD_CRITERIA_DETAIL` | when MEAT |
| Review body | `ADDRESS_REVIEW_BODY`, `REVIEW_PROCEDURE` | ~100 % | `PROCEDURES_FOR_APPEAL/…` | high |
| Duration | `DURATION@TYPE` MONTH/DAY | common | `DURATION_FRAMEWORK_MONTH` etc. | common |
| Framework/DPS | `FRAMEWORK`/`DPS` elements | present | `F02_FRAMEWORK`/`NOTICE_INVOLVES` | present |
| Modification data | F20 `MODIFICATIONS_CONTRACT` (before/after values, reason) | 100 % of F20 | **absent** (no F20) | — |
| Structured corrections | F14 typed CHANGES | 100 % of F14 | **absent** (OTH_NOT prose) | — |

Missing-in-both (canonical columns that stay NULL for the whole legacy era,
non-exhaustive, [inferred from dissection + eForms model]): procedure/lot
UUIDs, notice-internal entity registries (ORG/TPA/TEN/RES/CON ids),
tender-level data (individual bid values/ranks — legacy has only counts and
the winning value), UBOs, per-bidder subcontracting graphs,
green/social/innovation strategic-procurement codes, exclusion grounds
detail, e-invoicing/e-payment indicators, BT-22 internal ids, review
*outcomes* (only review-body addresses exist).

### 5.2 How many of the 357 BTs have legacy equivalents?

Use OP's own material rather than guessing; two authoritative sources:

- **OP-TED/ted-xml-data-converter** (GitHub): the official R2.0.9→eForms
  XSLT. Verified from its README (fetched 2026-07-19) [docs]: "All the
  standard TED XML forms using the R2.0.9 TED XML schema (with the
  exceptions of **F14 Corrigendum and F20 Modification**) are now
  convertible" — PIN (F01/F04/F07/F08/F21/F22), CN
  (F02/F05/F12/F21–F24), CAN (F03/F06/F13/F15/F21–F25). Defence R2.0.8
  forms: "work has begun" (i.e. not supported). R2.0.8 standard forms:
  not mentioned anywhere — entirely out of scope. OP's own disclaimer:
  it "cannot guarantee the accuracy, adequacy, validity, reliability,
  availability or completeness" of the conversion; the converter's stated
  purpose is producing a *draft* for the publisher to "correct and
  complete", not a faithful data migration. The `xslt/` folder is the
  per-element mapping reference to mine.
- The repo's **`ted-elements-not-convertible.md`** [docs, fetched]: ~23
  data-bearing R2.0.9 elements have **no eForms representation at all** —
  reverse lossiness. Notable entries: `REFERENCE_NUMBER` ("eForms does not
  have a BT to hold a reference number" — which is *our linkage fallback*,
  §3), `*_OTHER` free-text variants of coded fields (CA_TYPE_OTHER,
  CA_ACTIVITY_OTHER, LEGAL_BASIS_OTHER), `ECONOMIC/TECHNICAL_CRITERIA_DOC`,
  `CRITERIA_CANDIDATE`, `URL_NATIONAL_PROCEDURE`, `DATE_AWARD_SCHEDULED`,
  offer-range totals (`VAL_RANGE_TOTAL/HIGH|LOW`), `VAL_PRIZE`,
  `VAL_BARGAIN_PURCHASE`, non-FA `VAL_ESTIMATED_TOTAL` on awards.
  **Consequence for us**: an eForms-shaped canonical schema alone cannot
  hold everything legacy notices publish — the legacy profiles need a small
  set of legacy-only canonical columns/satellites (~20 fields), otherwise
  ADR-0004's "everything mapped" is unachievable for the legacy era by
  construction.
- **eForms↔standard-forms mapping documentation** ("eForms and Standard
  forms compared" tables on docs.ted.europa.eu, deriving from the
  Regulation 2019/1780 annex): per BT, the corresponding standard-form
  field (section number) if any [docs — **not independently re-verified;
  the delegated web-research agent did not return**].

Working coverage estimate **[unverified estimate — needs the mechanical
tally listed in Open questions]**: of the **357 BT ids**, roughly **half
have a legacy R2.0.9 source** (the annex tables map standard-form sections
I–VII onto BTs; everything in §5.1 plus procedure flags, framework/DPS,
EU-funds, accessibility, e-communication fields), a further slice is
*derivable* (notice type/subtype from TD+FORM, publication data from the
coded section), and the remainder — largely the eForms extension entities
(UBO, per-bid data, review outcomes, strategic procurement, identifiers) —
has **no legacy source and must be declared era-absent**. For R2.0.8
subtract the F14/F20 structures and the SME/e-means statistics; for the
text era see §7.

### 5.3 What OP's converter lossiness implies for us

- Their converter's *scope* — F01–F13, F15, F21–F25, R2.0.9 only — covers
  ~85 % of R2.0.9-era notices but 0 % of R2.0.8/R2.0.7 (2.2 M notices) and
  none of the correction/modification stream that our versioned canonical
  layer most needs (F14 12 %, F20 3 % of 2019 volume) [measured share].
- Reusing the converter as a *pipeline stage* (legacy→eForms XML→our eForms
  importer) is therefore not an option for completeness reasons alone —
  quite apart from importing "some errors" as canonical truth and pinning
  our archive semantics to their XSLT's maintenance status (archived
  schemas, closed submission channel).
- Reusing it as a **mapping reference** is exactly right: its XSLT encodes
  hundreds of reviewed element→BT decisions (code-list translations,
  procedure-type mappings, address handling). Mine it when writing our
  R2.0.9 profile; diverge knowingly (e.g. we map F14 as version events
  instead of refusing).
- Their F14/F20 exclusion also validates our ADR-0001 design choice: we do
  not need to synthesize full corrected notices (the eForms change-notice
  shape); we apply typed deltas to the canonical layer, which legacy F14
  supports natively (§2.3).

---

## 6. Organization data in legacy notices

No ORG- registry exists: every party is an **inline address block** at its
role's location in the form (buyer, further-info, participation, contractor,
review body, appeal-mediation, on-behalf-of buyers) [measured]. That maps
1:1 onto our OrganizationMention concept — one mention per address block,
role from the enclosing element.

Identifier quality [measured, 2019 package]:

- `ADDRESS_CONTRACTING_BODY` blocks with NATIONALID: **799/1 513 = 53 %**.
- `ADDRESS_CONTRACTOR` (winner) blocks with NATIONALID: **1 676/3 442 = 49 %**.
- Of filled contractor NATIONALIDs, **275/1 676 = 16 % contain no digit** —
  junk like the literal string "Romania" (a national eSender fills the
  country name), or "Ukjent"/"n/a" variants. Digit-containing values are a
  mix of national registry numbers (CZ ICO `65993390`), VAT ids
  (`NL804595859B01`, `GB287461957`), and free-form (`FR463307 15368` with
  spaces).
- **No scheme attribute exists** — unlike eForms BT-501's
  `schemeName`, a legacy NATIONALID never says *which* register the number
  is from. Country prefix heuristics identify VAT ids; bare numerics are
  ambiguous per country.
- Era floor: 2011 (R2.0.7) has **zero** NATIONALID anywhere; 2014 has it in
  17 % of files (all form types, XSD-optional).

Implications for ID-only auto-merge (CONTEXT.md):

1. The exact-identifier merge rule still works but needs a **normalization +
   plausibility gate**: uppercase/strip spaces, reject values without
   digits, tag as `vat` vs `national` by pattern, and scope merges to
   `(country, normalized_id)` — never merge on the raw string [inferred].
2. Expect roughly **half of legacy-era mentions to be name-only provisional
   profiles** (vs eForms, where BT-501 is broadly present), and 100 % of
   2011–2013 mentions. Organization analytics ("all wins of company X")
   thin out accordingly before 2016 — a dashboard-visible coverage fact,
   not a bug.
3. Buyer identity has a strong fallback: buyers recur constantly, and
   `ML_AA_NAMES` + ISO_COUNTRY + town give stable name-keys for provisional
   profiles; winners are the harder half.

---

## 7. Text era (1993–2010) — quick assessment only

Confirmed structure [measured, 1993/2005/2008 files]: per-language files of
concatenated records; ~20–30 coded header lines then `TX:` free prose. The
2008-era field inventory (1 095 records, EN file): always present — TI
title, PD publication date, ND document number, OJ issue, DS dispatch date,
DR reception, TD/NC/PR/RP/AA/TY/AC codes (same code lists as XML-era
CODIF_DATA), CY country, AU buyer name, TW town, OL original language,
**PC CPV code** (100 % by 2005), PN CPV label; frequent — AB abstract
(92 %), DT deadline (49–61 %), **RN reference-to-previous-notice** (27–37 %,
present since 1993), CO/RC/RG (NUTS-ish region codes, minority). 1993 files
lack PC/OL/TW but have CC/CT (pre-CPV product codes) [measured].

From 2008 a parallel `meta` file per language wraps the same records in
pseudo-XML (`<codifdata><refnotice>2007/S 243-295856</refnotice>…`) — the
coded header is *more* parseable there (refnotice already in canonical OJS
form) but adds no new content [measured].

Minimal-mapping yield per text-era notice: publication/dispatch dates,
deadline (half), buyer name+country+town, title, CPV (post-~2002; product
codes before), contract nature / procedure / authority-type / award-criteria
codes, original language, chain reference (a third), plus the full text
blob. Corrigenda are TD:2 records whose TX contains "for: … read: …" prose —
version events with prose diffs, same treatment as OTH_NOT. **No lots,
values, winners, or structured awards** (award notices exist as TD:7 records
but the winner is named only in prose). Deep-dive deferred; no official spec
found for the tagged format (field semantics above are internally consistent
with the XML-era code lists) [inferred, medium-high confidence].

---

## 8. Recommended era-aware policy

### 8.1 Era mapping profiles

Dispatch per file (root element + namespace + VERSION attr), never per
package date. Four profiles, each owning a frozen source-schema inventory:

| Profile | Selector | Completeness checklist ("mapped-or-ignored" universe) | Checklist size |
|---|---|---|---|
| `text` | non-XML file in pre-2011 package layout | the header field-code list (TI, PD, ND, …) per sub-era (1993/2000/2008-meta variants) + `TX` as one blob field | ~30 codes |
| `ted-export-r208` | `TED_EXPORT` in `TED_schema/Export` ns (VERSION R2.0.5–R2.0.8) **or** R2.0.8 publication ns (defence) | element+attribute inventory of R2.0.8.S03…S05 publication XSDs (mirrored), plus the empirically discovered R2.0.7 delta | ~1 050 element names |
| `ted-export-r209` | `TED_EXPORT` in any R2.0.9 ns spelling | R2.0.9.S01…S05 publication XSDs (mirrored) | ~420 element names |
| `eforms` | UBL roots | fields.json per (SDK version, profile) — as already decided in ADR-0002 | 1 256 fields |

"Era completeness" for a profile = every element/attribute in its checklist
has an explicit mapping (column / satellite / version-event) **or** an
explicit ignore rule with a reason (`boilerplate-link`, `display-label`,
`translation-copy`, …). This keeps the ADR-0002 mechanical-checklist idea
intact per era: for eForms the checklist is fields.json; for legacy eras it
is the era's XSD element inventory; for text it is the field-code list.

### 8.2 Quarantine semantics (ADR-0004, era-scoped)

- **Unchanged rule, era-scoped universe**: a notice quarantines iff it
  contains content not consumed by *its own profile's* mappings/ignore
  rules. A text-era notice whose TX blob is mapped-as-blob is *complete*;
  free text never counts as unmapped content when the profile declares it a
  blob field.
- Quarantine reasons carry the profile id; the dashboard reports quarantine
  counts **per profile**, so a parser gap in R2.0.8 doesn't drown the
  eForms metric and vice versa.
- Unknown profile (new namespace spelling, unexpected root) is itself a
  quarantine reason — that is how the next namespace surprise surfaces.
- Schema-valid-but-weird content (e.g. junk NATIONALID) does **not**
  quarantine — it is consumed by a mapping whose normalization may reject
  the value into a "raw kept, normalized NULL" pair. Quarantine is for
  *unconsumed structure*, not low-quality values.

### 8.3 Canonical-layer semantics per era

- One canonical schema (the eForms-shaped one) for all eras; earlier eras
  simply populate fewer columns. Add a per-Tender/per-Notice
  `source_profile` so every NULL is interpretable ("absent in era" vs
  "absent in notice").
- Per-profile *era coverage declarations*: a static table (in code, tested)
  of which canonical fields the profile can ever populate. The dashboard
  derives "field X available from year Y" documentation from it, and the
  completeness test asserts the mapping actually writes only/all declared
  fields.
- Corrections: F14 typed changes and eForms change notices become the same
  canonical version events; OTH_NOT and text-era corrigenda become
  *prose version events* (version bump + attached text, no field diffs).
- Chains: build Tender aggregates by union-find on `(year, number)`
  reference edges within the legacy Source; keep the "referenced but never
  seen" set as dangling edges (they resolve when backfill deepens —
  re-projection handles late merges, same machinery as ADR-0003).
- Re-scope the product promise as: **"every field the source era publishes
  is represented — all 357 eForms BTs for the eForms era, the full XSD
  content for TED-XML eras, the full coded header for the text era; nothing
  silently dropped in any era."** That sentence is implementable and
  testable; the current CONTEXT.md wording is only satisfiable for eForms.

### 8.4 Suggested build order [inferred]

1. eForms profile (the target shape; already researched).
2. `ted-export-r209` F02/F03/F14/F20 (75 %+12 % of era volume; exercises
   chains + version events), then the remaining F-forms, then defence.
3. `ted-export-r208` (reusing the shared coded-section mapper and address
   mapper; OTH_NOT as prose events).
4. `text` header-only profile (cheap, mostly the same code lists), if/when
   pre-2011 backfill is wanted.

---

## Implications for tender-db

1. **ADR-0001/ADR-0004 survive intact** with era-scoped semantics: the
   two-layer model needs no changes; quarantine needs only a profile-scoped
   definition of "mapped"; the versioned canonical layer gets *better* raw
   material from legacy F14 (typed diffs) than eForms change notices give it
   (full re-statements).
2. **Write the legacy mappings by hand, mine OP's converter for decisions,
   never run it in the pipeline** (§5.3).
3. **Linkage is chain-based and imperfect**: expose "unchained award
   notices" (~17 % of awards) as a dashboard data-quality metric alongside
   quarantine; treat the eForms-boundary split (legacy CN + eForms CAN =
   two Tenders) as a known, documented limitation.
4. **Organization merging degrades gracefully**: keep the exact-ID merge
   rule but add normalization + plausibility gating (16 % junk ids
   measured); accept that pre-2016 organization profiles are mostly
   name-keyed provisionals.
5. **Parser architecture**: match by local name within a profile (namespace
   spellings vary even for one schema revision); treat attributes as the
   code carriers and element text as disposable labels; normalize
   `(year, number)` notice keys everywhere.
6. **Mirrored XSDs are now on the VPS** (`/opt/tender-db/ted-xsd/`:
   R2.0.9 S01+S05, R2.0.8 S03+S05, XML-labels mappings, validation-rules
   workbook, form-label PDFs) — the archive page could disappear without
   blocking us. R2.0.7 XSDs were *not* on the page (open question).

## Open questions

### Needs more research

- **Mechanical BT tally**: tabulate the regulation-annex eForms↔standard-
  forms mapping table (and the converter XSLT in
  `OP-TED/ted-xml-data-converter/xslt/`) into an exact per-BT
  has-legacy-source list, to replace the "roughly half" **unverified
  estimate** in §5.2 and to seed the R2.0.9 profile's field mapping. Also
  locate the exact docs.ted.europa.eu URL of the "eForms and Standard forms
  compared" tables (the delegated web-research agent did not return; only
  the converter repo's facts were verified first-hand).
- **Legacy-only canonical fields**: turn `ted-elements-not-convertible.md`
  (~23 elements) plus an XSD sweep into the definitive list of legacy
  fields needing canonical columns outside the eForms shape (§5.2).
- **R2.0.7 XSD**: not on the OP archive page. Either locate an older
  snapshot (web.archive.org of the tedschemas page / SIMAP) or derive the
  2011–2013 delta empirically from a full-year element sweep (cheap on the
  VPS) and fold it into the R2.0.8 checklist.
- **Full-archive linkage rates**: my 71–74 % award-chaining figures come
  from two January packages; a whole-year sweep (one evening on the VPS)
  would give per-year, per-country curves and validate the union-find
  approach at scale before the importer is built.
- **R2.0.9 S-revision deltas**: S01→S05 changelogs are in the mirrored zips;
  diff the XSD inventories to confirm one R2.0.9 profile plus additive
  deltas suffices (LEGAL_BASIS, NUTS-2016 namespace, DT_DATE_FOR_SUBMISSION
  appeared mid-era [measured]).
- **Utilities/concession/social forms (F04–F13, F21–F25) dissection**: form
  grammar spot-checked only via XSDs; dissect real samples when writing
  their mappings (same shape family as F02/F03, low risk).
- **eForms-boundary stitching**: whether national procedure ids (e.g. the
  Hungarian EKR number seen in free text) or TED's internal
  RECEPTION_ID lineage can machine-link legacy CN → eForms CAN for some
  countries. Low priority, measurable later.

### Needs a user decision

- **Adopt the re-worded completeness promise** (§8.3) in CONTEXT.md — the
  current "all business terms, no omissions" phrasing is era-impossible and
  already flagged there as pending.
- **Text-era ingestion depth**: header-only profile now (cheap, makes
  XML-era chains terminate cleanly, enables 1993+ counts on the dashboard)
  vs deferring the text era entirely. (Interacts with the EN-only vs
  all-languages storage decision already open in ted-access-channels.md.)
- **Whether "unchained award" and "junk organization id" metrics join the
  dashboard's headline data-quality panel** next to quarantine counts —
  recommended, since both are measured, era-specific, and user-visible.
