# 368 — an unmapped field id or subtype is dropped with no diagnostic: 29,455 titleless r208 Tenders and whole eras of lot titles

Status: ready-for-agent — **UNIT ORDER REVISED 2026-09-08 by measurement: unit 4 (the unmapped-field diagnostic) goes FIRST.** Unit 1 as written would have mapped `TED-TI_TEXT` to `title`, which is the CPV category label in 23 languages, not the procurement's title — see "Unit 1, measured". Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
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

## Unit 1, measured 2026-09-08 — and it refutes the mapping it proposed

Unit 1 says to map `TED-TI_TEXT`, `TED-TITLE_QUALIFICATION_SYSTEM` and `TED-DESCRIPTION` into `TEXTS`.
**`TED-TI_TEXT` must NOT be mapped to `title`, and mapping it would have been the worst kind of fix:
one that makes the column look populated while carrying nothing about the procurement.**

Read off notice **17449765** (tender 5,000,205 — one of this issue's own four samples), the field
carries **23 rows, one per EU language**, and the values are the CPV category label:

| ordinal | lang | value |
| --- | --- | --- |
| 3 | DE | Dienstleistungen von Wäschereien und chemischen Reinigungen |
| 6 | EN | Washing and dry-cleaning services |
| 8 | FR | Services de blanchisserie et de nettoyage à sec |
| 9 | IT | Servizi di lavanderia e di lavaggio a secco |

Its siblings say the same thing about what the block is: `TED-TI_TOWN` and `TED-TI_CY` also carry 23
rows each (`TI_CY` is the literal `"F"` in every language). This is the OJ heading — town, country,
subject — not the procurement's own title. `TED-TI_DOC`, the sibling the existing last-resort fallback
DOES read (`crates/ingest/src/project.rs:3053-3072`), is excluded from `TEXTS` for exactly this reason,
and the comment there predicted this shape.

A CPV label as a title would be identical across every tender sharing that CPV — thousands of rows —
which is precisely what the genericness machinery exists to keep out of name and title space, and the
tender already carries the CPV as a classification. Nothing is gained and search is degraded.

**And the same notice publishes no title and no description at all.** Its complete text inventory:
`TED-TI_TOWN` 23, `TED-TI_TEXT` 23, `TED-TI_CY` 23, `TED-ADDITIONAL_INFORMATION` 12,
`TED-TOWN`/`TED-POSTAL_CODE`/`TED-OFFICIALNAME`/`TED-ADDRESS` 5 each, phone/fax 4,
weighting/order/e-mail/criteria 3. **No `TED-DESCRIPTION`, no `TED-TITLE_QUALIFICATION_SYSTEM`, no
title field of any spelling.** So for this notice the title is not dropped by an unmapped vocabulary —
it was never published.

### What that means for the units

- The **"Done when" expectation that the r208 cohort goes to ~0 is wrong** and should not be used as an
  acceptance test. Some unknown fraction of the 29,455 is genuinely untitled at source.
- `TED-DESCRIPTION` reached only **3 of the 5** titleless tenders in the 5.000–5.004M band, ~9 rows
  each, so its scope and repetition are not yet understood; `TED-TITLE_QUALIFICATION_SYSTEM` reached 3
  with exactly 1 row each, which looks like a genuine single title. Neither is safe to map on this
  evidence — one sample said one thing, the band says another.
- **Unit 4 is therefore the first unit, not the last.** The unmapped-field diagnostic is what turns
  "which spellings are we dropping, on how many notices, at what scope" from a guess into a count, and
  every other unit here depends on that answer. Ordering it last was the mistake this measurement
  found; unit 1 as written would have shipped a fabricated title to ~29,000 tenders.

## Unit 4a done (`24b676f`) — the predicate; 4b is the probe

`project::has_destination(field_id, Channel)` + `any_channel_reads(field_id)` are now real,
`pub`, and called by both test gates that used to open-code their own partial unions. Per-channel
because `role_name` accepts any `TED-` id: a channel-blind predicate reports the whole legacy era as
read, which is the one era it has to be honest about. DE-1.x alias sources resolve to their eForms
target first, or the entire eForms-DE vocabulary reads as dropped.

### 4b BUILT, then CORRECTED by its own first measurement (2026-09-09, `894f623`)

The cost question 4a left open is **answered**: over the newest ~105k notice ids the eight parsed
value tables hold **~1.0M rows** together (`notice_texts` 257,185; `notice_codes` 287,168;
`notice_ids` 229,861; `notice_classifications` 65,067; `notice_integers` 62,226;
`notice_numbers` 41,551; `notice_dates` 36,856; `notice_amounts` 23,023). Extrapolated
whole-corpus that is ~300M rows across eight `GROUP BY`s — a long scan AND the issue-278 turso
hash-state shape `longest_chain` is kept group-by-free to avoid. So it is **windowed in the SQL**
(the `fresh_holds_sql` pattern, 1,000,000 notice ids off `MAX(id)`), which also asks the more
useful question since a closed vocabulary goes stale as new spellings arrive. The eight-arm union
was then run against prod through the capped public endpoint and **returned 400 rows inside 10 s**
— cheaper than the estimate.

**And then the first real output showed the section does not answer this issue's question.**
Applying the projection's own `any_channel_reads` to those 400 rows:

| distinct published field ids in the window | 164 |
| of them, no destination at all | **109** |
| share of all published field rows they carry | **61.7 %** |

The head of that list is `BT-67(a)/(b)-Procedure` (exclusion grounds),
`BT-513/512/510(a)/507/506/503-Organization-Company` (postal address parts), `BT-539/540/5421-Lot`
(award-criterion detail), `BT-23-Lot` (main nature), `OPT-200-Organization-Company`. **Every one
correctly out of scope** — the canonical model is a narrow subset on purpose. None is a defect.

So a section listing 60 of these would have put a weekly page in front of a reader where nine in
ten entries are working as intended, which is exactly how a diagnostic earns being ignored. The
cap is now **15** and the section is named for what it actually is: the largest field volumes the
model does not hold, useful for scope decisions.

**What 368 actually needs, and why it is a separate unit.** This issue's failures were a MODELLED
concept going missing because the closed vocabulary did not know a publisher's spelling — 29,455
titleless tenders, r208's 100 %-null lot titles. The entry point for that is the completeness
section (a profile with a title gap), and *then* this list restricted to that profile. It cannot
be inferred from what 4b built, because `any_channel_reads` is profile-BLIND: a field is either
always read or never, so no per-profile asymmetry is detectable through it. Filed as unit 4c below
rather than guessed at.

### Unit 4c (new) — the targeted vocabulary diagnostic 4b turned out not to be

Join the two signals that already exist: for each profile whose completeness on a modelled field
is materially below its peers, list the field ids that profile publishes which nothing reads. That
names the missing SPELLING, which is the actionable output — "r208 publishes X where the map
expects Y" — rather than "the model does not hold postal addresses".

Needs a profile-aware destination test, since `any_channel_reads` cannot distinguish; the DE-1.x
alias resolution in `has_destination` is the existing precedent for profile-dependent mapping.

### 4b — what the probe originally needed to decide (kept for the record)

The diagnostic itself is not built. What the reading turned up about its shape, so the next firing does
not rediscover it:

- **Where.** `crates/ingest/src/data_quality.rs`. Register in `whole_corpus_queries()` (not
  `windowed_queries()`): the `notice_*` tables key on `notice_id`, and the report's windowing runs over
  `tender_versions.tender_id`, so there is nothing for `{window}` to bind to. `fresh_holds` is the
  precedent for a whole-corpus, ranked, denominator-free section.
- **Shape.** `SELECT n.profile, x.field_id, COUNT(*) … GROUP BY 1,2` per value table, then filter and
  rank **in Rust** through `has_destination`. It cannot be a SQL predicate: matching is full-id OR
  `stem` (first two dash-separated segments), and the DE-1.x alias rewrite happens in Rust.
- **`sum_profile_counts` will corrupt a three-column row** — it treats column 0 as the group label and
  `as_i64()`s everything after, so a `field_id` in column 1 becomes 0. Whole-corpus registration
  avoids it; a composite `profile || '\t' || field_id` in column 0 is the alternative.
- **Cost is the open question.** There is no index on any `field_id` (it is the 4th column of each
  table's PK), so each table is a full scan — eight tables, the largest far bigger than
  `tender_version_amounts`. `notice_id` IS available as a window key even though the report's existing
  machinery does not use it, so a notice-id-windowed variant is the way to bound it. Decide that before
  adding eight unbounded scans to a weekly job; the report's own convention is to state what it
  deliberately does not measure rather than let a reader assume completeness.
- **Mandatory touch points:** the label-order test (`data_quality.rs`
  `queries_are_labelled_in_execution_order`), the `Raw` slot + `Raw::from_labelled`, and
  `tests/data_quality.rs`'s `measure()` helper, which runs every label in `queries()`.

## Done when

- the titleless count is re-measured after re-projection and written here (the r208 cohort should go to ~0 plus the genuinely untitled);
- a lot in the 6.0M band serves its title;
- X02 notices are `kind='registration'`, not titleless procedures;
- the unmapped-field diagnostic is in the weekly report and its top entries are triaged.

*One issue because:* the 29,455 titleless tenders, the 100%-null r208 lot titles and the X02 misclassification are all "a closed hand-maintained vocabulary silently defaults instead of flagging" — three tables in two files, one missing diagnostic.
