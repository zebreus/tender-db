# 513 unit 1 — results (2026-10-11)

Inputs, all in this directory:
- `nhs-listing-2236.json`: the `registry_nhs_listing` stored by dry altid job 2236 (deployed 027f2ef). 195 GB-NHS ↔ PPON
  pairs, not truncated.
- `ods/`: the ODS ORD record of every code, fetched from the agent container on 2026-10-11 (`ods_fetch.py` lives in the
  session scratchpad). 190 were found; 5 were not: the typed names `NHSCHESHIREANDMERSEYSIDEICB` and
  `WESTMIDLANDSAMBULANCESERVICE`, two PPONs typed under GB-NHS (`PHHT9556BWJJ`, `PWNJ1991NGDW`), and the NUTS code `UKN06`.
- `nhs-ods-joined.json` (`ods_classify.py`): the listing joined with ODS.
- `adjudication-wf_a168eb50.json` → `adjudication-rows.json`: every LISTED holder of the 131 identity-null codes. The listing caps
  registry holders at 5 (`REGISTRY_NHS_REGISTER_HOLDERS`), so 35 of 304 registry holders, in 11 codes, were never listed
  or adjudicated: QHM 16, QYG 11, QMF 10, QKK 8, QRL 8, QOQ 7, and QMJ, RBL, RBN, RJZ and RWW at 6. Every figure below
  is over the listed subset, and the strays may be under-counted (review wf_4e7d846d).. Each was
  classified by one agent and re-checked by an independent skeptic told to refute it (workflow wf_a168eb50-548,
  14 agents). 392 of 393 claims agreed and 0 were disputed. One was left unverified (TAJ's PPON holder, Black Country
  Healthcare NHS FT, which is TAJ's own name), so it counts as SAME.
- `mention-counts-by-org.jsonl`: each holder's mentions grouped by `raw_identifier`. 16 bounded `/v1/sql` seeks on
  `organization_mentions_org`, 25 org ids each; no 408s.

## 1. Which codes mint no identity

Of the 195 codes, 131 mint no identity and 64 do. All 131 are in the digit rule's family:
- 122 are three letters;
- 7 are three characters whose only digit is `0`: `0AR 0CX 0DE 0DF R0A R0B R0D` (the all-zero filler rule);
- 2 are names typed into the identifier field.

By lead character, the 129 three-character codes start with R (99), Q (23), 0 (4), T (2: `TAD`, `TAJ`, care trusts) and
D (1: `DQF`, an independent-sector provider).

## 2. What ODS says

All 129 three-character codes are **Active RC1 organisation-level records**:
- NHS TRUST: 98;
- STRATEGIC PARTNERSHIP (an ICB): 23;
- COMMISSIONING SUPPORT UNIT: 4;
- CARE TRUST: 2;
- INDEPENDENT SECTOR HEALTHCARE PROVIDER: 1.

None is a site or a sub-unit. The two typed names are not ODS records.

Among the 64 codes that already mint, the picture is mixed:
- sites: `NCS04`, `U6N4T`;
- sub-ICB locations (the `\d\d[A-Z]` CCG codes such as `06H`);
- PPONs and a NUTS code typed under GB-NHS.

So the digit rule is not what separates organisations from noise here. It refuses 129 real organisation codes and
admits sites and sub-units.

## 3. Multi-target codes: name variants of one body, plus a few strays

Across the 393 holders of the 131 identity-null codes:

| class | registry holders | PPON holders | coded mentions (registry side) |
|---|---:|---:|---:|
| SAME (the code's own body: variant, abbreviation, typo, predecessor name) | 238 | 117 | 7,408 |
| HOSTED_OR_SITE (a hosted unit, collaborative or hospital site of the body) | 9 | 1 | 15 |
| DIFFERENT (another legal body — almost always a neighbour's code picked by mistake) | 19 | 5 | 68 |
| NOT_ORG (`TEST`, a tender title, framework eligibility wording) | 3 | 1 | 4 |

- **Strays are 0.96% of coded mentions** (72 of 7,495). The largest are Alder Hey under `RET` (16), North of England CSU
  under `QHM` (14), and the Walton Centre under `REN` (9 + 6).
- They follow the pattern of a dropdown picked one row off: QKK ↔ QMF (south-east and north-east London ICBs, swapped
  both ways), REN ↔ RET, RJN → RJR, RWW → RWV, RXX → RX2, R0D → RDY, RJE → RRK.
- **61 of the 73 multi-target codes have 2–5 SAME registry holders**: one body split by spelling. The other 12 have one
  SAME holder plus strays.
- **53 of the 54 identity-null `both_distinct` codes are the same body on both sides.** The register org and the PPON org
  are one NHS body, so U7 would pair them. The exception is `RGT` ↔ Teleflex, a supplier's PPON.

## 4. What the uncoded mentions do

The 238 SAME registry holders carry 7,408 coded mentions and **48,360 uncoded ones**. Only 36 holders have no uncoded
mention.

The resolver binds an identifier-less mention only to a same-name row with `identifier IS NULL` (issue 234,
canonical.rs ~17346), never to a canonical row. Any identity the exemption mints therefore stands beside a provisional
twin that holds the body's uncoded mentions.

That is already the state of every digit-coded NHS body. Bounded `name_norm` seeks, 2026-10-11:

| body | provisional twin | PPON org | NHS code org | further code orgs |
|---|---|---|---|---|
| South Yorkshire ICB | 30913756 | 30918122 | 31538551 (`QF7`) | 31593407 (`03N`, a sub-ICB location) |
| Isle of Wight NHS Trust | 2696414 | 31571667 | 31542853 (`R1F`) | 31611764 (`RF1`, a transposed typo) |
| Derby & Derbyshire ICB | 30967466 | — | 31545866 (`QJ2`) | 31606796 (`15M`, a sub-ICB location) |
| Essex Partnership UFT | yes | yes | yes | — |
| Herefordshire & Worcestershire H&C | yes | yes | yes | — |

The sub-ICB location codes and the typo codes are filed separately as issue 519.

## 5. Wider tails on the same orgs (review wf_4e7d846d, verified)

The holders' own mentions carry 22 three-character digit-rule-family tails that are not in the PPON-paired listing.
Only `RWK` and `AAV` are the holder's real code. The others:
- 404 at ODS: `ICB` (on two different ICBs), `NHS`, `TBC`, `OYG`, `ROB`, `Q0X`, `ODX`, `QYC`, `QKJ`, `QLG`, `0YD`, `OYD`,
  `UHS`, `KGH`, `RGY`;
- inactive trust codes on other trusts: `RCN`, `RMB`;
- sub-ICB locations: `00N`, `00P`;
- another body's code: `NGH` (Amplifon HQ).

So the 129 ODS-confirmed codes are a selected sample. A shape rule alone would mint junk identities, and one of them
(`ICB`) would fuse two bodies.

## 6. Coverage of a body by its coded mentions (review wf_4e7d846d)

The median coded share of a body's SAME registry mentions is 9.7%; 65 of 130 codes sit below 10%. Org counts (SAME
registry clusters plus distinct SAME PPON orgs):

| | orgs |
|---|---:|
| today | 355 |
| after a mint and mention re-bind, before 469 U7 | 448 |
| after U7 | 332 |

Per code after U7: 92 unchanged, 26 better, 13 worse.

## 7. Unit 2 decision

See issue 513. The first decision was withdrawn after review.
