# 548 — the head refresh's holds block mostly right changes (consortium, family, gb_family, part of initialism)

Status: needs-triage — filed 2026-10-15 from the judgement of 524's dry plan 2515 (workflow wf_f9b41c3f-643). Next:
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
