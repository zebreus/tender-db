# 545 u3, second skeptic pass, row 3: Hull ×1 (guard item d). Result: NOT REFUTED (2026-10-19)

**Verdict under test** (`cohort.json`, a guard addition):

| org_id (holder) | notice_id | section_id | target_org_id | target_name |
|---|---|---|---|---|
| 30975996 | 46760078 | ORG-GB-COH-02007321 | 30914698 | Hull City Council |

**Correction to the task label.** The label reads "printed on row 31541038 keyed GBCOH168524345". That row is not the holder.
- 31541038 was unit 2b's refused **destination**. It is the REFUSE_DEST row in u2b `notes.md` line 114.
- The verdict's holder is **30975996, ARTIFAX SOFTWARE LIMITED**. The live tender (below) shows that org holding the mention today.
- The POST therefore moves the mention 30975996 → 30914698. That is the right source.

**GETs.** I made 4 new public GETs, all rc=200, with no retries. They were spaced 2 s apart under the shared `.get.lock` and logged with the `(refute2)` tag in `api_requests.log` and `refute2_3_gets.log`. The script is `refute2_3_get.sh`. The bodies are saved in `bodies/`.
- `/v1/tenders/8639942`
- `/v1/organizations/30975996`
- `/v1/organizations/31541038`
- `/v1/organizations?name_prefix=Hull%20City%20Council&limit=50` (`more`=false, so this is the complete list)

I also reused these saved bodies: `v1_notices_46760078_content.json`, `v1_organizations_30914698.json` and `v1_organizations_18819350.json`.

## 1. Is the printed name a different legal entity from the holder? Yes

**What the notice prints** (46760078, publication 071180-2025, subtype "tender"):
- Section `ORG-GB-COH-02007321`: BT-500 "Kingston Upon Hull City Council", BT-501 GB-COH-02007321, country GB.
- It is referenced exactly once, as `OPT-300-Procedure-Buyer`.
- Section `ORG-GB-FTS-41663`: "Hull City Council", with no identifier. It is the `OPT-301-Lot-ReviewOrg`.
- There is no tenderer, no bid, no contract and no other organisation.

**The holder.** 30975996 is keyed GBCOH02007321, merged with GBPPONPBTC8575BQPM, and served as "ARTIFAX SOFTWARE LIMITED" with 8 mentions. Its other 7 mentions all print Artifax names (u2b `cand2b.json`, field `others`). So 02007321 really is Artifax's company number.

**Why the name, not the number, is right.** The lot reads "A contract for The provision of **Artifax** Event Management Software for Theatre, Halls & Museums via the CCS G-Cloud 14 Framework (RM1557.14)", place UKE11 (Kingston upon Hull). Artifax is the supplier of the product being bought.
- The role (buyer), the printed name, the place and the review body all say Hull. Only the number says Artifax.
- So the number is the error and the name is correct. This is the opposite of the Monthind/Cotswold case, where the name was the error.

**None of the near-miss relations applies.** A software vendor and a local authority cannot be:
- a trading or former name of one another;
- agent and principal (Artifax does not buy on G-Cloud for Hull; G-Cloud call-offs are made by the buyer);
- consortium members or sister bodies;
- the same body.

## 2. Is target_org_id the same legal entity as the printed name? Yes

- 30914698 is identified (not provisional), keyed GBPPONPRNP1874YZZR, served as "Hull City Council", with 360 mentions. adv2b c2 also confirmed it.
- "Kingston upon Hull City Council" is the council's formal name and "Hull City Council" is its everyday name. They are one corporate body.
- The same notice prints the council as "Hull City Council" in the review section, which supports reading the two names as one.

**Every Hull row seen names the council. None is a council company** (such as a culture or leisure trust):

| Row | Key | Mentions | Provisional | Served name |
|---|---|---|---|---|
| 1290903 | none | **818** | yes | Hull City Council |
| 30914698 | GBPPONPRNP1874YZZR | 360 | no | Hull City Council |
| 4354314 | none | 9 | yes | Hull City Council |
| 18819350 | 168524345 | 1 | no | Hull City Council |
| 31541038 | GBCOH168524345 | 1 | no | Kingston Upon Hull City Council |

The remaining 25 prefix matches are provisional department or address variants of the council, such as "Monitoring Officer", "Procurement Team" and "City Treasury".

## 3. Does the section bind only this party in this notice? Yes

- The section is referenced once, in a single role. `/v1/tenders/8639942` shows that same role on 30975996: Procedure-Buyer, mention_name "Kingston Upon Hull City Council".
- The tender has one version (seq 1, caused by 46760078). So no other notice of this key exists: n=1, and first = last = 46760078.
- The review section is separate and stays where it is.

## 4. Would the move lose information? No

- Artifax holds no true role on tender 8639942. It is falsely shown as the **buyer**.
- Artifax's real relation, as supplier of the named product, is not printed as a party anywhere on the notice. Only the prose implies it, and that prose stays on the tender.
- The holder is not the real buyer with the printed name as its delegate. The real buyer is the printed name.
- After the move, the tender shows the correct buyer instead of a software vendor.

## Findings that do not refute but should be fixed in the record

**F1. Guard (d) understates the Hull split.**
- It says "Hull is split over three identified rows … 30914698 the unique maximum". That is true among **identified** rows only.
- Provisional row **1290903 "Hull City Council" has 818 mentions**, more than 30914698. It also holds **this same tender's review-org mention**.
- The pick of 30914698 stands under the cohort's own practice:
  - identified rows are preferred;
  - a provisional destination is used only when it is the only exact-name candidate (Compendium, Law Society, TPS);
  - the decision's literal reading asks for "a GBCOH- or GBPPON-keyed row".
- All of these rows are the same legal entity, so the choice does not affect correctness.
- But u3.md's merge-debt line "Hull: 3 rows" should read **at least 5 rows**: 1290903, 30914698, 4354314, 18819350 and 31541038, plus about 25 provisional department variants.

**F2. Expected dry-plan shape for unit 4.** After the move, tender 8639942 will show:
- the buyer on 30914698;
- the review org on 1290903.

That is two different "Hull City Council" rows on one tender. It is harmless, but it should not be mistaken for a wrong move.

**F3. procedure_key.** It is `refused:ocds-h6vhtk-05dabf:GB:national:GBCOH02007321`, which embeds the old buyer key. Unit 4 should note whether the rehome re-keys it or leaves it. Either way it is only a label, because the key is already in its refused form.

**F4. Name gate.** The target name ("Hull City Council") differs from the printed name ("Kingston Upon Hull City Council"). u3.md already lists this pair under the apply-step name differences.

## Verdict

| notice_id | section_id | refuted | why |
|---|---|---|---|
| 46760078 | ORG-GB-COH-02007321 | **false** | Printed buyer is Hull's council, carrying supplier Artifax's company number (30975996's own key) on a G-Cloud call-off for Artifax software in UKE11. The same notice names "Hull City Council" as review body. The section is one party in one role, on one notice. 30914698 (PPON, 360, served "Hull City Council") is the council. Nothing true is lost. |
