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

Total: 32 notice files, 914 KB.

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

## `eforms/` — eForms (TED), 10 files, 179 KB

All but the last two from TED daily package **`daily-202600136`** (`20260717_136`,
published 2026-07-17, 3722 notices). That day spans three SDK customizations
(`eforms-sdk-1.12` ×448, `1.13` ×2202, `1.14` ×1027), so the set below is a
live multi-version sample, not a single-SDK snapshot. The two issue-195
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
| `cn-selc-tp-00157944-2024.xml` | 24 303 | 16 | 1.8 | BE | **Issue 195: selection criteria under TenderingProcess.** From daily `20240315_2024054`. Each lot repeats the identical `efac:SelectionCriteria` block under both TenderingTerms (the SDK mount) and TenderingProcess (unenumerated). Exercises the TenderingTerms→TenderingProcess `ALIASES` graft. Non-EN (NLD): the quirk class is Belgian-platform specific. |

### On BT-195 naming

Withheld fields do **not** appear as the literal string `BT-195` in TED XML — a
grep for `BT-195` over the whole 2026 daily returns zero files. The mechanism is
`efac:FieldsPrivacy` / `efbc:FieldIdentifierCode` with `listName=
"non-publication-identifier"`, whose values are short codes (`win-ten-val`,
`not-val`, …). Across that day's daily the most frequent withheld fields were
`win-ten-val` (66), `rec-sub-typ` (51), `rec-sub-cou` (50), `not-val` (24).

---

## `eforms-chain/` — one complete real procedure, 4 files, 77 KB

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

## `r209/` — TED_EXPORT R2.0.9, 6 files, 59 KB

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

## `r208/` — TED_EXPORT R2.0.8 (and R2.0.7), 3 files, 65 KB

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

## `text/` — text era (1993–2010), 4 files, 410 KB

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

## `doe/` — oeffentlichevergabe.de (DÖE), 3 files, 26 KB

From `samples/oeffentlichevergabe/`. The DÖE feed carries **three distinct
encodings** and all three are represented, because issue 12 needs a checklist
per channel.

| File | Bytes | Customization | Root | Source | Why |
|---|---|---|---|---|---|
| `eforms-de-2.1-can-15063f7d-…-01.xml` | 16 445 | `eforms-de-2.1` | `<ContractAwardNotice>` | `2026-07-18.eforms.zip` | Above-threshold German profile. Declares `cbc:ProfileID` = `eforms-sdk-1.13`, which is exactly the DE→EU base disambiguation docs/research/eforms-de-profile.md describes (eForms-DE 2.1 maps to *two* EU bases). A result-side subtype (29) was chosen deliberately: DEX statistics fields are only permitted on subtypes 29–35. |
| `sdk-0.1-numeric-cn-25599482-1.xml` | 3 003 | `eforms-sdk-0.1` | `<ns9:ContractNotice>` | `2026-07-18.eforms.zip` | **Numeric channel.** Legacy below-threshold encoding: numeric file id, fully **prefix-mangled namespaces** (`ns2`…`ns9`, with the *default* namespace bound to the eForms extension basic-components URI rather than UBL) — the hardest namespace case in the corpus. Carries an **empty `<ns3:ContractFolderID/>`**, which is why below-threshold DÖE notices can never match a TED twin and become single-notice Tenders. |
| `sdk-0.1-uuid-can-427d4645-…-1.xml` | 6 380 | `eforms-sdk-0.1` | `<can:ContractAwardNotice>` | `2023-01.eforms.zip` | **UUID channel** — same `eforms-sdk-0.1` customization, but UUID-named and a *third* prefix scheme (`can:`). Sourced from 2023-01 deliberately: by the 2026-07-18 export the uuid channel has died out (that day is 346 numeric + 221 `eforms-de-2.1`, zero uuid `sdk-0.1`), so this encoding only exists in the older months. 2023-01 holds 1493 uuid-named vs 12807 numeric-named files. |

---

## `doe-ted-pair/` — the verified cross-source pair, 2 files, 79 KB

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
