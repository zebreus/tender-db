# 545 unit 3, refute round 2, row 2: Wyre Forest x2 (item b)

The row: Shropshire Fire Rescue Service (31560408) holds 2 mentions printed "Wyre Forest District Council". The cohort moves them to 31550990.

| notice | section | tender | role | target |
|---|---|---|---|---|
| 46826419 | ORG-GB-PPON-PTQY-8812-XHLH | 8660381 | Procedure-Buyer | 31550990 |
| 46826428 | ORG-GB-PPON-PTQY-8812-XHLH | 8660381 | Procedure-Buyer | 31550990 |

**Result: neither mention is refuted. Both rehomes stand.**

## GETs (4 of 4, logged with "(refute2)")

The fetch script is `refute2_2_get.sh`. It runs under the shared flock, waits at least 2 s between requests, and does not retry.

1. `/v1/organizations/31560408`. Saved as `bodies/v1_organizations_31560408.json`.
2. `/v1/tenders?buyer=31560408&limit=20`. Saved as `bodies/v1_tenders_buyer_31560408_limit_20.json`.
3. `/v1/notices/46842882/content`. Saved as `bodies/v1_notices_46842882_content.json`. This is the third mention, which the cohort leaves out.
4. `/v1/organizations?name_prefix=Wyre%20Forest&limit=50`. Saved as `bodies/v1_organizations_name_prefix_Wyre%20Forest_limit_50.json`.

Bodies I reused without fetching again: `v1_notices_46826419_content.json`, `v1_notices_46826428_content.json` and `v1_organizations_31550990.json`.

## Evidence

**The holder is Shropshire Fire & Rescue Service, and PTQY-8812-XHLH is its own PPON.**
- 31560408 serves the name "Shropshire Fire Rescue Service" with identifier GBPPONPTQY8812XHLH. It has 8 mentions, is not provisional and has no merged identifiers.
- Per `u2b/tail_k6.json`, those 8 mentions are printed under three names:
  - "Shropshire Fire and Rescue Service" 3 times;
  - "Shropshire Fire Rescue Service" 2 times;
  - "Wyre Forest District Council" 3 times.
- So the body prints this PPON as its own 5 times.
- It is the buyer on 6 tenders. Four are its own fire-service buys:
  - 8637642, Project Portfolio Management Software;
  - 8640097, "SFRS - Control & Incident Room AV";
  - 8641476, Civica CFRMIS G-Cloud call-off;
  - 8846460, Emergency Response Equipment framework 2026-2033.
- The other 2 are the Wyre Forest tenders, 8660381 and 8677275.
- None of its own tenders involves Wyre Forest District Council (WFDC).

**Notices 46826419 and 46826428 (same tender 8660381, ocds-h6vhtk-06ac7d).**
- The two notices are identical except for the value (0 on one, GBP 35m on the other) and the publication time (13:50 and 13:59). Both bodies were read in full.
- The title is "Lion Joint Venture - SPV - Wyre Forest District Council".
- BT-24 says: "The Authority (Wyre Forest District Council) wishes to appoint a Development Partner for the redevelopment of the Lion Fields site in Kidderminster". "Authority" is singular, and no partner authority is described.
- There are three OPT-300-Procedure-Buyer refs, in this order:
  - ord 0: WFDC on PTVB-7542-QLLX, its own PPON;
  - ord 1: **WFDC on PTQY-8812-XHLH**, the section under review;
  - ord 2: West Mercia Police on PJHH-5271-ZMQP.
- Neither body contains "Shropshire" or "Fire".
- Section PTQY is referenced exactly once, by Procedure-Buyer ord 1. No other role points at it: no tenderer, review body or contact. So the section binds only this party.

**The target 31550990 is the printed entity.**
- 31550990 serves the name "Wyre Forest District Council" with GBPPONPTVB7542QLLX. It has 11 mentions, is not provisional and has no merged identifiers.
- The same two notices print that PPON beside the same name as buyer ord 0. That makes it the same legal entity.
- The name-prefix scan returns 33 "Wyre Forest" rows. 31550990 is the **only identified, non-provisional** WFDC row.
- The scan also shows provisional WFDC name rows: 1299537 (218 mentions), 2456347 (2) and departmental variants. They have no identifier, so they cannot compete as the destination for a PPON-bearing section. They are an identity split of the provisional-name class, not 545. Recorded below as a follow-up.

**Pattern check from the third mention (46842882, tender 8677275, "Worcester Home Adaptation Service", UK7, outside the cohort).**
- BT-24 says: "Wyre Forest District Council would like to invite tenders for … Lot 1 – Level Access Showers, Lot 2 – Ramps, Lot 3 – Major Works".
- It has two buyers: "West Mercia Police and Crime Commissioner" (PJHH-5271-ZMQP) and **"Wyre Forest District Council" on PTQY-8812-XHLH**.
- WFDC's own PPON is not printed on this notice.
- So WFDC's publisher puts its own name on Shropshire's PPON across two unrelated WFDC tenders: a site JV, and a home-adaptations framework. That is a repeated template or account error on the publisher's side. It is not a one-off joint procurement with Shropshire, which has no conceivable buyer role in either buy.

## Refutation attempts, one per question

1. **Is the printed name a different legal entity from the holder? Yes.**
   - WFDC is a Worcestershire district council (Kidderminster).
   - The holder is the Shropshire & Wrekin combined fire authority.
   - The name is not a trading name or a former name: the holder prints its own name on its own tenders.
   - They are not sister bodies: no common parent, and a different county. Kidderminster's fire cover is Hereford & Worcester, not Shropshire.
   - The holder is not a consortium member here, because Shropshire is not named on either notice.
   - The holder itself is not what is printed: the name says WFDC and the text says WFDC.
2. **Is the target the same legal entity as the printed name? Yes.** The identifier match is shown on the same notices, and 31550990 is the only identified WFDC row.
3. **Does the section bind only this party? Yes.** There is a single ref, Procedure-Buyer ord 1, on both notices.
4. **Would the move lose information? No.**
   - The "holder is the real buyer and the printed name is its delegate" case is the reverse of what the notices print: the text names WFDC as the sole Authority.
   - Agent reading: Shropshire F&RS published on WFDC's behalf and the name was edited. Nothing printed supports it: no "on behalf of", no Shropshire name. Even if it were true, keeping the mention serves the false claim "Shropshire F&RS is a buyer of a Kidderminster housing JV".
   - The PTQY identifier stays in the notice content either way.
   - After the move, the holder keeps 6 mentions: its 5 own and 46842882. So it is not emptied.

## Notes for unit 4 and follow-ups (not refutations)

- After the move, tender 8660381 serves WFDC twice as Procedure-Buyer, from two sections, and Shropshire F&RS drops off it. `guards.md` already records this. The dry plan should show the duplicate and should not refuse it.
- **46842882 (tender 8677275) can now be lifted.** It is read, it repeats the same misbinding, and it has a single role (Procedure-Buyer ord 1). It would be a rehome-high to 31550990 in a later cohort. On this tender it would be WFDC's only buyer section, because the own PPON is absent. Unit 3's exclusion reason, "not read", no longer holds. It is still outside cohort 545 as built.
- West Mercia Police / PCC (PJHH-5271-ZMQP) is printed as buyer on both WFDC tenders, apparently from the same publisher-side template. The name matches its identifier, so this is not 545, but the buyer claim looks false. Flag only.
- Provisional WFDC rows 1299537 (218 mentions) and 2456347 are a name split against the identified 31550990. That belongs to the identity-split classes, not this cohort.

## Verdicts

| notice | section | refuted | why |
|---|---|---|---|
| 46826419 | ORG-GB-PPON-PTQY-8812-XHLH | **false** | The text names WFDC as the sole Authority. WFDC is also printed on its own PPON, which is the target 31550990. Shropshire is never printed. The section has a single Procedure-Buyer ref. The holder self-prints PTQY on its own fire-service tenders. |
| 46826428 | ORG-GB-PPON-PTQY-8812-XHLH | **false** | Same tender and the same buyer block (body read): WFDC on PTVB, WFDC on PTQY, West Mercia Police. The same text, no Shropshire, a single Procedure-Buyer ref. |
