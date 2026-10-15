# 524 — an org's served name is its first-seen spelling forever, so merges and drops leave a wrong or junk head

Status: ready-for-agent — UNIT 1 DONE (census job 2492, 2026-10-15). UNIT 2 DECIDED 2026-10-15 by a design panel:
three angles (election rule; keys and downstream; rollout), and a judge who simulated the rule on the census samples
(see "Unit 2 — decided"). Next: unit 2a, the pure `elect_head` and a DRY `refresh-org-names` planner, then one dry
run, then 2b (wet + ledger) and 2c (weekly upkeep).
Was: needs-triage — filed 2026-10-12 from the hourly audit after 514's rollout and 511's drain.
Kind: data quality (served organization names)
Relates to:
- 514 (A6);
- 511 (unit 6 item 7);
- 522 (the s.94 PPON org);
- 513 (nhs-body survivors keep their head name);
- 321 (rehoming leaves the name satellite behind);
- 234 (`name_norm` is the resolver's key).

## Verify

    ssh root@zebreus.click "curl -s http://127.0.0.1:8080/v1/organizations/31558903" | head -c 200

- **done**: the name reads 'The University of Sheffield' (or its register name);
- **open**: 'Sheffield Hallam University', as served on 2026-10-12.

## What was found

`organizations.name` is written at mint and never again. No `UPDATE organizations SET name` exists; only `name_norm`
is ever rewritten (canonical.rs ~32951, ~33481, lib.rs ~4063). A merge keeps the survivor's own head, and a dropped
mention leaves the head it named. Served examples on 2026-10-12:
- **31558903**, UKPRN 10007157, which is the University of Sheffield. It is served as **'Sheffield Hallam
  University'**, from one stray mention. 514's review measured 15 of its 16 mentions as Sheffield (A6; the survivor
  of G24 by lower id).
- **22713056** is served as **'s.o.'** ('siehe oben', a placeholder). 511's drain retired the placeholder mentions,
  and its 28 remaining ones read 'Vergabekammer bei der Bezirksregierung'. They bind by the chamber's phone number
  0251411691.
- **30914328** is served as 'Withheld Section94 Supplier' while holding HISCOX and DOC-Labor mentions (522).

## Why it matters

The API, search and every statistic show the head name. Merges (R2/E0/altid/nhs-body/514) and drops (510/511) are
now routine, so the head drifts away from what the org's mentions say.

## Proposed units

1. **Measure.** Orgs whose head name no longer appears among their mention names, or is a minority of them weighted
   by count, or is a placeholder under `partyname::not_a_name`. Run it as an in-app dry census, never a corpus scan
   from the agent.
2. **Decide.** The display name rule:
   - the weighted-majority mention name;
   - the register name for a register identity (UKRLP/CC/FCA/ODS), when one is cached;
   - otherwise keep, with stickiness so a single new mention cannot flip it.
   Decide also whether `name_norm` (the 234 key) follows. Probably not, so binding keys are left alone and only the
   display name is refreshed. ADR needed.
3. **Build** a dry-first `refresh-org-names` job with a ledger (old/new), then the D5 correction rows.

## Triage (2026-10-12, owner; panel wf_73cc2c40-e1c)

Triage 2026-10-12: claims verified.

- **Code.** `organizations.name` is written only at mint, from the first mention's name (canonical.rs ~20171 and the other mint sites). No non-test code ever updates it. `repoint_org_references` (canonical.rs ~4616) moves mentions, parties, winners and `organization_names` but never the head, and the loser's head is not kept even as a satellite. The `name_norm` writes have moved to canonical.rs 34430/34960 (lib.rs 4065 is unchanged).
- **Prod.** 31558903 is served as "Sheffield Hallam University" over 17 mentions: UNIVERSITY OF SHEFFIELD 14, University of Sheffield 1, The University of Sheffield 1, Sheffield Hallam University 1. 22713056 is served as "s.o." over 28 of 28 "Vergabekammer bei der Bezirksregierung". 30914328 now has 4 mentions (Individual 2, HISCOX, DOC-Labor), and 522's verdict is its fix, not this issue.
- **Overlap.** 456's design question 2 ("elect the head by majority, not first-seen") is this issue's unit 2. 524 now owns head election, 456 keeps the bind, and one census serves both. Update 456 to point here.
- **Inputs for the decision:**
  1. A plain count-majority picks the all-caps "UNIVERSITY OF SHEFFIELD". Elect by key (`name_norm` or the altid key) weighted by count, then pick the display spelling within that key: ENG-or-original per ADR-0013 D4, mixed case preferred, the register name when cached.
  2. `name_norm` is a bind key only where `identifier IS NULL` (every resolver lookup carries that predicate). For identifier-bearing orgs it can follow the head, which keeps name search consistent. For identifier-NULL orgs, refresh only within the same key.
  3. `org_match_keys` is built by org-id watermark, so a head refresh needs a targeted re-key.
  4. Run the refresh after the 511 drains, since its NonNames net walks head names.
  5. Announce each change through append_change plus D5 correction rows.
- **Verify.** Switch to the free public `curl -s https://tenders.zebreus.click/v1/organizations/31558903 | jq -r .name`. done = any University of Sheffield spelling; open = "Sheffield Hallam University".

NEXT: unit 1, the in-app dry `org-head-drift` census (size M).

**Claims:** Yes, all three hold. Two details need correcting.

Code:
- `organizations.name` is set only at mint, from the first mention's name: `t(&m.name)` in the INSERT at canonical.rs ~20171-20179, and the same at every other mint site (canonical.rs 18598/18643/20305/20368/20401/20416/29114/37549, lib.rs 6509-6952).
- Outside tests, no code ever runs `UPDATE organizations SET name`. The only matches are test fixtures: supervisor.rs 17130/17289, tests/api.rs 2072, tests/satellite_drops.rs 289.
- `name_norm` is rewritten only at canonical.rs 34430 (`rekey_keep`) and 34960, and at lib.rs 4065 (backfill). The issue cites ~32951 and ~33481; those numbers have drifted. lib.rs ~4063 is right.
- A merge leaves the survivor's head untouched. `repoint_org_references` (canonical.rs ~4616-4683) repoints mentions, parties, bid parties and winners, and moves `organization_names` with INSERT OR IGNORE. It never touches `organizations.name`. The loser's head is not kept as a satellite either: after a merge the old head survives only through the mentions.
- The API serves `o.name` raw (read.rs 4218, `ORG_COLUMNS`). Name search is a prefix on `o.name_norm` (read.rs 4502).

Prod, 2026-10-12 (3 public GETs and 1 bounded SELECT, LIMIT 40):
- **31558903** (GBUKPRN10007157; it carries GBUKPRNUKPRN10007157 as a merged identifier from 514) is served as "Sheffield Hallam University". It now has 17 mentions, not 16: "UNIVERSITY OF SHEFFIELD" 14, "University of Sheffield" 1, "The University of Sheffield" 1, "Sheffield Hallam University" 1.
- **22713056** (DE, phone-number identifier 0251411691) is served as "s.o.". It has 28 mentions, all "Vergabekammer bei der Bezirksregierung". "s.o." is a known pointer in partyname.rs (line 266, test at 3336).
- **30914328** is served as "Withheld Section94 Supplier". It now has 4 mentions, not 522's 9: "Individual" 2, "HISCOX INSURANCE COMPANY LIMITED" 1, "DOC-Labor GmbH" 1. The s.94 sentences were dropped by 511. So the head matches none of its mentions, but the real defect there is 522's bind.

**Evidence:** - **Mint and serve:** canonical.rs ~20171 (INSERT … name = m.name); read.rs 4218 (`ORG_COLUMNS` serves `o.name`).
- **Merge:** canonical.rs ~4616-4683 (`repoint_org_references`; no name update).
- **`name_norm` as a bind key:** it binds only rows with no identifier. Every resolver lookup carries `identifier IS NULL` (canonical.rs 2949, 6458, 18630, 20288, 29054, 37527). For identifier-bearing orgs it is only the name-search key (read.rs 4502) and the `org_name_verdicts` key, which already has a re-key precedent (canonical.rs 34894).
- **ADR-0013 D4** defines `name` as "the ENG-or-original denormalized head", so changing the head rule amends D4.
- **GETs:**
  - `/v1/organizations/31558903` returns name "Sheffield Hallam University", mentions 17.
  - `/v1/organizations/22713056` returns "s.o.", 28.
  - `/v1/organizations/30914328` returns "Withheld Section94 Supplier", 4.
- **SELECT:** `organization_id, name, count(*)` grouped over those 3 orgs gave the breakdowns above.
- **Overlap:**
  - 456 design question 2 is the same decision: "Head. Elect the head by majority across mentions, not first-seen".
  - 456's unmeasured unit-1 census already plans a bucket for "orgs where the foreign mention is the HEAD".
  - 511 item 7 and 514's A6 point here.
- **Verify block:** it uses ssh plus curl on 127.0.0.1. The public `curl -s https://tenders.zebreus.click/v1/organizations/31558903 | jq -r .name` is free and gives the same answer.

**First unit (M):** Build a dry census job `org-head-drift` (supervisor arm, store planner, JSON report; batched org-id walk; no writes). Fold in 456 unit 1's "foreign mention is the HEAD" bucket so one walk serves both issues. For each org with at least 2 mentions it groups the mention names by `org_name_norm`, and also by `crosswalk::altid_name_key`. It counts:
- (a) the head key appears in no mention;
- (b) the head key is under 50% of mentions by count;
- (c) the head is `partyname::not_a_name`;
- (d) the drift is casing or spelling only within the head's key (cosmetic).

Each count is split by `identifier IS NULL` vs not (that decides whether `name_norm` can follow the head), by provisional, and by whether the org is a keep in `org_merge_log` (read through an index, never a scan).

Output: totals, the mentions affected, 30 samples per bucket, and the three exhibits as canaries.

Then gate, deploy and do one dry run. The decision after it needs:
- the casing rule: the majority spelling for 31558903 is all-caps "UNIVERSITY OF SHEFFIELD", so elect by key, then choose the display spelling within the winning key (ENG-or-original per ADR-0013 D4, mixed case preferred, register name when cached);
- whether `name_norm` follows (yes for identifier-bearing orgs; for identifier-NULL orgs, only within the same key);
- the `org_match_keys` re-key (it is watermark-built, so changed heads below the watermark go stale);
- sequencing after the 511 drains, since its NonNames net walks head names.

**Value:** Every surface that shows an org name shows the head:
- `/v1/organizations/{id}`;
- `parties[].organization_name` on tenders (the mention's own name is served beside it as `mention_name` since 456 design 3);
- name-prefix search (by `name_norm`);
- every per-org statistic.

Confirmed wrong today:
- a UK university is served under another university's name (31558903; 16 of its 17 mentions say Sheffield);
- a German review chamber is served as the pointer "s.o." (22713056; 28 mentions);
- 30914328 is in the class too, but 522's identifier verdict is its fix.

The class size is unknown until unit 1 runs; a corpus count from the agent would break the read rules. Where it comes from:
- merge survivors from this month's rule merges: 513 nhs-body removed 273 rows into about 171 survivors, 514 merged 26, 469 U1 merged 495 pairs, plus 448 altid and E0;
- orgs that lost their head-named mentions to the 510/511 drops: 511 alone retired about 95k mentions across units 3 to 6.

Provisional (identifier-NULL) orgs bound by `name_norm` share one key by construction, so the drift is mostly in identifier-bearing orgs and merge survivors. Expect thousands rather than millions, with a large casing-only share that the census should split out.

**Overlap:** - **456** has the strongest overlap. Its design question 2 (elect the head by majority across mentions, not first-seen) is 524's unit 2, and its unit-1 census plans a "foreign mention is the HEAD" bucket that is a subset of 524's unit 1. Proposal: 524 owns head election (decision and build), 456 keeps the bind (design 1), and the census is built once under 524 unit 1 with 456's buckets. Both files should say so.
- **511** item 7 (junk head names on real orgs) defers to 524; no conflict.
- **514** A6 is the 31558903 exhibit; 514 itself is unaffected.
- **522** owns 30914328: the `wrong` verdict unbinds HISCOX and DOC-Labor, so 524 should list it as following 522. Side note: 522's Verify "open: 9" is stale, since the count is now 4.
- **321** (done) is related, not a duplicate: satellites left on the origin after a rehome. It is a reminder that `org_match_keys` reads head plus satellites, so a head refresh needs a match-key re-key.
- **234** is the `name_norm` key and constrains the decision only for identifier-NULL rows.

**Rank:** 6/10 across 520/521/523/524. The defect is systemic and shows on every /v1 surface that serves an org head: org pages, name-prefix search and per-org statistics. It gets worse with every merge campaign that is queued: 519 M6, 469 U7 and 523 all create survivors. The two confirmed exhibits are bad (31558903 served as another university, 22713056 served as `s.o.`).

The value claim is not measured. Only 2 real exhibits exist; the third, 30914328, is 522's fix. "Thousands" is a guess, and the census will probably find mostly casing-only drift. 456 design 3 already serves `mention_name` beside the head in `parties[]`, which reduces the damage on tender pages.

The next unit as proposed is M: a new Spec arm, the gate (~35 min, Box::pin for the arm), a deploy and a dry run. Its bonus is that it also delivers 456's census, pending since 2026-10-01.

It could shrink to S first. Take about 50 recent merge survivors from org_merge_log keeps (indexed reads) and compare each head with its mention names. That gives a drift rate before committing to the M build. Any head refresh has to wait for 511's drains, which is fine because the census is read-only.

## Unit 1 — built (2026-10-14)

A read-only census job, `{"kind":"org-head-drift"}`. It accepts optional `min_mentions` (default 2; under 2 is
refused) and `samples` (default 30, clamped to 1–200). Code:
- `Db::org_head_drift` in `crates/store/src/canonical.rs`;
- the supervisor arm, which is Box::pin'd and sits in the `run_census_spec` group;
- tests in `crates/store/tests/org_head_drift.rs`;
- the operations.md job row.

The issue 456 unit-1 stray buckets ride the same walk.

**What it reports.**
- **Who votes.** Mentions VOTE unless their name is NULL or blank, or is a `partyname::not_a_name` class other than
  `award_summary`. Those classes are counted in `non_name_mentions`.
- **Buckets** on the `altid_name_key`:
  - `aligned` (the head's key is the top key, or tied for it);
  - `minority`;
  - `absent`;
  - `unnamed`, when no mention votes.
- **Flags:**
  - `cosmetic`;
  - `not_a_name`, with `head_class`;
  - `single_stray` (the Sheffield shape);
  - `below_half` (the issue's (b): head key under 50% of voting mentions);
  - `form_only` (a GB legal form apart, under `altid_keys_agree`);
  - the norm-key twins `minority_norm` / `absent_norm`.
- **Splits:** every bucket and flag is split by identifier NULL or not, by provisional, and by merge survivor.
  - A merge survivor has an `organization_merged_identifiers` row with `loser <> org_id`.
  - Carried rows count, which is how 514's E0 survivors read `merged`.
  - p0/p1 folds and rekeys are invisible.
- **456's `stray_mentions`:** a key held once that agrees with no other voting key of its org, under a top key of 2
  or more. Split `by_scheme`.
- **Samples and canaries:** 30 samples per bucket or flag. Canaries are 31558903, 22713056 and 30914328.

**Cost on prod.** Not yet measured. The walk reads every mention through `organization_mentions_org` (about 49.6M
rowid lookups), 2,000 orgs per page. It can be stopped between pages. It refuses while either index is absent (a
rebuild's Phase 1).

**Review.** Three lenses (semantics / cost / conventions), then a fix round, an adversarial re-check, and the
re-check's D1–D5. Fixes:
- **F1:** an alias row (`loser = org_id`) is not a merge.
- **F2:** `below_half` added.
- **F3:** non-name mentions do not vote. Without this, `0` and `-` manufactured the Sheffield shape.
- **F4:** 456's agreement rule replaces exact key equality, and one `STRAY_TOP_MIN = 2` bar serves both strays.
- **F5:** schemes are measured.
- **F6:** test gaps closed (norm vs alt key, a head that sorts last in a tie, provisional apart from identifier).
- **F7:** docs.
- **D1:** the merged-basis wording now covers carried rows.
- **D2:** the voting class label is a store constant, pinned by a supervisor test.
- **D3:** the `below_half` samples list only the aligned plurality heads.
- **D4:** a stray is judged against every other key, not only the top key, so the count no longer depends on which key
  a tie put on top.
- **D5:** the scheme-cap wording.

## Unit 1 — the first prod census (2026-10-15, job 2492)

`org-head-drift min_mentions=2 samples=30` (deployed e2da1e9) walked 5,676,483 orgs and 49,671,073 mention rows in
211 s. The report is `/admin/reports/org-head-drift`.
- **Census:** 2,101,301 orgs with 2+ mention rows and a voting one (46.1M mentions). 3,575,149 are below the minimum,
  310,247 mention rows are nameless, and 33 orgs have no voting mention.
- **Buckets** (orgs / mentions):
  - aligned: 2,043,572 / 41.56M (97.3% of orgs);
  - minority head: 56,144 / 4.51M, of them 8,055 merged and 56,132 identifier-bearing;
  - absent head: 1,585 / 26,249, all identifier-bearing, 3 merged;
  - below half: 67,720;
  - form-only: 591;
  - cosmetic: 6,215, almost all provisional;
  - single stray: 25,238.
- **On the secondary key** (`org_name_norm`): minority 75,337, absent 1,588.
- **Non-name heads:** award_summary 69 orgs (338 mentions), blank 1, placeholder 1 (22713056 `s.o.`, a canary).
- **Stray mentions:** 151,307 on 82,999 orgs. The schemes with the most are `002` 4,007, `GB-COH` 1,690 and `GB-PPON`
  1,162.

Next: the unit-2 decision, read from the 30-per-bucket samples. The minority and absent buckets are where a head
rewrite would act.

## Unit 2 — decided (2026-10-15; panel wf_8a507dec-00b)

The judge's full record is below, verbatim apart from headings. It cites code lines as of 2026-10-15; builders should
grep, not trust offsets.

### The election rule

THE ELECTION RULE (issue 524 unit 2). This synthesizes the three designs: design 1's structure, design 3's 2x margin, and one plain vote key in place of the union-find. The rule is one pure function, `elect_head(votes, head, fns) -> HeadVerdict`, and the dry planner, the wet re-check and the later weekly run all call it. It is applied per org. The steps run in order and the first one that decides wins.

**Inputs**
- **Voters:** exactly the census's voters (canonical.rs 35750-35763). A mention votes unless its name is NULL or blank, or its `not_a_name` class is anything other than `award_summary`.
- **Population:** orgs with `min_mentions` >= 2, the census population.
- **Vote key:** `head_vote_key(s)`. Take `crosswalk::altid_name_key(s)` and drop every token that starts with `§`. Also drop every bare token that `family_token` knows; these come from the initialism collapse, which turns "S.R.L." into `srl` while "SRL" becomes `§srl` (crosswalk.rs 942-968, 1026-1046, 1325-1333). If nothing is left, fall back to the alt key.
  - This one plain key subsumes `altid_keys_agree` (GB trailing forms, crosswalk.rs 1392-1407). So there is no union-find and no transitivity question.
  - It groups spellings, casing, "the", Ltd/Limited/none and S.R.L./SRL/none within ONE org. It merges no orgs.
- **Counts:** clusters are voters grouped by vote key.
  - V = all voting mentions.
  - T = the top cluster's count, and "unique" means no other cluster ties it.
  - H = the count of the cluster holding `head_vote_key(head)`, or 0 if there is none.

**Exclusions (KEEP, never planned)**
- **E1:** `identifier IS NULL`. There, `name_norm` is the issue-234 bind key on every resolver lookup.
- **E2:** the org's triple carries an `org_identifier_verdicts` 'wrong' verdict, or a 'related' verdict with `applied_at IS NULL`. This is the same predicate as `ORG_COLUMNS` (read.rs 4489-4499); there the bind is the defect (issue 456 design 1).
- **E3:** no voter.
- Merge survivors are NOT excluded.

**R0, defective head.** The head is defective when it is empty after trim (test this explicitly: `not_a_name("")` is None, partyname.rs 1184-1187), or when `not_a_name(head)` is placeholder, blank or void_lot.
- Elect the top cluster if T >= 1 and it has a fit spelling.
- On a tie for top: first a cluster with a fit mixed-case spelling, then the larger count, then the smallest vote key.
- Nothing fit means HOLD (no-fit).
- R0 skips the D4 guard.

**D4 guard.** Take the dominant script of the head's letters and of the elected spelling's letters, counting Latin, Cyrillic, Greek and other. If they differ, KEEP the head. It is TED's ENG rendering, the "ENG" leg of ADR-0013 D4: the 29/30 BG absent samples, e.g. 7912592 "Obshtina Belovo" over "ОБЩИНА БЕЛОВО".

**S1, sticky.** If the head's cluster is the top cluster or tied for it, KEEP the head byte for byte.
- This covers aligned heads, form-only heads (all 591, plus the non-GB S.R.L./SRL shapes 10, 25, 208, 301, 745), cosmetic heads and the aligned below-half heads.

**R1, majority with hysteresis.** CHANGE when all four hold:
- T*20 >= V*11 (at least 55%);
- T >= 3;
- T >= 2*H;
- the top cluster is unique (55% implies it).

**R2, stray or absent head.** CHANGE when all four hold:
- H <= 1;
- the top cluster is unique;
- T >= 3;
- T*4 >= V (at least 25%).

H <= 1 under the vote key already implies the head agrees with no other voting key.

**Otherwise KEEP** (the bar).

**Stickiness**
- No cluster of fewer than 3 mentions ever replaces a real name.
- R2 dies once the head holds 2 or more votes.
- To flip an R1 head back, the old cluster must reach 55% AND twice the new head's count. So a single mention, a near tie (84 at 50.3%, 131 at 52.5%) or an abbreviation majority (178: 63 vs 32) cannot move it.

**Display spelling within the winning cluster.** Never use the census's count-first `top_spelling` (canonical.rs 35808-35816); it serves "UNIVERSITY OF SHEFFIELD".
- (1) **Fit spellings only.** A spelling is unfit if any of these holds:
  - it has a C0/C1 control character or U+FFFD (99's "C\x8cUR", 98's "\n");
  - it has more than 200 chars;
  - it has cased letters but no upper-case one (334's "les alchimistes");
  - `not_a_name` is Some (any class, `award_summary` included);
  - `mentions_placeholder` or `mentions_void` hits and the head does not (the 510/511 nets read heads);
  - `consortium_name` is true and the head's is not.
- (2) Not-ALL-CAPS before ALL-CAPS.
- (3) Most mentions.
- (4) Byte-wise smallest.

For 31558903 this gives "The University of Sheffield". No fit spelling in the winning cluster means HOLD (no-fit). There is no ENG-satellite preference inside a cluster, because the words are identical and the satellite is latest-per-language (INSERT OR REPLACE, canonical.rs ~20906-20921), not a quality signal.

**Bind guards (HOLD, listed in the plan, not applied).** The head is a resolver and merge input for identifier-bearing orgs: the altid bar's consortium veto and GB head-vs-head form (canonical.rs 19270-19296), the anchor's owner consortium veto, `legal_form_family` and N2 corroboration (20315-20345), and the guarded/folded owner match (20184-20199). Hold a change when:
- `legal_form_family` is Some on both sides and differs;
- `gb_legal_family` is Some on both sides and differs;
- the old head is `consortium_name` and the new one is not;
- the org has a GB-NHS identifier, and the new head fails `nhs_ods::check(code, head)` where the old one passed.

None->X and X->None family moves are allowed and counted.

**After a CHANGE**
- `name = new spelling` and `name_norm = org_name_norm(new spelling)`.
- One ledger row and one seq-less `organization changed` row.
- No other table is written.

**Canaries the dry plan must report**
- Positive:
  - 31558903, R1 -> "The University of Sheffield";
  - 22713056, R0 -> "Vergabekammer bei der Bezirksregierung";
  - 6341366, R0 -> "Kalmar kommun";
  - 760, R1 -> "Getinge Polska Sp. z o.o.";
  - 61, R1 -> "Tribunal administratif de Nice".
- Negative:
  - 7912592, D4 keep;
  - 178, bar keep (2x);
  - 131, bar keep (55%);
  - 99, bar keep;
  - 10, S1 keep;
  - 334, hold no-fit;
  - 1165973, E1 keep.

A simulation of this rule over all 182 samples is at /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/r524judge/sim.py. Its key approximation, keys.py, reproduces the census `top_key` on 182/182 samples. Both canaries change as expected.

### name_norm and keys

**name_norm FOLLOWS on every change**
- Set `name_norm = org_name_norm(new_head)` (canonical.rs 4586-4591) in the same compare-and-set UPDATE. This is the mint's derivation (canonical.rs 20563-20578).
- It is safe because every change is to an identifier-bearing org (E1), and every name_norm bind or repair lookup carries `identifier IS NULL`. Verified at canonical.rs 2952, 6487, 7177, 19002, 20681, 21838, 29447, 35076, 38554. `repair_provisional_name_norm` walks identifier-NULL rows only (`ORG_IDENTLESS_WINDOW_SQL`, 2937-2938).
- For identifier-bearing rows name_norm is read only by:
  - the name-prefix search and its keyset (read.rs 4584, 4702-4704, 4773-4800);
  - the SSE organization `name_prefix` filter (sse.rs 439-466; an in-place org change diffs as changed, or as an over-delivered removed);
  - `nhs_org_facts`.
- Freezing it would serve 31558903 as Sheffield while `?name_prefix=sheffield hallam` still finds it.
- The invariant `name_norm == org_name_norm(name)` is kept for every row. So an identifier the case reviews later strip (canonical.rs ~32133) leaves a self-consistent key that the issue-432 repair will not re-key.

**Identifier-NULL orgs: untouched in unit 2** (name and name_norm both). Their heads are already mixed case (the 6,211 cosmetic GR/CY/BG transliterations). Only 12 minority, 6 single-stray and 22 below-half orgs are identifier-NULL. A later unit may refresh display only where `org_name_norm(new) == name_norm` byte for byte; that is the 234 key, not the alt key, because minority_norm shows 16 identifier-NULL orgs against 12 on the alt key.

**organization_names (satellite): untouched.** PK (org_id, lang) at canonical.rs 467-473. The old head is kept only in the ledger.

**org_name_verdicts: NO re-key** (this overrules design 1). It is keyed by a name VALUE, not an org (PK name_norm, canonical.rs 535-545). It is read only by the identifier-NULL echo fold and the country-less reuse (9999 and the echo walk). 35287 was a precedent for a change to the key DERIVATION. An identifier-bearing org's key move never touches it.

**org_match_keys: NO targeted re-key** (this overrules the triage and design 1).
- A completed build resets the watermark to 0 (`finish_org_match_keys`, canonical.rs 13935-13948). The next wet build drops the table and rebuilds wholesale from head plus satellites (`reset_org_match_keys` 13788-13797; `build_org_match_keys_batch` 13825ff; supervisor.rs 14005-14035).
- It is already pushed weekly (supervisor.rs ~15689; 85 s on prod).
- There is no org_id-leading index (`org_match_keys_kk(key_kind, key, org_id)`), so a targeted delete would be wrong anyway.
- Action: enqueue `{"kind":"build-org-match-keys","dry_run":false}` after the full wet run, or accept up to a week of stale wall counts. Run it outside fold windows.

**Election key: the vote key**, i.e. `altid_name_key` minus legal-form tokens; see the decision. It is not `org_name_norm`: that key splits "The"/"Ltd"/"Limited" (minority 75,337 on norm against 56,144 on alt, about 19k extra flips that are spelling noise). `altid_name_key`, `altid_keys_agree` and `family_token` stay unchanged. The new `crosswalk::head_vote_key` wraps them, so the altid arm, the R2/R3 vetoes and `NAME_KEY_EPOCH` do not move.

**Change feed**
- One `('organization', id, NULL, 'changed')` per moved org, in the window transaction, with `publish_cursor` after COMMIT.
- No Tender or lot rows:
  - `parties[].organization_name` is a read-time join (read.rs 3057/3069; winners 3371);
  - tender SSE filters read no org name (`name_prefix` is honoured on Organizations only, read.rs ~807);
  - in-place org writes announce the organization only (identifier restore, canonical.rs ~32298-32313).
  The ADR amendment states this reading of ADR-0017 D5.

**REST cursor (adjacent fix, unit 2b).** v1/mod.rs:1808 builds the next cursor from `last.name.to_lowercase()`, not from the stored name_norm. `organizations_by_name` discards the name_norm it reads (read.rs 4501-4517, 4730-4750). This bug exists today for names with collapsed whitespace or trailing punctuation. Unit 2 adds no new divergence, because name_norm follows via `org_name_norm`, but carry the row's name_norm in the cursor while touching this surface.

### ADR-0013 amendment (to commit with unit 2a)

Append to docs/adr/0013-language-model.md. Commit it with unit 2a so the dry plan's semantics are recorded; it takes effect at unit 2b's first wet run.

## Amendment 2026-10-15 — D4's head is ELECTED from the org's mentions, not first-seen (issue 524)

D4 called `organizations.name` "the ENG-or-original denormalized head". The mint never did that. It writes the first mention's first-seen spelling, in whatever language (`ingest::project`: "the designated single head keeps its first-seen semantics"), and nothing ever rewrote it. So merges, drops and in-place mention refreshes left heads that few or none of an org's mentions say. Census job 2492 counted 56,144 minority heads and 1,585 absent heads, among them 31558903 served as "Sheffield Hallam University" over 16 Sheffield mentions, and 22713056 served as "s.o.".

Decided:

1. **Who.** The mint still writes the first-seen spelling. The dry-first, ledgered job `refresh-org-names` re-elects the head of IDENTIFIER-BEARING organizations only. An identifier-NULL organization keeps its head, because its `name_norm` is the issue-234 bind and provisional-merge key. So D4's provisional-merge key and merge semantics do not change.
2. **Votes.** A mention votes unless its name is NULL, blank, or a `not_a_name` class other than `award_summary`. Votes group by the head vote key: `crosswalk::altid_name_key` with every legal-form token dropped. Casing, spelling marks, "the" and a legal form (Ltd/Limited/none, S.R.L./SRL/none) are therefore one name.
3. **When the head changes.** Only when one of these holds:
   - (a) the head is empty or a non-name (placeholder, blank, void_lot) and some name votes;
   - (b) a cluster other than the head's holds at least 55% of the votes, at least 3 votes, and at least twice the head's votes;
   - (c) the head's cluster holds at most 1 vote, and a unique top cluster holds at least 3 votes and at least 25%.

   Otherwise the head stays byte for byte. No single mention, near tie or legal-form difference moves it.
4. **D4's ENG-or-original survives as a guard.** A head whose dominant script differs from the elected spelling's (TED's Latin rendering over Cyrillic or Greek mentions) is kept: it is the ENG head.
5. **The served spelling.** Within the winning cluster, the job serves the most-voted FIT spelling:
   - no control character or U+FFFD;
   - no non-name class;
   - no placeholder or void net hit;
   - no consortium mark the old head lacks;
   - at most 200 characters;
   - not all lower case.

   Mixed case comes before ALL-CAPS, and byte order breaks ties. With no fit spelling, nothing changes. Register names are not used until a register-name cache exists.
6. **Held for review.** A change is held when it would:
   - swap one legal-form family for another (`legal_form_family` or `gb_legal_family` set on both sides and different);
   - drop a consortium mark;
   - fail the NHS ODS name check the old head passed.

   The head is an input to the resolver's and merge planners' name gates.
7. **Keys.** `name_norm` follows, as `org_name_norm(new head)`. For identifier-bearing rows it is the name-search key, never a bind key. `organization_names` and `org_name_verdicts` are untouched. `org_match_keys` catches up at its next wholesale build.
8. **Announcement.** Every change writes a byte-exact pre-image to `org_name_refresh_log` (revertible by job) and one seq-less `organization changed` row. No Tender or lot rows are written: `parties[].organization_name` is a read-time join of the organization, so under ADR-0017 D5 the moved entity is the organization. This follows the identifier-restore precedent.
9. **Upkeep.** A weekly capped auto-run keeps heads current after merges and drops. A from-archive rebuild restores first-seen heads until the job runs again.

What a notice published stays available as `parties[].mention_name` (issue 456 design 3). Issue 456's design question 2 is answered here.

### Build units

For any focused test run, use the gate flags AND the gate package set: `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server <name>`. Then run `ops/check.sh` redirected to a file, and READ `GATE-EXIT=`. Push with `git push origin HEAD:main HEAD:<handover>`.

**UNIT 2a (M): pure election plus a DRY planner. No writes.**

Files:
- **crates/ingest/src/crosswalk.rs:**
  - `pub fn head_vote_key(name: &str) -> String`, which is `altid_name_key` minus `§*` tokens and minus bare `family_token` tokens, falling back to the alt key when that leaves nothing.
  - Tests:
    - "WECO TMC" / "WECO TMC S.R.L." / "WECO TMC SRL" give one key;
    - "The University of Sheffield" and "UNIVERSITY OF SHEFFIELD" give one key;
    - "Arkil" and "Arkil Ltd" give one key;
    - "SARL Fradin" and "FRADIN SARL" give one key;
    - "S.R.L." alone gives a non-empty key;
    - `altid_name_key` and `family_token` are unchanged.
- **crates/ingest (crosswalk.rs or partyname.rs):** two composite predicates for injection.
  - `head_spelling_unfit(candidate, head) -> Option<&'static str>`. It returns control, long, lowercase, non_name, net or consortium, as listed in the decision.
  - `head_change_hold(old, new, identifier_kind, identifier) -> Option<&'static str>`. It returns family (`legal_form_family`), gb_family (`gb_legal_family`), consortium_removed or nhs_check (`nhs_ods::check` for GB-NHS codes).
  - Unit tests on the sample spellings: 99 control, 334 lowercase, a consortium spelling, plc->Ltd held, None->spzoo allowed (760).
- **crates/store/src/canonical.rs**, beside `Db::org_head_drift` (or a new store module):
  - The pure `elect_head(votes: &[(&str,u64)], head, &HeadElectFns) -> HeadVerdict {Keep(reason) | Hold(reason) | Change{rule, new}}`. Keep reasons are E1/E2/E3/S1/D4/bar; rules are R0e/R0n/R1/R2.
  - `Db::plan_org_name_refresh(args) -> OrgNameRefreshPlan`. It reuses `ORG_HEAD_DRIFT_ORGS_SQL`, `org_head_drift_mentions_sql`, IN_CHUNK, the page size of 2,000 and the index refusal.
  - For candidates only, it adds:
    - the identifier-verdict probe (the `ORG_COLUMNS` predicate);
    - the `organization_names` PK seek for ENG on D4-held orgs, reporting equal / agrees / absent.
  - The plan holds:
    - `planned` entries `[org_id, rule, old_name, old_name_norm, new_name, new_name_norm, V, T, H, second, clusters, merged, family_old, family_new, top-5 spellings with counts]`, capped at 100k, with `truncated`;
    - counts by rule, by keep reason and by hold reason;
    - the empty-head count;
    - the mention sum of the planned orgs;
    - canary verdicts;
    - 30 UNIFORM samples per rule and per reason, chosen by smallest `(org_id * 2654435761) mod 2^32`, not the first 30 by id;
    - the binary rev / rule epoch.
- **crates/app/src/supervisor.rs:**
  - The `{"kind":"refresh-org-names","dry_run":true,"min_mentions"?,"samples"?,"org_ids"?}` parse. It refuses `dry_run:false` in 2a.
  - A Box::pin'd arm in the `run_census_spec` group.
  - The kind in `STOPPABLE_KINDS` (4373, with its contract test).
  - `put_report("org-name-refresh-plan")`.
  - The canary list above.
- **crates/store/tests/refresh_org_names.rs** fixtures:
  - Sheffield (R1, "The University of Sheffield");
  - s.o. (R0);
  - an empty head (R0);
  - a BG Latin head over Cyrillic (D4 keep);
  - the 178 shape (2x keep);
  - 84/131 at 50-53% (keep);
  - 10/301 form-blind (S1);
  - 334 (hold no-fit);
  - identifier-NULL (E1);
  - a 'wrong' verdict (E2);
  - T=2 (bar);
  - an R2 stray at 25% (change);
  - an R2 top tie (keep);
  - a plc->Ltd change (hold).
- **docs/operations.md:** the job row.
- **Issue files:** record the decision in 524, and point 456 design 2 here.

Prod for 2a:
1. Gate, then deploy in a queue gap. Not during 534 U3's refold (~7.5 h), not between an altid / nhs-body / r3 dry run and its wet run.
2. Run one dry job when the queue is idle (~211 s walk plus seeks).
3. Read the counts, the uniform samples and the canaries.
4. Make one bounded /v1/sql read: `SELECT org_id, lang, name FROM organization_names WHERE org_id IN (7912592,8068512,8240078,8392264,6341366) LIMIT 40`.
5. Go to 2b only if both hold:
   - every canary is as expected;
   - at most 1 wrong in each 30-sample R1/R2 list.

**UNIT 2b (M): wet, revert, ledger and docs.**
- **Ledger table** `org_name_refresh_log(org_id, at, job_id, rule, old_name, old_name_norm, new_name, new_name_norm, evidence JSON, reverted_at, reverted_job, PRIMARY KEY(org_id, at)) STRICT`. No FK. Not on the /v1/sql allow-list (the `org_merge_log` precedent).
- **`Db::apply_org_name_refresh`:**
  - Refuse if there is no stored plan, the plan is truncated, its epoch or rev differs, or the index is absent.
  - Pre-pass: re-elect every planned id. Abort with no write if drifted plus gone ids exceed max(2%, 5).
  - Windows of 2,000 ids, each through `Db::immediate` (tx.rs 27), holding:
    - `UPDATE organizations SET name=?, name_norm=? WHERE id=? AND name=? AND name_norm IS ?`;
    - a ledger INSERT;
    - `append_change('organization', id, NULL, 'changed')` (canonical.rs 41066).
  - After each COMMIT: `publish_cursor` (lib.rs 1417), `record_job_progress` (jobs.rs 237), a TRUNCATE checkpoint, `set_phase`, a stop check, and `resume_after`.
  - Scope options: `org_ids`, `max_groups`.
- **`revert_job`:** a compare-and-set restore guarded on the new pair. It stamps `reverted_at`, announces `organization changed`, and counts orgs that were renamed again or are gone.
- **The kind** goes in `heavy_write_kind` (15968).
- **Docs:**
  - the ADR-0013 amendment (already committed in 2a; mark it effective);
  - the openapi `Organization.name` description (crates/app/data/openapi.json ~1023);
  - the `organization_name` descriptions and the docs.rs party paragraph;
  - a sql.rs column note;
  - CHANGELOG (the meaning of an existing column changes, ADR-0015 D1);
  - the operations.md runbook.
- **The v1/mod.rs:1808 cursor:** take it from the stored name_norm, which `read::organizations_by_name` returns.
- **Tests:**
  - a replayed window changes nothing;
  - a revert restores the old pair;
  - one change row per moved org;
  - name_norm follows;
  - identifier-NULL rows stay untouched;
  - the cursor survives a diverged name.

Prod for 2b:
1. Gate, then deploy.
2. Run a fresh dry plan on that binary.
3. Wet run with `org_ids=[31558903,22713056,6341366,760,61]`. Expect 5 ledger rows. Check each with `curl -s https://tenders.zebreus.click/v1/organizations/31558903 | jq -r .name` (done = a University of Sheffield spelling).
4. Wet run with `max_groups=500`. Spot-check 30 ledger rows with GETs.
5. Full wet run.
6. Re-run the dry plan; expect about 0 planned.
7. Re-run `org-head-drift`; minority and single_stray should fall by about the applied count, and the negative canaries should be unchanged.
8. Run `build-org-match-keys` wet.
9. Re-run any pending altid / nhs-body / r3 dry plans, whose wet parity fails closed on changed heads.

**UNIT 2c (S): upkeep.**
- Add `refresh-org-names-auto` to the weekly org chain, ahead of `build-org-match-keys`.
- It runs dry, then applies only when planned <= 2,000. Otherwise it stores the plan, writes nothing and logs a loud line to the journal (the `AUTO_SWEEP_CAP` precedent, supervisor.rs 3856/5695).
- Runbook lines: merge campaigns end with a dry run scoped to `org_ids=<keeps>`; a rebuild is followed by a full run.
- Update or retire the census canary 30914328 (`ORG_HEAD_DRIFT_CANARIES`, supervisor.rs 1228); 522 dissolved it.

**LATER (not unit 2):**
- form-only flips (GB plc->Ltd, 448's root; needs the hold review);
- ALL-CAPS -> mixed case within aligned clusters;
- identifier-NULL display within the same name_norm;
- register names, once a UKRLP/CC/FCA/CH name cache exists;
- 46's junk-sentence head (a not_a_name lexicon gap, the 511 line);
- conflated identifiers (456 design 1).

### Expected effect (simulated on job 2492's samples)

These figures come from job 2492's totals and its 30-per-bucket samples, simulated with the rule as decided (sim.py at /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/r524judge/). The samples are the first 30 by id, which skews toward old, large orgs. The T >= 3 bar bites harder on the many small orgs, so read the point estimates as an upper half.

**Changes 0:**
- aligned, 2,043,572 orgs (S1);
- aligned plurality below half: 67,720 - 56,144 - 1,585 = 9,991 orgs (S1);
- cosmetic, 6,215 orgs: 6,211 identifier-NULL (E1) and 4 identifier-bearing (S1); 30/30 samples kept;
- form_only, 591 orgs (S1 through the vote key; 30/30 kept);
- all identifier-NULL orgs, by exclusion: minority 12, single_stray 6, below_half 22, cosmetic 6,211.

**single_stray, 25,232 identifier-bearing orgs:**
- Samples: 21/30 change (7 by R2, 14 by R1), 4 kept by S1 (form-blind), 4 kept by the bar (T < 3), 1 held no-fit (334).
- About 17,700 orgs (95% band about 13.1k-21.0k; likely lower).
- Changed samples:
  - right (14): 98, 140, 338, 396, 760, 762, 763, 764, 765, 879, 944, 1033, 1074, 1620, 1309;
  - neutral (7): 115, 1073, 1132, 1710, 1720, 1758.

**Other minority, 56,144 - 25,238 - 591 = about 30,315 orgs:**
- Samples: 10/27 change, about 11,200 orgs (band about 6.5k-16.9k).
  - right: 61, 96, 100, 120, 121, 173;
  - neutral: 8, 102, 153, 222.
- Kept by the bar: 45, 46, 51, 80, 84, 97, 99, 105, 116, 130, 131, 146, 178. 178's abbreviation majority is stopped by 2x; 84 and 131 are stopped by 55%.
- Kept by S1: 10, 25, 208, 262.

**absent, 1,585 orgs, all identifier-bearing:**
- Samples: 1/30 change (6341366, '' -> 'Kalmar kommun', R0), about 50 orgs.
- 29/30 are held by the D4 script guard, about 1,530 BG/GR transliterated heads.
- The tail above id 8.4M was never sampled; the dry plan counts it.

**Small classes:**
- non-name heads: 2 change (22713056 s.o., and 15906016 'NC+' -> 'NC + S.A.', a harmless false-blank brand), plus the empty heads hiding in absent (at least 1);
- award_summary heads, 69 orgs: change only via R1/R2, so at most 69.

**Total:**
- About 29k orgs changed, plausible range about 18k-38k.
- All identifier-bearing.
- About 1.4% of the 2,101,301-org census, or 0.5% of 5,676,483 orgs walked.
- About 15% are merge survivors (minority merged 8,055/56,144; single_stray 3,781/25,238).
- Upper bound on the joined tender reach: the planned orgs' mention sum, under 4.54M. No tender rows are written.

**In-sample quality over the 35 distinct changed orgs (canaries included):**
- 24 right (69%), 11 neutral respellings (31%), 0 wrong.
- Against design 1's rule: 178 (wrong) is now kept, and form churn (10, 25, 208, 301, 745) stays kept.
- Against design 3's rule: 121, 173, 98, 764, 765, 1033 and 1309 are now fixed. Design 3's conflated exclusion or lack of a stray rule left them.

**Canaries:** 31558903 -> R1 "The University of Sheffield" (16/17 = 94%, T >= 2H); 22713056 -> R0.

**Cost:**
- Dry: about 4 min read-only (the census walk was 211 s over 49.7M mention rows, plus candidate seeks).
- Wet: about 15 windows of 2,000 and about 1-2 min, plus a 10 s parity pre-pass; about 29k `organization changed` rows.
- Plan report: about 3-5 MB.

### Rejected, and why

**Plain count-majority on spellings (the issue's first idea).**
- It elects "UNIVERSITY OF SHEFFIELD", because the census `top_spelling` is count-first (canonical.rs 35808-35816).
- It would turn the 6,215 cosmetic mixed-case heads ALL-CAPS.

**`org_name_norm` as the election key.**
- It splits "The" from no "The", and Ltd from Limited.
- minority_norm is 75,337 against 56,144 on the alt key: about 19k extra flips that are spelling noise.

**A plain 50% / strict-majority bar (design 3).**
- It flips the coin-flip conflated identifiers 84 (50.3%) and 131 (52.5%, Grasse -> Marseille).
- 55% gives a 10-point dead band.

**Design 1's T-H >= 2 margin alone.**
- It lets through 178's abbreviation majority ("CA Plaine Vallée" 63 vs 32). T >= 2H stops it.
- It costs none of the samples' right changes: 61, 96, 100, 121, 173 and 222 all clear 2x.

**Design 3's conflated exclusion** (distinct keys >= 10 and top share < 2/3).
- It drops right changes: 121 (BMI "und für Heimat", 60%) and 173 (Hamburg, 58%).
- It is redundant with 55% plus 2x. That bar already keeps every conflated sample: 45, 84, 97, 105, 116, 131, 146.

**Design 3's majority-only rule (no stray rule).**
- It leaves single-stray junk heads served: 764 and 765 (PL heads with an address, NIP and REGON), 98 (a multi-line consortium list), 1033 ("TAR BRESCIA" on Palermo's code) and 1309.
- R2 needs H <= 1, T >= 3 and at least 25%. It can never fire again once a head holds 2 votes.

**Design 1's union-find over `altid_keys_agree` plus form-blind edges.**
- One plain vote key (the alt key minus form tokens) gives the same grouping. It subsumes `altid_keys_agree` and has no transitivity question.

**ENG-satellite-first spelling inside the winning cluster (design 3).**
- The satellite is latest-per-language (INSERT OR REPLACE, canonical.rs ~20906-20921), not a quality signal.
- GB/IE notices label names ENG routinely, so it could serve the ALL-CAPS latest spelling.
- The spellings in one cluster are the same words. D4 acts through the script guard instead.

**Design 1's NHS veto and t/a tie-break inside the spelling order.**
- The NHS check moves to the bind-guard holds.
- The t/a tie-break is dropped to keep the order to four keys.

**Register names now.**
- There is no UKRLP/CC/FCA/CH name cache in crates/. The only one is the frozen ODS table, which is upper case.

**Freezing name_norm (the triage's "probably not").**
- Search would still find 31558903 under "sheffield hallam".
- name_norm is not a bind key for identifier-bearing rows: all binds carry `identifier IS NULL` (canonical.rs 2952, 6487, 7177, 19002, 20681, 21838, 29447, 35076, 38554).

**Identifier-NULL refresh within the key (design 2), now.**
- Its value is nil: the cosmetic heads are already mixed case, and only 12 orgs are in minority.
- It needs the exact-name_norm gate, because `repair_provisional_name_norm` re-keys and folds a diverged row (canonical.rs 35008, 35353). Deferred.

**Re-keying `org_name_verdicts` (design 1).**
- That table is keyed by name value and read only by identifier-NULL paths.

**A targeted `org_match_keys` re-key (the triage, design 1).**
- The build is wholesale from zero and runs weekly.
- There is no org_id index.

**Rule T tender correction rows (design 2).**
- They would mean millions of rows for a join-time display value.
- In-place org writes announce the organization only (identifier restore, canonical.rs ~32298-32313).
- No tender filter reads an org name.
- The ADR amendment states the D5 reading instead.

**Re-election inside the merge transactions (design 2).**
- There are more than 10 merge paths. Each would need ingest's key functions and a full read of the keep's mentions in-tx; a hub keep can hold 66k mentions (org 105).
- The capped weekly auto-run (2c) meets the upkeep need. ADR-0017 E6 is the fold-consistency test for R2 backfills; this is a maintained-state job like `sweep-orphan-orgs` auto.

**Design 3's "any `org_identifier_verdicts` row" exclusion.**
- It is narrowed to `ORG_COLUMNS`' predicate ('wrong', or 'related' not yet applied). An applied related verdict or a 'right' verdict is no bind defect.

**Excluding merge survivors.**
- Survivors are the motivating class: 31558903, and 8,055 of the minority orgs.

**Holding every legal-family change.**
- None -> X would hold the right PL address-tail fixes. 760 and 1620 gain `spzoo` once the address is cut.
- Only X -> Y is held.

### Open risks

1. **Neutral churn.** About 30% of changes are respellings: abbreviation majorities that clear 2x (102 VSIA, 153 UAB), legal-form prefixes (8 SC), the long HU form (1710), "ENTECH SE" -> "ENTECH" (1758). "S.A." is not a `family_token`, so "X" vs "X S.A." still flips (1309). Each change costs one org change row. Accepted; the samples show no wrong change.
2. **The abbreviation-majority shape.** It can still win where the abbreviation holds at least 2x the full name (the CA/CC/TA/TAR/CCI family). No key-level guard exists. Read the uniform R1 samples for it before the full wet run.
3. **The D4 script guard can keep a WRONG cross-script head** (a foreign Latin stray on a Cyrillic org). No sample shows one. The ENG-satellite question is UNVERIFIED: the bounded /v1/sql read on 7912592, 8068512, 8240078, 8392264 and 6341366 has not been run (forbidden here). The dry plan's ENG equal/agrees/absent counts settle it.
4. **The head is a bind input** for identifier-bearing orgs: the consortium veto, the legal-form head-vs-head check and N2 corroboration (canonical.rs 19270-19296, 20184-20199, 20315-20345). X -> Y family changes and consortium removals are held, but None -> X adds veto potential and X -> None removes it. The dry plan counts both, and a later re-resolve can bind differently.
5. **Sample bias.** The samples are the first 30 by id, old and high-volume; the 95% bands are about plus or minus 16-19 points. The estimates (about 29k, range 18k-38k) are soft. The dry plan's uniform hash samples are the go/no-go. The absent tail above id 8.4M was never sampled.
6. **The 510/511 nets read heads** (`mentions_placeholder` contract, partyname.rs ~1225-1253). The drains through unit 6 are done. The 529/545 lexicon changes post-date job 2492, so the dry plan must run on the deployed binary, and again after 545 if it changes `not_a_name`. A refresh before a future drain hides junk-minority orgs from the net.
7. **Stale plans and contention.** Stored altid / nhs-body / r3 dry plans go stale when heads change, and their wet parity fails closed, so never interleave. Queue contention: the 534 U3 refold (about 7.5 h), 519 M6, the 523 merge campaigns.
8. **No rebuild-proofing.** A from-archive rebuild re-mints first-seen heads under new ids. The ledger is then historical only, and the job must re-run. Drift regrows between weekly runs (2c).
9. **Kept wrong by design:**
   - 46 AT, a junk-sentence head with 641 mentions: a `not_a_name` lexicon gap, the 511 line;
   - conflated identifiers 45, 105, 116, 131: issue 456 design 1;
   - small orgs under T = 3, e.g. 320's CAO/CAP typo;
   - 99, whose mojibake winner is only 32% anyway.
10. **Consumer contract.** Tender payloads' `organization_name` changes with no tender change row. An SSE `name_prefix` organization subscriber may get an over-delivered removed (sse.rs ~464). This must be written in the ADR amendment, the openapi entry and CHANGELOG (ADR-0015 D1 covers existing column meaning). If ADR-0017 D5 is read strictly to require tender rows, that costs 50-100 times more.
11. **The REST name cursor bug** (v1/mod.rs:1808, `name.to_lowercase()`) exists today. It is fixed in 2b, not by the rule.
12. **NHS.** The `nhs_ods::check` hold depends on the frozen ODS table (ods-2026-10-11). The 519 M6 GB-NHS identifier cohort is pending; prefer running 2b's full wet after M6 or excluding its orgs.
13. **Line drift since the triage.** These line numbers have moved:
    - the mint is at canonical.rs 20563-20578, not ~20171;
    - the name_norm writes are at 34823, 35353 and lib.rs 4067, not 34430/34960/4065.

    Builders should grep, not trust offsets.
