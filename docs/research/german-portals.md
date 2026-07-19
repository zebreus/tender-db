# German procurement portals as additional Sources

Research, 2026-07-19. Goal: pick 1–2 German portals besides TED, chosen to
prove the tender-db model is not TED-exclusive. Hard criteria: no sign-in,
long history. Soft criteria: machine-readable access, ongoing updates
(continuous change detection), reuse-permitting terms (data is republished
through our AGPL API).

Method: primary German-language sources; every load-bearing claim verified
hands-on where feasible (marked **verified**). Bulk samples live on the VPS
under `/opt/tender-db/samples/oeffentlichevergabe/` (~370 MB).

---

## 1. Landscape: how German procurement publication works

Germany has no single historical procurement gazette. Publication duties split
along the EU threshold:

- **Above threshold** (GWB/VgV/SektVO/KonzVgV): EU-wide publication in the
  OJEU/TED is mandatory. Since 2023-10-25 notices must be submitted as eForms.
- **Below threshold** (UVgO/VOB-A §12): publication on "Internetportalen" or
  official gazettes of the Bund/Land/municipality — historically scattered
  across dozens of portals with no central sink.

Where notices actually **originate**: on ~30+ *Vergabeplattformen* — the
systems buyers run their procedures on. The big operators:

- **cosinex** ecosystem: DTVP "Deutsches Vergabeportal" (dtvp.de),
  vergabe.metropoleruhr.de, and the white-labeled *Vergabemarktplätze* of
  several Länder (NRW, Brandenburg, …).
- **Healy Hudson / e-Vergabe des Bundes** (evergabe-online.de, run by the
  Beschaffungsamt des BMI for federal buyers).
- **subreport ELViS**, **evergabe.de**, **Vergabe24** (staatsanzeiger-owned),
  **Administration Intelligence (AI)** platforms used by several Länder
  (e.g. evergabe.sachsen.de), Berlin's "Meine Vergabeplattform", Bayern's
  Staatsanzeiger eServices, etc.

Everything else is a **mirror**. Two federal aggregation layers exist:

### service.bund.de (Bundesverwaltungsamt) — the old aggregator

The federal administration's service portal carries an "Ausschreibungen"
section fed by imports from the platforms above (detail links point back to
the origin platform). It is a *current-notices* board: notices are removed
once expired — there is deliberately **no archive** (sub-survey finding, and
consistent with its self-description as a search service for running
procedures). Access is account-free HTML plus RSS
(**verified**: `https://www.service.bund.de/Content/Globals/Functions/RSSFeed/RSSGenerator_Ausschreibungen.xml`
serves account-free RSS 2.0, `ttl` 60 min, operated by BVA/GSB CMS; items
link to `service.bund.de/IMPORTE/Ausschreibungen/<platform>/<year>/<month>/<id>.html`).

### Datenservice Öffentlicher Einkauf (DÖE) — the new central sink (2023+)

Built by the Beschaffungsamt des BMI (bescha.bund.de) under the IT-Planungsrat
"XEinkauf" standardisation programme. Five components
([bescha.bund.de](https://www.bescha.bund.de/DE/ElektronischerEinkauf/Datenservice_Oeffentlicher_Einkauf/Datenservice-Oeffentlicher-Einkauf_node.html)):

1. **Vermittlungsdienst** — platforms submit notices in **eForms-DE**; it
   validates them and routes them;
2. **eSender-Hub** — converts above-threshold notices from eForms-DE to
   eForms-EU and forwards them to TED (Germany's central TED eSender);
3. **Bekanntmachungsservice** — the public publication platform,
   **oeffentlichevergabe.de**: all routed notices (above *and* below
   threshold) are published and archived here;
4. Self-Service-Portal and Redaktionssystem (internal/onboarding).

Consequences for source selection:

- German **above-threshold** notices reach TED *through* the DÖE — the
  Bekanntmachungsservice holds the same notices in original eForms-DE.
- German **below-threshold** notices (UVgO) have their first-ever central,
  machine-readable, archived home at oeffentlichevergabe.de. This data
  never reaches TED.
- The commercial/state platforms are upstream origins but expose no open bulk
  access; service.bund.de mirrors without history. The DÖE is where the data
  converges *and* stays retrievable.

### State and commercial portals (sub-survey verdict)

A parallel survey of cosinex/DTVP, Vergabe.NRW, Bayern, Berlin,
evergabe.sachsen.de, Brandenburg, evergabe-online.de, subreport, evergabe.de,
Vergabe24, ausschreibungen-deutschland.de, ibau/bi-medien concluded: **all
fail at least one hard criterion** — expired notices disappear or move behind
accounts (cosinex-family, evergabe-online, most Länder portals), access is
paywalled/account-gated (commercial aggregators like ibau,
ausschreibungen-deutschland.de), and none offers a documented open bulk API.
They also increasingly *feed* the DÖE anyway (moderate confidence; connection
rollout per platform is ongoing). None beats the federal options.

---

## 2. Primary candidate: oeffentlichevergabe.de (Bekanntmachungsservice)

### Access method — **verified**

One documented, account-free bulk endpoint (OpenData API):

```
GET https://oeffentlichevergabe.de/api/notice-exports?pubMonth=YYYY-MM&format=eforms.zip
GET https://oeffentlichevergabe.de/api/notice-exports?pubDay=YYYY-MM-DD&format=ocds.zip
```

- OpenAPI spec: `https://oeffentlichevergabe.de/documentation/api/opendata`
  (JSON) / `…/api.yaml/opendata` (YAML); Swagger UI at
  `https://oeffentlichevergabe.de/documentation/swagger-ui/opendata/index.html`.
- No authentication of any kind (**verified**: anonymous curl works; the
  OpenAPI spec declares no security schemes). No documented rate limits.
- `pubMonth` and `pubDay` are mutually exclusive; times are Europe/Berlin;
  "all published notices processed before midnight on the previous day are
  available" — i.e. a completed day becomes fetchable the next day
  (**verified**: `pubDay=<yesterday>` works, `pubDay=<today>` is rejected 400).
- Formats (`format` param or `Accept` header): `eforms.zip` (original
  eForms-DE XML, one file per notice version, named `<uuid>-<version>.xml`),
  `ocds.zip` (OCDS 1.1 JSON releases, prefix `ocds-mnwr74`), `csv.zip`
  (19 relational tables: notice, lot, organisation, tender, contract, … keyed
  by `noticeIdentifier`+`noticeVersion`; mapping doc:
  `…/documentation/api/opendata/Documentation Bekanntmachungsservice CSV Format.ods`).
  All formats are conversions of the eForms original. (**verified** all three.)
- An undocumented search API backs the SPA UI (`/api/notices…`), not needed
  for ingestion.
- `robots.txt`: none — request returns the 404 page (**verified**).

### History depth — **verified**

The API rejects anything before **2022-12** ( `pubMonth=2022-11` → HTTP 400,
per spec "When pubMonth prior to 2022-12 is requested"). Actual downloads:

| Export | Notice versions | ZIP size |
|---|---|---|
| 2022-12 | 121,947 | 168 MB |
| 2023-01 | 14,300 | 24 MB |
| 2023-06 | 19,411 | 33 MB |
| 2023-11 | 20,030 | 65 MB |
| 2026-06 | 23,398 | 94 MB |
| 2026-07-18 (one day) | 567 | — |

So: **complete retrievable history back to December 2022** (~3.6 years at
time of writing), fetchable month-by-month with ~40 requests. The 2022-12
bucket is anomalous (121k notices, almost all in the legacy `eforms-sdk-0.1`
encoding) — it looks like an initial bulk load of notices existing at service
launch, not one month of organic volume (moderate confidence; provenance not
documented).

Caveat against the "long history" criterion: no German portal offers open
history beyond this. Deeper German above-threshold history (back to 2011+)
exists only via TED itself, which we already ingest; pre-2023 below-threshold
notices are effectively lost to fragmentation. DÖE's archive is the longest
open machine-readable German record there is, and it grows monotonically.

### Volume, coverage, cadence

- ~20–25k notice versions/month currently (~750/day), rising as more
  platforms connect.
- Content mix (2026-06 sample, ~400 notices read in full): ~50%
  `eforms-de-2.1`, ~8% `eforms-de-2.0`, ~42% `eforms-sdk-0.1`-encoded
  **below-threshold** notices (`RegulatoryDomain de-uvgo`) (**verified** by
  inspection; the below-threshold sample was a UVgO contract notice of
  Stadtverwaltung Genthin). Above-threshold notices carry standard EU subtype
  codes (16, 29, 17, 30, …); national subtypes E2/E3/E4 also occur.
- Corrections/changes appear as **new versions of the same notice UUID**
  (`<uuid>-02.xml`, `efac:Change` blocks) inside the day/month they were
  published — change detection = fetch each completed day once.

### Cross-referencing TED — **verified**

The identifiers survive the eForms-DE→eForms-EU conversion. Hands-on test: a
2026-06 above-threshold notice `ebb72363-832d-4cea-8db6-04999414ea8c-01` with
`ContractFolderID 1af86e3c-411f-4c2e-aacc-ecac61717472` is on TED as
publication `373130-2026` with the **identical** `notice-identifier` and
`procedure-identifier` (queried via the TED v3 search API). This is exactly
the "strong explicit cross-reference" ADR-0003 requires: cross-source merge on
exact UUID equality, no heuristics.

### Terms of use — **verified**

The site's *Open-Data-Richtlinie*
(`https://oeffentlichevergabe.de/ui/de/Open-Data-Richtlinie`; text extracted
from the app bundle since the page is a JS SPA) states:

> „Für die Nutzung der als Open Data bereitgestellten Daten gilt die Creative
> Commons **CC Zero**-Lizenz" — with the explicit purpose that notices "von
> allen Interessierten genutzt, wiederverwendet und weiterverbreitet werden
> können".

Every OCDS package also machine-declares
`"license": "https://opendefinition.org/licenses/cc-zero/"` (**verified**).
CC0 is maximally compatible with republishing through our AGPL-licensed API.
Contact for feedback: support@datenservice-oeffentlicher-einkauf.de.

---

## 3. Secondary candidate: service.bund.de

- **Operator**: Bundesverwaltungsamt; aggregates/mirrors notices from federal,
  state and municipal *Vergabestellen* and platforms.
- **Access**: account-free HTML search + RSS 2.0 feeds (**verified**); stable
  detail URLs under `/IMPORTE/Ausschreibungen/…`; no structured-data API — the
  payload is HTML with a link back to the origin platform.
- **History**: fails hard — notices are delisted after expiry; no archive
  (sub-survey finding, high confidence).
- **Terms**: standard federal imprint terms; no open-data grant comparable to
  DÖE's CC0 (sub-survey finding: weak/unclear reuse terms).
- **Value if added**: purely as a *live* discovery feed for notices from
  platforms not yet connected to the DÖE. Metadata quality (HTML) is far below
  eForms. Not recommended for v1.

---

## 4. eForms-DE: the German national eForms profile

This matters beyond Germany — it shows what a national eForms profile does to
the base standard.

### Governance and artefacts

- **Standard**: "eForms-DE", maintained by **KoSIT** (Koordinierungsstelle für
  IT-Standards, Bremen) under the XEinkauf umbrella; legally binding since
  2023-10-25. Spec: https://xeinkauf.de/eforms-de/ (current version 2.1.0;
  spec ZIP via projekte.kosit.org Maven registry).
- **SDK-DE**: https://gitlab.opencode.de/OC000008125155/SDK-eforms-de — a
  **patched fork of the EU eForms SDK**, same layout (`fields/`,
  `notice-types/`, `codelists/`, `schematrons/`, `schemas/`, `translations/`),
  tagged in lockstep with EU SDK versions (1.12.x–1.14.x; SDK-DE 1.14.4
  implements eForms-DE 2.1.0 on EU SDK 1.14.2). (**verified** via GitLab API.)
- Supporting repos: eforms-de-codelist and eforms-de-schematron on
  projekte.kosit.org.
- Version mapping (from the Bekanntmachungsservice API docs): eForms-DE 1.0→EU
  1.5, 1.1→1.7, 1.2→1.10, 2.0→1.12, 2.1→1.13/1.14. A notice declares its
  profile in `cbc:CustomizationID` (`eforms-de-2.1` vs EU `eforms-sdk-1.x`).

### How the profile differs from base eForms (verified from SDK-DE 1.14.4)

1. **Restriction**: national business rules `BR-DE-*` (Schematron) tighten
   optionality and value combinations (e.g. mandatory buyer postcodes for
   statistics, forbidden code combinations under AVV rules).
2. **National notice types**: subtypes **E1** (voluntary pre-market
   consultation), **E2** (voluntary prior-information, below threshold),
   **E3** (contract notice below threshold, `cn-standard`), **E4** (award
   notice below threshold, `can-standard`) — added alongside the EU's 4–40 and
   T01/T02. These are UVgO/below-threshold forms with no EU equivalent and
   never forwarded to TED.
3. **Extension fields**: national Business Terms in a DEX namespace —
   currently **BT-001-DEX** (Berichtseinheits-ID) and **BT-002-DEX**
   (sustainability-criteria phase), carried in a national UBL extension
   (`/schema/common/german-eforms-extension.xsd`), injected into notice types
   25–40 and E4, with a national codelist
   (`sustainability-criteria-setting.gc`). Purpose: automated statistics
   reporting under the VergStatVO (go-live announced for H2 2026).
4. **fields.json is extended in place** — the SDK-DE `fields.json` is a
   superset/patch of the EU one.

### Live-data reality (verified against exports)

- The Bekanntmachungsservice archive holds a **mix of profiles at all times**:
  eForms-DE 2.x, older eForms-DE, plain eForms-EU versions, and — even in
  2026 — a large share (~40%) of below-threshold notices encoded with
  `CustomizationID eforms-sdk-0.1` + `RegulatoryDomain de-uvgo`, an early
  reduced encoding predating the current E-form types. Any importer must
  accept several customization IDs concurrently, not just the newest.

### Impact on our hand-designed schema

- The EU-SDK `fields.json` completeness checklist (ADR-0002) generalises: run
  the same mechanical walk **per profile SDK** (EU SDK for TED, SDK-DE for
  DÖE). Because SDK-DE is a fork with identical structure, no new tooling
  shape is needed.
- National BTs (BT-xxx-DEX) fit as **source-profile satellite data**: a small
  table (or columns) for German statistics fields, keyed off the Notice — they
  must not leak into the core Tender/Lot/Bid shape, since only one profile
  carries them. Expect every national profile (AT, FR, …) to add its own small
  DEX-like set; the schema needs one general pattern for "national extension
  BTs", not per-country hacks.
- National notice subtypes (E1–E4) need the `notice subtype` domain to be a
  per-source-profile codelist, not a closed EU enum. Same for
  `RegulatoryDomain` values like `de-uvgo`.
- Below-threshold notices are sparser (no EU-mandated fields) — the canonical
  layer must tolerate legitimately-absent fields without treating them as
  quality failures.

---

## 5. Comparison

| | oeffentlichevergabe.de (DÖE) | service.bund.de | cosinex/DTVP & Länder portals | subreport / evergabe.de / Vergabe24 | ibau, ausschreibungen-deutschland.de |
|---|---|---|---|---|---|
| Role | central official sink + archive | federal mirror board | origin platforms | origin platforms (commercial) | commercial aggregators |
| No sign-in | **yes** (verified) | yes | search yes, docs/details often gated | partially | no (paywall) |
| History | **2022-12 → today, complete via API** (verified) | none (expired = gone) | none/short | none/short | proprietary |
| Machine access | **bulk API: eForms-DE XML, OCDS JSON, CSV** (verified) | RSS + HTML | HTML (some JSON, undocumented) | HTML | n/a |
| Update cadence | daily exports, day-complete next morning | RSS ~hourly | live | live | n/a |
| Reuse terms | **CC0** (verified) | unclear/weak | unclear | proprietary | proprietary |
| Below-threshold | yes (~40–45% of volume) | partially | yes | yes | yes |
| TED cross-ref | exact UUIDs (verified) | none (HTML) | varies | varies | n/a |

## 6. Recommendation

**Pick one portal: oeffentlichevergabe.de (Bekanntmachungsservice / DÖE) as
Source #2.** It is the only German portal that satisfies every criterion:
account-free (verified), the longest open machine-readable history any German
source offers (complete since 2022-12, verified by download), a documented
bulk API in three formats, daily update granularity that fits our
fetcher/processor split (fetch yesterday's ZIP, store raw, process), CC0
reuse terms, and — decisive for the actual goal — it exercises the model in
exactly the ways TED doesn't: a national eForms profile with extension BTs,
national notice types, below-threshold data, legacy encodings, and verified
exact-UUID overlap with TED for the ADR-0003 merge path.

A second German portal adds little: everything else is either upstream of the
DÖE or fails the history criterion. If a second Source is still wanted for
model-proving, take service.bund.de as a deliberately *ugly* one (HTML/RSS,
live-only) — it stresses the "Source without history/structured data" corner
of the model. **Fallback candidate** if the DÖE API were ever restricted:
service.bund.de (live-only, HTML quality), with TED covering history above
threshold.

## 7. Reuse / legal (recommended portal)

- Data: **CC0** per the Open-Data-Richtlinie and per the `license` field
  embedded in every OCDS release package (both verified). No attribution
  required; republication through our API is unambiguously permitted.
- No robots.txt, no ToS clause against automated access found; the OpenData
  API exists precisely for bulk retrieval. Undocumented: rate limits — be a
  polite client (sequential monthly backfill ≈ 44 requests / ~3 GB, then one
  request/day).
- Liability note in the Richtlinie: content responsibility stays with the
  publishing authority — worth mirroring in our own API terms.

## 8. Implications for tender-db

A DÖE importer must handle things the TED importer never sees:

1. **Multiple concurrent eForms profiles per Source**: `CustomizationID` ∈
   {eforms-de-2.1, 2.0, 1.x, eforms-sdk-1.x, eforms-sdk-0.1}. The strict
   parser (ADR-0004) needs per-profile mappings; unknown customization IDs
   quarantine cleanly.
2. **A legacy encoding at scale**: ~40% of current volume is below-threshold
   in the old `eforms-sdk-0.1` shape (different element ordering, fewer
   fields, `RegulatoryDomain de-uvgo`). This is not a fringe case.
3. **National extension BTs** (BT-xxx-DEX) → a general satellite-table pattern
   for profile-specific fields.
4. **National notice subtypes** (E1–E4) and national codelists → open,
   per-profile code domains.
5. **Bulk-ZIP fetching, no per-notice endpoint**: the fetcher stores whole
   day/month ZIPs as the raw payload versions; change detection is "fetch each
   completed day once" — no cursor, no diff API. Corrigenda arrive as new
   `<uuid>-<n>` versions, mapping directly onto our append-only Notice layer.
6. **Cross-source merge becomes real**: German above-threshold procedures
   arrive from both TED (eForms-EU) and DÖE (eForms-DE) with identical
   notice/procedure UUIDs (verified) — the first live exercise of ADR-0003,
   including field-conflict precedence between the converted (TED) and
   original (DÖE) representation of the same notice.
7. **Legitimate sparseness**: below-threshold notices lack many EU-mandatory
   fields; "missing" must be distinguishable from "unmapped".

## 9. Open questions

Needs more research:
- What exactly the 2022-12 bucket (121k notices) contains — initial migration
  scope and provenance (moderate confidence it's a launch backfill).
- The formal spec for the legacy `eforms-sdk-0.1` below-threshold encoding
  (predates SDK-DE 1.0; may only be pinned down empirically from samples).
- Below-threshold *coverage*: which platforms/Länder are connected to the
  Vermittlungsdienst as of 2026, i.e. what share of German below-threshold
  procurement the DÖE actually sees.
- Whether undocumented rate limits exist on `/api/notice-exports` (ask
  support@datenservice-oeffentlicher-einkauf.de before the full backfill).
- Whether monthly export content is immutable after month end (late
  processing could append) — affects raw-payload versioning of re-fetches.

Needs a user decision:
- One portal (oeffentlichevergabe.de) vs adding service.bund.de as a second,
  deliberately unstructured Source to stress the model further. Recommendation
  here: start with one; service.bund.de can be added later without schema
  impact.
