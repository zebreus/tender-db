# 473 — the non-GB wrong-identifier and shared-identifier residue has no owner: 362's held groups, two admitted-but-unmerged RO groups, the 357/359 shapes, Deutsche Bahn file numbers, the shared Bavarian VAT

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit re-measures each class below with one bounded read each (Proposed fix, step 1; four of the reads were taken at filing and stand here as the baseline) and records, per class, whether to build, run a review campaign, or park against a named signal.
Kind: data quality (organization identity, non-GB): residue that closed issues left with no owner
Relates to: 456 (the same mention-binding question, scoped to GB), 362 / 359 / 357 (the closed campaigns that left the
residue), 452 (identifier verdicts, keyed per triple), 439 (left the Bavarian VAT's decision open), 440 (left the DB
family open), 434 (a full fold re-resolves recorded mentions), `300-exemplars.md` 40 / 64 / 69 (the shared-registration
shape)

## What is wrong

Six non-GB classes of wrong or shared organization identifiers were each left open by an issue that is now closed
(357 reads CAMPAIGN COMPLETE, 359 and 362 DONE, 439 and 440 RESOLVED-VERIFIED). No open issue references them. The closest is 456, which has
the same mechanism (the identifier binds the mention and the name is never consulted), but 456 measures GB
PPON/COH/CHC only and treats the identifier as foreign, not legitimately shared. All reads below were taken
2026-10-01 12:4x–12:5x UTC against rev `9b44528`. The cited files (`canonical.rs`, `supervisor.rs`, `idgate.rs`) are
identical to that rev in the working tree.

### 1. The groups the R2 name gate holds (left by 362)

362 closed with: "The 87 the gate still holds … names alone cannot tell; a third pass would need register-history
evidence (PRH, KRS, INSEE), which is a different tool". The queue has roughly doubled since. The newest R2 dry, job 1668
(2026-09-29 22:45 UTC, stored as the `r2-merge-plan` report's previous version), counts `172 names`. Its
`denied_names_listing` holds all 172 groups (not truncated): FR 71, PL 57, DK 16, FI 12, IT 9, GB 6, CZ 1.

- **88 that 362's campaign 2 already reviewed:**
  - 56 carry a merge/medium verdict, which is never applied by design;
  - 27 were marked needs-more-evidence;
  - 4 are keeps the challenger disputed, so they were never recorded;
  - 1 is Safege / Suez Consulting (`FR:siren` 542021829). Job 794 applied its HIGH merge ("merged 3 row(s) into
    8734175"), and the key has since gathered 9 more rows, so the verdict no longer matches the live group.
- **84 never reviewed:** FR 62, DK 16, GB 6. The 6 `GB:coh` groups are counted here but left to the GB work (452's
  identifier verdicts, 456).

R2 has no schedule. `Spec::MatchOrgIdentifiersR2` is constructed only by the admin job parse (`supervisor.rs:1441`), so
this count stays a photograph until someone runs an R2 dry.

### 2. Two verdict-admitted RO groups that never merge (left by 362, campaign 1)

The merge verdict store (`GET /admin/case-reviews?table=merge`) holds two merge/high verdicts in cohort
`r2-denied-2026-09-06` with `applied_at` null:

| key | reviewed members = live members (public API) |
|---|---|
| `RO:cui` 14838148 | 13291636 Direcția Națională Anticorupție (157), 22572276 DNA - Serviciul Investiții, achiziții si administrativ (105), 22572277 (10), 22577869 (29) |
| `RO:cui` 4267117 | 2203 MUNICIPIUL BUCURESTI (629), 2204 DIRECTIA GENERALA ACHIZITII PUBLICE (221), 22537494 (7), 22537495 (76) |

The member sets still match, so R2 admits both groups on every run (job 1668: `2 verdict-merge`; these are the only
unapplied merge/high verdicts on an R2 scheme). Neither group is in the 34 groups job 1668 planned and job 1669 merged,
and the only denial after the verdict step (4a, `crates/store/src/canonical.rs:12082`) is the VAT-group wall (step 5,
`canonical.rs:12228–12296`). That wall denies a group when two members' mention raw identifiers carry disjoint keys in
one scheme, and job 1668 counts `10 vat-group-wall`. So, by elimination, the wall stops them. The report lists neither
admitted nor walled groups, so nothing records which mention carries the conflicting number.

The third group in the original record, `RO:cui` 2779625, merged later: R2 wet job 1650 (2026-09-29 10:05 UTC)
stamped it "merged 2 row(s) into 13013889", and 22547109 and 22706545 now answer 308 → 13013889. The stopper is
therefore mention evidence that can change, and nothing watches it.

### 3. A parent row carrying its branch's or subsidiary's number (left by 357)

357's "What is left" calls these "parents carrying a branch's number, which are a wrong IDENTIFIER, not a wrong
country — no executable path today". Its slice-2 exhibit still stands (`?identifier=5262938543`): the Polish NIP is
carried by Agfa Graphics NV, BE, org 8957943 (15 mentions), and by Agfa NV, BE, org 22180417 (1), beside AGFA NV, PL,
org 21011485 (12). No count of the class exists. 357 parked it under its own-legal-form floor and in move/medium
verdicts, and the rationales do not tag it: 709 move/medium verdicts stand unapplied in cohort
`cluster-country-2026-09-05`, and 24 of their rationales say "branch".

### 4. The buyer's number on the winner's row (left by 359 and 362)

When a winner's row carries the buyer's number, 359's name gate denies the group, and 362 recorded `keep`. That keeps
the two organizations apart, but the winner's row still carries the buyer's number as its identifier. The merge store
holds 192 keeps across the two `r2-denied` cohorts. 82 of their rationales name a buyer, contracting authority or
awarding authority (a keyword count over 362's records, not a classification).

Exhibit (`?identifier=1070046338`): the PL NIP stands on Sieć Badawcza Łukasiewicz – Instytut Lotnictwa (17283953, 173
mentions) and on Simplicity Sp. z o.o. (21590143, 16). The keep/high rationale says "the institute's NIP ended up on
Simplicity's row".

452's tool cannot single out the winner. `org_identifier_verdicts` is keyed `PRIMARY KEY (identifier,
identifier_kind, country)` (`canonical.rs:642`), and `withheld_identifier_orgs` (`canonical.rs:3277`) withholds every
org that carries a `wrong` triple, through `orgs_with_triple` (`canonical.rs:3137`). It does this even though each
verdict row records an `org_id`. A `wrong` verdict on PL/national/1070046338 would therefore withhold the institute,
the number's rightful owner, along with Simplicity. All 685 verdicts in the store are GB (371 wrong, 212 right,
102 related; `GET /admin/case-reviews?table=identifier`).

### 5. Deutsche Bahn's procurement file numbers (left by 440)

440 dissolved the OJ S notice-number class. It left DB's own file numbers (`YY{T,F,G}E{A,I}nnnnn`) "Not built;
recorded for whoever takes the DB family" (`440-…md:129`). On 2026-09-27, by prefix seeks, it counted 197 org rows (196 DE, 1 FR) on
196 values, 47 of them with ≥ 2 names (worst 12), and 1,059 mentions.

440 also tested a same-notice check: it matched 0 of 18 sampled DB mentions, because DB swaps NATIONALID and
REFERENCE_NUMBER. What would work is a corpus-level lookup: "a NATIONALID equal to ANY notice's reference number".
`crates/ingest/src/idgate.rs` has no rule for this class.

Live reads:
- `18GEI32660` keys org 13438560 Deutsche Bahn AG (14 mentions);
- `14TEI10487` keys org 8055796 (1);
- `17FEI28567LOS2` keys org 14068182 (3);
- of the first 200 rows under `name_prefix=DB Netz AG`, 72 carry a file number. All 72 are non-provisional and
  together hold 528 mentions; the largest is `15TEI14667` on org 9378844, with 131. The provisional `DB Netz AG` row
  1197942 holds 15,836.

The class mostly splits DB by procedure rather than fusing it.

### 6. The shared Bavarian state VAT DE811335517 (left by 439)

439 made the bind deterministic: the preload walks `ORDER BY id` and keeps the first row per triple
(`org_of.entry(…).or_insert(id)`, `canonical.rs:9846`). Its header still reads "Open decision, deliberately not taken here: what
the shared Bavarian VAT DE811335517 identifies". The lowest id is 1179, Regierung von Oberbayern, which is neither review
chamber. So every new mention that carries the VAT joins 1179, whichever authority it names.

| row | name | 2026-09-28 (439's read) | 2026-10-01 |
|---|---|---|---|
| 1179 | Regierung von Oberbayern | 3,777 | 3,824 |
| 1448 | Regierung von Oberbayern, Vergabekammer Südbayern | 18,552 | 18,552 |
| 14988 | Vergabekammer Nordbayern bei der Regierung von Mittelfranken | 2,564 | 2,564 |
| 23247725 | Regierung von Mittelfranken | 1,101 | 1,058 |
| 22646937, 22478372, 22378236, 22852760 | duplicates of the names above (22852760's is an address block) | not read | 79, 39, 2, 2 |

1448 also holds 2,204 Vergabekammer Nordbayern mentions (439, 2026-09-27), which the lowest-id rule never moves. Nothing
explains the 43 that left 23247725. The same shape appears in `300-exemplars.md` 64 (two NRW chambers on one id) and 69
(Italian TARs on one fiscal code).

## Proposed fix

There is one root under all six classes. Identity is decided by the identifier literal alone, and the verdict layer
cannot speak about one org:
- The resolver binds a triple to its lowest-id row, whatever the mention's name says (`canonical.rs:9846`). That is
  right when a triple names one entity. It is wrong for a shared registration (class 6), and for a triple a publisher
  wrote on someone else (classes 3 and 4; 456 for GB).
- Verdicts exist per group (`org_merge_verdicts`: merge or keep the whole group) or per triple
  (`org_identifier_verdicts`). Neither can say "this org's identifier is not its own" or "this triple names several
  entities".

**1. Measure (the first unit).** One bounded read per class; record each read and its date in this file:

| class | read | at filing |
|---|---|---|
| 1 held groups | `/root/aj.sh /admin/reports/r2-merge-plan` after a fresh R2 dry (the stored one is job 1668) | 172 (88 reviewed, 84 not) |
| 2 RO groups | the 8 members' `organization_mentions.raw_identifier`, one seek by `organization_id` each: a `/v1/sql` data read, gated per `prod-box-reads.md`, with its plan read locally first | not taken |
| 3 parent / branch | `GET /admin/case-reviews?table=country&cohort=cluster-country-2026-09-05`, then a hand read of the 709 move/medium rows for the shape | 709 rows, 24 say "branch" |
| 4 buyer's number | `GET /admin/case-reviews?table=merge`, then classify the 192 `r2-denied` keeps | 82 by keyword |
| 5 DB file numbers | 440's identity-index prefix seeks (`05`–`26` × `{T,F,G}E{A,I}`): a `/v1/sql` data read, gated | 197 on 2026-09-27 (440) |
| 6 Bavarian VAT | the Verify's public read | 8 rows, table above |

**2. Per-org identifier verdicts (classes 3, 4, and maybe 2).** Let a `wrong` verdict name one org; the `org_id` column
already exists. Such a verdict withholds only that org. The triple-wide form stays for 452's GB cohort. Then re-record
the class-4 keeps and the class-3 parent rows as per-org `wrong`. Withholding removes the org from R2's groups, so
any group verdict on those keys goes stale and needs a fresh read. Pin with
`a_wrong_verdict_scoped_to_one_org_withholds_only_that_org` (`crates/store/tests/identifier_verdicts.rs`), which
withholds Simplicity's shape and leaves the institute's shape bound.

**3. A `shared` triple and a name-aware bind (class 6; this is 439's deferred decision).** For a triple recorded as
`shared`, the resolver binds a mention to the row of that triple whose names agree with the mention's name, and mints
a row when none agrees. Same-name rows inside the triple fold to one per authority. Under the shared triple, R2 and E0
group by name and never by the triple alone. This is 456's design question 1 for a registration that is legitimately
shared, so build it with 456's bind rule or after it, never as a second bind beside it. Pin with
`a_shared_triple_binds_each_mention_to_the_row_its_name_agrees_with` (`crates/store/tests/mention_refresh.rs`, where
439's lowest-id test lives). A full fold re-resolves recorded mentions (434), so 1448's 2,204 Nordbayern mentions
re-home through the same rule.

**4. DB file numbers (class 5): decide.** One option is to build 440's corpus-level check: an index over the reference
numbers that notices publish about themselves, under which a NATIONALID found there keys no org. The rows would then
dissolve through `repair-placeholder-orgs`, the path 440 used. The other is to decline and record why: the class
splits DB more than it fuses, and 440 rated the value low. If built, pin with
`a_nationalid_another_notice_publishes_as_its_reference_keys_no_org`.

**5. RO groups (class 2).** Step 1's read names the conflicting key. If it is a member's own second registration (a
department with its own CUI), that member is a distinct entity: re-review the group without it. If it is a publisher's
stray number, then the wall is wrong for this group. The question is then whether a HIGH verdict may outrank the wall,
which 362 deliberately did not allow. Decide that once, in this file, not per group.

**6. Held groups (class 1).** Review the 84 unreviewed groups with 362's checked-in tooling
(`.scratch/tender-db/362-campaign/`: `enrich.py`, `split.py`, `review.js`, `rubric.md`, `post.py`), then run R2 dry and
wet. For the 88 already reviewed, names cannot decide (362). Build register evidence, on the pattern of 448's
`.scratch/tender-db/448-campaign/ch_fetch.py`, only for a country whose count pays for it. Park the rest against a
computed signal: add the R2 dry to the weekly org batch (the `[schedule]` arm, `supervisor.rs:11916–11991`, that runs
`build-org-match-keys`, `org-merge-health` and `scan-org-match-keys`) so that `denied_names` is re-taken without anyone
asking.

## Verify

    curl -s "https://tenders.zebreus.click/v1/organizations?identifier={14838148,4267117,18GEI32660,DE811335517}" | jq -c '[.items[] | [.id, .mentions]]'

This covers classes 2, 5 and 6. Classes 1, 3 and 4 are read through `/admin`, which a free command cannot reach;
their numbers are in the body and in step 1.

- **open** (2026-10-01 12:5x UTC; re-read identical 2026-10-01 by the review): four lines of 4, 4, 1 and 8 rows.
  - RO 14838148, four rows: `[[13291636,157],[22572276,105],[22572277,10],[22577869,29]]`
  - RO 4267117, four rows: `[[2203,629],[2204,221],[22537494,7],[22537495,76]]`
  - a DB file number keys Deutsche Bahn AG: `[[13438560,14]]`
  - the eight DE811335517 rows:
    `[[1179,3824],[1448,18552],[14988,2564],[22378236,2],[22478372,39],[22646937,79],[22852760,2],[23247725,1058]]`
- **done**: line 1 one row, line 2 one row, line 3 `[]`, line 4 one row per recognised authority:
  - lines 1–2 read one row each: the reviewed merges executed. If step 5 finds the wall right and records `keep`, it
    rewrites these two expectations here.
  - line 3 reads `[]`: no org is keyed by a DB file number. If step 4 declines, it rewrites this line.
  - line 4 reads one row per authority the recorded Bavarian decision recognises (four under step 3: Oberbayern,
    Südbayern, Nordbayern, Mittelfranken), with 1448 about 2,204 mentions lighter.
