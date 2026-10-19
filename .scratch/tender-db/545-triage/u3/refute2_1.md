# 545 unit 3, second skeptic pass: Hounslow x3 (refute2_1, 2026-10-19)

Row: holder 30913851 (London Borough of Wandsworth, PPON PVLQ-8225-QVHT), printed "London Borough of Hounslow",
rehome to 30918222. Guard review item (b). Tender 8639243, framework 2425-0045 (CCTV), 4 boroughs.

**Result: all 3 mentions stand (refuted=false).** A sibling notice of the same procedure settles it, independently of the
PPON: the framework's own description lists the three other boroughs by name. Hounslow is one of them. Wandsworth is not.

## GETs (4 new, all rc=200, no retries; script `refute2_1_get.sh`, log `refute2_1_gets.log`)

1. `/v1/organizations/30913851`: London Borough of Wandsworth, GBPPONPVLQ8225QVHT, 214 mentions, not provisional.
2. `/v1/tenders/8639243`: 3 versions, all UK7 (46825561 on 06-01, then 46381132 and 46381196 on 09-17). Live parties: Islington
   30913699, Ealing 30913772, **mention "London Borough of Hounslow" on 30913851 (Wandsworth)**, Southwark 30913870, and Tyco as Tenderer.
3. `/v1/tenders?buyer=30913699&cpv=35&limit=50`: Islington's CPV-35 tenders. 8639242 is the same framework's UK3, UK4 and UK6 chain.
4. `/v1/tenders/8639242`: same OCDS process (ocds-h6vhtk-05570f), Islington as sole Procedure-Buyer. The description says:
   "procure a single supplier framework agreement ... including three other boroughs ... Those organisations are:
   London Borough of Ealing, London Borough of Hounslow, London Borough of Southwark". Wandsworth occurs 0 times.

Reused: the 3 notice bodies, `v1_organizations_30918222.json`, and the unit 2b name pool (`u2b/tail_k6.json`).

## Checks

| Question | Finding |
|---|---|
| Printed name a different legal entity from the holder? | Yes. Hounslow and Wandsworth are separate statutory London borough councils. Neither is a trading name, former name, agent or consortium member of the other. Wandsworth's shared service is with Richmond, not Hounslow. The holder pool shows PVLQ as Wandsworth's own number: 189 "London Borough of Wandsworth" + 22 "Wandsworth Borough Council" + 3 "Hounslow", and those 3 are all on this tender. |
| Name or PPON wrong? (the real risk: name typo, PPON right) | **The PPON is wrong.** In the 3 UK7s, names are typed free-hand: Islington appears as "Islington" in two versions and "London Borough of Islington" in the third. So a name typo was possible. But the UK4/UK6 procedure text for the same framework names Hounslow as a participating borough and never names Wandsworth. After the move, 8639243's 4 buyers match that list exactly (Islington + Ealing, Hounslow, Southwark). |
| Target = printed entity? | 30918222: "London Borough of Hounslow" (exactly the printed name), GBPPONPQYY1684DYLV, 118 mentions, identified, not provisional, no merged identifiers. Hounslow's own PPON row. |
| Section binds only this party? | Yes, in all 3 notices: `ORG-GB-PPON-PVLQ-8225-QVHT` has one BT-500 ("London Borough of Hounslow") and is referenced once, as OPT-300-Procedure-Buyer ordinal 2. It is not a Tenderer. Counts per body: Wandsworth 0, PVLQ 3 (section id, BT-501, reference), Hounslow 1, PQYY 0. |
| Information lost? | No. The holder is not a party to this framework, so the current binding is a false Wandsworth buyer role. Wandsworth keeps its other 211 mentions. Hounslow is printed nowhere else on 8639243, so the move adds no duplicate Hounslow buyer (unlike Wyre Forest). |
| Cohort entries | org_id 30913851, the 3 notice ids, section id exact, action rehome, target 30918222 with its served name, confidence high. Each (notice, section) appears once in cohort.json. |

## Caveats for unit 4 (none refutes)

- The rationale phrase "Wandsworth's PPON" rests on the holder's mention pool (211/214), not on a register check. The procedure text makes the rehome right either way.
- 8639243's `procedure_key` is built from identifiers, so it still carries GBPPONPVLQ8225QVHT (it is `refused:` anyway). The rehome moves the mention, not the key.
- The UK6 tender 8639242 binds only Islington as buyer. The three other boroughs appear only in its description, so nothing there needs moving.
