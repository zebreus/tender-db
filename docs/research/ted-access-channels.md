# TED access channels

Research date: 2026-07-19. All downloads/probes were executed on the Hetzner
VPS (`root@zebreus.click`); samples live under `/opt/tender-db/samples/`
(era-ladder daily packages 1993–2026, extracted copies under `samples/x/`,
plus `monthly-sizes.csv` and `yearly-counts.csv` produced by the sweep
scripts `sweep.sh` / `lastpkg.sh` in the same directory).

Everything marked **[verified]** was tested live with curl on 2026-07-19;
**[docs]** comes from official documentation; **[unverified]** is inferred.

## Summary

- TED offers four practical channels: **bulk tar.gz packages** (daily +
  monthly, 1993→today, anonymous), the **Search API v3** (anonymous, but only
  indexes a **rolling `today − 10 years` window** — see the correction below),
  **per-notice direct URLs** (XML/PDF/HTML, work back to 2011), and derived
  open-data channels (CSV on data.europa.eu, SPARQL) we don't need.

  > **Correction (2026-08-09 drift audit).** This doc originally said the API
  > "only indexes notices from July 2016" — measured 2026-07-19, that was the
  > rolling edge, not a fixed floor. Re-probed 2026-08-09: the boundary sits at
  > exactly 2016-08-09 (2016-08-08 → 0 results, 2016-08-09 → 1,575, scope ALL),
  > i.e. **the floor advances one day per day**, confirmed by TED's own Q&A
  > ("the last 10 years from today"). Consequences: gap-fill/cross-check logic
  > must compute the floor as `today − 10y`, never hard-code a date; and
  > archive data older than the window can only ever be verified against bulk
  > packages, not the API. See docs/research/upstream-drift-2026-08.md.
- The archive spans **three fundamentally different format eras**: tagged
  plain text (1993–2010), legacy TED XML `TED_EXPORT` R2.0.8/R2.0.9
  (2011–2024), and eForms UBL (late 2023→). "All business terms" can only
  ever hold for the eForms era; this is the single biggest consequence for
  the canonical model (details in [Format eras](#format-eras)).
- Full raw history (all languages) is **~188 GB compressed — it does not fit
  on the 75 GB disk**. The XML era only (2011→mid-2026) is **~36 GB
  compressed** and fits comfortably; the text era can be reduced to ~8 GB by
  keeping only the English zips.
- Publication is a strictly **daily cycle** (Mon–Fri, package final by 09:30
  CET); there is no intra-day channel, so "near-realtime" means "same
  morning".

## 1. Bulk download packages

### URLs [verified]

| Package | URL pattern | Example |
|---|---|---|
| Daily | `https://ted.europa.eu/packages/daily/{yyyy}{nnnnn}` (OJ S issue number, zero-padded to 5) | `https://ted.europa.eu/packages/daily/202600136` → `20260717_136.tar.gz` |
| Monthly | `https://ted.europa.eu/packages/monthly/{yyyy}-{m}` (month **not** zero-padded) | `https://ted.europa.eu/packages/monthly/2019-1` → `2019-01.tar.gz` |

- Anonymous — no login, no API key, no cookie. [verified]
- Nonexistent packages return **404** (e.g. future issues); no soft-200s. [verified]
- The pattern `…/packages/notice/daily/…` that appears in older docs and the
  Developers' Corner page returns **404** — only `/packages/daily/` works.
  The URL scheme has therefore changed at least once already; treat it as
  medium-stability. [verified]
- Real filename comes in `Content-Disposition`. Format drifted once already
  (2026-08 audit): originally `20260717_136.tar.gz`, now
  `{yyyymmdd}_{yyyy}{nnn}.tar.gz` (e.g. `20260717_2026136.tar.gz`) —
  retroactively, with package bytes unregenerated. We key blobs by our own
  scheme, so this is cosmetic; do not key anything off that header.
- Coverage: **1993-01 through today**, both daily and monthly, no gaps found
  in a full HEAD sweep of all 402 monthly packages (a handful of first-sweep
  misses re-probed fine — they were transient timeouts). [verified]
- Served via CloudFront; `accept-ranges: bytes` (resumable); **no ETag, no
  Last-Modified, no checksums**, `cache-control: no-store`. [verified]

Probe commands used:

```sh
curl -sI https://ted.europa.eu/packages/daily/202600001      # headers
curl -sI https://ted.europa.eu/packages/monthly/1993-1
# full sweep: HEAD every monthly package 1993-2026 -> monthly-sizes.csv
```

### Package structure [verified, from extracted samples]

A **monthly package is exactly the concatenation of its daily packages**
(a tar.gz containing the daily `*.tar.gz` files; since ~2025 nested in a
`{month}/` directory). Same bytes, fewer requests — use monthlies for
backfill, dailies for the live tail.

Daily package layout by era:

| Era | Layout | Sample |
|---|---|---|
| 1993–1999 | flat zips, initially **English only**: `EN_19930102_1993001_ISO_ORG.zip`; more languages added over the 90s | `daily-199300001` (84 KB, 1 zip) |
| 2000–2007 | one zip per language, `{LG}_{yyyymmdd}_{nnn}_ISO_ORG.ZIP` (UTF8 variants appear mid-era) | `daily-200500001` (48 MB, 38 zips) |
| 2008–2010 | two zips per language: `{lg}_…_utf8_org.zip` (tagged text) **and** `{lg}_…_meta_org.zip` (structured SGML-ish `<part><doc>` records) | `daily-200800001` (63 MB, 46 zips) |
| 2011→ | one directory `{yyyymmdd}_{nnn}/` with **one XML file per notice**: `000002_2011.xml` (6-digit) / `00497150_2026.xml` (8-digit since eForms) | `daily-201900001` (1 529 XML), `daily-202600136` (3 722 XML) |

Notice filenames are the publication number (`{number}_{year}`), assigned
sequentially within a year — which is why `max(number)` in the last package
of a year is a good proxy for the year's notice count (used below).

### Publication schedule [docs]

From [TED help — data reuse](https://ted.europa.eu/en/help/data-reuse):

- OJ S is published **Monday–Friday** except Publications Office holidays
  (2026 has 254 issues; machine-readable release calendar:
  `https://ted.europa.eu/en/release-calendar/-/download/file/CSV/2026`
  [verified]).
- **Daily package**: uploaded between **00:01 and 09:00 CET** on publication
  day, "finalized by 09:30" — i.e. the file may be replaced during that
  window; do not fetch before ~09:30 unless you re-check afterwards.
- **Monthly package**: available the **5th working day** of the following
  month by 09:30.

### Rate limits [docs]

From the [Developers' Corner](https://ted.europa.eu/en/simap/developers-corner-for-reusers):
max **3 concurrent package downloads**, **700 HTTP requests/min**, and 600
notice visualisations/downloads per IP per <6 min. Generous for our purposes;
a serial backfill loop stays far below all of them.

## 2. Format eras — what the archive actually contains

This is the critical section. **TED historical data is NOT eForms**, and the
pre-2011 data is not even XML. Era boundaries below were established by
inspecting real notices from Jan-1 packages of 1993, 2000, 2005, 2007, 2008,
2010, 2011, 2014, 2015, 2016, 2017, 2018, 2019, 2023, 2024, 2025, 2026
(all on the VPS under `samples/x/`).

### Era 0: tagged text, 1993 – 2010 [verified]

Not XML. Per-language files of concatenated records in the "TED
DAILY-DELIVERY" format: ~20 coded header fields (`TI:` title, `PD:`
publication date, `ND:` document number, `TD:` document type, `NC:` contract
nature, `PR:` procedure, `AA:` authority type, `CY:` country, `CC:`/`PC:`
CPV-ish classification, `AU:` authority name, …) followed by `TX:` **free
text** — the entire body of the notice is unstructured prose. From 2008
there is additionally a `meta` variant per language: records like
`<part id="2008001" lg="en"><doc id="1-2008" …><codifdata>…` — the same
coded fields as pseudo-XML plus the text body, slightly more parseable but
no more structured.

- Encoding: `ISO` (Latin-1) zips until ~2005, `UTF8` variants alongside/after.
- **No schema documentation found** for either variant on docs.ted.europa.eu
  or op.europa.eu; the field codes match the legacy TED website search codes
  (documented historically in TED help). Confidence in the field-code
  semantics: medium; would need reverse-engineering + the old TED help pages.
- Canonical-model coverage: **only the coded header fields** (~15–20 BT
  equivalents: dates, buyer name/country, procedure type, contract nature,
  CPV, title) plus a text blob. Lots, bids, values, winners etc. exist only
  inside free text. **"All business terms" is impossible for this era.**

### Era 1: TED XML `TED_EXPORT` R2.0.8, 2011 – mid-2016 [verified]

One XML per notice, root `<TED_EXPORT>`; namespace
`http://publications.europa.eu/TED_schema/Export` (2011 files carry no
version in `schemaLocation`; by 2014 it reads `…/Export/R2.0.8.S02.E01`).
Structured `FORM_SECTION` per standard form (directives 2004/17/EC,
2004/18/EC forms F01–F19 plus defence forms). Multilingual: original
language + translated summaries in one file.

### Era 2: TED XML `TED_EXPORT` R2.0.9, mid-2016 – early 2024 [verified]

Same root element, new namespaces:
`http://formex.publications.europa.eu/ted/schema/export/R2.0.9.S01.E01`
(2016-11 / 2017 samples), then `…/resource/schema/ted/R2.0.9/publication`
(+ `ted/2016/nuts`) from 2018, revisions S01→S05. Forms F01–F25 for the
2014 directives (2014/23/24/25/EU). The changeover happened **during 2016**
(2016-01-01 package: R2.0.8; 2016-11 package: R2.0.9), so 2016 packages
contain a mix; defence-directive forms 16–19 stayed on R2.0.8 even later.

**Schema docs & XSDs for eras 1–2**: archived at the Publications Office
[EU Vocabularies — TED XML schemas archive](https://op.europa.eu/en/web/eu-vocabularies/e-procurement/tedschemas)
— Reception/Internal/**Publication** XSD zips for R2.0.9 (dated versions
2016-06-03 … 2021-07-30) and R2.0.8, plus a "TED XML general description"
PDF. Page explicitly labels itself an archive. Recommend mirroring these
zips to the VPS soon — archived pages have finite lifetimes. [docs, page
fetched 2026-07-19]

### Era 3: eForms (UBL), in packages from Nov 2022 (mandatory 2023-10-25) → today [verified]

Roots `ContractNotice` / `ContractAwardNotice` / `PriorInformationNotice` /
`BusinessRegistrationInformationNotice` in OASIS UBL 2.x namespaces plus the
`efac/efbc/efext` eForms extension namespaces. Governed by Implementing
Regulation (EU) 2019/1780; **mandatory for new notices since 2023-10-25**;
**legacy-schema submission closed 2024-01-31** ([Publications Office
tedschemas page]). The eForms SDK (already mirrored at
`/opt/tender-db/eforms-sdk`) is the authoritative, versioned schema +
`fields.json` source.

**Processor dispatch keys** (per notice, all in the header): the SDK version
`<cbc:CustomizationID>eforms-sdk-1.14</cbc:CustomizationID>`, the notice
subtype `<cbc:SubTypeCode listName="notice-subtype">29</cbc:SubTypeCode>`
(1–40), the result/family code `<cbc:NoticeTypeCode listName="result">`, and
`<cbc:UBLVersionID>2.3` / `<cbc:VersionID>`. The subtype + `CustomizationID`
together select the field set.

**A single package mixes SDK versions** [verified]: `daily-202600136`
(2026-07-17) carried `eforms-sdk-1.12`, `1.13` and `1.14` notices side by side
(19 / 114 / 59 in the first 200 files). The sender's SDK version at submission
time is frozen into the notice, so the processor must accept a *range* of SDK
versions concurrently and the fields.json completeness test (ADR-0002) must
run against the *set* of versions actually ingested, not one pinned version.

**National eForms extensions appear** [verified]: German eForms notices in the
2026 sample declare an extra namespace `xmlns:defext="german-eforms-extension"`
(eForms-DE). Country-specific extension content must be explicitly mapped or
ignore-ruled, or ADR-0004 quarantines those notices whole. Cross-reference the
German-portals research for the eForms-DE field set.

### Measured era mix in real packages [verified]

Root-element counts per daily package:

| Package (date) | TED_EXPORT | eForms roots |
|---|---|---|
| 2023-10-24 (S 205) | 2 951 | 284 |
| 2024-01-02 (S 1) | 1 410 | 826 |
| 2024-07-02 (S 125) | 18 | 3 355 |
| 2025-01-02 (S 1) | 0 | 1 763 |
| 2026-07-17 (S 136) | 0 | 3 722 |

So: eForms appear well **before** 2023-10-25 (voluntary use from ~2023),
legacy XML persists well **after** it (tail through 2024). **Any package
from ~mid-2023 to late 2024 is mixed** — the processor must dispatch on the
root element, never on the package date.

### Consequence for "all business terms"

| Era | Years | Share of archive (notices) | BT coverage achievable |
|---|---|---|---|
| eForms | 2023-10 → | ~2.5 M and growing ~850 k/yr | **Full** (this is what BTs are) |
| R2.0.9 | 2016 – 2024 | ~4.5 M | High but incomplete — OP's own [ted-xml-data-converter](https://github.com/OP-TED/ted-xml-data-converter) (R2.0.9→eForms XSLT) states output "will not be complete and will also contain some errors"; F14/F20 and defence-directive notices unsupported; some BTs (e.g. BT-22) simply don't exist in TED XML |
| R2.0.8 | 2011 – 2016 | ~2.2 M | Moderate — same shape, older forms, **no official converter** |
| Text | 1993 – 2010 | ~4.2 M | Minimal: ~15–20 header fields + free-text blob |

**Flag prominently**: the CONTEXT.md guarantee "all eForms Business Terms
representable, no omissions" can only be a statement about the *schema* and
about *eForms-era ingestion*. For earlier eras the same canonical schema can
hold whatever exists, but most BTs will be legitimately NULL, and ADR-0004's
"unmapped content quarantines the notice" needs an era-aware notion of
"mapped" (a text-era notice is 90 % free text by design — that must not
count as unmapped content, or the whole pre-2011 archive quarantines).

## 3. Search API v3 (`api.ted.europa.eu`)

Docs: <https://docs.ted.europa.eu/api/latest/index.html>, Swagger UI at
<https://api.ted.europa.eu/swagger-ui/index.html>.

- Endpoint: `POST https://api.ted.europa.eu/v3/notices/search`, JSON body
  `{query, fields, page, limit, scope, paginationMode, iterationNextToken}`.
  **Anonymous** — verified working with no key. Other v3 APIs (publication,
  validation, `/v3/notices/fields`, …) require an API key from
  developer.ted.europa.eu (the fields endpoint answered
  `400 Missing Authorization header`). [verified]
- Query language: "expert search" — `publication-date>=20260716 AND …`,
  supports `SORT BY publication-date DESC` inline. Field names are the
  v3/eForms-aligned names (`publication-number`, `publication-date`,
  `notice-type`, `buyer-country`, …). [verified]
- Limits [verified by triggering the errors]: `limit` ≤ **250**
  (`SEARCH_EXCEEDS_MAX_LIMIT`), `page×limit` ≤ **15 000**
  (`SEARCH_WINDOW_TOO_WIDE`); beyond that use
  `paginationMode: "ITERATION"`, which returns an `iterationNextToken`
  (deep-scroll, verified working).
- Response: JSON metadata per notice plus **links** to the full notice in
  every format/language: `links.xml.MUL` →
  `https://ted.europa.eu/en/notice/{number}-{year}/xml`, plus pdf/html
  variants. It does not inline full notice XML; you follow the link.
  [verified]
- **Coverage floor: July 2016.** `publication-date=20110104` → 0 results in
  every scope; monthly counts show 0 through 2016-06 and 21 482 for 2016-07.
  The API cannot drive historical backfill. [verified]
- Freshness: for 2026-07-17 the API count (3 722) exactly equals the daily
  package's file count — same daily publication cycle, no earlier access.
  [verified]
- Rate limit: 700 req/min per IP (site-wide figure from Developers' Corner).
  [docs]
- Stability signal: **v2 is already dead** (`POST /v2/notices/search` → 404).
  Documented plan was "v2 supported until end of transition period
  (Sept 2025)"; that happened. The current docs promise v3 support "until
  version 4 becomes available". Expect a breaking migration every few years;
  the bulk-package channel has been more stable than the API (but note the
  `/packages/notice/…` → `/packages/…` URL change). [verified/docs]

Per-notice direct URLs (usable without the API):
`https://ted.europa.eu/en/notice/{n}-{yyyy}/xml` works for eForms **and**
legacy notices back to 2011 (`2-2011` → TED_EXPORT XML) even though search
can't find them; pre-2011 numbers → 404 ("Invalid document number").
[verified]

## 4. Other channels (not recommended as primary)

- **RSS feeds** (`https://ted.europa.eu/en/simap/rss-feed`): latest notices
  by sector; same daily cycle, no advantage over API polling. [docs]
- **data.europa.eu CSV** ([ted-csv dataset](https://data.europa.eu/data/datasets/ted-csv)):
  Commission-curated annual CSV extracts (subset of fields, from 2006) —
  useful as an external cross-check for coverage counts, not as a source.
  [docs, not deeply verified]
- **TED Open Data / SPARQL** (Cellar, docs at
  <https://docs.ted.europa.eu/ODS/latest/index.html>): RDF via the ePO
  ontology; eForms-era only, adds a semantic layer we'd immediately flatten.
  [docs]
- **eForms SDK** (github OP-TED/eforms-sdk, mirrored on the VPS): not a data
  channel, but the schema source the processor validates against.

## 5. Realtime strategy inputs

- Publication cadence: **once per day, Mon–Fri**, package final 09:30 CET;
  ~254 OJ S issues/year. Weekend/holiday = nothing new. The **minimum
  achievable latency on any channel is the daily publication itself**;
  within a publication day, package and API become available in the same
  00:01–09:30 window.
- Volume today: **~1.8–3.7 k notices/issue** (measured range across
  2025–2026 samples), ~870 k/year, average daily package ~19 MB compressed /
  ~200 MB unpacked.
- Recommended fetch plan:
  1. **Backfill**: monthly packages, newest→oldest (product value is highest
     for recent years), 2011+ first.
  2. **Live**: fetch the daily package each publication day after ~09:30 CET
     (release calendar tells you which days); retry on 404 until it appears.
  3. **Search API as safety net only**: daily
     `publication-date=YYYYMMDD` count compared against package file count
     (they matched exactly in testing — a free integrity check), and for
     ad-hoc gap re-fetches of single notices via the per-notice XML URL.
- Corrections arrive as **new notices** (legacy: corrigenda; eForms: change
  notices carrying `ChangedNoticeIdentifier`), not as edits to old packages
  — the daily stream is append-only at the notice level, which fits the
  append-only Notice store directly.

## 6. Data volumes (measured)

Full HEAD sweep of all monthly packages (Content-Length,
`samples/monthly-sizes.csv`), gaps re-probed:

| Years | Era | Compressed size |
|---|---|---|
| 1993–2003 | text | 15.3 GB (grows 0.03→3.1 GB/yr) |
| 2004–2010 | text, 20+ languages × utf8+meta | **136 GB** (10–26 GB/yr — every notice duplicated per language) |
| 2011–2022 | TED XML | 23.1 GB (1.6–2.3 GB/yr, flat) |
| 2023 | mixed | 2.7 GB |
| 2024 | mixed→eForms | 3.8 GB |
| 2025 | eForms | 4.2 GB |
| 2026 (Jan–Jun) | eForms | 2.3 GB (→ ~4.7 GB/yr) |
| **Total 1993–2026H1** | | **~188 GB** |

- Compression ratio (tar.gz → unpacked XML): ~6.6× (2019) to ~10.7× (2026
  eForms). XML era unpacked ≈ **250–350 GB** — raw payloads must be stored
  compressed.
- **XML era 2011→today ≈ 36 GB compressed** — fits the 75 GB disk (68 GB
  free) with headroom, but note growth: eForms is fatter every year
  (2.7 → 3.8 → 4.2 → ~4.7 GB/yr, ~12–25 %/yr). At ~5 GB/yr the raw store
  alone adds ~25 GB over 5 years, **before** the SQLite parsed/canonical
  layer, which will plausibly be the same order of magnitude as the
  uncompressed source it retains. 75 GB is workable for raw-2011+ but tight
  once the DB grows; disk needs monitoring from day one.
- **Full raw history in all languages (~188 GB) does not fit.** The
  text-era bloat is per-language duplication: the English zips are ~4.5–5 %
  of a 2005/2008 package (measured), so an English-only text-era mirror is
  **~7–8 GB** — with that reduction, full-history raw storage is ~44 GB and
  fits today, at the cost of discarding the other language editions
  (originals! the EN edition of a French notice is partly a translation —
  a real data-fidelity decision, see open questions).

Notice counts per year (ground truth for the dashboard coverage metric;
1993–2016 = highest publication number in the year's last daily package,
`samples/yearly-counts.csv`; 2017+ = Search API `totalNoticeCount`):

| Year | Notices | Year | Notices | Year | Notices |
|---|---|---|---|---|---|
| 1993 | 74 433 | 2005 | 249 437 | 2017 | 528 975 |
| 1994 | 94 954 | 2006 | 268 060 | 2018 | 578 501 |
| 1995 | 138 824 | 2007 | 307 255 | 2019 | 622 786 |
| 1996 | 151 945 | 2008 | 339 534 | 2020 | 643 552 |
| 1997 | 166 394 | 2009 | 363 230 | 2021 | 676 734 |
| 1998 | 177 012 | 2010 | 391 397 | 2022 | 735 067 |
| 1999 | 209 009 | 2011 | 411 850 | 2023 | 795 680 |
| 2000 | 161 228 | 2012 | 414 837 | 2024 | 801 444 |
| 2001 | 172 194 | 2013 | 443 079 | 2025 | 871 149 |
| 2002 | 202 684 | 2014 | 446 419 | 2026→07-17 | 497 791 |
| 2003 | 224 144 | 2015 | 463 821 | | |
| 2004 | 221 786 | 2016 | 466 898 | | |

Total ≈ **12.9 M notices** 1993–mid-2026; ~9.0 M in the XML era (2011+).
(Max-publication-number slightly overcounts if numbers were skipped;
API-vs-filename comparison for 2026 differed by 0.13 %.)

## 7. Legal terms of reuse

From the [TED legal notice](https://ted.europa.eu/en/legal-notice) [fetched
2026-07-19]: procurement notices in the OJ S supplement "can be freely
reused, for commercial or non-commercial purposes", under the Commission's
reuse policy (Commission Decision 2011/833/EU of 12 December 2011).
Editorial site content is CC BY 4.0 (credit + indicate changes); metadata is
CC0 1.0. Practical obligation for tender-db: **attribute TED/Publications
Office as source and indicate that data has been transformed** — a footer
line and an API `source` field satisfy this. No registration, no quota, no
share-alike. Low risk; AGPL for our code is unaffected.

## 8. Fetcher implementation notes

- **Conditional requests are useless**: no ETag/Last-Modified,
  `cache-control: no-store`. Idempotency must come from our side: key stored
  payloads by package identifier (OJ S issue number / year-month) + SHA-256
  computed on receipt.
- **No official checksums** anywhere; store our own hash with the raw blob
  (also gives ADR-0004 raw-payload versioning a natural version key: same
  package URL re-fetched with a different hash = new version).
- `Content-Length` is reliable and `accept-ranges: bytes` works — verify
  length after download, resume with Range on failure.
- **Do not trust old documented URL patterns** — `/packages/notice/daily/`
  is dead. Keep URL templates in config, not code.
- 404 means "not (yet) published" for future issues — the live fetcher can
  poll the next issue number with backoff; use the release calendar CSV to
  know which dates have issues at all.
- Daily package may be **rewritten until 09:30 CET** on its publication day;
  fetch after that, or re-fetch and compare hashes.
- Republication of *historical* packages: no mechanism documented and no
  ETags to detect it cheaply; corrections flow as new notices instead.
  Suggest a low-frequency (monthly) spot re-hash of a few old packages for
  the first year to confirm they are immutable. [unverified]
- Mixed-format packages (2023–2024) mean format dispatch happens **per
  file** on the XML root element, not per package.
- Notice files inside a package are one XML per notice with the publication
  number as filename — the natural raw-store key is
  `(year, publication_number)`.
- Concurrency: keep ≤3 parallel package downloads (documented limit); one is
  plenty on the 1 Gb/s VPS anyway (a monthly ~350 MB arrives in seconds).
- The whole XML-era backfill is only **~36 GB / ~190 fetches** (monthlies) —
  a night's work on the VPS; don't over-engineer the backfill scheduler.

## Implications for tender-db

1. **Channel choice is settled**: bulk packages (monthly for backfill, daily
   for live) as the only ingestion path; Search API v3 as a count
   cross-check and single-notice gap-filler. The API cannot do backfill
   (July-2016 floor) and offers zero latency advantage.
2. **The canonical model needs an era story.** eForms BTs are fully
   populatable only from late 2023. R2.0.9/R2.0.8 need hand-written mappings
   (the official converter is a useful mapping *reference* but is lossy and
   excludes F14/F20/defence forms). Text-era notices can populate only a
   thin header + a text blob. The "no omissions" guarantee must be re-scoped
   to "no omissions *of what the source era contains*", and ADR-0004's
   unmapped-content rule needs per-era mapping definitions.
3. **Disk fits only with policy**: raw 2011+ compressed ≈ 36 GB (fine);
   all-language full history ≈ 188 GB (does not fit); EN-only text era ≈
   +8 GB (fits). Growth ~5 GB/yr raw + DB growth means the 75 GB box has
   headroom measured in a few years, not decades.
4. **Daily cadence simplifies the live path**: one scheduled fetch per
   publication day after 09:30 CET; SSE/webhook "liveness" is bounded by
   TED's own daily cycle, worth stating in product docs.
5. **Free integrity check**: API day-count == package file-count (matched
   exactly in testing). Cheap nightly assertion; feeds the dashboard
   coverage metric directly, with the per-year table above as historical
   ground truth.

## Open questions

### Needs more research

- **Text-era (1993–2010) semantics**: no official schema/spec found for the
  tagged-text or `meta` formats; field-code meanings inferred. Need to hunt
  archived TED/SIMAP documentation (or Publications Office contact) before
  committing to parse it.
- **Pre-2011 daily-vs-meta fidelity**: does the `meta` variant (2008+)
  contain anything the text variant lacks (or vice versa), and which
  language edition is authoritative per notice (`OL:` field marks original
  language — verify it's always present).
- **Package immutability**: are historical packages ever regenerated?
  No signal either way; needs a re-hash probe over time.
- **R2.0.8 XSD completeness**: confirm the op.europa.eu archive zips cover
  the 2011–2013 files (whose `schemaLocation` carries no revision) — mirror
  the archive zips to the VPS regardless.
- **Exact API floor**: search index starts during July 2016 (2016-06 → 0,
  2016-07 → 21 482); exact cutover date unimportant but unconfirmed.
- **data.europa.eu CSV** as an independent cross-check of the per-year
  counts (not yet compared).

### Needs a user decision

- **How far back does backfill go?** Options with measured costs:
  (a) eForms only (2023→, ~11 GB raw, full BTs), (b) XML era 2011→ (~36 GB
  raw, good-but-partial BTs, 9.0 M notices), (c) full history 1993→
  (+8 GB EN-only or +150 GB all languages, minimal structure pre-2011,
  12.9 M notices). Recommendation: (b) now, (c)-EN-only later if wanted.
- **Do legacy-era notices get a reduced canonical mapping** (thin
  header + raw text, most BT columns NULL) — and is that acceptable against
  the "all business terms" product promise, suitably re-worded?
- **Text era: keep English only, or all language editions?** EN-only is a
  20× size saving but discards original-language texts (EN is often a
  translation for non-UK notices).
- **Era-aware quarantine policy** (ADR-0004): agree that "fully mapped" is
  defined per era, so structurally-poor old notices don't flood the
  quarantine metric.
