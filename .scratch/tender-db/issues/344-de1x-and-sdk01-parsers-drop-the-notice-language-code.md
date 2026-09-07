# 344 — the eForms-DE 1.x and DÖE sdk-0.1 parsers drop `cbc:NoticeLanguageCode`, so their versions have no `original_lang`

Status: DE 1.x DONE 2026-09-03 (`1e895b5` deployed 11:14 UTC, backfill 633 stamped every DE 1.x version in 78 s) / sdk-0.1 DECIDED 2026-09-07 (owner): no profile default — issue CLOSED (see the bottom). Was: ready-for-agent (filed 2026-09-03 from the 340 close-out read)
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

## Measured 2026-09-03 11:0x UTC — two different causes, one per era

**eForms-DE 1.x (145,859 de-1.1 + 72,986 de-1.2 notices, fetches 412–434): a leg
gap, fixed.** Prod's parsed layer already holds `PROCEDURE/DE1-NoticeLanguageCode =
DEU` (checked on a live notice); only `original_lang()` and the backfill's field
list never looked at that id. Both now list `DE1-NoticeLanguageCode` and
`SDK01-NoticeLanguageCode` beside the three era fields. Red-first test
`original_lang_eras.rs` (the two DE 1.x fixtures and the sdk-0.1 one resolve to
`DEU`; it failed before the change with the parse carrying exactly
`PROCEDURE/DE1-NoticeLanguageCode: Code DEU`), and the backfill test seeds both
ids. After the deploy, one `backfill-original-lang` run stamps the DE 1.x versions
(no re-parse needed).

**DÖE sdk-0.1 (671,879 notices, fetches 4–535): a source absence, not a parser
gap.** Sampled on prod against the archived raw members of the 2026-09-02 daily:

| | |
| --- | --- |
| sdk-0.1 notices in today's ingest carrying `SDK01-NoticeLanguageCode` | 4 of 60 sampled (33 of 445 in the window, **7%**) |
| old window (ids 468k–473k, fetch 444) | **0 of 5,000** |
| raw XML of two stored-WITH members (UUID-named, `b85934e6-…-1.xml`) | `<cbc:NoticeLanguageCode>DEU</cbc:NoticeLanguageCode>` present |
| raw XML of two stored-WITHOUT members (numeric-named, `25776650-1.xml`) | **no `NoticeLanguageCode` element at all** |

So the parser stores it exactly when the publisher sent it; the numeric-id
below-threshold shape — the bulk of the dialect — never carries a notice
language. A deterministic `DEU` for `eforms-sdk-0.1` would be right in practice
(the platform is German-only by construction) but it is an INFERENCE, and
ADR-0013's leg is "the notice's original language". Whether to add a
profile-level default (a one-line rule in `original_lang()` + a backfill re-run
≈ 10 min) is a model decision for Lennart. Until then those versions rank as
"leg absent" — which for a German-only source with German-only texts changes no
pick in practice, because there is nothing else to fall through to.

Method note: an earlier read in this session said "no NoticeLanguage code on
prod for either era" — a quoting slip (a remote loop variable expanded locally
to an empty profile string, every query matched nothing). Re-run with literal
profile strings; the DE 1.x finding above is the corrected one.

## DE 1.x half DONE (2026-09-03 11:1x UTC)

`1e895b5` deployed at 11:14 UTC (health ok); `backfill-original-lang` re-run as job
633: **walked 7,924,745 tenders in 78 s** (the one-transaction batches from
`b02a222` — the first run took 70 min), stamping every version whose notice now
resolves through the two added ids.

| window (tender ids) | before | after |
| --- | --- | --- |
| 300,000–320,000 (mixed eras) | 10% NULL (DE 1.x + sdk-0.1) | **0 NULL of 43,573**; DE 1.x versions read DEU 3,477 / ENG 13 |
| 1,500,000–1,520,000 (the sdk-0.1 era) | 19,986 NULL of 20,000 | 19,752 NULL of 20,000 — every remaining NULL is `eforms-sdk-0.1` |

So the leg is complete for every era that publishes a notice language: r207–r209,
the text era's `OL:` line, EU and DE 2.x eForms, and now DE 1.x. What is left is
exactly the sdk-0.1 residue, which is the source's silence, not ours — the
decision item above. Test coverage: `original_lang_eras.rs` pins the three
national fixtures; the backfill test seeds both ids.

## Decision (2026-09-07, owner): no profile default for sdk-0.1

93 % of DÖE sdk-0.1 notices publish no language element. Stamping `DEU` by profile
would make `original_lang` say "the notice said German" where the notice said nothing,
and the column's documented NULL ("the era never said") would become a lie for the one
era where it is literally true. The fallback chain's labelled leg already serves the
German text for these single-language notices, so nothing is lost at the read surface.
The 7 % that do publish a code are stamped by the DE 1.x fix's shared path. Closed.
