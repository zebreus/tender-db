# 552 — a head whose only defect is a line break is swapped for another spelling instead of being served collapsed

Status: ready-for-agent — unit 3 (rule v12) DEPLOYED 2026-10-17 (`3475272`); the v12 re-judge FAILED its bar (2/54), so the filters stay on every rule-driven wet run; unit 4's census ship is DONE: jobs 2570 (R0w 4), 2571 (49), 2572 (22), 0 drifted, dated spot check 55 right / 1 neutral / 0 wrong, Verify reads done (22289866, 22661109, 23474998 serve the collapse; 22737299 its own head). NEXT: unit 5 — the two wrongs' shapes (4040: a release electing a former name while a fresher rival exists; 22678568: a brand short form over a complete head) before 22678568's head_current hold lapses ~2027-07-10; docs/operations.md re-base after 551 lands.
check wf_0744b445-0ee (5 blocking findings, all taken; below). Next: unit 3, built as a 524 epoch after rule v11 has
rolled out (v11's wet runs keep `exclude_released: ["unfit"]` until this ships).
Kind: data quality (served organization names; issue 524's head election)
Relates to:
- 524 (rule v10's wet runs exclude the `unfit` junk releases because of this class: `exclude_released: ["unfit"]`);
- 548 (the head refresh's over-blocking classes);
- 520 / 526 (name cleanup precedents).

## Verify

    for o in 22289866 22661109 23474998 22737299; do curl -s https://tenders.zebreus.click/v1/organizations/$o | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['id'], repr(d['name']))"; done

- **done**: 22289866, 22661109 and 23474998 each serve their head's own words with the line break collapsed to one
  space (rule R0w), e.g. 22661109 `Szkoła Podstawowa nr 13 z Oddziałami Dwujęzycznymi im. Jana III Sobieskiego w
  Tychach`, not a shorter sibling spelling and not the head with a raw `\n`. 22737299 still serves its own head,
  NOT the truncated `BPB Büro für Planung und Bauüberwac - im Verkehrswesen GmbH`. Its address-tailed head is
  withheld `lines` with no R0w (unit 2, point 3).
- **open (2026-10-16)**: each serves its head with a literal line break. 524 v10 and v11 exclude their junk release,
  so the planned changes, which would serve worse spellings, do not ship.

## What was found

Plan 2543 released 102 `unfit` heads from the recency holds. 81 of them carry control whitespace (a `\n`, `\r` or
`\t` inside the name); 79 are unfit through it alone, and 2 are also `long`. In every one, the change serves the elected cluster's spelling, never the
head collapsed (0 of 81). The re-judge read 101 unfit releases: 91 right, 5 neutral, 5 wrong. All five wrongs are
in the control-whitespace group:
- 22289866: `Vergabekammer Rheinland-Pfalz\nbei dem Ministerium …` → `Vergabekammer Ministerium für …`, losing the
  Land.
- 22545991: a ministry service `… (ΥΠΑΑΤ)\nΕιδική Υπηρεσία Διαχείρισης ΣΣ ΚΑΠ` → its bare acronym `ΕΥΔ ΣΣ ΚΑΠ`.
- 22661109: a school's full name → the name without its patron and town.
- 22718921: a company's name → an over-long legal-title spelling with quotes and a `διακριτικό τίτλο` tail.
- 22737299: an address-tailed head → a truncated `BPB Büro für Planung und Bauüberwac - im Verkehrswesen GmbH`.

Of the other 76 control-whitespace releases, 75 read right or neutral and 1 (22667173) was not judged. Their elected spelling was as good as the head or
better, so the collapsed head would usually tie.

## Proposed units

1. **Measure (S).** On plan 2543's planned and held lists, for every control-whitespace head:
   - compare the collapsed head against the elected spelling;
   - count where they are equal up to case, punctuation and legal form;
   - count where the collapsed head is longer, with the elected spelling a prefix or subset of its words (the loss
     shape);
   - count where the head carries an address or extra block after the break (`… GmbH\nHohenzollernstr. 18\nD-56068
     KOBLENZ`).
2. **Decide (S).** Options:
   - (a) collapse a control-whitespace head in place (R0-style, always) and only then elect;
   - (b) keep the election, but serve the collapsed head when the elected spelling drops head words;
   - (c) split the head at the break and keep the first line when the rest is an address block.

   Each needs a rule for the address-tail case.
3. **Build** as a 524 epoch. Re-plan, then re-judge the control-whitespace releases (all 81) before lifting
   `exclude_released: ["unfit"]`.

## Unit 1 — measured (2026-10-16; corrected the same day after the adversarial check wf_0744b445-0ee)

Evidence: `.scratch/tender-db/552-control-whitespace/`:
- `cw_rows.json`: all 104 rows;
- `cw_listing-plan2543.txt`;
- `rejudge2543_result.json`: issue 524's re-judge of plan 2543;
- `scripts/`.

From the pack directory, `measure_552.py` reads `../plan2543.json.gz` (the session scratchpad's copy of the plan), and
`guard_552.py`, `wrap_552.py` and `join_552.py`, run from the pack directory, read `cw_rows.json` and
`rejudge2543_result.json`. Those three are
the FIRST pass, which tokenised the display words. The decided figures below come from `decide_552_v2.py`. It keys
with the verbatim crosswalk `head_vote_key`. The reviewers lifted it into `scripts/votekey/vk.rs` (build it with
`rustc --edition 2024 -O -o vk vk.rs`) and wrapped it in `rk.py`; `vk_py.py` is a Python port that agreed with the
binary on all 571 strings. Run `python3 -I scripts/decide_552_v2.py scripts/votekey cw_rows.json
rejudge2543_result.json`.

Plan 2543 carries **104** heads with a `\n`, `\r` or `\t`:
- **90 planned**: 85 R1 and 5 R2. 81 of them are `unfit` releases. The other 9 change without a release: the 5
  R2, and 4 R1 that the guard below would pass anyway.
- **14 held**: 8 `consortium_removed`, 3 `no_fit`, and one each `review_body`, `initialism` and `address_added`.

Of the 81 `unfit` releases, **79 are unfit only through control whitespace**. Two are also `long` once collapsed:
22354266 at 315 characters and 22650988 at 288. No head carries a control character other than `\n`, `\r` or `\t`.

What follows the break, read by hand over all 104 rows:

| shape after the break | examples | collapsed head is right? |
| --- | --- | --- |
| **wrap**: one name broken mid-phrase | 22289866 `…Rheinland-Pfalz\nbei dem Ministerium…`, 22661109's school, 22393047 `SPÓŁKA Z OGRANICZONĄ\nODPOWIEDZIALNOŚCIĄ` | yes |
| **address / registry block** | 14770 `Landkreis Saalekreis\nDomplatz 9\n06217 Merseburg`, 23066716 `Adres: … NIP … REGON …` | no |
| **consortium / member list / on behalf of** | 15395 `Konsorcjum firm:\n1) …`, `RTI …`, 22216826 `c/o`, 22632467 `über die Stadt Laage` | no |
| **unit, branch or role line** | 21955455 `PROVINCIA DI FOGGIA\nSETTORE SUA…`, 22685142 `…KG\nNiederlassung…`, 22505944 `\nPouvoir Adjudicateur` | usually no |
| **trade or person line** | 22667173 `BAUHAUF GmbH\nHoch- und Tiefbau`, 22299001 `Stadt Remagen\nBürgermeister Björn Ingendahl` | no |
| **list number + tab, label + tab** | 19635627 `1.\tGmina i Miasto Krajenka`, 30914521 `Name of Supplier\tPublic Sector Media Ltd` | no |

The flags in `measure_552.py` are rough. Its classes compare order-sensitive token lists with legal forms dropped,
so as word sets the comparison is 72 superset / 3 equal / 6 subset / 23 different. 32 elect their first line up to
case, punctuation and legal form; 24 byte for byte. Its member-list flag fires on stray single-letter "legal
forms". By hand there are about 12 real member lists and at least 23 address tails.

**The issue's three options:**
- **(a) Collapse every control-whitespace head in place.** Over all 104, about 60 would serve an address, a member
  list, a person or a `1.\t` prefix. **Rejected.**
- **(b) Serve the collapsed head when the elected spelling drops head words.** Read as "the elected core word set
  is a strict subset of the head's", it fires on 72 of 104, nearly all of them right drops: an address, a partner,
  a unit. Read literally, it fires on 95. **Rejected.**
- **(c) Keep the first line when the rest is an address.** It never reaches the wraps, and it cuts 22737299 to
  `BPB - Büro für Planung und`. **Rejected.**

**(d) A vote-key line guard.** The elected key's tokens must sit inside ONE head line's vote-key tokens, or equal,
as a set, the union of a contiguous run of whole lines. On the 79 control-only `unfit` releases, joined with the
re-judge:

| on the 79 control-only releases | right | neutral | wrong | unjudged |
| --- | --- | --- | --- | --- |
| guard passes (54) | 52 | 1 | **0** | 1 (22667173 → `BAUHAUF GmbH`, its first line) |
| guard withholds (25) | 17 | 3 | **5** | 0 |

The 2 `long` releases are not asked and release as today; both were judged right. The first pass on display words
gave 55 / 26 over 81. Five rows differ between the two tokenisers:
- vote keys pass 22354266, 22393047 and 22648679;
- vote keys withhold 22571984 (`Α.Ε.` keys as `αε`) and 23294406 (`Alexander's` → `alexander s`). Both were judged
  right.

Under either tokeniser, all five judged wrongs are withheld.

Many of the 17 rights the guard withholds are newer names: 22419170 `… w spadku`, 22981989 and 22862316. Others are
identity corrections, where the head was another entity's print: 22962566 and 22199243. They stay as they are
today, held with the raw break.

**Fallback for the withheld heads.** "Serve the collapsed head, address cut" is a net loss, about 7–8 better
against 11 or more worse. It would serve:
- a person or an agent tail: 12300, 22299001, 22962566;
- a label or letterhead: 22648679, 22354266;
- a member list: 22302655, 22923998;
- a role line: 22505944;
- older names: 22398613, 22419170.

A cut also re-keys the head (22737299's head key includes the address), so the written head would leave its own
cluster and lose every recency hold. **(e) A wrap collapse with no cut, scoped to the withheld** fixes this:
- the written name is the head with whitespace collapsed, so the vote key does not move and the holds keep
  working;
- it fires only when every break is a wrap.

On the 25 withheld it fires on **4**:
- 22289866 and 22661109, judged-wrong elections, now serve the full name on one line;
- 23474998: neutral; the judge wrote that the collapsed head "would serve better than either";
- 22981989: its elected full name was judged right, and the collapsed head keeps the abbreviated patron `Wł. St.`.
  That is better than today's raw break but worse than the withheld election.

It does not fire on 22737299 (its first break follows `GmbH`), so that head stays withheld with its raw break and
address. The same predicate run unscoped fires on 14 of 104, about 8 of them wrongly (`c/o`, `mit`, `über`,
`vertreten durch`, `cotraitant` lists). It must never run outside the guard's withheld set.

## Unit 2 — decided (2026-10-16, revised after wf_0744b445-0ee)

Adopt (d) and (e) as one 524 epoch: rule v12, or folded into whatever epoch follows v11.

1. **Control-only test.** Let `c` be the head with every whitespace run collapsed to one space. The head is
   control-only when all three hold:
   - `(fns.unfit)(head, head) == Some("control")`;
   - `c` carries no control character and no U+FFFD;
   - `(fns.unfit)(&c, &c)` is `None`, or is `Some("lowercase")` while `(fns.not_a_name)(&c)` is `None`.

   Every other unfit head, including control plus `long` or control plus mojibake, releases `unfit` as today. A
   control-plus-lowercase head goes to the guard: it no longer bypasses v8's lowercase protection unasked.
2. **`lines` release guard.** This lives in `recent_release`, the junk exemption.
   - Split the head into lines on exactly the characters that are both control and whitespace (`\t \n \x0b \x0c
     \r \x85`), trimmed, with empty lines dropped. Key each line with `fns.vote_key`.
   - A control-only head releases `unfit` only when the elected key's token set is a subset of one line's token
     set, or equals the union of the token sets of a contiguous run of whole lines.
   - Otherwise the release is `Junk::Withheld("lines")`: the hold stands.
3. **R0w wrap collapse.** A head withheld `lines` changes to `c` (rule `R0w`) when all of these hold:
   - it has at least one break, and every break is a wrap;
   - `c` passes the same fitness test as point 1;
   - no bind guard (`fns.hold`) names a reason, as for R0n. The recency, residue, `head_current`, `top_stale` and
     serve holds are not asked;
   - (added at unit 3's review) `c` would itself stay held: `recent_release(c, head key, elected key)` does not
     release it. Otherwise a bare number the elected key lacks (`Szkoła Podstawowa nr 13\nw Tychach` against
     `Szkoła Podstawowa w Tychach`) releases the written head `digits` on the next plan, and it changes R1 to the
     spelling the guard withheld. Then the `lines` hold stands. The 4 real R0w rows all stay held.

   A break between line A and line B is a wrap when A does not end in a legal form (`HeadElectFns::family` reads a
   family on A's tail, multi-token forms such as `Sp. z o.o.` and `S.C.p.A.` included), A does not end in `,`, `;`
   or `:`, and one of these holds:
   - A's last word ends in a hyphen with no space before it (`Planungs-`; a spaced ` -` is no wrap);
   - A's last word has no `.` and is a connector: `und and de del della dei degli di du des der den of for für fur
     i z w ze we e ed et y και του της των en van voor u na do od a o`;
   - B starts with a lower-case letter.

   `c`'s vote key equals the head's (`head_vote_key` collapses whitespace), so the written head stays in its own
   cluster and its recency holds keep standing on later plans.
4. **Wet runs keep both classes out until judged.**
   - R0w joins `ORG_NAME_REFRESH_EXCLUDABLE_RULES`, and every wet run passes `exclude_rules: ["R2", "R0w"]`.
   - The guard-passed releases are still `released: unfit`, so `exclude_released: ["unfit"]` keeps them out too.
   - Both filters lift only after unit 3's re-judge passes.
5. **Scope.** The guard sits in the release path only. Of the 9 planned control-whitespace changes made without a
   release, the 4 R1 all pass the guard, and the 5 R2 are dropped by v9's `exclude_rules: ["R2"]`. Both are
   accepted as they are.
6. **Known limit, accepted.** An R0w head is fit, so once its `head_current` lapses (3 years after its last print)
   v10's policy may again change it to the majority spelling. On plan 2543's dates, 22661109 would revert to
   `Szkoła Podstawowa nr 13 z Oddziałami Dwujęzycznymi` around 2027-08. That belongs to issue 548's over-blocking
   and supersede work, not to this rule.

Not adopted:
- Greek `και με το διακριτικό τίτλο` tails (22718921's elected spelling) as a serve hold. The guard already
  withholds that row, and a corpus-wide boilerplate hold needs its own measurement.
- An address-cutting R0w. Its re-keyed head loses its holds, as point 3 explains.

## Unit 3 — build notes

**Pins** (real heads from plan 2543):
- 22289866 and 22661109: withheld `lines`, then R0w to `c`;
- 22981989 and 23474998: R0w;
- 22737299: withheld `lines`, no R0w (`GmbH` ends line 2);
- 22862316: withheld `lines`, no R0w (`GmbH` ends line 1);
- 22299001: withheld `lines`, no R0w (person line);
- 22545991 and 22718921: withheld `lines`;
- 14770, 22216826, 22925490, 4040, 22393047 and 22648679: the guard passes;
- 22354266 and 22650988: `long`, released `unfit`, the guard not asked.

Add a synthetic pin for the legal-form clause: `X S.C.p.A.\nper il tramite …`, under 200 characters, is no wrap.
Added at the review: `Szkoła Podstawowa nr 13\nw Tychach` against `Szkoła Podstawowa w Tychach` wraps but is no R0w
(its collapse would be released `digits`), so it stays withheld `lines`.

**Durability test.** Re-elect each R0w-written head on the same votes with a fresh elected date. It must come out a
Hold, not a Change.

**Counts.** The planner counts `release_withheld` only for verdicts that stay a Hold, so on the v10 population
expect about 21 `release_withheld.lines` and about 4 `by_rule.R0w` (25 withheld in all). Allow ±10% for drift
since 2026-10-16. The control-only `unfit` releases should be about 79, of which about 54 pass.

**Go.** Re-judge every guard-passed control-only release and every R0w change:
- an R0w change is judged against the raw head it replaces, because the elected spelling is withheld either way;
- go needs 0 R0w changes worse than the raw head, and at most 1 wrong among the passed releases.

Then the wet runs drop `"R0w"` from `exclude_rules` and drop `exclude_released: ["unfit"]`.

## Unit 4 — the v12 re-judge FAILED; census ship decided (2026-10-17, before the second reading's results)

**The re-judge** (plan 2569, rev `3475272`, epoch `524u2-v12`, as_of 2026-10-16; workflow wf_a5e2383f-61d, dated,
a skeptic on every wrong; evidence in `.scratch/tender-db/524-v12/rejudge-2569/`). Plan 2569 passed step 2's v12 GO
bullet exactly: `undated` 0, `truncated` false, 59/60 canaries as expected (30914328 reported only), every hold count
unchanged but `head_current` +21 (the `lines` withholdings), `release_withheld.lines` 21, `by_rule.R0w` 4,
`recent_released.unfit` 77, `by_rule.R1` 77 (all 77 R1 changes are `unfit` releases; with v12's filters the wet
scope is EMPTY). All 81 judgeable changes were read:
- the 54 guard-passed control-only releases: 51 right, 1 neutral (23038177), **2 wrong** —
  - 4040, a REVERT: `WIMED Sp. z o.o., Sp. k.` is the limited partnership's name; the same NIP now prints
    `WIMED 4.0 Sp. z o.o.` and then `WIMED Sp. z o.o. (dawniej WIMED 4.0 …)` (206463 2026-10-01, 266036
    2026-10-16). The head is a consortium label; the better name is the rival cluster.
  - 22678568, borderline: `MZP-Ingenieure` is the firm's brand short form for a complete head whose only defect is
    the line break (precedent 4609556 `McKinsey & Company` → `McKinsey`).
- the 4 R0w changes: 4 better than the raw head, 0 worse;
- the 23 other `unfit` releases (not gated by the bar, but they ship once `exclude_released` lifts): 23 right.

**The bar fails** (2 wrong > 1). Per step 3, both filters STAY on every wet run of a plan whose class has not been
judged item by item: `"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]`, including unit 2c's weekly run
when it is built. The two wrongs' shapes are this issue's to study (unit 5): a release whose elected spelling is a
former name while a fresher rival exists, and a brand short form electing over a complete head.

**The amendment (decided, recorded before the second reading's results).** A pre-committed bar bounds the wrongs a
rule ships UNSEEN. Plan 2569's class was read item by item (a census, not a sample), so its judged-right items may ship
by explicit `org_ids` on the SAME stored plan — a strict subset of what a pass would have shipped, with the filters
kept for every later plan. A one-off for plan 2569, not a codified step. The decision review (wf_f02eaa69-6f2) found it
sound on conditions, all adopted:
1. **A blind second reading of every shippable id** (dated, the reader not told the first verdict; a skeptic on every
   wrong). The 16 already read blind count (15 right, 1 neutral). Acceptance, fixed now: an id ships only if EVERY
   reading calls it right (R0w: better than the raw head) — today's, the blind second, and plan 2543's for a
   release it judged (R0w ids excepted: 2543 judged the elected spelling, not the collapse). Any neutral or wrong in
   any reading drops the id. Dropped already: 4040 and 22678568 (wrong), 23038177 (neutral today), 22307905
   (neutral blind), 7899 (neutral in 2543). **Stop:** 2 or more confirmed wrongs in the blind reading among the
   items today's judge called right → no census ship at all; the class goes to unit 5.
2. **The id list is built mechanically** from the result files and asserted before enqueue: every id is in plan
   2569's `planned` with verdict `change` and rule `R1` or `R0w`, none `R2`, 4040 and 22678568 absent, the R0w ids
   ⊆ the plan's `by_rule.R0w`. The list and its sha256 are kept under `.scratch/tender-db/524-v12/`.
3. **Same-plan guard** before each enqueue: prod rev `3475272`, the stored plan is 2569's (planned 1,607), nothing
   queued or running, no deploy (551 unit 3 deploys AFTER), and the run lands before the ~07:00 UTC ingest. On an
   abort (more than 5 drifted or gone) or a missed window: re-plan and re-check each id's (old_name, new_name)
   byte-for-byte against the judged pair; never re-apply the list blindly.
4. **One wet job per class**, each `{"kind":"refresh-org-names","dry_run":false,"org_ids":[…],"exclude_rules":["R2"]}`
   so each reverts alone: R0w, then the control-only releases, then the other `unfit` releases.
5. **After each job**: the report's job is that job, `excluded_by_rule` 0, `excluded_by_release` 0, `unplanned` 0,
   `window_error` null, ready + already + drifted + gone = the ids asked; GET every applied id (served name = the
   plan's `new_name`); then a dated read of its applied samples, bar 0 reverts and 0 wrongs (every item was judged):
   a fail reverts that class's job.
6. **Same day**: docs/operations.md records step 3 v12 FAILED (filters stay), the census ship with its job ids, and
   re-bases step 2's v12 expectations for the next plan (R0w ~0, `recent_released.unfit` ~2 plus new ones, holds
   up by ~4: the collapsed R0w heads are held and serve the collapse).
7. **Lapse dates** (unit 5 must land a rule, a held-id mechanism or a release-aware canary before the first): the two
   wrongs stay held only while their holds last — 22678568's head was printed only on 2024-07-10, so its
   `head_current` hold lapses about 2027-07-10 and `MZP-Ingenieure` becomes a plain R1 change no v12 filter drops;
   4040 about 2029-07.

**Unit 4 — done (2026-10-17).** The blind second reading (wf_f02eaa69-6f2 16 ids, wf_245f67ad-624 61 ids) read 75
right, 2 neutral (22307905, 22496731), 0 wrong: the stop did not fire. 75 ids shipped as jobs 2570 (R0w 4), 2571
(control-only 49) and 2572 (other unfit 22), each on plan 2569 (`plan_computed_at` 1792191802) with 0 drifted, gone,
raced or excluded, `window_error` null; every served name equals the plan's `new_name`; the dated read of the applied
samples (wf_6a7d39fc-24f) found 55 right, 1 neutral (22693690), 0 wrong, 0 reverts: no class job reverted. Evidence:
`.scratch/tender-db/524-v12/rejudge-2569/` (`blind2_result.json`, `spot_2570_2572_result.json`).

