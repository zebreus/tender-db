# 545 u3, second skeptic pass, row 7: The Law Society x1 -> provisional 14894295. Result: NOT REFUTED (2026-10-19)

**Verdict under test** (`cohort.json`; u3.md rehome #17; review_3 row 204; refute_3 flagged it weak):

| org_id (holder) | notice_id | section_id | target_org_id | target_name |
|---|---|---|---|---|
| 30926847 "Network Rail" (GBPPONPZLB9446RVML, 9 mentions) | 46855540 | ORG-GB-PPON-PZLB-9446-RVML | 14894295 (provisional, GB, no identifier) | The Law Society |

This is the row's only verdict in `cohort.json`: action `rehome`, confidence `high`. `record.json` gives tender 8676092 and role OPT-300-Tenderer.

**GETs.** I made 4 new public GETs, all rc=200, with no retries. They ran under the shared `.get.lock`, at least 2 s apart, and are logged with the `(refute2)` tag in `api_requests.log` and `refute2_7_gets.log`. The script is `refute2_7_get.sh`, a copy of `refute2_6_get.sh` with only the header and log name changed. The bodies are in `bodies/`.
1. `/v1/organizations/14894295?fresh=1` (11:01:58Z). I wanted a fresh read of the target, but it fired by accident in the same shell command that created the script, and it carries a stray `?fresh=1`. The API ignores that parameter. The body is `v1_organizations_14894295_fresh_1.json`, and it is identical to reviewer 3's 09:50Z read.
2. `/v1/tenders?winner=14894295&limit=50`: 0 items.
3. `/v1/tenders?bidder=14894295&limit=50`: 0 items.
4. `/v1/tenders?buyer=14894295&limit=50`: 0 items.

I also reused these saved bodies: `v1_notices_46855540_content.json`, `v1_tenders_8676092.json`, `v1_organizations_14894295.json` and `v1_organizations_30926847.json`. From other passes I reused u2a's `r8_dest.out.json` (the `the law society` and `law society` name ranges) and refute2_6's list of the holder's 9 mentions.

## 1. Is the printed name a different legal entity from the holder? Yes
- The holder's head name is Network Rail, the rail infrastructure manager. "The Law Society" is the chartered professional body for solicitors in England and Wales; its legal name under its Royal Charter is "The Law Society". It is not a trading name, former name, agent, consortium member or sister body of Network Rail.
- The notice is National Highways paying solicitors' practising certificates: title "The Law Society - Practising Certificates (SRA)", description "To allow for qualified solicitors to ensure they remain authorised to practise law", GBP 75,000, 2026-08 to 2031-07. Network Rail has no part in that.
- The holder's name appears nowhere in notice 46855540.
- **Inverted-premise attack: could PZLB-9446-RVML be the Law Society's own PPON?** If it were, the fix would be a rename or identifier verdict, not a move. It is not:
  - The Law Society is only 1 of the PPON's 9 mentions.
  - The same buyer, National Highways, printed this PPON for RSS International (46709309).
  - Two of the other mentions are "Network Rail" / "NETWORK RAIL".

  So the PPON is either Network Rail's own or a magnet value. In neither case is it the Law Society's.
- **SRA attack: is the real counterparty the SRA, not the Law Society?** The SRA is the Law Society's regulatory arm. The Law Society is the approved regulator, and practising certificates are issued under its authority, so the printed name is the right legal entity. Even if a separately registered SRA row existed, moving the mention there would contradict the printed name. I did not spend a GET on an `SRA` name range, so this rests on background knowledge, not the API.

## 2. Is target_org_id the same legal entity as the printed name? Yes, by the resolver's own identity rule
- 14894295 is served live as "The Law Society", GB, provisional, with no identifier, no merged identifiers and 1 mention. The 11:01Z and 09:50Z reads agree, so the row was not swept or renamed during the review.
- **I tried to corroborate the target the way refute2_6 did for Compendium, and it did not work.** Its one existing mention is not bound as buyer, winner or bidder on any tender (GETs 2-4 returned 0 items). It is probably unbound or in another role, and the public API cannot show which notice it is on. So, unlike Compendium, no second notice confirms what the row stands for.
- **Why that does not leave the identity open:** a provisional row's identity is defined by its name. The `/docs` note and the `the_provisional_note_describes_what_the_resolver_actually_does` test in `crates/app/src/v1/sql.rs` (issues 234 and 351) both say so: identifier-less mentions with the same normalised name and country resolve to ONE provisional row.
  - Had National Highways printed "The Law Society" / GB with no PPON, ingest would have bound the mention to 14894295 itself.
  - So the rehome puts the mention exactly where the resolver would have put it without the wrong PPON.
- **No competing body has this exact name in GB.** The Scottish and Northern Irish bodies' legal names include their jurisdiction. The live name ranges (r8) hold:
  - `the law society`: only 14894295 (GB) and "The Law Society of Ireland" (IE).
  - `law society`: "Law Society of England" 12964211 (GB) and "Law Society of England and Wales" 31856251 (no country). Both are provisional and are the same chartered body under longer names. This range also holds the Scotland, Ireland and Skillnet/Finuas rows, which are different bodies.
  - **No identified row in either range.** That includes a Companies House style "LAW SOCIETY (THE)", which would sort inside the `law society` range. So there is no better identified destination to switch to.
  - Whichever of the three England-and-Wales rows was picked, the mention lands on the same entity. 14894295 is the exact-name one.
- The section prints only the magnet PPON and GB, so no identifier on it contradicts the target.

## 3. Does the section bind only this party in that notice? Yes, checked in the saved body
- `PZLB-9446-RVML` appears 3 times in 46855540: the section id, BT-501, and one `is_ref`, which is `TPA-1-0 OPT-300-Tenderer`.
- There is no `#2` section.
- The buyer, National Highways Limited (ORG-GB-PPON-PGMJ-3793-VBBN), has its own section and is the only `OPT-300-Procedure-Buyer`.
- The chain RES-1-1 -> TEN-1-0 -> TPA-1-0 -> this section is the same single mention. The rehome moves the tenderer, bid-party and winner rows of tender 8676092 together, which is intended.
- Tender 8676092 has one version (46855540, UK7). It still serves "The Law Society" under org 30926847 "Network Rail" as Tenderer, bid party and winner, so this mention has not been fixed yet.

## 4. Would moving it lose information? No
- The holder is not the buyer, not a delegate and not an agent. National Highways is the buyer in its own section, and the holder sits only on the Tenderer/winner section.
- Today tender 8676092 serves "Network Rail" as the winner of a GBP 75k practising-certificate contract, which is wrong. After the move it serves The Law Society.
- The only thing lost is the link from this mention to PZLB-9446-RVML. That PPON is not the Law Society's (see section 1), and it stays printed in the notice.
- The holder keeps its 8 other mentions. The Compendium and RSS verdicts on the same holder are separate notices.

## Bottom line
- **refuted = false** for the one rehome mention (46855540, ORG-GB-PPON-PZLB-9446-RVML) -> 14894295.
- **The "weak" flag is narrowed, not lifted.** The target cannot be corroborated by a second notice: its one other mention is not bound to any tender in a buyer, winner or bidder role. But a provisional row's identity is its name and country by design. The printed name is the body's exact legal name, and no other GB body carries it. So the move lands where ingest would have put the mention.
- **Residual risks.** Neither is a reason to hold the verdict back, but unit 4 should know them:
  - The unseen sole mention of 14894295 may be prose-derived. If a later void-prose or phrase cleanup targets the row, read this mention back.
  - The Law Society stays split across 3 provisional rows (14894295, 12964211, 31856251). That is 470/511-style merge debt, not 545 work.
- I made no /v1/sql, POST, ssh or git calls. The only files I created are `refute2_7.md`, `refute2_7_get.sh` and `refute2_7_gets.log`, the 4 bodies above, and the 4 `(refute2)` log lines.
