# 511 unit 3 — buyer canary (2026-10-12)

- Pre-image: `pre.json` (`snap.py`, bounded `/v1/sql`: tender_versions_notice seeks, tenders PK, the
  procedure-key index exact + refused range, tender_versions PK; IN lists ≤100; 0 × 408), taken at 6faf241
  after re-dry 2266.
- Refold: jobs 2267 (refold-notices, 389 notices, 326 tenders stamped) → 2268 (project: 389 notices →
  364 tenders, 302 non-name mentions retired: 32 placeholder, 270 blank, 0 identified; 11 issue-481
  links joined) → 2269 (auto sweep: 8 orphan orgs).
- Post-image: `post.json`; `compare.py pre.json post.json` → `compare.txt`.

## Result: PASS

**Tender changed on 10 of 389 buyer notices (2.6 %; stop line 10 %).** Every one is a German award
notice (subtype 30) whose placeholder buyer had kept it apart from its contract notice (subtype 17).
With the buyer dropped, issue 481's previous-notice link joined it, and its split Tender (ids 88xxxxx)
is gone:
- 207079 "Unterstützung bei Zulassungsverfahren";
- 231408 "digitales Eventmanagement";
- 1103606 "Anschaffung von Werkstattdrehgestellen und Adapterpaaren";
- 860829 "Speditionsleistungen Los 1+2", 4 notices, the job's largest component;
- 216329 "Payment Services 2025+: Los 3", 2 notices;
- 354400 "Schienengebundene Transport- und Transportorganisationsleistungen".

**One family change beyond the buyer notices**: key `9c410797…`. Notice 24889194's group
`refused:…:AT:national:FN248730F` (Tender 8816978) was split from the placeholder-buyer group
`refused:…:#gd05e3e94` (8816977, buyer notice 25741160). The split was refused because the two buyers
disagreed. With the placeholder gone the refusal no longer fires, and both notices sit in 354400. So
this is a heal, not a regression.

Keys changed on no other notice, and no family lost or gained a notice except as above.

## Wet floor (step 3), restated like-for-like

The dry plan's 42,488 mentions include the `name` class (3,225, which the fold keeps unless it is in
language scope). The floor is therefore taken over the drop classes:
- blank 29,386 + placeholder 9,877 = 39,263;
- minus 56 guard-3 identified-blank mentions;
- minus the canary's 302 already retired;
- **floor 38,905**.

The in-scope `ingen`/`inget`/`nd` names can add up to 535 on top. Above 42,488 is a read trigger.
