# 358 — Overseas-department and Åland country codes: one register, two tags

Status: needs-decision (filed 2026-09-05 from the issue-357 campaign; Lennart's call)
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
