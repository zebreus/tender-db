# 344 — the eForms-DE 1.x and DÖE sdk-0.1 parsers drop `cbc:NoticeLanguageCode`, so their versions have no `original_lang`

Status: ready-for-agent (filed 2026-09-03 from the 340 close-out read)
Kind: parse gap (era inventory) → data quality (ADR-0013 D3's third leg)
Relates to: 340 (the leg and its backfill), 88 (the UBL-* graft), 251 (era-scoped re-parse machinery)

## Observed

After the 340 backfill (job 632, every era walked), `tender_versions.original_lang`
is NULL for essentially every version caused by an `eforms:eforms-de-1.1`,
`eforms:eforms-de-1.2` or `eforms:eforms-sdk-0.1` notice — 99.9% of a 20k-tender
window at 1.5M (98.6% sdk-0.1), all of the 10% NULL share at 300k — while
`eforms-de-2.0`/`2.1` and every EU SDK version are 0% NULL.

The XML has the code. `crates/ingest/tests/fixtures/eforms/doe-sdk01-subcontract-rate.xml`
line 12 is `<cbc:NoticeLanguageCode>DEU</cbc:NoticeLanguageCode>`; the DE 1.1/1.2
fixtures under `fixtures/doe/` each carry one too. The stores do not: a prod
sdk-0.1 notice's `notice_codes` are five `SDK01-*` rows (NoticeTypeCode,
RegulatoryDomain, ProcedureCode, two country codes) and no language; a DE 1.1
notice's only language-shaped row is `UBL-DocumentLanguageID` under
`ND-CallForTendersDocumentReference#0` — the procurement documents' language
(BT-708 shape, listed in `project.rs` as "code; document plumbing"), not BT-702.

## Why it matters

ADR-0013's chain is requested → ENG → ORIGINAL → any labelled → unlabelled. With
the leg absent these versions fall to "any labelled", which for a German
single-language notice is the same answer today — but the moment a second
language exists on such a version (an EN translation, a bilingual buyer) the
pick is scan-order-among-equals instead of the notice's own language. And the
column's NULL is documented as "the era never said", which for these two eras
is false: the era said, the parser dropped it.

## Fix shape

1. In the eForms parser, claim `/*/cbc:NoticeLanguageCode` as `BT-702(a)-notice`
   for the sdk-0.1 and DE 1.x profiles the way the SDK-indexed profiles already
   do (find where `SDK01-NoticeTypeCode` is minted — the sibling root element
   that IS recorded — and add the language beside it). Pin with the two DÖE
   fixtures: `original_lang(&parsed) == Some("DEU")`.
2. Re-parse the two profile families (`reparse {profiles: [eforms-de-1.1,
   eforms-de-1.2, eforms-sdk-0.1]}` — ~7% of versions; hours, not a day).
3. Re-enqueue `backfill-original-lang` (idempotent; one transaction per batch
   since `b02a222`, so ~minutes for the remainder).
4. Re-read the 300k and 1.5M windows: NULL share → ~0 for those profiles.

Not a profile-level default ("DÖE is German, stamp DEU"): the value is
published, so read it rather than infer it.
