# 358 — Overseas-department and Åland country codes: one register, two tags

Status: DONE 2026-09-07 (owner) — unit 1 built and deployed (`e988e58`, live as rev `5b09a9c`), unit 2 applied (job 784: 651 rows moved, 0 no-ops, 313 landed on a parent-code twin), unit 3 folded (R2 job 786: 321 groups merged, 354 org rows removed, 1,894 mentions / 3,687 parties / 1,017 winners repointed, 1,134 tenders touched; project job 787 clean). Residue: 31 reunited pairs held by R2's name gate — issue 362's review queue, not this issue's. Follow-up filed: issue 363 (FO-nummer label on 16 AX rows). Was: needs-decision (filed 2026-09-05 from the issue-357 campaign)
Kind: policy (organization layer — country semantics)
Relates to: 357 (the campaign that parks this class every slice), 355 (the floor that parks
it), 326 (the census's shared-register blind spot), CONTEXT.md (country semantics)

## What the campaign keeps finding

In every slice of the 357 campaign about 30 high-confidence moves park under the
`shared-register` rule: rows tagged `MQ`, `RE`, `GP`, `GF`, `YT`, `PM` (French overseas
departments and collectivities) carrying a SIREN/SIRET that validates under `FR:siren`, beside
a row tagged `FR` with the same number — and `AX` (Åland) rows carrying a Finnish Y-tunnus
beside an `FI` row. Slice 3: MQ→FR 8, RE→FR 6, GP→FR 4, YT→FR 4, PM→FR 3, AX→FI 4, GF→FR 1.
Examples: `SDIS de la Réunion` under `RE` (5 mentions) and under `FR`; `SCLM Sarl` under `MQ`
and `FR`.

These are not contaminations: Réunion IS France for register purposes (one SIREN space), and
Åland companies sit in the Finnish trade register. The regional code is a finer, legitimate tag
that TED/eForms buyers use inconsistently, so the same entity ends up as two rows. The reviewers
call it `wrong-country` because the arithmetic validates only the parent code; the floor parks
it because which code such a row SHOULD carry is a policy question.

## The decision

Either:

1. **Normalise to the parent code** (`MQ/RE/GP/GF/YT/PM/BL/MF/WF/NC/PF → FR`, `AX → FI`,
   `FO/GL → DK`, `SJ → NO`, `AW/CW/SX/BQ → NL`): the organization's `country` means the
   register's jurisdiction; the parked moves apply (re-record as high, R2 folds the pairs), and
   the resolver maps the regional code to the parent on ingest so the class stops reappearing.
   Loses the regional signal on the org row (it stays on the notice/buyer side).
2. **Keep the regional code** as a legitimate country value: the pairs stay as two rows (or
   R2 learns a "same register" arm that folds them while keeping one code by a rule), and the
   floor's park becomes a `keep` verdict class.

Recommendation: (1). The org layer's `country` is documented as the register jurisdiction
(CONTEXT.md), the regional information survives on the notice, and every other consumer of
`country` (identifier schemes, the merge arms, the census) already treats the parent code as
the entity's country.

## Not in scope

Any other one-letter or spray class: those are contaminations and the campaign moves them.

## Decision (2026-09-07, owner): option 1, the register's jurisdiction

`organizations.country` exists to key identity (R2 keys on `(country, kind,
identifier)`), and the identifier's register is the jurisdiction: a SIREN is French
whether the buyer wrote `FR` or `RE`, a Y-tunnus is Finnish under `AX`. Keeping the
regional code on the org row buys a signal that the notice and buyer rows already carry,
at the price of one entity standing as two rows forever. So: `MQ/RE/GP/GF/YT/PM/BL/MF/WF/
NC/PF → FR`, `AX → FI`, `FO/GL → DK`, `SJ → NO`, `AW/CW/SX/BQ → NL` on the org row.

Units: (1) the resolver maps a regional code to the parent when it mints or binds an
org row (ingest; test per code); (2) the parked `shared-register` moves re-record as
high and apply through `apply-country-verdicts` (dry, then wet with the expected count);
(3) R2 folds the reunited pairs on its next run. The 355 floor's `shared-register` park
becomes unnecessary once (1) is live and is removed then.

## Unit 1 (2026-09-07): the fold, at every site that keys on the org row's country

**The list is narrower than first recorded, and the criterion is the reason.** A mapped code
claims "this number is the same series as the parent's". That is true where the parent's
register covers the territory and false where the territory runs its own:

| folds | to | register |
|---|---|---|
| GP, MQ, GF, RE, YT, PM, BL, MF, WF | FR | INSEE Sirene (the five DOM plus the collectivities Sirene covers) |
| AX | FI | Finnish trade register (Y-tunnus) |
| GL | DK | Danish CVR |
| SJ | NO | Brønnøysund (orgnr) |

Kept as their own jurisdiction: **NC** (RIDET), **PF** (numéro Tahiti), **FO** (Skráseting
Føroya, not the CVR), **AW / CW / SX / BQ** (each its own chamber of commerce). The first
recording listed those under their ISO parents; a RIDET keyed as a SIREN would be a wrong
claim, so they stay. WF is the least certain Sirene entry (one national-id row on prod) —
the arm's comment in `store::register_jurisdiction` is where to correct it.

**Sites** (one helper, `store::register_jurisdiction`, applied wherever the org row's
country is derived or an identifier is classified by country):

- `ingest::project::normalise_identifier_with` — `Identifier.country` maps before the
  country-specific folds and the idgate, so a Y-tunnus under `AX` is checked as the Finnish
  number it is (a failing one is refused like one under `FI`). The mention's own `country`
  is untouched: `organization_mentions.country` keeps `RE`.
- the resolver's provisional (name-only) path — `(name_norm, country)` reuse and both
  provisional INSERTs use the folded country, so `SDIS de la Réunion` under `RE` and under
  `FR` is one provisional row.
- `resolver_canon_key` (the Stage-2 canonical pre-probe, at preload AND at bind) — the
  BIND half: a standing pre-358 row under `RE` and a new mention keyed `FR` share one
  prevention key, so the mention binds to the standing row instead of minting the twin R2
  would fold back (which would have been a mint-fold-mint loop until unit 2 moves the row).
- the R2 planner's and the E0/R3 scan's row-country fold (beside the EL→GR fold), so the
  VAT-prefix agreement check and the group scope read the register's code.
- `ingest::idgate::census` (`cc`) — a SIREN under `RE` scores `FR:siren`, the census and
  the gate read the row the way the resolver mints it.
- `ingest::crosswalk::canonical_key` — keys in the register's series, so the standing
  `RE`/`FR` pair shares an E1 key (unit 3's fold).

**Tests:** `register_jurisdiction_folds_only_codes_whose_register_is_the_parents` (the table
is the decision), `a_regional_code_mention_mints_under_its_register_jurisdiction` (store,
provisional path; the mention row keeps `RE`; NC stays its own),
`a_standing_regional_code_row_takes_the_mention_keyed_under_its_register`
(`resolver_prevention.rs`, the bind half, no twin), `a_regional_code_scopes_its_identifier_to_
the_register` (normaliser incl. the AX checksum refusal and the VAT arm untouched),
`a_regional_code_scores_in_its_registers_series` (idgate), `a_regional_code_keys_in_its_
registers_series` (crosswalk). CONTEXT.md's Organization entry now states the semantics.

**Gate and deploy:** `ops/check.sh` GATE-EXIT=0, 861 passed; committed `e988e58`, pushed to
`main` and the handover branch. First `./deploy.sh origin/main` (gate green on the box in
343 s) REFUSED the restart because the daily `project` job 781 had started meanwhile — the
right refusal; redeployed at the next idle window (below).

**Standing rows on prod (bounded index read, 2026-09-07):** identifier rows under the mapped
codes — RE 252, MQ 130, GP 93, AX 81, YT 37, GF 25, PM 14, GL 15, MF 2, WF 1, SJ 1 (≈650);
provisional rows RE 2,449, MQ 1,123, GP 948, GF 352, YT 176, AX 40, GL 18, WF 10, PM 6,
MF 3, SJ 2. Unmapped: NC 36/21, PF 49/16, FO 9/8, AW 149/3, CW 8/4, SX 2/0, BQ 2/7.

**Unit 2, sharpened by unit 1's shape:** the identifier rows are the ones that key (R2, the
pre-probe); the provisional rows only reuse by `(name_norm, country)` and fold naturally as
mentions re-project. So unit 2 = record `move` × `high` verdicts (cohort
`358-register-jurisdiction`) for every identifier-bearing org row under a MAPPED code, from
a bounded listing, and apply through `apply-country-verdicts` dry → wet with the expected
count; collisions with the standing parent-code twin are counted and left for R2 (unit 3),
exactly the 355 path. Run it before the next R2 wet, so R2's survivor already carries the
register's code. The 355/357 campaign floor's `shared-register` park stays correct for the
UNMAPPED codes only; a future floor should list just those.

## Units 2 and 3 (2026-09-07): the standing rows move, R2 folds the pairs

Run after the unit-1 deploy (`5b09a9c` live, queue idle — the first deploy attempt refused
the restart under the daily `project` job and was repeated at the idle window).

1. **Record.** `POST /admin/country-verdicts`, cohort `358-register-jurisdiction`: 651
   `move` × `high` verdicts, one per identifier-bearing org row under a mapped code (body
   committed as `358-campaign/358-post-body.json`, built from the bounded listing
   `standing-identifier-rows.json`). Recorded 651.
2. **Dry plan** (job 783): 2,657 pending verdicts corpus-wide, 651 eligible, 651 would move,
   0 no-ops, **313 land on a triple another row holds**. The plan's tuples equal the recorded
   set exactly (RE 252, MQ 130, GP 93, AX 81, YT 37, GF 25, GL 15, PM 14, MF 2, WF 1, SJ 1);
   1,596 mentions on the movers. Record: `358-campaign/country-verdict-plan-2026-09-07.json`.
3. **Wet** (job 784): 651 rows moved, 0 no-ops, 313 collisions counted and left. Bounded
   re-read: **0 identifier rows remain under any mapped code.**
4. **R2 dry** (job 785) against the previous dry (2026-09-06): groups ≥2 1,113 → 1,303,
   plan 0 → **321 groups** (318 carry a moved row); 259 of the 313 collision pairs sit
   fully inside a plan group; 31 pairs sit in the name gate's denied listing (142 vs 105
   before) — some rightly (`Ville de Baillif` and `Caisse des écoles` share one SIRET and
   are two bodies), some the issue-362 review shape (`SEMAVIL — SAEML` beside `Soc mixte
   aménag Ville Lamentin`, `NPEI` beside `Nalem peinture étanchéité isolation`); the rest
   fall to the consortium / legal-form guards. Record: `358-campaign/r2-merge-plan-2026-09-07.json`
   (plan and denied listings, both complete under the cap).
5. **R2 wet** (job 786): 321 groups merged, 354 org rows removed, 1,894 mentions, 3,687
   parties, 552 bid-parties, 1,017 winners repointed, 1,134 tenders touched. `project`
   (job 787): 0 notices → 0 tenders, nothing to rewrite. Spot-check: `SCLM SARL`
   (303171573), three rows under MQ/MQ/FR before, is one row under FR (15435311);
   `Territoire de la Côte Ouest` likewise (9852812). /health ok, journal clean.

**What is left, and whose it is.** The 31 name-gated pairs are issue 362's queue (a merge
verdict per group). The GL→DK rows carry 8-digit CVRs no crosswalk arm keys, so their twins
are E0's (issue 329, the rule whose enqueue the classifier refuses). The 16 AX rows with
the glued `FONR`/`FONUMMER` label are issue 363. The provisional (name-only) rows under the
regional codes (RE 2,449, MQ 1,123, GP 948 …) were not moved: they carry no key, the
resolver now reuses them by `(name_norm, register)`, and a later identifier canonicalises
them — a bulk move would buy nothing the next projection does not.

**The floor.** The 355/357 campaign floor's `shared-register` park is now wrong for the
mapped codes (their moves are policy, applied) and right for the unmapped ones (NC, PF, FO,
AW, CW, SX, BQ). A future campaign's `post.py` should park only those seven.
