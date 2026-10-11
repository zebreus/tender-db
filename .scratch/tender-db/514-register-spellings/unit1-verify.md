# Issue 514 unit 1: skeptic's verification of the census (2026-10-11)

**Verdict: sound-with-fixes.** All 35 claimed splits re-derive from the raw rows, and none is a false split. Mention
counts reconcile, and the read discipline was clean. The fixes below are needed before unit 2 relies on the census:

1. One name verdict is wrong (10007157).
2. One in-scheme split was missed (a leading-zero CHC spelling).
3. The level-B figures include level A, which the summary doesn't say.
4. Several scannable legacy and bare ranges were never read, and they hold more level-B groups. Among them, SC's
   level-B result "none" is wrong.

I made 15 reads of my own, all bounded, index-served and sequential. None returned 408 and none was retried. The
responses are in `verify-raw/`: `vNN-*.out`, with `journal.jsonl` holding the SQL, seconds, rows and status of each
call. `verify-raw/rederive.py` is the independent re-derivation.

## Findings, most material first

### F1. 10007157 is the same entity, not a name-unsafe pair (correction)

The census called this split "differ". It said the name gate must refuse the pair, and that folding it would put
"Hallam-named mentions on Sheffield's identity". It read only the organisations' head names. Read 2 groups the
mentions by name:

| org | stored | mentions by name |
|---|---|---|
| 31558903 | `GBUKPRN10007157` (head name "Sheffield Hallam University") | 13 "UNIVERSITY OF SHEFFIELD", 1 "The University of Sheffield", 1 "University of Sheffield", **1 "Sheffield Hallam University"** |
| 31630773 | `GBUKPRNUKPRN10007157` | 1 "UNIVERSITY OF SHEFFIELD" |

So 15 of the bare org's 16 mentions name the University of Sheffield, which is what UKPRN 10007157 is. A single
stray Hallam mention won the head name. Both orgs are the University of Sheffield.

- The defect is a mis-chosen head name, not a split that is unsafe to fold.
- A gate that reads head names only would wrongly refuse this true pair. A gate that reads "some name of each" (the
  altid-style `altid_keys_agree`) would pass it, and correctly.
- The census's unit-2 pointer says the gate "holds back exactly those two splits" (10007157 and 1179472). That is
  wrong for 10007157. Of the 35 splits, 34 are same-entity and 1179472 stays partial.

Mention names (read 2) confirm the other 33 "same" verdicts: no split carries a second entity's name.
- 30915632 (10005790, Hallam) carries one "University of Sheffield" mention under its PPON and one "Access UK Ltd"
  mention. That is noise, not a split.
- 1179472's "Living Well UK" stands on a single mention, so "partial" stays.

### F2. A missed in-scheme split: the leading-zero CHC spelling

`GB-CHC-0518801` (org 31567375) and `GB-CHC-518801` (org 31567383) are both "Queenscourt Hospice", one mention
each. The census rule `chc_base` requires a leading `[2-9]` or `1`, so `0518801` was filed as "nonconforming
digits" and never grouped. 469's `REGISTRIES` CHC shape refuses it too.

This is a spelling class the census didn't consider: a zero-padded charity number. No other leading-zero value
stands in the five in-scheme ranges. `GBCHC075028` (YMCA East Surrey) is not one, because `75028` is too short to be
a charity number; it is more likely `1075028` with its `1` dropped, so it stays a near-miss.

Corrected CHC count: **15 splits, 31 orgs, 63 mentions**. Corrected total: **36 splits, 73 orgs, 852 mentions**.

### F3. The level-B figures include level A (reporting)

The summary says level B is "Same number under other prefixes (reported separately, not counted above)" and that
MPR is "about 8× the in-scheme mass". In fact, `splits_B` groups every target-scheme row, in-scheme rows included,
so each level-B figure contains the level-A splits:
- All 15 MPR level-A keys appear in the 91 MPR level-B groups.
- 14 of the 20 CHC level-B groups are the level-A splits.
- 5 of the 6 UKPRN groups are level A.

What the cross-prefix view really adds (computed from `splits.json`):

| Scheme | Level B as reported (A included) | Groups new or enlarged by cross-prefix | Mentions added beyond A |
|---|---|---|---|
| MPR | 91 / 216 / 3,730 | 76 new, plus enlarged A groups | 3,238 (6.6× A, not 8×) |
| CHC | 20 / 42 / 88 | 6 new, plus 216250 enlarged to three orgs | 27 |
| UKPRN | 6 / 12 / 300 | 1 (10007455) | 5 |
| NIC | 2 / 4 / 43 | 1 (100166) | 41 |

The census's own md table labels a column "Rows added", but its group, org and mention columns are inclusive too.

### F4. Unscanned ranges hold more level-B groups, and SC level B is not "none" (miss)

The census caveat says legacy bare literals were scanned only under `IP`, `RS`, `UKPRN`, `CHARITY` and `NIC`. It
presents the rest as unscannable: bare digits, and `SC0`, which collides with Companies House numbers. Several of the
rest are cheap label prefixes, and one is resolvable by name:

- **`REG…` (read 4, 50 rows).** None of these strings is in `LABEL_PREFIXES`, so they stand as published.
  - **MPR +9 groups, 18 orgs, 54 mentions (read 11).** Each pairs `REGISTEREDSOCIETYREGISTRATIONNUMBERnnnnnR…HCA…`
    with a standing bare `IPnnnnnR`, `IP0nnnnn` or `IP0nnnnnR`:
    - 16836R Mercian
    - 18652R Circle 33
    - 26622R Wherry
    - 27076R Russet
    - 28100R South Anglia
    - 30161R Roddons
    - 30312R Mole Valley
    - 30843R Merton Priory
    - 31218R Aspire, whose names only overlap: "Aspire Housing Ltd" against "Aspire Housing Group"

    All the others are same-name.
  - **CHC +4 groups, 8 orgs, 15 mentions:**
    - 312278 Coram: `REGISTEREDCHARITY312278` with `GBCHC312278`
    - 1065846 RISE
    - 207043 Royal Society, 4 + 5 mentions
    - 1124127 Catch22: two legacy orgs, `REGISTEREDCHARITY1124127ANDACOMPANY…6577534` and
      `REGISTEREDCHARITYNUMBER1124127`, with a third truncated `REGISTEREDCHARITYNUMBER1124` as a near-miss
- **`SC0…` bare (read 6, 79 rows).** Most are Companies House Scottish numbers (Aviva `SC002116`), but three are
  name-confirmed twins of in-scheme OSCR holders. **SC level B: 3 groups, 6 orgs, 39 mentions.**
  - SC004401 University of Glasgow, 1 + 2
  - SC005336 University of Edinburgh, 14 + 15 (the bare org also carries `GB-COH-SC005336`)
  - SC005619 Viewpoint HA, 4 + 3
- **`SCO…` (read 12).** These are OSCR numbers spelled with a letter O for the zero, a lookalike class the census
  checked only for MPR (`IPI…`, `…O…`).
  - `SCO13683` "University of Aberdeen (via Centre for Energy Law)" is `SC013683`, held in-scheme by `GBSCSC013683`
    University of Aberdeen. That makes a lookalike pair.
  - Five more `SCO…` values have no twin.
  - `SCOTTISHCHARITYNUMBERSC029917` (Loreburn HA) is a label form with no twin.
- **`SOC…` (read 13).** `SOCNO28985R` (A2Dominion) joins the 28985R group, making 3 orgs.
- **`FCA…` (read 15).** `FCA30173R` Local Food Links has no twin.

These reads found nothing new:
- `NI1…` bare (read 3). It holds the issue's `NI 100770` shape: 15 rows, mostly NI company numbers.
  `NI105955` Co-operation Ireland has no `GBNIC` twin.
- `CHC…` bare (read 5): 0 rows.
- `GBC…`..`GBCHC` (read 7): 0 rows.
- The token-variant sweeps `GBU…` (522), `GBM…` (135) and `GBNI…`..`GBO` (19, read 10). Each returned exactly the
  census's in-scheme rows, so no other spelling of the scheme token stands, and those three ranges haven't moved
  since the census.
- `GBCOHSC0…` (read 14): 47 rows, no OSCR twin.

Corrected level B is a lower bound, inclusive of A, as the census counts it:

| Scheme | Groups | Orgs |
|---|---|---|
| MPR | 100 | 235 |
| CHC | 25 | 52 |
| SC | 3, plus 1 lookalike pair | 6 |
| UKPRN | unchanged | |
| NIC | unchanged | |

- MPR mentions are at least 3,784; the SOCNO org's mentions were not read.
- CHC has 105 mentions; its 25 groups include F2's pair.
- SC has 39 mentions.

### F5. Gaps in the near-miss list and lookalike traps

- **`GBCHCRSA007814`, Beyond Housing, 51 mentions** (org 30917450, literal `GB-CHC-RSA007814`), is missing from the
  near-miss list. Only the 1-mention `GBMPRRSA007814` is listed. The RSA spelling therefore carries 52 mentions over
  two orgs, more than the three `7814` holders combined (`GBMPR7814` 6, `GBMPRRS007814` 2, `GBCOHRS007814` 8).
- **`IPI6637R`, Metropolitan HT, 13 mentions** is listed as a lookalike of Metropolitan, but its I→1 reading is
  `16637R`, and Metropolitan is `16337R`. This is a second Hastoe-type trap: a letter fold applied by rule alone keys
  the org onto a number that is not the named body's.
- `RS0076173`, Castles & Coasts (bare, an extra digit beside `RS007617`), and `GBCHC277793R`, Greenwich Leisure (an
  extra 7), are unlisted.
- **`GBCHCX1089` (34 mentions) against `GBCHCXC1089` (1), both University of Sheffield**, are HMRC exempt-charity
  references published under GB-CHC. They are not CHC register numbers, so this is not a 514 split. But it is a
  same-entity split under the GB-CHC token that the census filed silently among 28 "nonconforming". So are
  `XN85208` DMU (57), `XR6326` New College Durham (15), `XN92247` Middlesex (10) and others.

### F6. Rule hazards that create no false split today

- **The IP+R rule is not typo-safe.**
  - The census counts `GBMPRIP6243R` (Hastoe, 30 mentions) as "with a candidate number", namely `6243R`. Its class
    table counts it among the 19 IP+R values.
  - No `6243R` holder stands, so no false split exists. But a unit-2 fold keyed by shape alone would mint `6243R`
    for Hastoe.
  - The fold needs a standing, name-agreeing twin, not shape alone.
- **`GBNIC1014860` (Triangle HA) is routed to CHC as `1014860`.** It is more likely a CCNI number with a stray digit.
  No CHC holder exists, so it forms no group, but routing 7-digit `1NNNNNN` values under GB-NIC to CHC is unsafe as a
  rule.
- **4-digit values under GB-CHC are routed to MPR bare.** `GBCHC7483`, "Anchor Hanover", is a transposition of 7843.
  No wrong holder stands, so no false group forms.
- **The checks the brief asked for all come back clean:**
  - No R-suffixed number is mapped to bare. An `IP007522` shape stays nonconforming.
  - No linked suffix is folded into its parent; both `-2` literals stay `base-N` with no parent standing.
  - No NIC value collides with an unrelated bare 6-digit one. Bare 6-digit values are never grouped, and read 3
    adds no `NI1…` twin.
  - The bare-vs-R overlap is empty over every census row and every row I read.

## (1) Re-derivation: all 35 splits, independent rules (`verify-raw/rederive.py`)

`rederive.py` keys each in-scheme row with a deliberately different, looser rule:
- MPR: the trailing number with its optional `R`/`RS`, and `RS`+6 digits taken to bare.
- UKPRN: any `1`+7-digit run.
- CHC: any 6-7-digit run with leading zeros stripped.
- NIC: any `1`+5-digit run.

It reproduces all 35 groups member for member. The differences are:
- It also finds 518801 (F2).
- It also folds `GBMPRRSA007814` into 7814. That is intended, since the census left RSA unmapped as a near-miss.

Every member's literal (from `raw/`, re-read live in read 2) normalises to the stored identifier.

| Scheme | Re-derived | Shapes confirmed |
|---|---|---|
| MPR | 15/15 | 10 `NNNNNR`↔`IPNNNNNR`; 4 bare↔`RS00NNNN` (7104, 7528, 7814, 8376); 1 `REGNO30938R` |
| UKPRN | 5/5 | 4 `UKPRN[:]`-labelled, 1 `GB-UKPRN` doubled (10004116) |
| CHC | 14/14 | 9 bare↔label, 2 label↔label, 3 bare↔compound |
| NIC | 1/1 | `CHARITYCOMMISSIONNONIC102575` ↔ `NIC102575` |
| SC | 0 | — |

- Read 1 re-read all 71 split orgs (plus 6 near-miss orgs). Every one still stands with the same identifier, the
  same head name and `provisional = 0`.
- Read 2's per-org mention totals equal the census's for every split org, so the drain has not moved any of them.
- Offline, the census's `mcount` and `mraw` sums agree for every org: 0 mismatches.
- Its per-scheme mention totals reproduce: 1,922 / 296 / 4,188 / 3,197 / 64.
- Every scanned org's literals were read: no in-scheme org lacks a mention row.

## (4) Read discipline

`raw/journal.jsonl` holds 29 entries, one per call number, with no duplicates and no retries.
- No call returned 408. Call 20 returned HTTP 400, because `organization_merged_identifiers` is not on the
  allow-list; it was a PK-prefix seek.
- Every call took 1.57-2.53 s, and no response was truncated.
- The shapes:
  - Calls 1-15 are `organizations` ranges on `(country='GB', identifier_kind, identifier)`. The index
    `organizations_identity` is confirmed at canonical.rs:12835.
  - Calls 16-19 and 21-29 are `organization_mentions WHERE organization_id IN (≤158 ids)`, grouped, on
    `organization_mentions_org` (canonical.rs:455).
- **Sound.** The only mild stretch is the `GROUP BY … raw_identifier` aggregate, which is still bounded by the id
  list.

My own reads: 15 sequential calls, each one bare SELECT. All took 1.57-1.79 s, none returned 408 or was truncated,
and none was retried.
- 12 are `organizations` ranges with `country='GB'`.
- 1 is `organizations WHERE id IN (77)`.
- 2 are `organization_mentions WHERE organization_id IN (77 / 33)`, grouped.

## What this verification did not cover

- **Countries other than GB.** The brief fixes the range shape to `country='GB'`. FTS composes the literal as
  `{scheme}-{id}` (fts/parse.rs:226-235) and defaults a missing country to GB. But a party whose `address.country` is
  `XI`, `IE` or another code would mint its register identity under that country (`register_jurisdiction` does not
  map `XI`). Such an identity sits outside every range read here and in the census.
- **No register lookups.** Queenscourt's number, Hastoe's `16243R`, and the subsidiary verdicts are judged from the
  published names alone.
- **Mention counts not read** for `SCO13683` and `SOCNO28985R`.
- **Bare 6-7-digit charity numbers and bare 4-digit FCA numbers** remain unscannable without a name join, as the
  census said.

## Corrected headline

- **Splits where both stored identifiers start with the scheme's own token:** 36 splits, 73 orgs, 852 mentions.
  - MPR 15 / 30 / 492
  - NIC 1 / 2 / 2
  - UKPRN 5 / 10 / 295
  - CHC 15 / 31 / 63
  - SC 0
- **Names:** 35 same-entity, including 10007157 (mis-headed, not name-unsafe), and 1 partial (1179472).
- **Name gate:** the gate unit 2 needs must read every name an org carries, not only its head name. Otherwise it
  refuses 10007157.
- **Level B:** report it as an increment over level A. It is larger than reported in MPR, CHC and SC.
