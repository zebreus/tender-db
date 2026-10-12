# 511 — placeholder names (pointers, withheld, `Various`, award summaries) mint parties in every role

Status: ready-for-agent — KEEP RULES DEPLOYED 2026-10-12 (6faf241, with 513 U4). Re-dry job 2266 (`../511-placeholders/u3/plan-2266.json`): award_summary 240 orgs / 510 mentions (+38 orgs: the perfil genitive owners, e.g. 'perfil de contratante del gobierno de aragon', and the High Court 'but' clause), blank 3,566 / 29,386, placeholder 2,534 / 9,877, name 1,670 / 3,225; 42,488 mentions on 34,534 notices, 390 buyer rows on 389 notices. Dropped orgs 6,100 clear the restated floor of 5,928. The 10 perfil+genitive names still placeholder are intended junk ('ver perfil del contratante', '… del organo de contratacion', URL forms) plus one accepted cost ('ver perfil de contratante del hospital clinic de barcelona', stem not at position 0 of the owner). NEXT: buyer canary (refold-notices jobs 2267/2268 on the 389 buyer notices; pre-image `../511-placeholders/u3/canary/pre.json`), then wet with expect 34534, then the sweep.
Was: ready-for-agent — UNIT 3 READS DONE 2026-10-12 (wf_a3c63d17-e99: 700 non-exact folds and every list read; gap analysis). GAP EXPLAINED, so no regression. The floor counted the 522 scoped ingen/inget/nd orgs as Blank, while the plan's by_class shows them as name and the fold decides them per notice language. Like for like, the plan catches 185 more drop-class orgs than expected; the restated floor is 5,928, and the plan has 6,138. STOP for real-party losses, two patterns:
- `Perfil del/de contratante de(l) <body>`: about 43 orgs, mostly specifications-provider / further-information slots, no winners or buyers;
- `Not applicable, but proceedings may be started in the High Court…` (a review body).
KEEP RULES BEING ADDED (partyname.rs: the genitive owner after a perfil stem, and a blank 'but' clause); they gate and deploy together with 513 U4, then a re-dry, the buyer canary, and the wet run. No identified placeholder is a real supplier. Follow-up (precision, not loss): junk the anchor keeps as name, e.g. 'Please see section VI.4.2' (759 orgs), 'For details of the awarded suppliers please see section VI.2', 'Information withheld for security reasons', and '… erfolgt hier keine Angabe'.
Was: ready-for-agent — UNIT 2 DEPLOYED 2026-10-12 (10cb829, gate green; built by agent, correctness and test-strength reviews, 13 owner-ruled fixes). Unit 3 drain: dry job 2265 caught 7,808 orgs, of which the fold drops blank 3,572 / 29,394 and placeholder 2,566 / 9,974, and keeps name 1,670 / 3,225 and award_summary 202 / 405. That is 42,593 mentions on 34,635 notices, 390 buyer rows. The ORG FLOOR TRIPS: 6,138 dropped orgs against a stop line of ~6,450 (mentions 39,368 clear their ~37,000 floor). The mandatory reads (700 non-exact folds, the identified lists, legal-form and digit-only names) and a gap analysis are running (wf_a3c63d17-e99). The wet run waits on them and on the buyer canary (runbook step 2). Daily folds since the deploy drop parties as they touch notices.
Was: ready-for-agent — UNIT 1 DECIDED 2026-10-11 (`../511-placeholders/unit1-decision.md`; census wf_9b2cb650-aa4, 36 bounded indexed calls, 0 × 408; design plus code and data reviews, both 'sound with fixes'; all 22 owner rulings applied, and the 7 extensions confirmed). Classes: VoidLot, Placeholder (narrowed), Blank (new) and AwardSummary (new, never dropped). About 5.1k orgs / 36k mentions are admitted. NEXT: unit 2 build: U2a predicate, U2b fold + NON_NAME_FOLDS, then U2c plan readers and U2d `refold-placeholder-names` once 513 U2 is committed; one gate, one deploy. Then the unit 3 drain (§8).
Was: needs-triage — filed 2026-10-10 from issue 510's unit-1 decision (§9, `../510-void-names/unit1-decision.md`),
which split non-names into two classes and took only `VoidLot` ("the lot was not awarded"). This is the other class.
Kind: data quality (organizations; all eras)
Relates to: 510 (the `partyname::not_a_name` predicate and its `Placeholder` class), 484 (`NON_NAME_FOLDS` in
`project/role_census.rs`, a fold-level placeholder list for the buyer-equal test), 483 (buyer-slot fixes)

## What is wrong

A party slot that names no organization — a pointer (`See section VI.3`, `Voir autres informations`, `Véase perfil del
contratante`, `See Contracts Finder Notice for full supplier list`), a withheld name (`Not applicable`, `Withheld
Section94 Supplier`, `Keine Angabe`), a summary (`Various`, `Various suppliers`, `Multiple Suppliers`, `Diverse
Firmen`) or a relaunch summary (`Lot déclaré infructueux … puis attribué à la Société X`) — is minted as an
organization in every role.

510's census (2026-10-10): 2,952 orgs, 5,672 mentions. Unlike the void class, these are mostly NOT winners:
review-body 1,782, mediation-body 1,252, winner 1,214, appeal-information 403, Lot-ReviewOrg 293,
RESPONSIBLE_FOR_MEDIATION_PROCEDURES 194, purchasing-body 172, specifications-provider 160, further-information 46,
Tenderer 46, tender-receipt 35, **buyer 20**.

## Award summaries kept by 510 (2026-10-11)

510's drain keeps, as `Placeholder`, every void-lot name that also names an award
(`partyname::names_an_award`): ~500 orgs, listed in the `void-names-refold` report's `exempted` (job 2182). Each
is a summary standing in for real winners — `Lot 1) Sarl Bremond. Lot 2) S.A. Les Rapides Varois. Lot 3) Déclaré
infructueux`, `lot 43 INFRUCTUEUX - 44 reckitt benckiser 28692.07 - …`, `Infructueux … attribué à l'entreprise
Promocash`. They belong to this issue: the right outcome is per-lot winners parsed out of the summary (or the
summary dropped and the result left without a winner), not one organization named after the whole list.

## The semantics differ from 510

A winner was chosen; its name is elsewhere or withheld. So a result keeps `selec-w`, but nothing should be bound as
the winner. A buyer-slot placeholder changes buyer tokens and grouping, so dropping one needs its own decision.

## Units

1. **Decide** per role: drop the party (winner, review/mediation slots), keep it, or map it (a buyer placeholder).
   Unify `NON_NAME_FOLDS` with `partyname::Placeholder`. Carry 510's deferred list (`neant`, `ingen`, `n a`, `none`,
   `nie dotyczy`, `entfällt`, `keine Angabe`, `brak`, `sans objet`, `aucun`, `nessuno`, `varios`, `diverse`,
   `various suppliers`, `multiple …`, `withheld …`), each with its role split and its real-name traps (`InGen`,
   `NIL d.o.o.`, `N/A s.r.o.`, `Diverse Care Services`).
2. **Build** on 510's module and retire path.
3. **Drain** with 510's `refold-void-names` job generalised, then the 443 sweep.

## Unit 1 decision (2026-10-11): owner adoption

The decision is `../511-placeholders/unit1-decision.md`. It was revised the same day after two adversarial reviews
(code and data lenses; findings in `../511-placeholders/workflow-wf_9b2cb650.json`). All 22 owner rulings are in, and
the doc's "Review" section lists them.

The revision agent made seven extensions beyond the rulings' letter. The owner CONFIRMED all seven:
1. `would prejudice` stays unanchored.
2. The keep rule also strips a lot designator, accepts en and em dashes, and lets a bare `see section` keep without a
   separator (Royal Courts).
3. Guard 3's echo test compares space-free folds, and only when the name has a letter.
4. Twelve more "name missing" words move to Placeholder (66 winner/bidder rows in total, none with an identifier).
5. FI counts as in scope for `ingen`.
6. 57 specimen assertions flip, not about 45.
7. Anchoring also narrows the text parser's pointer refusals on a reparse. This is accepted.

Figures:
- admitted: 5,089 orgs / 36,236 mentions / 45,910 party rows (Blank 3,917 / 29,605; Placeholder 1,172 / 6,631);
- buyer rows: 286;
- dry floor: about 6,475 orgs, stop below 6,450.

Still open, as the decision lists:
- 1188404 `Ingen` FR (language unread);
- whether to promote the "nobody" words to VoidLot (needs 510's per-word value/date table);
- the deferred words `nil`, `ei`, `sin`, `vari`;
- a follow-up issue for per-lot parsing of the ~500 award summaries, to be filed with unit 2.

