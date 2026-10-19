# 548 — the head refresh's holds block mostly right changes (consortium, family, gb_family, part of initialism)

Status: ready-for-agent — U1b + U2 DONE 2026-10-19 (548-triage/u1b/, u2/decision.md; adversarial re-run: all outputs reproduce byte-identically). U1b: T1's release arm is 1,318 of 2,425 top_stale holds (0 reverts: top never printed before the head; 3 same-day ties). The salted 30 ('548-u1b-') read 13R/15N/2W and FAILS the 1-wrong bar. The wrongs are 9168556, which adds a 'Partner konsorcjum' prefix, and 17269219 Mithieux TP -> MITHIEUX, a same-entity downgrade labelled like Brlic. The fail margin is that one judgment call. T1 is not adopted. 'T1 + a role-marker-added serve hold' passes only in-sample and needs a fresh dated 30. Votes per notice was measured (q03/q04). It blocks no recorded wrong on the release sets (gb 2R of the 3 reproduced, consortium 4R + 4 unread, T1 2R + 1N) and neither T1 wrong, so it is not adopted. U2 decisions: gb_family RELEASE at H<=1 (not whole: the H>=2 evidence is n=2, and family's H>=2 class holds all 4 of its wrongs). family RELEASE at H<=1. consortium_removed RELEASE on the token-subset test (crosswalk legal-form tables). Caveat: 34 of the 56 subset rows that land in head_current age out of it within 365 days and would then ship unread, so re-read them before they do. KEEP the serve holds, head_current, nhs_check, placeholder_id and top_stale. R2 stays planner-only. Mojibake 22435268 is its own R0-path unit. U3 by_hold predictions (full listing): gb_family -7 (-10 if whole), family -36, consortium_removed -179, head_current +64, top_stale +23; holds 11,652 -> 11,517; R1 changes up to +106 (5/22/79), R2 +29 (excluded on wet runs). 15783942 moves family -> consortium_removed if family ships alone. NEXT: U3 builds v13 (524u2-v13) with these predictions. Was: ready-for-agent — U1c DONE 2026-10-19 (548-triage/u1c/: subset.py, verdicts.json; adversarial check reproduced byte-identical). The consortium_removed token-subset test is now written down: tokens(new) minus legal forms must be non-empty and a subset of tokens(old). The tokenizer folds diacritics, splits on non-alphanumerics and joins single-letter runs; the legal forms are crosswalk's family_token keys plus R2_LEGAL_FORM_TOKENS. It matches 79 shippable rows (1,716 mentions) and 179 full-listing rows (153 R1 + 26 R2; 181 with sau/co). CPR is not matched. Salted 30: 29R/1N/0W, so the release passes the 0-wrong bar; 34 of the 79 now have recorded verdicts (33R/1N). Uncertain ids: 11026616 R (rename to Spring North), 9806693 R (3 of 4 mentions print KAPECI; the SIREN's owner is inferred, not confirmed), 10878002 N (casing). U3 by_hold drops, full listing: gb_family −10, family H<=1 −36 (33+3), consortium subset −179; 15783942 crosses from family to consortium if family ships alone. Caveat for U2: the 74 subset-matched rows now behind head_current (56) or top_stale (18) are unread and would ship without verdicts if those guards are also released. 53 public GETs, 0 /v1/sql. NEXT: U1b (T1 dated 30), then U2 decides (consortium subset now adoptable), then U3 builds v13 (compute gb_family's shippable 7 rather than hard-coding it; port the fold with fold_diacritics, not NFKD). Was: ready-for-agent — U1 DONE 2026-10-19 (548-triage/u1/judgements.json; the counts reproduce). Shippable / held: gb_family 7/10, family 33/79, consortium_removed 109/246. nhs_check and placeholder_id ship 0, because head_current re-holds them. Findings: - gb_family: 7/7 right (5 confirmed by CH register names). Release, recording whether the release is whole or limited to D3's H<=1 stale stray. - family: 29R/4W, all 4 wrongs H>=2. Release at H<=1: 22/22 R, 462 mentions. - consortium_removed: salted 9R/1W (CPR 18287356, wrong). The token-subset candidate is 79 rows only when legal-form tokens in the new name are ignored. Only 7 of the 79 have recorded verdicts (7R); the rest were skimmed, not recorded. Release is NOT yet adopted: write the test down, then record verdicts for the 79 (or a salted 30 excluding 524 skip-ids). - Serve holds and head_current (dated 30: 20R/7N/3W): keep. The serve holds were read on a salted 10 each, below U1's all-if-<=60 / 30 plan; that is enough for a keep. seat_dropped already holds the generic-name shape (5350675). - R2 stays planner-only (D1). - Mojibake 22435268 needs a defective-head (R0) path, because head_current and exclude_released unfit block it. - Votes per notice would stop 2 wrongs (Barameda, CPR); still to be measured. U3's by_hold check must predict full-listing drops, not shippable counts: gb_family -10, family(H<=1) -36, consortium subset about -181 (all R1+R2 rows leave the reason). NEXT: U1c (consortium subset: codify the test and record verdicts), U1b (T1 dated 30), then U2 decides, then U3 builds v13. Was: ready-for-agent — **U0 DONE 2026-10-19 (blocker cleared)**: the stored `org-name-refresh-plan` was read by the orchestrator (`/admin/reports/org-name-refresh-plan`, computed 2026-10-16 23:03Z, epoch 524u2-v12, scope_ids null, rev 3475272, population 2,102,132 orgs / 46,130,516 mentions, `held` 11,652 rows, held_truncated false) and saved as `548-triage/org-name-refresh-plan-2026-10-16.json.gz`; unit 1 runs on it. Triaged 2026-10-19 (.scratch/tender-db/548-triage/notes.md). BLOCKER, retry it: unit 1 needs the full `held` listing. It exists only in the stored `org-name-refresh-plan` report, and `reports` is not on /v1/sql. This session's classifier refused the `aj.sh /admin/reports/org-name-refresh-plan` read ('Production Reads'). U0 must also confirm that the report is an unscoped v12 plan (org_ids null), or run a fresh full dry plan. Found so far: (1) The 553 counts are rule v5's. A released bind-guard hold ships today only if: its rule is R0e, R0n or R1 (R2 and R0w are excluded); its release is not `unfit` (exclude_released); and, for R1 only, its head is older than 1,095 days or it has a `digits` release, and top_stale does not fire. Unit 1 filters `held` to that subset before judging. (2) initialism is R2-only and R2 is withheld, so it is decided moot. R2 stays planner-only; that is recorded as D1. (3) Both gb_family exhibits read the other way round from the issue: a single stale subsidiary or sibling print is the head on the register entity's own CH number. 11628199 has 3 of 4 votes and 10213161 has 49 of 51; both changes are right and would ship. (4) top_stale variant T1 (hold only when rival_newest > top_newest) was tested in-sample on gate (d)'s 20 holds. It releases 12 (5 harmful, 6 neutral, 1 same-entity downgrade) and keeps 7301922 (a real rename) and 8291325 (another entity) held. NEXT: U0, then U1 on the filtered listing; U1b, a fresh dated 30 from T1's release arm; and the classes routed to unit 1: a head_current 30-sample, mojibake before the serve holds, a generic-name signal, and votes per procedure. Was: needs-triage — filed 2026-10-15 from the judgement of 524's dry plan 2515 (workflow wf_f9b41c3f-643); 524's v8–v10 re-judges added the R2 withdrawal, generic names and a mojibake serve-hold harm (below). Next:
measure each hold's right/wrong split on its full listing (`held` in `org-name-refresh-plan`), then decide per hold.
Kind: data quality (served organization names; tuning)
Relates to:
- 524 (unit 2's holds: `crosswalk::head_change_hold`, `head_r2_hold`; unit 2c's weekly upkeep will re-meet them);
- 456 (5778's stray tribunal head, kept by design);
- 523 (NHS bodies).

## Verify

    ssh root@zebreus.click "/root/aj.sh /admin/reports/org-name-refresh-plan" | python3 -c "import json,sys; b=json.load(sys.stdin)['body']; b=json.loads(b) if isinstance(b,str) else b; print(b['epoch'], b['holds'], {k: v for k, v in b['by_hold'].items()})"

- **done**: the hold counts are lower, by the classes unit 2 decides to release, under the next epoch. Or this issue
  records that each over-block is accepted, with its reason.
- **open (2026-10-15, plan 2515, `524u2-v5`)**: `holds` 553: consortium_removed 251, family 84, gb_family 13,
  initialism 34, nhs_check 1, no_fit 170.

## What was found

The holds are safe: no held org is served a wrong name. They are not precise, though. On plan 2515's uniform hold
samples, the judge counted these holds as blocking a change that would have been RIGHT:

| hold | blocks a right change | examples |
| --- | --- | --- |
| consortium_removed | 9 of 12 | head `… (mandataire)`, `ARGE …`, `Titulaire 1: …`, `Chef de file`; 57-92% of votes name the holder alone (CGI Luxembourg S.A., Terhalle Holzbau GmbH, B2M Baumanagement GmbH) |
| family | 9 of 12 | a brand token read as a legal form (`Energie AG Kolín a.s.` → AG, `K-BV Development GmbH` → BV); a real conversion of the same register entry (Swietelsky m.b.H. → AG at 66/78; Vodos s.r.o. → Energie AG Kolín a.s. 19/20); a stray wrong-entity head (Toscana Bus S.r.l. on AVR S.p.A.) |
| gb_family | 10 of 12 | a parent PLC head on a subsidiary's CH number (CH 02081330 Capita plc → Capita Business Services Ltd; 04958135 Scotia Gas Networks Ltd 49/51) |
| initialism | 10 of 30 | right holds: CDC Terroir de Caux, CCI Nantes, OPH, ANER, BSO. Wrong holds: the listed ids 22536848, 10821179, 10078262, 30976129, 15430560, 10080241, 18288500, 19412534, 9705972, 2358700 |
| no_fit | 1 of 12 | 10829487 `vivida bkk` (a 2021 merger; the brand is lower-case by design) |
| keep:bar | 3 of 12 | 6071658 EPF (57%, 2x fails 37 < 50), 10982286 Hydria (65%, 41 < 44), 5778 (456, kept by design) |

The first three over-block about 75% of the time, so roughly 270 of their ~350 orgs keep a worse head. All three
are change holds (`head_change_hold`), applied after a rule has already passed the vote bar.

## Units

1. **Measure (S).** Judge each hold's FULL listing (`held`, ≤ 100,000; 553 today) per reason, not 12 samples.
   Record the right/neutral/wrong split per reason and per sub-shape. Sub-shapes: for consortium, a role marker
   vs a real consortium; for family, a brand token vs a conversion vs a stray entity; for gb_family, a parent vs a
   subsidiary.
2. **Decide (S).** Per sub-shape, release, tighten or keep. Candidates:
   - **consortium_removed:** release when the head's consortium/role marker is a role annotation of the holder
     (`mandataire`, `chef de file`, `Bietergemeinschaftsmitglied`) and the winner is that holder's own name.
   - **family:** read the family on the legal-form tail only, not on a brand token. Release a conversion within
     the same register entry above a higher bar.
   - **gb_family:** release a PLC → Ltd move when the identifier is a Companies House number and the winner holds
     ≥ 90%.
   - **initialism:** check the 10 wrong holds for a shared shape first.
3. **Build** as a new epoch (`524u2-v7` or later), re-plan, re-judge, and apply through 524's runbook. Do it before
   or with unit 2c, so the weekly upkeep does not re-meet the same holds every week.

## Classes 524's later re-judges routed here (rules v8–v10, plans 2523, 2526, 2527)

Unit 1 measures these alongside the holds above. None of them blocks 524; each is a missed fix or a
withheld rule, never a served wrong name.

- **R2, withheld whole (v9's pre-committed fallback, 2026-10-16).** change:R2 failed every dated re-judge:
  2/30, 2/30 and 4/30 wrong (6 of 90 pooled). Every 524 wet run passes `exclude_rules: ["R2"]`. The planner
  still elects and lists R2: plan 2527 had 1,520 changes, 42,203 mentions. Its wrongs are semantic singletons
  that no vote statistic or token test separates from its rights:
  - 4123715: a sub-service for the legal unit;
  - 16508165: one outdated sp.j. spelling for another, because Polish spelled-out forms are not vote-key tokens;
  - 13068036: a slash read as `I`;
  - 10487887: a `Cté de Cnes` abbreviation;
  - 21846426: a sibling office on a shared CH id;
  - 23252614: a co-buyer, which is another entity.

  Unit 2 decides whether any R2 sub-shape can ship, or whether R2 stays planner-only.
- **Generic names (R1).** 5350675 changed `Ośrodek Pomocy Społecznej w Kluczborku` to `Ośrodek Pomocy
  Społecznej`, a bare institution type that 15 organizations carry as their head. Its head is stale (≥ 3 years),
  so v10's `head_current` does not hold it. v9 rejected a `generic` hold because the corpus name count measures
  duplicate rows, not genericness (`Urtica`, `Région Occitanie`). A usable test needs a different signal, for
  example the name being a known institution-type phrase with the place token dropped.
- **Harmful serve hold: mojibake defeats `review_body`.** 22435268's head is `Comit consultatif interrgional du
  rglement amiable`. The hold keeps it because the broken spelling no longer matches the review-like stems.
  Candidate: ask the unfit/mojibake test (including dropped-accent shapes like `interrgional`) before the serve
  holds.
- **Harmful residue hold.** 10662302 keeps a `Sp. zo.o.` typo (v7 re-judge). Accepted at the bar.
- **The Sheffield shape.** An H=1 stray printed newest is held by `residue` (v8). This is a missed fix, never a
  regression.
- **Votes per mention, not per procedure.** Lot repeats can make a majority (`Consultorís`). Count votes per
  procedure, not per mention.
- **`head_current` (v10).** About 6,520 R1 changes become holds on a head printed within 3 years. Its right/wrong
  split is measured on the first v10 plan: a hold sample read the same way as the holds above.

## From 524 rule v11's gate (d), plan 2562 (2026-10-16)

The cost report of wf_46642f82-a94; items in `.scratch/tender-db/524-v11/gates-2562/` (`v11_gate_items.json`,
`rejudge2562_result.json`).

- **`hold:top_stale` (v11), 20 read: 3 right, 8 neutral, 9 HARMFUL.** The hold keeps a better name out in nearly half
  the sample:
  - newer legal names: 9938875 Praxair → Nippon Gases Sverige AB; 10071871 RebelGroup Executives BV → Rebel Strong
    Society;
  - agent tails: 6160661 `Kemira Rotterdam BV (agent voor Kemira Oyj)`;
  - list-number prefixes: 15346526 `199. Zakład …`;
  - other-entity prints: 15082073 (a one-off print of another company), 10878449 (a s.c. partner's name);
  - other better names: 9213677, 13897322 and 7940094.

  The rival's newest print often POSTDATES the elected spelling's (9938875 rival 2026-08-12, 6160661 2026-05-21,
  15082073 2026-09-04), so a stale top with a FRESHER rival is a supersede candidate. A direct test on `rival_newest`
  (the diagnostic v11 stores in every dated entry) is this issue's lead: elect the rival when it is the newest
  printed name and passes the serve holds.
- **`hold:seat_dropped` (v11), 5 read: 3 right, 2 neutral, 0 harmful.** It holds as designed.
- **`hold:doubled` (v11), 5 read (only those whose half differs from the head): 3 right, 1 neutral, 1 harmful.** The
  harmful one is 5857542: a former generic head `Centro Informático Municipal`, where the doubled
  `DonostiaTIK - DonostiaTIK` is the current body's name printed twice. The fix would be to serve the half
  (`DonostiaTIK`), not to hold.

## Triage (2026-10-19, workflow wf_2a43dfc2-d09: a measurer and an adversarial skeptic)

Files: `.scratch/tender-db/548-triage/`. The Status line above is the skeptic-corrected one.

### Measured

The full measurement the issue names as Next could not be run. The `held` listing lives only in the stored report (kind `org-name-refresh-plan` in the `reports` table). `reports` is not on /v1/sql's allow-list (`ALLOWED` in crates/app/src/v1/sql.rs). The one other path, `ssh root@zebreus.click "/root/aj.sh /admin/reports/org-name-refresh-plan"`, is a single primary-key read, but this session's permission classifier refused it ("Production Reads"). I did not retry it or look for a way around it. Rebuilding the holds from mentions would mean walking 5.68M orgs and 49.7M mention rows, which the rules forbid.

What I did instead, within the rules:
(1) Read the code at rev 75d8da3 (epoch 524u2-v12) to see when each hold fires: `elect_head_counted` in crates/store/src/head_election.rs, and `head_change_hold`, `head_r2_hold` and `consortium_name` in crates/ingest/src/crosswalk.rs.
(2) Two public GETs, `/v1/organizations?identifier=02081330` and `?identifier=04958135`, at least 2.2 s apart.
(3) One /v1/sql request (q01): each org's mention spellings with counts and their first and newest print date, for the two gb_family examples, orgs 11628199 and 10213161. I checked the query plan locally on the v551 copy first: an index search on `organization_mentions_org` plus a rowid lookup on notices, in each half of the UNION ALL. It took 1.6 s and returned 8 rows.
(4) An offline test of the issue's top_stale lead, using the 20 top_stale holds already judged in 524's v11 gate (d) (plan 2562 items with head_newest, top_newest and rival_newest). No box requests.
11 of the 12 allowed /v1/sql requests are unused.


### Numbers

CODE (rev 75d8da3, epoch v12). The order in which a verdict is reached:
E1–E3 → R0 → D4 → S1 → bars → recent (window) → residue → serve → head_current → top_stale.
- serve runs three kinds of hold in turn:
  - the bind guards, `head_change_hold`: family, gb_family, consortium_removed, nhs_check, placeholder_id;
  - then, for R2 changes only, `head_r2_hold` (initialism);
  - then the serve holds.
- head_current: an R1 change whose head was printed within the last 1,095 days.
- top_stale: an R1 change with a rival cluster whose top was last printed more than 365 days ago.

What follows from that order:
(a) The issue's 553 holds come from rule v5 (plan 2515). That plan ran before the recent, residue, serve, head_current and top_stale holds were added.
(b) A bind-guard hold that is released only ships if all of these are true:
  - its rule is R1 (every wet run passes exclude_rules R2 and R0w);
  - its head was last printed more than 3 years ago, or the junk exemption releases it;
  - top_stale does not fire on it.
  Every `held` entry carries the fields needed to check this (rule, head_newest, top_newest, rival_newest, clusters, head), so the listing can be filtered offline before anyone judges it. "Roughly 270 of ~350 keep a worse head" overstates what a release would ship today.
(c) initialism holds only R2 changes, and R2 is withheld whole. The 10 wrong initialism holds of 30 could not ship even if released.

PROBE q01: the gb_family examples run the other way round from what the issue says.
- 11628199, CH 02081330 (the register name is CAPITA PLC):
  - Head 'Capita Business Services Limited': 1 print, 2018-06-15.
  - Votes: 'Capita PLC' 2 (newest 2026-09-16) and 'Capita Plc' 1, so the top cluster has 3 of 4 votes (75%).
  - The issue reads this as a parent head on a subsidiary's number. It is the reverse: a subsidiary's single stray print sits on the parent's number, and the held change would serve the register entity's own name.
- 10213161, CH 04958135 (the register name is SCOTIA GAS NETWORKS LIMITED):
  - Head 'Southern Gas Network PLC': 1 print, 2017-08-11.
  - Top: 'SCOTIA GAS NETWORKS LIMITED' 47 (newest 2026-10-02), plus one print each of Ltd and Plc, so 49 of 51.
- In both, the head has one vote and is more than 3 years old, and the top is current. Neither head_current nor top_stale would hold them again, so releasing gb_family ships both changes, and both are right.
- The issue's candidate rule (winner holds at least 90%) would keep Capita held, at 75%.

TOP_STALE, tested offline on the 20 judged holds. Variant T1: hold only when rival_newest > top_newest, i.e. a rival was printed after the top.
- T1 releases 12:
  - 5 where the hold was harmful;
  - 6 neutral;
  - 1 where the hold was right: 5638950 'Brlić d.o.o.' → 'Brlic'. That is the same entity with a worse spelling, not another entity and not a revert.
- T1 keeps 8 held: 4 harmful (9938875, 6160661, 15346526, 15082073), 2 neutral and 2 right.
- So T1 recovers 5 of the 9 harmful holds.
- In both right holds the rival was printed after the top:
  - in 7301922 it is a real rename (electing the rival would be right);
  - in 8291325 it is another entity, Seine et Yvelines Numérique (electing the rival would be wrong).
  So "elect the rival" is not safe as a blanket rule.
- The sample is small, and these 20 ids are already on 524's skip list (ids the gates must not redraw), so a fresh dated sample is needed before anything ships.

BUDGET: 1 /v1/sql request, 2 public GETs, 1 admin read refused.


### Decision (as proposed; amended by the skeptic points below where they conflict)

Decided now on the evidence. The owner records these.

D1, initialism: no change. Its line is closed as accepted and moot.
- `head_r2_hold` runs only on R2 changes (head_election.rs ~776).
- R2 has been withheld whole since v9 (every wet run passes exclude_rules R2).
- So no initialism hold, right or wrong, decides a served name.
- Reopen it only if a unit ever ships an R2 sub-shape.

D2, how unit 1 measures: change the method before judging.
- Judge only the subset of `held` that a release would actually ship: rule R1, AND head_newest before as_of − 1,095 days (or a junk release), AND NOT (more than one cluster beyond the head's own, with top_newest more than 365 days before as_of).
- Report per reason: the size of the shippable subset and its right/neutral/wrong split.
- Reason: since v7–v11 the bind guards sit between the window/residue holds and head_current/top_stale, so the v5 rate (9 of 12 right) measures changes that mostly cannot ship.

D3, gb_family: the sub-shape the issue names is reframed; nothing is released yet.
- Not "parent vs subsidiary". The shape to test is "the head is a single stale stray (head ≤ 1, older than 3 years) and the change serves the top cluster".
- Both exhibits are that shape: a subsidiary or sibling print on the register entity's number. Both changes are right, and both would ship.
- The candidate "winner ≥ 90%" is dropped. It would miss 11628199 (75%).
- The release rule will be chosen from unit 1's measured split. The candidate release is: head ≤ 1 AND head stale AND R1 passed.
- The same stray-head test is the first thing to measure for family (Toscana Bus on AVR S.p.A.) and consortium_removed (single `(mandataire)` heads).

D4, top_stale: adopt T1 as the candidate rule, not yet built.
- T1: hold only when rival_newest > top_newest.
- Measured on 20 judged holds: it fixes 5 of 9 harmful holds, at the cost of 1 same-entity downgrade, and it keeps both cases where the rival is a real rename or another entity.
- "Elect the rival" is rejected as a blanket rule: 8291325's fresher rival is another entity.
- Bar before any build: a fresh dated sample of 30 from T1's release arm, drawn excluding 524's skip list. At most 1 wrong and 0 reverts.

No change to the other holds until unit 1 reports.


### Units (proposed)

- U0 (blocker, S): read the stored `org-name-refresh-plan` with `aj.sh /admin/reports/org-name-refresh-plan`, once, in a session whose classifier allows it. Save the body locally under .scratch/tender-db/548-triage/. Record the epoch, as_of, by_hold and held_truncated. Nothing else can start before this.
- U1 (measure, S–M): filter `held` offline to the subset a release would ship (D2: R1, head older than 1,095 days or a junk release, not top_stale). Count the subset per reason and per sub-shape: family = brand token / conversion / stray; consortium = role marker / real consortium; gb_family = stray stale head vs the rest; head ≤ 1 and stale vs H ≥ 2. Judge each shippable subset with dates: all of it when there are 60 or fewer, else a salted uniform draw of 30, excluding 524's skip-ids.tsv. Record right/neutral/wrong.
- U1b (measure, S): a fresh dated draw of 30 from top_stale's T1 release arm in the same plan (rival_newest <= top_newest or no rival_newest, the 20 gate-(d) ids excluded). Bar: at most 1 wrong, 0 reverts. Also report the arm's size against the 2,425 holds.
- U2 (decide, S): for each reason, release, narrow or keep, based on U1/U1b. Candidates: the stray-stale-head release (head ≤ 1, head_newest older than 3 years, R1 passed) for gb_family, family and consortium_removed; family read on the legal-form tail only (brand token); T1 for top_stale; serving the half for the `doubled` case (5857542). Record each decision and its reasoning in 548 and 524.
- U3 (build, M): build the adopted releases as a new epoch (524u2-v13). Add unit tests per release shape (11628199/10213161 for gb_family, 9213677/7940094 for T1, 8291325 kept held), run ops/check.sh, deploy, then a dry re-plan with a by_hold ±10% check against U1's predicted counts, a dated re-judge, and capped then full wet runs through 524's runbook with exclude_rules R2 and R0w. Do it before or together with 524 unit 2c, so the weekly upkeep starts with the narrowed holds.
- U4 (verify, S): update 548's ## Verify to the epoch-v13 by_hold reading. done = each released reason lower by the shipped count, or each remaining over-block recorded as accepted with its reason.


### Skeptic (amendments)

- The facts check out. q01's 8 rows match the raw body. The epochs convert to the stated dates (head 2018-06-15 and 2017-08-11; tops 2026-09-16 and 2026-10-02). The vote shares are right (3 of 4, and 49 of 51). The local 448/469 Companies House caches give CAPITA PLC for 02081330 and SCOTIA GAS NETWORKS LIMITED for 04958135. Both orgs pass R1 (top_min 3, 55%, at least 2H). `reports` is not in sql.rs ALLOWED. The hold order, HEAD_CURRENT_DAYS = 1,095, HEAD_TOP_STALE_DAYS = 365 and head_r2_hold being R2-only (line 776) all match the code. Re-running top_stale_t1.py on gate-2562 gives 12 released (5 harmful, 6 neutral, 1 right) and 8 held (4/2/2). There was 1 /v1/sql request, and the 2 GETs were 3 s apart. I found no locally saved plan body under .scratch or the scratchpad, so the blocker is real. Keeping ready-for-agent with the blocker recorded follows triage-labels.md.
- The shippable-subset filter in D2/U1 is wrong: it wrongly requires rule R1. The bind guards (`HeadElectFns::hold`, `head_change_hold`) are 'asked of every rule's change'. `serve()` calls them for R0e and R0n too, and wet runs exclude only R2 and R0w. head_current and top_stale apply to R1 only, so a released bind-guard hold on an R0e or R0n change ships with no later hold. placeholder_id is the guard most likely to meet R0 heads. The filter must be: rule ∈ {R1, R0e, R0n}, with the head_current and top_stale cuts applied to R1 only.
- D2 ignores `exclude_released:["unfit"]`. 524 has passed it on every wet run since v12. The window and residue run before the bind guards, so some bind-guard holds already carry released=unfit, and those cannot ship even if released. Also, the head_current exemption '(or a junk release)' ships only on a `digits` release (Junk::Released is only 'unfit' or 'digits'). Unit 1's filter has to drop entries whose release is `unfit`.
- Unit 1 leaves out measurements the issue gives it. The issue says these classes are measured alongside the holds ('Unit 1 measures these alongside the holds above'), yet D-final says 'no change to the other holds until unit 1 reports'. Missing are: the head_current hold split (the largest class, by_hold 6,309, with its dated 30-sample asked for in the issue); the mojibake case where the hold keeps a broken head because the test misses it (22435268, candidate: run the unfit test before the serve holds); a generic-name signal (5350675); and counting votes per procedure instead of per mention. Either add a unit for each, or record each as deferred with its reason.
- Neither D1 nor the units decide the issue's unit-2 R2 question: does any R2 sub-shape ship, or does R2 stay planner-only? D1 only calls initialism moot 'while R2 stays withheld'. Record the decision that R2 stays planner-only, or add a decide unit.
- U0 does not check what the stored report covers. `put_report("org-name-refresh-plan")` is overwritten by ANY dry plan, scoped `org_ids` plans included (supervisor.rs ~14821). U0 has to confirm that the latest report is an unscoped plan at epoch 524u2-v12 (org_ids null). If it is scoped or stale, unit 1 needs a fresh full dry plan (an admin job) before measuring.
- Wording. Status point (4) says T1 'keeps both genuine supersede cases held'. Only 7301922 (VOORMORGEN) is a supersede; 8291325's fresher rival is another entity. The decision text has this right. Also, T1 is not 'the issue's top_stale lead'. The lead was to elect a fresher rival, and T1 does the opposite: it keeps the fresher-rival cases held, including the 4 harmful ones the lead was aimed at. The rejection of 'elect the rival' is still sound, since judges marked the change TO the top harmful there and the rivals are junk, variants or another entity. T1 was also chosen on the same 20 items it was scored on, which the proposal does acknowledge.
- Sizing. U3 is probably L, not M. It covers several release shapes, epoch v13, the gate, deploy, a re-plan with a ±10% check, a dated re-judge, and capped then full wet runs.

## U0 — the held listing (2026-10-19, orchestrator)

The triage's blocker was the agent's own read permission, not the system: the orchestrator read
`/admin/reports/org-name-refresh-plan` directly. It is the unscoped v12 plan (`scope_ids` null, `epoch` 524u2-v12,
`dry_run` true, `stopped`/`truncated` false, `listing_cap` 100,000), computed 2026-10-16 23:03Z at rev 3475272.
`held` = 11,652 rows (not truncated) by reason: head_current 6,330; top_stale 2,425; residue 806; organ 460; recent 378;
caps_variant 290; consortium_removed 246; no_fit 157; review_body 103; doubled 91; address_added 85; code_added 81; family 79;
seat_dropped 68; initialism 32; gb_family 10; placeholder_id 10; nhs_check 1. Saved gzipped as
`548-triage/org-name-refresh-plan-2026-10-16.json.gz` (each row carries rule, reason, head_newest, rival_newest, clusters,
spellings, release fields). Unit 1 filters it with the skeptic's corrected subset (rule ∈ {R0e, R0n, R1}; release not
`unfit`; head_current / top_stale cuts for R1 only) before judging.

## Unit 1 — the held listing judged (2026-10-19, workflow wf_ceb29935-046)

Files: `548-triage/u1/` (filter.py, judge.py, judgements.json, hc_sample.json, api/).

I ran the U1 measurement on the held listing of the 2026-10-16 plan (epoch 524u2-v12). All files are in /home/user/tender-db/.scratch/tender-db/548-triage/u1/:
- filter.py: makes shippable.json and samples.json
- judge.py: writes judgements.json, which holds every verdict, the per-hold proposals and a meta block
- hc_sample.py / hc_sample.json: the dated head_current 30
- q02_votes_per_notice.sql and .plan.txt; the output is ../q02_votes_per_notice.out.json
- api/: the 20 public GET bodies

**Filter (the skeptic's corrected subset).**
- No row in `held` has rule R0e or R0n, and none has release `unfit`. So the subset comes down to: rule R1, the head not printed within 1,095 days, and top_stale not firing.
- The `digits` release is not modelled.
- A released bind guard still meets head_serve_hold before it ships, so every ship count below is an upper bound.

**Bind guards (the five that `head_change_hold` returns).** Counts are held rows, then shippable rows.
- **gb_family**, 10 → 7. All 7 read right. The local Companies House cache confirms 5 of them: Bethell Construction, EDF Energy Customers, Morgan Sindall C&I, Capita plc, Scotia Gas Networks. **Release.**
- **family**, 79 → 33. All 33 read: 29 right, 4 wrong. The salted 10 were 9 right, 1 wrong.
  - Every wrong has 2 or more head votes:
    - 3728305 and 10037554: language-annotation tails on the new name;
    - 7081895: a letter-spaced `A B A U T O B O S S`;
    - 20415415: the Lithuanian UAB Barameda on an Austrian VAT number, with all 7 votes from one notice.
  - Rows with a head of at most 1 vote: 22 of 22 right.
  - **Refine:** release only when the head has at most 1 vote (22 orgs, 462 mentions).
- **consortium_removed**, 246 → 109. Salted 10: 9 right, 1 wrong. I also skimmed all 109.
  - The wrong is 18287356: `CONSORTIUM DES PEINTURES ET REVETEMENTS` → `SARL CPR`. "Consortium" is the company's own name and the new name is an initialism.
  - Three are uncertain: 11026616, 9806693, 10878002.
  - Every row where the new name's tokens are a subset of the head's read right.
  - **Refine:** release on that token-subset test (79 orgs, 1,716 mentions). The alternative is to release when the head has at most 1 vote (88 orgs), but that lets CPR through.
- **nhs_check**, 1 → 0, and **placeholder_id**, 10 → 0. head_current would re-hold all of them. **Keep.**

**Serve holds (the task listed them too).** Counts are shippable rows, then the salted read.
- **Keep:**
  - organ: 188; 9 wrong, 1 neutral.
  - code_added: 46; 6 wrong, 4 neutral.
  - address_added: 38; 9 wrong, 1 neutral.
  - review_body: 8; 8 wrong.
  - caps_variant: 198; 4 wrong, 4 neutral, 2 right.
  - doubled: 80; 10 wrong as served. Serving the half instead would differ from the head in 30 of 80, mostly Basque→Spanish swaps, which are neutral.
  - no_fit: 60; there is no fit spelling to serve.
- **seat_dropped**, 38: 5 wrong, 5 neutral. **Keep.** It already acts as the generic-name guard:
  - At v12, 5350675 is held by seat_dropped.
  - Public `name_prefix` lookups count how many orgs are already served exactly the shortened name:
    - Ośrodek Pomocy Społecznej: 16
    - MOPS: 23
    - OSiR: 7
    - ZST: 4
    - BWA: 4
  - For names that really are specific the count is 0 to 3.

**Other items.**
- **head_current, dated 30.** 20 right, 7 neutral, 3 wrong:
  - 11059191: drops `ehf`;
  - 22627722: adds an org number;
  - 22667102: a buyer's name on a supplier org.
  Each wrong is a different shape and 3 of 30 fails the 1-wrong bar, so **keep**. The hold blocks about two thirds right changes. If released, 6,083 of its 6,330 would ship and 247 would meet top_stale.
- **Mojibake case 22435268** (held as review_body). Fixing review_body alone does not ship it:
  - Its head was printed 2024-02-09, so head_current re-holds it.
  - If the unfit test learned the dropped-accent shape, the result would be an `unfit` release, and wet runs pass exclude_released unfit.
  - It needs an R0-style path for defective (mojibake) heads.
- **Votes per procedure (q02).** Barameda's 7 mentions come from 1 notice, against the head's 2 notices. SARL CPR's 2 mentions come from 1 notice. Counting votes per notice would block both wrongs.
- **R2 stays planner-only (D1).** No R2 sub-shape ships.

**Budget used:** 1 /v1/sql request (plan-probed locally first), 20 public GETs at least 2.2 s apart, no admin reads, no writes, no code edits, no git.

### Decision (as proposed)

Release gb_family. Narrow family to a stray head (H≤1, stale). Narrow consortium_removed to the token-subset test, where the holder's own name is in the head. Keep nhs_check, placeholder_id, organ, code_added, address_added, seat_dropped, review_body, caps_variant, doubled, no_fit and head_current. R2 stays planner-only. 22435268 goes to an R0-style mojibake defective-head path, not a reordering of review_body. Count votes per notice or procedure as a separate candidate.

### Check (amendments, adopted)

- REPRODUCED. I re-ran filter.py and judge.py in the scratchpad: shippable.json and samples.json come out byte-identical, and every per-hold count and tally in the result matches. That covers family 33/79 (29R 4W, salted 9R 1W, H<=1 22/22R over 462 mentions), gb_family 7/10, consortium_removed 109/246 with a salted 9R 1W, all the serve-hold splits, head_current 20R/7N/3W with 6,083 of 6,330 shippable and 247 meeting top_stale, and nhs_check/placeholder_id at 0. No R0e/R0n row and no `unfit` release exist anywhere in the 11,652 held rows. q02's 39 rows back the per-notice claims: Barameda has 7 mentions from 1 notice against the head's 2 from 2, and SARL CPR has 2 mentions from 1 notice. The CH caches give the 5 register names cited. The budget is as stated: 20 GETs at 2.2 s or more apart, and 1 /v1/sql request with the index-only plan probed first. I checked the filter's day-granularity cut-offs against head_election.rs (as_of 1792191671): 4 rows sit within a day of either cut-off, and none of them changes a count.
- OVERSTATED COVERAGE on consortium_removed. Both the status line and the decision say the token-subset rule covers '79 orgs, all read right'. judgements.json holds 10 consortium verdicts, and only 7 of them fall inside the 79-row subset. The other 72 rest on a 'skim' that no file records, and the 3 'uncertain' ids (11026616, 9806693, 10878002) have no recorded verdicts either. The rule was also shaped after seeing the one wrong (CPR 18287356) and was scored on the same rows, so it has no out-of-sample read. My own skim of the 79 found no clear wrong, so the rule may well hold. Even so, '79/79 R' is not a measured number.
- THE RULE AS WORDED DOES NOT GIVE 79. A literal subset test ('the new name's tokens are a subset of the head's') gives 70 to 73 rows and 1,589 to 1,661 mentions, depending on the tokenizer, plus 56 rows on whitespace-split norms. 79 rows and 1,716 mentions reproduce only when legal-form tokens in the NEW name (SARL, SAS, SA, GmbH, Ltd, ...) are ignored. Examples are 21104135 `FRANCISCI TRANSPORT SARL` and 21480661 `SAS CHUDEAU`. No script records the test. U3 would build from a rule whose exact definition, including the legal-form list, is unwritten.
- DEPARTS FROM THE U1 METHOD WITHOUT SAYING SO. The U1 unit says: judge all of a shippable set of 60 or fewer, otherwise a salted 30, excluding 524's skip-ids.tsv. The worker drew 10 per class instead. code_added (46), address_added (38), seat_dropped (38) and no_fit (60) should each have been read in full. consortium_removed (109), organ (188), caps_variant (198) and doubled (80) should have had 30 each. Skip-ids were not excluded, and 10379298 (code_added) is on 524-v11/skip-ids.tsv. This does not change any KEEP, since those samples are 0 to 2 right of 10. It does weaken the consortium RELEASE, which rests on a salted 10 plus the unrecorded skim.
- THE U3 by_hold PREDICTION IS WRONG IN KIND. A release takes EVERY row matching the predicate out of that hold reason. Rows that are not shippable move to head_current, top_stale, a serve hold or an R2 change, so they also leave the reason. The expected drops are therefore: gb_family -10 (all of it, not -7); family at H<=1 -36 (33 R1 + 3 R2, not -22); consortium token-subset about -181 over the full listing (155 R1 + 26 R2 with the legal-form-free tokenizer, not -79). A ±10% by_hold check against -7/-22/-79 would fail on a correct build. The shippable counts belong in the predicted changes column, not in the hold drops.
- MINOR: the meta block says 'digits release not modelled', but filter.py does honour recent_released. Only 1 row carries it (12523760, no_fit), so nothing changes. 'R?' verdicts are counted as R: 14721467 in family (H=3, inside the kept-held H>=2 group, so it does not touch the 22/22) and 22671320 in head_current (20R includes 1 R?). The MOPS 23 and OSiR 7 counts are case-insensitive; exact matches are 21 and 6. In q02, CPR's top cluster is 3 votes (SARL CPR 2 + CPR 1, from 2 notices), not 2. Per-notice votes still block it (2 < top_min 3).
- JUDGEMENT RISK, not an error. Releasing gb_family WHOLE rests on all 7 shippable rows, 2 of which have H>=2 (13448374 H=7 and 9087207 H=2). The triage's D3 candidate was the narrower release: H<=1, head stale, R1 passed (5 rows). A whole-guard release also stops this guard firing on every future weekly-upkeep row. Either form is defensible on 7/7, but the decision should say why it goes beyond D3.
- WHAT FOLLOWS. Keeping head_current, the serve holds, nhs_check and placeholder_id, the family H<=1 release (22/22 recorded), R2 staying planner-only, and the mojibake R0-path finding all follow from the files. For mojibake, 22435268's head_newest of 2024-02-09 falls inside 1,095 days, so head_current re-holds it, and an unfit release is excluded on wet runs. The consortium release should stay a candidate until the 79-row subset's test is written down and read on recorded verdicts, all 79 or a salted 30 excluding skip-ids. U3's by_hold predictions need correcting to the full-listing counts.

## U1c — the consortium token-subset test codified and read (2026-10-19, workflow wf_fde95594-dbf)

U1c is done. The test is now a script and the release passes the 0-wrong bar on a recorded salted 30: 29 right, 1 neutral, 0 wrong. Files are in /home/user/tender-db/.scratch/tender-db/548-triage/u1c/: subset.py, subset.json, verdicts.py, verdicts.json, fetch.py and api/ (52 public GET bodies).

**The test, as subset.py writes it down.** A row held as consortium_removed is released when tokens(new) minus LEGAL_FORMS is non-empty and is a subset of tokens(old).
- Tokenizer steps:
  - lower-case, then fold diacritics (NFKD with combining marks dropped, plus ł ø đ ß æ œ ı þ ð);
  - split on anything that is not a letter or digit (underscore is a separator too);
  - join each run of single-letter tokens into one token, as crosswalk::folded_family does (S.A. → sa, S. L. U. → slu, A/S → as).
  - Legal forms are dropped from the new name only.
- LEGAL_FORMS is copied word for word from crosswalk.rs: family_token's keys plus R2_LEGAL_FORM_TOKENS. U3 can reuse those tables in Rust rather than add a new list.
- A wider list (sau, co, lda, sprl, bvba, cvba) is reported for sensitivity only.
- The script re-applies U1's ship filter and asserts that it reproduces u1/shippable.json exactly.

**Matches (held listing of 2026-10-16, consortium_removed: 246 held, 109 shippable).**

| | Shippable R1 | Full listing (R1 + R2) |
|---|---|---|
| Code legal-form list (the test) | 79 orgs, 1,716 mentions, 67 with H<=1 | 179 orgs (153 R1 + 26 R2), 2,769 mentions |
| Plus the wider list | 79 (unchanged) | 181 (155 + 26); the extra 2 are 10481809 (sau) and 22572058 (co) |
| No legal-form drop | 76 shippable | 168 |

- The full-listing 179 breaks down as 79 ship, 56 move to head_current, 18 to top_stale and 26 are R2.
- CPR 18287356 does not match (its leftover token is cpr).
- None of the 79 is on 524-v11/skip-ids.tsv.

**Verdicts (79 > 60, so a salted 30 with salt '548-u1c-', sha256 ascending).**
- 29 right, 1 neutral, 0 wrong. That passes the bar.
  - The neutral is 15497200: Groupement de coopération sanitaire (GCS) Achats du Centre → GCS Achats du Centre. The head is the body's full legal name, not a bidder group, and the new name is the short form 177 of 178 mentions print.
  - 23 of the 30 have H<=1; 7 have H>=2.
  - 3 of the 30 (8346204, 14259882, 15165788) were already R in U1, so 27 reads are new.
- Counting U1's 4 other recorded rows inside the 79, 34 of the 79 now have recorded verdicts: 33 right, 1 neutral.
- Evidence:
  - All 30 org GETs show the held head is still the live served name, with the same identifier and mention count as the listing.
  - 19 rows only drop a role annotation such as "(mandataire)" or "(as consortium member)".
  - The rows where the new name is one member of a group were checked against name_prefix GETs, which find the new name served on the same SIREN or identifier:
    - Guintoli (8217358, 11219959): 10135477 on SIREN 447754086
    - AEI (17144366): SIRET 41498295900036
    - Kardham (17308510): FR12384407896 and 38440789600198
    - AVR Ingénierie (16969861): 48811993400021
    - SATT AxLR (14493390): the same SIRET
  - For S Hoch 2 ZT GmbH (18124034), Val de Seine Aménagement (10266767), L'Eclaircie (18181616) and SEGC Topo (9225319), the new name appears in every mention on that identifier, or in most of them.

**The 3 'uncertain' ids.** None of them matches the test; all three are shippable rows the test leaves out.
- **11026616 — R.** It is a rename. The 2018 head is "Families Health & Wellbeing Consortium". Every FTS notice from 2024 to 2026 is won under Spring North on CH 07797133, and org 31604091 "Spring North" carries GBCHC07797133 (the same number). "Consortium" was the holder's own name: the same shape as CPR, but here the new name is the renamed entity, not an initialism.
- **9806693 — R.** The 2017 head is "Entreprise PORALU (mandataire)". The 3 other mentions are 2025 eForms notices that print KAPECI. Poralu Menuiserie is SIREN 418245262, not this org's 418245437, so the identifier is KAPECI's.
- **10878002 — N.** It is the right firm, but the elected display spelling has broken casing: "sas cabinet kerguen - mANDROIT".

**Correction to U3's by_hold predictions.** These are drops over the full listing; the shippable counts belong in the predicted-changes column.
- gb_family released whole: −10 orgs / −209 mentions, the whole reason. 7 of them are shippable changes.
- family at H<=1: −36 (33 R1 + 3 R2) / −680 mentions, out of 79. 22 are shippable.
- consortium subset: −179 (153 R1 + 26 R2) / −2,769 mentions, out of 246; −181 with the wider list. 79 are shippable.
- One row crosses guards. 15783942 ("Consortium consisting of Kontron …" → "Kontron Transportation GmbH", H=1) would move from family to consortium_removed if only the family release ships. It matches the subset test, so it leaves both reasons if both releases ship.
- I checked this with consortium_name's exact token list: it is the only family-H<=1 or gb_family row that crosses.

**Budget.**
- 0 /v1/sql requests and no box access.
- 52 public GETs to tenders.zebreus.click, at least 2.2 s apart, all returned 200 (no 408s). They are logged in ../api_requests.log, tagged (u1c).
- I made 2 attempts at the public SIRENE API (recherche-entreprises.api.gouv.fr). Both got connection-reset through the proxy, so no data came back and SIRENE was not used as evidence.
- No writes, no code edits, no git.

### Next units (proposed)

- U1b: head_current T1 dated 30 read (per U1 plan)
- U2: decide releases — gb_family (whole vs D3 H<=1), family H<=1, consortium token-subset (now 29R/1N/0W recorded), keep serve holds/head_current/nhs_check/placeholder_id
- U3: build v13 with by_hold predictions gb_family -10, family -36, consortium_removed -179 (code legal-form tables reused from crosswalk.rs), noting the 15783942 family->consortium cross-guard
- Optional: votes-per-notice candidate (Barameda, CPR) measured separately

### Check (sound)

- Minor count error in the result body: there were 53 public GETs to tenders.zebreus.click, not 52. api_requests.log has 53 zebreus lines tagged (u1c), with no duplicates, plus the 2 SIRENE attempts, 55 lines in all. u1c/api/ holds 53 bodies (15 np_, 33 org_, 3 t_, 2 td_). The rate rule was kept: the log rounds to whole seconds and its smallest gap is 2.0 s, consistent with fetch.py's 2.2 s sleep. No 408s.
- Everything reproduces exactly. I copied subset.py and verdicts.py to the scratchpad with only the output paths changed. Both exited 0 and their output is byte-identical (cmp) to u1c/subset.json and u1c/verdicts.json. The shippable.json assert holds. Every number checks out: 246/109, 79 orgs/1,716 mentions/67 with H<=1, 179 = 153+26 / 2,769 mentions, the 79/56/18/26 split, 181 with the wider list (the extras are 10481809 S.A.U and 22572058 & Co. KG), 168/76 with no legal-form drop, CPR not matched, none on skip-ids, 29R/1N, 23 with H<=1, and 34 recorded = 33R/1N. The 34 is the 7 U1 rows inside the 79, 3 of them in the salted 30. The by_hold totals match len(held) for every reason, so the listing is complete.
- LEGAL_FORMS really is verbatim. FAMILY_TOKEN_KEYS and R2_LEGAL_FORM_TOKENS match crates/ingest/src/crosswalk.rs family_token (line 942) and R2_LEGAL_FORM_TOKENS (line 1835) token for token. One caveat for U3's Rust port: Python NFKD also does compatibility folds that fold_diacritics does not. Over the whole consortium_removed listing only one character is affected ('º' in 13340328), and it does not change that row's match.
- Hard-coded value: subset.py writes 'gb_family': 7 into shippable_changes_not_drops instead of computing it. I recomputed it with cut(): it is 7 (9087207, 10047073, 10213161, 10319218, 11628199, 13448374, 15909819). Cosmetic, but it should be computed.
- The cross-guard claim checks out against head_change_hold's order (family, then gb_family, consortium_removed, nhs_check, placeholder_id). 15783942 is the only family-H<=1 or gb_family row that becomes consortium_removed, and it is top_stale, so it never ships either way. No family-H<=1 row crosses into gb_family. None of the 179 matched rows has a placeholder or GB-NHS identifier.
- Verdicts are recorded, not skimmed. All 33 org_ GETs show the held head as the live name, with the same identifier and mention count as the listing (checked by script). Spot-checks of the evidence hold: Guintoli 10135477 on SIREN 447754086, AEI 16444757 on 41498295900036, Kardham 10519114 FR12384407896 and 30929212 on 38440789600198, AVR 18911984, AxLR 10416059 on the same SIRET, and the spelling counts for 8217358 (42/64), 10266767 (7/9) and 15165788 (66/73). 19 of the 30 rest on the generic 'role annotation dropped' note, but the listing's spellings back each one.
- Overstated wording on 9806693 (one of the 3 uncertain ids, not in the release set). 'The identifier is KAPECI's' is an inference. 418245437 and Poralu Menuiserie's 418245262 are near-consecutive SIRENs, which suggests sibling registrations, and SIRENE could not be reached. The R stands because 3 of the 4 mentions on that SIRET print KAPECI, but the identifier's owner is not confirmed.
- Coverage caveat for U2. The 0-wrong read covers only the 79 shippable R1 rows. That is honest today: R2 is withheld whole, and the 56 head_current and 18 top_stale rows are re-held. But if U2 also releases head_current or top_stale (U1b/T1), the 74 subset-matched rows that stand behind those guards would ship with no verdict from this unit. U2 should read or sample them before combining releases.
- Budget: I used 0 of my 2 confirmation reads (none were needed), and no /v1/sql requests. I wrote nothing in the repo, only scratchpad copies under /tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/548chk/.

## U1b + U2 — the T1 dated 30, votes per notice, and the release decisions (2026-10-19, workflow wf_94723ef4-dcd)

**Decision.** gb_family: release at H<=1 (not whole). family: release at H<=1. consortium_removed: release on the token-subset test. Keep the serve holds, head_current, nhs_check, placeholder_id and top_stale (T1 fails the bar at 2 wrong of 30). R2 stays planner-only. The mojibake R0 path is its own unit. Votes per notice: measured, not adopted.

**Result.**

U1b and U2 are done. T1 fails its bar on the dated 30, so top_stale stays as it is. U2 adopts three releases: gb_family at H<=1, family at H<=1 and the consortium token-subset test. Nothing was edited in the repo; files are under /home/user/tender-db/.scratch/tender-db/548-triage/u1b/ (scripts, sample, verdicts, API bodies, sql/) and the decision is in /home/user/tender-db/.scratch/tender-db/548-triage/u2/decision.md.

U1b: the T1 dated 30
- T1 holds top_stale only when rival_newest > top_newest. Its release arm is 1,318 of the 2,425 top_stale holds (80,037 mentions, 742 with H<=1). Every arm row is R1 with no release flags, so all of it would ship.
- Reverts are 0 across the whole arm: the top is always printed after the head.
- The salted 30 (salt '548-u1b-') was drawn from 1,298 rows after excluding gate (d)'s 20, the 524 skip-ids and the v12 rejudge ids.
- All 30 org GETs show the held head is still the name being served.
- Verdicts: 13 right, 15 neutral, 2 wrong. That fails the bar of at most 1 wrong.
  - 9168556 Usługi Leśne Ryszard Flak -> 'Partner konsorcjum Usługi Leśne Ryszard Flak': the new name adds a consortium-role prefix, and no serve hold tests for that.
  - 17269219 Mithieux TP -> MITHIEUX: the same firm, but 'TP' is dropped and the name is all caps. Prints on the same SIRET say MITHIEUX TP, and MITHIEUX OI is a different firm. I counted it as a same-entity downgrade, the way 524 counted Brlic.
- Only 1 of the 1,318 arm rows adds a role marker, and it is the Flak row. A "T1 plus a role-marker-added serve hold" candidate would pass on this 30, but only because I shaped it on this sample. It needs a fresh dated 30 before anyone builds it.

Votes per notice
- Measured with 2 bounded /v1/sql reads on the release sets and the T1 30, both checked locally first. My cluster approximation reproduces the listing's vote counts for 132 of 138 orgs.
- Counting votes per notice blocks only right changes: 2 gb_family (Bethell, EDF), 0 family, 8 consortium (4 right, 4 unread), and 2 right plus 1 neutral in the T1 30. It stops neither T1 wrong.
- The 2 wrongs it does stop, Barameda and CPR, are already outside the adopted releases. Not adopted.

U2: the decision per hold
- **gb_family: release at H<=1 (D3's narrow form), not whole.** All 7 shippable rows read right, but the H>=2 evidence is 2 rows (13448374, 9087207). In the sibling family guard the H>=2 class is 7 right / 4 wrong and holds every one of its wrongs. One predicate for both guards also limits what the weekly upkeep can ship unread. The cost is 2 right changes kept held; no wrong name is served.
- **family: release at H<=1** (22/22 right, 462 mentions).
- **consortium_removed: release on the token-subset test** (salted 30: 29 right, 1 neutral, 0 wrong), using crosswalk's legal-form tables rather than the wider list. U1c's warning about 74 unread matched rows does not apply, because head_current and top_stale both stay.
- **Kept:** all serve holds, head_current, nhs_check, placeholder_id, and top_stale (T1 not adopted).
- R2 stays planner-only and initialism stays moot.
- The mojibake case 22435268 gets its own unit (an R0-style path for broken heads).

U3 predictions (full listing)
- by_hold drops: **gb_family −7** (it is −10 only if released whole, with head_current +65 instead of +64), family −36, consortium_removed −179.
- The released rows that don't ship move into other holds, so U3's ±10% check also needs **head_current +64 and top_stale +23**. Net holds go from 11,652 to 11,517.
- Predicted changes: up to 106 R1 changes (5 gb_family, 22 family, 79 consortium). These are upper bounds because the serve holds run later and the listing doesn't model them. 29 R2 changes are added but excluded on wet runs.
- 15783942 is counted in family's −36. If family ships without the consortium release, it moves to consortium_removed (+1, so that reason drops 178 net). With both releases it lands in top_stale. It never ships either way.

Budget: 42 public GETs (at least 2.0 s apart by the log, no 408s), 2 of 12 /v1/sql requests, no admin jobs, no box writes, no code edits, no git.

**Adversarial check** (sound: True):

- Nitpick (accuracy): decision.md says 'in all 1,318 arm rows the top was printed after the head (top_newest > head_newest)'. Re-checked on arm.json, it is strictly later in 1,315 rows and on the SAME DAY in 3. The 0-revert count still holds, because verdicts.py defines a revert as top_newest < head_newest. The wording should be 'never before the head'.
- Overstated wording: 'votes per notice blocks only right changes' (result, status line) is not supported for consortium. 4 of the 8 blocked rows there are unread (4R + 4 unread). The supported claim is 'blocks no recorded wrong on the release sets'. Also, gb's '2 blocked' counts only the 3 of 7 reproduced rows. The 2 unreproduced gb rows that are also blocked (10213161, 9087207, both R) are left out of the count. decision.md says this, but the result/status line does not.
- Coverage gap in U2's consortium reasoning: 'U1c's 74-unread caveat does not apply because head_current and top_stale both stay' is true only at this as_of. head_current is a time-based hold (head printed within 1,095 days). Of the 56 consortium-subset rows that land in head_current, 5 age out within 90 days, 16 within 180 and 34 within 365. After that, weekly upkeep ships them unread unless top_stale or a serve hold catches them. family's 7 head_current rows behave the same (4 within a year). This is the same 'what upkeep can ship unread' argument the decision uses to narrow gb_family to H<=1, and it is not applied to consortium. It probably does not change the decision: token-subset is structurally conservative and the salted 30 read 29R/1N/0W. It should still be stated, and U3/U4 could re-read the aging rows.
- Sensitivity not stated: T1 fails the bar by exactly one wrong, and one of the two wrongs (17269219 Mithieux TP -> MITHIEUX) is a judgment call. The new name is the top's own print on the SIRET (3 of 5), and the downgrade is the dropped 'TP' plus all caps. The verdict is consistent with 524's Brlic label, and the gate (d) prior had 1 wrong in T1's 12 released rows, so not adopting T1 is defensible. If 17269219 were read N, though, T1 would pass with 1W. The decision should say it rests on this verdict. The Flak 'Partner konsorcjum' wrong is solid.
- Minor: role_added.py's regex count of 37 has at least 2 false positives, not the 1 decision.md mentions ('Cundall ... and Partners' in head_current, and family's 'ACP IT Solutions AG' -> 'Office Partner GmbH'). The '1 of 1,318 arm rows' figure is a lower bound from a regex, not an exhaustive count.
- Checked and holds: every script (arm, verdicts, role_added, pernotice, u3_predict), re-run in a scratch copy, writes byte-identical outputs. Tallies reproduce: 1,318/2,425, 80,037 mentions, 742 at H<=1, pool 1,298, 13R/15N/2W, H<=1 10/8/1 and H>=2 3/7/1, 132/138 reproduced. U3 drops and landings reproduce (gb -7/-86 lands 5/1/1; family -36/-680 lands 22/7/4/3; consortium -179/-2,769 lands 79/56/18/26), and so does net 11,652 -> 11,517. U1 tallies reproduce: family H<=1 22R; H>=2 6R + 1 'R?' + 4W; gb 7/7 R. U1c reproduces: salted 30 at 29R/1N, and 33R/1N recorded within the 79. Unit-test ids have the stated H/reason/subset status. In my rough regex check only 15783942 crosses a later bind guard, so no other cross was missed. A rough check of the caps_variant, digits-added and doubled serve holds finds none of them firing on the 222 released rows. q03/q04 are bounded IN-list seeks (59 + 79 orgs, LIMIT 5000, not truncated) with local plans recorded. Budget: 42 u1b GETs, at least 2 s apart, no 408s, 42 bodies on disk, 2 /v1/sql requests logged. The checker made no confirmation reads (0 GETs, 0 /v1/sql).

**Next units.**

- U3: build 524u2-v13 with the gb_family H<=1, family H<=1 and consortium token-subset releases; by_hold check on full-listing moves gb_family -7, family -36, consortium_removed -179, head_current +64, top_stale +23 (+/-10%); unit tests 10213161/11628199 released, 13448374/9087207 held, 3728305 held, 8217358 released, 18287356 held, 15783942 cross-guard
- U1d (optional): candidate role-marker-added serve hold plus T1, read on a fresh dated 30 from T1's arm excluding the U1b 30, gate (d)'s 20 and skip-ids
- Mojibake R0 path for defective heads (22435268): its own unit
- U4: update 548 ## Verify to the v13 by_hold reading
