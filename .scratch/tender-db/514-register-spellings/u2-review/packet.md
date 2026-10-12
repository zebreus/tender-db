# Issue 514 U2.1: review packet (2026-10-12)

This packet covers "Measure first" #3 (register records for the canonical numbers) and #4 (the mention-name packet for the 52 group members and the 27 lone movers) of `../unit2-decision.md`. The data is in `packet.json`, with one entry per E0 group (`G01`-`G26`) and per lone mover (`L01`-`L27`). The pre-read applies §3.2 (a)-(c) mechanically to hand-read names. **It is not a verdict.** Rule (d), agreement between the reviewer and the challenger, is still to come.

## Counts

| | materialised | decision §1.4 |
|---|---:|---:|
| rows rewritten | 54 (UKPRN 6, CHC 28, NIC 1, MPR 19: 18 IP, 1 REGNO) | 54 (6 / 28 / 1 / 19: 18 IP, 1 REGNO) |
| E0 groups | 26 (25 onto a standing twin, 1 movers reunite: 1122831) | 26 (25 + 1122831) |
| group members | 52 (every group has 2) | 52 |
| group mentions | 743 | 743 |
| lone movers | 27 (CHC 18, MPR IP+R 8, UKPRN 1), 42 mentions | 27 (listed as 20 + 1 + 8: see A1) |
| distinct canonical numbers | 53 (all found on their register) | "54" (see A2) |

The rewrite reaches none of §1.4's 14 trap orgs: they appear neither as members nor as movers (S6), confirmed.

**Reads.**
- Prod: 8 sequential calls through `/root/sq.sh`, with no 408 and no retry:
  - 6 bounded index-served SELECTs:
    - organizations by primary key (79);
    - mentions by `organization_mentions_org`, grouped, in 2 calls;
    - organization_names by primary-key prefix;
    - holders of the 53 canonical and the 54 stored values on `organizations_identity`;
  - 2 allow-list probes, both refused at parse with HTTP 400.
  - The SQL, timings and status of each call are in `prod/journal.jsonl`.
- Registers: 100 GETs, all HTTP 200 (`registers/fetch-manifest.json`):
  - FCA SocietyList.csv, plus each society's search result, page, name changes and transfers;
  - UKRLP detail and status lists;
  - the CC JSON extracts, dated 2026-10-12, of which only the 27 numbers' rows are kept: 163 charity rows including linked charities, and 48 other-name rows;
  - the CCNI page, CSV and search.
  - The bulk files stay in the session scratchpad. Their sha256 is in the manifest.

**Pre-read** (mechanical, not a verdict): REGISTER MISMATCH candidate (hold list) 1, meets (a)-(c) on the hand read 23, register agrees (rule b) 26, reviewer attention 3.

## Anomalies

- **A1 §1.4's lone-mover breakdown sums to 29, not 27.** §1.4 lists 20 CHC + 1 UKPRN + 8 IP+R lone moves = 29. The rules give 18 CHC + 1 UKPRN + 8 MPR-a = 27 (groups.json). The 20 counts 1122831's two movers, which reunite with each other and so form E0 group G03, as §0's 'Reach' line says (29 = 27 lone + 1122831's 2). Totals agree: 54 rows, 26 groups (25 onto a standing twin + 1122831), 52 members, 27 lone movers, 743 group mentions.
- **A2 '54 canonical numbers' are 53 distinct numbers; CCNI 102575 is one of them.** 54 rewritten rows map to 53 distinct canonical values: 1122831's two rows share one. CCNI 102575 is the NIC group's canonical (G21), not a 55th number. 53 register records were fetched (MPR 19, CHC 27, UKPRN 6, NIC 1); all 53 were found.
- **A3 1003906's mover is a lone mover, and its census split stays open.** 31562667 (GBCHCREGISTEREDCHARITYNO1003906, Birmingham Mind, 7 mentions) moves to GBCHC1003906, but its census twin 31594757 (GBCHCREGISTRATIONNO1003906, a generic label, deferred by ruling 9) does not. The split goes from label-vs-label to bare-vs-generic. Its 8 mentions are in the 109 deferred, not the 743. No other lone mover belongs to a census split.
- **A4 GBCHC279057: lone mover 31594820 names another charity.** Mover 31594820 (GBCHCREGISTEREDCHARITYNUMBER279057, 1 mention) reads 'Birmingham Citizens Advice Bureaux'. The CC record for 279057 is 'THE NATIONAL ASSOCIATION OF CITIZENS ADVICE BUREAUX' (Registered, company 01436945). Birmingham's bureau is its own charity, 519639 BIRMINGHAM CITIZENS ADVICE BUREAU SERVICE LTD (Registered, company 02202427, same extract). Under §3.3, GBCHC279057 goes on REGISTER_SPELLING_HOLDS: the rewrite must not mint the national body's identity for a local member. This is ONE mismatch, so S11 (two or more) does not stop the unit. The census measured this hazard at 0 of 36 splits; it sits in a lone mover, which the census never name-checked.
- **A5 (b) rests on unsourced names in 3 entries.** Names read as the holder's but not register-sourced (rule (b) needs a sourced brand or service): G01 GBCHC1088835 org 31537211 'No Limits Safe Haven' 2/2; G09 GBCHC271028 org 31594839 'Rethink - Redwoods' 1/1; G11 GBMPR17437R org 31593260 'Gold Hill Care (B2G)' 1/7; G23 GBUKPRN10005790 org 30915632 'Sheffield Polytechnic' 2/182; G25 GBUKPRN10007788 org 31549754 'THE CHANCELLOR MASTERS AND SCHOLARS OF THE UNIVERSITY OF CAMBRIDGE' 4/13; G25 GBUKPRN10007788 org 31549754 'The Chancellor, Masters and Scholars of the University of Cambridge' 2/13; G25 GBUKPRN10007788 org 31633713 'The Chancellor, Masters and Scholars of the University of Cambridge' 1/1. Where the name carries the member's majority the reviewer must source it or post keep: G01 1088835 (31537211, 2/2 'No Limits Safe Haven'), G09 271028 (31594839, 1/1 'Rethink - Redwoods'), G25 10007788 (31633713, 1/1 statutory name; also 6/13 of 31549754).
- **A6 Other-entity names on group members, all below the 1/5 floor.** GBMPR28038R org 31534486 'Clarion Housing Association Ltd' 1/138; GBUKPRN10005790 org 30915632 'Access UK Ltd' 1/182; GBUKPRN10005790 org 30915632 'Sheffield Hallam University Enterprises Ltd' 1/182; GBUKPRN10005790 org 30915632 'University of Sheffield' 1/182; GBUKPRN10007157 org 31558903 'Sheffield Hallam University' 1/16. 31558903 still carries the head name 'Sheffield Hallam University' from its single stray mention (organization_names reads UNIVERSITY OF SHEFFIELD); it is the lower id, so it survives a G24 merge with that head name (risk 9, cosmetic) unless the head is re-chosen.
- **A7 No drift since the census.** All 79 orgs re-read with the census identifier, head name, provisional = 0 and mention total. Each canonical value is held only by its standing twin (25 holders; none for 1122831 or the 27 lone movers' values); each of the 54 stored polluted values is held only by its census org. Every mover's mention raws still normalise to its own stored value only. All 785 mentions are country GB.
- **A8 The existing verdicts on the 26 E0 keys cannot be listed through /v1/sql.** org_merge_verdicts and org_identifier_verdicts are not on /v1/sql's allow-list (prod calls 7-8, HTTP 400, no data pages read). Identifier verdicts surface on /v1/organizations as identifier_status, which is outside this unit's read path. P0 item 3 stays open and needs an operator read before the verdicts are posted (S8). Local record: no verdict file under .scratch/tender-db names any of the 26 E0 keys. The only verdicts naming these orgs are 469 U6's 5 GB:altid merges (cohort altid-registry-confirmed-2026-10-11, HIGH merge, applied), one per G4 holder: GB-MPR:GBMPR16337R~PMCL4972LYCY [30976449, 31538068]; GB-MPR:GBMPR27440R~PQBH3597XBJD [30913929, 31553186]; GB-MPR:GBMPR28038R~PQND5289JNDN [30914385, 31534486]; GB-UKPRN:GBUKPRN10005790~PVMW4512JZYY [30915632, 31535088]; GB-UKPRN:GBUKPRN10007788~PMCX4817NTNG [31549754, 31569620]. They are a different (scheme, key) and do not deny an E0 group.
- **A9 In 8 groups the survivor (lower id) is the mover.** G02 GBCHC1104951 survivor 31562628; G03 GBCHC1122831 survivor 31562660; G05 GBCHC1185092 survivor 31562679; G08 GBCHC221124 survivor 31562686; G11 GBMPR17437R survivor 31593260; G13 GBMPR19341R survivor 31550337; G16 GBMPR27793R survivor 31614098; G18 GBMPR30167R survivor 31604789. After the 345 re-key both members hold the canonical value, so only head names are affected (risk 9). The 5 G4 holders (Metropolitan, Clarion, Creative Support, Hallam, Cambridge) are each the lower id and survive.

## E0 groups

Members are listed ascending, and the first one is the survivor (`*` marks a mover). Each name share is weighted by mention count. "holder" is the share of mentions that name the register holder; unsourced brands are counted, and the flags show which ones they are.

| id | canonical | register (status) | members: stored, mentions, holder share | pre-read |
|---|---|---|---|---|
| G01 | `GBCHC1088835` | NO LIMITS (SOUTH) (Registered) | 31537211 `GBCHC1088835` 2, 100%<br>*31594929 `GBCHCREGISTEREDCHARITYNO1088835` 1, 100% | reviewer attention<br>(b) 31537211: majority rests on an unsourced name (0% sourced) |
| G02 | `GBCHC1104951` | ST ANDREW'S HEALTHCARE (Registered) | *31562628 `GBCHCREGISTEREDCHARITYNO1104951` 2, 100%<br>31607602 `GBCHC1104951` 1, 100% | meets (a)-(c) on the hand read |
| G03 | `GBCHC1122831` | ACACIA FAMILY SUPPORT (Registered) | *31562660 `GBCHCCHARITYNUMBER1122831` 1, 100%<br>*31594752 `GBCHCREGISTEREDCHARITYNO1122831` 2, 100% | meets (a)-(c) on the hand read |
| G04 | `GBCHC1159706` | ANAWIM - BIRMINGHAM’S CENTRE FOR WOMEN LIMITED (Registered) | 30916610 `GBCHC1159706` 1, 100%<br>*31542854 `GBCHCCHARITYNUMBER1159706` 5, 100% | meets (a)-(c) on the hand read |
| G05 | `GBCHC1185092` | FIRST STEPS ED (Registered) | *31562679 `GBCHCREGISTEREDCHARITY1185092` 1, 100%<br>31571677 `GBCHC1185092` 3, 100% | meets (a)-(c) on the hand read |
| G06 | `GBCHC208078` | Cruse Bereavement Support (Registered) | 31564669 `GBCHC208078` 3, 100%<br>*31594774 `GBCHCREGISTEREDCHARITYNO208078` 1, 100% | meets (a)-(c) on the hand read |
| G07 | `GBCHC216250` | BARNARDO'S (Registered) | 31561821 `GBCHC216250` 9, 100%<br>*31594691 `GBCHCCHARITYNUMBER216250` 1, 100% | meets (a)-(c) on the hand read |
| G08 | `GBCHC221124` | CHURCH OF ENGLAND CHILDREN'S SOCIETY (Registered) | *31562686 `GBCHCCHARITYREGISTRATIONNO221124` 1, 100%<br>31645298 `GBCHC221124` 2, 100% | meets (a)-(c) on the hand read |
| G09 | `GBCHC271028` | Rethink Mental Illness (Registered) | 31535844 `GBCHC271028` 6, 100%<br>*31594839 `GBCHCREGISTEREDCHARITY271028` 1, 100% | reviewer attention<br>(b) 31594839: majority rests on an unsourced name (0% sourced) |
| G10 | `GBMPR16337R` | Metropolitan Housing Trust Limited (Registered) | 30976449 `GBMPR16337R` 104, 100%<br>*31553962 `GBMPRIP16337R` 2, 100% | meets (a)-(c) on the hand read |
| G11 | `GBMPR17437R` | Gold Hill Housing Association Limited (Registered) | *31593260 `GBMPRIP17437R` 7, 100%<br>31631085 `GBMPR17437R` 3, 100% | meets (a)-(c) on the hand read |
| G12 | `GBMPR19059R` | Sanctuary Housing Association (Registered) | 31564048 `GBMPR19059R` 2, 100%<br>*31619954 `GBMPRIP19059R` 1, 100% | meets (a)-(c) on the hand read |
| G13 | `GBMPR19341R` | Habinteg Housing Association Limited (Registered) | *31550337 `GBMPRIP19341R` 1, 100%<br>31613175 `GBMPR19341R` 1, 100% | meets (a)-(c) on the hand read |
| G14 | `GBMPR21256R` | North Star Housing Group (Registered) | 31611304 `GBMPR21256R` 1, 100%<br>*31622550 `GBMPRIP21256R` 1, 100% | meets (a)-(c) on the hand read |
| G15 | `GBMPR27440R` | Creative Support Limited (Registered) | 30913929 `GBMPR27440R` 9, 100%<br>*31543018 `GBMPRIP27440R` 1, 100% | meets (a)-(c) on the hand read |
| G16 | `GBMPR27793R` | Greenwich Leisure Limited (Registered) | *31614098 `GBMPRIP27793R` 2, 100%<br>31624556 `GBMPR27793R` 1, 100% | meets (a)-(c) on the hand read |
| G17 | `GBMPR28038R` | Clarion Housing Group Limited (Registered) | 31534486 `GBMPR28038R` 138, 99%<br>*31596599 `GBMPRIP28038R` 6, 100% | meets (a)-(c) on the hand read |
| G18 | `GBMPR30167R` | Town and Country Housing (Registered) | *31604789 `GBMPRIP30167R` 1, 100%<br>31618490 `GBMPR30167R` 1, 100% | meets (a)-(c) on the hand read |
| G19 | `GBMPR30301R` | Herts Urgent Care (HUC) Limited (Registered) | 31551789 `GBMPR30301R` 2, 100%<br>*31551790 `GBMPRIP30301R` 2, 100% | meets (a)-(c) on the hand read |
| G20 | `GBMPR30938R` | The Riverside Group Limited (Registered) | 30914487 `GBMPR30938R` 116, 100%<br>*31591848 `GBMPRREGNO30938R` 1, 100% | meets (a)-(c) on the hand read |
| G21 | `GBNICNIC102575` | Radius Housing Association Limited (Received: on time) | 31621236 `GBNICNIC102575` 1, 100%<br>*31631048 `GBNICCHARITYCOMMISSIONNONIC102575` 1, 100% | meets (a)-(c) on the hand read |
| G22 | `GBUKPRN10004116` | EAST COAST COLLEGE (active) | 31549563 `GBUKPRN10004116` 7, 100%<br>*31622719 `GBUKPRNGBUKPRN10004116` 1, 100% | meets (a)-(c) on the hand read |
| G23 | `GBUKPRN10005790` | SHEFFIELD HALLAM UNIVERSITY (active) | 30915632 `GBUKPRN10005790` 182, 98%<br>*31624379 `GBUKPRNUKPRN10005790` 1, 100% | meets (a)-(c) on the hand read |
| G24 | `GBUKPRN10007157` | University of Sheffield (active) | 31558903 `GBUKPRN10007157` 16, 94%<br>*31630773 `GBUKPRNUKPRN10007157` 1, 100% | meets (a)-(c) on the hand read |
| G25 | `GBUKPRN10007788` | UNIVERSITY OF CAMBRIDGE (active) | 31549754 `GBUKPRN10007788` 13, 100%<br>*31633713 `GBUKPRNUKPRN10007788` 1, 100% | reviewer attention<br>(b) 31633713: majority rests on an unsourced name (0% sourced) |
| G26 | `GBUKPRN10007798` | University of Manchester (active) | 31600847 `GBUKPRN10007798` 51, 100%<br>*31612560 `GBUKPRNUKPRN10007798` 22, 100% | meets (a)-(c) on the hand read |

## Lone movers

| id | canonical | org, stored, mentions | mention names | register (status) | pre-read |
|---|---|---|---|---|---|
| L01 | `GBCHC1003906` | 31562667 `GBCHCREGISTEREDCHARITYNO1003906` 7 | Birmingham Mind ×7 | BIRMINGHAM ASSOCIATION FOR MENTAL HEALTH (Registered) | register agrees (rule b) |
| L02 | `GBCHC1034581` | 31562630 `GBCHCREGISTEREDCHARITYNO1034581` 2 | Birmingham Disability Resource Centre ×2 | BIRMINGHAM DISABILITY RESOURCE CENTRE (Registered) | register agrees (rule b) |
| L03 | `GBCHC1068106` | 31608050 `GBCHCCHARITYNUMBER1068106` 1 | Mount Zion Apostolic Church ×1 | MOUNT ZION APOSTOLIC CHURCH (Registered) | register agrees (rule b) |
| L04 | `GBCHC1072258` | 31562627 `GBCHCREGISTEREDCHARITYNO1072258` 2 | Birmingham Irish Association ×2 | BIRMINGHAM IRISH ASSOCIATION (Registered) | register agrees (rule b) |
| L05 | `GBCHC1073926` | 31594822 `GBCHCCHARITYNO1073926` 1 | Birmingham Solihull Womens Aid ×1 | BIRMINGHAM AND SOLIHULL WOMEN'S AID (Registered) | register agrees (rule b) |
| L06 | `GBCHC1077607` | 31618870 `GBCHCREGISTEREDCHARITYNO1077607` 1 | Daybreak Family Group Conference ×1 | DAYBREAK FAMILY GROUP CONFERENCES (Registered) | register agrees (rule b) |
| L07 | `GBCHC1094458` | 31594819 `GBCHCCHARITYNO1094458` 1 | St Martins Centre for health and healing ×1 | ST MARTIN'S CENTRE FOR HEALTH AND HEALING (Registered) | register agrees (rule b) |
| L08 | `GBCHC1107830` | 31537995 `GBCHCCHARITYREGISTRATIONNUMBER1107830` 1 | Citizens Advice Mid Mercia ×1 | Citizens Advice Mid Mercia (Registered) | register agrees (rule b) |
| L09 | `GBCHC1133510` | 31594841 `GBCHCREGISTEREDCHARITY1133510` 1 | Karis Neighbourhood Scheme ×1 | KARIS NEIGHBOUR SCHEME (Registered) | register agrees (rule b) |
| L10 | `GBCHC1175670` | 31598277 `GBCHCREGISTEREDCHARITYNUMBER1175670` 1 | Mental Health Innovations ×1 | MENTAL HEALTH INNOVATIONS (Registered) | register agrees (rule b) |
| L11 | `GBCHC1179376` | 31594828 `GBCHCCHARITYNUMBER1179376` 1 | Oasis mental health support ×1 | Oasis Mental Health Support (Registered) | register agrees (rule b) |
| L12 | `GBCHC1179683` | 31562625 `GBCHCCHARITYNUMBER1179683` 2 | Seven Up Charity ×2 | SEVEN UP CHARITY (Registered) | register agrees (rule b) |
| L13 | `GBCHC1205036` | 31537996 `GBCHCCHARITYNUMBER1205036` 1 | Ghamkol Sharif Central Jamia Charity ×1 | CENTRAL JAMIA MOSQUE GHAMKOL SHARIF (Registered) | register agrees (rule b) |
| L14 | `GBCHC279057` | 31594820 `GBCHCREGISTEREDCHARITYNUMBER279057` 1 | Birmingham Citizens Advice Bureaux ×1 | THE NATIONAL ASSOCIATION OF CITIZENS ADVICE BUREAUX (Registered) | REGISTER MISMATCH candidate (hold list) |
| L15 | `GBCHC517077` | 31562629 `GBCHCREGISTEREDCHARITYNUMBER517077` 3 | Autism West Midlands ×3 | AUTISM.WEST MIDLANDS (Registered) | register agrees (rule b) |
| L16 | `GBCHC518048` | 31562674 `GBCHCREGISTEREDCHARITYNUMBER518048` 2 | Compass ×2 | Compass - Services to Improve Health and Wellbeing (Registered) | register agrees (rule b) |
| L17 | `GBCHC518610` | 31575623 `GBCHCCHARITYNUMBER518610` 1 | Age Connect West Midlands ×1 | Age Connects West Midlands (Registered) | register agrees (rule b) |
| L18 | `GBCHC518753` | 31594838 `GBCHCREGISTEREDCHARITYNO518753` 1 | Solihull Bereavement Counselling Services ×1 | SOLIHULL BEREAVEMENT COUNSELLING SERVICE (Registered) | register agrees (rule b) |
| L19 | `GBMPR16243R` | 31613230 `GBMPRIP16243R` 1 | Hastoe Housing ×1 | Hastoe Housing Association Limited (Registered) | register agrees (rule b) |
| L20 | `GBMPR17704R` | 30975000 `GBMPRIP17704R` 1 | Your Housing Limited ×1 | Your Housing Limited (Registered) | register agrees (rule b) |
| L21 | `GBMPR19229R` | 31601449 `GBMPRIP19229R` 1 | ACCENT HOUSING LIMITED ×1 | Accent Housing Limited (Registered) | register agrees (rule b) |
| L22 | `GBMPR21180R` | 31567617 `GBMPRIP21180R` 1 | Cadarn Housing Group (AA58745) ×1 | The Cadarn Housing Group Limited (Registered) | register agrees (rule b) |
| L23 | `GBMPR26511R` | 31609584 `GBMPRIP26511R` 1 | Two Saints Ltd ×1 | Two Saints Limited (Registered) | register agrees (rule b) |
| L24 | `GBMPR27566R` | 31600628 `GBMPRIP27566R` 1 | National Development Team for Inclusion ×1 | The National Development Team for Inclusion (Registered) | register agrees (rule b) |
| L25 | `GBMPR29840R` | 31573433 `GBMPRIP29840R` 2 | LOCAL SPACE ×2 | Local Space (Registered) | register agrees (rule b) |
| L26 | `GBMPR30178R` | 31575842 `GBMPRIP30178R` 1 | INCOMMUNITIES LIMITED ×1 | Incommunities Limited (Registered) | register agrees (rule b) |
| L27 | `GBUKPRN10007339` | 31572226 `GBUKPRNUKPRN10007339` 3 | WARRINGTON & VALE ROYAL COLLEGE ×3 | Warrington and Vale Royal College (active) | register agrees (rule b) |

## Register facts a reviewer will want

- **MPR (FCA).**
  - All 19 are Registered Societies under the 2014 Act.
  - Name changes and transfers are in each record. Examples:
    - 28038R was Affinity Sutton Group Limited until 2016-11-29.
    - 30938R took One Housing Group (Full) on 2023-03-31.
    - 16337R took Thames Valley Charitable HA (TE) on 2018-10-08.
- **CHC (CC extract 2026-10-12).**
  - All 27 are Registered, with no removal.
  - `company_number`, other names and linked charities are recorded.
  - Linked charities: 216250 has 93 (11 registered), 221124 has 33 (14), 517077 has 5, 271028 has 3, 279057 has 1, 1104951 has 1.
- **UKPRN (UKRLP).**
  - All 6 are active.
  - 10007157 is the University of Sheffield, with trading names THE UNIVERSITY OF SHEFFIELD and University of Sheffield.
  - 10007788 is UNIVERSITY OF CAMBRIDGE, with trading names University of Cambridge and Cambridge University.
- **NIC (CCNI).**
  - 102575 is Radius Housing Association Limited, registered 2016-04-04.
  - CCNI's status column reads "Received: on time", which is its reporting-status label.

## Not covered here

- The verdict listing on the 26 keys (A8) needs an operator path. It is outside `/v1/sql`.
- "Measure first" #5 is not covered: fresh range re-reads just before deploy, to catch movers minted since 2026-10-11. #6 (XI) is not covered either.
- Cross-scheme twins of the same bodies stand outside unit 2, and the E0 groups leave them as they are. For example, `519-nhs-digit-codes/name-twins-2026-10-12.json` shows Cruse on `GBCOH00638709` and `GBCOH638709`, plus a PPON and an NHS code, and St Andrew's on NHS `NYA01`.

## Files

- `materialise.py` produces `groups.json`, re-implementing §1.3 and §1.2's I1-I3 over census `raw/001-004`. Mentions come from `raw/016-019` and `raw/021-029`.
- `prod_reads.py` writes `prod/01-06*.out` and `prod/journal.jsonl`. The two probes are `prod/07-08`.
- `fetch_registers.py` writes `registers/raw/**` and `registers/fetch-manifest.json`. `build_registers.py` turns them into `registers/records.json`.
- `namematch.py` holds the mechanical routing only. `build_packet.py` writes `packet.json` and `packet.md`, and holds the hand notes on every non-exact name.
