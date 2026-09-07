# 368 — an unmapped field id or subtype is dropped with no diagnostic: 29,455 titleless r208 Tenders and whole eras of lot titles

Status: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
Kind: defect (projection destinations) — the recurring 18/85/177/231 shape, plus the
standing detector none of them had
Relates to: 85 (DE-1.x facts), 18 (sdk-0.1 instants), 177 (r208 values), 231 (sdk-0.1
CPV/amounts), 41 (the vocab sweep tsv that already counted LOT_TITLE 427× across six
packages), 109 (the factless-version probe that cannot see these), 343/291 (which title
is picked, once one exists)

## Observed

**30,281 titleless tenders, and the sampled X01 explanation covers 131 of them.** One full-table aggregate: total 7,929,584; `current_title` NULL or `''` = **30,281**; `kind='registration'` = **131**; registration AND titleless = 131. X01 registration notices are documented and deliberate (CONTEXT.md:141, `crates/ingest/src/project.rs:703` and :3215-3220), and self-describing on `/v1/tenders?kind=registration`.

**Where the other 30,150 are.** Bounded PK-range counts of `current_title IS NULL`: 0–1M → 0; 1.0–1.5M → ~428; 1.5–3.0M → 78; 3.0–3.5M → 0; 3.5–5.0M → **11,817**; 5.0–6.5M → **17,638**; 6.5M–end → 316. The 29,455 in 3.5–6.5M are one cohort: source `ted`, kind `procedure`, `notice_subtype` NULL, profile `ted-export-r208`, published 2010–2016 and 2013–2019.

**Why.** Four sampled (ids 5,000,205 / 5,000,221 / 5,000,299 / 5,000,301 = publications 186997-2013 / 187024-2013 / 187180-2013 / 187182-2013) are all single-version (head = maxseq = 1), so no supersede is involved. Their notices' text field ids include `TED-TI_TEXT`, `TED-SHORT_CONTRACT_DESCRIPTION`, `TED-TITLE_QUALIFICATION_SYSTEM`, `TED-DESCRIPTION`, `TED-AA_NAME` — and **none of** `TED-TITLE`, `TED-TITLE_CONTRACT`, `TED-CONTRACT_TITLE`, the only three title ids the projection maps. `grep -c` for `TED-TI_TEXT`, `TED-TITLE_QUALIFICATION_SYSTEM` and `TED-DESCRIPTION` in `crates/ingest/src/project.rs` → **0 each** (re-verified 2026-09-07).

**Lot titles, same cause.** `lots.id` 6,000,001–6,000,100 → 100 lots, **100 NULL titles** (lot_key range LOT-1..LOT-9); 12,000,001–12,000,100 → **48 NULL** of 100. Worked example lot 6,000,001 (tender 5,427,171, `ted-export-r208`): seq 1 = notice 18278250 (`143065-2015`) with 34 Lot sections that DO carry the titles — as `TED-LOT_TITLE` / `TED-LOT_DESCRIPTION` ("Benzina rettificata F.U.", "Perossido di idrogeno flaconi…", 15 rows sampled) — and `SELECT seq, COUNT(*) FROM tender_version_texts WHERE tender_id=5427171 GROUP BY seq` returns 2 texts at each seq with **0 lot-scoped texts at both**. `grep -c 'TED-LOT_TITLE'` and `'TED-LOT_DESCRIPTION'` in project.rs → **0**, while `crates/ingest/src/r209/rules.rs:336` parses `LOT_TITLE` / `LOT_DESCRIPTION` as free text. (The 48% at 12.0M is a different, benign case: notices 22557275 / 22880457 publish Lot sections whose text ids are `TED-EU_PROGR_RELATED` / `TED-MAIN_SITE` / `TED-SHORT_DESCR` with no title present, while notice 23021307's titled lots carry `TED-TITLE`. Genuinely unpublished.)

**A second closed vocabulary.** In ids 1.0–1.5M the titleless rows break down as procedure/X02 **285** (across sdk-1.8 … sdk-1.14), registration/X01 119, procedure/NULL-subtype 24 (sdk-0.1). X02 is a BRIN too — the repo's own fixture `crates/ingest/tests/fixtures/eforms/brin-eu-00568126-2023.xml` carries `<cbc:SubTypeCode listName="notice-subtype">X02</cbc:SubTypeCode>`, and its publication `00568126-2023` is tender 1,167,207, one of the sampled titleless "procedure" rows; the SDK's own field metadata pairs them (`'noticeTypes': ['X01','X02']` throughout `crates/ingest/sdk/fields-1.8.0.json`).

**Lot key formats**: `lot_key` in ids 1000–21000 → `LOT-0001` 5,578, `LOT-0000` 2,018, `LOT-0002` 1,742 …, while the r208/r209 bands hold `LOT-1..LOT-9`. `LOT-0000` is genuinely published (a `notice_sections` row with section_id `LOT-0000` exists in the source notices for lots 1002/1003/1010, profiles sdk-1.7 / sdk-0.1 / sdk-1.13).

## Why, exactly

Two closed, hand-maintained vocabularies, each with a **silent default**.

- `TEXTS` (`crates/ingest/src/project.rs:92-124`) is an allow-list of field ids. The parse layer stores every published id faithfully; a value whose id is not in the list has no destination and is dropped with no diagnostic. The comment sitting directly above the three mapped title ids — "legacy R2.0.7–R2.0.9 (title 100% fill, research §5.1)", `crates/ingest/src/project.rs:95` — is contradicted by 29,455 rows, and because nothing re-derives it, it has stood as documentation of an untrue state.
- `kind_of` matches one literal: `const REGISTRATION_SUBTYPE: &str = "X01"` (`crates/ingest/src/project.rs:703`) consulted at `crates/ingest/src/project.rs:3215-3220` with catch-all `_ => "procedure"`. It keys on a subtype *string* rather than on the class of notices that carry no procurement project, so X02 is minted as a procurement procedure and surfaces titleless on `/v1/tenders`. X02's exclusion elsewhere is deliberate and tested ("X02 is unclassified by design, so the diagnostic can show it", `crates/ingest/src/data_quality.rs:2782`) — that decision was taken for the award/doc-type report and never revisited for the canonical `kind`.
- **Nothing detects either.** No gate asserts that a projected procedure Tender has a title, and issue 109's factless-version probe fires only when ALL satellites are empty — these versions carry dates, parties and classifications.
- **Correction to the review's reading of the lot half** (worth stating, because a fix built on it would miss): the fold DOES carry lots and their facts forward — `let mut lots: Vec<LotState> = previous.map(|p| p.lots.clone()).unwrap_or_default();` then `supersede(&mut carried.facts, &published.facts)` matched by lot key, `crates/ingest/src/project.rs:3402-3411`. So "the head notice has no Lot sections" is why the head is thin, but not why the title is missing: a title published at seq 1 would survive to the head. The titles are missing because `TED-LOT_TITLE` was never mapped in ANY version — the same cause as the tender titles, which is why both belong in this one issue.
- **The lot keys** are two published plus one synthetic: `LOT-0001` / `LOT-0000` are stored as published by policy ("Lot identity inside a Tender: the published id and nothing else", `crates/store/src/canonical.rs:204-210`), while r208/r209 lots have no published id and the parser MINTS `LOT-1..n` (`crates/ingest/src/r209/parse.rs:8-16`) in a shape that does not match the published convention — an internal inconsistency nobody chose, unlike the published two.

## Units

1. Map the r208/r209 title/description spellings into `TEXTS`: `TED-TI_TEXT`, `TED-TITLE_QUALIFICATION_SYSTEM`, `TED-DESCRIPTION` — read `.scratch/tender-db/issues/41-vocab-sweep.tsv` for the full inventory before choosing. Re-project the 3.5–6.5M band and re-count.
2. Map `TED-LOT_TITLE` / `TED-LOT_DESCRIPTION` at Lot scope; re-read lots 6,000,001–6,000,100.
3. `kind_of` keys on the BRIN class, not the literal — X01 and X02 both, with the SDK's `noticeTypes` pairing as the source of truth. Pin with the existing `brin-eu-00568126-2023.xml` fixture.
4. **The detector all four (and `notice-instants-resolved-once-and-flattened`) were found without**: a projection diagnostic counting parsed values whose field id has no destination, per profile and field id, top N in the weekly report. This is the standing signal that turns "found by an external reviewer" into "flagged the week the era was ingested".
5. Decide the synthetic lot-key shape (`LOT-1` vs `LOT-0001`) or record why the inconsistency stands.

## Done when

- the titleless count is re-measured after re-projection and written here (the r208 cohort should go to ~0 plus the genuinely untitled);
- a lot in the 6.0M band serves its title;
- X02 notices are `kind='registration'`, not titleless procedures;
- the unmapped-field diagnostic is in the weekly report and its top entries are triaged.

*One issue because:* the 29,455 titleless tenders, the 100%-null r208 lot titles and the X02 misclassification are all "a closed hand-maintained vocabulary silently defaults instead of flagging" — three tables in two files, one missing diagnostic.
