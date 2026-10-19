# 530 R2: result (tally of the reads of dry plan job 2659)

Inputs:
- Issue 530: "Unit 2 — decided" and "Unit 3 Build 1 … R1 PASSED".
- `530-triage/decision.md`, R2 and the critic amendments: A5 (random 60, bar 0 W and N ≤ 3), A6 (city screen, 0 W),
  A7 (role coherence, 0 W).
- The plan report `530-triage/guid_twin_plan_2659.json`. It has `r2_random` 60, `r2_top_targets` 30, `r2_city_flagged` 41,
  `r2_incoherent` 30 and `r2_office_two_words` 8. Every list was at or under 45, so every pick was read.
- The reads `r2/read_{rand1..4,top1,top2,city1..3,inc1,inc2,office}.md` and the skeptic passes
  `r2/skeptic_{city1,city2,inc1,office}.md`. Each skeptic pass checked one W and kept it.
- I matched every read to its pick by list and index. There are 169 picks: 162 distinct, plus 7 duplicates of targets
  already scored. Every pick has a verdict, and no read is left without a pick.

## Verdict: R2 FAILS. Fix three findings, re-run R1, then re-read.

| list | picks | R | N | W | bar | result |
|---|---|---|---|---|---|---|
| random 60 (buyer 25, review_or_info 25, other or tenderer 10; salt 530002) | 60 | 60 | 0 | 0 | 0 W, N ≤ 3 | **pass** |
| top 30 targets by planned mentions (the planner added none of the top 20 by rows, so they are inside it) | 30 | 30 | 0 | 0 | 0 W | **pass** |
| city-flagged targets (A6) | 41 | 39 | 0 | 2 | 0 W | **FAIL** |
| role-incoherent groups (A7) | 30 | 28 | 0 | 2 | 0 W | **FAIL** |
| office two-word keys | 8 | 7 | 0 | 1 | 0 W | **FAIL** (the W is the same target as city W 1) |

- **Random-60 bound (A5's corrected sentence):** 0 W in 60 puts the one-sided 95% upper bound on the plan's error rate at
  4.9%, about 541 of 11,103 rows. By stratum: buyer 0 of 25 and review_or_info 0 of 25 each bound at 11.3%; other or
  tenderer 0 of 10 bounds at 25.9%. The top-30 targets are a concentration check, not a sample.
- **Distinct W findings: 3.** They cover 4 distinct picks, or 5 W scores, because 3353580 sits in two lists.
  - Two are A6 umbrella names, 21 planned rows and 42 mentions in total.
  - One is an A7 group printed in two notice copies: 3 planned rows and 6 mentions. Of those, 1 row and 2 mentions are wrong.

## The three W findings

**W1. Target 3353580 "Vergabekammer NRW"** (DE, 675 mentions; `r2_city_flagged[9]` = `r2_office_two_words[5]`). Plan: 16 rows,
32 mentions.
- The target is an umbrella over two chambers. Its own newest mentions are Vergabekammer Rheinland (46376226 and 46082162:
  Zeughausstr. 2-10, Köln, Leitweg 05315-03002-81) and Vergabekammer Westfalen (44543854: Albrecht-Thaer-Str. 9, Münster).
- All 32 planned mentions are the Münster chamber (Wald und Holz notices 24313214 and 24330412, ReviewOrg and Mediator).
  The fold is right inside each notice, but it lands on a row that already mixes two chambers. A6 scores that W.
- The plan keeps the two chambers apart everywhere else: 7615 and 11823847 are Köln, 3233 and 6136373 are Münster.
- Read by read_city1 and read_office. Both skeptic passes kept the W with 0 new GETs.

**W2. Target 6622773 "Geschäftsstelle der Vergabekammer"** (DE, 105 mentions; `r2_city_flagged[15]`). Plan: 5 rows,
10 mentions.
- The target is the Thuringian chamber: 99423 Weimar, vergabekammer@tlvwa.thueringen.de (notices 23604678, 26223383,
  23464988). Its city set is {weimar}.
- The planned rows are Vergabekammer Westfalen, which prints this function name as BT-500/BT-502 (24336021, 24375779).
  5 of the 10 planned mentions print Münster; the other 5 print no city.
- This is a function name shared by at least two chambers. The fold would put Westfalen's mentions on the Weimar row.
- The guard misses it: the name has 2 content words and is not on the unsafe list. Because `geschäftsstelle` is not in
  `GUID_TWIN_OFFICE_NOUNS`, it was not read as an office key either.
- Read by read_city2. The skeptic pass kept the W with 0 new GETs.

**W3. Group "AOK - Die Gesundheitskasse für Niedersachsen", notice 24253392 and its DÖE copy 26483158** (`r2_incoherent[8]`,
`[12]`). Rows 22752175, 22752176 and 22752177 go to target 7861 (714 mentions).
- ORG-0001 (Buyer) and ORG-0002 (AddInfo) are the AOK: Peiner Str. 8 Hannover, +49 511870115210, DE929. Folding them is right.
- ORG-0003 (Lot-ReviewOrg, row 22752177) has the AOK's name, street and e-mail, but the phone +49 22894990 and NUTS DEA22
  belong to the Vergabekammern des Bundes.
  - That phone is on 21 saved sections: 19 are VK Bund sections, and the other 2 are this section in its two copies.
  - This is a review body entered under the buyer's name (A7's W). The fold would give 7861 a ReviewOrg role that was meant
    for the VK Bund.
- Wrong: 1 mention per copy, 2 in total. Read by read_inc1. The skeptic pass kept the W with moderate to good confidence and
  0 new GETs.

## Fixes the decision prescribes (all before Build 2)

**F1. Names (A6, and R2's "on any W").** Add exact folded names to `GUID_TWIN_UNSAFE_NAMES` in
`crates/ingest/src/project/guid_twin.rs`.
- **Required:** `vergabekammer nrw` (W1) and `geschäftsstelle der vergabekammer` (W2). Together they bind 21 rows and
  42 mentions, which then stay as today under (c).
- **Precautionary entries.** None of these is in today's report lists, and each guards future notices only. They are
  explicit names, not a pattern:
  - `vergabekammer nordrhein-westfalen`;
  - `vergabekammer bayern`, the umbrella over Nordbayern and Südbayern;
  - `geschäftsstelle der vergabekammern`, the bare plural.
- **Do not write a "Vergabekammer \<Land\>" pattern.** It would refuse the right single-chamber folds: Niedersachsen 1222120
  (+94), BW 2175136 (+48), Brandenburg (+272), Schleswig-Holstein 9190 (+4).
- **Do not touch** the MV key "Geschäftsstelle der Vergabekammern beim Ministerium … Mecklenburg-Vorpommern" (rand2 item 26,
  R). The list matches the whole folded name, so that key stays out of it.
- **R0 tests:**
  - a "Vergabekammer NRW" twin binds, and so does a "Geschäftsstelle der Vergabekammer" twin;
  - "Vergabekammer Westfalen", "Vergabekammer Niedersachsen" and the MV Geschäftsstelle key still refuse.
- **Read list:** add `geschäftsstelle` to `GUID_TWIN_OFFICE_NOUNS`, so that a re-read takes a two-word "Geschäftsstelle X"
  key whole.
- **No target exclusion by id.** The gate does not read the database and sees only names. An unsafe-name entry has the same
  effect as excluding 3353580 and 6622773, in both the stock and the gate.
- **Compile the names; don't rely on `extra_unsafe_names`.** A3(iii) refuses a wet run that has a non-empty
  `extra_unsafe_names`, so the names must be compiled in anyway. `extra_unsafe_names` remains usable for a dry check before
  the deploy.

**F2. Role-coherence guard (A7).** `guid_twin_refusals` gains a guard, in the gate and the planner as one function, computed
from the notice's own OPT-300/OPT-301 role references (DE1 party references in a DÖE copy).
- **Rule:** a twin group that mixes a buyer-side role with `Lot-ReviewOrg` binds. So does a group that mixes a tenderer or
  winner role with any role that is not a winner (0 such groups today).
- **Recommended form: A7 as written, where the whole group binds,** with buyer-side meaning Procedure-Buyer, Lot-AddInfo,
  Lot-TenderReceipt and Procedure-SProvider.
  - The defect in W3 is in the ReviewOrg section. It does not depend on which buyer-side section it was copied from.
  - The 70 ReviewOrg-mixed groups without Procedure-Buyer are not in the reported A7 list (66 AddInfo+ReviewOrg and
    4 SProvider+ReviewOrg; read_rand2's side note). Only one of them was read (rand2 item 25, GSI SLV, R).
  - **Cost.**
    - Procedure-Buyer scope: the 30 listed notice-copy groups, 37 rows and 78 mentions (0.33% of planned rows).
    - Wider scope: about 100 notice-copy groups in total. The re-run gives the exact figure.
  - All of these stay split as today, under (c). Nothing is merged wrongly.
- **If Build 2 keeps the Procedure-Buyer scope**, the re-read must also read the 70 other ReviewOrg-mixed groups (more than
  45, so the top 30 plus 15 at the re-read salt).
- **Alternative (read_inc2, skeptic_inc1):** refuse a ReviewOrg-mixed group only when every section's phone, NUTS, postcode
  and city agree inside the notice.
  - The fields are BT-503, BT-507, BT-512 and BT-513, or DE1 Contact-Telephone, CountrySubentityCode, PostalZone and
    CityName. Phones are compared after stripping spaces and dashes. E-mail, street, fax, URL and contact are not compared.
  - On today's 30 it binds only the AOK group and keeps the 28 R folds.
  - Its cost: four contact fields in `TwinView` for both notice formats, a parity test, and a rule fitted on one W.
  - A further refinement, binding only the divergent ReviewOrg section, would keep the AOK's 4 right mentions.
  - **Not recommended for Build 2.** On today's list it saves 34 rows and 72 mentions. Ship it later as its own measured
    change if wanted.
- **R0 tests:**
  - an AOK-shaped fixture (Buyer and AddInfo on two values, plus a ReviewOrg section under the same name on a third) binds;
  - a Buyer+AddInfo+TenderReceipt twin with no ReviewOrg still refuses;
  - planner and gate parity on both fixtures.

**F3. Ship F1 and F2 as one projection-neutral build (Build 1b), with the gate still unwired:** `ops/check.sh`, then deploy.
Build 2 stays as decided: KEEP file, gate wired, wet path, recurrence arm (A1).

## Re-run: required

- **R1 again** (dry plan on Build 1b) with all R1 bars exactly as before. Also check:
  - `guard.unsafe_list` counts the W1 and W2 groups (16 + 5 rows, 32 + 10 mentions), and the precautionary entries are
    expected to add nothing;
  - the role guard counts the ReviewOrg-mixed groups, and `r2_incoherent` holds no refused group;
  - 3353580 and 6622773 are no longer targets, and 22752175/76/77 are not planned.
- **Expected plan:** 11,103 − 16 − 5 − 37 = **11,045 rows and 22,204 mentions** under the Procedure-Buyer scope. The
  wider scope gives fewer. Rows minted above id 31,970,152 are counted separately, as before.
- **R2 re-read** (A5: after any W the re-read is 60 rows at salt 530003).
  - The trigger is the decision's "on any W: add that name … to `GUID_TWIN_UNSAFE_NAMES`, re-run R1, read a fresh random
    sample at salt 530003", which A5 sizes at 60. W1 and W2 are exactly that case. skeptic_inc1's point that an A7 W alone
    would not trigger it is moot.
  - **Fresh random 60 at salt 530003**, stratified as before (buyer 25, review_or_info 25, other or tenderer 10). Exclude
    the 60 rows already read by taking the next rows in salted order. **Bar: 0 W and N ≤ 3.**
  - Read any pick in the re-run's top-30, city-flagged, incoherent and office lists that was not already read. Bar: 0 W.
    The office list grows by any "Geschäftsstelle X" key.
- If the re-read is clean, R2 passes and Build 2 may ship.

## The 636 planned mentions of 312 rows with no identifier-NULL target (52 keys)

**Recommendation: let the 234 path mint the fallback row.** That means one fresh provisional identifier-NULL row per missing
`(org_name_norm, register_jurisdiction(country))` key, minted by the dissolve's own re-resolve (the 440 path). The rows are
not kept.

**Why mint:**
- **Parity.** The Build 2 gate is a pure function and does not read the database, so it cannot know whether a target
  exists. It will refuse these values in every twin notice and mint the 234 row on any from-archive rebuild, and for any
  new twin of these names. If the stock keeps the 312 GUID rows, it differs from what a rebuild produces. That is the
  stock/gate divergence A2 removed. Each body would also split between its old GUID rows and the new 234 row.
- **Keeping would need KEEP.** The only consistent way to keep the rows is to put their values in `KEEP_530`. That ties
  KEEP to whether a 234 row happened to exist at freeze time, which is the database-state dependence the decision rejected
  for exemption by bound key. It also strands the rows from any 234 row minted later.
- **Evidence.** Every missing-key pick read R, on the condition that the row is minted:
  - rand1 item 14, Stadt Spremberg - Vergabestelle (6 rows, 12 mentions);
  - rand2 item 17, Gemeinde Kreien über Amt Eldenburg-Lübz;
  - rand2 item 24, Sport- und Bäderbetriebe Essen (6 rows, 12 mentions);
  - city3 item 34, ]init[ DCP LDA (PT, Lisbon and Porto are offices of one Lda).
- **No homonym risk at mint.** A minted row starts with only the planned mentions, so it cannot hold another body's
  existing mentions, which is A6's W mode. Inside the planned mentions, flag c (two or more planned cities) fires on
  1 of the 52 keys (]init[, R). The 30 largest missing keys, 256 rows and 524 mentions, each have a single planned city.
- **Consolidation.** 312 rows become 52, which is the issue's fix. The change-feed cost is 52 rows `added` and 312 `removed`,
  each a 404 with no redirect, the same as the rest of the dissolve.

**What Build 2 and R3 must check:**
- **One mint per key.** Windows go by target key, so the first re-resolve of a missing key mints and the rest bind to that
  row. R3 checks:
  - the count of new identifier-NULL rows equals the wet re-plan's missing keys (52 at job 2659);
  - each new row has `country = register_jurisdiction` and mentions equal to its key's planned mentions;
  - no two identifier-NULL rows share a key afterwards.
- **The restore manifest** records the minted row as the target, and A1's recurrence arm takes it as input.
- **Unread names.** Most of the 52 keys are unread, but each has one planned city. Two are bare department names:
  - "Abteilung 5.2 - Haus- und Betriebstechnik" (Freiburg, 3 rows);
  - "Kommunale Informationstechnik und Telekommunikation" (Jena, 4 rows).

  They are safe at mint. Later 234 mentions of the same name land on them, as with any 234 row. Read them in the re-read if
  they come up.

## R verdicts with caveats (none affects a bar)

- **Conditional on the mint:** random items 14 (23053387), 17 (22856837) and 24 (22895106), and city item 34 (]init[).
- **Weakest evidence:**
  - random item 54, Facility Care AG 22987736 (no target mention read);
  - city item 8, FAU 31365873 (planned rows not located);
  - city item 39, BIB Augsburg 8545951 (Gersthofen mention not located).
- **Planned rows not located, city screen only:** office items 6 (3180, Südbayern) and 7 (1163917, Nordbayern).
- **Single cities not located:**
  - GIZ 1165385 (2 Bonn mentions; one company with two seats);
  - Aachen 30899495 (1 Geilenkirchen mention);
  - Wald und Holz 11195375 (10 Bad Driburg mentions);
  - RP Stuttgart 1366195 (Heilbronn);
  - VK Bund 2217 (Eschborn).
- **vdek 4193988** is "at worst N" if each Landesvertretung counts as a separate buyer. It is not W.
- **Gemeinde Ensdorf 1437249** is a latent homonym: Ensdorf in the Oberpfalz is also a Gemeinde Ensdorf. The target holds
  only Saarland mentions and the planned rows are Saarland, so the read is R. Listing the name would cost 4 rows and
  8 mentions, which is optional.
- **Follow-up outside R2:** the 234 rows 3353580 and possibly 6622773 already mix chambers through identifier-less
  mentions, the same shape as 1431661 "Vergabekammer". F1 stops 530 from extending them but does not repair them.

## Evidence base

- **GETs.** 558 public GETs logged in `r2/api_requests.log` between 17:27:03Z and 18:26:53Z: 331 `/v1/notices/{id}/content`
  and 227 `/v1/organizations/{id}`.
  - All returned 200, with no 408 and no retry. No gap was under 2 s at the log's resolution.
  - There are 558 saved bodies in `r2/bodies/`.
- No `/v1/sql`, ssh or POST was used. The skeptic passes and this tally made 0 GETs.
- Machine-readable copy: `r2/r2-result.json`.
