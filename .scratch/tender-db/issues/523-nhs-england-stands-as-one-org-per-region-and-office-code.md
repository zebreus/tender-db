# 523 — NHS England stands as one org per region and office code

Status: ready-for-agent — triaged 2026-10-12. The claims hold (X24 → 30929530 with 95 mentions; Y56 → 30914489 with 86; Q75 → 47; Y63 → 12), but the real split is national. Six orgs named "NHS England" hold 5,474 mentions: - name cluster 3845883 (3,395); - PPON org 30913629 (1,866); - X24 (95) and Y56 (86); - 4916125 (30) and COH-Y56 (2). The 513 survivor rule would pick the PPON org, not X24, and X24 (RO162, digit-bearing) is neither in nhs-ods.tsv nor allowlisted. NEXT, unit 1: decide and measure, offline plus bounded reads, no code. Record the decision with its reasoning: - **Phase A:** fold the national six into PPON survivor 30913629, with X24 and Y56 as identity losers. First rehome (317) the ~10 London-ICB mentions out of Y56 and Portsmouth's 1 mention out of the PPON org; send COH-Y56 to 452 `wrong`. - **Mechanism:** either a one-row national-body allowlist entry for X24 (like DQF), built together with 519 U6 identity bodies, or a separate reviewed arm. Say which, and why. - **Phase B cohort:** list it in full by paging `/v1/organizations?name_prefix=NHS England` to the end and reading mention names for the top ~30. Cohort: the 34 England-named coded orgs / 407 mentions, and at least 276 name-only regional clusters / about 1,800 mentions (first 3 pages only). - **Exclusions:** keep out the review panels (the 609-mention "Independent Patient Choice and Procurement Panel", the 17-mention "PSR Review Panel"; see 483) and CSU-agent names. - **Verify line:** point it at the survivor id 30913629 (see the triage note).
Was: needs-triage — filed 2026-10-12 from issue 519's unit-1 class C and unit-2 ruling 11.
Kind: data quality (organization identity)
Relates to: 519 (class C), 513 (nhs-body verdicts), 300 (identity ladder)

## Verify

    ssh root@zebreus.click "curl -s 'http://127.0.0.1:8080/v1/organizations?identifier=GBNHSX24'" | head -c 300

- **done**: one NHS England body, with the region and office codes as its merged identifiers (or a recorded decision to
  keep them apart);
- **open**: X24 answers its own org, and the region and office codes answer others.

## What was found

519 unit 1 (`../519-nhs-digit-codes/unit1-results.md`, class C: 24 orgs / 383 mentions) found NHS England as many
canonical orgs, each minted by a digit GB-NHS code:
- X24 "NHS England" (95 mentions), ODS RO162 (an other statutory authority, the legal body);
- Y56 (86), ODS "London Commissioning Region", RO209;
- the Y5x/Y6x commissioning regions;
- the Q4x/Q7x local offices (RO210, e.g. Q48 "Merseyside Area Team", Q71 "NHS England London");
- X26 (RO116), X2407 and XDH04 (sites);
- the typed names `NHSENGLANDNORTHEASTANDYORKSHIRE…Q72/Q74/Y63`.

Some of these codes are mis-coded on trusts and surgeries: Y54 is on York & Scarborough, Y61 on Swan Surgery, Y62 on
East Lancashire Hospitals.

## Decide

- (a) One body: fold the region and office codes into X24's org by reviewed verdict, 513/519-style. The regions are
  offices of one legal body, NHS England, which itself merges into DHSC from 2026–27.
- (b) Keep the regions apart as buying points, but fold the typed-name and duplicate orgs into their region.
- In either case, the mis-coded Y5x/Y6x on trusts are real codes on the wrong holder: 317 rehoming (519 §4), not
  452 `wrong` (ruling 8 is for typos).

Measure first:
- the mention names per org (bounded reads);
- ODS Rels (region → X24?);
- how publishers name themselves ("NHS England – North West" vs "NHS England").

## Triage (2026-10-12, owner; panel wf_73cc2c40-e1c)

Triage 2026-10-12. The claims hold, and the live Verify is open:
- X24 → 30929530 (95 mentions);
- Y56 → 30914489 (86);
- Q75 → 30916311 (47);
- Y63 → 31541949 (12).

The issue is scoped to the small layer, though. Six orgs named "NHS England" hold 5,474 mentions: cluster 3845883 (3,395, no identifier), PPON org 30913629 (GB-PPON-PDPT-3135-BWWY, 1,866), X24 (95), Y56 (86), 4916125 (30) and junk-COH 31603756 (2). Under 513's `nhs_body_survivor` (canonical.rs:6973) the survivor is the PPON org, not X24's org, so option (a) as written names the wrong target.

The existing machinery cannot fold it. X24 is RO162 and digit-bearing, so it is neither in nhs-ods.tsv nor on its allowlist. ODS relates no RO209 region or RO210 office to X24: regions have no parent, offices point to the old letter regions and Y5x, and only the X2407 site points RE6→X24. So no 519-style dated parent map can be built from ODS.

How publishers name themselves (bounded reads):
- the PPON org is 98% plain "NHS England";
- the X24 org carries about 20 regional names, so regional teams already publish as NHS England;
- Y56 is mostly "NHS England" (55), with about 10 London-ICB mentions mis-coded onto it.

Decision lean: (a), in two phases.
- **Phase A:** the national six into PPON survivor 30913629, with X24 and Y56 as identity losers. Rehome Y56's ICB mentions and Portsmouth's stray mention first (317), and give GBCOHY56 a 452 `wrong`.
- **Phase B:** a reviewed region/office cohort. 519's cache puts it at 34 England-named coded orgs / 407 mentions; the first 3 name_prefix pages add at least 276 name-only clusters / ~1,800 mentions. Exclude the review panels (483) and CSU-agent names.
- **Mechanism:** share 519 U6's identity-body build (X24 is a digit parent with a PPON tier, like QE1), and settle with 469 U7 who owns the X24↔PPON pair, since U7's ODS check has no RO162 row and would deny it.

Correction: the mis-coded Y5x/Y6x holders (11 orgs / 32 mentions) are real codes on the wrong holder. 519 §4 sends those to 317 rehoming; ruling 8 is for typos.

Verify fix: use the free public form, `curl -s 'https://tenders.zebreus.click/v1/organizations?identifier=GBNHSX24' | head -c 300`.
- **open:** id 30929530, 95 mentions;
- **done:** id 30913629, with GBNHSX24 in merged_identifiers.

NEXT, unit 1 (M, no code): record the decision and finish the measurement (the full name_prefix census, mention names for the top ~30 orgs and the coded set, and the 317 rehome rows).

**Claims:** Yes, the claims hold, but the issue looks at the wrong layer: the coded region and office orgs are the small part of the split.

What checks out:
- **Verify is open (public GETs, 2026-10-12):**
  - `?identifier=GBNHSX24` → 30929530 "NHS ENGLAND", 95 mentions, merged_identifiers [].
  - GBNHSY56 → 30914489, 86.
  - GBNHSQ75 → 30916311, 47.
  - GBNHSY63 → 31541949, 12.
- **ODS facts in 519's cache (`519-nhs-digit-codes/gbnhs-orgs-classified.json`) match the issue:**
  - X24 is RO162.
  - Y56 is RO209 "LONDON COMMISSIONING REGION".
  - Q48 is RO210 "MERSEYSIDE AREA TEAM"; Q71 is RO210/RO211.
  - X26 is RO116.
  - X2407 and XDH04 are sites.
  - Y54 sits on York & Scarborough (1 mention), Y61 on Swan Surgery (2), Y62 on East Lancashire Hospitals (4).

What the issue misses:
1. **The national split is the big one.** Six orgs with name_norm "nhs england" hold 5,474 mentions (519's `name-twins-2026-10-12.json` plus public GETs):

   | org | what it is | mentions |
   |---|---|---|
   | 3845883 | provisional, no identifier | 3,395 |
   | 30913629 | GB-PPON-PDPT-3135-BWWY | 1,866 |
   | 30929530 | X24 | 95 |
   | 30914489 | Y56 | 86 |
   | 4916125 | country-less | 30 |
   | 31603756 | GBCOHY56, a junk COH | 2 |

2. **Option (a) names the wrong survivor.** Under 513's machinery the survivor would be the PPON org, not X24's org. `nhs_body_survivor` (`crates/store/src/canonical.rs:6973`) picks the GB:ppon member; X24's identity org can only be a loser.
3. **The current machinery cannot do the fold.** X24 is not in `crates/ingest/sdk/nhs-ods.tsv`: the table holds only RO197/RO107/RO261/RO213/RO98 rows (plus RO172 for DQF), and the allowlist takes only three-character codes whose tail is letter-only or zero-only. No 513 nhs-body verdict can be keyed on X24. A 362 merge verdict needs a shared-key group, and these orgs share no key.
4. **ODS does not relate the regions to X24.** RO209 regions have parents []. RO210 local offices point RE5 to the old letter regions (A/B/D/E/F/H/K) and RE2 to Y5x/Y6x. Only X2407 has RE6→X24; X26 and XDH04 point to XDH (DHSC). A 519-style dated parent map cannot be built from ODS for these codes.

How publishers name themselves (bounded `/v1/sql` reads):
- **PPON org:** 1,831 × "NHS England" and 33 × "NHS England, Leeds". No regional names. One stray "Portsmouth Hospitals University NHS Trust".
- **Name cluster 3845883:** 3,377 × "NHS England", no identifier.
- **X24 org:** 65 × "NHS ENGLAND", plus about 20 regional names ("NHS England North West", "…North East and Yorkshire", "…Health and Justice"). Regional teams publish under X24.
- **Y56 org:** 55 × "NHS England" and 13 × "(London Region)". About 10 are London ICBs (North East London 4+2, South East London 2+2), which is mis-coding.

The coded family is larger than 24/383. In 519's cache, England-named coded orgs come to 34 / 407 mentions; the extra ones include:
- 03Q, 14N, 14J;
- the typed codes Q75Q83Q84 (5) and Q75Q84 (1);
- Y63-typed (9);
- A3HOT (3);
- RW5NC (11): "NHS England North West Health & Justice Team", on a Lancashire trust site code.

The mis-coded Y5x/Y6x holders are 11 orgs / 32 mentions. These include an org of 17 mentions named "NHS Lancashire and South Cumbria ICB" under a typed Y62 code, and the South and West Yorkshire ICBs under typed Y63 codes.

One citation is off. The issue says mis-coded codes go to 452 `wrong` under "519 ruling 8", but that ruling covers NHS→NHS typos. 519 decision §4 sends real codes on the wrong holder (Y54/Y61/Y62) to "317 rehoming or leave".

**First unit (M):** Decide and measure (no code). Write `.scratch/tender-db/523-nhs-england/decision.md`:
1. **(a) vs (b), with reasoning.** NHS England is one statutory body, and its regions and local offices are not legal entities. Regional teams already publish under X24, and the PPON is one per legal body. So the recommendation is (a). Folding a region needs a reviewed list, because ODS has no RE relation from RO209/RO210 to X24.
2. **Phase A scope and survivor.** The national six fold into PPON org 30913629. X24 org 30929530 and Y56 org 30914489 fold in as identity losers, together with clusters 3845883 and 4916125. Before that:
   - Y56's London-ICB mentions are rehomed to their ICB survivors with a 317 cohort;
   - the stray "Portsmouth Hospitals" mention on the PPON org is rehomed the same way;
   - 31603756 (GBCOHY56) gets a 452 `wrong`.
3. **Mechanism choice.** Either a one-row RO162 national-body entry for X24 in the frozen table plus allowlist, sharing 519 U6's identity-body build with its PPON tier, or another reviewed-merge path. Name the code sites: `nhs_ods.rs` allow rules, `nhs_body_survivor` at `canonical.rs:6973`, and the identity-loser branch.
4. **Phase B measure.** Page `name_prefix=NHS England` to the end (0.5 s pace). Read the mention names of the top ~30 orgs and the 34 coded orgs. Mark exclusions: the PSR and Patient Choice panels (483), and CSU-agent names ("through Arden…", "trading as NHS South, Central & West CSU").
5. **Mis-coded rows.** The Y54/Y58/Y60/Y61/Y62 holders and the typed Y62/Y63 ICB orgs (11 orgs / 32 mentions) go to a 317 rehoming list per 519 §4, not 452 (the issue's "ruling 8" citation is wrong).
6. **Coordination with 469 U7.** Rewrite the Verify to name 30913629, and record whether 469 U7 or 523 owns the X24↔PPON pair.

**Value:** Who is wrong today: anyone following NHS England.
- **Readers, by key:** by PPON (GB-PPON-PDPT-3135-BWWY) a reader gets 1,866 of the 5,474 mentions on the six orgs named "NHS England" (34%). By `?identifier=GBNHSX24` they get 95 (1.7%). The 3,395 name-only "NHS England" mentions (org 3845883) are reachable by neither key.
- **Name search:** the first 3 pages of `name_prefix=NHS England` alone return 300 orgs / 7,430 mentions, and more pages follow.

Rows involved:
| group | orgs | mentions |
|---|---|---|
| national core | 6 | 5,474 |
| England-named coded region/office/site orgs | 34 | 407 |
| name-only regional clusters (first 3 pages only; incl. one 609-mention review panel that should stay out) | ≥276 | ≥1,793 |
| mis-coded Y5x/Y6x holders (trusts, a surgery, ICBs) | 11 | 32 |
| London-ICB mentions inside the Y56 org | — | ~10 |

Why it matters downstream: 469 U7 would pair one org per NHS body. NHS England is the largest buyer in the NHS family on this measure and stands as hundreds of orgs.

**Overlap:** Not a duplicate of any open issue. Overlaps:
1. **519 U6 (identity bodies for digit parents, still a pending decision).** This is the same missing mechanism. X24 is a digit-identity parent with a standing PPON org beside it, the shape 519 notes for QE1 ("U6 needs a PPON tier"). Build them together or share the code.
2. **469 U7 (digit-bearing GB-NHS ↔ PPON pairs).** X24↔PDPT-3135-BWWY is that shape. But U7's ODS confirmation reads 513's frozen table, which has no RO162 row, so U7 would deny the pair. Decide which issue owns X24, so the two do not race or leave the pair to neither.
3. **483 (contractor or review body tagged as buyer).** "NHS England Independent Patient Choice and Procurement Panel" (609) and "NHS England (PSR Review Panel)" (17) are review panels; keep them out of the NHS England fold.
4. **317 rehoming and 452 identifier verdicts.** The mis-coded Y5x/Y6x holders and the London ICBs inside Y56 go to 317; GBCOHY56 goes to 452.
5. **524 (served name is the first-seen spelling).** Minor: the survivor's head name.
6. **513 and 519 phase A.** Done or applied. 519's `child_check` already refuses NHS England names into ICBs (`Refusal::Nhse`, `crates/ingest/src/nhs_ods.rs:347`), so nothing there conflicts with this issue.

**Rank:** 4/10 across 520/521/523/524. The user-visible harm is real and high-profile. NHS England's mentions are split: 5,474 sit on six orgs named "NHS England", and a lookup by PPON reaches only 34% of them. The triage enlarges the issue's own 24 orgs / 383 mentions mostly by adding the national name cluster.

The value is overstated in parts. The 3,395 name-only mentions are the generic shape of a provisional name cluster beside a PPON org, not something specific to NHS England. The ≥276 regional clusters / ~1,800 mentions count only if option (a) wins, and option (b) keeps regions apart.

The next unit is M and decision-only, and nothing can ship from it soon:
- the fold needs a mechanism that does not exist (X24 is RO162 and digit-bearing, so it is neither in nhs-ods.tsv nor on its allowlist, and ODS links no region to X24);
- that mechanism is shared with 519 U6, which is still an open decision;
- the 317 rehomes for the London ICBs and Portsmouth must come first;
- it has to be coordinated with 469 U7, which waits on 519 M6.

There is no race. U7's ODS check has no RO162 row, so U7 safely denies the X24↔PPON pair. Doing the M decision now risks it going stale before the 519 U6 decision it depends on.
