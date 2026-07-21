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
