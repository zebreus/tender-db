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

## Residue owner read and second cohort (2026-10-12)

The owner read of U3's 18-code residue was a reviewer plus an adversarial challenger per batch of three
(wf_2411cb6d-b89, 12 agents; `residue-read-wf_2411cb6d.json`). The challenger agreed on all 18.

**Posted**, cohort `nhs-body-residue-2026-10-12` (`verdicts-nhs-body-residue-2026-10-12.json`):
- QJM; QKK (SEL only, the NEL strays excluded); QMF (NEL); QMJ (PPON survivor); QOP (one member); QYG.
- Dry 2280: 6 admitted.
- Wet 2281: 5 merged, 1 one-member; 23 org rows removed; 1,952 mentions repointed; 967 tenders touched; held 0.
- S3 read (`s3check_residue.py` → `s3-read-residue.json`): 2 reported fails, both by design. 30914670 and 31570524 are
  NEL clusters, non-members of QKK and members of QMF. 30914670 is QMF's anchor and absorbed 31570524 (217 → 281).

**Totals after U5: 162 bodies applied, 260 org rows removed, 25,347 mentions repointed.**

**Held: 12.**
- DQF: a minted COH identity is already the body; nothing to fold.
- RFF: no SAME holder.
- RVY: every coded notice falls after its legal end (519 territory).
- QUY, RCB, RHM, RNK, RRV, RXF, RXT, RYC, RYG: `member-key-owned-elsewhere`. In 7 of these the blocked member is the
  body's own **PPON org, i.e. the survivor**, whose head key is owned by its uncoded twin (an identifier-NULL row
  outside the body).
  - U4's alias arms loser keys only: `NHS_BODY_ALIAS_LEDGER_SQL` reads `$.loser_name_norm`.
  - `arm_nhs_body_alias_on` already refuses, at arm time, a loser key that a standing identifier-NULL row owns.
  - So the census gate guards nothing when it catches the survivor. That is filed as U6.

## U6 cohort (2026-10-12)

U6 (the survivor-exempt D4) was gated at 6593d9c and deployed at c786f0e. Census 2284 confirmed that the nine bodies
held only on their PPON survivor's twin-owned key still had their reviewed holder sets.

Posted as cohort `nhs-body-u6-2026-10-12` (`verdicts-nhs-body-u6-2026-10-12.json`): QUY, RCB, RHM, RNK, RRV, RXF, RXT,
RYC, RYG.
- Dry 2285: 9 admitted.
- Wet 2286: 9 merged, every survivor the body's PPON org; 13 org rows removed; 1,358 mentions repointed; 798 tenders
  touched; held 0.
- S3 read (`s3check_u6.py` → `s3-read-u6.json`): 0 fails.
- Closing dry 2287: `admitted_codes` []; 171 applied; `nhs_body_regrown` []; ledger 273.

**Totals: 171 of 174 codes applied, 273 org rows removed, 26,705 mentions repointed.** Still held:
- DQF: the minted COH identity already is the body;
- RFF: no SAME holder;
- RVY: after its legal end, so it belongs to 519.
