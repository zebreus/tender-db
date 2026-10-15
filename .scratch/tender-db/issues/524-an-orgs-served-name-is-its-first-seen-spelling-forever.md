# 524 — an org's served name is its first-seen spelling forever, so merges and drops leave a wrong or junk head

Status: ready-for-agent — RULE v9 DECIDED 2026-10-15 (`## Rule v9 — decided`, below; panel wf_5cca11b1-622: three designers, two adversarial judges) after plan 2526's dated re-judge failed three bars under v8. v9 adds five pure serve holds (review_body, organ, code_added, address_added, caps_variant), narrows the digits release (glue + junk-only), and pre-builds per-class wet-run filters (`exclude_rules`, `exclude_released`) so one failed list no longer stops the others. No DB read is added. NEXT: build (an agent, then a 3-lens review), gate, deploy, re-plan, the fresh dated re-judge, canaries, capped 500, full. No wet run under v8.
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

## Unit 2a — built, deployed, first dry plan (2026-10-15)

- **Built** by an agent from the decision. A 3-lens review found 16 findings, all confirmed apart from one no-defect
  lens. The core rule was faithful. The one code defect: `all_caps` counted `ß`, `ª` and `º` as lower case, so a
  German or Portuguese ALL-CAPS spelling ranked as mixed case. Fixed, with 13 test and doc gaps, in one fix round.
- **Gated and deployed:** green at `fa2677c` (1,214 s), deployed 05:11 UTC.
- **Dry plan, job 2501** (109 s, rule `524u2-v2`). It walked 5,676,482 orgs and 49,671,069 mention rows, and elected
  2,101,333 of them.
  - **Changes: 30,832** (2,215,214 mentions; 4,155 on merge survivors): R0e 1, R0n 2, R1 28,881, R2 1,948.
  - **Holds: 533**: consortium_removed 244, no_fit 220, family 55, gb_family 13, nhs_check 1.
  - **Keeps:** D4 1,732, E1 1,547,068, E2 2, E3 6, S1 502,657, bar 18,503.
  - **D4 ENG satellites:** absent 1,678, agrees 2, differs 40, equal 12.
  - **Canaries:** 11 of 12 as expected. 31558903 → "The University of Sheffield"; 22713056 → "Vergabekammer bei der
    Bezirksregierung"; 6341366 → "Kalmar kommun". **334 read R1 "Les Alchimistes SAS" where the expectation said
    hold:no_fit.** The expectation is stale: its mentions are `les alchimistes` ×2, `LES ALCHIMISTES` ×2,
    `Les Alchimistes SAS` ×1 and `Les Alchimistes Loire Atlantique` ×1. The vote key drops `SAS`, so 5/6 votes form
    one cluster, and the fit mixed-case spelling is the SIREN's own name. The change is right; fix the expectation.
- **Sample judgement** (workflow wf_2ac4aece-d33). The 30 uniform samples per change rule were classified, and every
  "wrong" got an independent skeptic, who refuted 4 of 8:

  | rule | right | neutral | wrong |
  | --- | --- | --- | --- |
  | R1 (28,881) | 19 | 10 | **1** (13731240) |
  | R2 (1,948) | 13 | 14 | **3** (15075247, 13839658, 17348355) |
  | R0 (3) | 2 | 1 | 0 |

  Weighted, that is about 960 + 195 ≈ 1,150 worse names out of 30,832 (≈3.7%). The causes:
  1. **Latin diacritics are not folded in the vote key.** `match_norm` folds only Greek tonos, so `Comuna Dumbrăveni`
     and `COMUNA DUMBRAVENI` are two clusters. R1 fires and serves the ALL-CAPS ASCII form (13731240, the one R1
     wrong). The same split makes several accent-only R1 flips neutral at best (12615968 Szervíz, 16531730 Dotación,
     11385081 Géosat).
  2. **R2 swaps a full name for an abbreviation or truncation.**
     - 15075247: `Communauté de communes Terroir de Caux` → `CDC Terroir de Caux`.
     - 13839658: `C 3 I - CONCEPT EN INGENIERIE D'INFRASTRUCTURES INNOVANTES` → `SAS C3i`.
     - 17348355: `Dräger Safety România` → `DRAEGER ROMANIA`, the parent's name on the subsidiary's CUI.
     R2's 25% bar lets a minority shorthand beat a stray but correct full head.
- **Holds over-block** (they write nothing, so the cost is missed fixes, not damage):
  - consortium_removed: 10 of 12 unjustified. The identifier is the lead member's own, and 57–86% of votes name it.
  - family: 8 of 12. Some are false family reads with the same form on both sides, e.g. an `Energie AG` brand inside
    an s.r.o. name.
  - gb_family: 8 of 12. `gb_legal_family` misparses `Public Ltd Company`, and reads a trailing ownership note.
  - no_fit: 1 of 12 (`vivida bkk`, a lower-case brand).
  - nhs_check: 1 of 1 justified.
  - D4 keeps: 12 of 12 justified. All have eng=absent; 21652539 has transliterator casing artifacts.
  - bar keeps: 10 of 12. 6071658 is EPF Poitou-Charentes → Nouvelle-Aquitaine, a rename the 2x hysteresis blocks.
    5778 is a tribunal's name on a private company's SIRET, issue 456 design 1.

### Unit 2a.1 (next)

1. **Fold Latin diacritics in `head_vote_key` only** (NFD, then drop the combining marks; `altid_name_key` is left
   alone). Within a cluster the display order prefers the diacritic-bearing spelling among the fit mixed-case ones.
   Then 13731240 becomes an S1 keep, as do the accent-only flips.
2. **R2 abbreviation and truncation hold.** Hold an R2 change when:
   - (a) the new name has a 2–5 letter token equal to the initials of a run of the head's words (`CDC` ↔
     `Communauté de communes`); or
   - (b) the new name's vote-key tokens, with single-letter runs joined (`C 3 I` → `c3i`), are a proper subset of the
     head's, and the dropped tokens are all alphabetic words (not digits, postcodes or `ul.`). This leaves R2's
     address and NIP strip-downs (764/765) changing.
   Re-check the 13 right R2 samples against both arms before building.
3. **Fix the 334 canary expectation** to change:R1 "Les Alchimistes SAS".
4. Re-run the dry plan, re-judge the samples with the same workflow, and decide 2b's go from that reading. Refine the
   holds (consortium lead-member identifier, legal_form_family first-token misreads, gb_legal_family `Public Ltd
   Company` and trailing ownership notes) as a later unit; they only block.

### Unit 2a.1 — built, reviewed, held back (2026-10-15)

- Built as `0c52fef` and gated green (1,835 s). It is NOT on main: its review (workflow wf_970120c3-cb0, with a
  Python port of the keys replayed over the judged samples) found:
  - **The fold alone fixes 3 of the 4 judged wrongs.** 13731240 and 13839658 become S1 keeps. 17348355 becomes a bar
    keep (H=10, 12/22 < 55%). 15075247 (CDC) is caught only by the initialism hold. Six accent-only R1 flips also
    become S1. Cost: two right R1 changes are lost (2969471 õ→ő, 10982286 → Hydria).
  - **The R2 holds over-block.** 447 of 1,948 R2 changes are held (123 initialism, 324 truncation), 4 of the 13 right
    samples among them. The initialism arm fires on legal-form abbreviations (`SRL`, `SA`), on 2-letter tokens (`EY`
    ↔ Ernst & Young) and on short words in ALL-CAPS names. The truncation arm's motivating case is already fixed by
    the fold.
  - **The display order regressed.** "More non-ASCII letters first" applied across a whole cluster, which also merges
    t/a clauses, parentheticals and legal forms. So `Acme Ltd t/a Café Rouge` (1) beat `Acme Ltd` (20), and
    `Acme Sàrl` (1) beat `Acme GmbH` (20).
  - **A family-hold blind spot.** `Sàrl` / `S.à r.l.` drop out of the folded vote key, but the unfolded
    `legal_form_family` cannot see them, so AG→Sàrl changed with no hold.
  - **Mojibake.** Italian word-final `UniversitÃ\u00a0…` folds into the right cluster and wins on marked letters.
- **Unit 2a.2** (builder running):
  - the marked-letter preference applies only between true twins (equal after lower, fold and dropping punctuation);
  - the family hold folds before reading the family;
  - a `mojibake` unfit reason;
  - the truncation arm is dropped;
  - the initialism arm needs a 3–5 letter upper-case token in a non-ALL-CAPS spelling, never a legal-form token,
    matching a run of 2 or more head words;
  - rule epoch `524u2-v4`.
  Deploy after the 534 unit-3 refold, then re-run dry and re-judge.

### Units 2a.2 and 2a.3 (2026-10-15)

- **2a.2** (`b8aa743`, gate green). Twin-only accent preference, folded family hold, mojibake reason, the truncation
  arm dropped, and a tight initialism arm. Replayed over plan 2501's 1,948 R2 changes: 22 held, against 447 under
  2a.1. On the 30 judged samples only CDC is held.
  - Its review (wf_12233d66-5ab) confirmed five small should-fixes:
    - the twin swap ignored votes (1526605: a 1-vote typo twin beat 717 votes; `Zakłąd`; `spółką`;
      `FrieslandCâmpina`);
    - mojibake misses (cp1252 `€`/`™`, and `Ã` before a space: 14272015 went from hold to serving mojibake);
    - the folded family read missed the dotted `S.à r.l.` / `S.p.A.`;
    - the ALL-CAPS gate exempted a bare initialism;
    - the legal-form list was too narrow.
- **2a.3** (built on top, rule `524u2-v5`):
  - a twin swap replaces only an accent-free winner, picking its most-mentioned marked twin;
  - the mojibake set gains `€`, `™`, `È™`/`È›`, and `Ã`/`Â` after a lower-case letter before a space or the end;
  - `folded_family` also reads single-letter runs joined (`s a r l` → `sarl`);
  - a bare initialism holds whatever its case, and particles are skipped inside a run (`CDCPC`);
  - the designation list grows: sca scs scg spl snc scop sci selarl selas eurl sasu pph fuh phu zoz spzoz spz ett
    grsk.
  - The builder's replay over 2501:
    - twin swaps 154 → 124: about 10 typo fixes, about 20 fuller accents lost (`Alcaldía` → `Alcaldia`, `Cœur`,
      `Saúde`…);
    - R2 initialism holds 10 → 26 (CCAS ×4, CCI ×2, GIP, GECT, TGSS, AMPA, ОДМВР, bare ANER/BSO/EPLO/CTS/SOM);
    - 31 dotted-form family holds (S.r.l. ↔ S.p.A., m.b.H. ↔ AG, B.V. ↔ GmbH).
- **Owner decisions:**
  1. **Twin swap: keep v5.** A minority spelling must never beat a majority that already carries diacritics. Counts
     cannot tell a fuller-accent twin from a typo twin, and the ~20 losses are minor omissions on the same entity.
  2. **Canary 178 is re-decided as neutral.** The folded key gives `CA Plaine Vallée` 66 votes against the head's
     32 (2x passes). It is the buyer's own published form, the same body as `Communauté d'agglomération Plaine
     Vallée`. A 2-letter R1 initialism guard would also hold right brand changes (`EY`), so R1 stays un-second-
     guessed. The canary now expects change:R1 "CA Plaine Vallée".

## Unit 2b — deployed and the wet run started (2026-10-15)

- **Deploy `11803dd`** at 12:3x UTC, after the 534 refold (2513) ended. It carries unit 2b and both review rounds
  (`f3d5d06`, `e6ffd22`, `50b1e67`).
- **Dry plan 2515** (rule `524u2-v5`, binary `11803dd`):
  - **Walked:** 5,677,877 orgs and 49,688,766 mention rows.
  - **CHANGES 28,707:** R0e 1, R0n 2, R1 27,132 and R2 1,572.
  - **HOLDS 553:**
    - consortium_removed 251;
    - family 84;
    - gb_family 13;
    - initialism 34;
    - nhs_check 1;
    - no_fit 170.
  - **Canaries:** 0 unexpected.
- **Judged on its uniform samples** (workflow wf_f9b41c3f-643):
  - **Changes:** of 63 sampled, 39 right, 21 neutral and 0 wrong. Three first-reader "wrong" calls were all refuted:
    - 21558007 and 11206874 checked out as renames in the French register;
    - 22550865 has a majority short form.
  - **Holds and keeps** over-block rather than mis-serve. These holds would have stopped a change judged right:
    - consortium_removed 9 of 12;
    - family 9 of 12;
    - gb_family 10 of 12;
    - initialism 10 of 30;
    - no_fit 1 of 12 (`vivida bkk`, lower-case by design).
  - **bar keeps:** 3 of 12, the known 2x cases (6071658 EPF, 10982286 Hydria) and 5778 (issue 456).
  - **D4 keeps:** 12 of 12 justified. Filed as 548.
- **Canaries 2516:** the 5 decided canaries, 5 ready, 5 renamed, in 1 window. Live on `/v1/organizations`.
- **Capped 2517** (`max_groups` 500): 28,702 ready, 5 already, 0 drifted, 0 gone; 500 renamed; 0 raced. All 30
  `applied_samples` serve their new name live.
- **Full run held: whitespace runs in the display spelling.**
  - 137 planned new names carry a whitespace run. In 36 of them a spelling with no run ties on votes.
  - Org 22609166: `Julius Brune GmbH  Co. KG` (the `&` dropped) ties 2-2 with `Julius Brune GmbH & Co. KG`, and
    the byte-wise tie-break picks it (0x20 < 0x26).
  - S1 keeps an applied head byte for byte, so these spellings would stay forever.
  - None of the 505 applied heads has one: they are the lowest org ids, up to 11,210.
  - Fix, rule `524u2-v6`: group a cluster's fit spellings by their whitespace-collapsed form, summing their votes,
    and serve the collapsed form.
- **Capped 2517 REVERTED (job 2518, 2026-10-15 ~13:1x UTC):** 500 restored, 0 renamed again, 0 stripped, 0 gone.
  - **The dated spot check** (wf_7b6969e8-539) of its 30 `applied_samples`: 8 right, 16 neutral, 5 wrong. Each
    wrong verdict was upheld by a skeptic who read the dated notices on the public API.
    - 2186: a Castilla y León consejería, renamed 2026. Every notice since 2026-08 uses the old head.
    - 3262: `Mind Energy Oy`, renamed from `Energia Myynti Suomi Oy` in 2025/26. Every notice since 2026-03 uses it.
    - 3694: the Murcia consejería, which gained `Portavocía` from 2026-06.
    - 6867: `Askend Estonia OÜ`, renamed from `Wisercat Estonia OÜ`. Every notice since 2025-04 uses it.
    - 10671: TELEG-M d.o.o., which has used its long name since 2026.
  - **The shape:**
    - Low org ids were minted by the live daily feed before the historical backfills, so their first-seen head is
      often the CURRENT name.
    - An unweighted all-history majority then favours the former name, because it has more years of notices.
    - The capped run took the 500 lowest ids, which is exactly this population.
  - **Why the dry-plan judgement (0 of 63 wrong) missed it:** its samples carried vote counts and no dates, so a
    rename back to the former name read as `right` or `neutral` ("the dominant, current-looking registered name").
  - **The 5 canaries (2516) stand.** Each was decided by hand and all are right (an identifier-holder fix, an
    address-junk head, caps, `s.o.`, empty).

## Unit 2b — recency decision (2026-10-15; panel wf_ce13ee62-9e9)

Judge's synthesis of panel wf_ce13ee62-9e9 (three proposals), 2026-10-15. **Verdict: v7 adds one HOLD inside R1, `recent`.** It uses proposal 3's per-procedure window and the 2-procedure floor that proposals 1 and 2 chose. It adds a junk-head exemption that none of the three had, and corrects canary expectations that two of the three left stale. v7 adds no new kind of change: for the same data, its changes are a subset of v6's.

Note (v7 build review, 4f6dc11): the subset claim held for the rule as decided, but v7 also carries ef0ce4b's whitespace-invariant vote key (`head_vote_key` collapses whitespace before `altid_trim`), which both MERGES clusters (`Acme Ltd trading  as Foo` joins `Acme Ltd`) and SPLITS them, so v7 can plan changes v6 kept. A comparison with plans 2515/2517 must not assume v7 ⊆ v6; the dated re-judge's uniform change samples cover the new ones.

### Verified before deciding

**Code (`208c09c`)**
- The order is E1, E2, E3, R0, D4, S1, R1, R2, then bar. R1 runs through `serve`: display spelling, then no_fit, then the bind holds.
- `tender_versions(caused_by_notice_id)` is indexed by `tender_versions_notice`. That index is a `DEFERRED_TENDER_INDEXES` member, dropped for a rebuild's fold. The planner refuses today only when `organization_mentions_org` or `organization_merged_identifiers_org` is missing.
- `tender_versions.published_at` is NOT NULL, in unix seconds. `notices.published_at` is nullable.
- A notice folds into one tender "unless an issue-278 ghost doubles it" (`LINK_NOTICE_TENDERS_SQL`).
- `organization_mentions.notice_id` sits in the row the `organization_mentions_org` walk already fetches. Selecting it costs no extra I/O.

**Dates.** Notice ids are not in time order: 149 is 2026-07-17, while 25879436 is 2026-06-11.

**The 2516 canaries are live.** A GET returns 31558903 as 'The University of Sheffield' and 61 as 'Tribunal administratif de Nice'. All five now elect keep:S1, so the in-code expectations (change:R1/R0n/R0e) would read UNEXPECTED. Proposals 1 and 2 kept those expectations; proposal 3 caught it.

**178 would read UNEXPECTED under all three rules, and no proposal noticed.**
- The head 'Communauté d'agglomération Plaine Vallée' names every buyer procedure from 2024-05 to 2026-07. I checked 12, from 1139025 (2026-07-17) to 29281 (2024-10-28).
- 'ca Plaine Vallée' is the pre-eForms spelling, used up to 2023-12 (7905632, 7766986, 7696877).
- Job 2518 restored the full head. The abbreviation that 2a.3 accepted as "neutral" is the FORMER usage.

**445.** Its 4 'KADEA' mentions all sit in ONE procedure, 1051275 (notices 26970861 on 06-10, 25879436 on 06-11, 149 on 07-17; tender versions also include 27001247 on 07-16). This is from the cached tender JSON. Counting notices instead of procedures would hold this right fix.

**3406.** Both head mentions are in procedure 165172 (27001331 on 07-16, 1333 on 07-17). Which notice carries the second section does not change the outcome.

**Two more 2515 "right" R1 samples dated:**
- 18872111 (H=15): 'Skarb Państwa …' names every procedure from 2025-01 to 2026-09; the head form was last used in 2024-11.
- 9567901 (H=16): 'GOZDARSKE STORITVE, …' is used throughout 2023–2026.
- Both are forward changes and still change under v7. With proposal 1's 8765882 and 22658118, four of the 2515 H≥2 R1 rights are dated, and none is a rename in the wrong direction.

**Simulation** of the three rules on the cached dated items (`/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/judge524v7/cmp.py`):
- All three hold 2186, 3262, 3694, 6867 and 10671. In each, the head owns all of the newest k items.
- All three change 445, 1220, 2762, 3406, 6079, 10603, 11179 and 31558903.
- They differ only on neutral cases. 7066 is held with a floor of 2 and changed with a floor of 3.

### Why this shape

**Proposal 2's changepoint is rejected.** It counts head rows after the elected cluster's LAST use. A single late old-name straggler releases it, such as an award or modification notice for a pre-rename contract, or a buyer's stale supplier record. It also ignores third clusters.

**Proposal 1's fixed newest-3 window is rejected.** It does not scale with the head: a 600-vote head would be decided by 3 procedures. Proposal 3's window grows with k instead.

**Proposal 3's k ≥ 3 floor is lowered to 2.**
- At k = 2, the 2/3 bar means BOTH newest items are head items. That is stricter than 2-of-3. Hypergeometric, with 10 items: 2.2% by chance at k = 2, against 18% at k = 3.
- So the floor of 3 buys little noise reduction, and it lets every 2-procedure rename revert.
- A revert sticks: the old cluster is top again, so S1 keeps it. A hold releases as soon as a newer non-head item arrives.

**Hold, not keep.**
- A hold is "a change the rule elected but does not apply". The would-be spelling stays in the plan, so the re-judge and issue 548 (hold over-blocking) can read it.

**Junk exemption (added).** None of the three proposals guards a junk head that recent notices repeat, for example a buyer template that started appending an address or NIP, or a 'LOT 2 :' prefix. The task forbids keeping junk, so the exemption is added.

### The rule (`524u2-v7`, on v6)

Order in `elect_head_counted`:
1. E1, E2, E3: unchanged.
2. R0: unchanged; it never reads dates.
3. D4: unchanged.
4. S1: unchanged.
5. R1's bar: unchanged (unique top, T·20 ≥ V·11, T ≥ 3, T ≥ 2H). Then, **NEW and only when H ≥ 2**:
   - `dated` is None → `Hold{reason:"undated", rule:R1, new:None}`. This fails closed: a caller without dates can never write an R1 change that recency might have stopped.
   - Otherwise, the window fires and the junk exemption does not apply → `Hold{reason:"recent", rule:R1, new: display_spelling(elected)}`. The `new` value may be None.
   - Otherwise, `serve(R1)` exactly as v6.
6. R2: unchanged. It needs H ≤ 1, so the window can never apply.
7. Keep (bar).

**Input.** `HeadElectOrg.dated: Option<&[DatedVote]>`, with `DatedVote{spelling:&str, tender:i64, at:i64}`. There is one row per mention whose notice has a usable tender version.
- `tender` is the smallest `tender_id` among the notice's `tender_versions` rows. This handles the issue-278 ghost.
- `at` is that row's `published_at`.
- The caller drops rows with `at <= 0` or `at > job start + 86_400` and counts them as undated.
- `elect_head` filters `dated` with the same voter rule it applies to `votes`, and keys each row by `vote_key`.

**The window.**
1. An **item** is a distinct pair (vote key, tender). Its time is the max `at` of its rows. A tender that names two clusters gives one item to each.
2. **k** is the number of the head cluster's items, keyed by `vote_key(head)`.
3. **Rank** every item of the org, newest first:
   - by `at`, descending;
   - on an equal `at`, every non-head item ranks before (newer than) every head item;
   - then by tender, descending;
   - then by key, ascending.
4. **W** is the first k items. **h_W** is the number of head items in W.
5. The window **fires** when k ≥ `HEAD_RECENT_MIN_PROCEDURES` = 2 and 3·h_W ≥ 2·k.
   - At k = 2 that means both newest items; at k = 3, 2 of 3; at k = 6, 4 of 6.
   - Third clusters count against the head (2762's Presidencia and Pleno).

**Junk exemption.** The hold does not apply, and R1 serves as v6, when either holds:
- (a) `(fns.unfit)(head, head)` is Some: control character, mojibake, long, lowercase, or non_name including award_summary;
- (b) the head's vote key has a token containing a digit that the elected cluster's key lacks: an address, a postcode, a NIP/REGON, or 'LOT 2'.

Each release is counted as `recent_released` with the value `unfit` or `digits`.

**Readings.** `HeadElection.recent: Option<RecentReading>` is filled whenever `dated` is Some. It carries:
- k and h_W;
- `head_newest` and `top_newest` (the elected cluster's newest item);
- `newest_is_head` (the top-ranked item is a head item);
- `undated` (voting mentions that are not in `dated`).

**Constants.** `HEAD_HOLD_RECENT = "recent"`, `HEAD_HOLD_UNDATED = "undated"`, `HEAD_RECENT_MIN_PROCEDURES = 2`, `HEAD_ELECTION_EPOCH = "524u2-v7"`.

### Data path

**Page read.**
- A new `org_name_refresh_mentions_sql(n)`: `SELECT organization_id, name, scheme, notice_id FROM organization_mentions WHERE organization_id IN (…)`. The census SQL is left untouched.
- `PageOrg` keeps one (spelling, notice_id) per named mention: about 17k i64 on an average page, at most about 66k for a hub. This adds no I/O.

**Per page, in order:**
1. Elect without dates.
2. Run the E2 probe on the candidates (existing).
3. Read dates for every remaining candidate, change or hold, `hold:undated` included. Changes are dated too, so every planned entry and its ledger row carry dates and the residue (below) is measured.
4. Re-elect the candidates with `Some(dated)` and the probe's `bind_defect`.

**Date read.**
- Take the distinct notice ids of the candidates' voting mentions, `IN_CHUNK` (512) per statement: `SELECT caused_by_notice_id, tender_id, published_at FROM tender_versions WHERE caused_by_notice_id IN (…)`, through `tender_versions_notice`.
- In Rust, keep the row with the smallest `tender_id` per notice.
- No GROUP BY (no sorter), and no `notices.published_at` fallback.

**Refusals.**
- Add `tender_versions_notice` to the index refusal of both the planner and `apply_org_name_refresh`, with the message "absent (a rebuild's fold?)".
- Extend `the_refresh_plans_candidate_reads_seek`: the date SQL must show `SEARCH tender_versions USING INDEX tender_versions_notice`, and the new mention SQL must use `organization_mentions_org`.

**Wet pre-pass (`read_readiness`).**
- The same `notice_id` column, and the same date read for every target (all targets are changes). Then re-elect.
- A target that re-elects `hold:recent` or `hold:undated` counts as drifted, under the existing max(2%, 5) abort.
- The wet run refuses a plan whose epoch is not v7, or whose `by_hold.undated` is not 0.

**Unit 2c.** Inherits all of this through the planner.

**Cost.**
- Plan 2501's candidates: 31.4k orgs, about 2.2M mention rows (R1 with H ≥ 2 alone: 16,348 orgs, 1.85M rows).
- That means at most that many distinct notices, each one index seek plus one row read: about 4.5M B-tree descents or fewer.
- Expected time: about 10–30 s warm, up to a few minutes cold, on top of the ~110 s walk.
- The largest planned org has 18,995 mentions (8087975).
- The wet pre-pass pays the same for up to 28.7k targets, before any write.
- No schema change and no backfill.
- A recent-window ELECTION would need dates for every elected org: about 46M seeks. Rejected.

### Plan output and counters

**Holds.**
- `recent` appears in `by_hold`, `held` and the samples (`hold:recent`, 30 uniform).
- `undated` must read 0. The summary prints `undated N (MUST be 0)`.

**Entry fields.** Each is None when the org was not dated: `head_procs` (k), `head_in_window` (h_W), `head_newest`, `top_newest` (YYYY-MM-DD UTC), `newest_is_head`, `undated_mentions`, `recent_released`.

**Plan totals:**
- `dated_orgs`, `dated_notices`, `dated_version_rows`, `undated_mentions`;
- `recent_released` {unfit, digits}, listing up to 30 ids;
- **residue**: planned changes with `newest_is_head`, counted by rule, with 30 uniform samples under `residue:head_newest`. These are the unprotected class: by dates, an H=1 or single-procedure rename looks the same as 31558903's newest stray.

**Ledger evidence** gains `head_procs`, `head_in_window`, `head_newest` and `top_newest`.

### Canaries (`ORG_NAME_REFRESH_CANARIES`)

**Expectations that change:**
- 31558903, 22713056, 6341366, 760 and 61 become **keep:S1**. All five were applied by 2516.
- 178 becomes **hold:recent**, with serves None.

**Unchanged:**
- 7912592 keep:D4;
- 131 and 99 keep:bar;
- 10 keep:S1;
- 334 change:R1 'Les Alchimistes SAS' (H=1, so the window cannot reach it);
- 1165973 keep:E1;
- 30914328, reported only.

**New, hold:recent:**
- 2186, 3262, 3694, 6867 and 10671 (the five 2517 wrongs, reverted by 2518);
- 1076. Its head names 2024-07, 2025-07 and 2026-07; 'professionnel' appears only in 2018–19. This exercises k=3.

**New, change:R1 (2517's rights, reverted):**
- 445 'KADEA Berlin GmbH';
- 1220 'Miasto Bielsko-Biała Urząd Miejski w Bielsku-Białej';
- 2762 'Diputación Provincial de Valladolid';
- 3406 'Staatliches Hochbauamt Freiburg';
- 6079 'Terideal Segex Energies';
- 10603 'Uždaroji akcinė bendrovė „Visagino energija“';
- 11179 'Mapfre España, Compañía de Seguros y Reaseguros, S. A.';
- 1919 'Ondernemingsrechtbank, Afdeling ANTWERPEN'. It is undated, because it appears only as a review body. If it reads hold:recent, date-check it with one bounded /v1/sql read before counting it as a no-go.

The Sheffield shape has no live canary any more, since it is applied. The unit test carries it.

### Tests

In `head_election.rs` (pure), `tests/refresh_org_names.rs` (planner and wet) and the supervisor:
1. **Rename** (2186 shape): 4 head procedures, all newer than every elected item → hold:recent, with new = the R1 display spelling.
2. **One procedure, three notices** (445: 06-10, 06-11, 07-17), with elected items both older and newer → k=1 → change:R1.
3. **A single newest stray, H=1** (31558903) → change:R1, with `newest_is_head`.
4. **Two head sections of one notice plus its twin in the same tender** (3406) → k=1 → change.
5. **Head and elected at the same instant** (10603) → the tie ranks the elected item first → change.
6. **Floor:**
   - k=2, both newest → hold;
   - k=2 with one newer elected item → change;
   - k=1 → change.
7. **Share:** k=6 with 2 elected items inside W → hold; with 3 → change.
8. **A third cluster counts against the head** (2762).
9. **178:** the abbreviation until 2023-12 and the head since 2024 → hold:recent.
10. **Junk:**
    - an all-lowercase head, k=3 newest → change:R1, released `unfit`;
    - an address/NIP head, k=2 newest → change:R1, released `digits`.
11. **Undated:**
    - None with H≥2 → hold:undated;
    - None with H=1 → change:R1;
    - `Some(&[])` → change:R1.
12. **R0 and R2 never consult dates.**
13. **Planner:**
    - `at` ≤ 0 and future dates are dropped;
    - a notice in two tenders gives the smaller tender;
    - a finished plan has `undated` = 0;
    - it refuses without `tender_versions_notice`;
    - the EXPLAIN shows a seek;
    - entries carry their dates.
14. **Wet:**
    - a v6 plan is refused;
    - a plan with undated > 0 is refused;
    - a target that gained a newer head procedure after the plan reads drifted.
15. **Supervisor:** the canary table (27 rows) and its readings.

### Rollout

1. **Build and gate.** Build by agent, then a 3-lens review. Gate with `ops/check.sh` redirected to a file, and read `GATE-EXIT=`. Push with `HEAD:main HEAD:<handover>`. Deploy in a queue gap, never during a fold.
2. **Dry plan** (full walk) on that binary.
   - Go requires:
     - `undated` = 0;
     - 0 unexpected canaries (1919 as above).
   - Record the walk time and the dated read counts.
   - Stop and re-decide if `hold:recent` is above about 4,000 (a quarter of R1 with H≥2): the window would be reading noise.
3. **Dated re-judge** (workflow: classifier plus skeptic). Each judge GETs the newest 3+ procedures on each side.
   - Lists: change:R1 30, change:R2 30, hold:recent 30, residue:head_newest 30, recent_released up to 30.
   - **Definitions:**
     - a change is *wrong* if it serves a FORMER name (every procedure since some date uses the head), another entity or a worse name;
     - a hold is *harmful* if the head it keeps is junk, a stray or other entity, or defective.
   - **Bars:**
     - at most 1 wrong in each change list;
     - at most 1 harmful in hold:recent;
     - at most 1 wrong in recent_released;
     - residue: 3 or more wrong out of 30 → stop and re-decide (a residue holdout).
4. **Canary wet run** with `org_ids` [445, 1220, 2762, 3406, 6079, 10603, 11179], plus 1919 if its date check passed. Expect 7 renames; GET each one.
5. **Capped wet run**, `max_groups` 500. This is the lowest ids again, the population that failed. Spot-check its 30 `applied_samples` with dates.
   - Go requires 0 former-name reverts (any one → a revert job at once, as 2518) and at most 1 other wrong.
6. **Full wet run**, then the 2b tail:
   - re-run the dry plan (expect about 0 planned, and `hold:recent` about the same as step 2);
   - the `org-head-drift` census;
   - `build-org-match-keys` wet;
   - re-plan the pending altid / nhs-body / r3 dry plans.

Run each plan and its wet run back to back, outside the daily ingest, so a new procedure does not drift the targets. Unit 2c then inherits the dated protocol; its 2,000 cap counts changes only.

### Risks accepted

- **The residue.** H=1 renames and single-procedure renames still revert, because by dates they look the same as 31558903's stray. It is measured by the residue samples, not guessed.
- **Stragglers** above 1/3 of W release the hold, which gives v6 behaviour. A notice with no tender version cannot protect a head; it is counted.
- **Over-holding.** A sub-unit, abbreviation or buyer-habit head that recently dominates is held, and so is a 2-procedure stray (a conflated identifier) until the next non-head procedure arrives. These are missed fixes, never regressions, tracked in 548.
- **Procedure identity** depends on twins folding into one tender; ghosts are handled by MIN(tender_id).
- **The vote key is word-order sensitive**, so it can split a new name (10870's 'w spadku').
- **Review-body names** (61, 1919) reflect how third parties write them.
- **Not fixed: the converse.** An old head with a new name below R1's bar (6071658, EPF Poitou-Charentes → Nouvelle-Aquitaine) stays. A symmetric rename rule is a later unit, report-only first.

### Rejected at decision
- Proposal 2's changepoint veto (head rows strictly after the elected cluster's last use t_E): one late old-name straggler (an award or modification of a pre-rename contract, or a stale buyer record) releases it, and third clusters are ignored. Replaced by proposal 3's k-sized window, which tolerates up to 1/3 non-head items.
- Proposal 1's fixed newest-3-procedure window with h>=2 and 2h>N: it does not scale with the head (a 600-vote head is decided by 3 procedures), 2-of-3 is reachable by chance, and its C(P) newest-UTC-day keys add a timezone tie artefact. Replaced by the window W = the newest k items.
- Proposal 1's notices.published_at fallback, with each versionless notice as its own procedure (-notice_id): a second read path that double-counts twins (proposal 1's own risk 4). Such notices are undated and counted instead.
- Proposal 3's k>=3 floor: at k=2 the 2/3 bar already demands both newest items (2.2% by chance at 10 items, against 18% for 2-of-3 at k=3), and a 2-procedure rename revert sticks under S1 while a hold self-releases. The floor is 2, as proposals 1 and 2 had it.
- Keep(S2) or Keep(current) as the verdict (proposals 2 and 3): a HOLD keeps the would-be spelling in the plan's held list for the dated re-judge and issue 548. Reason `recent`.
- Leaving the 2516 canaries' expectations unchanged (proposals 1 and 2): 31558903, 22713056, 6341366, 760 and 61 are live under their new names (GET-confirmed for 31558903 and 61) and elect keep:S1.
- Leaving canary 178 at change:R1 'CA Plaine Vallée' (all three proposals): every buyer procedure from 2024-05 to 2026-07 uses the full head, and 'ca Plaine Vallée' ends in 2023-12, so v7 holds it. Its expectation becomes hold:recent.
- Protecting any recently repeated head without a junk test (all three proposals): an unfit head (control, mojibake, long, lowercase, non_name) or one with a digit token absent from the elected key (address, NIP/REGON, 'LOT 2') is released to R1 and counted.
- Dating only the hold:undated candidates (proposals 1 and 3): every candidate is dated (+~0.4M rows), so every planned entry and ledger row carries dates and the H=1 or single-procedure residue is sampled.
- A recent-window or recency-weighted ELECTION: it reverts 2186 at 12 months, loses 445, 3406 and 31558903 at 6 months, re-scores the 502k S1 heads weekly, and needs dates for every elected org (~46M seeks).
- Counting notices or mentions instead of procedures: it holds 445 (three notices of tender 1051275, TED plus eForms-DE twins) and 3406, and lets lot or role multiplicity (18872111's 11 Lot-AddInfo sections) weigh in.
- A symmetric R3 change rule ('renamed away from the head', 6071658) inside v7: it is a new change kind that needs its own judged samples. Later unit, report-only first.
- Using notice id, org id, created_at or ingested_at as a time proxy: the live feed's notice ids 64, 149 and 1333 are dated 2026-07, below the 2023–26 backfill ids.

## Rule v7 on prod: the dated re-judge (2026-10-15, plan 2523, wf_b6d9f416-8dd)

**Plan 2523** (binary `d23ce5a`, epoch `524u2-v7`):
- CHANGES 28,338: R0n 1, R1 26,765, R2 1,572.
- HOLDS 917, of which `recent` 370.
- Undated 0.
- Junk releases 10: 8 for digits, 2 for unfit.
- Residue (planned changes whose org's newest dated procedure is the head's): R1 795, R2 8.
- Canaries: 26 of 27 as expected. 10603 reads `hold:recent` where `change:R1` was expected. Its head form `… (PV)` has 2
  procedures, the newest 2026-09-24, while the elected spelling's newest is 2025-09-15. The decision's dates for it were
  incomplete. The hold is the rule working as specified: at worst it misses a role suffix, and it is no regression.
  The expectation is corrected with v8.

**The re-judge.** Each verdict read dated notices on the public API. Each `wrong` went to a skeptic.

| list | right | neutral | wrong (after verify) | bar |
| --- | --- | --- | --- | --- |
| change:R1 (30) | 24 | 6 | 0 | ≤ 1: pass |
| change:R2 (30) | 24 | 6 | 0 | ≤ 1: pass |
| hold:recent (30) | 13 | 16 | 1 harmful | ≤ 1: pass, at the bar |
| recent_released (10) | 6 | 2 | **2** | ≤ 1: **FAIL** |
| residue:head_newest (30) | 6 | 15 | **9** | ≥ 3 means stop: **FAIL** |

**Wrong releases:**
- **1846** (`digits`): the digit is in the body's own acronym, `CA2BM`, and every recent notice prints the head.
- **3909** (`unfit`, lowercase): `act legal s.r.o., advokátní kancelář` is the registered current name (ARES; renamed
  2026-03-01 from `act Řanda Havel Legal …`). The release reverted it to the former name.

**Residue wrongs**, mostly not former-name reverts:
- an organ served for the entity: 5831168 Rectorado, 3740262 Junta de Gobierno, 4911107 Alcaldía;
- a majority made of one notice's lot repeats: 12463423 `Consultorís`, 7 votes from one 2022 notice;
- misspellings: 3835747 `Vôľa`, 12268 `Stolecznego`;
- a placeholder-identifier pile-up (22183903, `08AXXXXX`) and another entity sharing a code (22754404);
- one estate-succession rename (10870, `w spadku`).

The residue holds the cases where the head is printed newest, and there the head is more often the right name.

**Harmful hold:** 10662302 keeps a `Sp. zo.o.` typo. This is accepted, and filed with 548's over-hold classes.

## Rule v8 — decided (2026-10-15, owner, from the re-judge above)

1. **Residue holdout.** After R1/R2 elect a change and before `serve`, when the dated reading has `newest_is_head`
   true (the org's single newest dated item is a head item) and the junk exemption does not apply, the verdict is
   `Hold{reason: "residue", rule, new: display}`.
   - R0 is exempt: a defective head always changes.
   - Undated orgs stay governed by `hold:undated` (H ≥ 2) or change as before (H ≤ 1, `Some(&[])`).
   - Cost: the plan's ~803 residue changes become holds. By the sample that is about 6/30 right fixes lost against
     about 9/30 wrong renames avoided.
   - The Sheffield shape (an H=1 stray printed newest) is held too. That is a missed fix, never a regression. It is
     listed in 548.
2. **Junk exemption, narrowed.**
   - (b) counts only tokens made entirely of decimal digits (an address number, postcode, NIP, `LOT 2`'s `2`), so
     `CA2BM` no longer counts.
   - (a) drops the `lowercase` unfit class, so a deliberately lower-case brand is protected. Control, mojibake, long
     and non_name still release.
3. **Epoch `524u2-v8`.** Canary 10603 expects `hold:recent`. Add canaries:
   - 3909: `hold:recent` (the lowercase brand);
   - 1846: `hold:recent` (the digit acronym);
   - one residue case, e.g. 3740262: `hold:residue`.
4. **Re-judge after v8**, dated as before:
   - change:R1 and change:R2 (30 each, fresh samples);
   - `hold:residue` (30): harmful ≤ 2;
   - `recent_released` (all): wrong ≤ 1.

   Then the canaries, the capped 500 run with a dated spot check, and the full run.
5. **Follow-up, not in v8:** count votes per procedure rather than per mention, so lot repeats cannot make a majority
   (`Consultorís`). Filed under 548.

## Rule v8 on prod: the dated re-judge (2026-10-15, plan 2526, wf_62b83a3d-26b)

**Plan 2526** (binary `1d95c22`, epoch `524u2-v8`, walk 143 s):
- CHANGES 27,559: R0n 1, R1 25,994, R2 1,564.
- HOLDS 1,696, of which `recent` 374 and `residue` 794 (R1 795 → held; R2 8).
- Undated 0; junk releases 27 (26 `digits`, 1 `unfit`).
- Canaries: 0 unexpected (30 rows). Its own dates for the six `change:R1` canaries are in the runbook's step 2.
  The five that share `head_newest` 2026-07-17 (445, 1919, 2762, 3406, 6079) are genuine: low org ids were
  minted from the first package ingested, OJ 2026-07-17 (notice 1333 is 00495778-2026), so their first-seen heads
  carry that date.

**The re-judge** — fresh samples (the first 30 of each list not in plan 2523's), each verdict from dated notices on
the public API, each `wrong` re-read by a skeptic told to refute it:

| list | right | neutral | wrong (skeptic-confirmed) | bar |
| --- | --- | --- | --- | --- |
| change:R1 (30) | 20 | 8 | **2**: 8069375, 5350675 | ≤ 1: **FAIL** |
| change:R2 (30) | 22 | 6 | **2**: 4123715, 16508165 | ≤ 1: **FAIL** |
| hold:residue (30) + v7 releases v8 holds (4) | 22 | 10 | 2 harmful: 19427430, 15829801 | ≤ 2 together: pass, at the bar |
| released (27, every one) | 19 | 3 | **5**: 9806, 9814, 9833, 4295185, 15855816 | ≤ 1: **FAIL** |

- **8069375** R1: `Prometni institut Ljubljana d.o.o.` → the long registered name plus `…, Ljubljana, Kolodvorska 11`,
  a spelling only older notices print. The served name ADDS a digits-only token (an address number).
- **5350675** R1: `Ośrodek Pomocy Społecznej w Kluczborku` → `Ośrodek Pomocy Społecznej`, a generic institution type
  (15 organizations carry it as their head).
- **4123715** R2: `Rectorat de l'académie de Paris` → `SIEC — Maison des examens`, a sub-service for the legal unit.
- **16508165** R2: `Hurt-Papier Ryszard Cebula Sp. j.` → `HURT PAPIER RYSZARD CEBULA SPÓŁKA JAWNA`: one outdated
  sp.j. spelling for another (a sp.k. since 2023). Polish spelled-out forms are not legal-form tokens in
  `head_vote_key`, so the two never share a cluster.
- **Released by `digits`**: 9806 and 9814 (a Croatian obrt's owner-and-address head → the bare activity phrase),
  9833 (back to the 2014 wording), 4295185 (`SMD 3` → `Smd3`: the name's own `3` read as an address digit, the
  elected key holding it inside `smd3`), 15855816 (`Materská škola, Družstevná 22, Šaľa` → `Materská škola`: in
  Šaľa the address IS the school's name). Over both plans the `digits` releases read 24 right, 4 neutral, 6 wrong.
- **Residue harms** (accepted at the bar): 19427430, a contract-modification notice reprinting the pre-rename name;
  15829801, one buyer's word-order misspelling printed newest.

Pooled over plans 2523 and 2526: R1 2 of 60 wrong, R2 2 of 60. R1's two are structural classes a rule can see (an
added digits token; a generic name); R2's are semantic singletons that neither its vote statistics nor any token test
measured so far separates from its 46 rights. The labelled set (both re-judges, final verdicts) and the measurements
are kept with the evidence pack for the v9 decision.

## Rule v9 — decided (2026-10-15, owner; panel wf_5cca11b1-622)

Three designers (precision-first, recall-first, root-cause) each proposed and MEASURED a v9 on plan 2526's full plan
JSON and the labelled set (both dated re-judges, final verdicts); two adversarial judges re-ran their numbers, read random
plan entries each predicate touches, and ranked root-cause first, precision second. This adopts the judges' synthesis.
Every piece is a pure function of (head, display, rule, keys), so the planner and the wet pre-pass agree by
construction; **no DB read, index or fail-closed counter is added.** Epoch `524u2-v9`. Tokens: `T(s)` =
`crosswalk::fold_diacritics(s)` split on every char that is not `char::is_alphanumeric`, empties dropped.

**Serve holds** — one injected `crosswalk::head_serve_hold(old, new, r2) -> Option<&'static str>`, asked inside
`serve()` for R1 and R2 only (R0 exempt) after `fns.hold` and `fns.r2_hold`; `recent`, `residue` and `undated` keep
precedence. First match wins:
1. **`review_body`.** `J(s) = " " + T(s).join(" ") + " "`; review_body(s) when J(s) contains `" "+p+" "` for some p in
   REVIEW_PHRASES (folded): vergabekammer, vergabesenat, nachprufungsstelle, nachprufungsbehorde, bundesvergabeamt,
   landesverwaltungsgericht, verwaltungsgericht, oberlandesgericht, krajowa izba odwolawcza, krajowej izby
   odwolawczej, tribunal administratif, cour administrative d appel, conseil d etat, reglement amiable, tribunal
   judiciaire, tribunal de grande instance, tar, t a r, tribunale amministrativo, consiglio di stato, recursos
   contractuales, recursos contractuals, tribunal catala de contractes, tribunal administrativo central, tribunal
   administrativo de contratos, tribunal administrativo de recursos, organo administrativo de recursos, tribunal
   superior de justicia, juzgado de lo contencioso, raad van state, rechtbank, tribunal d affaires, commercial court,
   solutionare a contestatiilor, solutionarea contestatiilor, klagenaevnet for udbud, markkinaoikeus,
   forvaltningsratten, kammarratten, klagenemnda for offentlige anskaffelser, kozbeszerzesi dontobizottsag, urad pre
   verejne obstaravanie, urad pro ochranu hospodarske souteze, kontrolu postupaka javne nabave, drzavna revizijska
   komisija, zalbi po javni nabavki, комисия за защита на конкуренцията, αρχη εξετασης προδικαστικων προσφυγων, public
   procurement review, high court, procurement review board. Hold when review_body(display) and NOT review_like(head):
   review_like(head) = review_body(head), or the joined head contains a stem of {kammer, tribunal, trybunal, odwola,
   izba, rechtbank, court, recurs, contencios, nachprufung, gericht, conseil d etat, amministrativ, administratif,
   adminsitratif, amminitrativ, reglement amiable, ccira, ccra, kio, uzp, zamowien publicznych, contestatii,
   klagenaevn, markkinaoikeus}, or starts with `ta `, `tar ` or `vk ` — so a review body's own self-rename still changes
   (791 `Urząd Zamówień Publicznych Departament Odwołań` → `Krajowa Izba Odwoławcza`, V 28,725; 100 `TA de Toulouse`;
   395 `Tribunal Adminsitratif d'Amiens`). Plan 2526: ~104 holds; 16 of 16 read another-entity (`Universitätsklinikum
   Freiburg` → `Vergabekammer BW`, a prison → KIO).
2. **`organ`.** organ_lead(s) when `T(s).join(" ")` equals or starts with (`p + " "`) some p in ORGAN_PHRASES (folded):
   alcaldia, alcalde, alcaldesa, junta de gobierno, junta de govern, pleno, ple de, presidencia, presidente, rectorado,
   rectorat de la universitat, vicerrectorado, consejo de administracion, consell d administracio, consejo de gobierno,
   consell de govern, consejo rector, consell rector, gerencia, direccion general, direccion gerencia, direccion,
   consejeria delegada, consejero delegado, concejalia, junta de contratacion, mesa de contratacion, mesa del, organo de
   contratacion, comite ejecutivo, comision ejecutiva, secretaria general, magistrat der, der magistrat. Hold when
   organ_lead(display) and NOT organ_lead(head); the reverse (canary 2762, organ head → entity) is never held. Plan
   2526: 460 (R1 445, R2 15; ES 450); reads ~32 of 37 an organ or sub-unit for the entity. Known false-positive pocket
   accepted: Castilla y León `Gerencia de Asistencia Sanitaria de X`.
3. **`code_added`.** A = the ASCII-digit-only tokens of T(display) that do not occur in the head's alphanumeric stream
   (lower-cased, folded, every non-alphanumeric removed, so `Pragma9` covers `9`). Hold when A ≠ ∅, some A token is in a
   code position (inside `(…)`/`[…]`, joined by `_`, the last whitespace token, or a leading list number before `.`/`)`),
   and `head_vote_key(display)` minus the A tokens equals `head_vote_key(head)` token for token. Plan 2526: 81 (mostly
   French department codes, `(976)`); 0 right of 16 read.
4. **`address_added`.** A as in 3. Hold when some A token lies in a `,`/`;` segment after the display's first, unless the
   folded display contains a school stem {skol, jidel, jedal, gymnazi, internat, druzin} (CZ/SK schools carry their
   seat by law: labelled rights 3377646, 4594482). Plan 2526: ~85; holds 8069375.
5. **`caps_variant`.** All four: store's `all_caps(display)`; the head has an upper-case letter and a lower-case letter
   that has a one-letter capital; C(head) = C(display) as non-empty multisets, C(s) = tokens of T(s) with ≥ 2 chars not
   in FORM_WORDS (every `family_token` key, `R2_LEGAL_FORM_TOKENS`, and: sp spolka z ograniczona odpowiedzialnoscia
   jawna komandytowa akcyjna cywilna partnerska komandytowo spzoo spj spk ska sa sl slu sau scarl scrl srl sarl sas sasu
   eurl spa snc gmbh mbh ag kg ohg ug haftungsbeschrankt co ev ek eg ltd limited plc llp lp cic company bv nv vof cv ab
   oy oyj abp as asa aps doo dd sro spol kft zrt nyrt bt korlatolt felelossegu tarsasag uab vsia sia societe anonyme
   responsabilite limitee par actions simplifiee sociedad anonima limitada unipersonal societa per azioni
   responsabilita lda unipessoal). An ALL-CAPS respelling of the same words over a mixed-case head. Plan 2526: ~290;
   holds 16508165; both judges read ~0 right in it (cosmetic).

**The digits release** (`recent_release`, both the window and the residue paths; the `unfit` arm unchanged):
- **glue:** a digits-only head-key token t does not count when the elected key contains its neighbour glued to it
  (prev+t or t+next) — `SMD 3` vs `smd3` is the name's own digit (holds 4295185, wrong; 6693098, 11087087 neutral);
- **junk-only:** a digits release also needs |lost| ≤ 1, lost = the head-key tokens of ≥ 3 chars with no ASCII digit
  absent from the elected key's tokens (the elected name is the head minus junk; one lost token lets an expanded acronym
  through: `SDIS 49` → `Service départemental…`). Holds 1341, 9793, 9796, 9806, 9807, 9810, 9814, 9833, 9846, 9852,
  9853, 15855816.
- A withheld release returns to its v8 hold reason and is listed under a new counter `release_withheld` (ids). Plan
  2526: releases 11 (labelled 10 right, 1 neutral), withheld 15 (labelled 5 wrong, 2 neutral, 8 right — the 8 forgone
  right fixes are this decision's accepted cost, recorded, not pooled into any hold bar).

**Per-class wet-run filters** (pre-built, tested in v9): wet-run args `exclude_rules` (e.g. `["R2"]`) and
`exclude_released` (e.g. `["digits"]`) drop targets before the pre-pass — no re-plan, no parity question. Pre-committed
use: if change:R2 fails the re-judge, every wet run passes `exclude_rules: ["R2"]` (R2 goes to 548, no refit inside 524
unit 2); if `released` fails, `exclude_released: ["digits"]`. Only a change:R1 failure stops the run.

**Rejected:** a `generic` hold (the only DB-read verdict; sized on 13 sample hits; the corpus name count measures
duplicate rows, not genericness — `Urtica`, `Région Occitanie`); `r2_disjoint` (~65–70 % right); a broad `digits_added`
(~50 % right beyond code/address; holds schools); dropping every digits release (forgoes 10 more rights for no extra
wrong; kept as the `exclude_released` fallback); folding Polish spelled-out forms into the vote key (cancels 596 mostly
good cosmetic changes); the root-cause form-spelling and structural-organ rules (superseded by caps_variant and the
lexicon). The designers' own expected rates (fitted on the same 120 items) are not used as evidence.

**Measured on plan 2526** (judges' Python ports): ~930 serve holds (≈3.4 % of R1, ≈2.6 % of R2) + 15 withheld releases.
Labelled change lists: holds 8069375 (W) and 16508165 (W) and 2 neutrals; 0 labelled rights. Residual known wrongs:
5350675 (a generic prefix drop) and 4123715 (a semantic R2 singleton). Expected fresh rates (judges): change:R1 1.5–2.5 %
(P(pass) ≈ 0.6–0.9), change:R2 2–3 % (likely to trip; the R2 filter then fires).

**Protocol after the build:**
1. Gate (`ops/check.sh`, read GATE-EXIT); unit tests pin 22028982 review_body; 791, 100 and 1919 not held; 4065575 organ;
   2762 not held; 17116440 code_added; 8069375 address_added; 3377646 and 4594482 not held; 16508165 caps_variant;
   URTICA (caps → mixed) not held; 4295185 glue hold; 9814 junk-only hold; 3042 still released; 14971333 still released;
   the wet-run filters; `head_vote_key`, `family_token` and `NAME_KEY_EPOCH` unchanged.
2. Deploy in a queue gap; re-plan (full walk). Go: undated 0; 0 unexpected canaries (new: hold:review_body 22028982,
   hold:organ 4065575, hold:code_added 17116440, hold:address_added 8069375, hold:caps_variant 16508165; change:R1 791;
   4295185 and 9814 back under their v8 reasons; 3042 released; the existing 30 keep theirs); each new `by_hold` within
   ±10 % (floor ±10) of plan 2526's deterministic counts above, a larger gap being a port bug to reconcile first.
3. Fresh dated re-judge, a skeptic on every wrong/harmful. The uniform sample is a fixed hash, so the plan SALTS its
   sample key with the epoch (or the re-judge skips every org in the labelled set and every id the panel read):
   change:R1 30 (≤ 1 wrong; fail → stop and re-decide); change:R2 30 (≤ 1; fail → `exclude_rules: ["R2"]`); released,
   every entry (≤ 1; fail → `exclude_released: ["digits"]`); the new serve holds, 30 stratified (organ 12, review_body
   6, code_added 4, address_added 4, caps_variant 4) as a COST REPORT, not a gate — a reason reading more right than
   wrong goes to 548 for narrowing, never released inside v9. hold:recent and hold:residue are unchanged and not
   re-judged.
4. Canaries wet run, capped 500 with a dated spot check (0 former-name reverts, ≤ 1 other wrong), full — each with the
   filters the re-judge fired.
