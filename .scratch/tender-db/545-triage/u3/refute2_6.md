# 545 u3, second skeptic pass, row 6: Compendium Regeneration Limited x1 -> provisional 1307252. Result: NOT REFUTED, now corroborated (2026-10-19)

**Verdict under test** (`cohort.json`; u3.md rehome #15; review_3 row 200; refute_3 called it "the weakest destination in the set"):

| org_id (holder) | notice_id | section_id | target_org_id | target_name |
|---|---|---|---|---|
| 30926847 "Network Rail" (GBPPONPZLB9446RVML, 9 mentions) | 46853619 | ORG-GB-PPON-PZLB-9446-RVML | 1307252 (provisional, GB, no identifier) | Compendium Regeneration Limited |

This is the row's only verdict in `cohort.json` (action `rehome`, confidence `high`; `record.json` lists tender 8675109 and role OPT-300-Tenderer).

**GETs.** I made 4 new public GETs, all rc=200, with no retries. They were spaced 2 s or more apart under the shared `.get.lock` and logged with the `(refute2)` tag in `api_requests.log` and `refute2_6_gets.log`. The script is `refute2_6_get.sh` (a copy of `refute2_5_get.sh` with only the header and log name changed). The bodies are saved in `bodies/`.
- `/v1/organizations?name_prefix=Compendium&limit=50`: 18 rows, `more`=false, so this is the complete live range.
- `/v1/tenders?bidder=1307252&limit=50`: 0 items. That is expected: pre-eForms TED publishes no bids, see issue 531.
- `/v1/tenders?winner=1307252&limit=50`: 1 item, tender 4086149, `more`=false.
- `/v1/tenders/4086149`

I also reused these saved bodies: `v1_notices_46853619_content.json`, `v1_tenders_8675109.json`, `v1_organizations_1307252.json` and `v1_organizations_30926847.json`. I also reused u2a's `s6.out.json`, `cand.json` and `r8_dest.out.json`.

## The open doubt is now closed

refute_3 accepted 1307252 only because the decision allows exact-name provisional rows. It wrote: "provisional, GB, 1 mention, very low id ... the API cannot show which notice it is", and it could not rule out "dissolved, name reused". The winner query does show which notice it is.

**1307252's only other mention is a 2011 regeneration development award to the same printed name.** Tender **4086149** is a TED award with these details:
- publication 333079-2011, notice 12487774, procedure `ojs:2009-135284`, competitive dialogue;
- title "Development Agreement relating to the Derby Castleward Urban Village Development.";
- place UKF11 (Derby), CPV 70111000/70112000/70332000/71220000 (property development);
- decided 2011-03-03, awarded value about GBP 120.3m, 2 tenders.

Its parties are:
- buyer Derby City Council (1238765);
- winner mention "Compendium Regeneration Limited", bound to **1307252**.

The award text reads: "Derby City Council has appointed Compendium Regeneration Limited to obtain planning permssion and to carry out the development of a high quality, sustainable, residential led, mixed use, multi-phase development known as the Castleward Urban Village."

What this establishes:
- **The target is a real Compendium Regeneration Limited, and its business is residential-led regeneration development.** The 2026 evidence notice is the same kind of deal: Thirteen Housing Group, "Leeway, Hull - New Build Development", "Packaged Land Deal. Brought to THG by Lovells.", CPV 45000000, GBP 3.32m.
- **Name continuity.** The name is a registered-style name ending in "Limited", and a UK register name belongs to one company at a time. The work in the two notices is the same kind, and the Castleward agreement was multi-phase and long-running. So an unrelated company re-using the name is implausible.
- The Lovell link in the 2026 text also fits the neighbouring row 1307773 "Compendium Group" (24 mentions). As background knowledge, which the API does not show: Compendium is the Lovell/Riverside regeneration partnership, and it developed Castleward. The two notices agree with each other even without this background.
- **Remaining limit.** Neither notice prints a company number, so identity at company-number level cannot be pinned from the public API. That limit is shared by every exact-name provisional destination, and the decision accepts it. Nothing contradicts the identity.

## 1. Is the printed name a different legal entity from the holder? Yes
- Network Rail is the rail infrastructure manager. Compendium Regeneration Limited is a housing regeneration developer. It is not a trading name, former name, agent, consortium partner or sister body of Network Rail.
- The holder is a **magnet PPON** row. u2a's `s6.out.json` lists its 9 mentions under PZLB-9446-RVML as:
  - "Bupa Insurance Limted" x2;
  - "Network Rail" (31146524) and "NETWORK RAIL" (46854245);
  - M Group Services, RSS International Ltd, Taylor Wimpey, The Law Society, and Compendium Regeneration Limited, each x1.

  Different buyers printed these, including Thirteen Housing and National Highways.
- **Could PZLB-9446-RVML really be Compendium's own PPON?** Only 1 of the 9 mentions supports that, and nothing else does. Even if it were, moving this mention to the exact-name row still lands on the same entity. The only cost would be the PPON link at org level, and the PPON stays in the notice. So this attack does not make the move wrong.
- The holder's name ("Network Rail") appears nowhere in notice 46853619.

## 2. Is target_org_id the same legal entity as the printed name? Yes, as far as the public record can show
- 1307252 is served today as "Compendium Regeneration Limited", GB, provisional, with no identifier and no merged identifiers. It has 1 mention, the Derby award above. The saved 09:49Z body agrees with the 10:59Z name_prefix read.
- The live `compendium` range (complete, `more`=false) has **no identified GB row**. The only identified row is Polish (22991761 "COMPENDIUM - CENTRUM EDUKACYJNE", PL 357260753). The other GB rows have different names: 1307773 "Compendium Group" (24), 16970543 "Compendium Living" (1) and 31634969 "Compendium Education & Training Ltd" (1). Group and Living may be sister entities in the same family, but they are not this company. The exact-name row is the precise destination, so there is no better identified destination to switch to.
- The section prints only the magnet PPON and GB, so no identifier on it contradicts the target.

## 3. Does the section bind only this party in that notice? Yes, checked in the saved body
- `PZLB-9446-RVML` appears 3 times in 46853619: the section id, BT-501, and a single `is_ref`, which is `TPA-1-0 OPT-300-Tenderer`.
- There is no `#2` section. The buyer, Thirteen Housing Group Limited, has its own section (ORG-GB-MPR-7522 with PPON PYPT-9749-ZYLR) and is the Procedure-Buyer.
- The result chain RES-1-1 -> TEN-1-0 -> TPA-1-0 -> this section is the same mention. So the rehome moves both the tenderer row and the winner row of tender 8675109, and that is intended.
- `v1_tenders_8675109.json` still shows the mention on 30926847 as Tenderer, bid party and winner, so this mention has not been fixed yet.

## 4. Would moving it lose information? No
- The holder is not the buyer, not a delegate and not an agent here. It sits only on the Tenderer/winner section.
- Today tender 8675109 serves "Network Rail" as the winner of a GBP 3.3m Hull housing land deal, which is wrong. After the move it serves Compendium Regeneration Limited, which now has 2 mentions (Derby 2011 and Hull 2026).
- Nothing about the buyer changes. The PPON stays printed in the notice. The holder keeps its 8 other mentions, and the other magnet-PPON verdicts (RSS, Law Society) are separate notices.

## Bottom line
- **refuted = false** for the one rehome mention (46853619, ORG-GB-PPON-PZLB-9446-RVML) -> 1307252.
- The "flagged weak" caveat from refute_3 is lifted. The destination is no longer a bare name match: its only other mention is a 2011 Derby Castleward regeneration development award to the identical name, in the same line of business as the 2026 Lovell-linked Hull land deal.
- Residual risk: there is no company number on either side. Proving identity beyond that needs a Companies House read, which this pass cannot make.
- No /v1/sql, POST, ssh or git. I edited nothing outside u3, and I made only the 4 GETs above.
