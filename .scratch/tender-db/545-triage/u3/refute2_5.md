# 545 u3, second skeptic pass, row 5: ERMC Ltd ×1 → 18928807. Result: NOT REFUTED, now corroborated (2026-10-19)

**Verdict under test** (`cohort.json`; u3.md rehome #12; review_2 row 136; refute_2 called it "the weakest of batch 2"):

| org_id (holder) | notice_id | section_id | target_org_id | target_name |
|---|---|---|---|---|
| 31571667 "Isle of Wight NHS Trust" (GBPPONPRBY9272TQGX, 4 mentions) | 46819437 | ORG-GB-PPON-PRBY-9272-TQGX | 18928807 | ERMC Limited |

**GETs.** I made 4 new public GETs, all rc=200, with no retries. They were spaced 2 s or more apart under the shared `.get.lock` and logged with the `(refute2)` tag in `api_requests.log` and `refute2_5_gets.log`. The script is `refute2_5_get.sh` (a copy of `refute2_3_get.sh` with only the log name changed). The bodies are saved in `bodies/`.
- `/v1/organizations?name_prefix=ERMC&limit=50` (`more`=false, so this is the complete list)
- `/v1/tenders?bidder=18928807&limit=50` (`more`=false)
- `/v1/tenders/7967731`
- `/v1/notices/31320657/content`

I also reused these saved bodies: `v1_notices_46819437_content.json`, `v1_tenders_8656068.json`, `v1_organizations_18928807.json` and `v1_organizations_31571667.json`.

## The skeptic's open doubt is now closed

refute_2 passed this row "on name equality alone". It could not rule out a pre-PPON or dissolved ERMC behind the bare key 04489340, and it left open whose PPON PRBY-9272-TQGX is. A bidder query on the destination answers both questions.

**The same buyer prints the destination's number, live, four months later.** `/v1/tenders?bidder=18928807` returns tender **7967731**, "Old Barton conversion for Medina House School (Consultancy)". Its details:
- FTS UK7, publication 085378-2026, published 2026-09-09, place UKJ34 (Isle of Wight);
- contract concluded 2026-04-01, worth £87,085;
- the work is RIBA stages 2 to 6 for converting a school.

Its only notice, **31320657**, has these two sections:
- `ORG-GB-PPON-PGYX-7387-YJXG` is "Isle of Wight Council", the Procedure-Buyer. This is the **same buyer PPON** as on 46819437.
- `ORG-GB-COH-04489340` is "ERMC LIMITED", the single `OPT-300-Tenderer`. It carries two identifiers:
  - **GB-COH-04489340**, which is 18928807's own key;
  - **GB-PPON-PHQD-7479-PRPZ**.
- `/v1/tenders/7967731` shows that mention bound to **18928807** today, as both tenderer and winner.

What this establishes:
- **04489340 is a live company in 2026.** It is ERMC Limited, and it is a construction consultant to Isle of Wight Council. A dissolved predecessor with a reused name is ruled out. The number is printed alongside the name, and companies-register names are unique up to Ltd/Limited.
- **ERMC's own PPON is PHQD-7479-PRPZ, not PRBY-9272-TQGX.** So refute_2's "PRBY could even be ERMC's own PPON" is false. PRBY belongs to someone else, which fits the holder's 3 trust-named mentions. The ERMC section on 46819437 carries a **wrong PPON** with the **right name**.
- **The pattern matches.** On both notices the buyer is IoW Council, the year is 2026, the work is construction professional services (QS and design versus RIBA-stage consultancy), ERMC is the sole tenderer, and each is a direct-award UK7.
- The other bidder hit, 7955870 "Construction Professional Services 2" (FTS UK10, 11 lots), is a construction-consultancy framework. That fits as well.

## 1. Is the printed name a different legal entity from the holder? Yes

- "ERMC Ltd" is a private surveying and design consultancy. The holder is served as "Isle of Wight NHS Trust", a public NHS body.
- No near-miss relation applies:
  - it is not a trading name or a former name;
  - it is not an agent, because the trust holds no role on 46819437 and the buyer is the Council, under its own PGYX PPON;
  - it is not a consortium member, a sister body or the holder itself.
- Westminster House is the Council's care home in Newport. The notice does not mention the NHS anywhere.

## 2. Is target_org_id the same legal entity as the printed name? Yes

- 18928807 is identified, keyed 04489340 (national), served as "ERMC Limited", with 13 mentions.
- "ERMC Ltd" and "ERMC Limited" are equal after legal-form folding.
- The live corroboration on 31320657 is in the section above: same buyer, the COH number printed with the name, and the mention already bound to 18928807.

**The rationale is wrong in one detail.** It says "the only ERMC row". `name_prefix=ERMC` returns **5** rows:

| Row | Key | Mentions | Provisional | Served name |
|---|---|---|---|---|
| **18928807** | 04489340 | 13 | no | ERMC Limited |
| 3642298 | none | 9 | yes | ERMC Limited |
| 25643734 | none (country null) | 3 | yes | ERMC Limited |
| 1538609 | none | 2 | yes | ERMC Ltd |
| 31640454 | none | 1 | yes | ERMC |

- The pick still stands. 18928807 is the **only identified** ERMC row. It holds the number the same buyer prints. The cohort's practice uses a provisional destination only when it is the sole exact-name candidate.
- No ERMC-prefixed row is keyed by a PPON. Within the four permitted GETs I cannot rule out a PHQD-keyed row served under some other name. It would not change the destination, because the live system already binds the COH and PPON section to 18928807.
- Suggested rationale fix: replace "the only ERMC row" with "the only identified ERMC row (four provisional ERMC rows exist), corroborated by the same buyer's 31320657, which prints ERMC LIMITED with GB-COH-04489340 and is bound to 18928807".
- The provisional rows 3642298, 25643734, 1538609 and 31640454 are merge debt. They are not 545 work.

## 3. Does the section bind only this party in this notice? Yes

- `ORG-GB-PPON-PRBY-9272-TQGX` is one Organization section: BT-500 "ERMC Ltd", BT-501 GB-PPON-PRBY-9272-TQGX, country GB.
- It is referenced once, as `TPA-1-0 / OPT-300-Tenderer` (tender TEN-1-0 → result RES-1-1 → contract CON-1, worth £100,000).
- The buyer is a separate section, `ORG-GB-PPON-PGYX-7387-YJXG`, bound to 30913687 "Isle of Wight Council". The move does not touch it.
- Tender 8656068 has a single version (seq 1, caused by 46819437). No other notice of this procedure carries the section, so n=1.
- After the move, the bid and the derived winner show ERMC Limited instead of the NHS trust.

## 4. Would the move lose information? No

- The holder is not the real party with ERMC as its delegate. The trust has no role at all on this Council award.
- The holder keeps its 3 trust-named mentions and loses only the stray one.
- The move lines this award up with the identical-pattern Old Barton award, which is already on 18928807.

## Result

**Not refuted.** The rehome 31571667 → 18928807 for (46819437, ORG-GB-PPON-PRBY-9272-TQGX) is correct. It no longer rests on name equality alone: the same buyer's later notice prints ERMC LIMITED with GB-COH-04489340, and that mention is bound to 18928807. The "weakest rehome" flag in u3.md can be lifted. Fix the rationale wording ("only ERMC row" → "only identified ERMC row"); the verdict itself needs no change.
