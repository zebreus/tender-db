# 530 R2b: fresh random planned GUID rows, items 30-44 (read_rand3)

## Scope

This read covers `body.r2_random[30..44]` (0-based) of `guid_twin_plan_2660.json`.
- The plan comes from Build 1b, deployed as 8e9222c. It was drawn at salt 530003, with the 60 rows R2 read excluded.
- All 15 rows are in the review_or_info stratum (items 25-49).
- Every row is planned `whole`, with `twin:true`, `twin_after:false`, `in_keep:false`, `stable:false` and country DE.
- Every row has 2 mentions, the TED copy and the DÖE copy of one notice, and sits on 1 Tender. First years are 2024 and
  2025.
- 14 rows have a found identifier-NULL target. Item 32 is a missing key, which Build 2 mints by the issue-234 path.

## Question

For each row: if all its mentions move to its target, is that R, N or W? The target is the identifier-NULL row for
(name_norm, DE), or for item 32 the row Build 2 would mint.
- **R:** the target is the same body, and the GUID was minted per notice and per section.
- **N:** I cannot tell.
- **W:** any one of these holds:
  - the target name covers more than one body;
  - the target would take another body's mentions;
  - this notice prints a different body under this name in this role;
  - the GUID is a real stable key.

## Method

**Row side.**
- For each row I fetched:
  - the GUID row (`/v1/organizations/{id}`);
  - the target;
  - **both** notice copies (`/v1/notices/{id}/content` for the TED copy and the DÖE copy).
- From each copy I took, for each ORG section:
  - the name and the BT-501 / DE1 CompanyID;
  - the street, postcode and city;
  - the phone and e-mail;
  - its OPT-300/301 roles (DE1 party references in the DÖE copy).
- **In all 15 pairs the DÖE copy carries the row's UUID on the same ORG section as the TED copy.** The role and the address
  are also the same.
- The GUID row's `identifier` equals that section's BT-501 after folding and removing dashes. So both mentions of each row
  are the one section described below.
- Each row's twin partners carry different UUIDs. Where I read a sibling notice, the same body has yet other UUIDs there.
  So no row is a stable key: each was minted per notice.

**Target side.**
- `/v1/organizations/{id}` today gives identifier NULL, provisional and the mention count.
- The plan's city screen (A6):
  - 7615 is on `city.flagged`, but only for flag b ({50667, koln}). It is also top target 0 and an office key.
  - 1175826, 1166261 and 1168328 are top targets. Each has one city, and it equals the planned city.
  - Every other found target is on no list. With `city.flagged` complete (36 = `flagged_count`), that means three
    things:
    - no planned city is absent from the target's newest-50 city set;
    - that set has fewer than 2 cities;
    - the key's planned mentions show one city.
- The plan lists target notice ids only for flagged or top targets. So for 7615 I read its third-newest mention, 47382952;
  R2 had read the two newest.
- For 1173255 I read the sibling planned notice 24334379, to see what else the target would receive.
- Where R2 already read a target, or another row of the same notice, I cite that read.

**Missing key (item 32).**
- `missing_target_samples` gives the key's planned rows, mentions and cities.
- A GET of its `sample_id` 22809680, the smallest planned row of the key, confirms the same name.

**A7.** Role sets come from each copy's own references.
- No group mixes a buyer-side role with Lot-ReviewOrg. The buyer-side roles are Procedure-Buyer, Lot-AddInfo,
  Lot-TenderReceipt and Procedure-SProvider.
- No group has a tenderer.
- 8 groups are buyer-side only. Item 44's group also includes Lot-ReviewInfo, which A7 does not bind.
- 7 groups are review-side only (ReviewInfo, ReviewOrg, Mediator).
- `roles.incoherent` and `r2_incoherent` are empty in 2660.

**F1.** None of the 15 names is on the Build-1b `GUID_TWIN_UNSAFE_NAMES`, which I read in
`crates/ingest/src/project/guid_twin.rs`. "Vergabekammer Rheinland" is one of the single-chamber names the build pins as
safe.

**GETs.**
- I made 59 public GETs. All returned 200, with no 408 and no retry.
- They are logged as `reader=rand3` in `r2b/api_requests.log`, from 20:54:39Z to 21:01:49Z.
- They were serialized with the other readers by `flock` on `r2b/.api.lock`, each at least 2.2 s after `.api.last`. The
  smallest gap between any two lines in the shared log is 2.05 s.
- Bodies are in `r2b/bodies/`.
- No `/v1/sql`, ssh, POST, git or cargo.

**Independence note.**
- Five rows are other twin rows of notices that R2 read at salt 530002: items 33, 38, 39, 40 and 41. Two more sit in
  notices that R2 read for 7615: items 42 and 43.
- The draw is still random. But for these seven, the notice-level evidence repeats R2's evidence.

## Tally: 15 R, 0 N, 0 W

- One R depends on the mint: item 32, as decided for the 52 missing keys.
- Role of the row's own section:

  | Role | Count |
  |---|---|
  | Lot-TenderReceipt | 4 |
  | Lot-ReviewInfo | 4 |
  | Lot-AddInfo | 3 |
  | Lot-ReviewOrg | 2 |
  | Lot-Mediator | 2 |

- The 15 rows go to 12 distinct keys: 11 found targets plus 1 missing key. Four rows go to 7615.

| # | row | name | own role | target (mentions today) | verdict |
|---|---|---|---|---|---|
| 30 | 23096532 | Vergabekammer Rheinland | Lot-ReviewOrg | 7615 (29,617) | R |
| 31 | 22858437 | Hamburg Messe und Congress GmbH | Lot-TenderReceipt | 1401506 (211) | R |
| 32 | 22828976 | Stadt Boizenburg/Elbe - Der Bürgermeister - c/o KSM … - Zentrale Vergabestelle | Lot-TenderReceipt | missing, minted (16 rows / 32 mentions) | R if minted |
| 33 | 22824043 | Bau- und Liegenschaftsbetrieb NRW Bielefeld | Lot-AddInfo | 1203308 (796) | R |
| 34 | 23037085 | Vergabekammer Rheinland | Lot-ReviewInfo | 7615 (29,617) | R |
| 35 | 22833833 | Universitätsklinikum Köln AöR | Lot-TenderReceipt | 1281557 (655) | R |
| 36 | 22852079 | Bau- und Liegenschaftsbetrieb NRW Düsseldorf | Lot-TenderReceipt | 1173255 (755) | R |
| 37 | 22926566 | Landesverwaltungsamt Sachsen-Anhalt | Lot-Mediator | 1168880 (3,749) | R |
| 38 | 23001168 | Vergabekammer des Landes Hessen bei dem Regierungspräsidium Darmstadt | Lot-ReviewInfo | 1175826 (6,648) | R |
| 39 | 22842359 | Norddeutscher Rundfunk | Lot-AddInfo | 1202351 (540) | R |
| 40 | 22816859 | Stadt Kyritz | Lot-AddInfo | 7175314 (137) | R |
| 41 | 22840799 | Vergabekammer Rheinland-Pfalz beim Ministerium für Wirtschaft, Verkehr, Landwirtschaft und Weinbau | Lot-Mediator | 1166261 (8,500) | R |
| 42 | 22897539 | Vergabekammer Rheinland | Lot-ReviewOrg | 7615 (29,617) | R |
| 43 | 22966356 | Vergabekammer Rheinland | Lot-ReviewInfo | 7615 (29,617) | R |
| 44 | 22914740 | Zentraldienst der Polizei des Landes Brandenburg | Lot-ReviewInfo | 1168328 (2,154) | R |

## Items

**30: 23096532 "Vergabekammer Rheinland" (review_or_info), target 7615: R.**
TED notice 24719405 (can-standard, 2025-03-31) is Kreis Mettmann's Zentrale Vergabestelle awarding VDE 0105 inspections
for Stadt Wülfrath; the DÖE copy is 26615115. The row is ORG-0003 Lot-ReviewOrg (`fea2d489…`). It is twinned with ORG-0002
Lot-ReviewInfo (`2a609e95…`). Both sections give Zeughausstraße 2-10, 50667 Köln, +49 2211473055 and
VKRheinland@bezreg-koeln.nrw.de. The other sections all have their own names: the buyer (Kreis Mettmann, Buyer and AddInfo
on one section), the tenderer HD Elektrotechnik and the DÖE SProvider. The target 7615 has identifier NULL, is provisional
and has 29,617 mentions.
- In the plan it carries flag b only. The cities are {50667, koln}: Köln's postcode typed into BT-513.
- The planned cities are {koln} over 669 rows, and `city_absent` is 0.
- Its newest mentions are all the same chamber. R2 read 47408004 and 47404257 (`r2/read_top1.md` item 0, c/o
  Bezirksregierung Köln). I read 47382952, a 2026-10-08 LVR-InfoKom PIN, whose Part-ReviewOrg is "Vergabekammer
  Rheinland", 50679 Köln, VKRheinland@bezreg-koeln.nrw.de.

"Vergabekammer Rheinland" is the one NRW chamber seated at Bezirksregierung Köln. A suffix such as "(Spruchkörper Köln)"
names a panel of it. It is not the "Vergabekammer NRW" umbrella that F1 listed, and Rheinland-Pfalz's chamber prints its
own name (item 41). One body.

**31: 22858437 "Hamburg Messe und Congress GmbH" (review_or_info), target 1401506: R.**
TED 24394386 (cn-standard, 2024-11-06, "Vermarktung Außenwerbung und Promotion-Lizenzen"); DÖE copy 26499275. The row is
ORG-0005 Lot-TenderReceipt (`a86c2f9b…`). It is twinned with ORG-0001 Procedure-Buyer (`528bf83f…`) and ORG-0002 Lot-AddInfo
(`62b3278b…`). All three give Messeplatz 1, 20357 Hamburg, +49 4035692354 and ausschreibungen@hamburg-messe.de, so the group
holds buyer-side roles only. The review body is a separate group: "Vergabekammer bei der Finanzbehörde der Freien und
Hansestadt Hamburg", printed as ReviewOrg and Mediator at Gänsemarkt 36. The target 1401506 has identifier NULL, is
provisional, has 211 mentions and is on no screen list. The name is that of one trade-fair company, a GmbH. One body.

**32: 22828976 "Stadt Boizenburg/Elbe - Der Bürgermeister - c/o KSM Kommunalservice Mecklenburg AöR - Zentrale Vergabestelle" (review_or_info), missing target: R if minted.**
TED 24356527 (cn-standard, 2024-11-26, "Erweiterungsbau Regionale Schule Rudolf Tarnow - Los 30 Rohbauarbeiten Aufzug");
DÖE copy 26514927. The row is ORG-0005 Lot-TenderReceipt (`a94a3e88…`). It is twinned with ORG-0003 Lot-AddInfo
(`e8c1c048…`). Both print the long c/o name at KSM's address: Eckdrift 93, 19061 Schwerin, vergabestelle@ks-mecklenburg.de.

Three other sections have other names, so they are other keys: they stay split under (c), and none folds into this key.
- The buyer ORG-0001 "Stadt Boizenburg/Elbe - Der Bürgermeister" (Kirchplatz 1, Boizenburg/Elbe, with the same phone and
  e-mail).
- The SProvider ORG-0002 "KSM Kommunalservice Mecklenburg AöR - Zentrale Vergabestelle".
- The MV Vergabekammern as ReviewOrg.

The group holds buyer-side roles only. No identifier-NULL row exists for the key in DE, so Build 2 mints one by the 234
path.
- The key's planned set (`missing_target_samples`) is 16 rows and 32 mentions. The planned cities are {schwerin} only, and
  flag c is not set.
- The sample row 22809680 (GET) has the same name.

The name fixes one principal: the town of Boizenburg, acting through KSM's procurement office. No other municipality can
print it. The minted row would start with only these 32 mentions of one body, so A6's homonym mode cannot arise. R, on the
condition that the row is minted, like R2's four missing-key reads.

**33: 22824043 "Bau- und Liegenschaftsbetrieb NRW Bielefeld" (review_or_info), target 1203308: R.**
TED 24349929 (cn-standard, 2024-11-19, Universität Bielefeld main building, 3rd construction phase); DÖE copy 26509277. The
row is ORG-0002 Lot-AddInfo (`22792b7d…`). It is twinned with ORG-0001 Procedure-Buyer (`9bd001b9…`) and ORG-0005
Lot-TenderReceipt (`3006aa57…`). All three give August-Bebel-Straße 91, 33602 Bielefeld and BLBVergabe@blb.nrw.de. The
Mediator "Bau- und Liegenschaftsbetrieb NRW Zentrale" (Mercedesstraße 12, Düsseldorf) and the ReviewOrg "Vergabekammer
Westfalen" have other names. This is the notice R2 read for the sibling row 22824042 (`r2/read_rand1.md` item 0, R). The
target 1203308 has identifier NULL, is provisional, has 796 mentions and carries no screen flag; R2 saw its city as
{bielefeld}. It is the Bielefeld branch of the state building agency. One body.

**34: 23037085 "Vergabekammer Rheinland" (review_or_info), target 7615: R.**
TED 24630886 (cn-standard, 2025-03-28, winter-service vehicle rental for bonnorange AöR); DÖE copy 26614407. The buyer is
"Bundesstadt Bonn - Referat Vergabedienste", which holds Buyer, AddInfo and TenderReceipt on one section. The row is ORG-0002
Lot-ReviewInfo (`88552c90…`). It is twinned with ORG-0003 Lot-ReviewOrg (`1e282a69…`). Both give Zeughausstraße 2-10,
50667 Köln, +49 221-147-3116 and vergabekammer@bezreg-koeln.nrw.de. The group's roles are ReviewInfo and ReviewOrg only.
The target is the one described in item 30. One body.

**35: 22833833 "Universitätsklinikum Köln AöR" (review_or_info), target 1281557: R.**
TED 24362430 (cn-standard, 2024-11-22, "TRIO Liquid Handler"); DÖE copy 26513238. The row is ORG-0004 Lot-TenderReceipt
(`ee2b2a72…`). It is twinned with ORG-0001 Procedure-Buyer (`9a01073f…`) and ORG-0002 Lot-AddInfo (`059e47d9…`). All three
give Kerpener Straße 62, 50937 Köln, +49 22147842121 and vergabestelle@uk-koeln.de. The ReviewOrg has another name,
"Vergabekammer Rheinland (Spruchkörper Köln) bei der Bezirksregierung Köln". So the group does not mix a buyer role with
ReviewOrg, which is the shape A7 now binds. The target 1281557 has identifier NULL, is provisional, has 655 mentions and
carries no screen flag. R2 read it through another UK Köln notice (`r2/read_rand3.md` item 39, R). One Anstalt des
öffentlichen Rechts.

**36: 22852079 "Bau- und Liegenschaftsbetrieb NRW Düsseldorf" (review_or_info), target 1173255: R.**
TED 24385695 (cn-standard, 2024-11-15, dismantling of the electrical installation at Justizzentrum Wuppertal); DÖE copy
26506999. The row is ORG-0005 Lot-TenderReceipt (`01dd3c56…`). It is twinned with ORG-0001 Procedure-Buyer (`bb4339db…`)
and ORG-0002 Lot-AddInfo (`ed3640ad…`). All three give Eduard-Schulte-Straße 1, 40225 Düsseldorf and BLBVergabe@blb.nrw.de.
Two sections have other names: "Bau- und Liegenschaftsbetrieb NRW - Justiziariat" (ReviewInfo, Mercedesstr. 12) and
"Vergabekammer Rheinland, c/o Bezirksregierung Köln·" (ReviewOrg).
- The sibling planned notice 24334379 (sports and swimming hall in Wuppertal, which I read) has the same layout. Its three
  other GUIDs sit on the same three roles at the same address. This is what else the target receives.
- The target 1173255 has identifier NULL, is provisional, has 755 mentions and carries no screen flag.

It is the Düsseldorf branch of BLB NRW. The BLB head office, also in Düsseldorf, prints its own name, "… NRW Zentrale". One
body.

**37: 22926566 "Landesverwaltungsamt Sachsen-Anhalt" (review_or_info), target 1168880: R.**
TED 24481606 (cn-standard, 2025-01-28, rental of two equipment carriers for Stadt Sangerhausen); DÖE copy 26560445. The row
is ORG-0004 Lot-Mediator (`782a9efe…`). It is twinned with ORG-0003 Lot-ReviewOrg (`8e7d71c3…`). Both give Ernst-Kamieth-Str.
2, 06112 Halle (Saale), +49 3455141536 and vergabekammer@lvwa.sachsen-anhalt.de. The buyer, Stadt Sangerhausen, is a
separate group: Buyer, AddInfo and TenderReceipt on ORG-0001, and ReviewInfo on ORG-0002.
- The group's roles are ReviewOrg and Mediator. It has no buyer-side role, so A7 does not bind.
- The contact details are the LVwA's own: its seat and its mail domain. They are not another body's, so this is not R2's
  W3 shape.
- The Sachsen-Anhalt review chambers are units of the Landesverwaltungsamt. The name printed here is the authority that
  hosts them: one Landesbehörde.
- The target may hold LVwA mentions both as a buyer and as a review body. That is one body in two roles, not two bodies.
- The target 1168880 has identifier NULL, is provisional, has 3,749 mentions and carries no screen flag. So its newest-50
  city set is empty or {halle saale}.
- The name passes the content-word guard. It is not a polity umbrella; "Land Sachsen-Anhalt" is the listed one.

Caveat: I read none of the target's own mentions. The plan gives no notice ids for an unflagged target, and the public API
has no reverse listing. The verdict rests on the notice, the name and the city screen.

**38: 23001168 "Vergabekammer des Landes Hessen bei dem Regierungspräsidium Darmstadt" (review_or_info), target 1175826: R.**
TED 24578315 (can-standard, 2025-02-04, Landkreis Gießen, foundation works for the Lollar primary school); DÖE copy 26567631.
The chamber is printed three times: ORG-0003 Lot-ReviewInfo (`98a64879…`, this row), ORG-0004 Lot-ReviewOrg and ORG-0005
Lot-Mediator. All three give Wilhelminenstraße 1-3, 64278 Darmstadt and vergabekammer@rpda.hessen.de. Landkreis Gießen
(Buyer and AddInfo) is another group, and so is the tenderer Geopier. This is the notice of R2's row 23001169
(`r2/read_rand3.md` item 37, R). The target 1175826 has identifier NULL and 6,648 mentions. It is top target 20 (101 planned
rows), and its city set {darmstadt} equals the planned set. It is the one Hessian chamber.

**39: 22842359 "Norddeutscher Rundfunk" (review_or_info), target 1202351: R.**
TED 24373729 (can-standard, 2024-11-27, maintenance of chillers); DÖE copy 26515848. The row is ORG-0002 Lot-AddInfo
(`94ad5587…`). It is twinned with ORG-0001 Procedure-Buyer (`2726cbab…`). Both give Rothenbaumchaussee 132-134, 20149
Hamburg and e-vergaben@ndr.de. The other sections have other names: the Vergabekammer bei der Finanzbehörde Hamburg
(ReviewInfo, ReviewOrg and Mediator) and two tenderers. This is the notice of R2's row 22842358 (`r2/read_rand1.md` item 1,
R). The target 1202351 has identifier NULL, is provisional, has 540 mentions and carries no screen flag. It is one public
broadcaster.

**40: 22816859 "Stadt Kyritz" (review_or_info), target 7175314: R.**
TED 24340998 (can-standard, 2024-11-05, "Los N007 Tischlerarbeiten - Kultur|Kloster|Kyritz"); DÖE copy 26498110. The row is
ORG-0002 Lot-AddInfo (`955fff2b…`). It is twinned with ORG-0001 Procedure-Buyer (`4ac42eae…`). Both give Marktplatz 1, 16866
Kyritz and vergabe@kyritz.de. The ReviewOrg is the Vergabekammer des Landes Brandenburg. This is the notice of R2's row
22816858 (`r2/read_rand1.md` item 2, R). The target 7175314 has identifier NULL, is provisional, has 137 mentions and carries
no screen flag. One town.

**41: 22840799 "Vergabekammer Rheinland-Pfalz beim Ministerium für Wirtschaft, Verkehr, Landwirtschaft und Weinbau" (review_or_info), target 1166261: R.**
TED 24371427 (cn-standard, 2024-11-12, building cleaning at the RPTU's Landau campus); DÖE copy 26503163. The chamber is
printed three times: ORG-0003 ReviewInfo, ORG-0004 ReviewOrg and ORG-0005 Lot-Mediator (`72e18c44…`, this row). All three
give Stiftsstraße 9, 55116 Mainz and vergabekammer.rlp@mwvlw.rlp.de. RPTU prints itself three times (Buyer, AddInfo and
TenderReceipt), which is its own group. This is the notice of R2's row 22840798 (`r2/read_rand3.md` item 38, R). The target
1166261 has identifier NULL and 8,500 mentions. It is top target 12 (130 planned rows), and its city set {mainz} equals the
planned set. It is the one chamber of Rheinland-Pfalz.

**42: 22897539 "Vergabekammer Rheinland" (review_or_info), target 7615: R.**
TED 24443775 (cn-standard, 2024-12-03, Bundesstadt Bonn, unified-communications system for the WCCB); DÖE copy 26520669.
The row is ORG-0004 Lot-ReviewOrg (`8df7d451…`). It is twinned with ORG-0003 Lot-ReviewInfo (`17ed73ab…`). Both give
Zeughausstraße 2-10, 50667 Köln and vergabekammer@bezreg-koeln.nrw.de. Bonn's Referat Vergabedienste (Buyer, AddInfo and
TenderReceipt) is its own group. R2 read this notice as one of 7615's planned notices (`r2/read_top1.md` item 0). The target
is the one described in item 30. One body.

**43: 22966356 "Vergabekammer Rheinland" (review_or_info), target 7615: R.**
TED 24531553 (can-standard, 2025-01-16, "Bonns Fünfte - Malerarbeiten Innen"); DÖE copy 26551363. The row is ORG-0003
Lot-ReviewInfo (`0b9ad13d…`). It is twinned with ORG-0004 Lot-ReviewOrg (`16bd2059…`), at the same Köln address and e-mail.
The tenderer Malerwerkstätte Wallbrück and Bonn's Referat Vergabedienste (Buyer and AddInfo) have other names. R2 also read
this notice for 7615 (`r2/read_top1.md` item 0). The target is the one described in item 30. One body.

**44: 22914740 "Zentraldienst der Polizei des Landes Brandenburg" (review_or_info), target 1168328: R.**
TED 24466923 (can-standard, 2025-01-02, framework contract for destroying data carriers, Brandenburg Nord lot); DÖE copy
26541433. The ZDPol is printed four times under four UUIDs: ORG-0001 Procedure-Buyer, ORG-0002 Procedure-SProvider, ORG-0003
Lot-AddInfo and ORG-0004 Lot-ReviewInfo (`2cdbcbef…`, this row). All four give Am Baruther Tor 20, 15806 Zossen and
vergabestelle.zdpol@polizei.brandenburg.de. The ReviewOrg and Mediator are the Vergabekammer des Landes Brandenburg in
Potsdam, which has another name.
- A buyer that prints itself as ReviewInfo is giving the contact for review deadlines. That is not the Lot-ReviewOrg mix
  A7 binds, and the slot prints nothing of another body.
- The target 1168328 has identifier NULL and 2,154 mentions. It is top target 5 (218 planned rows), and its city set
  {zossen} equals the planned set.
- R2 read the target as R (`r2/read_top1.md` item 5).

It is one police service authority.
