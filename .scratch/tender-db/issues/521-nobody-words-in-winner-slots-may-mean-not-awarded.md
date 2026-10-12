# 521 — "nobody" words in a winner slot (`ingen`, `néant`, `geen`, `aucun`, `niet van toepassing`) may mean "not awarded"

Status: ready-for-agent — triaged 2026-10-12, low priority. The claims hold, but the 510-style per-org reads no longer work: unit 3 swept the néant/aucun/geen/niet-van-toepassing orgs and retired their mentions. `ingen` is mostly a French name, which the language guard keeps on purpose. NEXT (M): a read-only dry census job over the legacy-era results (r208/r209/internal-ojs/text) on French, Dutch, Danish, Swedish and Norwegian notices. Output: per word × profile, how many results have a value (=0 / >0), a date only, or neither, plus the full notice list. Then decide per word (S), using a winner-slot marker rather than VOID_WHOLE.
Was: needs-triage — filed 2026-10-12 with issue 511 unit 2 (decision §8.7, owner ruling 16).
Kind: data quality (results)
Relates to:
- 511 (these words are `Blank`, so the existing evidence rule decides);
- 510 (`VoidLot` and its per-word value/date table).

## What is open

511 classes the "nobody" words as `Blank`. With that class, a legacy result whose only winner slot reads one of them
is decided by the existing evidence rule:
- a value → `selec-w`;
- no date → `clos-nw`;
- date only → NULL.

Promoting a word to `VoidLot` would mark those results `clos-nw` outright, outranking a value or a date. That is
correct only if the word, in a winner slot, means "the lot was not awarded" rather than "the name is missing".

Census rows in winner slots (511 census, 2026-10-11): `ingen` 461 admitted, `néant` 458, `niet van toepassing` 117,
`geen` 110, `aucun` 100.

## Proposed unit

Build 510's per-word table for these words. For each word, the count of its winner-slot results that carry:
- a value (and how many are exactly 0);
- a date only;
- neither.

Then decide per word, as 510 did for its whole values: a word whose results are mostly value-less and dateless joins
`VoidLot`.

## Triage (2026-10-12, owner; panel wf_73cc2c40-e1c)

Triage 2026-10-12. The claims hold against the code. `neant`, `geen` and `aucun` are in `BLANK_WHOLE`. `niet van toepassing` is a `BLANK_STEMS` stem. `ingen` is a scoped word, Blank only on Danish, Swedish or Norwegian notices (partyname.rs:241, 342, 429). `read_legacy_results` gives a result whose only winner is Blank no marker. So a value gives `selec-w`, no date gives `clos-nw`, and a date alone gives NULL (project.rs:6524-6531, 6590-6604). sdk-0.1 stays NULL unless coded, and eForms/FTS keep BT-142, so only the legacy eras can move.

Two things have changed since the census.

(1) Unit 3's drain (fold 2271, sweep 2272) removed almost all of four words' orgs. Org rows by name_norm: néant 495→2, neant 38→1, aucun 76→0, geen 98→0, niet van toepassing 100→0. The hubs 1166840, 1220397, 10006727, 1581670 and 1702621 now return 404. `retire_mentions` deletes the mention and party rows and keeps no log, and nothing indexes a raw name back to its notice. So the 510-style per-org reads this issue implies are no longer possible.

(2) `ingen`'s 461 is overstated. 459 of the census's 518 `ingen` orgs survive. 9 of 9 sampled sit on French notices (TED-LG_ORIG/TXT-OL = FR: 4203205, 12461203, 16900929, 18650373, 2258987, 3489249, 3249815, 4175743, 3134883). The language guard correctly keeps those as a name, so the Danish/Swedish/Norwegian `ingen` cohort is a few dozen orgs at most. The same sample answers 511's open item on 1188404's language: French.

The live cohort is about 785 legacy-era winner rows. The dateless ones already read `clos-nw`. A promotion would move only those with a value or only a date, about 15% on 510's French hubs, so on the order of 120 lot results.

NEXT (M): a read-only census job that only counts. It walks legacy lot-result sections on French, Dutch, Danish, Swedish and Norwegian notices and classifies each winner name with `partyname::not_a_name_in(name, original_lang)`. For results whose only winners are a nobody word, it counts per word × profile: value = 0, value > 0, date only, neither. It records 20 sample notice ids per bucket and the full notice list, which becomes the refold cohort because `refold-placeholder-names` cannot find orgs that are gone. Run it before any text reparse, which would erase the text-era slot.

Then decide per word (S). If a word promotes, do not add it to `VOID_WHOLE`. It fails 510's admission rule (no use in other roles), and `VoidLot` skips guard 3, the Blank-only identifier guard (project.rs:4600). Instead, add a winner-slot `nobody` marker that `read_legacy_results` reads, then run `refold-notices` over the census list.

Add a `## Verify`. Suggested: `grep -c '^## Decision' .scratch/tender-db/issues/521-nobody-words-in-winner-slots-may-mean-not-awarded.md`, which prints 0 while open and 1 once decided. Low priority.

**Claims:** Mostly yes. The code mechanics and the census figures are correct. Two premises are out of date, and one gap needs fixing. (1) The proposed unit assumes the 510-style method: bounded per-org reads on the hub orgs. That method no longer works. Issue 511 unit 3's drain (fold 2271, sweep 2272) dropped almost every néant/neant/aucun/geen/niet-van-toepassing party. `retire_mentions` deletes the mention and party rows and keeps no log, and nothing indexes a raw name back to its notice. (2) The `ingen` figure of 461 is overstated. Most of those rows are a name on French notices, which the language guard keeps on purpose, so they are not Blank. (3) The issue has no `## Verify` block, which the tracker requires.

**Evidence:** CODE
- partyname.rs:241: `BLANK_STEMS` includes " niet van toepassing".
- partyname.rs:342: `BLANK_WHOLE` includes neant, geen, aucun and aucune. Its doc comment (lines 331-335) cites 511 §8.7.
- partyname.rs:429: `SCOPED_WHOLE` limits `ingen`/`inget` to DAN, SWE, NOR, NOB and NNO notices.
- project.rs `read_legacy_results`, lines 6524-6531: a Blank winner sets no marker (`Some(_) => {}`).
- project.rs lines 6590-6604: a real winner or a placeholder gives `selec-w`; `void_lot` gives `clos-nw`; otherwise a value gives `selec-w`, no date gives `clos-nw`, and a date alone gives NULL. This matches the issue. `void_lot` does outrank value and date.
- sdk-0.1 (lines 6679-6687): a Blank winner stays NULL unless a code is published. eForms and FTS keep BT-142. So only the legacy eras can change: r208, r209, internal-ojs and text.
- project.rs:4600: guard 3, the identifier guard, applies to `Blank` only.
- canonical.rs:19343 `retire_mentions`: DELETEs the `organization_mentions`, `tender_version_parties` and `tender_version_bid_parties` rows.
- Indexes: `notice_texts` has none on value; the only `notice_ids` index is on is_ref values.
- text/parse.rs:143-155: the text parser refuses every class at parse time, so a text-era slot disappears on a reparse.

CENSUS (census.md lines 134-166; unit1-decision.md lexicon)
- Winner/bidder rows: ingen/inget 461 admitted (600, less 137 on 1188404 FR, less 2 `InGen`); néant+neant 458; niet van toepassing 117; geen 110; aucun 100. These match the issue.

PROD READS, 2026-10-12 (all bounded)
- Org rows by name_norm, census then now: néant 495→2, neant 38→1, aucun 76→0, geen 98→0, niet van toepassing 100→0, ingen 518→459. Of the 459 `ingen` rows, 456 have no country, and 58 have ids ≥31.9M, i.e. minted in the last few days.
- `/v1/organizations/{id}`: hubs 1166840, 1220397, 10006727, 1581670 and 1702621 return 404. 1889418 Ingen DK returns 200 with 1 mention (census: 274). 1188404 Ingen FR still has 134 mentions.
- I sampled 9 surviving `Ingen`/`INGEN` winner mentions: orgs 31919248, 1211974, 1946579, 7752221, 25694698, 29115545, 30379757, 31785285, 31917805. All 9 are on French-language notices (TED-LG_ORIG or TXT-OL = FR): 4203205, 12461203, 16900929, 18650373, 2258987, 3489249, 3249815, 4175743, 3134883. The ORG sections hold only `TED-OFFICIALNAME`.

PRECEDENT
- 510's French hubs had 14-16% of rows with a value or only a date: infructueux 547 of 3,570; Sans suite 322 of 2,265; Lot infructueux 184 of 1,135.

**First unit (M):** Build a read-only census job that only counts (dry) and run it. This is the 510 per-word table, rebuilt from the parse layer because the mention rows are gone.

What the job walks:
- Lot-result sections of the legacy profiles (r208, r209, internal-ojs, text), in cursor slices on `notice_sections_kind_notice`. Profile and language are read from `notices.profile` and TED-LG_ORIG/TXT-OL by PK seeks.
- Only notices in FRA, NLD (both cover BE), DAN, SWE, NOR, NOB and NNO.

Per result:
- Resolve the winner references to their names (ORG_NAME_FIELDS).
- Classify each name with `partyname::not_a_name_in(name, original_lang)`.
- If every winner name is one of néant/neant, aucun/aucune, geen, niet van toepassing or ingen/inget, count the result by word × profile in four buckets: value = 0, value > 0, date only, neither.

Output:
- Up to 20 sample notice ids per bucket.
- The full notice-id list. This is the drain cohort for a later refold-notices: `refold-placeholder-names` walks org head names, and these orgs no longer exist.

Build notes:
- Wrap the new `Spec` arm in `Box::pin`.
- Read the text era before any text reparse, which would erase the slot.

Deliverable: the table in the issue, with the decision as the next S unit. A word promotes when its results are mostly without a value and without a date. A promotion should be a winner-slot `nobody` marker that `read_legacy_results` reads (`void_lot = true` when the word is the only winner), not an entry in `VOID_WHOLE`. Adding the words to `VOID_WHOLE` would break 510's admission rule (no use in other roles) and would bypass guard 3.

**Value:** What is wrong today:
- Legacy-era lot results whose only winner slot read néant/neant (458 winner rows in the census), niet van toepassing (117), geen (110) or aucun (100): about 785 rows. Since the unit 3 drain they no longer have a winner and are decided by the evidence rule.
- The dateless ones already read `clos-nw`, which is what a promotion would also give.
- Only two groups would change if promoted: the ones with a value (now `selec-w` with no winner bound) and the ones with only a date (now NULL).
- On 510's French hubs that share was 14-16%. So the expected change is about 100-150 lot results, with 785 as the hard upper bound. The exact figure is unknown until the census runs.

`ingen` adds little:
- 459 of the census's 518 `ingen` orgs survive, and 9 of 9 sampled sit on French notices, kept as a name.
- The Danish/Swedish/Norwegian `ingen` winner rows that were actually dropped belong to at most about 60 orgs.

Risk: a wrong promotion would claim `clos-nw` over a value the publisher printed. The current Blank reading claims nothing beyond the published evidence. So this is low value and low urgency.

**Overlap:** Not a duplicate. This issue is split out of 511's unit 1 "still open" item, "whether to promote the nobody words to VoidLot (needs 510's per-word value/date table)". That line in 511 should point to 521.

The same sample also answers another 511 open item, "1188404 Ingen FR (language unread)": its notices, and the about 456 surviving no-country `Ingen` singletons, are French-language. Whether "INGEN" is a real French firm, and whether those singletons should be one org, belongs to 511 (or 351), not here.

Related but separate:
- 510 (done): the VoidLot class and its per-org table.
- 512: the text-era no-award Code marker.
- 520: award-summary names standing in for winners.

**Rank:** 2/10 across 520/521/523/524. Low value for an M build. The cohort is about 785 legacy winner rows as an upper bound. The dateless ones already read `clos-nw`, so a promotion would flip only results that carry a value or only a date. That is about 15% on 510's precedent, roughly 100-150 lot-status values on old legacy results.

The `ingen` 461 is mostly French names, which the language guard keeps on purpose. The 510-style per-org method stopped working when 511 unit 3 retired the mentions, so the next unit has to be a new census job: a new Spec arm (stack risk, Box::pin), the gate and a deploy.

The downside is real. A wrong promotion would mark lots `clos-nw` over a value the publisher printed, while today's Blank reading claims nothing beyond the published evidence. The triage's proposed Verify (grep for a Decision heading) checks process, not data. It is only time-sensitive if a text-era reparse is scheduled, and none is. Park it at low priority.
