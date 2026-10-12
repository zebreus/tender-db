# 519 — NHS sub-ICB location codes and typo ODS codes mint identities of their own, so one NHS body stands as several canonical orgs

Status: ready-for-agent — UNIT 1 MEASURED 2026-10-12 (`../519-nhs-digit-codes/unit1-results.md`). 2,057 GB-NHS identity orgs / 8,049 mentions were read with indexed seeks; the identity index serves the prefix, so the dry report was not needed. ODS records were fetched for all 1,733 off-table codes. Most are correct own-codes: GP practices, pharmacies, providers (1,527 / 2,668). The split class is A: sub-ICB locations, 52 orgs / 1,844 mentions. 39 of them (1,170 mentions) carry their parent ICB's name beside its standing 513 body, e.g. B2M3M Coventry and Warwickshire, 600. 13T (504) is the CSU, not the ICB. Parents are dated (the 2026-04 mergers). Also measured: trust sites (25 / 36), NHS England codes (24 / 383), foreign or junk values under the scheme (195 + 52 / 517), and typos (RF1, T6YOW, QHL15E). NEXT: the unit 2 decision. Option (a) is leading: extend 513's reviewed verdicts to RO98 / RO198 child codes via a frozen dated child map, plus 452 identifier verdicts for typos and foreign values. Do it before 469 U7.
Was: needs-triage — filed 2026-10-11 from issue 513 unit 1 (`../513-nhs/u1/u1-results.md` §2, §4).
Kind: data quality (organization identity)
Relates to:
- 513 (letter-only ODS codes; this is the digit-bearing half);
- 469 U7 (the NHS register ↔ PPON pairs);
- 452 (identifier verdicts);
- 300 (the identity ladder).

## What was found

A GB-NHS value with a digit mints an identity under the digit rule, whatever ODS says it is. Bounded `name_norm`
seeks, 2026-10-11, show two kinds of extra identity org standing beside a body's own code org.

**1. Sub-ICB location codes.** These are former CCG codes in the `\d\d[A-Z]` shape. ODS records them as RC1 with the
primary role CLINICAL COMMISSIONING GROUP and the active role SUB ICB LOCATION, which makes them sub-units of an ICB
rather than organisations. Examples:

| ICB | ICB's own code org | sub-ICB location org |
|---|---|---|
| NHS South Yorkshire ICB | 31538551 (`GBNHSQF7`) | 31593407 (`GBNHS03N`) |
| NHS Derby and Derbyshire ICB | 31545866 (`GBNHSQJ2`) | 31606796 (`GBNHS15M`) |

Two related cases:
- `GBNHS06H`: the register org 31537455 is named "Cambridgeshire & Peterborough Integrated Care Board", but ODS names the
  code "NHS CENTRAL EAST ICB - 06H". It sits inside S1Y5D since 2026-04-01, and inside QUE before that.
- Since the 2026-04 ICB mergers, ODS has moved parents. A sub-ICB code's ICB is a dated relationship, not a constant.

**2. Typo codes.** Isle of Wight NHS Trust is `R1F`, and stands as 31542853 (`GBNHSR1F`) and as 31611764 (`GBNHSRF1`,
a transposition). Issue 513's listing found the letter-only equivalents too: neighbour codes picked by mistake
(QKK ↔ QMF, REN ↔ RET, RJN → RJR). Those bind by name today. Under 513's D2 they would bind literally to the wrong body,
at 0.96% of coded mentions.

## Why it matters

Each extra org is a canonical row that no merge arm joins. R2 needs a shared key, and the altid and register arms pair
a register identity only with a PPON. So a reader following the ICB's code misses the tenders published under its
sub-ICB code, and the 469 U7 register pairs would pick up a different org per code.

## Proposed units

1. **Measure.**
   - The distinct digit-bearing GB-NHS values standing as org identifiers, read in-app: a dry report, because
     `organizations` has no index on the identifier prefix.
   - Each value's ODS record class and role (organisation / sub-ICB location / site / not found).
   - For sub-ICB codes, the parent ICB on the notice's date.
   - For each code org, whether a same-name org carries a different GB-NHS code (the typo class).
2. **Decide.** Options:
   - (a) merge a sub-ICB location into its ICB, by verdict or by a dated parent map;
   - (b) keep them apart, since a sub-ICB location is a published buying point, like a site;
   - (c) withhold typo codes through identifier verdicts (452's path).
3. Do it before or with 469 U7, so the NHS register family pairs one org per body.

## Handoff from 513's second decision (2026-10-11)

513 takes the letter-only and zero-only ODS organisation codes through reviewed body verdicts and mints no identity.
The following land here:
- the RO98 sub-ICB location codes, including the 10 Active `00[A-Z]` ones carried by NENC ICB clusters (`00N`, `00P`);
- the typo codes (`RF1` for `R1F`, and the letter-only `ROB`, `Q0X` and `OYG` that 513's census will list).

Their orgs never join a 513 body. 513 U1's frozen ODS table classifies them. A per-(triple, name_norm) pin is a
candidate fix: see 513 design A's U2 in `../513-nhs/u1/design-panel-wf_75976f73.json`. This issue should land before
469 U7 or with it.
