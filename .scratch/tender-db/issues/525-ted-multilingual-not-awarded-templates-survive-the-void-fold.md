# 525 — TED's multilingual "not awarded" templates survive the void fold, and the void walk re-lists them forever

Status: done — BUILT, GATED, DEPLOYED AND DRAINED 2026-10-12. Build wf_b2a69580-a7e: 3 reviewers (2 must-fix, the walk skipped DE-1.x normalisation; 4 should-fix/nit) and fixes, then a second panel wf_151870b7-e19 on the fix delta (1 should-fix, a pointer is one group; 1 nit, a void entry reads a template as classify does), fixed with mutation-checked tests. Gate green at ec1b121, deployed ec1b121. Drain: dry 2355 listed 92 void-lot orgs / 128 mentions / 89 notices (26 distinct folds, every one a plain 'not awarded / no tenders received / unsuccessful' phrase, read before the wet run); wet 2356 → fold 2357 retired exactly 128 non-name mentions and announced 255 correction rows → sweep 2358 swept 92 of 92. Verify reads done: dry 2359 lists 0 void-lot orgs and re-queues nothing; 6709213, 1354108, 1265984 and 27220329 answer 404. Residue by design: 7969717 ('… — Lot 5 non attribué', a lot-numbered compound no exact template holds) is exempted as 'party kept: 2/24 names void'. The 376 exemptions are 520's (344 award summaries, 31 single-copy names the fold reads as no void, and 7969717).
Was: ready-for-agent — filed and DIAGNOSED 2026-10-12 (two independent diagnoses and a verifier, wf_8a719bd1-4b6,
`../520-award-summaries/void-residue-diagnosis-wf_8a719bd1-4b6.json`; both reproduced it with probe tests built from
the notices' public `/content`). NEXT: the build unit below (all four parts in one deploy), then a drain.
Kind: data quality (void-lot winners served as organizations) and a walk/fold contract bug
Relates to: 510 (VoidLot), 511 (placeholder classes; the walk), 520 (award summaries, whose exemption shares the walk)

## Verify

    ssh root@zebreus.click "/root/aj.sh /admin/jobs '{\"kind\":\"refold-void-names\"}'"   # then read the job line

- **done**: after the drain, the dry walk lists 0 void-lot orgs and does not re-list notices; org 6709213 ('Non
  applicable — Lot non attribué') and 1354108 ('non aggiudicato') answer 404.
- **open**: 37 void-lot orgs / 40 mentions on 26 notices, re-listed after each wet run (2351, 2352→2353, 2354 on
  2026-10-12).

## What was found

On 2026-10-12, `refold-void-names` dry 2351 listed 37 void-lot orgs / 40 mentions. The wet run 2352 re-queued the 26
notices and fold 2353 retired **0** mentions (`0 non-name retired`). A dry re-walk 2354 listed the same 37. They were
already in 510's drain cohort, so they have survived every drain since 2026-10-11.

The causes, verified against the code:

- **A — 38 mentions, all 24 ted-export-r208 notices.**
  - These are TED multilingual winner slots. The head copy is void, but most of the party's 2–24 language copies
    are TED wordings `VOID_WHOLE` (partyname.rs:141) does not hold: 'Zuschlag nicht vergeben', 'Der Auftrag wurde
    nicht vergeben.', 'Kontrakten ikke tildelt.', 'Partiet blev ikke tildelt', 'Los wurde nicht vergeben', 'Lote no
    adjudicado', 'El contrato no ha sido adjudicado.' and others.
  - `drop_decision` (project.rs:4504) drops a party only when every copy is classed (:4548), or when there are at
    least 4 copies and at least half are void (:4556). So these parties are KEPT, and they are served as winners
    named 'Lot not awarded', 'Marché non attribué.' and so on.
  - Examples: 16893864 ORG-2 (6 of 23 copies void), 12501289 ORG-11 (IT/DE, 1 of 2), and 6709213's 4 mentions
    ('Non applicable — …', where the EN/PL copies class as Blank).
- **B — 1 mention, 23151933 (r209).**
  - The void name sits in a NESTED section: ORG-3 inside ORG-2, and RES-1's winner points at ORG-2. The fold drops
    the party, and `NoticeState::read` drops roles through the inner half.
  - But `mentions_and_dropped` builds its retire keys (`skipped`) from outer sections only (project.rs:5292-5297,
    the `!alias.contains_key` term). So the ORG-3 mention row stands, keeping org 1265984 alive. This is a fold bug.
- **C — 1 mention, text 3322691 ORG-2.**
  - It is published as 'NO AWARD WAS MADE'. `void_prefix` takes a lower-case next word as part of the void phrase
    (partyname.rs:2500), so the all-caps copy reads as a name, while the org's mixed-case head 'No award was made'
    reads VoidLot.
- **The walk/fold contract.**
  - The walk splits its net with `is_void_lot(organizations.name)`, a one-name test on the head (supervisor.rs:3794,
    :5581). It does not run the fold's party decision. So it re-queues notices the fold will keep, and the dry count
    never converges.
  - The doc comment (supervisor.rs:3788-3791) and docs/operations.md both claim the walk uses "the fold's own
    predicate", which is not true.

## The unit (one build, one deploy)

1. **Fold lexicon (A, C).**
   - Add TED's other "not awarded" template wordings to `VOID_WHOLE` as WHOLE values. Take them from the kept
     parties' actual copies (the diagnosis lists them per notice) and the TED template translations. Add 'no award
     was made' as a whole value, so that case does not matter.
   - Keep every addition a whole-value match, so no company name can match.
   - Tests: 16893864's 23 copies drop the party; 12501289's IT/DE pair drops; 16967678 ORG-2 still drops and ORG-4
     now drops; near-miss real names stay.
2. **Fold retire keys (B).**
   - Build `skipped` from every dropped party section, inner aliases included. Minting and the `DroppedParties`
     count stay outer.
   - Test: the nested 23151933 shape retires the ORG-3 mention on an incremental and on a full refold.
3. **Walk (contract).**
   - Re-queue a net hit only when the fold's own `drop_decision` drops that (notice, section). Use a public wrapper
     over `dropped_party_sections`, with the same sdk01 flag, section map and `nested_org_aliases` the fold uses, read
     via `parsed_by_ids`.
   - List every other hit under `exempted` with the reason ('party kept: 6/23 names void').
   - Re-queue whenever the parse is missing or unreadable, so the re-queued set always covers the fold's drops.
   - Test that the re-queued set covers every fold drop.
   - Recommended hardening: widen the net from the head to every `organization_names` copy.
4. **Docs.** Fix the "fold's own predicate" wording, and record the party rule in docs/operations.md.

**Then the drain** (510's runbook): dry, read every listed name, wet with `expect`, then fold, then
`sweep-orphan-orgs`.

**Risk.** Whole-value lexicon additions only touch names that are exactly a "not awarded" template. A real award can
become no-award only if a template wording is ALSO a company's exact name. Read the drained list before the wet run,
as 510 did.
