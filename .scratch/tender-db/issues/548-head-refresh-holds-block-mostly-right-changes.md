# 548 — the head refresh's holds block mostly right changes (consortium, family, gb_family, part of initialism)

Status: ready-for-agent — **U0 DONE 2026-10-19 (blocker cleared)**: the stored `org-name-refresh-plan` was read by the orchestrator (`/admin/reports/org-name-refresh-plan`, computed 2026-10-16 23:03Z, epoch 524u2-v12, scope_ids null, rev 3475272, population 2,102,132 orgs / 46,130,516 mentions, `held` 11,652 rows, held_truncated false) and saved as `548-triage/org-name-refresh-plan-2026-10-16.json.gz`; unit 1 runs on it. Triaged 2026-10-19 (.scratch/tender-db/548-triage/notes.md). BLOCKER, retry it: unit 1 needs the full `held` listing. It exists only in the stored `org-name-refresh-plan` report, and `reports` is not on /v1/sql. This session's classifier refused the `aj.sh /admin/reports/org-name-refresh-plan` read ('Production Reads'). U0 must also confirm that the report is an unscoped v12 plan (org_ids null), or run a fresh full dry plan. Found so far: (1) The 553 counts are rule v5's. A released bind-guard hold ships today only if: its rule is R0e, R0n or R1 (R2 and R0w are excluded); its release is not `unfit` (exclude_released); and, for R1 only, its head is older than 1,095 days or it has a `digits` release, and top_stale does not fire. Unit 1 filters `held` to that subset before judging. (2) initialism is R2-only and R2 is withheld, so it is decided moot. R2 stays planner-only; that is recorded as D1. (3) Both gb_family exhibits read the other way round from the issue: a single stale subsidiary or sibling print is the head on the register entity's own CH number. 11628199 has 3 of 4 votes and 10213161 has 49 of 51; both changes are right and would ship. (4) top_stale variant T1 (hold only when rival_newest > top_newest) was tested in-sample on gate (d)'s 20 holds. It releases 12 (5 harmful, 6 neutral, 1 same-entity downgrade) and keeps 7301922 (a real rename) and 8291325 (another entity) held. NEXT: U0, then U1 on the filtered listing; U1b, a fresh dated 30 from T1's release arm; and the classes routed to unit 1: a head_current 30-sample, mojibake before the serve holds, a generic-name signal, and votes per procedure. Was: needs-triage — filed 2026-10-15 from the judgement of 524's dry plan 2515 (workflow wf_f9b41c3f-643); 524's v8–v10 re-judges added the R2 withdrawal, generic names and a mojibake serve-hold harm (below). Next:
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

