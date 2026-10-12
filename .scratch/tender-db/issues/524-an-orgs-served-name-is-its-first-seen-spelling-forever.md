# 524 — an org's served name is its first-seen spelling forever, so merges and drops leave a wrong or junk head

Status: ready-for-agent — triaged 2026-10-12; claims verified on prod (31558903 'Sheffield Hallam University' over 16 of 17 Sheffield mentions; 22713056 's.o.' over 28 of 28 'Vergabekammer bei der Bezirksregierung'). NEXT: unit 1, an in-app dry census (`org-head-drift`, or an arm on 456's planned unit-1 walk, built once for both). For orgs with ≥2 mentions it reports: - orgs whose head key (`name_norm` and `altid_name_key`) matches no mention; - orgs where the head key is a count-minority; - orgs whose head is `not_a_name`. Each is split by identifier NULL vs not, merge survivor vs not, and casing-only vs real drift, with 30 samples per bucket. Then the unit-2 decision (an ADR-0013 D4 amendment).
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
