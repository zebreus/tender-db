# Parser test fixtures

Real notice files, committed verbatim. Every file here is **byte-identical to
the original** as published — nothing reformatted, re-indented, re-encoded or
truncated. Parser tests may assert on exact bytes and offsets.

Sources are the sample archives on the scraping VPS
(`root@zebreus.click:/opt/tender-db/samples/`). Each entry below records the
source package, the publication id, why the instance was picked, and what it
exercises.

Layout is `<profile>/<notice-type>-<publication-id>.xml`, one profile directory
per mapping profile in docs/architecture.md ("Notice identity and profiles").

Total: 86 fixture files, 2.3 MB (every file under this directory except this README).

## Selection policy

Instances were picked small-to-medium *within* their type — around the 20–25th
size percentile of that type in the source package — so the corpus stays lean
without any file being an unrepresentative degenerate minimum. Where a type had
an interesting edge case (withheld fields, framework agreements, a corrigendum
chain), the edge case drove the pick instead of size.

**Language:** EN was preferred for readability wherever the package offered it.
Two files are non-EN because the source package contained no EN instance of that
form at all (noted per entry). The schema is multilingual; language is
incidental to what these fixtures test.

---

## `eforms/` — eForms (TED and DÖE quirk members), 39 files, 903 KB

All but the last six from TED daily package **`daily-202600136`** (`20260717_136`,
published 2026-07-17, 3722 notices). That day spans three SDK customizations
(`eforms-sdk-1.12` ×448, `1.13` ×2202, `1.14` ×1027), so the set below is a
live multi-version sample, not a single-SDK snapshot. The six issue-195/180-adjacent
fixtures come from earlier dailies (noted per entry) because the quirk they
exercise is specific to those vintages.

| File | Bytes | Subtype | SDK | Country | Why / what it exercises |
|---|---|---|---|---|---|
| `cn-16-00494343-2026.xml` | 19 607 | 16 — contract notice | 1.13 | DE | The single most common notice type on TED (1458/3722 that day). Baseline CN mapping: 1 lot, 1 organization. |
| `can-29-00495054-2026.xml` | 18 842 | 29 — contract award notice | 1.13 | NO | Second most common type (1286/3722). Award-side baseline with a rich party set — **9 organization mentions**, good for `organization_mentions` and role mapping. Non-EU (Norway/EEA) buyer country. |
| `change-16-00494185-2026.xml` | 10 353 | 16 + `efac:Changes` | 1.13 | ES | Corrigendum/change notice. **Also the namespace edge case**: root element is serialised as `<ns8:ContractNotice xmlns:ns8="…ContractNotice-2">` — a *prefixed* root, where most TED files use a default namespace. Issue 03 requires namespace-URI + local-name matching; a prefix-literal parser fails this file and passes the others. |
| `pin-4-00496860-2026.xml` | 8 573 | 4 — prior information notice | 1.14 | LV | PIN (planning). **Has no `cbc:ContractFolderID`** — a planning notice published before a procedure identity exists. Exercises the projection path where procedure-level linkage is absent. |
| `veat-25-00496202-2026.xml` | 11 722 | 25 — voluntary ex-ante transparency | 1.14 | IT | VEAT / direct award. Carries `ProcessJustification` (direct-award justification), a field class absent from ordinary CNs. |
| `brin-x01-00497689-2026.xml` | 3 613 | X01 | 1.14 | DE | Business Registration Information Notice — **a different root element entirely** (`<BusinessRegistrationInformationNotice>` in the `…/p27/eforms-business-registration-information-notice/1` namespace), not a UBL ContractNotice/ContractAwardNotice. Verifies root-element dispatch rather than assuming UBL. Zero lots, carries a company register id (`HRA 133853`). Smallest file in the corpus and the only X01 in that day. |
| `can-withheld-29-00495618-2026.xml` | 14 970 | 29 | 1.13 | NL | **Withheld fields (BT-195).** Contains `efac:FieldsPrivacy` blocks with `non-publication-identifier` codes `awa-cri-nam`, `awa-cri-num`, `awa-cri-typ`, `rec-sub-cou`, `rec-sub-typ` and reason codes `oth-int`, `chan-need`. Drives the withheld-field satellite table from issue 03. |
| `can-fa-29-00495185-2026.xml` | 11 036 | 29 | 1.13 | NL | **Framework agreement CAN** — `ContractingSystemTypeCode` = `fa-wo-rc` (framework without reopening competition). Exercises the FA/DPS lot-relabelling concern called out in docs/architecture.md ("FA/DPS rounds relabel them"). |
| `can-cvd-lot-00054478-2025.xml` | 57 870 | 29 | 1.10 | HR | **Issue 195: lot-mounted CVD statistics.** From daily `20250127_2025018`. The Clean Vehicles Directive block (`AssetCategoryCode`, `StrategicProcurementStatistics`) published forward-looking under the *Lot's* TenderingTerms extension, where sdk-1.10 anchors `efac:ProcurementDetails` only at LotResult. Exercises the LotResult→Lot `ALIASES` graft. |
| `cn-fma-root-00660539-2023.xml` | 35 816 | 16 | 1.7 | DE | **Issue 195: root-level framework maximum.** From daily `20231030_2023209`. `efbc:FrameworkMaximumAmount` published directly under the root EformsExtension — the eForms-DE tailoring emitted onto a plain EU customization (the vendored eforms-de-1.x inventory declares this exact path as DE1-FrameworkMaximumAmount). Exercises the gap-filled root claim as `UBL-FrameworkMaximumAmount`. |
| `cn-renewals-00660164-2023.xml` | 23 064 | 17 | 1.7 | DE | **Issue 195: lot renewals indicator.** From monthly `2023-10`. `cbc:RenewalsIndicator` beside BT-58 in the lot's ContractExtension — eForms-DE tailoring on a plain EU customization (declared verbatim by eforms-de-1.x); claimed as `UBL-RenewalsIndicator` behind the predicate-free-construct guard. |
| `pin-part-rl-00679774-2023.xml` | 7 669 | 4 | 1.7 | LV | **Issue 195: address description on a Part.** From monthly `2023-11`. `RealizedLocation/Address/cbc:Description` on a PIN whose lots are Parts — outside the procedure→Lot alias, and aliases do not compose, so `UBL-AddressDescription` is also written at Lot level for the Lot→Part alias to mirror. |
| `cn-shortlist-tp-00047617-2024.xml` | 20 323 | 23 | 1.7 | FR | **Issue 195: design-contest shortlist under TenderingProcess.** From monthly `2024-01`. The publisher merges the pre-selected participants (BT-47, declared under TenderingTerms) into the TenderingProcess shortlist block beside the BT-51/BT-50 quantities. Exercises the EconomicOperatorShortList `ALIASES` graft. |
| `cn-selc-tp-00157944-2024.xml` | 24 303 | 16 | 1.8 | BE | **Issue 195: selection criteria under TenderingProcess.** From daily `20240315_2024054`. Each lot repeats the identical `efac:SelectionCriteria` block under both TenderingTerms (the SDK mount) and TenderingProcess (unenumerated). Exercises the TenderingTerms→TenderingProcess `ALIASES` graft. Non-EN (NLD): the quirk class is Belgian-platform specific. |
| `doe-sdk10-selc-appealterms.xml` | 14,968 | 4 | 1.0 | DE (DÖE) | **Issue 195: selection criteria inside AppealTerms.** From DÖE monthly `2022-12`. A 2022 tool writes the `efac:SelectionCriteria` extension under the procedure TenderingTerms' `cac:AppealTerms` — one mount deeper than any SDK declares. Exercises the AppealTerms `ALIASES` graft. |
| `doe-sdk01-subcontract.xml` | 17,207 | 29 | 0.1 | DE (DÖE) | **Issue 195: TenderResult subcontracting value.** From DÖE monthly `2023-02`. Plain-UBL pre-release result model: `cac:TenderResult/cac:SubcontractTerms` with `cbc:Amount` (BT-553 shape) beside the conditions code the empirical inventory already knew. Pins `SDK01-TenderResult-SubcontractTerms-Amount`. |
| `doe-sdk01-subcontract-rate.xml` | 60,053 | 29 | 0.1 | DE (DÖE) | **Issue 195: subcontracted share.** Same block, `cbc:Rate` variant (the BT-555 percentage shape). Pins `SDK01-TenderResult-SubcontractTerms-Rate`. |
| `doe-sdk01-ple-mounts.xml` | 21,805 | 29 | 0.1 | DE (DÖE) | **Issue 195: PartyLegalEntity under the lot appeal parties.** From DÖE monthly `2023-08`. `cac:PartyLegalEntity/cbc:CompanyID` under the lot's AppealReceiverParty AND MediationParty — two mounts the empirical inventory lacked (the member advanced from one to the other across re-parses, the issue-87 rewrite in action). |
| `doe-sdk01-ple-addinfo.xml` | 30,461 | 29 | 0.1 | DE (DÖE) | **Issue 195: PartyLegalEntity under AdditionalInformationParty.** From DÖE monthly `2023-09`. The third missing CompanyID mount. |
| `pin-pat-supplies-00660476-2023.xml` | 8,271 | 4 | 1.8 | DE | **Issue 195: unlisted ProcurementTypeCode listName.** From monthly `2023-10`. The eSender writes `listName="supplies"` carrying a contract-nature value — no SDK predicate matches; claimed as published under `UBL-ProcurementAdditionalTypeCode` (policy reversal of the issue-144 negative control). |
| `can-pat-social-00250633-2024.xml` | 19,706 | 29 | 1.10 | FR | **Issue 195: BT-775's SDK-1.0 shape.** From monthly `2024-04`. `listName="social-procurement"` (the field moved into the StrategicProcurement extension after 1.0) beside DECLARED accessibility/environmental-impact codes — pins that BT-754/BT-774 keep their exact ids while the dead shape claims relaxed. |
| `can-fp-root-profea-00462901-2024.xml` | 14,182 | 29 | 1.9 | PL | **Issue 195: FieldsPrivacy at the root.** From monthly `2024-08`. The pro-fea (BT-88) withheld block hoisted to the root EformsExtension; grafted from the procedure TenderingProcess anchor — the published FieldIdentifierCode predicate keeps BT-195(BT-88) exact. |
| `cn-fp-root-awacrityp-00004191-2025.xml` | 13,713 | 16 | 1.9 | PL | **Issue 195: FieldsPrivacy at the root, award-criterion family.** From monthly `2025-01`. awa-cri-typ (BT-539) blocks at the root; pins the second graft source (lot SubordinateAwardingCriterion anchor). |
| `cn-fa-expected-00586487-2024.xml` | 73,183 | 16 | 1.10 | FR | **Issue 195: ExpectedOperatorQuantity.** From monthly `2024-09`. The framework's expected participant count beside the declared BT-113 maximum — no SDK minor declares the element; claimed as `UBL-ExpectedOperatorQuantity`, BT-113 keeps its id. |
| `cn-spp-nested-00232905-2025.xml` | 19,054 | 16 | 1.12 | SK | **Issue 195: a provider's provider.** From monthly `2025-04`. ServiceProviderParty/Party/ServiceProviderParty (self-referential, the outer level an empty shell); the declared level grafts onto the nested mount. |
| `can-sp-awcrit-00344162-2025.xml` | 12,840 | 29 | 1.12 | IT | **Issue 195: CVD flag on an award criterion + zoneless OPT-999.** From monthly `2025-05`. efac:StrategicProcurement (ApplicableLegalBasis) inside the AwardingCriterion extension, grafted from the lot TenderingTerms anchor; also pins the dummy TenderResult AwardDate reading zoneless as UTC. |
| `brin-eu-00568126-2023.xml` | 3,082 | X01 | 1.8 | — | **Issue 195: EU-scheme BRIN on 1.8.** From monthly `2023-09` (a sandbox-grade notice TED published for real). PartyLegalEntity with schemeName='EU' — SDK ≤1.8 declares only the 'national' BT-500 branch, and the AdditionalDocumentReference block without its cbc:ID (OPP-124 enters later). Both later-SDK shapes claimed gap-filled. |
| `cn-bt707-16-00042304-2024.xml` | 30,874 | 16 | 1.6 | RO | **Issue 195: BT-707 published before its SDK.** From monthly `2024-01`. The lot CallForTendersDocumentReference/DocumentTypeCode enters the vendored line at 1.7.0; claimed gap-filled with the declared shape. |
| `can-inline-org-00530983-2024.xml` | 18,960 | 29 | 1.7 | NO | **Issue 195: the whole org inlined + a real TenderResult.** From monthly `2024-09`. Contracting party inlined in full under cac:Party (Company graft, the DÖE issue-78 shape on TED), a cac:Person contact, and UBL 2.3's forced TenderResult filled in for real (count, low/high amounts, zoneless StartDate, winner reference) — all claimed as published. |
| `can-subdesc-00570953-2025.xml` | 21,034 | 32 | 1.12 | IT | **Issue 195: SubTypeDescription.** From monthly `2025-09`. Free text beside the notice-subtype code, declared by no minor or dialect; claimed as `UBL-SubTypeDescription`. |
| `doe-sdk10-mediation.xml` | 18,602 | 4 | 1.0 | DE (DÖE) | **Issue 195: inline mediation body.** From DÖE monthly `2023-05` (uuid channel). cac:MediationParty inlined at BOTH procedure and lot level; the Company graft gains both targets. |
| `doe-sdk10-dup-org.xml` | 38,106 | 29 | 1.0 | DE (DÖE) | **Issue 201: org republished per UBO.** From DÖE monthly `2022-12`. The serializer emits the whole Organization block once per ultimate beneficial owner — four ORG-0010 copies differing only in their UBO reference. Pins section-id MERGING. |
| `can-dup-org-00305298-2024.xml` | 30,551 | 29 | 1.11 | — | **Issue 201: org registered twice.** From monthly `2024-05`. The same ORG-0000 published full then sparse; merges into one section. |
| `cn-cer-typo-00081074-2024.xml` | 9,855 | 16 | 1.6 | — | **Issue 201: misspelled BT-736 list.** From monthly `2024-02`. `listName="reserved-executionn"` (double n) relaxed across four candidates and died ambiguous-field; leaf-predicated carve-out preserves the typo. |
| `cn-selc-param-00228777-2024.xml` | 36,019 | 16 | 1.7 | — | **Issue 201: attrless selection ParameterCode.** From monthly `2024-04`. `per-exa` with no listName — the selection-side twin of the cause-J award carve-out, same predicate-free-branch guard. |
| `cn-pt-national-00245088-2024.xml` | 26,832 | 16 | 1.10 | DE | **Issue 201: national buyer-legal-type list.** From monthly `2024-04`. `listName="stift-oer-kommun"` on a plain EU customization — BT-11/BT-740 discriminate by list, so it relaxed to both. |

### On BT-195 naming

Withheld fields do **not** appear as the literal string `BT-195` in TED XML — a
grep for `BT-195` over the whole 2026 daily returns zero files. The mechanism is
`efac:FieldsPrivacy` / `efbc:FieldIdentifierCode` with `listName=
"non-publication-identifier"`, whose values are short codes (`win-ten-val`,
`not-val`, …). Across that day's daily the most frequent withheld fields were
`win-ten-val` (66), `rec-sub-typ` (51), `rec-sub-cou` (50), `not-val` (24).

---

## `eforms-chain/` — one complete real procedure, 4 files, 76 KB

Procedure **`32c34097-960e-4d02-b04d-3ceac32cf020`** (Malta), harvested from
`samples/chains/`. All four notices share that `cbc:ContractFolderID`, and the
change notices form an unbroken linked list via
`efbc:ChangedNoticeIdentifier` → the *previous* notice's `notice-id` + version.
Files are numbered `1-`…`4-` so the intended replay order is obvious.

| # | File | Bytes | Notice id (`schemeName="notice-id"`) | Ver | Points at |
|---|---|---|---|---|---|
| 1 | `1-cn-16-831374-2025.xml` | 18 729 | `a6e3de9d-b506-45df-869b-ea7b3749cfc9` | 01 | — (original CN, sub 16, issued 2025-12-11) |
| 2 | `2-change-16-6281-2026.xml` | 19 123 | `49d1143d-349d-4113-b0b2-6cf28a943850` | 02 | `a6e3de9d-…-01` |
| 3 | `3-change-16-18902-2026.xml` | 19 123 | `3ed4e7f6-d11d-48d0-9a91-cd2585e11b17` | 03 | `49d1143d-…-02` |
| 4 | `4-can-29-380868-2026.xml` | 20 510 | `f5bec521-611c-4d54-9646-59d643a00627` | 01 | `3ed4e7f6-…-03` (award, sub 29, issued 2026-05-20) |

This is the CN → corrigendum → corrigendum → CAN lifecycle over ~6 months. It is
the intended end-to-end test for the version-row pattern (ADR-0001): four
notices must collapse into **one** Tender with four `tender_versions` rows in
`published_at` order, with the CAN carrying the result.

Change reasons present: `update-add`. All four are `eforms-sdk-1.13`.

**Serialisation quirk worth knowing:** these files carry redundant `xmlns=""`
resets on prefixed elements, e.g.
`<cbc:ID xmlns="" schemeName="notice-id">`. The `cbc:` prefix still binds
correctly, so a namespace-URI matcher is unaffected — but a parser that tracks
the default namespace naively can trip here. Retained deliberately.

---

## `eforms-prev-ref/` — one real procedure under TWO procedure keys, 2 files, 39 KB

The counter-example to `eforms-chain/`: two notices of **one** procedure whose
`cbc:ContractFolderID` (BT-04) values **differ**, so the keyed grouping cannot see
that they belong together. Harvested from the production archive
(`ted/monthly/2024-10.tar` and `2025-01.tar`), byte-identical, sha256 verified
against the archived members.

| # | File | Bytes | Publication | Profile | BT-04 | Sub |
|---|---|---|---|---|---|---|
| 1 | `1-cn-16-615938-2024.xml` | 20 625 | `00615938-2024` | `eforms-sdk-1.7` | (its own) | 16 |
| 2 | `2-can-29-566-2025.xml` | 18 935 | `00000566-2025` | `eforms-sdk-1.13` | `00a143ab-…` | 29 |

The award carries `OPP-090-Procedure = 615938-2024` — a
`cac:NoticeDocumentReference` naming file 1's publication — which is the only
published statement that the two are one procedure. ADR-0011 makes that reference
an identity edge; issue 236 has the corpus measurements behind it (27–39 % of EU
eForms award Tenders are single-notice islands for exactly this reason).

Note the two vocabularies for one thing: the XML element is
`cac:NoticeDocumentReference`, while `PreviousNoticeReference` is the SDK **node**
id that tender-db uses as the section id. Grepping the payload for the latter finds
nothing.

Also worth knowing: they are 3 months apart and from **different SDK versions**,
which is normal for a real procedure and makes the pair a fair test of
cross-version chaining rather than a same-package coincidence.

---

## `r209/` — TED_EXPORT R2.0.9 (and R2.0.8 members it parses), 11 files, 121 KB

From TED daily package **`daily-201900001`** (`20190102_001`, published
2019-01-02, 1529 notices). That package is itself a useful artefact: it is
**mixed-schema** — 1370 notices at `R2.0.9.S03.E01` and 154 still at
`R2.0.8.S04.E01` (the defence forms, which stayed on the older schema). Issue 09
covers R2.0.9; the defence file below is included here rather than in `r208/`
because that is where it is actually found in the wild.

| File | Bytes | Form | Schema version | Lang | Country | Why |
|---|---|---|---|---|---|---|
| `f02-000245-2019.xml` | 10 122 | F02 — contract notice | R2.0.9.S03.E01 | EN | MT | Highest-priority form in issue 09 (448 in this package). |
| `f03-000988-2019.xml` | 9 045 | F03 — contract award | R2.0.9.S03.E01 | EN | UK | Most common form in this package (556). Award side. |
| `f14-001311-2019.xml` | 8 447 | F14 — corrigendum | R2.0.9.S03.E01 | EN | UK | **Contains a `REF_NOTICE` back-reference** — the OJS-number chain edge that feeds the projection's union-find grouping, and the source of issue 09's "F14 typed diffs as version events". The only fixture here with a legacy chain edge. |
| `f20-000591-2019.xml` | 10 630 | F20 — modification notice | R2.0.9.S03.E01 | EN | UK | Fourth of the four forms issue 09 names first. |
| `f05-001315-2019.xml` | 9 246 | F05 — utilities contract notice | R2.0.9.S03.E01 | **NL** | BE | Utilities sector (Directive 2014/25). **No EN F05 exists in this package** — all 40 instances are non-EN; this Belgian/Dutch one is the smallest. Language is incidental to the structural mapping. |
| `f18-defence-001420-2019.xml` | 10 913 | 18 (defence) | **R2.0.8.S04.E01** | **RO** | RO | **Defence form**, `DIRECTIVE VALUE="2009/81/EC"`. Note it is R2.0.8 inside a 2019 package — defence notices did not migrate to R2.0.9. **No EN defence notice exists in this package** (all 2009/81 instances are RO/IT/ES/DE); this is the smallest. |

---

- `f02-co-original-160877-2015.xml` (21,117 B): **issue 201** — Belgian-style bilingual F02 with TWO CATEGORY="ORIGINAL" sections (DE primary, FR co-original carrying a third ORGANISATION). Pins co-original section ADOPTION: the extra org is opened and fully emitted, while a relabelled EN TRANSLATION with the same extra section still rejects.
- `f19-concession-award-criteria-281627-2012.xml` (12,346 B): **issue 194 residue** — F19 sub-contract concession (defence, R2.0.8.S02.E01, EN/BE) whose `AWARD_CRITERIA_DETAIL` carries the award-criteria sentence as BARE TEXT where every other form nests children there. Pins the TextGroup rule (both shapes consumed). One of exactly 2 such members in 30 years of corpus.
- `f13-prize-winner-362996-2018.xml` (7,875 B): **issue 259** — F13 design-contest result (PT, from monthly `2018-08`) whose prize block nests `<ADDRESS_WINNER>` inside `<WINNER>`. Both are `Rule::Org`, so ONE company opens TWO Organization sections: the outer empty and referenced as the winner, the inner holding `OFFICIALNAME`. The only `WINNER`/`ADDRESS_WINNER` pair in the corpus — every other award fixture uses `CONTRACTOR` > `ADDRESS_CONTRACTOR`, where the wrapper is a transparent container and nothing nests, which is why the defect was invisible for as long as it was.

## `r208/` — TED_EXPORT R2.0.8 (and R2.0.7), 5 files, 281 KB

Mostly from TED daily package **`daily-201400001`** (published 2014-01-01, 1139
notices, uniformly `R2.0.8.S02.E01`), plus one R2.0.7 file — issue 10 scopes
R2.0.7 into this profile. Note both R2.0.7 and R2.0.8 use **numeric** `FORM`
attributes (`FORM="2"`), not the `F02` spelling R2.0.9 uses — a real dispatch
difference between the eras.

| File | Bytes | Form | Schema version | Lang | Country | Why |
|---|---|---|---|---|---|---|
| `f02-000333-2014.xml` | 13 050 | `FORM="2"` — contract notice (F02-equivalent) | R2.0.8.S02.E01 | EN | UK | The R2.0.8 contract-notice shape, `DIRECTIVE VALUE="2004/18/EC"`. Second most common form in the package (322). Diff this against `r209/f02-000245-2019.xml` to see the era delta issue 10 must absorb. |
| `oth-not-000030-2014.xml` | 41 204 | `OTH_NOT` | R2.0.8.S02.E01 | DA | PT | Prose corrigendum — issue 10's "OTH_NOT prose corrigenda as version events **without** typed diffs". Carries a `REF_NOTICE` edge. **This is the largest legacy fixture and that is inherent to the type**: OTH_NOT bodies are free prose and all 150 instances in this package run 41–55 KB; this is the smallest one. All OTH_NOT in this package are DA. |
| `f02-r207-001441-2011.xml` | 11 982 | `FORM="2"` — contract notice | **R2.0.7.S03.E01** | EN | UK | **The R2.0.7 case.** From `daily-201100001` (`20110104_001`, published 2011-01-04), which is uniformly `R2.0.7.S03.E01` — 533 `FORM="2"` and 853 `FORM="3"`. Issue 10 notes the R2.0.7 XSD hunt was inconclusive and the delta must be derived from real files: this is that file, and the same-form/same-language/same-country pairing with `f02-000333-2014.xml` makes the R2.0.7→R2.0.8 delta a direct diff. |

---

## `text/` — text era (1993–2010), 6 files, 416 KB

Text-era dailies are **not** one-file-per-notice. Each language ships as a
single concatenated stream of plain-text records, each record starting at a
column-0 line like `1.0/003065` and carrying ~20–30 coded header fields
(`TI:`, `PD:`, `ND:`, `TD:`, `NC:`, `CY:`, `RN:`, `TX:` …) — exactly the
header-only shape issue 11 describes.

| File | Bytes | Source | Why |
|---|---|---|---|
| `1993-daily-en-19930102.txt` | 404 767 | `daily-199300001` → `EN_19930102_1993001_ISO_ORG` | **A complete, unmodified daily package member** — the whole English delivery for 1993-01-02, all records, including the `T E D   D A I L Y - D E L I V E R Y` banner. This is the fixture for the record **splitter**: everything else in this corpus is a single notice, so nothing else proves the splitter works at real scale. It is the only text-era daily small enough to commit whole (2005 EN is 6.2 MB, 2008 EN is 11 MB). Declared ISO-8859-1 (`_ISO_ORG`); the content of this particular day happens to be pure ASCII, so it does **not** by itself exercise the Latin-1 path in `encoding_rs` — see TODO below. |
| `2000-pin-130-2000.txt` | 3 408 | `daily-200000001` → `EN_20000104_001_ISO_ORG.ZIP`, record `ND: 130-2000` | Single record, extracted byte-exactly. Prior-information notice, ES. **The Latin-1 fixture**: declared ISO-8859-1 with 30 real high bytes (á/é/í/ó in the Spanish `OT:` body — "Bilbao Ría 2000", "José María Olábarri"), exercising the `encoding_rs` decode path the 1993 file cannot (see the closed TODO below). Also the 2000 vintage: `OT`/`CO`/`RC`/`RG` exist, `IA`/`MA` do not yet. |
| `2005-can-154-2005.txt` | 4 039 | `daily-200500001` → `EN_20050101_2005001_UTF8_ORG`, record `ND: 154-2005` | Single record, **extracted byte-exactly** (see note). Contract award, UK. **Has an `RN: 108785-2003` back-reference** — the text-era chain edge issue 11 needs ("RN back-references feed chains", and "XML-era chains … terminate at real text-era records"). Also shows the multi-value `PC:`/`PN:` continuation-line format (3 CPV codes across indented lines), which is a real parsing hazard. |
| `2008-cn-723-2008.txt` | 5 976 | `daily-200800001` → `EN_20080103_2008001_UTF8_ORG`, record `ND: 723-2008` | Single record, extracted byte-exactly. Contract notice, FR. Later text-era vintage — shows `TD: 3 - Contract notice` where 1993 spells the same concept `TD: 3 - Invitation to tender`, i.e. the coded vocabularies drift within the text era and the checklist must be vintage-aware. |

**On the two extracted records.** The 2005 and 2008 entries are single complete
notice records sliced out of their multi-notice daily at exact record
boundaries. Each is a whole notice with every byte as published — this is
*record selection*, not truncation, and it matches the one-notice-per-file
convention of every other fixture here. The full dailies were too large to
commit (6.2 MB / 11 MB); they remain on the VPS at the paths above, and the
1993 file covers the whole-daily case.

---

## `doe/` — oeffentlichevergabe.de (DÖE), 7 files, 65 KB

From `samples/oeffentlichevergabe/`. The DÖE feed carries **three distinct
encodings** and all three are represented, because issue 12 needs a checklist
per channel.

| File | Bytes | Customization | Root | Source | Why |
|---|---|---|---|---|---|
| `eforms-de-2.1-can-15063f7d-…-01.xml` | 16 445 | `eforms-de-2.1` | `<ContractAwardNotice>` | `2026-07-18.eforms.zip` | Above-threshold German profile. Declares `cbc:ProfileID` = `eforms-sdk-1.13`, which is exactly the DE→EU base disambiguation docs/research/eforms-de-profile.md describes (eForms-DE 2.1 maps to *two* EU bases). A result-side subtype (29) was chosen deliberately: DEX statistics fields are only permitted on subtypes 29–35. |
| `sdk-0.1-numeric-cn-25599482-1.xml` | 3 003 | `eforms-sdk-0.1` | `<ns9:ContractNotice>` | `2026-07-18.eforms.zip` | **Numeric channel.** Legacy below-threshold encoding: numeric file id, fully **prefix-mangled namespaces** (`ns2`…`ns9`, with the *default* namespace bound to the eForms extension basic-components URI rather than UBL) — the hardest namespace case in the corpus. Carries an **empty `<ns3:ContractFolderID/>`**, which is why below-threshold DÖE notices can never match a TED twin and become single-notice Tenders. |
| `sdk-0.1-uuid-can-427d4645-…-1.xml` | 6 380 | `eforms-sdk-0.1` | `<can:ContractAwardNotice>` | `2023-01.eforms.zip` | **UUID channel** — same `eforms-sdk-0.1` customization, but UUID-named and a *third* prefix scheme (`can:`). Sourced from 2023-01 deliberately: by the 2026-07-18 export the uuid channel has died out (that day is 346 numeric + 221 `eforms-de-2.1`, zero uuid `sdk-0.1`), so this encoding only exists in the older months. 2023-01 holds 1493 uuid-named vs 12807 numeric-named files. |
| `sdk-0.1-can-awarddate-only-19191760-1.xml` | 4 960 | `eforms-sdk-0.1` | `<ns9:ContractAwardNotice>` | `2023-01.zip` | **The dialect's normal award notice** (issue 257). Its whole result block is `<ns5:TenderResult><ns3:AwardDate/><ns3:AwardTime/></>` — no `TenderResultCode`, no `WinningParty`, no value. Not an outlier: every award-type notice in the month carries a `TenderResult` (2 895 of 2 895 — the serializer, not richness) and only 13.5 % of them name a winner; by 2024-06 it is 1.9 %. Committed because the projection used to read this silence as `clos-nw` ("closed, no award") on a notice that states the day of the award. |

---

## `fts/` — UK Find a Tender Service (FTS), 2 files, 47 KB

**Not verbatim pages** — the one exception to the rule at the top of this file,
by construction: a real FTS page is 100 releases (≈1 MB), so these are the
recorded pages of 3 September 2026 (`.scratch/tender-db/342-fts/recorded/`,
issue 342 unit 1) with the `releases` array cut down to a handful. The package
header is kept whole, each kept release is its own object unchanged (Python
`json.dump(indent=4)` re-emits the API's own 4-space layout and key order), and
the page-level fields the fetcher must DROP from a member (`uri`,
`publishedDate`, `links`) are present so the tests can prove they are dropped.

| File | Bytes | Releases | Shape | Why |
|---|---|---|---|---|
| `pages/2026-09-03-p001.json` | 29 146 | 5 | page 1 of the day, real `links.next` (opaque cursor) | The non-final page: `083674-2026` UK10 contractAmendment, `083664-2026` UK1 and `083655-2026` UK2 planning, `083608-2026` UK7 award+contract, `083662-2026` UK4 tender (the smallest tender-tagged release of the page) — every notice family on one page. |
| `pages/2026-09-03-p002.json` | 18 666 | 3 | page 5 of the day: the final page, NO `links` | `083257-2026` a CELEX (PCR 2015) award+contract with no `noticeType`, `083256-2026` UK12 tenderCancellation, `083253-2026` UK5 award. |

`tests/fetch.rs` serves the two as a paged window (rewriting `links.next` to the
fixture server) and asserts the assembled zip holds one member per release id.
The single-release member packages for the profile/parse tests (unit 2 commits b
and c) are cut from the same recorded pages.

Contains public sector information licensed under the Open Government Licence
v3.0.

---

## `doe-ted-pair/` — the verified cross-source pair, 2 files, 78 KB

The **same procedure published on both sources**, the concrete case behind
ADR-0003 (cross-source merge) and issue 12's acceptance criterion. Verified
hands-on in docs/research/german-portals.md §"Cross-referencing TED".

| File | Bytes | Source | Identity |
|---|---|---|---|
| `ted-cn-00373130-2026.xml` | 40 518 | TED | publication `00373130-2026`, `eforms-sdk-1.13` |
| `doe-cn-ebb72363-832d-4cea-8db6-04999414ea8c-01.xml` | 39 092 | DÖE | notice-id `ebb72363-832d-4cea-8db6-04999414ea8c`, `eforms-de-2.1` |

Both are contract notices (subtype 16), Germany, **2 lots, 7 organizations
each**, and both carry the identical
`cbc:ContractFolderID` = **`1af86e3c-411f-4c2e-aacc-ecac61717472`**. The TED
side additionally carries the DÖE `notice-id` verbatim, giving the two
exact-equality merge levels the ADR relies on (notice level and procedure
level) with no heuristics.

Expected test outcome: these two files ingest to **one** Tender carrying the
TED publication identity plus the DÖE national codes.

---

## TODO / gaps

Things a fixture is wanted for but which do not exist in the sampled data:

- **DEX / VergStatVO statistics fields** (`defext:` extension, `BT-001-DEX`,
  `BT-002-DEX`). No fixture. Zero files in the 2026-07-18 DÖE export contain
  any `defext:` element — the fields are defined in SDK-DE from 1.14.0 but
  VergStatVO go-live is announced for H2 2026, so no real instance exists yet.
  Re-check a DÖE export after go-live and add a subtype-29–35 notice.
- **DPS contract award notice.** Only the framework-agreement (`fa-wo-rc`)
  case is committed. DPS instances do exist in the 2026 daily — e.g.
  `00495281_2026.xml` (12 993 B, subtype 29, `dps-nlist`) — and can be added
  cheaply if DPS lot handling turns out to diverge from FA.
- **EN defence notice** and **EN F05**: neither exists in the 2019-01-02
  package (see `r209/`). If an EN instance matters for readability, sweep
  another 2019 daily.
- **2008 `META` format.** The 2008 package also ships a `_META_ORG` variant
  (`en_20080103_001_meta_org.zip`) which is *not* the plain-text era format at
  all — it is a markup format with `<part>`/`<doc>`/`<codifdata>` elements and
  structured coded fields. Issue 11 decided: it stays a walker-level skip
  (`text-era-meta-variant`) — it renders the very same notices as the tagged
  text and would double every ingested record; the raw archive keeps it for a
  possible later profile. No fixture committed.
- **R2.0.7 award side.** The R2.0.7 contract *notice* is covered
  (`r208/f02-r207-001441-2011.xml`); the award form (`FORM="3"`, 853 instances
  in `daily-201100001`) is not. Add one if the R2.0.7 delta turns out to differ
  on the award side.
- ~~**Latin-1 high bytes in the text era.**~~ Closed by
  `text/2000-pin-130-2000.txt` (issue 11): a real EN record whose declared-ISO
  bytes carry Spanish accents, extracted from `daily-200000001` — no non-EN
  member needed.

## Reproducing / extending

Source packages live on the VPS under `/opt/tender-db/samples/`, with the
era-ladder dailies pre-extracted under `samples/x/<package>/` and the eForms
procedure chains under `samples/chains/<contract-folder-id>/`. The DÖE monthly
and completed-day exports are the zips under
`samples/oeffentlichevergabe/`, with the 2026-07-18 day pre-extracted to
`samples/oeffentlichevergabe/x-ef/`.
