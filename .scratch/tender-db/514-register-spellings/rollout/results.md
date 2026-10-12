# 514 unit 2 — rollout on prod (2026-10-12)

The build was fd509b6 (gate green, 1,188 s), deployed as 31f0872. Every step followed the decision's §5.1.

| step | job | result |
|---|---|---|
| 345 dry | 2311 | 53 planned (the 54 reviewed movers less L14, held by `REGISTER_SPELLING_HOLDS`); 25 reunions; 0 now refused. Each target equals its reviewed canonical value. |
| E0 dry, B1 | 2312 | plan 0. W6 strip: 0 admitted, 0 walled (S9a: W6's footprint before the re-key is 0) |
| 345 wet, bound | 2313 | applied 53; 0 held; 0 skipped; 53 pre-images recorded (rule `renormalise`) |
| E0 dry, B2 | 2314 | plan **26** groups, all admitted by the posted verdicts (cohort `514-spellings-2026-10-12`); W6 strip admitted 26. The listing equals the 26 reviewed keys and members exactly (S9b) |
| loser snapshot | — | 26 losers, 64 mentions (`loser-mentions-snapshot.json`) |
| E0 wet, max 5 | 2315 | 5 merged; S10: each loser answers 308, each canonical value answers its one survivor with the summed mentions |
| E0 wet, full | 2316 | 21 merged (bound to the listing, 0 held); 3 winner duplicates deleted |
| repeat dries | 2317, 2318 | E0 plan 0; 345 planned 0 |
| altid dry | 2319 | registry pairs unchanged from the previous plan (0 live, deferred or unreviewed); `registry_regrown` 0 |

Verify:
- all 26 canonical values answer exactly one org (`verify-canonical.json`);
- `?identifier=GBUKPRNUKPRN10007798` answers survivor 31600847 with `merged_into`.

**Totals: 53 identities re-keyed (25 onto a standing twin), 26 org rows removed.**

Still open:
- the next daily fold's 448 line "register targets on two or more rows" should read 0;
- unit 3 (the Companies House mirror, RS→bare, IP0), CHC compounds and generic labels, the near-miss re-keys
  (Hastoe `ip6243r`, `RSA007814`, the UKPRN zero typos), and XI;
- A6: 31558903's head name is still "Sheffield Hallam University", from one stray mention (head-name refresh).
