# 41 — INTERNAL_OJS R2.0.5 era profile (the 2008 OPOCE export)

Status: ready-for-agent
Priority: completeness-critical (the last verify-blocking year)

Split out of issue 36 (2026-07-21) once the DTD investigation overturned the
"duplicate era" hypothesis. The 2008 `opoce-input/` bucket is **real,
opoce-only notices** — not duplicates of the text channel — and they are the
whole 2008 coverage gap.

## The evidence (issue 36)

- Members: `20080502_2008085.tar.gz/<num>/opoce-input/<num>_2008.<lg>`, one small
  file per language (~22 langs), so **621,863 members ≈ 28k distinct notices**
  (all 2008, +1,898 in 2010). All outstanding.
- Format: `<!DOCTYPE INTERNAL_OJS PUBLIC "-//OPOCE OJS//DTD INTERNAL_OJS XML
  R2.0.5//EN" "Internal_Ojs.dtd" […]><INTERNAL_OJS HEADING="…">…`. A **distinct
  vocabulary** from TED_EXPORT (r208/r209), the text era, and eForms:
  `TECHNICAL_INFO / BIB_INFO / REF_OJS / BIB_DOC_S / codified single-char codes
  (SECTOR/NAT_NOTICE/MARKET/PROC/MARKET_ORG/TYPE_BID/AWARD_CRIT) / ORIGINAL_CPV /
  DATE_DISP / DATE_REC / ISO_COUNTRY / NO_DOC_OJS` plus a per-heading form body
  (`EEIG`, `FD_*`, `GROUP_NAME / TXT_MARK / P …`).
- Mainstream S-series notices (headings 3310/3340/3540/45xx/33xx/02A0…), proper
  `NO_DOC_OJS 2008/S 85-114238`.
- **Not duplicates:** 0 of 350 sampled opoce notices have a `ted` text twin in
  `notices` (id format validated). ≈28k opoce-only ≈ the ~27k 2008 shortfall.
- 2008 coverage today = 312,567 / 339,534 = **92.1 %**, failing verify's ±2%.
  Parsing this era is what lifts 2008 into tolerance — the last such year.

## What issue 36 already left in place

- `profile::strip_doctype` — XXE-safe DTD removal (tested both ways).
- Dispatch routes `INTERNAL_OJS` roots to a tracked `unmapped-era` quarantine
  (profile `internal-ojs`). **This issue replaces that branch with real parsing.**
- One byte-exact fixture: `crates/ingest/tests/fixtures/internal_ojs/114238_2008.en`.

## Vocabulary scope (mined 2026-07-21, one package 20080502_2008085, 1693 EN notices)

- **577 distinct elements**, 20 attributes
  (`@CODE @VALUE @CTYPE @CURRENCY @LG @CATEGORY @FORM @VERSION @ITEM @KEY @QUOTE
  @SEP @CHOICE @CLASS @NO_SEQ @PRICE @SERVICES_CATEGORY @CONTRACT_TYPE @HEADING
  @TYPE`). ~55 heading families in this one package (21xx/22xx notices,
  33xx/45xx awards, etc.). Full catalog saved to the session scratchpad
  (`internal_ojs_vocab.txt`); a complete inventory should sweep several packages
  across months for heading coverage, like the text sweep did.
- **Two layers**, mirroring TED_EXPORT: an envelope
  `INTERNAL_OJS/TECHNICAL_INFO + BIB_INFO/REF_OJS/BIB_DOC_S` carrying the coded
  backbone (`NO_DOC_OJS`, single-char CODIF codes SECTOR/NAT_NOTICE/MARKET/PROC/
  MARKET_ORG/TYPE_BID/AWARD_CRIT, `ORIGINAL_CPV`, `ORIGINAL_NUTS`, `DATE_DISP`/
  `DATE_REC`, `ISO_COUNTRY`, `DEADLINE_REC`), and a per-heading **form body**.
- **KEY DESIGN LEAD:** the form body vocabulary looks like a `_SUM`-suffixed
  variant of the r208/r209 TED_EXPORT forms — same element names and attribute
  conventions (`CONTRACT_SUM/FD_CONTRACT_SUM`, `CA_CE_CONCESSIONAIRE_PROFILE`,
  `NAME_ADDRESSES_CONTACT_CONTRACT`, `CPV_MAIN/CPV_CODE @CODE`,
  `VALUE_COST @CURRENCY`, `@LG/@CATEGORY/@FORM/@VERSION` on the form root). So the
  r209 form walker (`crates/ingest/src/r209/`) may be largely reusable with a
  different envelope + the `_SUM` element aliases — potentially turning a
  from-scratch 577-element profile into "new envelope + a rules delta over r209".
  **First design step: diff the INTERNAL_OJS form vocabulary against r209's
  consumed set** — decide reuse-vs-new before writing the inventory.

## r209-diff result (2026-07-21) — REUSE, not from-scratch

Diffed the 577 INTERNAL_OJS elements against `sdk/ted-export-inventory.json`
(the r208/r209/defence vocabulary, 1314 elements):
- **492 of 577 (85%) are shared** with r208/r209; 80% by occurrence.
- Of the 85 INTERNAL_OJS-only elements, **65 are `_SUM`-suffixed aliases whose
  base name is already an r209 element** (`CONTRACT_SUM`→`CONTRACT`,
  `FD_CONTRACT_SUM`→`FD_CONTRACT`, `AWARD_OF_CONTRACT_SUM`→`AWARD_OF_CONTRACT`,
  the `*_INFORMATION_SUM` section wrappers, …) — mechanical: strip `_SUM`.
- The genuine remainder (~20) is the **envelope** (`INTERNAL_OJS`,
  `TECHNICAL_INFO`, `BIB_INFO`, `REF_OJS`, `BIB_DOC_S`, `LG_OJ`) and the **coded
  backbone** (`SECTOR NAT_NOTICE MARKET PROC MARKET_ORG TYPE_BID AWARD_CRIT
  MAIN_ACTIVITIES DATE_DISP DATE_REC DEADLINE_REC DEADLINE_REQ`) — the same
  CODIF single-char code families the text/r209 eras already carry.

**Implementation shape (decided): reuse the r209 walker with a thin delta.**
1. A new envelope reader: from `INTERNAL_OJS`, take identity + coded backbone
   from `BIB_INFO/BIB_DOC_S` (map the CODIF codes with the shared code lists,
   `ORIGINAL_CPV`→cpv, `ORIGINAL_NUTS`→nuts, dates, `ISO_COUNTRY`), then hand the
   `FD_*_SUM` form body to the r209 form walker.
2. A `_SUM` alias normalisation so the r209 rules match the summary elements
   (treat `X_SUM` as `X`) — 65 aliases, generated not hand-listed.
3. An era-scoped inventory over the ~20 envelope/backbone additions + the alias
   set; the 492 shared elements already have r209 rules.

This turns a 577-element profile into a small envelope module + an alias shim
over `crates/ingest/src/r209/`. The full multi-package breadth sweep (below) is
still needed to catch drift and any headings absent from the first package
before the bijection tests are trusted.

**Corpus note:** the opoce-input format is NOT year-wide — the bucket's packages
run `20080502_2008085` … `20080531_2008105`, i.e. ~22 daily packages in **May
2008** (~30k members each ≈ the 620k bucket), plus a small 2010 tail. Breadth =
across those ~22 May packages and their heading families, not 12 months.

## Plan (scope it like the `text` / `r208` profiles)

1. **Inventory** the INTERNAL_OJS R2.0.5 vocabulary from real payloads — every
   element and attribute across the heading families and a spread of languages
   (ADR-0002, era-scoped: the completeness test walks the vendored inventory and
   fails on any element/attribute without a decision, and vice versa). No public
   spec exists; mine it from the archive like the text inventory was. Pull member
   paths via `/v1/sql` (token at the owner-verify scratchpad path), bytes from the
   VPS `/data/archive` `opoce-input/` subtrees.
2. **Map** the coded backbone into the model (`NoticeValue`): `NO_DOC_OJS`/OJS ref
   as the identity, the single-char CODIF codes (reuse the shared code lists where
   they match the text/r209 vocabularies), `ORIGINAL_CPV` as a cpv classification,
   `ISO_COUNTRY`, `DATE_DISP`/`DATE_REC`/deadlines, `HEADING`, and the form body
   text. EN-primary per the text-era policy (one language ingested, the rest
   documented skips) unless the payloads argue otherwise — the per-language files
   are the same notice in N languages, exactly the ISO/UTF8 twin situation.
3. **Dispatch**: add a `ted-export`-style profile selector (a new `internal-ojs`
   profile module) and replace issue 36's `unmapped-era` quarantine branch with a
   call into it. Per-language sibling handling like the text era's
   `PackageContext` (ingest EN, skip the rest as documented duplicates).
4. **Fixtures + tests**: byte-exact real notices across ≥3 heading families and
   the EN + one other language; exhaustive-consumption (ADR-0004) and inventory
   bijection (ADR-0002) suites, mirroring `crates/ingest/tests/text.rs`.
5. **Reprocess** after deploy: `reason='unmapped-era'` (profile `internal-ojs`) →
   notices; confirm 2008 coverage rises into the ±2% tolerance. Ledger entry
   (issue 40) keyed to the reprocessed bucket.

Acceptance: INTERNAL_OJS 2008 notices parse into the model; the fixtures parse
exhaustively; the inventory is bijective with the rules; dispatch routes the era
to the profile (no more `unmapped-era`); after reprocess, 2008 coverage reaches
the verify ±2% tolerance — the last completeness-blocking year cleared.

Lane: `crates/ingest` (new `internal-ojs` profile + fixtures) + `crates/app/data`
ledger. No overlap with other active work.

---

## Handoff for a fresh agent (self-contained — no re-mining needed)

Everything below is from the 2026-07-21 mining of package `20080502_2008085`
(1693 EN notices) + the r209-diff. Recheck against the full May sweep before
trusting the bijection tests, but this is enough to start.

### The complete delta over the r209 vocabulary

**20 non-`_SUM` new elements** (the envelope + coded backbone):
`AGREEMENT_PUBLICATION AWARD_CRIT BIB_DOC_S BIB_INFO DATE_DISP DATE_REC
DEADLINE_REC DEADLINE_REQ INTERNAL_OJS LG_OJ LOTS MAIN_ACTIVITIES MARKET
MARKET_ORG NAT_NOTICE PROC SECTOR SERVICES TECHNICAL_INFO TYPE_BID`

**65 `_SUM` aliases** — every base name (strip `_SUM`) is already an r209
element, so a generated `X_SUM → X` normalisation makes the r209 rules match:
`ADMINISTRATIVE_INFORMATION_CONCESSION_SUM ADMINISTRATIVE_INFORMATION_CONTRACT_NOTICE_SUM
ADMINISTRATIVE_INFORMATION_CONTRACT_UTILITIES_SUM ADMINISTRATIVE_INFORMATION_DEF_SUM
ADMINISTRATIVE_INFORMATION_DESIGN_CONTEST_NOTICE_SUM AI_PROCEDURE_PERIODIC_INDICATIVE_SUM
ANNEX_I_SUM AUTHORITY_CONCESSION_SUM AUTHORITY_ENTITY_DESIGN_CONTEST_SUM
AUTHORITY_ENTITY_NOTICE_BUYER_PROFILE_SUM AUTHORITY_PERIODIC_INDICATIVE_SUM
AUTHORITY_PRIOR_INFORMATION_SUM AWARD_AND_CONTRACT_VALUE_SUM
AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES_SUM AWARD_OF_CONTRACT_SUM AWARD_PRIZES_SUM
BUYER_PROFILE_SUM CONCESSION_SUM CONDITIONS_FOR_MORE_INFORMATION_SUM
CONTACTING_AUTHORITY_INFORMATION_SUM CONTACTING_AUTHORITY_INFO_SUM
CONTRACTING_AUTHORITY_INFORMATION_SUM CONTRACTING_ENTITY_CONTRACT_AWARD_UTILITIES_SUM
CONTRACTING_ENTITY_RESULT_DESIGN_CONTEST_SUM CONTRACT_AWARD_SUM
CONTRACT_AWARD_UTILITIES_SUM CONTRACT_OBJECT_DESCRIPTION_SUM CONTRACT_SUM
CONTRACT_UTILITIES_SUM DESCRIPTION_AWARD_NOTICE_INFORMATION_SUM DESCRIPTION_CONCESSION_SUM
DESCRIPTION_CONTRACT_AWARD_UTILITIES_SUM DESCRIPTION_CONTRACT_INFORMATION_SUM
DESIGN_CONTEST_SUM FD_BUYER_PROFILE_SUM FD_CONCESSION_SUM FD_CONTRACT_AWARD_SUM
FD_CONTRACT_AWARD_UTILITIES_SUM FD_CONTRACT_SUM FD_CONTRACT_UTILITIES_SUM
FD_DESIGN_CONTEST_SUM FD_PERIODIC_INDICATIVE_UTILITIES_SUM FD_PRIOR_INFORMATION_SUM
FD_RESULT_DESIGN_CONTEST_SUM INTRODUCTION_PERIODIC_INDICATIVE_SUM OBJECT_CONCESSION_SUM
OBJECT_CONTRACT_AWARD_UTILITIES_SUM OBJECT_CONTRACT_INFORMATION_CONTRACT_AWARD_NOTICE_SUM
OBJECT_CONTRACT_INFORMATION_CONTRACT_UTILITIES_SUM OBJECT_CONTRACT_INFORMATION_SUM
OBJECT_CONTRACT_PERIODIC_INDICATIVE_SUM OBJECT_DESIGN_CONTEST_SUM
OBJECT_NOTICE_BUYER_PROFILE_SUM OBJECT_RESULT_DESIGN_CONTEST_SUM
OBJECT_SUPPLY_SERVICE_PRIOR_INFORMATION_SUM OBJECT_WORKS_PRIOR_INFORMATION_SUM
PERIODIC_INDICATIVE_UTILITIES_SUM PRIOR_INFORMATION_SUM PROCEDURES_CONCESSION_SUM
PROCEDURES_DESIGN_CONTEST_SUM PROCEDURE_DEFINITION_CONTRACT_NOTICE_SUM
PROCEDURE_DEFINITION_CONTRACT_NOTICE_UTILITIES_SUM RESULTS_CONTEST_RESULT_DESIGN_CONTEST_SUM
RESULT_CONTEST_SUM RESULT_DESIGN_CONTEST_SUM`

### Envelope divergence (where INTERNAL_OJS is NOT TED_EXPORT)

- `TED_EXPORT` → `INTERNAL_OJS` (root, `@HEADING`).
- `CODED_DATA_SECTION` → `BIB_INFO` (`REF_OJS/COLL_OJ/NO_OJ/DATE_PUB/LG_OJ`) +
  `BIB_DOC_S`. The coded backbone here is **bare-text single-char codes**
  (`<SECTOR>9</SECTOR>`, `<MARKET>9</MARKET>`, `<PROC>9</PROC>`, `<NAT_NOTICE>G</NAT_NOTICE>`,
  `TYPE_BID/AWARD_CRIT/MARKET_ORG/SECTOR`) — NOT r209's `@CODE`-attribute CODIF
  elements. Map these in the envelope; they are the ~15 new backbone names above.
- Identity: `NO_DOC_OJS` = `2008/S 85-114238` (S-issue + doc-year); the member
  path's `<num>_2008` is the `<doc>-<year>` id (`114238-2008`), matching how the
  text channel keys 2008 notices.
- `TRANSLATION_SECTION`/`ML_*` — not present; INTERNAL_OJS is one language per
  file (per-language siblings, handle like the text era's UTF8/ISO twins:
  ingest EN, skip the rest as documented duplicates).
- `FORM_SECTION` → the `FD_*_SUM` form body (hand to the r209 walker via the
  `_SUM` shim).

### TRAPS (same name, different meaning — verified against the fixtures)

1. **`ORIGINAL_CPV` / `ORIGINAL_NUTS`**: INTERNAL_OJS carries the code as **text
   content** (`<ORIGINAL_CPV>74111000</ORIGINAL_CPV>`); r208/r209 carry it in
   **`@CODE`** with a label (`<ORIGINAL_CPV CODE="34928530">Street lamps</…>`).
   Reusing the r209 rule would read an empty code. Handle these in the envelope
   (read text), do NOT delegate to the r209 rule.
2. **`SERVICE_CATEGORY` / `SERVICE_CATEGORY_PUB`**: carry an extra `@VALUE` in
   INTERNAL_OJS that the r209 inventory does not list (the only two shared
   elements with an attribute delta) — extend their rule or they trip the
   completeness test.
3. **The committed fixture (`114238_2008.en`) is an EEIG notice (HEADING 02A0)**
   with a minimal `FD_EEIG` body — it does NOT exercise the `_SUM` contract form
   reuse. The "hand `FD_*_SUM` to the r209 walker" claim is inferred from the
   aggregate vocabulary, not proven. **Get a contract-heading fixture (21xx/22xx
   notice, which have `FD_CONTRACT_SUM`) and an award (33xx) before trusting the
   reuse** — those are where the 492 shared form elements actually appear.
4. `LOTS`, `SERVICES`, `AGREEMENT_PUBLICATION` are non-`_SUM` new elements whose
   role is unconfirmed from the EEIG fixture — check them in a contract fixture.

### Regeneration recipe (to re-sweep or verify)

- `/v1/sql` token: `…/aaef215c-…/scratchpad/token.json` (account owner-verify).
  Extract with `grep -oE 'tdb_[a-f0-9]{64}'`. Base `https://tenders.zebreus.click`.
  Bucket filter: `reason='unparsable-xml' AND detail='XML with DTD detected'`.
- Packages: `20080502_2008085.tar.gz` … `20080531_2008105.tar.gz` (~22 May-2008
  dailies + a small 2010 tail), in `/data/archive/ted/monthly/2008-05.tar` etc.
  on `root@zebreus.click`. Paths: `<pkg>/<num>/opoce-input/<num>_2008.<lg>`.
- Mine: extract a daily, `for f in <pkg>/*/opoce-input/*_2008.en`, slice each
  from `raw.find('<INTERNAL_OJS')` (drops the xml decl + DOCTYPE), parse with
  ElementTree, aggregate element+attr counts and `@HEADING`. Diff element names
  against `sdk/ted-export-inventory.json` (`{e['name']}`).
- The DTD strip is already done (`profile::strip_doctype`); dispatch already
  routes `INTERNAL_OJS` roots — this issue replaces that `unmapped-era` branch
  with a call into the new profile.
