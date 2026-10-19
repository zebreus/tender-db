# 530 R2 skeptic: read_inc1's two W (skeptic_inc1)

Scope: the two W in `read_inc1.md`, items 8 and 12. They are one A7 group, AOK Niedersachsen, in its TED copy (notice
24253392, TED 00636996-2024) and its DÖE copy (notice 26483158). The plan entries are `r2_incoherent`:
- `org_ids` [22752175, 22752176, 22752177];
- sections ORG-0001 Procedure-Buyer, ORG-0002 Lot-AddInfo and ORG-0003 Lot-ReviewOrg, all three refused;
- every section is planned whole into target **7861**, "AOK - Die Gesundheitskasse für Niedersachsen": DE, identifier
  NULL, provisional, 714 mentions (`planned_mention_keys`).

Method:
- I tried to show the group is one body printed once per role (R), or that the documents cannot tell (N).
- I re-read both notice bodies field by field, including the `code` values. BT-507 and the DE1 CountrySubentityCode
  come back as `code`, not `value`, so a plain value dump shows them as null.
- I checked every claim behind the W against `bodies/`.
- **New GETs: 0.** Nothing was added to `api_requests.log`. A GET could have changed the verdict only by reaching this
  procedure's other notices (the contract notice of 2024-013-AE) or 7861's own mentions. The notice content carries no
  tender id, and the allowed endpoints have no notice-to-tender or mention listing. No GET could have reached either.

## Verdict: W stands for both copies (moderate to good confidence; the defect is ORG-0003 only)

| key | kind | reader | skeptic |
|---|---|---|---|
| 24253392\|aok - die gesundheitskasse für niedersachsen | incoherent | W | **W (kept)** |
| 26483158\|aok - die gesundheitskasse für niedersachsen | incoherent | W | **W (kept)** |

## What I checked

The facts in read_inc1 hold. They are the same in both copies:

| field | ORG-0001 Buyer | ORG-0002 AddInfo | ORG-0003 ReviewOrg |
|---|---|---|---|
| name (BT-500 / PartyName) | AOK - Die Gesundheitskasse für Niedersachsen | same | same |
| BT-501 / CompanyID | `617a9b4a…` | `dfe86ded…` | `26a934f6…` |
| street, postcode, city | Peiner Str. 8, 30519 Hannover | same | same |
| e-mail | VMS-Vergabe@nds.aok.de | same | same |
| phone | +49 511870115210 | same | **+49 22894990** |
| NUTS (BT-507 / CountrySubentityCode) | DE929 | DE929 | **DEA22** |

- The notice contains no other review section, mediator or review-info section. The only other ORGs are the winner,
  Bechtle GmbH in Hannover (ORG-0004, Tenderer), and DÖE (ORG-0005, SProvider). OPT-301-Lot-ReviewOrg points to ORG-0003
  on both the lot and the procedure. In the DÖE copy, AppealReceiverParty points to ORG-0003.
- **The phone is the VK Bund's, exactly.** Today `+49 22894990` appears in `bodies/` on 21 sections. Nineteen of them are
  sections named Vergabekammer(n) des Bundes [beim Bundeskartellamt]:
  - 24263691 ×2, 24277826, 24293738, 24294429, 24312523, 24330844, 24460436 ×2, 26532531 ×2, 32094941, 44498728, 44501561,
    44539049, 45647132, 47363720, 47393440 and 47401851;
  - these notices come from five buyers (GSI, BLB Brandenburg, BLB NRW, AOK NordWest and IKK classic) and several
    publishing platforms.

  The other two are AOK ORG-0003, one per copy. Read_inc1 counted 15; the folder has grown since then. No section with a
  different name carries that phone.
- **The NUTS is the VK Bund's, not a platform default.** 236 saved sections carry DEA22 (counted across the TED, DE1
  and pre-eForms address fields). 233 say Bonn. One is in Alfter, next to Bonn. The other two are AOK ORG-0003, the only
  DEA22 sections with an address far from Bonn: Hannover, whose own code is DE929.
- **The AOK's review body is the VK Bund, by its own notices.** In the joint AOK procurements 27339608, 27351981 and
  45852543, AOK Niedersachsen is one of the eleven Procedure-Buyers (ORG-0005). Lot-ReviewOrg there is ORG-0012
  "Vergabekammer des Bundes", Villemombler Straße 76, Bonn, 0228-94990, DEA22, printed under its own name. The AOK
  NordWest notices do the same (24293738: ORG-0021 Lot-ReviewOrg "Vergabekammern des Bundes").

## Why R fails

R requires that the platform printed one body once per role. That is the shape of all 28 R items in read_inc1 and
read_inc2: every refused section carries the buyer's own phone, NUTS, postcode and city. The only differences are a URL,
a fax typo, and the Handwerkskammer's general e-mail and street spelling.
- ORG-0002 here is that shape. It is a byte-for-byte copy of ORG-0001.
- ORG-0003 is not. It differs from the buyer in exactly the two fields that locate a review chamber, and both carry the
  values of one specific other body. Two independent fields matching the same body exactly cannot be a typo.
- Compare GSI (read_inc2 item 18), which is R. GSI's ReviewOrg section carries GSI's own phone and NUTS, and the VK
  Bund's values sit only in the VK Bund's own sections. Here the VK Bund's values sit inside the section that has the
  AOK's name.

## Why N fails

The case for N is that the name, street, postcode, city and e-mail are the AOK's, five fields against two, and no
second name is printed anywhere in the notice. Three things defeat it:
- **The two divergent fields are a signal, not noise.** They come from one specific body. Among 236 DEA22 sections,
  this section (in its two copies) is the only one with a far-off address.
- **The slot is the review body's, and this buyer fills it with the VK Bund.** The joint notices show the AOK's own
  practice. This notice has no other review section where the VK Bund could have been named.
- **The likely generative story is therefore clear.** The platform copied the buyer's data into the ReviewOrg form,
  which is the per-role copy behind the 28 R items. The publisher then entered the VK Bund's phone and region but did not
  overwrite the name, address or e-mail.

  So the publisher meant a different body for this section. That is A7's W shape: "a different body printed under the
  buyer's name (a Lot-ReviewOrg …)".

Two further rescue arguments fail:
- **"Binding would not send it to 255 either."** That is true. Under binding, ORG-0003 stays on 22752177, a single-use
  fragment that is also named AOK. A7's remedy accepts that on purpose: it keeps a different body's mention off the
  buyer's 234 row rather than merge it in. Folding makes 7861 claim a ReviewOrg role, on the AOK's own tender, that the
  publisher meant for the VK Bund.
- **"The 234 fallback is the identifier-less baseline."** That bounds the cost. It does not make the fold right.

**What does not apply:**
- This is A7's W, not A6's. The fold does not turn 7861 into a homonym umbrella: every one of its sections is still
  named AOK.
- No stable key is dissolved. `26a934f6…` occurs only in this notice's two copies (22752177 holds 2 mentions), and its
  twins prove per-section minting.

## What would flip it

Either of the following would move the verdict to N:
- a document showing that the AOK Niedersachsen prints this exact hybrid section as its standing "review contact" on
  its own notices, which would make it the buyer's own template;
- this procedure's contract notice naming the AOK, with Hannover data, in the ReviewOrg slot.

Neither is reachable through the allowed endpoints, and I found neither in `bodies/`.

## Scope and consequence

- **The defect is ORG-0003 only: 1 mention per copy, 2 in total.** ORG-0001 and ORG-0002 are the AOK. Folding their 4
  mentions into 7861 is right.
- **A7 is triggered.** Before Build 2, `guid_twin_refusals` gains a role-coherence guard.
- **Recommended shape: read_inc2's.** In a buyer-family + Lot-ReviewOrg group, refuse only when every section's
  **phone, NUTS, postcode and city** agree within the notice.
  - The fields are BT-503, BT-507, BT-512 and BT-513, or the DE1 Contact-Telephone, CountrySubentityCode, PostalZone and
    CityName in the DÖE copy.
  - Compare phones after stripping spaces and dashes.
  - Do not compare the e-mail, street, fax, URL or contact.
  - On today's 30 items this binds exactly the AOK group (both copies) and refuses all 28 R items.
  - inc1's exact-agreement variant would wrongly bind the 6 Handwerkskammer items.
- **Optional refinement for Build 2 to weigh.** Bind only the divergent ReviewOrg section, and keep refusing the
  sections that agree with the buyer. ORG-0001 and ORG-0002 still form a group with 2 distinct values, so they would still
  refuse.
  - This saves the 4 right AOK mentions.
  - 22752177 holds only ORG-0003's mentions, so it stays whole as a kept row, and 22752175/76 are still planned whole.
  - A7 as written binds the whole group. That is simpler and costs only those 4 mentions, which stay split as today.
- **A5 does not trigger.** These are A7 picks, not random-60 rows, so they do not trigger A5's salt-530003 re-read. They
  count against R2's "0 W" bar on the incoherent stratum until the guard lands, followed by a re-run of the dry plan
  (R1).
