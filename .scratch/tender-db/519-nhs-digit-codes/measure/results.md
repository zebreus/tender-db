# 519 measure-first (M1–M7): results, and recommendations for rulings 2 and 3 (2026-10-12)

Synthesis of the measure-first step of `../unit2-decision.md`. This file reads the per-item outputs in this
directory (`m1.md`/`m1.json`, `m2.md`/`child-map.json`, `m3.md`/`m3.json`, `m4.md`/`m4-results.json`,
`m5.md`/`m5.json`, `m7.md`/`m7.json`). For the claims below that the agent summaries did not carry, I checked
`crates/` at HEAD 8b73494; nothing there was edited.

## 0. Bottom line

- **Ruling 2: keep the strict all-names gate, and drop the attestation alternative.** In phase A the strict
  gate holds 1 holder / 1 mention (0.07%) on the body's own name, and that refusal is a `tie{QMF}`, which the
  narrow attestation could not reach either. Both options fold exactly the same phase-A holders. Every phase-A
  name the attestation *could* reach (`zero`/`coverage`, 7 mentions) turned out, on a notice read, to be
  another party or a site. S1 does not trip.
- **Ruling 3: take the first branch.** Rehome the 15 CSU-named 13T mentions to 30929531 and fold the 488
  NENC-named ones into QHM. M7 shows NECS as its own party in the buyer slot, with NENC listed beside it in 14 of
  15. Two things the measurements found must be settled before 13T joins: the 16th refused mention
  (46991199), and the head-name gate (§5). If either stays open, 13T is held whole, which costs 488 of the 1,475
  phase-A fold mentions and nothing else.
- **The phase-A fold, measured:** 34 holders / 1,475 mentions after 26 rehomes (33 / 987 without 13T). The
  decision estimated 22 / 945 before its zero-only and site rows.
- **The plan changes the measurements force (§6):**
  - E5's head-name clause for identity holders;
  - two wrong §1.3/§6 pins (04Y answers `zero`, College Road answers `non-nhs`);
  - zero-only members drop to 3 holders / 3 mentions;
  - U6 needs a PPON tier;
  - M6 moves into U3.
  - Nothing found changes the design's mechanics (§2–§3).

## 1. Reads

All were read-only, with no 408s, no retries and no truncated answer. Nothing was written on prod. `nhs-ods.tsv`
is byte-identical (sha256 recorded in `child-map.json`).
- **M1/M5:** 52 bounded `/v1/sql` SELECTs, reusing M4's reads of the 77 sub-ICB and site holders.
- **M2/M4:** 24 SELECTs, plus 369 ODS ORD fetches (344 RO98, all HTTP 200; 25 RO198).
- **M7:** 13 SELECTs.
- **Other read-only app endpoints**, not the `/v1/sql` seeks the brief named:
  - M3: 77 public org details (`/v1/organizations/<id>`) and six admin review-table readbacks (limit 5,000);
  - M7: 43 × 3 public notice, content and FTS OCDS reads, and 21 org details.

## 2. Per item

### M1: every child holder's names under `child_check` (85 holders)

The port (`child_check_sim.py`) still reproduces 513's tally: 233/235 SAME registry, 116/116 SAME PPON, and 0
DIFFERENT/NOT_ORG admitted. Its dated parents match M2 for all 85 holders. 22 names were called by hand
(`m1_overrides.json`).

| kind | holders / mentions | split |
|---|---|---|
| sub-ICB | 52 / 1,844 | all-admitted 34 / 861; other-body strays 9 / 811; own-name noise 3 / 83; strays + noise 1 / 80 (92G); none admitted 3 / 5 (W2U3Z, 03Q, 04Y); no RO261 parent 2 / 4 (14J, 14N) |
| trust site | 25 / 36 | trust-named, admitted 12 / 12; site-named 8 / 9; wholly another body's 5 / 15 |
| zero-only | 8 / 265 | admitted 7 / 264; typo 1 / 1 |

**Phase A** (chosen parent with an applied PPON survivor): 41 holders / 1,514 mentions.
- Clean: 19 / 742.
- Mixed: 6 / 750. These hold 26 strays: 13T 16, 15E 4, 26A 2, 15N 2, 15C 1, 93C 1.
- 03Q: 1 / 1, NHS England.
- Zero-only: 6 / 12.
- Sites: 9 / 9.

**Own-name refusals:**
- In phase A there is exactly one: 31615636 "NHS North East and North Cumbira ICB" (00P, 1 mention,
  `tie{QMF}`).
- Across all 85 holders there are 6 holders / 11 mentions. The other five are 92G, 72Q, 36L, 70F and W2U3Z, all
  under anchor parents that ruling 5 holds anyway.

**Ruling 7 cohort:** 6 sites / 6 mentions (B4Z5Z, Q7N9Z, RBT20, RCU51, RCX70, RYY13). B4Z5Z's ODS name is "THE
ROYAL MARSDEN HOSPITAL (SUTTON)", so its mention "The Royal Marsden Hospital" is the site's name as much as the
trust's. That one is the reviewer's call.

**Expected answers that differ from the decision:**
- "College Road Surgery" under QHL answers `non-nhs`, not `zero`: `college` without `hospital` trips `non_nhs`
  first.
- 04Y's "NHS Shropshire, Telford and Wrekin ICB" under QNC answers `zero`, not `better{QOC}`. `check` refuses
  Zero (`nhs_ods.rs:295-297`) before it looks for a better rival (`298-312`).
- M1's selftest did not cover the 04Y row, so `m1.md`'s "only College Road differs" is one short. 03Q (`nhse`)
  and W2U3Z (`tie{Z9B2Z}`) match.

### M2: the dated child map

- **Output:** 410 rows. That is 223 RE5 interval rows over 182 coded sub-ICB children, 162 `rel -` rows and 25 RE6
  site rows.
- **Exit conditions (§2.2):** all clear, with 0 overlapping intervals and 0 parents missing from `nhs-ods.tsv`.
  One adjacent pair was merged (06K under S1Y5D).
- **Dropped relations (for the table header):**
  - RE5→RO210: 495;
  - RE5→RO173: 215;
  - RE5→RO209: 14;
  - RE3→RO162: 2;
  - RE2→RO261: 53. All 53 duplicate an RE5 row exactly.
- **03K** has a parent: QOQ from 2020-04-01, open. Unit 1's `"000"` was a failed fetch. All 12 of its mentions are
  named Humber and North Yorkshire ICB.
- **The 52 sub-ICB holders:** 41 have an allowlisted parent, 9 only off-allowlist parents, and 2 none (14J, 14N).
  Only D4U1Y sits under two allowlisted parents (QNQ, then QRL). Three further codes have two allowlisted parents
  through a one-week first parent, but no holder: L7N8K, Q0S2C and O4I9P (O4I9P has three parents).
- **Parent changes:** all 41 run straight on, 37 of them at 2026-03-31 → 04-01.
- **Sites:** 14 sit under allowlisted trusts. 11 sit under trusts whose codes contain a digit (U6 territory).

### M3: purity and flags (77 holders, 1,880 mentions)

| denial | holders |
|---|---:|
| `child-impure` | 0 |
| `child-names-unread` | 0 |
| `child-carries-ppon` | 0 |
| `child-withheld` | 0 |

- **Identifier spellings:** all 1,880 mentions carry the holder's own triple. 97 are other spellings of it: `26a`
  ×81 and other lower-case tails, plus `GB-NHS-16 (c)`. None is country-less, foreign or another value.
- **Ledgers and review stores:** 0 hits. The merge ledger was read through two stand-ins: empty
  `merged_identifiers`, and every holder answering 200 under its own id. The name-verdict readback hit its 5,000
  limit, so only those rows were checked.
- **469:** only 06H appears, in its register sample. 06H is A2 and must be denied at 469 U7, which §7 already
  plans.
- **Risk 9 does not happen.** Names and anchors are the only things between phase A and the fold.

### M4: alias load and the share published after the parent's end

- **Last 12 months:** 419 child-coded mentions, a mean of 34.9 a month and a median of about 20. 119 of them are
  one B2M3M bulk day, 2026-05-27.
- **2026-06..09:** 49 mentions, of which 8 are phase A, about 2 a month for the armed triple after U5.
- **13T** has had no mention since 2024-12-02.
- **After the parent's end:** 14 of 1,880 mentions (0.74%) overall, and 13 of 419 (3.1%) in the last 12 months.
  - All 11 phase-A cases use the ended body's own name (D4U1Y "Frimley" ×8, the newest 2026-10-05; 26A ×2; 93C
    ×1). With no date fence (ruling 4), none of them becomes a stray.
  - Only one mention in the whole set names the new body: 10Q "NHS Thames Valley ICB" (A3).
  - So strays come from other bodies' names, not from dates.
- **Canary notices:** the post-April notices exist, 46810637 (26A) and 47366322 (D4U1Y).

### M5: the zero-only tails' holders

All 10 holders were found; the census counts reconcile (00P 6 / 25, 00N 1 / 5, 00L 1 / 1, 00X 2 / 2).
- **Already folded.** 25 of the decision's 31 "zero-only parties under QHM" already sit on QHM's survivor
  31556600 through 513's folds: 00P ×20 and all 5 of 00N. 00N has no holder left.
- **Left under QHM:** six provisional holders, 12 mentions.
  - **Members, admitted:** 31610859, 31615979 and 31632651 (00L). 3 mentions.
  - **Leave out:** 31603816 "… County Durham Place" and 31605903 "… Tees Valley Place", 8 mentions. They carry
    QHM raws, and 513 U3 adjudicated them `HOSTED_OR_SITE`. The gate admits them, but naming them would reverse
    a reviewed 513 call.
  - **Held:** 31615636, the "Cumbira" typo (`tie{QMF}`).
  - None of the six names is an armed 513 alias key. Folding the 3 members arms their names (`loser_name_norms`).
- **00X sits under QE1, which is off the allowlist.**
  - 30915461 is the Lancashire and South Cumbria ICB *name cluster*: 252 mentions, only 1 of them coded 00X.
  - 31624168 has 1 mention and is admitted under QE1.
  - QE1's body stands as four rows:
    - the PPON org 31569664;
    - the identity org 30915463 (`GBNHSQE1`);
    - a typo identity 31549058 (`GBNHSQE1000`);
    - that name cluster.

### M6: not run

M6 needs a prod write (posting QHL15E's 452 correction) and a dry `rekey-plan` job. Both are outside this phase's
read-only constraint, and no agent ran them. M6 does not bear on U1 or U2; it becomes U3's first step (§6).

### M7: notice reads (44 mentions / 43 notices)

- **13T** is org 31600907. Its 504 mentions split 488 NENC-named, 15 CSU-named and 1 NHS England-named. The
  decision's 481 counted only the top five names.
- **The 15 CSU-named mentions** (2024-04-23..05-09) are PSR direct awards from NECS's account
  (`necsu.psrprocurement@nhs.net`).
  - The party `GB-NHS-13T` is named "NORTH OF ENGLAND COMMISSIONING SUPPORT" and is the release's buyer in all 15.
  - NENC is listed separately as review body in 14 of 15.
  - The 14 sampled NENC-named 13T notices come from the same publisher, which states it acts "for and on behalf
    of" NENC.
  - **Precedents:**
    - 0AR's survivor 30929531 already holds NECS-as-agent mentions;
    - 513's QHM verdict kept the NECS-named org 3257070 out of QHM.
  - **Reading:** the CSU as its own party. Rehome to 30929531, HIGH.
- **The 16th mention:** 46991199 / 036139-2022, a PIN, with the buyer named "NHS England - North East &
  Yorkshire".
  - The gate refuses it (`nhse`).
  - It is the notice's only party.
  - Its text says NECS acts "for and on behalf of North East and North Cumbria ICB (The Contracting Authority)".
  - M7 proposed Y63 31541949 as the target, MEDIUM on the row choice (the exact-name twin is 31619786).
- **The other 10 phase-A strays are all other parties, HIGH, with a target each:**
  - 26A → 30915905 (RM1) and 1280779 (RGP);
  - 15E → 31542353 (QUA), 31601027, 31601028 and 31601029 (GP practices M85074, M85087 and M85108);
  - 15C → 30917049 (QJK);
  - 15N ×2 → 31536069 (QUY);
  - 93C → 31617815 (the NEL CSU exact-name cluster; no CSU body exists).
  - **Three shapes:**
    - 5 are suppliers carrying the ICB's party id (the issue-484 shape);
    - 4 are another ICB's buyer typed with the wrong code (15C and 15N are swapped between Devon and BNSSG);
    - 1 is a CSU acting as its own party.
- **After the 26 rehomes** the six mixed holders' mentions are all admitted: 724 = 13T 488 + 26A 113 + 15E 42 +
  15C 30 + 15N 29 + 93C 22. 13T's head name is the exception (§5).
- **Optional rehomes:** RTR54 and X7V4Q are NENC published under trust site codes and could go to 31556600, HIGH.
  RNZ91 already sits on the right org.

## 3. The measured phase-A partition (replaces §1.5's phase-A rows)

| group | holders | mentions | unit 2 |
|---|---:|---:|---|
| A1 clean sub-ICB: B2M3M 600, D4U1Y 65, D2P2L 16, 03K 12, D9Y0V 10, 16C 8, 99C 6, 03R 5, M2L0M 4, 11J 3, 15F 3, 42D 2, 78H 2, 01H, 02Y, 18C, 36J, 84H, 92A (1 each) | 19 | 742 | fold |
| A1m mixed: 13T 504, 26A 115, 15E 46, 15C 31, 15N 31, 93C 23 | 6 | 750 | rehome 26, then fold 724 (13T per §5) |
| zero-only members under QHM: 31610859, 31615979, 31632651 | 3 | 3 | fold as name members |
| zero-only place orgs 31603816, 31605903 | 2 | 8 | left out (513's `HOSTED_OR_SITE`) |
| zero-only typo 31615636 | 1 | 1 | held (strict gate) |
| ruling-7 sites: B4Z5Z, Q7N9Z, RBT20, RCU51, RCX70, RYY13 | 6 | 6 | fold (B4Z5Z is the reviewer's call) |
| sites wholly NENC's: RTR54, X7V4Q | 2 | 2 | not members; optional 317 rehome to 31556600 |
| site-named RNZ91 | 1 | 1 | stays |
| 03Q (NHS England) | 1 | 1 | never; issue 523 |
| **phase A** | **41** | **1,514** | **fold 34 / 1,475** (33 / 987 without 13T) |

The other 1,514 − 1,475 = 39 mentions are the 26 rehomed strays plus the 13 mentions on the 7 non-members.

## 4. Ruling 2: strict all-names gate or attestation

| | strict all-names gate (default) | narrow attestation (`zero`/`coverage`, no rival fits) |
|---|---|---|
| phase-A holders / mentions folded | 34 / 1,475 | 34 / 1,475, identical |
| phase-A holders held by own-name refusals | 1 / 1 (31615636) | 1 / 1: its refusal is `tie{QMF}`, so a rival fits and the attestation cannot reach it |
| S1 (more than 3 holders or more than 5% halts) | 1 holder, 0.07%: no halt | the same |
| the pool the attestation would cover, phase A | — | 7 mentions, all other parties or a site: 15E Black Country, Wake Green, Small Heath (`zero`); 15C Devon (`zero`); 15N BNSSG ×2 (`zero`); RNZ91 Salisbury Medical Practice (`coverage`). 0 own names. |
| the same pool, all 85 holders | — | 19 names / 21 mentions. Only 1 name / 2 mentions is the body's own: 72Q "SEL ICB (Lewisham & Greenwich)", anchor-held under ruling 5. |
| bind time | each own-name refusal mints a listed `identity-stray`. Expect about 0 a year: about 2 phase-A coded mentions a month (M4) × 1 own-name refusal in 1,514 (M1). | needs the attestation consulted at bind time too, or the fold becomes looser than the bind |

**Recommendation: rule the strict all-names gate, and withdraw the attestation alternative.**
- The attestation folds nothing more: +0 holders in phase A. Outside phase A it would reach only 72Q's 2
  mentions, and only if U7 ever lifts the anchor hold.
- Its pool is exactly where the strict gate earned its keep. A reviewer attesting 15E's `zero`-refused "Wake
  Green Surgery" or "Small Heath Medical Practice" as BSol noise would have folded GP practices into the ICB.
  M7's notice read showed them to be suppliers carrying the ICB's party id.
- Keep S1 as written. It guards the live U2 census against drift from M1's offline numbers.

**Proposed ruling text:** "2. Strict all-names gate, at fold and at bind. The attestation alternative is
withdrawn: M1 holds 1 phase-A holder / 1 mention (a tie the attestation could not reach), and every attestable
phase-A refusal was another party (M7)."

## 5. Ruling 3: 13T

**Recommendation: take the first branch.**
- Rehome the 15 CSU-named mentions to 0AR's survivor 30929531 (HIGH; M7 §2 lists all 15 by notice and
  `ORG-GB-NHS-13T`).
- Fold the 488 NENC-named mentions into QHM.

M7's facts answer the ruling's condition: in the 15, NECS is the buyer party *as itself*, and NENC is a separate
party on the same notice in 14 of them. In 488 other notices from the same account, NECS names NENC as the buyer.
The two precedents read the same way. Holding 13T whole would keep 488 NENC mentions (a third of the phase-A fold)
off QHM's survivor over 16 mentions that can be rehomed.

Before 13T joins, two conditions must hold. The measurements surfaced both.

1. **The 16th mention, 46991199, must leave 13T.** Under the strict gate it holds 13T on its own.
   - **What the notice shows.** Its only party is the buyer, published as "NHS England - North East & Yorkshire",
     under NENC's own sub-ICB code. The text names NENC ICB "The Contracting Authority". NHS England NEY is the
     region of NECS's host body.
   - **Two targets are defensible:**
     - (a) QHM's survivor 31556600, on the code and the text;
     - (b) Y63 31541949, on the published name.

     Under (b), the row choice should be read as HIGH. It is the canonical, code-keyed row of the named body, the
     same shape ruling 3 uses for 0AR (30929531, not the provisional NECS cluster 3257070). The twin 31619786 is
     a provisional row of the same body, which issue 523 consolidates.
   - **Recommend (b) under the decision as written.** §1.4 lets only another body's mention be rehomed, and by
     name this one is. Reviewer and challenger must both record HIGH (S1).
   - **Choose (a) only with the owner's say-so.** It needs §1.4 amended so that a reviewed 317 verdict may move a
     body's own refused mention to P's survivor.
2. **E5 must stop gating 13T's stale head name.**
   - 13T's head is "NORTH OF ENGLAND COMMISSIONING SUPPORT". `organizations.name` is written at mint and never
     again (issue 524; no `UPDATE organizations SET name` exists in `crates/` outside tests). So the head stays the
     CSU's name after the rehome, although no mention carries it any more.
   - E5 gates "the head name and every distinct mention name", so 13T stays held. M7's "all six fully admitted"
     overlooks this; M1 flagged it.
   - **The fix is the E5 amendment in §6 (U2).** The alternative is to hold 13T until 524 ships a head refresh.

If either condition is not met, hold 13T whole. No other phase-A holder depends on it. The fold becomes
33 / 987, and S3 and S5 drop 13T.

**Proposed ruling text:** "3. 13T: the CSU acted as its own party (M7). Rehome the 15 CSU-named mentions to
30929531, rehome 46991199 to Y63 31541949, then fold 13T's 488 NENC-named mentions into QHM under the E5 amendment.
If either rehome falls short of HIGH from reviewer and challenger, hold 13T whole."

## 6. Changes the measurements force on the build plan (§7)

**U0 (docs).** Record these corrections in issue 519 and the decision:
- 13T is org 31600907, with 488 / 15 / 1 mentions.
- 15C is mixed: 1 Devon stray.
- A1 is 19 / 742: 03K in, under QOQ; 15C out.
- The mixed group is 6 / 750, with 26 strays.
- 93C's stray is NEL CSU (0AQ), not "NCL ICS": that name is admitted.
- 25 of the 31 zero-only QHM parties are already folded, and 00N has no holder.
- Phase A folds 34 / 1,475.
- M6 moves to U3.

**U1 (generator, table, `nhs_ods.rs`).**
- **Generator.** Port `build_child_map.py`'s rules and seed `raw/` from `measure/ods-ro98/` (344 records, all
  200), not from the 66 in `ods-table-codes/`.
  - Expect 410 rows: 223 RE5, 162 `rel -`, 25 RE6.
  - Expect the dropped-relation header counts listed under M2.
  - Exit non-zero if an RE2 row ever disagrees with its RE5 twin.
- **Fix two §1.3/§6 pins:**
  - 04Y's name under QNC → `zero`;
  - "College Road Surgery" under QHL → `non-nhs`.
- **Add pins:**
  - 03K → QOQ (2020-04-01, open);
  - O4I9P has three parents;
  - D4U1Y is the one held child with two allowlisted parents.
  - `MEASURED_RO198` is the 25 site codes.

**U2 (census).**
- **E5, amended for identity holders.**
  - The gate admits every distinct mention name read; E3 guarantees the read is complete.
  - The head name is checked and reported. It gates only as one of those mention names, so a head that no
    mention carries any more is informational.
  - For zero-only clusters the head still gates: the fold arms their name keys.
  - Why this is safe:
    - an identity loser's keys are empty (§3.4) and its row is deleted at the fold;
    - the bind gate sees only `m.name`;
    - so a stale head keys nothing.
  - Only 13T is affected today. Every other holder's head is one of its mention names (`m1.json` `head`).
  - Tests:
    - `a_head_name_no_mention_carries_does_not_hold_an_identity_child`
    - `a_zero_only_cluster_head_name_still_gates`
- **S0 reads for the first live census:**
  - E3/E4 denials should be 0 (M3); any non-zero value is drift to explain before posting;
  - the zero-only holder ids equal M5's list;
  - the phase-A own-name hold equals 1 (31615636).
- **00X** is listed as `child_unanchored` under QE1. Its 30915461 is QE1's name cluster, never a zero-only member.

**U3 (adjudication).**
- **M6 first:** post QHL15E's 452 correction, run a dry `rekey-plan` (expect `merge` into 30916123), and list
  the COH and PPON correction shapes, holding the PPON moves. Then the 452 cohort.
- **The rehome batch.**
  - The 26 rows of `m7.json` with `reading.action == "rehome"`. 25 are HIGH already; 46991199 is re-posted at
    HIGH to the §5 target.
  - Optionally RTR54 and X7V4Q → 31556600.
  - Dry `apply-rehoming` should list exactly these moves plus `missing_target 4`: four pending
    `rehoming-2026-08-30` PMO verdicts that have no target (M3's readback). The wet `expect` is these tuples only.
- **Re-census.** The six mixed holders and every rehome target will show `under_review: true`, because
  `review_keep_set_but` reads `org_mention_rehoming` case and target ids (`canonical.rs:36271-36281`). It is a
  reviewer flag, not a D6 denial.
- **Post cohort `nhs-body-children-<date>`.**
  - **QHM members:** 16C, 99C, 01H, 84H, 31610859, 31615979, 31632651, and 13T per §5. Not 31603816, 31605903
    or 31615636.
  - **Ruling-7 sites:** the six above; B4Z5Z is the reviewer's call.
  - **Record for the reviewer:** §6's pin folds 15F "(Leeds Place)" into QWO, while 513 left QHM's
    "County Durham Place" and "Tees Valley Place" orgs out. Keep that asymmetry deliberately, or re-adjudicate
    those two as a 513 amendment. The default is to keep it.

**U4 (alias, fold, unwind).**
- **Tests:**
  - `a_lower_case_child_literal_binds_the_same_triple`: `26a` is 81 of 26A's 115 mentions, and
    `GB-NHS-16 (c)` occurs once;
  - an identity loser whose ledger `loser_name` is a stale head (13T's ledger row will read the CSU name, which is
    expected and keys nothing).
- **Ruling 9:** as written it records `GBNHS00P` and `GBNHS00L` only. 00N's parties already sit on the survivor
  with no holder left to fold, so `?identifier=GBNHS00N` answers nobody unless U4 also records each zero-only tail
  the survivor's own mentions publish. The default is to accept the gap, since the record is lookup only.

**U5 (the run).**
- **S3:** QHM's capped fold carries 13T's 488 mentions if 13T joins. Its S3 read adds:
  - 13T's 16 rehomed mentions stay on their targets;
  - `GBNHS13T` answers 31556600.
- **S5:** 13T's newest notice is from 2024 (2024-12-02). Use 46810637 (26A) and 47366322 (D4U1Y) as the
  post-April notices, and include one `26a`-spelled notice.
- **S1:** no halt expected (1 holder, 0.07%).

**U6 (identity bodies, a separate decision).**
- A PPON tier is needed first. QE1's body has a standing PPON org (31569664) beside its `GBNHSQE1` row (30915463).
  "The one standing `GBNHS<P>` org becomes the survivor" would leave the PPON org apart.
- 00X's 30915461 belongs with the QE1 body, not with the zero-only children.

**U7 (if anchors are ever lifted).**
- The strict gate would still hold 5 anchor children on their own names: 92G (`better{13G}`, a false refusal by
  an inactive-CCG rival), 72Q (`zero`), and 36L, 70F and W2U3Z (ties).
- 92G's SCW CSU mention is another body's (0DF → 30915590).

## 7. Outside 519 (follow-ups, not unit 2)

- **NECS stands as three orgs:** 30929531 (0AR, PPON), 3257070 (provisional, 253 mentions, 14 coded QHM) and
  10286686 (under US). This needs a 513 / 362 follow-up for 0AR.
- **NEL CSU stands as five provisional clusters** (407 mentions) with no body.
- **For issue 523:** NHS England NEY stands twice (31541949 Y63 and 31619786), and two holders are wholly NHS
  England's: 03Q and RW5NC (11 mentions).
- **Wholly another body's holders:** RH8C3 → QJK, RJ128 → RJZ, 04Y → QOC. These are 317 or 452 material.
