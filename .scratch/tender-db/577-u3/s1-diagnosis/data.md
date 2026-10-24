# 577 U3 slice 1: data diagnosis (party rows of the sample Tenders vs their notices)

Read-only, 2026-10-24 19:56–20:07Z, at prod rev 6cb4408. No cargo, no git writes.
- **Box reads by this line:** 5 bounded `/v1/sql` statements (`raw/s1_versions`, `s2_parties`, `s3_bid_parties`,
  `s4_changes`, `s5_mentions`; `.sql` and `.out` in `raw/`, log in `raw/sql-state.json`). All were plan-probed first:
  SEARCH only, about 0.4 s each, no 408, none truncated. With the code line's `code-r1`, the workflow has used 6 of 8.
- **Archive:** 2 members extracted with `unzip -p` from the DÖE monthly zips. `xml/26213590.xml` comes from
  `doe/monthly/2023-11.zip` (sha256 5d43…9fed). `xml/26198825.xml` comes from `2023-10.zip` (sha256 bec8…a60f).
- **GETs:** 4, spaced 3 s apart (`raw/get1`–`get4`).

## Verdict

- **The stored party rows of the sample are complete and correct for their notices.** Every role the XML declares is
  present exactly once per scope, on the right lot. It binds the right org, which is the same org the TED twin's mention
  of the same section binds. There are no duplicates within a version and no missing roles.
- **The TED copies (sdk-1.7/1.10/1.12) were not rewritten.** Their rows still carry 10-08 rowids. They show the same role
  pattern as the DÖE version, plus the eSender's `Procedure-SProvider`, which DÖE notices do not publish.
- **Class: neither order-only nor content.** It is the reparse's own delete followed by a restore. 2773's
  `reparse_notice` → `clear_parsed` deletes every `tender_version_parties` and `tender_version_bid_parties` row whose
  `mention_notice_id` is a re-parsed notice (crates/store/src/lib.rs at 6cb4408, `clear_parsed`; the code line found
  it, see `code.md`). Project 2774 then compared a stored side that was missing those rows against a fresh fold that
  has them. So every table citing a de-1.x mention "differed" and was re-inserted.
- **The data fits that and nothing else:**
  1. Every DÖE-cited table was rewritten as a whole, including its carried-forward TED rows.
  2. Every TED-only table kept its old rowids.
  3. All lots were announced.
  4. Where it can be measured, the row counts before and after are equal.
  5. The restored rows match the XML.
- The expected net change in served party content against the state before 2773 is 0. The compare can neither confirm
  nor refute that, because the old rows were gone before it ran.
- **The 389 dropped placeholder parties are normal.** 539 run b's project 2738 folded the same 218,876 notices and
  reported exactly `0 void, 389 placeholder (195 with an identifier …)`.
- **Live defect (served).** Each reparse slice opens a window in which the Tenders it touches serve no DÖE party or
  bid-party rows until the paired project ends. Slice 2's window is open now: see §5.

## 1. What the rows look like now (s1 + s2 + s3, 19:57Z)

rid is the rowid range. A DÖE de-1.x version (**bold**) carries 809,774,782–829, one contiguous block written by
2774. Every other version carries the 10-08 block (667,9xx,xxx).

| tender/seq | notice (profile) | rows | rid | Tender-scope roles | per-lot roles (lots) | carried from another notice | bid parties (rid) |
|---|---|---|---|---|---|---|---|
| **2/1** | 26213590 (de-1.1, sub 16) | 8 | 809774782–789 | Buyer, L-ReviewInfo, L-ReviewOrg, L-TenderReceipt | AddInfo, ReviewInfo, ReviewOrg, TenderReceipt (1) | – | – |
| 2/2 | 23555356 (sdk-1.7) | 9 | 667936401–409 | same + SProvider | same (1) | – | – |
| **26/1** | 26198825 (de-1.1, sub 20) | 11 | 809774790–800 | Buyer | AddInfo, ReviewOrg (5) | – | – |
| 26/2 | 23491395 (sdk-1.7) | 12 | 667941060–071 | Buyer, SProvider | AddInfo, ReviewOrg (5) | – | – |
| **26/3** | 26290799 (de-1.1, sub 33) | 17 | 809774801–817 | Buyer, SProvider | AddInfo, ReviewOrg, Tenderer (5) | SProvider ← 23491395, at rid **809774802** | 5 (563858742–746) |
| 26/4 | 23851800 (sdk-1.7) | 17 | 667941089–105 | Buyer, SProvider | AddInfo, ReviewOrg, Tenderer (5) | – | 10 (563858747–756): 5 DÖE-round rows ← 26290799 plus its own 5 |
| **28/1** | 26608207 (de-1.2) | 4 | 809774818–821 | Buyer | AddInfo, ReviewOrg, TenderReceipt (1) | – | – |
| 28/2 | 24610830 (sdk-1.10) | 5 | 667941118–122 | Buyer, SProvider | same (1) | – | – |
| **28/3** | 26635993 (de-1.2) | 5 | 809774822–826 | Buyer, SProvider | same (1) | SProvider ← 24610830, at rid **809774823** | – |
| 28/4 | 24720730 (sdk-1.10) | 5 | 667941128–132 | Buyer, SProvider | same (1) | – | – |
| 28/5 | 26710678 (de-2.0, slice 2) | 6 | 667941133–138 | Buyer, SProvider | AddInfo, ReviewOrg, TenderReceipt, Tenderer (1) | SProvider and TenderReceipt ← 24720730 | 1 (415558793) |
| 28/6 | 24872092 (sdk-1.12) | 6 | 667941139–144 | Buyer, SProvider | same as 28/5 (1) | TenderReceipt ← 24720730 | 2 (415558794 ← 26710678, 415558795 own) |
| **37/1** | 26588853 (de-1.1) | 3 | 809774827–829 | Buyer | ReviewInfo, ReviewOrg (1) | – | – |
| 37/2 | 24666434 (sdk-1.7) | 4 | 667941372–375 | Buyer, SProvider | same (1) | – | – |

Every section on DÖE and TED sides binds the same org:

| section | org | name |
|---|---|---|
| tender 2 ORG-0001/0002/0005 | 22183653 | Landkreis Göttingen |
| tender 2 ORG-0003/0004 | 21338391 | Vergabekammer Niedersachsen |
| tender 26 ORG-7001 | 3517 | BA REZ Südwest |
| tender 26 ORG-7004 | 3518 | Bundeskartellamt VK Bund |
| tender 26 winners | 22271067, 22559921, 22559922 | – |
| tender 28 ORG-7001 | 574 | – |
| tender 28 ORG-7004 | 1690 | – |
| tender 28 winner | 22612898 | – |
| tender 37 | 17627248 and 1513 | – |

What these rows show:
- **What was rewritten.** It is exactly the set of tables holding at least one de-1.x-cited row: all 6 DÖE de-1.x party
  tables, and the bid parties of 26/3 and 26/4. The TED rows inside them were rewritten too: 809774802 and 809774823
  are carried-forward sdk rows inside the new block, and 26/4's own TED round is inside 563858747–756.
- **What was kept.** Tables citing only sdk-1.x or de-2.0 mentions kept their rows. That covers every TED version's
  parties, 28/5 and 28/6's parties, and 28's bid parties.
- **No change rows in between.** s4 shows no change row for these 4 Tenders or their 8 lots between the 10-08
  replay (cursor ~849.6M, the likely writer of the 667.9M rows) and 2774. 539 run b (2738) re-folded all four and
  verified them unchanged. 546's 2765 and the Touchpoint refold 2769 either verified them or did not touch them.
- **Equal row counts before and after, where measurable.**
  - The 10-08 replay wrote each Tender's versions contiguously. The rowid gap a rewritten version left is therefore its
    old row count.
  - 26/3: gap 667941072–088 = 17 slots, and 17 rows now.
  - 28/3: gap 667941123–127 = 5 slots, and 5 rows now.
  - For the other 4 DÖE versions the start of the gap cannot be bounded without a neighbouring Tender's rows.
- **Bid parties are dated by inference.** Tender 26's bid parties (563.86M) are about 148M rowids newer than tender
  28's (415.56M), which were written in the same 10-08 replay. With no change row between the two writes, only 2774
  can have rewritten them.
- **Change rows of 2774 (s4, cursor 964676277–288).** Every Tender has one `tender changed`, and every lot it holds has
  one `lot changed`: 2:{2}, 26:{133..137}, 28:{139}, 37:{153}. This fits rows missing on the stored side. In these
  tables every row cites the DÖE notice, apart from the carried SProvider at Tender scope, so every lot is announced.
  At corpus scale, 126,812 + 200,027 = 326,839 correction rows. Tables rewritten: 220,649 parties + 186,010 bid
  parties = 406,659, which matches the summary.

## 2. Notice XML vs stored rows

**26213590** (tender 2 v1; `xml/26213590.xml`, ContractNotice, eforms-de-1.1, subtype 16).
- 5 ORG sections and no TouchPoint:
  - ORG-0001, ORG-0002, ORG-0005: Landkreis Göttingen, DE 308 703 297.
  - ORG-0003, ORG-0004: Vergabekammer Niedersachsen, t:04131153308.
- 1 lot: LOT-0001 (`schemeName="Lot"`).

| XML element | ref | DE1 id (parse now, get1) | role (DE1_FIELD_ALIASES) | stored row |
|---|---|---|---|---|
| ContractingParty/Party/PartyIdentification/ID | ORG-0001 | DE1-ContractingParty-Party-PartyIdentification-ID | Procedure-Buyer | T, 22183653, ORG-0001 ✓ |
| /TenderingTerms/TenderRecipientParty | ORG-0005 | DE1-TenderingTerms-TenderRecipientParty-… | Lot-TenderReceipt | T, 22183653, ORG-0005 ✓ |
| /TenderingTerms/AppealTerms/AppealInformationParty (`schemeName=organization`) | ORG-0003 | DE1-TenderingTerms-AppealTerms-AppealInformationParty-… | Lot-ReviewInfo | T, 21338391, ORG-0003 ✓ |
| /TenderingTerms/AppealTerms/AppealReceiverParty | ORG-0004 | DE1-TenderingTerms-AppealTerms-AppealReceiverParty-… | Lot-ReviewOrg | T, 21338391, ORG-0004 ✓ |
| LOT-0001 …/AdditionalInformationParty | ORG-0002 | DE1-ProcurementProjectLot-…-AdditionalInformationParty-… (lot-only id; no move) | Lot-AddInfo | lot 2, 22183653, ORG-0002 ✓ |
| LOT-0001 …/TenderRecipientParty | ORG-0005 | DE1-ProcurementProjectLot-…-TenderRecipientParty-… (moved by 577 from DE1-TenderingTerms-…) | Lot-TenderReceipt | lot 2, 22183653, ORG-0005 ✓ |
| LOT-0001 …/AppealInformationParty | ORG-0003 | DE1-ProcurementProjectLot-…-AppealInformationParty-… (moved) | Lot-ReviewInfo | lot 2, 21338391, ORG-0003 ✓ |
| LOT-0001 …/AppealReceiverParty | ORG-0004 | DE1-ProcurementProjectLot-…-AppealReceiverParty-… (moved) | Lot-ReviewOrg | lot 2, 21338391, ORG-0004 ✓ |

8 declared refs and 8 stored rows, one to one. Each moved pair aliases to the same `OPT-301-Lot-*` role on both
spellings, so the old parse folds to the same 8 rows. That matches 26/3 and 28/3's equal row counts in §1.

**26198825** (tender 26 v1; `xml/26198825.xml`, eforms-de-1.1, subtype 20).
- 2 ORG sections and no TouchPoint:
  - ORG-7001: BA REZ Südwest, DE811458858 (group lead / awarding CPB).
  - ORG-7004: Bundeskartellamt VK Bund, `---`.
- 5 lots: LOT-0001 to LOT-0005.
- Declared refs:
  - ContractingParty → ORG-7001.
  - On each lot: AdditionalInformationParty → ORG-7001 and AppealReceiverParty → ORG-7004.
  - Each lot's TenderRecipientParty carries only an EndpointID (`http://www.evergabe-online.de`), so it names no party.
  - No procedure-level TenderingTerms parties.
- Stored rows: Buyer 3517 ORG-7001, plus on each of lots 133–137 Lot-AddInfo 3517 ORG-7001 and Lot-ReviewOrg 3518
  ORG-7004. That is 11 = 1 + 5×2 ✓.
- No TenderReceipt row, which is correct: there is no PartyIdentification.

## 3. TED copies (not re-parsed) vs DÖE versions

- **Not rewritten.** Every TED version's party table kept its 10-08 rowids, so 2774 compared it identical.
- **Same pattern.** For each Tender the TED version carries the same pattern as the DÖE version: same roles per scope,
  same orgs, same section ids. The only addition is the eSender `Procedure-SProvider`: 9200 Beschaffungsamt des BMI,
  or 28 Datenservice Öffentlicher Einkauf.
  - Row multiplicity pairs (DÖE/TED): 8/9, 11/12, 17/17, 4/5, 5/5, 3/4.
  - The 17/17 and 5/5 pairs are DÖE versions that carry the TED SProvider forward from the previous version.
- **Bid parties.** The TED head 26/4 serves the DÖE round (bids 6913014–18 ← 26290799) and its own round (bids 136–140
  ← 23851800), with the same 5 tenderers twice. This is the existing per-notice round design, and both rounds were
  restored together. It is not a slice-1 change.

## 4. The 389 placeholder parties (issue 511)

- **Same count every run.** 2738 (539 run b, the same 218,876 de-1.x notices, rev 512b297) reported
  `dropped parties (issue 511): 0 void, 389 placeholder (195 with an identifier …)`. The sample is DÖE review and
  mediation slot pointers:
  - `'s.o.'` in ORG-0003/0004 with the buyer's echoed VAT `DE 164 242 157`;
  - `'s.o.'` ORG-7005/7006 [022894990];
  - `'siehe I.1) Öffentlicher Auftraggeber'` [01061-0000-66];
  - `'siehe I.1 und VI.4.1'` [13-X71610001000-88].
  - The source is `551-lot-sum/b6/u9/jobs/chain-1024.json`.
- **Same notices, same count.** The drop is decided in every fold from the parse, so a fold of the same notices reports
  the same count each time. 2774's 389 equals 2738's. These parties were not served before slice 1 and are not served
  now. They are not part of the corrections.
- **Slice 2.** For de-2.0/2.1, run c's 2741 reported 199 placeholder. Expect about 199 from 2777.

## 5. Defects found

1. **A reparse deletes the served party rows until its project runs (live now, slice 2).**
   - At 19:57Z tender 28's bid-party row 415558794 (bid 141, ← 26710678, de-2.0) existed (s3).
   - At about 20:06Z `GET /v1/tenders/28` (`raw/get3`) serves that DÖE bid (`TEN-0001`, notice 26710678) with
     `"parties": []`. The TED bid and both lot results still name the winner 22612898.
   - So reparse 2776 has cleared it, and project 2777 has not restored it yet.
   - Every slice-2 Tender is in this state for the window. It loses its de-2.x-cited buyer, review, AddInfo and
     Tenderer rows on every version, and the head's `bids[].parties` from DÖE rounds.
   - The same happened for slice 1's 126,812 Tenders between 2773 and the end of 2774. Slice 3 will repeat it.
   - The banner should say so while a slice runs.
2. **The compare cannot test party content on a reparse.** Because of (1), any reparse "corrects" these two tables
   whatever the parser does. The slice bar ("0 corrections outside the NOTE classes") cannot be met as the code stands.
   A content check needs a before/after read. Slice 2's before-image is partly in hand (§6).
3. **Side observation, not slice 1, not investigated.** Org 574 (`GET /v1/organizations/574`: national id
   062593043574, 3,288 mentions) serves the head name **"Vergabekammer des Landes Hessen beim Regierungspräsidium
   Darmstadt"**. Its mentions here are the buyer, "Land Hessen, vertreten durch den Landesbetrieb Bau und Immobilien
   Hessen, Zentrale Vergabe". So tender 28's buyer reads as the review chamber. The real chamber, org 1690, carries the
   same name. This looks like the buyer's Leitweg id echoed into review slots (the 511 / 572 / 586 family). Org names
   are current-state, so the reparse cannot have caused it.

## 6. Before-image for slice 2, and a check

Tender 28's de-2.0 rows were read at 19:57Z, before 2776 cleared them (`raw/s2_parties.out`,
`raw/s3_bid_parties.out`). They are the only pre-reparse rows any slice has kept.
- **28/5 parties, in order:**
  1. Procedure-Buyer 574 ←26710678 ORG-7001;
  2. Procedure-SProvider 28 ←24720730 ORG-7005;
  3. lot 139: Lot-AddInfo 574 ←26710678 ORG-7001;
  4. Lot-ReviewOrg 1690 ←26710678 ORG-7004;
  5. Lot-TenderReceipt 574 ←24720730 ORG-7001;
  6. Tenderer 22612898 ←26710678 ORG-0001.
- **Bid parties:**
  - 28/5: bid 141 (lot 139) tenderer 22612898 ←26710678 ORG-0001.
  - 28/6: the same row, then bid 142 (lot 139) tenderer 22612898 ←24872092 ORG-0001.

After 2777, one statement of the s2/s3 shape on `tender_id = 28` should show the following. Any other difference is a
real content change from the de-2.x reparse.
- The same values in the same order.
- New rowids on 28/5's parties and on the bid parties of 28/5 and 28/6.
- 28/6's parties untouched, because they cite no de-2.x mention.

For slice 3, snapshot 3–5 sdk-1.x Tenders the same way before 2778 starts, as `code.md` §6 proposes.

## Files

- `raw/s1_versions.{sql,out}`, `raw/s2_parties.{sql,out}`, `raw/s3_bid_parties.{sql,out}`, `raw/s4_changes.{sql,out}`,
  `raw/s5_mentions.{sql,out}`, `raw/sql-state.json`.
- `raw/get1-26213590-content.json`, `raw/get2-26198825-content.json`, `raw/get3-tender-28.json`,
  `raw/get4-org-574.json`.
- `xml/26213590.xml`, `xml/26198825.xml`.
