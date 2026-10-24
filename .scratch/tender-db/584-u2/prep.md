# 584 unit 2 (preparer): the KIO-NIP rehoming cohort, 2026-10-24

**Outcome.** Of the 38 mentions on org 49 whose name does not read as "Krajowa Izba Odwoławcza", **32 rehome** (all `high`) and **6 keep**. **0 are parked.**
- `cohort.json` holds cohort `584-2026-10-24` in 545's exact per-mention shape (317's `POST /admin/rehoming` body): 38 verdicts, `org_id` 49 on each.
- `parked.json` is an empty list, with the reason no row needed parking.
- The 32 rehomes cover 15 bodies and touch **18 tenders**. Every destination exists and was read live today.
  - 14 destinations are identified rows, each keyed by the body's own register number. A register confirmed every one of those numbers.
  - 1 destination is a provisional row carrying exactly the printed name (the J&M Engineers consortium leader). 545 accepted three such rows.
- The 6 keeps are KIO/UZP: 5 × "National Appeals Chamber" (bilingual, with the POL variant "Krajowa Izba Odwoławcza") and 1 × "Urząd Zamóweń Publicznych" (a UZP typo). All 6 sit in review roles.

**What was and was not done.** This step was read-only.
- Box reads: 6 `/v1/sql` statements and 7 public GETs. There was no POST, no admin endpoint, no cargo, no git, and no edits outside `584-u2/`.
- Registers: 8 WebFetches. One went to rejestr.io and was refused with a 403, so it was not used. The other 7 went to the official KRS API (`api-krs.ms.gov.pl`) and the MF VAT white list (`wl-api.mf.gov.pl`).
- `requests.log` logs every request.
- `scripts/build_cohort.py` rebuilds the cohort from the saved reads and asserts every rule below.

## The identifier itself

The MF white list says that **NIP 5262239325 belongs to Urząd Zamówień Publicznych** (REGON 010828091, Postępu 17A, Warszawa). KIO sits inside UZP and has no NIP of its own.
- So the identifier's true holder is UZP/KIO.
- A UZP-named mention on org 49 is therefore right, not foreign. That matters for the keep row and for the guard (`guard-spec.md`).

## Rules applied (545's, mechanically; `build_cohort.py` asserts each)

- **Rehome, high**, only when two things hold:
  - the section prints another body's name with KIO/UZP's NIP;
  - an EXISTING destination is found, in this order:
    1. the org holding the body's own NIP, confirmed by a register;
    2. if no org holds the NIP, the org holding another own register number (REGON), confirmed by a register;
    3. else the PL row carrying exactly the printed name with the most mentions.

  NIP comes before REGON or KRS because the publisher's field was a NIP field (`NIP 5262239325`), and the NIP is the key the mention would have bound had it been printed right.
- **Keep**: the name is a KIO/UZP spelling in a review role.
- **Park** if any of these holds (none fired):
  - no existing destination;
  - the section is bound on more than one tender;
  - the section binds roles of more than one side (buyer / supplier / review);
  - the printed name names more than one body;
  - the destination is ambiguous.
- **"More than one party", read for this cohort.** The 13 Uniwersytet Warszawski sections are each bound as Procedure-Buyer and also as Lot-TenderEval, Lot-TenderReceipt and Lot-DocProvider. That is one body in buyer-side roles on one tender, so it counts as one party, as Watkins' 6 lots were one party in 545.
  - Every section is bound on exactly one tender.
  - Every supplier section is Tenderer plus its bid-party row. On 13 of the 14 supplier-side tenders it is also the winner: PKO BP on 136020 was a tenderer only.
- **J&M Engineers "(Lider)".** The section names the consortium leader alone. The partner, BICO GROUP, prints its own NIP in its own section (ORG-0003, org 15949738), so the section names one body.
- **MPWiK.** MPWiK is a **co-buyer**: Procedure-Buyer on GDDKiA's tender 997224, a joint procurement in which GDDKiA is ORG-0001 with its own NIP 1132097244. So the move lands a buyer role on a buyer body.

## Evidence (per mention)

Sources: the roles come from S2 (`tender_version_parties`, grouped by tender and role) and the names and raw ids from S1. "cur" says whether any of the mention's party rows is on the tender's current version.

| # | notice / section | tender | roles bound to org 49 | cur | printed name (BT-500) | verdict | destination (served name) | key used |
|---|---|---|---|---|---|---|---|---|
| 1 | 23540012 / ORG-0002 | 997224 | Buyer | no | Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. s… | rehome | 848 Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. st. Warszawie S.A. | NIP 5250005662 |
| 2 | 23580677 / ORG-0002 | 997224 | Buyer | no | Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. s… | rehome | 848 Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. st. Warszawie S.A. | NIP 5250005662 |
| 3 | 23620337 / ORG-0002 | 997224 | Buyer | no | Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. s… | rehome | 848 Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. st. Warszawie S.A. | NIP 5250005662 |
| 4 | 23661303 / ORG-0002 | 997224 | Buyer | no | Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. s… | rehome | 848 Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. st. Warszawie S.A. | NIP 5250005662 |
| 5 | 24300743 / ORG-0003 | 136020 | Tenderer+bid | yes | Powszechna Kasa Oszczędności Bank Polski Spółka Akcyjna | rehome | 10689264 Powszechna Kasa Oszczędności Bank Polski S.A. | NIP 5250007738 |
| 6 | 24368074 / ORG-0002 | 997224 | Buyer | yes | Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. s… | rehome | 848 Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. st. Warszawie S.A. | NIP 5250005662 |
| 7 | 24425302 / ORG-0002 | 241175 | Tenderer+bid+winner | yes | Ireneusz Malewicz J&M Engineers (Lider) | rehome | 18347269 Ireneusz Malewicz J&M Engineers (Lider) (prov.) | exact name |
| 8 | 24532293 / ORG-0006 | 924958 | ReviewInfo+ReviewOrg | yes | Urząd Zamóweń Publicznych | keep | — (stays on 49) | — |
| 9 | 24766144 / ORG-0003 | 134851 | Tenderer+bid+winner | yes | Terra Mota Sp. z o.o. | rehome | 14553663 Terra Mota | NIP 7382070551 |
| 10 | 24820243 / ORG-0004 | 614155 | Tenderer+bid+winner | yes | Terra Mota Sp. z o.o. | rehome | 14553663 Terra Mota | NIP 7382070551 |
| 11 | 24831057 / ORG-0002 | 320481 | ReviewOrg | no | National Appeals Chamber | keep | — (stays on 49) | — |
| 12 | 24865719 / ORG-0005 | 313839 | Tenderer+bid+winner | yes | ARCHITEKTURA PLUS DESIGN STUDIO MICHAŁ MARCZAK | rehome | 23202450 ARCHITEKTURA PLUS DESIGN STUDIO MICHAŁ MARCZAK | NIP 6871892878 |
| 13 | 24906332 / ORG-0002 | 733110 | ReviewOrg | no | National Appeals Chamber | keep | — (stays on 49) | — |
| 14 | 24993174 / ORG-0002 | 733110 | ReviewOrg | no | National Appeals Chamber | keep | — (stays on 49) | — |
| 15 | 25117974 / ORG-0002 | 320481 | ReviewOrg | yes | National Appeals Chamber | keep | — (stays on 49) | — |
| 16 | 25374444 / ORG-0003 | 572868 | Tenderer+bid+winner | yes | REMONDIS Bydgoszcz  S.A. | rehome | 15146954 Remondis Bydgoszcz Sp. z o.o. | NIP 5540312757 |
| 17 | 25398003 / ORG-0002 | 733110 | Mediator+ReviewOrg | yes | National Appeals Chamber | keep | — (stays on 49) | — |
| 18 | 25511030 / ORG-0001 | 306091 | DocProv+TEval+TRcpt+Buyer | no | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 19 | 25522748 / ORG-0003 | 418730 | Tenderer+bid+winner | yes | Digitex Sp.z o.o Sp. k. | rehome | 20111694 Digitex Sp. z o.o.sp.k. | NIP 5851476863 |
| 20 | 25564521 / ORG-0003 | 1092955 | Tenderer+bid+winner | yes | NOVUM S.A. | rehome | 8998814 NOVUM S.A. | NIP 9491924705 |
| 21 | 25576610 / ORG-0001 | 306091 | DocProv+TEval+TRcpt+Buyer | yes | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 22 | 25591527 / ORG-0001 | 990702 | DocProv+TEval+TRcpt+Buyer | no | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 23 | 25592320 / ORG-0001 | 445071 | DocProv+TEval+TRcpt+Buyer | no | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 24 | 25608668 / ORG-0001 | 990702 | DocProv+TEval+TRcpt+Buyer | yes | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 25 | 25636957 / ORG-0003 | 914542 | Tenderer+bid+winner | yes | WTU Spółka z o.o. | rehome | 13575097 WTU Sp. z o.o. | NIP 8992680461 |
| 26 | 25643080 / ORG-0001 | 445071 | DocProv+TEval+TRcpt+Buyer | yes | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 27 | 25643735 / ORG-0001 | 990702 | DocProv+TEval+TRcpt+Buyer | no | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 28 | 25649705 / ORG-0003 | 78715 | Tenderer+bid+winner | yes | PZU ZDROWIE S.A. Odział Centra Medyczne w Gdańsku | rehome | 18389587 PZU Zdrowie Spółka Akcyjna | NIP 5272663852 |
| 29 | 25664005 / ORG-0001 | 990702 | DocProv+TEval+TRcpt+Buyer | no | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 30 | 25678097 / ORG-0001 | 990702 | DocProv+Buyer | no | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 31 | 25710042 / ORG-0001 | 306091 | DocProv+Buyer | yes | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 32 | 25767621 / ORG-0001 | 445071 | Buyer | yes | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 33 | 25791501 / ORG-0001 | 990702 | DocProv+Buyer | yes | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 34 | 25791954 / ORG-0004 | 497408 | Tenderer+bid+winner | yes | Evalu Sp. z o.o. | rehome | 20491831 EVALU Sp. z o.o. | NIP 5262825916 |
| 35 | 25800438 / ORG-0001 | 445071 | DocProv+Buyer | yes | Uniwersytet Warszawski | rehome | 7788 Uniwersytet Warszawski | NIP 5250011266 |
| 36 | 25867833 / ORG-0003 | 174615 | Tenderer+bid+winner | yes | Zarys International Sp. z o.o. | rehome | 976 ZARYS INTERNATIONAL GROUP SP .Z O.O. | NIP 6481997718 |
| 37 | 29173048 / ORG-0003 | 655893 | Tenderer+bid+winner | yes | Gminny Zakład Gospodarki Komunalnej Sp. z o.o. | rehome | 22583689 Gminny Zakład Gospodarki Komunalnej spółka z o.o. | REGON 061423397 |
| 38 | 29920705 / ORG-0003 | 824617 | Tenderer+bid+winner | yes | Pro3Ds Paweł Krawczyk | rehome | 22456216 Pro 3Ds Paweł Krawczyk | NIP 7123245197 |

Every row's raw identifier is a spelling of 5262239325: `NIP 5262239325`, `NIP: 5262239325`, `NIP 526 223 9325`, `NIP: 526-223-93-25`, and so on (S1).

## Destinations (15 bodies)

Every "key used" was read on `organizations` by identifier (S5) and by name prefix (S3/S4). The register is the source quoted. Five destinations were also read on `/v1/organizations/{id}`: 7788, 848, 976, 15146954 and 18347269. Each answered with the name and mentions below.

| body (mentions) | destination | key | register evidence | alternatives seen (id, key, mentions) |
|---|---|---|---|---|
| Uniwersytet Warszawski (13) | **7788** Uniwersytet Warszawski, 1,463 mentions | NIP 5250011266 | MF: UNIWERSYTET WARSZAWSKI, NIP 5250011266, REGON 000001258, Krakowskie Przedmieście 26/28 (the address the buyer section prints). Also, org 12921797's raw string `REGON00001258NIP5250011266` pairs them | 11785 (REGON, 351); 1188351 (provisional, 2,255) |
| MPWiK m.st. Warszawie (5) | **848** Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. st. Warszawie S.A., 1,645 | NIP 5250005662 | KRS 0000146138: NIP 5250005662, Plac Starynkiewicza 5 (printed in the section's name) | 16545317 (REGON 015314758, 1,304); 1240504 (KRS, 22); provisionals 1191582 (1,197), 1191583 (1,035) |
| Terra Mota Sp. z o.o. (2) | **14553663** Terra Mota, 2 | NIP 7382070551 | MF: TERRA MOTA Sp. z o.o., KRS 0000294862, Gorlice | none: the only row under the prefix |
| PKO BP S.A. (1) | **10689264** Powszechna Kasa Oszczędności Bank Polski S.A., 139 | NIP 5250007738 | MF: PKO BP S.A., KRS 0000026438 | 10635638 (REGON, 34); 15814004 (KRS, 2) |
| Ireneusz Malewicz J&M Engineers (Lider) (1) | **18347269** (provisional), the same exact name, 1 | exact name | none: a sole trader whose NIP is unknown. No identified row under `ireneusz malewicz`, `malewicz`, `j&m`, `j & m`, `i&m`, `i & m` or `jm engineers` | 20534059 (provisional, "Ireneusz Malewicz prowadzący działalność gospodarczą pod firmą I&M ENGINEERS…", 1) |
| ARCHITEKTURA PLUS DESIGN STUDIO MICHAŁ MARCZAK (1) | **23202450**, the same exact name, 2 | NIP 6871892878 | MF: MICHAŁ MARCZAK, a sole trader in Wrocław | none |
| REMONDIS Bydgoszcz S.A. (1) | **15146954** Remondis Bydgoszcz Sp. z o.o., 78 | NIP 5540312757 | KRS 0001015725: the S.A. arose in 2023 by transformation of the Sp. z o.o. (KRS 0000037763) and kept the same NIP and REGON | 20376473 (REGON 090484404, head "REMONDIS Bydgoszcz S.A.", 20); 20798693 (KRS, 3); provisional 22053675 (the double-space spelling, 6) |
| Digitex Sp. z o.o. Sp. k. (1) | **20111694** Digitex Sp. z o.o.sp.k., 6 | NIP 5851476863 | MF: DIGITEX Sp. z o.o. Sp.k., REGON 366410177, Sopot. **Org 16922331's NIP 5851469863 belongs to "CHATA CHŁOPSKA" s.c.**, so that row is a typo row | 19005947 (REGON 366410177, head "Digitex Sp. z o.o.", 18); provisional 10248431 (6) |
| NOVUM S.A. (1) | **8998814** NOVUM S.A., 18 | NIP 9491924705 | MF: NOVUM S.A., REGON 152185875, KRS 0000368094, Warszawa | 23169433 (REGON, 2; that REGON is also held by "Dorota Kuszneruk", 24498247); provisional 6334556 (94) |
| WTU Spółka z o.o. (1) | **13575097** WTU Sp. z o.o., 13 | NIP 8992680461 | MF: 'WTU' Sp. z o.o., REGON 021065730, KRS 0000340110 | 9713424 (REGON, 4); provisional 1629934 (16) |
| PZU ZDROWIE S.A. Odział Centra Medyczne w Gdańsku (1) | **18389587** PZU Zdrowie Spółka Akcyjna, 22 | NIP 5272663852 | MF: PZU ZDROWIE S.A., KRS 0000395215. The printed "Oddział" is a branch of this legal person and carries no NIP of its own | 20008293 (REGON, 3); 14416611 (KRS, 1) |
| Evalu Sp. z o.o. (1) | **20491831** EVALU Sp. z o.o., 15 | NIP 5262825916 | MF: EVALU Sp. z o.o., REGON 015891042 | 18436807 (REGON, 2); provisional 10008177 (exact spelling, 19) |
| Zarys International Sp. z o.o. (1) | **976** ZARYS INTERNATIONAL GROUP SP .Z O.O., 4,380 | NIP 6481997718 | KRS 0001213002: ZARYS INTERNATIONAL Sp. z o.o. arose on 2025-11-14 by transformation of ZARYS INTERNATIONAL GROUP sp. z o.o. sp.k. (KRS 0000540772) and kept the same NIP and REGON | **9241** (REGON 273295877, head "Zarys International Sp. z o.o.", 2,129): the reviewer's alternative if the head name should match; 23719024 (KRS 0001213002, 2); provisional 6545028 (31) |
| Gminny Zakład Gospodarki Komunalnej Sp. z o.o. (1) | **22583689** Gminny Zakład Gospodarki Komunalnej spółka z o.o., 1 | REGON 061423397 (no org holds its NIP 8250003819) | MF: GZGK Sp. z o.o., Kościelna 46, 21-411 Wojcieszków. That is the street, postcode and town notice 29173048 prints in ORG-0003 (GET of `/v1/notices/29173048/content`; the buyer is RDW Biała Podlaska) | provisional **2199877** (34), rejected: it pools a generic name many gmina companies share |
| Pro3Ds Paweł Krawczyk (1) | **22456216** Pro 3Ds Paweł Krawczyk, 3 | NIP 7123245197 | MF: PAWEŁ KRAWCZYK, Lublin (the buyer is OISW Lublin) | none |

Points for the reviewer:
- **Remondis, Zarys, Digitex.** Their destinations' head names carry the PRE-transformation legal form. They are the same legal persons, by NIP and by the KRS transformation records. The served name changes form, not entity.
- **Zarys alternative.** If the review prefers a head-name match, 9241 (the REGON row with the exact head) is the alternative. Both rows are the same body.

## Read for the apply step (job `apply-rehoming`, dry then wet, then project)

- **Expected dry plan:** exactly 32 moves, each from 49 to the destination above, and `missing_target` 0.
- **The dry plan's parties and bid-parties counts will be larger than the 32 mentions' own rows.** The 32 mentions' own rows are 186 party rows (55 on a current version) and 19 bid-party rows.
  - `apply_rehoming` counts its blast radius by (notice, origin org), not by section (canonical.rs:33950).
  - On 17 of the 32 notices, org 49 also holds the notice's own KIO/UZP section.
  - Those KIO/UZP rows are re-derived unchanged by the refold. The plan reviewer should expect them and not read them as extra moves.
- **Winners.** Org 49 is currently served as the winner on 15 tenders.
  - 13 of them are this cohort's supplier tenders, and the rehome fixes them.
  - The other 2 are **not** fixable by rehoming, because the publisher printed KIO's own name in the tenderer section: tender 816754 (notice 24389363 ORG-0002 "KIO") and tender 1058540 (notice 24613742 ORG-0003 "Krajowa Izba Odwoławcza"). They remain a known W, out of this cohort.
- **Persistence.** A moved mention stays put while its notice keeps publishing the same name, country, raw identifier and scheme (`RecordedMention::publishes`, canonical.rs:15758). A re-parse that changes any of them re-resolves the mention, and today's E0 identifier bind (canonical.rs:23198) would send it back to 49.
- **Recurrence.** A new notice from the same publishers that prints 5262239325 under its own name (UW printed it 13 times in 2025) still binds org 49 until the code half ships (`guard-spec.md`). This is 545's LHP/Accenture caveat.
- **Verify** (issue line). `/v1/tenders/306091` today serves Procedure-Buyer, Lot-TenderEval, Lot-TenderReceipt and Lot-DocProvider as `49 "Krajowa Izba Odwoławcza"` with `mention_name` "Uniwersytet Warszawski" (GET 7). Done means `7788 "Uniwersytet Warszawski"`.

## Side findings (not in the cohort)

1. **The mirror shape: KIO's name under a buyer's identifier.**
   - Org **11786**, headed "Krajowa Izba Odwoławcza" with 173 mentions, holds **UW's REGON 000001258**, the same literal as UW's org 11785 (S5). UW's template prints its own REGON into the KIO block.
   - Notice 24300743 ORG-0002 prints "Krajowa Izba Odwoławcza" with **BGK's NIP 5250012372** and binds Bank Gospodarstwa Krajowego, org 11293239 (S1).
   - Both are KIO served as another body, the reverse of this issue. Neither is sized here.
2. **The template copy is wider than the NIP.**
   - In notice 29173048 the GZGK section also carries KIO's website (`https://www.gov.pl/web/uzp/krajowa-izba-odwolawcza`, BT-505).
   - So the platform copies the KIO block's contact fields into the next organization section. That supports a source-side cause rather than our identifier read.
3. **A REGON shared by an unrelated sole trader.** NOVUM S.A.'s REGON 152185875 is also held by org 24498247, "Dorota Kuszneruk".

## Budget used (this agent)

- **`/v1/sql`: 6 of the workflow's 10.** Each statement was plan-probed locally first, with turso `plan-probe` on 576-u4's prod-index copy of `577-triage/plan-copy.db`.
  - No plan step SCANs a table. The only SCANs are of LIMITed subqueries; each statement's `raw/sql/*.plan` has the seeks.
  - No 408, no retry. Times: 0.37–0.62 s.
  - The statements: S1 mentions of the 38 notices (175 rows), S2 party, bid-party and winner rows (99), S3 and S4 name-prefix candidates (253 + 83), S5 41 identifier seeks (33), S6 org 49's satellites plus 7 BT-500 rows (14).
- **Public GETs: 7 of 40.** All were sequential, at least 3 s apart, and all answered 200.
- **Registers: 8 WebFetches.** One was a refused 403; the 7 others are listed in `requests.log`.
