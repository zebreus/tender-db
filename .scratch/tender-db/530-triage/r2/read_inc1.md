# 530 R2: role-incoherent groups, items 0-14 (read_inc1)

Source: `guid_twin_plan_2659.json` `body.r2_incoherent[0..14]`. Key = `notice_id|name_norm`. Each item is a refused twin
group (rule 530-T) whose sections mix a buyer-family role with Lot-ReviewOrg (A7). Every item is DE, every org id in it
is in `planned_row_ids`, and every GUID row read holds 2 mentions (the TED copy plus the DÖE copy of one notice).

Question (A7): is refusing every section of the group right, because one body was printed once per role (**R**)? Or is
it wrong, because two different bodies share the name in that notice, for example a buyer and a separate review body
(**W**)?

**Evidence for each item.**
- `/v1/notices/{id}/content` for both copies:
  - the TED copy (eForms SDK 1.7): BT-500, BT-501, BT-503, BT-506, BT-507, BT-510(a), BT-512 and BT-513, with roles
    from OPT-300 and OPT-301;
  - the DÖE copy (eforms-de-1.1): the DE1 `PartyName`, `PartyLegalEntity-CompanyID`, `Contact-*` and `PostalAddress-*`
    fields, with roles from `ContractingParty`, `AdditionalInformationParty`, `AppealReceiverParty`,
    `TenderRecipientParty` and `MediationParty`.
- Each section's BT-501 matched the report's `values`, in the same order, for all 15 items.
- Every refused section's contact fields were compared field by field with the buyer section's fields.
- `/v1/organizations/{id}` for the Lot-ReviewOrg GUID row of every distinct group, and for the targets known from
  `planned_mention_keys` or `r2_random`:
  - 8907951 Universitätsklinikum Essen;
  - 7861 AOK Niedersachsen;
  - 2383569 Kerpen, reused from read_rand1.
- The targets of the JVA Rheinbach, H-BRS and RBB keys are not in the truncated `planned_mention_keys`, and the allowed
  endpoints have no name lookup. Those targets were not read. That does not change the A7 question, which is about the
  group.

**GETs.** 27 public GETs, tag `inc1`: 16 notice contents and 11 organizations.
- All returned 200, with no 408s.
- They were spaced at least 2.1 s apart behind the shared `.api.lock`/`.api.last`, and every one is logged in
  `api_requests.log`.
- Notice 26549982 is item 15, the DÖE copy of item 11. It was fetched to check item 11 and is not scored here.

## Tally

**Items 0-14: R 13, N 0, W 2.** The two W are one group in its two copies: items 8 and 12, AOK Niedersachsen, notices
24253392 and 26483158.

- **The 13 R.** Each item is 6 distinct groups in their TED and DÖE copies (two of them hold two procedures). Every
  refused section of a group repeats the buyer's name, street, postcode, city, phone, e-mail and NUTS exactly. The only
  exception is Kerpen, whose buyer section adds a website URL.
  - So the platform (evergabe.nrw, vmp-rheinland or the RBB publisher) printed the buyer once per role.
  - Where the real review chamber is named, it has its own name and its own section:
    - Vergabekammer bei der Bezirksregierung Köln as ReviewInfo/Mediator for H-BRS;
    - Vergabekammer Rheinland as ReviewInfo for Kerpen.
  - Folding these groups into the buyer's 234 row is what the notices say.
- **The W.** AOK's Lot-ReviewOrg section, ORG-0003, prints the AOK's name, street, postcode, city and e-mail. Its phone
  `+49 22894990` and NUTS `DEA22` (Bonn) are not the buyer's (`+49 511870115210`, `DE929`). They are the values of the
  **Vergabekammern des Bundes**: that exact phone string appears under "Vergabekammer(n) des Bundes", Bonn, in 15 other
  sections in `bodies/`. Several of those are AOK NordWest notices, where the AOK family names the VK Bund as its review
  body.
  - The AddInfo section, ORG-0002, is a byte-for-byte copy of the buyer. ORG-0003 differs in exactly the two fields that
    identify the review chamber.
  - So ORG-0003 is a separately entered review body that carries the buyer's name. That is A7's W shape: the fold would
    give 7861 (AOK, 714 mentions) a ReviewOrg mention the publisher meant for the federal chamber.
  - Confidence is moderate. On name and address alone it reads R, and no second body's name is printed anywhere in the
    notice. Even so, it is not "one body printed per role", because the copy was edited.
  - The buyer and AddInfo sections in the same group are the AOK and fold rightly.

**Consequence under A7.** Any W means that, before Build 2, `guid_twin_refusals` gains a role-coherence guard under
which these groups bind.
- A guard keyed only on the role set (buyer-family + Lot-ReviewOrg) would also keep the 13 R groups split.
- A narrower pure guard would catch this W and still refuse the R groups. Under it, a buyer + ReviewOrg group refuses
  only when every section's phone, e-mail, NUTS, street, postcode and city agree within the notice, and otherwise binds.
  The fields come from the notice's own BT-503/506/507/510/512/513, or their DE1 equivalents in the DÖE copy.
- That is a suggestion for the Build 2 decision. It was not measured across the other 15 groups (items 15-29).

| # | notice | name | GUID rows | sections and roles | target | contact fields vs buyer | verdict |
|---|---|---|---|---|---|---|---|
| 0 | 24311920 (TED) | Universitätsklinikum Essen, AöR | 22795019-22 | 0001 Buyer, 0002 AddInfo, 0003 ReviewOrg, 0004 TenderReceipt | 8907951 (207) | identical | R |
| 1 | 24363701 (TED) | Justizvollzugsanstalt Rheinbach | 22834907-10 | 0001 Buyer, 0002 AddInfo, 0003 ReviewOrg, 0004 TenderReceipt | not read | identical | R |
| 2 | 24385146 (TED) | Hochschule Bonn-Rhein-Sieg | 22851636/37/39/41 | 0001 Buyer, 0002 AddInfo, 0004 ReviewOrg, 0006 TenderReceipt | not read | identical | R |
| 3 | 24453957 (TED) | Kolpingstadt Kerpen | 22905103/04/06/08 | 0001 Buyer, 0002 AddInfo, 0004 ReviewOrg, 0006 TenderReceipt | 2383569 (97) | identical except buyer's URL | R |
| 4 | 26492363 (DÖE) | Universitätsklinikum Essen, AöR | as 0 | as 0 | 8907951 | identical | R |
| 5 | 26496944 (DÖE) | Justizvollzugsanstalt Rheinbach | as 1 | as 1 | not read | identical | R |
| 6 | 26506733 (DÖE) | Hochschule Bonn-Rhein-Sieg | as 2 | as 2 | not read | identical | R |
| 7 | 26522080 (DÖE) | Kolpingstadt Kerpen | as 3 | as 3 | 2383569 | identical except buyer's URL | R |
| 8 | 24253392 (TED) | AOK - Die Gesundheitskasse für Niedersachsen | 22752175-77 | 0001 Buyer, 0002 AddInfo, 0003 ReviewOrg | 7861 (714) | ReviewOrg: phone and NUTS are VK Bund's | **W** |
| 9 | 24417385 (TED) | RBB Vermögensgesellschaft mbH & Co. KG | 22876714-16 | 0001 Buyer+AddInfo, 0002 ReviewOrg, 0003 Mediator | not read | identical | R |
| 10 | 24417606 (TED) | RBB Vermögensgesellschaft mbH & Co. KG | 22876875-77 | 0001 Buyer+AddInfo, 0002 ReviewOrg, 0003 Mediator | not read | identical | R |
| 11 | 24527963 (TED) | Justizvollzugsanstalt Rheinbach | 22963579-81 | 0001 Buyer, 0002 AddInfo, 0003 ReviewOrg | not read | identical | R |
| 12 | 26483158 (DÖE) | AOK - Die Gesundheitskasse für Niedersachsen | as 8 | as 8 | 7861 | as 8 | **W** |
| 13 | 26537626 (DÖE) | RBB Vermögensgesellschaft mbH & Co. KG | as 10 | as 10 | not read | identical | R |
| 14 | 26537732 (DÖE) | RBB Vermögensgesellschaft mbH & Co. KG | as 9 | as 9 | not read | identical | R |

## Items

**0 — 24311920|universitätsklinikum essen, aör — R.** TED 00658945-2024 (contract notice of 2024-10-29, corrigendum
1; "Ausschreibung eines Rahmenvertrages für Clients", via evergabe.nrw).
- It prints four ORG sections named "Universitätsklinikum Essen, AöR", each with its own v4 UUID:
  - ORG-0001 Procedure-Buyer (`59f715d9…`);
  - ORG-0002 Lot-AddInfo (`4d58c6dd…`);
  - ORG-0003 Lot-ReviewOrg on the lot and the procedure (`c0e4fdf2…`);
  - ORG-0004 Lot-TenderReceipt (`8cda1f3b…`).
- All four give Hufelandstraße 55, 45147 Essen, vergabe@uk-essen.de, +49 2017232633, NUTS DEA13 and the same fax. The
  field-by-field comparison finds no difference.
- The only other ORG is DÖE as SProvider. No review chamber is named at all. The ReviewOrg is the buyer printing itself
  into that slot with its own data, not a second body under its name.
- GUID rows: 22795021 (the ReviewOrg) holds 2 mentions, identifier `C0E4FDF2…`. The target is 8907951, "Universitätsklinikum
  Essen, AöR", DE, identifier NULL, provisional, 207 mentions.
- Refusing all four sends one body's four sections to that body's 234 row.

**1 — 24363701|justizvollzugsanstalt rheinbach — R.** TED 00676068-2024 (contract notice of 2024-11-05, "Beschaffung eines
CNC-gesteuerten Bearbeitungszentrums", evergabe.nrw; procedure `613bc252…`).
- ORG-0001 to ORG-0004 are all "Justizvollzugsanstalt Rheinbach", in the roles Buyer, AddInfo, ReviewOrg and
  TenderReceipt. Their values are `ef628945…`, `041a068a…`, `fd0f40cc…` and `88f1c8e1…`.
- All four give Aachener Straße 47, 53359 Rheinbach, vergabe@jva-rheinbach.nrw.de, +492226-86261, NUTS DEA2C. There is
  no difference.
- No other review body is printed. GUID row 22834909 (ReviewOrg) holds 2 mentions.
- JVA Rheinbach is one prison. All four sections are that one body, printed once per role.
- The target was not read, because the key is beyond the truncated `planned_mention_keys`. A JVA's name with its town is
  unique.

**2 — 24385146|hochschule bonn-rhein-sieg — R.** TED 00701992-2024 (contract notice of 2024-11-14, "Laborausstattung
Standort Rheinbach", evergabe.nrw).
- Four refused sections, all "Hochschule Bonn-Rhein-Sieg":
  - ORG-0001 Buyer (`b1cca80d…`);
  - ORG-0002 AddInfo (`d6b8cde2…`);
  - ORG-0004 Lot-ReviewOrg (`4b4556ab…`);
  - ORG-0006 TenderReceipt (`a5c9be65…`).
- All four give Grantham-Allee 20, 53757 Sankt Augustin, einkauf@h-brs.de, 02241865648, DEA2C. There is no difference.
- The actual review chamber is printed separately under its own name: "Vergabekammer bei der Bezirksregierung Köln",
  Zeughausstraße 2-10, 50667 Köln, poststelle@bezreg-koeln.nrw.de, as ORG-0003 ReviewInfo and ORG-0005 Mediator. It is
  a different name and another group, and it is not part of this refusal.
- So the H-BRS ReviewOrg section is the buyer naming itself, not the chamber under the buyer's name. GUID row 22851639
  holds 2 mentions.
- One university. R.

**3 — 24453957|kolpingstadt kerpen — R.** TED 00742477-2024 (contract notice of 2024-12-03, "Sicherheitsdienst für
kommunale Flüchtlingsunterkunft in Kerpen Bruchhöhe", vmp-rheinland).
- Four refused sections, all "Kolpingstadt Kerpen":
  - ORG-0001 Buyer (`6c51a64d…`);
  - ORG-0002 AddInfo (`9c280515…`);
  - ORG-0004 Lot-ReviewOrg (`47297654…`);
  - ORG-0006 TenderReceipt (`7d61f980…`).
- All four give Jahnplatz 1, 50171 Kerpen, vergabestelle@stadt-kerpen.de, +49 2237-580, DEA27. The only difference is
  that the buyer section adds the URL stadt-kerpen.de/ausschreibungen.
- The review chamber and the mediator have their own names: Vergabekammer Rheinland, Köln, VKRheinland@bezreg-koeln.nrw.de
  (ORG-0003 ReviewInfo), and "Rhein-Erft-Kreis, Der Landrat", Bergheim (ORG-0005 Mediator).
- GUID row 22905106 (ReviewOrg) holds 2 mentions. The target is 2383569, "Kolpingstadt Kerpen", identifier NULL, 97
  mentions. read_rand1 item 6 and read_rand3 item 30 scored it R for this town. One town. R.

**4 — 26492363|universitätsklinikum essen, aör — R.** The DÖE copy of item 0. The ContractFolderID and the four BT-501
values (`59f715d9…`, `4d58c6dd…`, `c0e4fdf2…`, `8cda1f3b…`) are the same. The DE1 PartyName for all four is
"Universitätsklinikum Essen, AöR", with the same Hufelandstraße 55, 45147 Essen, vergabe@uk-essen.de and phone. The roles
are ContractingParty → ORG-0001, AdditionalInformationParty → ORG-0002, AppealReceiverParty → ORG-0003 and
TenderRecipientParty → ORG-0004. The copy agrees with item 0 field for field and is decided alone with the same outcome. R.

**5 — 26496944|justizvollzugsanstalt rheinbach — R.** The DÖE copy of item 1 (ContractFolderID `613bc252…`, issued
2024-11-05). The four sections carry the same four values, are all named "Justizvollzugsanstalt Rheinbach", and give
Aachener Straße 47, 53359 Rheinbach, vergabe@jva-rheinbach.nrw.de, +492226-86261. Their roles are Buyer, AddInfo, the
AppealReceiver (ReviewOrg) and TenderRecipient. There is no difference between sections or from the TED copy. One body
printed once per role. R.

**6 — 26506733|hochschule bonn-rhein-sieg — R.** The DÖE copy of item 2. ORG-0001, -0002, -0004 and -0006 are all
"Hochschule Bonn-Rhein-Sieg", Grantham-Allee 20, 53757 Sankt Augustin, einkauf@h-brs.de, with the same values as item 2.
ORG-0004 is the AppealReceiverParty. ORG-0003 (AppealInformationParty) and ORG-0005 (MediationParty) are again "Vergabekammer
bei der Bezirksregierung Köln" under its own name. The copy agrees field for field. R.

**7 — 26522080|kolpingstadt kerpen — R.** The DÖE copy of item 3. The four "Kolpingstadt Kerpen" sections give Jahnplatz 1,
50171 Kerpen, vergabestelle@stadt-kerpen.de, with the same values. ORG-0004 is the AppealReceiverParty. Vergabekammer
Rheinland (AppealInformation) and Rhein-Erft-Kreis (Mediation) are printed under their own names. The only difference
between sections is the buyer's URL, exactly as in the TED copy. R.

**8 — 24253392|aok - die gesundheitskasse für niedersachsen — W (moderate confidence).** TED 00636996-2024 (contract award
notice of 2024-10-18, "Videokonferenzequipment"; winner Bechtle GmbH, Hannover).
- The three refused sections are all named "AOK - Die Gesundheitskasse für Niedersachsen", all at Peiner Str. 8, 30519
  Hannover, VMS-Vergabe@nds.aok.de:
  - ORG-0001 Procedure-Buyer (`617a9b4a…`): +49 511870115210, NUTS DE929;
  - ORG-0002 Lot-AddInfo (`dfe86ded…`): an exact copy of ORG-0001;
  - ORG-0003 Lot-ReviewOrg on the lot and the procedure (`26a934f6…`): **+49 22894990, NUTS DEA22**.
- `+49 22894990` is the Bundeskartellamt / Vergabekammern des Bundes switchboard in Bonn, and DEA22 is Bonn.
  - In `bodies/` the same string sits under "Vergabekammer(n) des Bundes", Bonn, in 15 other sections. They include the
    AOK NordWest notices 24293738, 24294429, 24312523, 32094941, 44539049 and 45647132, where the AOK family names the VK
    Bund as its review body.
  - The notice names no other review body. Bechtle and DÖE are the only other ORGs.
- So ORG-0003 is not the buyer copied into another role, the way AddInfo is. It was entered separately, with the federal
  chamber's phone and region, and it kept the buyer's name, address and e-mail.
- Refusing it moves a section the publisher meant for the VK Bund onto the AOK's 234 row: target 7861, "AOK - Die
  Gesundheitskasse für Niedersachsen", identifier NULL, 714 mentions. 7861 then gains a ReviewOrg mention for a tender it
  bought. That is A7's W shape, a separate review body printed under the buyer's name.
- Why the confidence is moderate:
  - by name, street and city the section is the AOK;
  - with no GUID, the 234 path would land it on 7861 anyway;
  - the cost is 1 mention per copy.
- ORG-0001 and ORG-0002 are the AOK, and folding them is right. GUID row 22752177 (the ReviewOrg) holds 2 mentions.

**9 — 24417385|rbb vermögensgesellschaft mbh & co. kg — R.** TED 00784928-2024 (contract award notice of 2024-12-19,
"ASP06 - Maler Gipser WDVS"; winner Fa. Schnabel, Mosbach).
- Three refused sections, all "RBB Vermögensgesellschaft mbH & Co. KG":
  - ORG-0001, Procedure-Buyer and Lot-AddInfo on one section (`fd2dfaa7…`);
  - ORG-0002 Lot-ReviewOrg (`b4ce4a18…`);
  - ORG-0003 Lot-Mediator (`930c6ef7…`).
- All three give Musberger Sträßle 11, 71032 Böblingen, karin.rebmann@zvrbb.de, +49 70312118116, DE112. There is no
  difference.
- No review chamber is named. The buyer is printed in every role. The zvrbb.de domain, and item 10's title "ZV RBB …",
  point to the Böblingen Zweckverband behind the KG (presumably the Restmüllheizkraftwerk; this is not needed for the
  verdict).
- GUID row 22876715 (ReviewOrg) holds 2 mentions. "RBB Vermögensgesellschaft mbH & Co. KG" is a specific company name.
  One body. R.

**10 — 24417606|rbb vermögensgesellschaft mbh & co. kg — R.** TED 00784299-2024 (contract award notice of 2024-12-19, "ZV
RBB Neubau Büro- und Sozialgebäude"; procedure `ce754e09…`, a different procedure from item 9; winner Wolf GmbH & Co.
KG, Schönaich).
- The same shape, with fresh values: ORG-0001 Buyer+AddInfo (`8da0b66a…`), ORG-0002 ReviewOrg (`bda1a182…`) and ORG-0003
  Mediator (`205fc347…`).
- All three are at Musberger Sträßle 11, Böblingen, karin.rebmann@zvrbb.de, with the same phone and NUTS. There is no
  difference.
- GUID row 22876876 holds 2 mentions.
- Two procedures of one buyer minted six GUID rows, which is the per-notice split itself. R.

**11 — 24527963|justizvollzugsanstalt rheinbach — R.** TED 00029653-2025 (contract award notice of 2025-01-15). It belongs
to the same procedure as item 1 (`613bc252…`, the CNC machining centre; winner Maschinen-Kaul, Düsseldorf).
- The JVA got three fresh values here:
  - ORG-0001 Buyer `3c804278…`;
  - ORG-0002 AddInfo `1a683d71…`;
  - ORG-0003 ReviewOrg `cf2c3511…`.
- All three are at Aachener Straße 47, 53359 Rheinbach, vergabe@jva-rheinbach.nrw.de, with the same phone and NUTS. There
  is no difference.
- Its DÖE copy, 26549982 (item 15, read for this check only), agrees.
- One procedure has now spread the one prison over seven GUID rows. GUID row 22963581 holds 2 mentions. R.

**12 — 26483158|aok - die gesundheitskasse für niedersachsen — W (moderate confidence; same group as item 8).** The DÖE
copy of item 8 (ContractFolderID `ca9aa184…`, issued 2024-10-18).
- ORG-0001 (ContractingParty) and ORG-0002 (AdditionalInformationParty) are the AOK at Peiner Str. 8, 30519 Hannover,
  +49 511870115210, DE929.
- ORG-0003 (AppealReceiverParty) has the AOK's name, street and e-mail, but phone **+49 22894990** and
  CountrySubentityCode **DEA22**, the VK Bund's values, exactly as in the TED copy.
- The copy agrees with item 8 field for field, so it carries the same finding: the ReviewOrg section is a separately
  entered review body under the buyer's name, and refusing it moves that section onto 7861.

**13 — 26537626|rbb vermögensgesellschaft mbh & co. kg — R.** The DÖE copy of item 10 (ContractFolderID `ce754e09…`).
- ORG-0001 (ContractingParty + AdditionalInformationParty), ORG-0002 (AppealReceiverParty) and ORG-0003 (MediationParty)
  are all "RBB Vermögensgesellschaft mbH & Co. KG", Musberger Sträßle 11, 71032 Böblingen, karin.rebmann@zvrbb.de, with
  the values of item 10.
- There is no difference, and the copy agrees. R.

**14 — 26537732|rbb vermögensgesellschaft mbh & co. kg — R.** The DÖE copy of item 9 (ContractFolderID `061eb830…`).
- The same three sections, with the values of item 9 (`fd2dfaa7…`, `b4ce4a18…`, `930c6ef7…`), are all at Musberger
  Sträßle 11, Böblingen, karin.rebmann@zvrbb.de, in the roles Buyer+AddInfo, AppealReceiver and Mediation.
- There is no difference, and the copy agrees. R.
