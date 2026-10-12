# 519 unit 1 — every GB-NHS identity on prod, classified (2026-10-12)

## Method

All reads were bounded and indexed; there were no 408s.

1. **The identities.** `organizations_identity(country, identifier_kind, identifier)` serves the prefix as a range seek:
   `country='GB' AND identifier_kind='national' AND identifier >= 'GBNHS' AND identifier < 'GBNHT'`. It returns **2,057
   orgs** (`gbnhs-orgs-2026-10-12.json`), all non-provisional and each with a distinct value. The issue's "no index"
   premise was wrong: the identity index leads with country and kind.
2. **Mentions** per org: `organization_mentions_org`, 100 ids per read. Total 8,049; every org has at least one.
3. **Same-name twins** by `name_norm` (`name-twins-2026-10-12.json`).
4. **ODS.**
   - ORD records for the 1,733 ODS-shaped values outside 513's frozen table: `ods/`, fetched by `fetch_ods.py`.
   - ORD records for the 66 table codes with role RO98 or RO261, for their dated parent relation: `ods-table-codes/`.
   - The role names: `ods-role-names.json`.
   - The classification itself: `gbnhs-orgs-classified.json`.

## Classes

| class | orgs | mentions |
|---|---:|---:|
| A sub-ICB location (RO98) | 52 | 1,844 |
| B NHS trust site (RC2 RO198) | 25 | 36 |
| C NHS England and national body codes (RO209/RO210/RO162/RO116 and their sites) | 24 | 383 |
| D NHS organisation (trust, ICB, CSU, SpHA, health board; the body's own digit code) | 96 | 2,495 |
| E not an ODS code (404; e.g. `110001` on Gloucestershire Hospitals) | 52 | 130 |
| F foreign scheme or junk typed under GB-NHS (not ODS-shaped) | 195 | 387 |
| G other site (ISHP, pharmacy, branch surgery, PCT, LA sites) | 86 | 106 |
| H other RC1 organisation (GP practice 719, pharmacy 613, ISHP 50, optician, dentist, hospice, PCN, LA) | 1,527 | 2,668 |

**H is correct as it stands.** A GP practice's or pharmacy's own ODS code is its identity. One wrinkle: a chain stands
as one org per premises (Boots 64, Day Lewis 17, Asda 9). Every pharmacy premises is its own ODS contractor code, and
ODS keeps them apart too. That is a chain-level question for a supplier view, not 519's.

**F** holds:
- Companies House numbers (8 digits, 61);
- PPONs (16) and OCDS ids (5);
- concatenated pharmacy codes (`FL317FEN74…`, up to 7 glued codes);
- CQC registration numbers;
- typed names (`NHSTRUSTMERSEYCARENHSFOUNDATIONTRUSTRW4`, `Y63NORTHEASTANDYORKSHIRE…`).

Each mints an identity beside the real one. Mersey Care, for example, has RW4 (180 mentions) and the typed-name org
(1 mention).

## A — sub-ICB locations: the ICB publishing under a child code

Of the 52, **39 orgs / 1,170 mentions carry their parent ICB's name** (token overlap ≥ 0.5 against the name of the
parent's ODS record, now or before the 2026-04 mergers), and that parent's 513 body already stands (`sub-icb.json`):
- B2M3M "NHS Coventry and Warwickshire ICB", 600 mentions, beside the QWU body 31542351 (89);
- 26A "NHS Norfolk and Waveney ICB", 115, beside the QMM body 31542073 (112);
- 92G (80) beside QOX; D4U1Y (65) beside QNQ; 15E (46) beside QHL; 91Q (41) beside QKS; 36L (33) beside QWE;
  15N (31) beside QJK; 11X (30) beside QSL; and so on.

The rest:
- 9 more name-match a parent whose body is still in 513's 18-code residue: 72Q → QKK, A3A8R → QMF, 15C → QUY, 02E and
  27D → QYG, 93C → QMJ, 15A → S0E4D (a 2026 ICB with no body yet).
- **13T (504 mentions) is not the ICB.** It is "North of England Commissioning Support" (the CSU, 0AR's body),
  publishing under NENC's sub-ICB code.
- 03K, 14J and 14N have no RO261 parent in ODS. 03Q and 04Y name another body.

The parent is dated (RE5, Operational start and end). The 2026-04 mergers moved children:
- 26A from QMM to T6Y0W;
- D4U1Y from QNQ to QRL;
- 06H from QUE to S1Y5D;
- 93C from QMJ to Z9B2Z;
- and others.

So a fold target must be the body the org's NAME is, among the code's parents over time. It must never be today's
parent by rule: 26A's notices are Norfolk and Waveney's (QMM), not Norfolk and Suffolk's.

## B, C — sites and NHS England

- **B.** Trust sites stand beside their trust. Examples: A2X8Q "Kingston Hospital NHS FT" (a site code); RBT20 "Mid
  Cheshire Hospitals"; RJ128 "King's College Hospital". The names are usually the trust's own, so these split the
  trust, but only 36 mentions in total.
- **C.** NHS England stands as many orgs:
  - X24 "NHS England" (95) and Y56 (86, "London Commissioning Region");
  - the Q4x/Q7x local offices and Y5x/Y6x regions;
  - Y54, Y61 and Y62, which are coded on trusts and surgeries (mis-coding).

## Mis-coding

36 NHS-family codes (808 mentions) carry a holder name that does not match ODS (`miscoded-nhs-family.json`).
- Most are sub-ICB codes that ODS renamed after the mergers ("NHS CENTRAL EAST ICB - 06H"). Those are not mis-coding.
- The real ones:
  - 13T (the CSU);
  - RJ7 "Kingston and Richmond" (ODS: St George's);
  - Y54 / Y61 / Y62 on trusts and a surgery;
  - D8D1G (a provider collaborative hosted by BWC);
  - RH8C3 "NHS Devon ICB" (ODS: Vranch House, a site).
- Typos: RF1 for R1F (Isle of Wight), T6YOW for T6Y0W (Norfolk and Suffolk), QHL15E (two codes glued), Q75Q83Q84.

## Bearing on 469 U7

U7 pairs a register identity with its PPON. Class D (the body's own digit code) pairs one-to-one already. Class A is
what makes "one org per body" false for ICBs: the ICB's body plus up to 5 sub-ICB orgs (NENC has 01H, 16C, 84H, 99C
and 13T). Classes B, C and F add little mass.

## Unit 2 options (for the decision)

- (a) **Extend 513's reviewed nhs-body verdicts to child codes.** A verdict for an ICB (or trust) body may list, as
  members, holders of the body's RO98 (resp. RO198) child codes under any dated parent. The same reviewer + challenger
  discipline applies, with members-exact, `reviewed_holders` over the child codes' live holders, the ledger and the
  unwind. This needs a frozen child map in `nhs-ods.tsv` (code, parent, start, end) and a member rule in
  `plan_nhs_bodies`. 13T-like holders are excluded by the verdict, not by rule.
- (b) Keep sub-ICB locations apart as buying points (the issue's option b). The evidence is against it: the orgs carry
  the ICB's name, not a location's.
- (c) Typo and foreign values (F, E, RF1, T6YOW): 452's identifier verdicts, `wrong` with a `correct_identifier`, then
  the re-key arm. These are per-org reviewed rows, small.
- NHS England (C): a separate question; X24 is the legal body, and the regions are its offices. Defer, and file it if
  wanted.
