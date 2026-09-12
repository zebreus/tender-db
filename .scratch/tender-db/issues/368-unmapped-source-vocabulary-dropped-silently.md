# 368 — an unmapped field id or subtype is dropped with no diagnostic: 29,455 titleless r208 Tenders and whole eras of lot titles

Status: ready-for-agent — **UNIT 4b WAS INERT AND IS NOW FIXED 2026-09-10 (`c5d6e79`): the query ran every week and had NO render section, so the diagnostic showed nobody anything. See the last section, including the source-reading guard that now holds it.** Units 1-3 unblock on the next weekly run. Was: **UNIT ORDER REVISED 2026-09-08 by measurement: unit 4 (the unmapped-field diagnostic) goes FIRST.** Unit 1 as written would have mapped `TED-TI_TEXT` to `title`, which is the CPV category label in 23 languages, not the procurement's title — see "Unit 1, measured". Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
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

## Unit 4b was INERT until 2026-09-10 (`c5d6e79`) — built, deployed, marked done, rendering nothing

The query was registered in `whole_corpus_queries()`, ran on every weekly report, and its rows were
filtered through `any_channel_reads` and capped into `Report::unmapped_fields`. **`render_text` never
mentioned the field.** So the diagnostic paid its scan every run and produced nothing a reader could
see, for as long as nobody looked.

**How it was found, which matters more than the fix.** The first report carrying section 12 came back
and I searched it for the unmapped-field listing to advance units 1–3, which depend on that answer.
There was none — no header, no rows, not even an `UNMEASURED` line, while the job reported *0 labels
unmeasured*. The label HAD run. Walking `Report`'s 19 fields against the renderer's body found exactly
one that nothing read.

**Nothing about the query or the assembly was wrong**, which is why no test caught it: every unit test
here asserts on `Raw`, on the SQL text, or on the assembled `Report`, and all of those were correct.
The gap was between a correct `Report` and the text a person reads, and no assertion spanned it.

**The guard now does.** `every_report_field_is_read_by_the_renderer` reads this file's own source and
fails if any `Report` field is assembled and never rendered. It has to read the SOURCE: a field that
renders nothing is invisible to any assertion about output, so the property cannot be stated in terms
of the text. Same shape as the head-column writer-count test in `store`, for the same reason.
Negative-checked by deleting the new section and watching it name `unmapped_fields`.

A second guard holds the sections to ascending contiguous numbers. Section 12 had been added beside
the query it belonged to, which put it AHEAD of section 11 — the first real report printed 10, 12, 11.

**Units 1–3 are still blocked on the numbers**, but no longer on the instrument: the next weekly run
renders section 13, and the listing is the cohort those units need. The prior measurement stands (109
of 164 published ids had no destination), and the cap of 15 is why the section is a queue rather than
a wall.

## Section 13 verified on prod (2026-09-10, job 1005) — and the render was mis-framed

The section renders, in order, with real data. Its top 15 reproduce the 2026-09-09 hand-read exactly:

| profile | field id | window rows |
| --- | --- | --- |
| sdk-1.13 | `BT-67(a)-Procedure` | 63,814 |
| sdk-1.13 | `BT-67(b)-Procedure` | 54,510 |
| sdk-1.13 | `OPT-200-Organization-Company` | 38,194 |
| sdk-1.13 | `BT-512/513/507/506/503-Organization-Company` | 38,193 … 34,971 |
| sdk-1.13 | `BT-23-Lot` (main nature) | 37,576 |
| sdk-1.13 | `BT-5141-Lot`, `BT-540-Lot`, `BT-539-Lot` | 36,931 / 36,808 / 36,767 |
| sdk-1.13 | `OPT-321-Tender`, `BT-3201-Tender`, `BT-773-Tender` | 34,200 / 34,200 / 33,668 |

Exclusion grounds, postal-address parts, award-criterion detail, main nature, tender identifiers.
**The same set this issue already hand-read and found correctly out of scope.**

**The render was wrong about them, and it was my wording.** It said a field here "is dropped
silently — the 18/85/177/231 shape, with no standing detector until this one". That reads as a defect
list. It is not one: the canonical model is a narrow subset ON PURPOSE, and a page where nine entries
in ten are working as intended is how a diagnostic earns being ignored — which is the exact reason
this issue set the cap to 15 in the first place. Corrected in `1447b45`, with a test pinning it.

The section now says two things it must: **being listed is not a defect**, and **this is not the
detector for 368's own failures** — `any_channel_reads` is profile-blind, so a field is either always
read or never and no per-profile asymmetry can show through it. That entry point is unit 4c.

**Third time today the same mistake, in three different sections.** 364's ">=50 is safe" was an
assertion in the shape of a calibration; its cost figure was borrowed from a run that predated it;
this one described a scope list as a defect list. None was caught by a test, because all three were
prose. Two were caught by the first data they met and one by re-reading the issue that had already
measured it. The pattern is worth naming: **the numbers in these sections get measured, and the
sentences around them do not.** Each now has a test asserting what the sentence may claim, which is
the only mechanism that has actually held.

## Unit 4c BUILT (2026-09-11, `9943fef`) — a per-profile quota, because the section showed one profile

The first live section 13 returned **15 of 15 rows under `eforms:eforms-sdk-1.13`**. The rows arrive
sorted by raw hits and that era is the largest, so the listing had no room for anything else — a
section whose entire purpose is per-profile was showing one profile, and this issue's own failures
(r208's 100 %-null lot titles, the 29,455 titleless tenders) live in SMALL eras that could not have
appeared at any cap.

Each profile now takes at most **2** of the 15 (`UNMAPPED_FIELD_PER_PROFILE`), the per-scope quota
issue 347 established for the org census. The total cap is unchanged, so this buys breadth out of
depth rather than out of the decision that set it.

**What this does and does not do for units 1–3.** It makes a small profile's unmodelled spellings
*visible*, which is the precondition. It does not tell you which of them is a missing MODELLED concept
— `any_channel_reads` is profile-blind, so that still requires reading the completeness section for a
profile with a gap and then looking up that profile's rows here. The section is now able to answer
that question; before it could not.

*One thing the test caught, worth keeping:* the fixture's first draft used `TED-LOT_TITLE` as a small
era's unmodelled field. That id IS modelled, so the reads filter removed it before the quota saw it —
the test would have measured the filter and passed for the wrong reason. The fixture uses ids nothing
reads and says why.


## Unit 1 ANSWERED (2026-09-11): it is a family of FOUR form-specific title elements, none of them mapped

The two weekly runs unblocked this (jobs 1305 and 1306). The answer is not the one unit 1 was written
with, and not the one the 2026-09-08 correction left either.

### The gap is still real and still r208's

| | |
| --- | --- |
| Tenders with no `current_title` | **30,285** |
| …on `ted-export-r208` | **29,763 (98.3 %)** |
| next largest profile | eforms-sdk-1.13, **119** |

Section 1 agrees: r208 is the only era below 99.9 % on title, at **98.0 %**.

*(A five-row sample ordered by tender id showed eForms profiles and looked like a contradiction. It
was not — low tender ids skew eForms. The grouped count is the reading; the five rows were the
sampling trap this repo keeps documenting.)*

### What the titleless notices publish

Over **300 titleless r208 notices** (a sample, taken in scan order, notice ids spanning
4,352,732–27,161,161):

- **None of the four mapped title fields appears** — not `TED-TITLE`, `TED-TITLE_CONTRACT`,
  `TED-CONTRACT_TITLE`, nor `TXT-TI`. So it is not a case of one of them being spelled differently.
- Every notice carries `TED-TI_TEXT` / `TI_TOWN` / `TI_CY` at 23 rows each — the OJ heading, which
  the 2026-09-08 correction already established is the CPV category label and is correctly unmapped.
- **148 of the 300 — 49 % — carry a form-specific title element that nothing reads:**

| field id | notices (of 300) |
| --- | --- |
| `TED-TITLE_QUALIFICATION_SYSTEM` | 65 |
| `TED-TITLE_RESULT_DESIGN_CONTEST` | 40 |
| `TED-TITLE_DESIGN_CONTACT_NOTICE` | 38 |
| `TED-TITLE_NOTICE_BUYER_PROFILE` | 5 |

All four are parsed — they sit in `r209/rules.rs`'s element list — and none has a projection
destination. `project.rs`'s title map holds seven ids and none of these.

**This IS this issue's stated shape**, with one word changed: *"a MODELLED concept going missing
because the closed vocabulary did not know one publisher's spelling"* — except the spelling belongs
to a FORM, not a publisher. A qualification-system notice, a design-contest result and a
buyer-profile notice each name their subject in their own element. The concept is the same one.

### Why this needed hand queries, which is the diagnostic's remaining gap

Section 13 is **windowed to the newest 1,000,000 notice ids**, so it cannot see r208 at all — the one
profile with the title gap is outside the diagnostic built to find such gaps. The section's own note
names the entry point ("the completeness section, then this list restricted to that profile") and
that path does not work for a legacy profile. Sizing this corpus-wide also exceeds the 10 s read cap,
so it needs to run inside the job.

### Next units

1. **A per-profile arm for section 13 that is not windowed**, or a window keyed to the profile rather
   than to the corpus head. Without it, every legacy-era gap is invisible to the instrument.
2. **Map the four**, once (1) has sized them corpus-wide. Care needed on the destination: a
   qualification system's name is arguably the Tender's title for that form, but
   `TITLE_RESULT_DESIGN_CONTEST` may name the CONTEST rather than the procurement. Read one of each
   before choosing, the way `TED-TI_TEXT` was read before being rejected.
3. **The other half.** 152 of the 300 carry no title element of any kind; for those the gap is
   publisher omission and the honest answer is that no title exists.

## The per-profile arm was built, SHIPPED, MEASURED and REVERTED (2026-09-12)

`9073eca` added it; `5b0eb4c` reverted it. What follows is the measurement, including a correction to
what I first claimed about it.

### The cost, measured on prod (job 1313)

| | |
| --- | --- |
| whole job | **11,457 s** (3 h 11 m) |
| windowed phase (32 windows × 16 queries) | 5,771 s |
| whole-corpus phase **with** the per-profile arm | **~5,686 s** |
| whole-corpus phase on the two runs before it | ~1,300–1,600 s |
| **attributable to the new query** | **~4,000 s (67 min)** |

**Correction to my own revert message.** `5b0eb4c` says *"the weekly report would not have
finished"*. That is false — job 1313 completed, `ok`, `0 label(s) unmeasured`. I cancelled it while
it was still inside the query and wrote the revert from that observation instead of waiting for the
number. The honest statement is a **55 % increase on a two-hour job for one diagnostic listing**,
which is still a bad trade, and not the same claim.

### Why it is still reverted

67 minutes buys one listing, and the cost is a design error rather than an inherent price. The
corpus arm's floor is a **scalar constant**, so `notice_id > <const>` is an index range on each
table's primary key. Joining to a per-profile maximum makes the floor **join-dependent**, which
defeats that index; and `GROUP BY profile` over 31 M notices has no index to serve it, because
`notices` is not keyed on profile.

### What a redesign must preserve

**The scalar floor.** Two candidates, both to be MEASURED before shipping this time:

1. **Compute the per-profile floors in a prior pass** and emit them as literal constants — a union of
   ~24 explicit id ranges, each an index range. Needs the job to support a query whose text depends
   on an earlier result, which `queries()` does not do today.
2. **An index on `notices(profile, id)`**, which turns the maxima into a cheap skip-scan over ~24
   groups. Cheaper to build, but it does not fix the join-dependent floor — worth measuring whether
   the floor alone was the cost or the grouping was.

### The requirement has not changed

r208 still holds **29,763 of the 30,285** titleless Tenders and its newest notice is still 4.3 M ids
below the corpus window's floor. The profile with the gap is still invisible to the diagnostic built
to find gaps, and the four form-specific title elements still have no destination. Only the
instrument for seeing them corpus-wide is missing.

## The per-profile arm failed TWICE, and both failures were the same mistake (2026-09-12)

`9073eca` shipped it, `5b0eb4c` reverted it, `61f18dd` re-landed it with a fix, and it is reverted
again. Two hypotheses, two ~3-hour runs, both wrong. **The common cause is not the SQL — it is that I
measured a PROXY for the statement instead of the statement.**

| attempt | hypothesis | what I measured | time on that step |
| --- | --- | --- | --- |
| 1 (`9073eca`) | — (shipped without measuring; cost flagged as the open risk) | nothing | ~67 min |
| 2 (`61f18dd`) | the window was one-sided, so each profile scanned to the end of the corpus | a CONSTANT-floor range on ONE table: `BETWEEN` 3.5 s vs `>` over 10 s | **~58 min and counting** |

Attempt 2's measurement was real and it was about a different query. The real statement's floor comes
from a JOIN (`h.max_id - W`), not from a constant, and that is exactly the property the probe could
not exercise — a constant-floor range is an index seek whether or not it has a ceiling. **The
two-sidedness was never the variable.** I tested the thing I could test cheaply and treated it as
evidence about the thing I could not.

That is `docs/agents/prod-box-reads.md`'s sampling rule — *pick the sample that contains the
phenomenon, then check that it does* — in a performance costume, and it is the third time in this
session's work that the same shape has cost real time.

### What is actually established

- The corpus arm (`unmapped_fields_sql`, constant floor) costs ~1,300–1,600 s for the whole
  whole-corpus phase, i.e. it is not the problem.
- The per-profile arm costs **~60 minutes on its own** in both the one-sided and two-sided forms.
- Therefore the cost is in the **join-dependent floor**, the **`GROUP BY profile` over 31 M notices**,
  or both — and NOTHING measured so far separates those two.
- `notices` carries `notices_profile(profile)`, a single-column index. It is NOT covering for
  `MAX(id)`, so the grouped subquery still fetches every row it walks.

### The rule for the next attempt, which is not optional

**Measure the REAL statement before writing any more of it.** `/v1/sql` refuses `EXPLAIN`, so this
needs `sqlite3` against a SNAPSHOT per `docs/agents/prod-box-reads.md` — that is the documented path
for exactly this and it should have been the first step, not the fourth. What to get:

1. `EXPLAIN QUERY PLAN` of the real statement, to see whether the notice-id predicate is a range scan
   or a full scan, and what the subquery does.
2. The same for the corpus arm, as the control that is known to be fast.
3. A timing of the grouped-maximum subquery ALONE, which separates the two candidate causes.

Only then choose between the candidates (a covering `notices(profile, id)` index; per-profile floors
computed in a prior pass and emitted as constants; or dropping the idea). **Do not ship another
attempt on a hypothesis.**

### The requirement is unchanged and still unmet

r208 holds 29,763 of the 30,285 titleless Tenders, its newest notice is 4.3 M ids below the corpus
window's floor, and the four form-specific title elements still have no destination. The gap is real.
The instrument for it is not built, and three-quarters of the cost so far has been avoidable.
