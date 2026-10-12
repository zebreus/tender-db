# 513 U5 — the nhs-body campaign on prod (2026-10-12)

Rev 6faf241. Every step followed `docs/operations.md` "The nhs-body campaign".

| step | job | result |
|---|---|---|
| 1. dry | 2274 | 156 admitted; `nhs_body_regrown` []; ledger 0 |
| 2. capped wet (20) | 2275 | 20 bodies folded (18 merged, 2 one-member); 60 org rows removed; 4,826 mentions, 5,401 parties, 71 bid-parties, 76 winners repointed; 2,822 tenders touched; held 0 |
| 3. S3 read | dry 2276 + `s3check.py` → `s3-read.json` | **0 fails** (details below) |
| 3. S5 canary | 2277 | **AGREES**: 29 mentions of 9 notices re-resolved, 29 agree, 0 disagree; alias asked 6, bound 6, refused 0 |
| 4. full wet | 2278 | 136 bodies folded (101 merged, 35 one-member); 177 org rows removed; 18,569 mentions, 21,270 parties, 1,649 bid-parties, 2,121 winners repointed; 11,621 tenders touched; held 0 |
| 4. closing dry | 2279 | `admitted_codes` []; `nhs_body_regrown` []; verdict status `applied` 156; ledger 237 rows |

What the S3 read checked, per folded body:
- the survivor is the reviewed one (the PPON org for `target: ppon`; the anchor only when no member carries a PPON);
- every folded member is in the verdict's `reviewed_holders`;
- each loser answers 308 at `/v1/organizations/<loser>`;
- `?identifier=GBNHS<code>` resolves to exactly the survivor;
- the fresh census lists no loser as a holder, and every non-member keeps its mentions;
- the survivor's mentions are at least the members' sum.

For the 20 capped bodies the checker used dry 2274 as the pre-image. For the 136, `s3check_full.py` used dry 2276 as the
pre-image and dry 2279 as the post-image (`s3-full.txt`).

The full read reported 7 "fails". All seven were verified as by design: a multi-code holder (a stray, which published
a neighbour's code by mistake, u1-results §3) listed as a non-member of the stray code and folded, as a member, into its
own body. Each answers 308.
- R0D 1240944 Dorset HealthCare → RDY
- REN 1371484 and 4275910, the Walton Centre → RET
- RJE 1167736 UHB → RRK
- RJZ 1432862 Kettering → RNQ
- RTF 4474166 UHL → RWE
- RWW 3793356 Devon Partnership → RWV

Their stray-coded mentions now sit on the right body's survivor. D2 never binds a code literally, so nothing is
mis-filed.

Canary notices (`canary-job.json`): for each of 0AR, 0DE and 0DF, a TED uncoded mention, an FTS coded party and a
PPON-first party.

Still open (U5 tail):
- S4: after the next daily folds, read the `[issue 513]` diag lines (refusals ≤ binds) and a dry run's
  `nhs_body_regrown` (must stay []).
- The owner read of the 18-code residue (174 codes − 156 verdicts).
- Then 469 U7, with 519 before or alongside it.
