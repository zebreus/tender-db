# 520 — an award summary stands as one winner org where the notice names a winner per lot

Status: ready-for-agent — U2 DEPLOYED 2026-10-13 (1f6c58f, gate green). Build wf_adaab0d6-174 (3 reviewers, 23 fixes); second panel wf_3592a3ce-8f8 (8 verified, the costly direction) fixed and independently checked (wf_cd50049c-c58: invariant holds by construction and over 32,363 strings; one more costly case fixed by the owner). U3 dry 2360 matches the decision exactly: 173 void-lot orgs / 187 mentions / 116 notices, 203 exempted (A 57, B 99, C 29, F 6, residual D 12); every re-queued org is class D. NEXT: the award-evidence read of the 116 result blocks (wf_2f835d23-17e, stop if any reads as an award), then wet (expect 116), fold, sweep; Verify Infructueux 7 → 6. Then U4 (A1 winner extraction).
Was: ready-for-agent — unit 1 DECIDED 2026-10-12 (`../520-award-summaries/unit1-decision.md`; classifiers + challengers wf_d27669a6-357 over all 376 void-walk exemptions after 525, `unit1-classified.json`). Classes: D pure void prose 185 (names_an_award misreads it), B lists/co-winners 99, A one winner 57, C text `Lot n)` lists 29, F 6, E 0. NEXT: U2 build — D1–D7, seven narrowings inside names_an_award's own arms (partyname only), moving 173 D orgs / 187 mentions to VoidLot with 0 A/B/C/F moved in simulation; then U3 drain (dry; read every re-queued result's award evidence; stop rules in §6). Then U4 (A1 winner extraction `award_winner`, 47 orgs) and U5 drain; U6 (A2) conditional. B kept per 511 §5. (C) filed as 526; the eForms '(niet gegunde partij)' role defect filed as 527. Verify corrected: Infructueux prefix 7 → 6 after U3 → 1 when done (6603158, a kept B list).
Was: ready-for-agent — triaged 2026-10-12. The claims hold for the XML era. In the text era the parser already drops these summaries on reparse, so 2004–2010 lost their winners and only pre-2004 rows still stand. The issue also undercounts: per-lot lists without a void word are ordinary names, and text `Lot n)` names come to about 20k orgs. NEXT: unit 1, measure and decide (S). Re-take the exempted list with a `refold-void-names` dry; 2182's list is a photograph. Sort the names into three classes by profile: (A) relaunch prose with one winner per RES (extract the winner); (B) XML multi-lot lists in one award block (keep, per 511 §5; no RES synthesis, no unit-price figures); (C) text `Lot n) X. Lot m) Y` lists, a `lot_prefix_len` parser fix plus a reparse of the 1997–2003 text fetches. Record the decision and file (C) as its own issue. Add a `## Verify` block.
Was: needs-triage — filed 2026-10-12 with issue 511 unit 2 (decision §5).
Kind: data quality (results; organizations)
Relates to:
- 510 (`partyname::names_an_award`);
- 511 (the `AwardSummary` class, which the fold keeps);
- 491 (per-award figures in the text era).

## What is wrong

About 500 orgs are named after a whole award list instead of a body. Examples:
- `Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré infructueux`
- `lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - …`
- `Infructueux … attribué à l'entreprise Promocash`

510's drain keeps them: they are listed in the `void-names-refold` report's `exempted`, job 2182. 511 classes them
`AwardSummary`, and the fold keeps them.

The right outcome is one of:
- **(a)** parse per-lot winners (and figures) out of the summary into their `RES-n` lot results, and drop the summary
  org; or
- **(b)** drop the summary, leaving each result without a winner, where parsing is not reliable.

## Proposed units

1. **Measure.** Read the ~500 names by era and publisher, and class each one:
   - parseable (`Lot n) <name>` lists);
   - prose relaunches (`… puis attribué à X`);
   - mixed.
2. **Decide** (a) or (b) per class, and decide which parser owns each era. The text era is `text/parse.rs`; r208 and
   r209 are the legacy readers.
3. **Build** behind the same refold machinery as 510 and 511.

## Triage (2026-10-12, owner; panel wf_73cc2c40-e1c)

Triage 2026-10-12. The claims hold for the XML era.
- `partyname::classify` returns AwardSummary for a void stem plus an award (partyname.rs:631), and `drop_decision` keeps any party with an AwardSummary name (project.rs:4544).
- The examples are live: 7791767 (r208 063638-2016, ORG-13) and 3371523 (r208 073345-2013, ORG-4 in RES-3). `name_prefix=Infructueux` still returns 510's 7.

The text era differs. `plausible_name` (text/parse.rs:148-156) refuses every not_a_name class. So 491's reparse of fetches 185–269 already dropped the 2004–2010 summaries, outcome (b), and lost their winners. `Lot 1) Sarl Bremond …` is gone (prefix read returns 0). Text summaries survive only in unreparsed pre-2004 notices, e.g. 31735007 (20187-2002).

"~500" counts void-plus-award names only. The same list without a void word is an ordinary name, and in the text era it comes from a parser defect: `lot_prefix_len` (:540) accepts a `)` reference only when it is all digits, so `Lot 1)` is never stripped and `winner_segments` (:660) never splits at `. Lot 2)`. Counts on the name_norm index range, 2026-10-12:
- 20,481 orgs start with `lot `;
- 5,239 start with `lot 1)`, of which 994 are lists and 17 carry a void word;
- 141 bare `Lot 1)` and 19 bare `lot 2)`;
- a 52-org stride sample has 48 text orgs (43 from 1997–2003) and 4 r208.

Each prefixed name splits an identifier-less company: `Lot 1) SARL Bonsignore & Olivetta` (30587843) sits beside `Sarl Bonsignore & Olivetta` (25920173).

Option (a) as written does not fit the r208 lists. 17304500's RES-3 holds lots 43–57+ in one AWARD_CONTRACT, the list spills into TED-ADDRESS, the names are abbreviations (`reckitt b`) and the figures are unit prices.

Proposed classes:
- (A) relaunch prose with one winner per RES (`… puis attribué à la Société X`): extract X in the fold;
- (B) XML multi-lot lists: keep, per 511 §5; no RES synthesis and no figures;
- (C) text `Lot n)` lists: a `lot_prefix_len` fix plus a reparse of 1997–2003. Filed as its own issue, and it must land before any pre-2004 reparse, which today would refuse whole lists and lose their real winners.

NEXT is unit 1 (S): re-take the exempted list with a `refold-void-names` dry, class each name by profile with bounded reads, record the decision, and file (C).

Verify: `curl -s "https://tenders.zebreus.click/v1/organizations?name_prefix=Infructueux&limit=50" | jq '.items|length'`. Open: 7 (2026-10-12). Done: 0.

**Claims:** Mostly yes, but the issue undercounts the problem and leaves out an asymmetry between eras.

What holds:
(1) The AwardSummary class exists and the fold keeps it. A void stem that also names an award returns AwardSummary (partyname.rs:631, `names_an_award` :2540). `drop_decision` skips any party with an AwardSummary name (project.rs:4544). The walk lists these names as `exempted` and never re-queues them (supervisor.rs:5567-5708).
(2) The issue's examples are live in the XML era. Org 7791767 (`Infructueux … attribué à l'entreprise Promocash`) is r208 063638-2016, ORG-13. Org 3371523 (`lot 43 INFRUCTUEUX - 44 reckitt benckiser …`) is r208 073345-2013, ORG-4 in RES-3. `name_prefix=Infructueux` still returns the same 7 summaries 510's Verify recorded.

What is wrong or missing:
(a) The text era is not "kept by the fold". `plausible_name` (text/parse.rs:148-156) refuses every not_a_name class, AwardSummary included. So any text notice reparsed since a3153bc (2026-10-10), which includes 491's drain of fetches 185–269, has already had its summaries dropped (outcome (b)), and those winners are gone. The `Lot 1) Sarl Bremond …` example no longer exists: `name_prefix=Lot 1) Sarl Br` returns 0. Text summaries only survive in unreparsed pre-2004 notices.
(b) "~500" counts only void-plus-award names. The same per-lot list without a void word classifies as an ordinary name. That class is far larger, and in the text era the cause is a parser defect. `lot_prefix_len` (text/parse.rs:540) only accepts a `)` reference when it is all digits. So `Lot 1)` is never stripped, and `winner_segments` (:660) never splits at `. Lot 2)`. There is no test for a `Lot n)` prefix (only `1) CGC`, :3452).
(c) Option (a) as written ("parse into their RES-n") does not fit the r208 lists. In 17304500 one AWARD_CONTRACT holds lots 43–57+, and the list spills into TED-ADDRESS (`72 JOHNSON J 1160.5 - … 78 SANS SUITE …`). The names are abbreviations (`reckitt b`, `arrow g`, `sanofi av`) and the figures are unit prices (0.08, 5.476). Splitting would mean inventing RES sections and minting junk names, and claiming those figures would pollute amounts.
(d) The issue has no `## Verify` block.

**Evidence:** Code: partyname.rs:24-29, 95, 631, 2540-2563 (`names_an_award`), 2755 (`several_lots`); project.rs:4457-4458, 4544; text/parse.rs:148-156 (`plausible_name` → `not_a_name(name).is_none()`), 540-580 (`lot_prefix_len`: the `)` branch needs head.bytes().all(is_ascii_digit)), 660-688 (`winner_segments` splits at '.' only before a `lot_prefix_len` hit), 892 (`strip_lot_prefix`). 511 unit1-decision §5 (lines 252-259) states the keep, and says the text parser keeps refusing these names.

Prod reads (2026-10-12; public GETs plus bounded /v1/sql; 0 failures, 0 timeouts):
- Example orgs: 7791767 → notice 18662480 (r208 063638-2016, ORG-13); 3371523 → 17304500 (r208 073345-2013, ORG-4).
- Notice 17304500 has 37 LotResult sections, each with exactly one ORG.
- RES-3 has only TED-CONTRACT_TITLE `Classe thérapeutique Voies digestives`. ORG-4's TED-OFFICIALNAME holds lots 43–57, and its TED-ADDRESS continues the list from lot 72.
- More void-plus-award examples, all r208: relaunch sentences 1435452 (369376-2011) and 31482009 (242013-2011); lists 2381358 (215115-2012), 5351950 (226555-2014), 2957542 (376709-2012), 9811971 (173221-2017), 31428158 (107995-2011). One is pre-2004 text: 31735007 (20187-2002, `Lot 1) Blin. Lot 2) Prodirest. … Lot 6) Déclaré infructueux …`).

Index-range counts on organizations.name_norm:
- `lot ` prefix: 20,481 orgs.
- `lot 1)` prefix: 5,239. Of these, 994 contain `lot 2)` / `lot 3)` / `lots 2` (a list) and 17 carry infructu / sans suite / non attribu.
- Exactly `lot 1)`: 141 bare orgs; exactly `lot 2)`: 19.
- A stride sample of 52 `lot …` orgs (id % 400 = 7), one mention each, splits 48 text (43 from 1997–2003) and 4 r208.

Identity split: `Lot 1) SARL Bonsignore & Olivetta` (org 30587843, 4 mentions) sits beside `Sarl Bonsignore & Olivetta` (25920173, 2) and `SARL Bonsignore et Olivetta` (25405458, 1). `Prodirest` (24667739, 878 mentions) does not receive the awards that the lists name.

A void-plus-award `lot …` read (94 rows) also turned up leftover VoidLot misses: `Lot 1 sans suite / lot 2 infructueux` (31428159), `lot déclaré iinfructueux` (31919275), `Lot non attribué` (4826424).

**First unit (S):** Measure and decide, no build. Steps:
1. Enqueue a `refold-void-names` dry. Its report's `exempted` lists every current void-plus-award org with its mention count. Add the AwardSummary exemptions from the latest placeholder dry (`../511-placeholders/u6/plan-dry.json`, 242 entries, mostly pointer-plus-body names, out of scope).
2. For each exempted org, read one mention's profile with bounded reads: `organization_mentions` WHERE organization_id = ? LIMIT 1, then notices by PK. Expect r208 French 2011–2017 plus pre-2004 text, and no 2004–2010 text.
3. Sort each name into a class:
   - (A) relaunch prose naming one winner in its own RES (`… puis attribué à la Société X …`, `… a été attribué à l'entreprise Promocash (02100 …)`);
   - (B) an XML multi-lot list inside one AWARD_CONTRACT (the Brest and AP-HP pharmacy shapes);
   - (C) a text `Lot n)` list.
4. Decide per class and write the reasoning into the issue. Proposed:
   - (A): a fold-level winner-name extraction from the award clause, in the same party and RES, with tests for its near misses;
   - (B): keep as AwardSummary (511 §5: leaf orgs, the only readable winners). Splitting would invent RES sections from a name string, mint abbreviated names (`reckitt b`) and claim unit prices as awards;
   - (C): file a sibling issue. `lot_prefix_len` should accept an optional `Lot`/`Lots`/`Lot n°` word before the digits in the `)` and `:` branches, with tests (`Lot 1) X. Lot 2) Y`, `Lots 2) 8) 20) X`, the bare `Lot 1)` orgs, keeping `Lot A)`, `Lot unique:` and real names). Then reparse the 1997–2003 text fetches (270+). This must land before any pre-2004 reparse, because today a reparse refuses the void-bearing lists whole and loses their real winners (31735007's Blin and Prodirest).
5. Add the `## Verify` block:
   `curl -s "https://tenders.zebreus.click/v1/organizations?name_prefix=Infructueux&limit=50" | jq '.items|length'`
   It reads 7 while open (2026-10-12) and 0 when (A) and (B) are resolved.

**Value:** In the issue's scope, about 500 leaf organizations are named after an award list or a relaunch sentence (job 2182's photograph, 2026-10-11), mostly with one mention each. They are r208 French awards from 2011–2017 plus pre-2004 text. The harm: junk names in the org directory, and the real winners never reach their own organizations. Prodirest's 878-mention org, for example, misses the lots these lists give it. The 2004–2010 text summaries were already dropped by the parser on 491's reparse, so their winners are gone rather than kept.

The adjacent class the issue does not count is bigger:
- 20,481 orgs whose name starts with `lot `;
- 5,239 starting with `lot 1)`, of which 994 are lists;
- 160 bare `Lot 1)` / `lot 2)` orgs.

A sample puts about 92% of these in the text era, mostly 1997–2003. Because an identifier-less winner's name is its identity, every prefixed name splits the company: `Lot 1) SARL Bonsignore & Olivetta` (4 mentions) does not merge with `Sarl Bonsignore & Olivetta`. That class carries most of the value, and a parser fix plus a pre-2004 reparse clears it.

The class (A) extraction is small and precise. Class (B) has low value and carries precision risk.

**Overlap:** No duplicate among the open issues, but several are adjacent:
- 511 §5 is the decision that keeps AwardSummary, and 511 unit 7 is a placeholder tail, not summaries.
- 521 (the "nobody" words) is a different class.
- 491 unit 4 (c) and its per-award figure rule are the precedent for keeping list figures (unit prices) out of awarded values.
- 510 is done. Its accepted residuals overlap the VoidLot misses seen here (`lot déclaré iinfructueux`, `Lot 1 sans suite / lot 2 infructueux`, `Lot non attribué`); those could go to 511 unit 7 or a 510 residual note.

The text `Lot n)` parser defect (`lot_prefix_len`) has no issue yet. It continues issues 484/485/244 (`lot_prefix_len` and `winner_segments`) and should be filed as its own issue, with 520 pointing to it.

**Rank:** 7/10 across 520/521/523/524. Best value per effort, because the next unit is cheap: S, no build, one dry run of the existing `refold-void-names` job plus bounded reads. Its payoff is the parser defect the triage found, which I confirmed in code. `lot_prefix_len` (crates/ingest/src/text/parse.rs:540) accepts a `)` reference only when the text before it is all digits, so `Lot 1)` is never stripped and `winner_segments` (:675) never splits at `. Lot 2)`. The only test is `1) CGC` (:3452).

The value claims need trimming. The ~500 summary names in the issue's own scope are mostly one-mention junk orgs from r208 2011-2017, and classes (A) and (B) will move only dozens to a few hundred rows. The 20,481 `lot ` prefix count is an upper bound: it also catches real names such as LOT Polish Airlines and Lot-et-Garonne bodies. The firmer figure is 5,239 `lot 1)` orgs, 994 of them lists, about 92% from the text era and mostly 1997-2003, which is old, low-traffic data.

Urgency is lower than the triage implies. 491 has already reparsed fetches 185-288 (2010 back to 2002-06), and I found no reparse of fetches 289+ scheduled. So "must land before any pre-2004 reparse" is a guard, not a race. It has no dependency on 519 or 469. A cheap unit that clarifies the scope and spins off a well-defined parser issue, (C), for thousands of split identifier-less orgs ranks first.
