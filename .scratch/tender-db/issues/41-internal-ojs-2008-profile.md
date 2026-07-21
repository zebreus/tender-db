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
