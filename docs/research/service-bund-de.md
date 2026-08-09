# service.bund.de as Source #3 — deep-dive (deferred decision C22)

Research, 2026-08-09. This is the deferred research behind SUMMARY.md §2.C
**C22** ("second German source: service.bund.de as deliberately-ugly stress
Source — later; no schema impact"). Goal: a full source evaluation —
access channels, content scope, **overlap with our two existing sources**
(TED bulk, DÖE/oeffentlichevergabe.de daily exports), format, history, legal,
and a verdict including whether C22's "no schema impact" assumption survives.

Method: hands-on verification via curl/WebFetch on 2026-08-09 (a Sunday;
weekday-cadence claims are extrapolated where noted). Every load-bearing
claim fetched and quoted (marked **verified**). Sample HTML/JSON lives only
in the session scratchpad (not durable); all URLs are reproducible.
One access limitation: **web.archive.org is blocked by the egress proxy**
("Blocked by egress policy"), so Wayback-based checks of historical delisting
behaviour were not possible — affected claims are marked.

---

## 1. What service.bund.de is (and why it exists)

Operator: **Bundesverwaltungsamt (BVA)**, Referat SQ 5, hosted by ITZBund,
built on the Government Site Builder CMS (GSB 7.x on CoreMedia)
(**verified**, Impressum + RSS `generator` tag). It is the federal
administration's general service portal; its "Ausschreibungen" section is a
*current-notices board*, not a procurement platform: buyers publish
elsewhere, service.bund.de mirrors.

Three distinct legal anchors make its coverage broad — this is why it is not
just a courtesy site:

1. **Kabinettsbeschluss of 2005-03-09**: all federal authorities are obliged
   to publish suitable tenders and job offers on service.bund.de
   (**verified**, quoted on `/Content/DE/Service/Redaktionssystem/…`).
2. **§ 28 Abs. 1 S. 3 UVgO**: „Auftragsbekanntmachungen auf Internetseiten
   des Auftraggebers oder auf Internetportalen müssen zentral über die
   Suchfunktion des Internetportals **www.bund.de** ermittelt werden können."
   (**verified** verbatim via lxgesetze.de/uvgo/28.)
3. **§ 12 VOB/A 2019** carries the equivalent below-threshold requirement for
   construction (secondary sources; not independently quoted — medium
   confidence on exact wording, high on substance).

So every German below-threshold notice published on any portal is *required*
to be findable through this site's search. Mechanically, per the site's own
"Informationen zu Ausschreibungen" page (**verified**): content arrives

- by **automated import from 70 cooperating Vergabeplattformen**, through an
  exhaustive ("abschließend") list of 16 interface partners: Administration
  Intelligence AG, aumass, Beschaffungsamt des BMI, B_I MEDIEN, cosinex,
  Lubey AG, DTVP, eVergabe.de GmbH, HAD/Healy Hudson, RIB Deutschland,
  Staatsanzeiger eServices, Stadt Mülheim a.d. Ruhr, subreport, Vergabe24,
  VIZSON;
- below-threshold feeds partly via the **XVergabe-Proxy des Beschaffungsamts
  des BMI** (the same BeschA that runs the DÖE);
- or **manually**, by registered "Lokalredakteure" (the `editor` platform key
  in URLs) using the site's own Redaktionssystem — free of charge.

In a 2-day RSS window, 38 distinct platform keys appeared (**verified**,
feed analysis): `evergabede`, `subreport`, `eVergabe` (e-Vergabe des Bundes),
`ai-ag-prod`, `dtvp`, `vmp-*` (cosinex Vergabemarktplätze), `abc`
(Vergabe24), `aumass`, `bi-medien-prod`, `healyhudson`, `vizson`, `editor`, …

## 2. Access channels — **verified**

There is **no API, no bulk export, no sitemap, and no open-data listing** for
tender data:

- `robots.txt` (**verified**): `Disallow: /Content/DE/Ausschreibungen/Suche/`
  (the search UI is disallowed for all agents!), `Crawl-delay: 30`. Detail
  pages under `/IMPORTE/` are *not* disallowed. No `Sitemap:` line;
  `/sitemap.xml` → 404.
- The site's own **Open Data page** (**verified**) offers exactly two
  datasets on GovData — the federal *Anschriftenverzeichnis* and
  *Abkürzungsverzeichnis* — nothing about tenders. A govdata.de search for
  the tender data returns „keine Treffer" (**verified**).

What does exist, all account-free:

### 2.1 RSS 2.0 feeds (the only machine-readable channel)

- **Global tender feed**
  `…/Content/Globals/Functions/RSSFeed/RSSGenerator_Ausschreibungen.xml`
  (**verified**: 200, 354 KB, RSS 2.0, `ttl` 60): fixed **500-item** window.
  On 2026-08-09 the 500 items spanned Aug 6 17:39 → Aug 8 18:20 (~49 h) —
  i.e. **~250 new tenders/day**; even daily polling loses nothing.
- **Global awards feed** `…/RSSGenerator_Vergebene_Auftraege.xml`
  (**verified**: 500 items spanning ~4.2 days → **~120 awards/day**).
- **Per-search RSS**: any search-form query can be subscribed by appending
  `&jobsrss=true` (**verified**:
  `Formular.html?nn=4641482&resultsPerPage=100&sortOrder=dateOfIssue_dt+desc&jobsrss=true`
  → RSS with 100 items; `resultsPerPage` caps at 100 — a value of 200
  degraded to a 15-item default). This gives filtered feeds (per category,
  Bundesland, procedure type, keyword).
- Item payload: title, detail-page GUID/link, pubDate, and an HTML
  `description` with exactly Erfüllungsort, Vergabestelle, Angebotsfrist,
  Veröffentlichungsende. Nothing else.
- **No conditional GET**: `Cache-Control: no-cache`, no ETag, `Last-Modified`
  = request time (**verified** via response headers). Poll and dedupe by GUID.
- The site itself calls RSS/newsletter „sogenannte 'pro bono'-Dienste oder
  sekundäre Publikationskanäle" with no availability promise (**verified**,
  RSS info page) — the primary channel is the HTML portal.

### 2.2 HTML search + detail pages

- Search: `/Content/DE/Ausschreibungen/Suche/Formular.html` with GET
  parameters (**verified** working): `type` (0 = Ausschreibungen,
  1 = Vergebene Aufträge, 2 = Vergebene Aufträge evergabe-online.de),
  `resultsPerPage` (≤100), `templateQueryString` (keyword), sort via
  `gts=4642258_list=dateOfIssue_dt+asc|desc`, pagination via
  `gtp=4642258_list=<n>` (**verified**: page 2 of 141 via plain GET, 100
  items). Facet filters: Leistungen/Erzeugnisse (19 categories),
  AllocationType (VOB, VgV, VOL, UVgO, VOF, SektVO, VSVgV, KonzVgV,
  Haushaltsrecht, sonstige), AllocationMode (26 procedure kinds),
  Bundesland, postcode+radius. **No date-range filter** — only date *sort*.
  The result header carries an exact total (`<h1>14057 Ausschreibungen`).
- Detail pages: stable, flat URLs
  `/IMPORTE/Ausschreibungen/<platformKey>/<year>/<month>/<originId>.html`
  (`eVergabe` omits year/month; `editor` inserts the org slug). IDs are the
  **origin platform's IDs** — numeric (dtvp `337818`), alphanumeric
  (subreport `E81132322`, aumass `AV284A44`), composite hex (ai-ag-prod
  `54321-Tender-19fb…`), or platform GUIDs that *look* like UUIDs
  (healyhudson) but are **not** DÖE notice UUIDs (3/3 probed against
  `oeffentlichevergabe.de/api/notices/{uuid}` → 404, **verified**).
- Sessions: the CMS injects `;jsessionid=…` into links and sets
  load-balancer cookies; URLs work fine with the jsessionid stripped
  (**verified**).
- Scraping robustness: markup is server-rendered GSB with stable class names
  (`result-list`, `aria-labelledby` structure); technically easy. But note
  the robots.txt posture (search disallowed, crawl-delay 30) and one
  connection reset observed during rapid sequential fetches (single
  occurrence; not conclusively rate limiting). No documented rate limits.

### 2.3 Newsletter

Email subscription per search exists; not machine-useful. (Not tested.)

## 3. Content scope and volume

- **Stock** (2026-08-09, **verified** result headers): **14,057** running
  Ausschreibungen (type 0), **18,571** vergebene Aufträge (type 1), **0**
  under type 2 („Keine Treffer" — the dedicated evergabe-online award
  category is currently empty).
- **Inflow** (RSS window math, **verified** feeds): ~250 tenders/day + ~120
  awards/day ⇒ roughly **8–11 k notices/month**. For comparison: DÖE
  publishes ~20–25 k notice versions/month (July research) — service.bund.de
  is materially *smaller* than the DÖE feed we already ingest.
- **Scope** (**verified** on sampled pages): both **EU-wide and national**
  procedures (page field „Ausschreibungsweite": EU-Ausschreibung / Nationale
  Ausschreibung / Internationale Ausschreibung), across VOB, VgV, VOL, UVgO,
  VOF, SektVO, VSVgV, KonzVgV, Haushaltsrecht. Below-threshold content is a
  large share, consistent with the § 28 UVgO mandate.
- **Awards**: type 1 includes EU award mirrors (thin) *and* below-threshold
  **ex-post transparency notices** (e.g. § 8 Abs. 4 Nr. 10 UVgO direct
  awards with named winner — **verified** on an editor-entered
  Forschungszentrum Jülich award naming "Hembach Photonik GmbH", legal basis
  and delivery date, but no value).
- **Manual (`editor`) channel share**: 9/500 items in the tender feed
  (~2 %), 34/500 in the award feed (~7 %) (**verified** feed counts).

## 4. Overlap with DÖE and TED — the decisive question

### 4.1 Identifier situation — **verified**

service.bund.de pages and feeds carry **no eForms/DÖE notice UUIDs, no
BT-04/procedure UUIDs, no TED publication numbers, no OJ references** —
checked on 12 sampled detail pages across 12 platform keys (regex sweep for
UUIDs and TED patterns: zero hits). The only identifiers are origin-platform
IDs in the URL plus, on richer pages, the buyer's internal reference
(„Interne Kennung", e.g. `NFW_403.0`) inside prose. **Cross-source merging
would be heuristic-only (title+buyer+date)** — exactly what ADR-0003 forbids.

### 4.2 Sample overlap check — **verified**

Method: 12 current tenders (one per platform key) + 2 current awards sampled
from the live feeds; each searched in DÖE via the anonymous search API that
backs the oeffentlichevergabe.de UI (see §4.4), by distinctive title words,
buyer and publication window; TED v3 API as backstop for EU notices.

| # | platform key (operator) | scope/law | found in DÖE? | DÖE evidence |
|---|---|---|---|---|
| 1 | evergabede | EU/VOB | **yes** | `7eed59e3-…` cn-standard, eforms-de-1.13, pub 08-06 |
| 2 | subreport | national/sonstige | **yes** | `25685198` cn-standard, sdk-0.1, pub 08-07 |
| 3 | eVergabe (Bund) | national/UVgO | **yes** | `408fe98e-…` cn-standard, sdk-0.1, pub 08-07 |
| 4 | dtvp (cosinex) | national | **yes** | `25687808` sdk-0.1, pub 08-07 |
| 5 | vmp-nrw (cosinex) | national/VOB | **yes** | `25690956` sdk-0.1, pub 08-07 |
| 6 | abc (Vergabe24) | EU/VOL | **yes** | `b6f613d2-…` eforms-de-1.13, pub 08-05 |
| 7 | aumass | national | **yes** | `25691238` sdk-0.1, pub 08-07 |
| 8 | obb (RIB Bayern) | national/VOB | **yes** | `25686110` sdk-0.1, pub 08-07 |
| 9 | bi-medien-prod | national/VOB | **yes** | `25689446` sdk-0.1, buyer-matched Landkreis Friesland |
| 10 | editor (manual, Stadtmuseum Berlin) | national/UVgO | **yes** | `25686992` sdk-0.1, buyer-matched |
| 11 | ai-ag-prod (AI/tender24) | EU/VOL | **not yet** | 0 hits in DÖE *and* TED as of 08-09; buyer's other notices in both |
| 12 | asp (RIB Hamburg) | EU/VOB | **not yet** | same pattern |
| A1 | dtvp award | EU/VgV award | **yes** | `fdec19c4-…` can-standard pub 08-06 — **absent from TED on 08-09** (DÖE leads TED, consistent with July findings) |
| A2 | editor award (FZ Jülich) | UVgO §8(4)10 ex-post | **NO** | 0 hits (title, buyer, keyword variants) |

Tally: **11/12 tenders and 1/2 awards verified present in DÖE**, typically
published there **the same day or one day earlier** than on service.bund.de.
The two "not yet" cases are EU notices published Aug 6–8 whose buyers'
other notices flow through DÖE/TED normally — consistent with publication
lag over a weekend (checked Sunday; TED indexed through Aug 7). They cannot
be confirmed as net-new; they *can* be confirmed as EU procedures, which
must reach TED regardless.

The single durable miss is the **manually-entered below-threshold ex-post
award** (A2). That is the one content class where service.bund.de plausibly
has data neither TED nor DÖE gets: authorities that use the BVA
Redaktionssystem (not a DÖE-connected platform) for UVgO ex-post
transparency notices. Upper bound from feed shares: ~7 % of ~120 awards/day
plus ~2 % of ~250 tenders/day ⇒ **order of 10–15 notices/day, minus the
editor entries that *are* in DÖE anyway** (the Stadtmuseum tender was).
Realistic net-new: **low single-digit notices/day, prose-quality,
value-free ex-post awards and small manual tenders.**

### 4.3 A useful correction to the July picture

The July survey assumed the state/commercial platforms "increasingly feed
the DÖE (moderate confidence)". This deep-dive **verifies it empirically
from the other side**: notices originating on subreport, Vergabe24,
bi-medien, aumass, RIB, cosinex, evergabe.de and e-Vergabe all showed up in
DÖE, mostly through the `eforms-sdk-0.1` numeric channel we already parse.
DÖE subsumption of the German portal landscape is stronger than "moderate
confidence" now — for this sample, total.

### 4.4 Bonus finding: DÖE has an anonymous search API

Discovered while testing overlap (from the SPA bundle): **`POST
https://oeffentlichevergabe.de/bkmk/searches`**, anonymous, JSON body in a
"BkmsQL" shape (**verified** working):

```json
{"SELECT":"ALL",
 "WHERE":[{"fields":["allFreeText"],"operator":"MATCH_ALL","operands":["…"]},
          {"fields":["publicationDate"],"operator":">=","operands":["2026-07-01"]}],
 "FROM":"lots","PAGE":{"number":0,"size":25},
 "ORDER":{"field":"publicationDate","direction":"DESC"}}
```

Filterable fields include `buyers.name` (exact `IN`), `allCpvCodes`
(`STARTS_WITH`), `procedureIdentifier`, `noticeType`, `procedureLegalBasis`,
`contractingPlatform`, `allPlacesOfPerformanceNutsCodes`; responses return
`noticeIdentifier`, `noticeVersion`, `procedureIdentifier`, `noticeType`,
`eformsVersion`, buyer, CPV, dates. Undocumented, so not an ingestion path —
but valuable for **targeted QA/cross-checks** of our DÖE importer (e.g.
verifying a notice's presence without downloading a day ZIP). Worth noting
in the DÖE runbook.

## 5. Format — **verified**

- Pages are server-rendered GSB HTML. **No JSON-LD, no microdata, no
  embedded XML, no eForms anything** (0 structured-data blocks on all
  sampled pages).
- Every notice has a fixed-vocabulary **Kurzinfo** block: Leistungen und
  Erzeugnisse, Ausschreibungsweite, Vergabeverfahren (the law),
  Vergabeart (procedure kind), Angebotsfrist, Erfüllungsort
  (postcode/city/Land, with map), CPV-Code(s) — cleanly scrapable.
- Below that, content depends on the feeding platform: some imports carry a
  **full German prose rendering of the notice** (the evergabede sample
  contained an eForms-shaped text dump: buyer with registration number
  `14628110-SV01-42`, NUTS, legal basis "Richtlinie 2014/24/EU", internal
  reference, submission conditions); others carry **only the Kurzinfo** plus
  an outbound link „Bekanntmachung (HTML-Seite)" to the origin platform
  (cosinex `CX…` notice URLs, RIB `meinauftrag.rib.de` IDs, …) or „(PDF-
  Dokument)" directly to a PDF hosted on the origin (subreport). One
  **degenerate import** was found: a healyhudson notice page with empty
  title, no buyer, no fields (**verified**) — imports are not validated.
- **No document/attachment hosting**: Vergabeunterlagen always live on the
  origin platform; the site says so explicitly on every page.

## 6. History and retention

- Design intent, from the Impressum (**verified**, quote): „**Datenbevorratung:**
  service.bund.de ist als reine Bekanntmachungsplattform **analog einer
  Litfaßsäule** konzipiert, eine Langzeitarchivierung von Stellenangeboten,
  Ausschreibungen und Behördendaten **erfolgt nicht**." The Redaktionssystem
  page even tells publishers to screenshot their own notices for proof
  duties. This confirms July's "no archive" with an exact source.
- In practice, delisting happens at the origin-supplied
  Veröffentlichungsende (usually the Angebotsfrist), and delisted URLs
  return **404** (**verified** by probing plausible expired IDs).
  BUT enforcement depends on the origin data: sorting the live stock
  ascending shows a **zombie tail** — the oldest live tender is from
  **2016-07** (subreport, Angebotsfrist 24.08.2016, still served), with
  ~100 pre-2024 and ~300 pre-2025-05 items among 14,057 (~2–5 %)
  (**verified** via paged sampling: page 1 spans 2016→2024-05, p3 ends
  2025-05, p8 reaches 2026-01, p20 2026-04). Awards similar (oldest
  2020-01; bulk within ~5 months, matching ~120/day × 18.5 k stock).
- Wayback-based verification of *when* items disappear was **not possible**
  (web.archive.org blocked by the egress proxy — unverifiable from this
  environment).
- Consequence: as a Source it is **live-only**; a backfill is impossible and
  the retrievable stock is a biased 2–5-month window plus noise.

## 7. Legal / reuse terms — **verified**, and bad

From the Impressum (quote): „**Urheberrecht:** Das Copyright für Texte und
Bilder liegt bei eigenen Inhalten beim Bundesverwaltungsamt. Bei den
Inhalten anderweitiger Anbieter verbleibt das Copyright bei der
einstellenden Behörde. Auf dem Portal zur Verfügung gestellte Texte,
Textteile, Grafiken, Tabellen oder Bildmaterialien **dürfen, soweit diese
urheberrechtlich geschützt sind, ohne vorherige Zustimmung des
Bundesverwaltungsamtes nicht vervielfältigt, nicht verbreitet und nicht
ausgestellt werden**."

- No open-data licence of any kind for the tender data (unlike DÖE's CC0 and
  TED's attribution terms). The RSS feeds are explicitly "pro bono" side
  channels.
- Counter-arguments exist (notice *facts* are unprotectable; § 5 UrhG
  amtliche Werke; the § 28 UVgO publicity purpose), but unlike our two
  existing sources there is **no affirmative reuse grant** — republication
  through our AGPL API would rest on our own legal analysis, not on the
  operator's terms. The July survey's "unclear/weak" is confirmed and now
  precisely quoted.
- robots.txt disallows the search UI and sets `Crawl-delay: 30` — a polite
  scraper must lean on the RSS feeds (which are offered for exactly this)
  rather than crawling search.

## 8. Comparison

| | DÖE (already Source #2) | service.bund.de |
|---|---|---|
| Access | documented bulk API, 3 formats | RSS (500/100-item windows) + HTML |
| Identifiers | eForms UUIDs + BT-04 (TED-mergeable) | per-platform IDs only, no UUIDs |
| History | complete since 2022-12 | none by design (Litfaßsäule), 404 after expiry |
| Volume | ~20–25 k versions/month | ~8–11 k notices/month |
| Below threshold | ~40–45 % of volume, structured eForms | large share, HTML prose |
| Ex-post UVgO direct awards | partial (via platforms) | yes, incl. manual entries |
| Licence | CC0 | restrictive Impressum, no grant |
| Overlap | — | **≥11/13 verified already in DÖE, rest EU→TED** |

## 9. Verdict

**Do not ingest service.bund.de — not now, and (new versus July) probably
not later either.** July kept it alive as a "deliberately-ugly stress
source"; this deep-dive weakens even that rationale:

1. **Coverage**: near-total overlap with DÖE, verified notice-by-notice
   (§4.2). The durable net-new residue is a trickle (low single-digit
   notices/day) of prose-only, value-free manual entries — mostly UVgO
   ex-post awards. tender-db gains no meaningful coverage.
2. **Merge**: no shared identifiers ⇒ every ingested notice would be a
   permanently unmergeable near-duplicate of a Tender we already have in
   structured form — *worse* than useless under ADR-0003, since it creates
   systematic duplicate Tenders that heuristics may not safely collapse.
3. **Cost**: technically easy (RSS + stable URLs + fixed Kurzinfo block),
   but with real fragility: no conditional GET, session-URL noise,
   per-platform body variance, unvalidated/empty imports, zombie retention,
   robots crawl-delay, and a licence posture that would force a per-source
   rights review before our API may republish text.
4. **Schema impact — C22's "no schema impact" mostly holds, with two
   caveats.** (a) Notice/Tender shape: fine — source-local IDs and
   single-notice Tenders are already covered by the C8 decision and the
   sdk-0.1 numeric channel. (b) **Rights metadata**: a source whose content
   we may store but not freely republish would need a per-source (or
   per-notice) exposure flag in the API layer — a *product/API* schema
   concern neither TED (attribution) nor DÖE (CC0) ever raised. (c) A new
   notice kind, "below-threshold ex-post award, prose-only", would be
   Notice-layer-only. If a stress source is ever wanted purely to exercise
   the "no history / no structure / no merge / restricted-reuse" corners of
   the model, service.bund.de remains the canonical specimen — but it should
   be ingested as quarantine-class data, not exposed content.

**If the model-stress goal resurfaces**, the cheaper substitute is a synthetic
fixture source replaying captured service.bund.de RSS+HTML samples — same
stress, no legal exposure, no live scraping duty.

## 10. Corrections/additions to existing docs

- german-portals.md §3 "notices are removed once expired": refine to
  "delisted (404) at origin-supplied Veröffentlichungsende; enforcement is
  origin-dependent — verified zombies back to 2016 remain".
- german-portals.md §3 "no open-data grant": now backed by the exact
  Impressum quote (§7 above).
- Survey doc's "state/commercial platforms feed DÖE (moderate confidence)":
  upgrade — empirically verified for cosinex, subreport, Vergabe24,
  bi-medien, aumass, RIB, evergabe.de, e-Vergabe (§4.3).
- DÖE runbook: note the anonymous `POST /bkmk/searches` search API (§4.4)
  as a QA tool for the importer.

## 11. Open questions

Needs more research (only if the source is ever reconsidered):
- The two lag cases (§4.2 #11/#12): re-check after a few business days
  whether AI-AG- and RIB-originated EU notices appear in DÖE or only in TED
  (would tell us whether some eSenders still bypass the BKMS — relevant to
  DÖE-completeness claims, not to this verdict).
- True size of the DÖE-invisible `editor` residue: needs a 2–4-week diff of
  the editor-channel RSS items against DÖE search — the single-day sample
  (1 miss, 1 hit) only bounds it.
- Whether GSB exposes any undocumented JSON search view (none found; GSB
  documentation not exhaustively searched).
- Delisting timeline precision (blocked here: web.archive.org unreachable
  through the egress proxy).
- Whether the § 5 UrhG / facts-only argument suffices for republishing
  Kurzinfo *metadata* (not prose) — lawyer question, only if ingestion is
  ever revisited.

Needs a user decision:
- **C22 closure**: accept this verdict ("no third source; C22 closed as
  won't-do, synthetic fixture instead if stress is wanted") or keep C22
  parked. Recommendation: close it — the overlap evidence removes the
  coverage rationale, and the stress rationale is servable without live
  ingestion.
