# UK Find a Tender Service as a tender-db source (issue 342, unit 1)

Measured on 2026-09-07 (live API calls from this container through the session's egress proxy; every number marked "measured" comes from those calls, scripts and raw pages kept under the session scratchpad).

## 1. Access and terms

FTS publishes notices as OCDS 1.1.5 release packages and record packages from `https://www.find-tender.service.gov.uk/api/1.0/` (https://www.find-tender.service.gov.uk/Developer/Documentation).
`GET /api/1.0/ocdsReleasePackages` takes exactly five parameters: `limit` (1–100, default 100), `cursor` (`[A-Za-z0-9=]*`, max 300 chars), `updatedFrom`, `updatedTo` (`YYYY-MM-DDTHH:MM:SS`, 19 chars) and `stages` (`planning,tender,award`) (https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages).
Any other parameter is a 400 with the message "Request parameter 'statuses' is not recognised, allowed parameters are: stages, limit, cursor, updatedFrom, updatedTo" (measured; same text in the docs above).
`GET /api/1.0/ocdsReleasePackages/{id}` filters to one notice (`nnnnnn-yyyy`) or one process (`ocds-h6vhtk-hhhhhh`, zero-padded hex) (https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages).
`GET /api/1.0/ocdsRecordPackages/{ocid}` returns one record with all releases, a `compiledRelease` and a `versionedRelease`; it has no URI parameters and rejects `updatedFrom` with a 400 (https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsRecordPackages; measured).
Authentication: none — every call above succeeded with a plain unauthenticated GET and no API key (measured).
The XML notice feed is separately available as daily zip files on data.gov.uk, published by Crown Commercial Service (https://www.find-tender.service.gov.uk/Developer/Documentation; https://www.data.gov.uk/dataset/uk-public-procurement-notices-june-2026).

Rate limits are documented only as "HTTP 429 ... no further requests should be made until after the number of seconds specified in the Retry-After header value", with 503 handled the same way (https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages).
The live 429 body is the 58-byte text `Rate limit of 12 exceeded. Please retry after 120 seconds.` with `Retry-After: 120` (measured).
The limiter's window could not be pinned down from outside: 19 page fetches back-to-back (~1.1 s each) all returned 200, yet after 150 s of silence a burst of 16 got 429 on 10, a 6-second cadence got 429 on 5 of 14, and an 11-second cadence got 429 on 7 of 13 (measured).
A shared egress address or per-node counters would explain that pattern; the fetcher must treat 429 as routine, sleep `Retry-After`, and resume the same URL (measured; docs above).
Response headers carry no `X-RateLimit-*` fields and `Cache-Control: no-store` (measured).

Licence: every package declares `"license": "http://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/"` and `"publicationPolicy": "https://www.gov.uk/government/publications/open-contracting"` (measured on the package header).
The FTS terms say "You can reproduce content published on Find a Tender under the OGL as long as you follow the licence's conditions" and that feeds may be cached by third parties (https://www.find-tender.service.gov.uk/Home/TermsAndConditions).
OGL v3 grants the right to "copy, publish, distribute and transmit the Information; adapt the Information; exploit the Information commercially and non-commercially" (https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/).
The licence's condition is to "acknowledge the source of the Information in your product or application by including or linking to any attribution statement specified by the Information Provider(s) and, where possible, provide a link to this licence"; absent a specified statement the wording is "Contains public sector information licensed under the Open Government Licence v3.0.", and where several providers are combined "you may include a URI or hyperlink to a resource that contains the required attribution statements" (same URL).
Personal data, logos and third-party rights are excluded, and "If you fail to comply with them the rights granted to you under this licence ... will end automatically" (same URL).

## 2. Fetch channel

A release package is `{uri, version:"1.1", extensions[], publishedDate, publisher{name:"Cabinet Office", scheme:"GB-GOR", uid:"D2"}, license, publicationPolicy, releases[], links{next}}` (measured, `ocdsReleasePackages?updatedFrom=2026-09-03T00:00:00&updatedTo=2026-09-03T23:59:59`).
Pages hold at most 100 releases; `limit=500` was never answered (429 both times) and `limit=0` is a 400 "'limit' must be greater than 0" (docs; measured).
Releases within a window come newest-first: page 1 of 3 September ran from `2026-09-03T23:31:32+01:00` down to `15:46:51+01:00` (measured).
Paging is by `links.next`, a full URL with an opaque `cursor`; the last page has no `links.next` (measured; https://raw.githubusercontent.com/open-contracting-extensions/ocds_pagination_extension/master/README.md).
The cursor is base64 of `updatedFrom=...|updatedTo=...|nextCursor=590457`, i.e. a server-side sequence position, not a timestamp (measured by decoding).
`updatedFrom`/`updatedTo` are interpreted in UK local time: the 3 September window returned dates from `00:26:55+01:00` to `23:31:32+01:00`, and a January 2021 window ended at `23:08:12Z` (measured).
`updatedFrom` alone works; the server appends `updatedTo=<now>` to the echoed `uri` (measured).
The documented example window is seven days; my 31-day and 41-day windows were rate-limited before answering, so wide windows are unverified (docs; measured).
`stages=tender` on 3 September returned 3 releases against 73 releases tagged `tender`, so the filter is not a tag filter and should not be used (measured).
A single-notice fetch returns one release of ~13 KB; a record package for one process (`ocds-h6vhtk-0510f8`, 12 releases) was 21.6 MB because `versionedRelease` repeats every field's history (measured).

Bulk alternatives exist but neither is the OCDS JSON the API serves.
data.gov.uk hosts one zip per day since January 2021 (`.../Harvester/2026-09/UK Public Procurement Notices - 3rd September 2026.zip`, 975,358 bytes, 436 XML files) with no rate limiting observed (measured; https://ckan.publishing.service.gov.uk/api/action/package_search?q=uk-public-procurement-notices).
Those files are `TED_EXPORT` XML: R2.0.9-schema forms for legacy notices, and `<UK6_2023 FORM="UK6" LEGAL_BASIS="2023/54">` elements that serialise the OCDS release element-by-element for Procurement Act notices (measured on the 2021-06-16 and 2026-09-03 zips).
The dataset page shows licence "Not set" even though the FTS site states OGL for its content (https://www.data.gov.uk/dataset/uk-public-procurement-notices-june-2026; https://www.find-tender.service.gov.uk/Developer/Documentation).
The OCP Data Registry re-publishes FTS as a 214 MB compressed all-time JSON download, retrieved weekly, covering January 2021 to August 2026 (https://data.open-contracting.org/en/publication/41).

Recommended strategy: daily poll of the previous UK-local day with a 2-hour overlap on `updatedFrom`, idempotent on release `id` (`nnnnnn-yyyy`).
That is at most 6 requests per day at the current volume (busiest measured day: 471 releases = 5 pages) (measured, table in §7).
One-time backfill by 1-day windows from `2021-01-01` gives ~4,700 requests (2,076 days, weekdays 2021–2024 mostly 2 pages, 2025–2026 4–5 pages, weekends 1); 7-day windows for 2021–2024 cut it to ~3,300 (measured day counts, §7; 319,742 releases / 100 per page = 3,198 pages minimum).
At the ~5 successful requests per minute this environment sustained, the backfill is 11–16 hours of wall-clock plus 429 back-offs, i.e. one to two days unattended (measured cadence, §1).
Retry-After must be honoured per request and progress recorded per (window, cursor) so a restart resumes mid-window (docs, §1).

## 3. History depth

| Source | Earliest data | Evidence |
|---|---|---|
| FTS OCDS API | 2 January 2021 | December 2020 window: 0 releases; 1–3 Jan 2021: 8 releases, newest `000008-2021` dated `2021-01-02T17:00:05Z` (measured) |
| FTS XML zips | January 2021 | monthly datasets start `uk-public-procurement-notices-january-2021` (https://www.find-tender.service.gov.uk/Developer/Documentation) |
| Contracts Finder OCDS API | 26 February 2015 | `publishedFrom=2015-02-26..28` returns 38 releases with `tender.datePublished` 2015-02-27; a `2015-01-01..02-25` window also returns 100+ releases (measured) |
| Contracts Finder archive | 11 Feb 2011 – 25 Feb 2015 | SQLite dump of the closed Business Link site, OGL v3 (https://www.data.gov.uk/dataset/97c75a0c-dd9b-42f9-969c-5e667d8c80f1/contracts-finder-archive-2011-to-2015) |
| TED (UK) | 1993 – 31 Dec 2020 | tender-db's TED archive starts in 1993 (docs/research/ted-access-channels.md); "From 1 January 2021 ... notices for higher value contracts have been published on the UK e-notification service Find a Tender Service (FTS)" (https://www.gov.wales/wppn-0320-post-eu-transition-public-procurement-including-find-tender-service-fts-html) |

FTS "replaces the role of Tenders Electronic Daily, the Official Journal of the EU (OJEU/TED) for procurements in the UK", and "Procurements on OJEU/TED that were commenced prior to the end of the Transition Period must be concluded on OJEU/TED" (https://www.gov.uk/guidance/public-sector-procurement).
The FTS footer credits "Notices are based on TED XML schema. © European Union, https://ted.europa.eu, 1998–2020" (https://www.find-tender.service.gov.uk/Developer/Documentation).
Pre-2021 UK notices are therefore already in tender-db via TED; the data profile counts ≥3,298 `GBR` organizations and TED's non-ISO `UK` country code in the R2.0.8 era (docs/research/data-profile-2026-08.md).
Because TED-started procedures finished on TED and FTS holds nothing dated before 2021 (table above), notice-level overlap between TED and FTS is not expected; the overlap is at organization level (the same buyers and suppliers under `UK`/`GBR` in TED and `GB` in FTS), which the mention resolver already keys on identifier, not on notice.
Contracts Finder overlaps FTS for 2021 to February 2025: PCR 2015 reg. 110 required publishing "information about the opportunity on Contracts Finder" within 24 hours of advertising elsewhere, and reg. 112 required award information on Contracts Finder (https://www.legislation.gov.uk/uksi/2015/102/regulation/110/2021-01-01; https://www.legislation.gov.uk/uksi/2015/102/regulation/112/2021-01-01).
Contracts Finder releases carry no machine link to FTS: a 100-release page has zero occurrences of `find-tender`, `h6vhtk` or `relatedProcesses` (measured).
Both regulations were revoked on 24 February 2025 by the Procurement Act 2023 (https://www.legislation.gov.uk/uksi/2015/102/regulation/110).
Under the Act "A below-threshold tender notice must be published on the central digital platform before being published elsewhere" (thresholds £12,000 central government, £30,000 others) (https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-below-threshold-contracts-html).
Contracts Finder is consequently shrinking: 137 releases (816,936 bytes) on 3 September 2026 versus daily CSVs of ~1.6 MB in August 2023 and ~0.44 MB in September 2026 (measured; https://ckan.publishing.service.gov.uk/api/action/package_search?q=name:contracts-finder-notices-*).

Contracts Finder access: `GET https://www.contractsfinder.service.gov.uk/Published/Notices/OCDS/Search?publishedFrom&publishedTo&stages&limit&cursor` (limit 1–100, stages `planning,tender,award,implementation`), `Published/OCDS/Release/{id}`, `Published/OCDS/Record/{ocid}`, and daily CSV `Harvester/Notices/Data/CSV/{yyyy}/{mm}/{dd}` (https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Published-Notice-OCDS-Search; https://www.contractsfinder.service.gov.uk/apidocumentation/home).
Those read endpoints need no token — OAuth2 client credentials apply only to the publishing endpoints (measured; https://www.contractsfinder.service.gov.uk/apidocumentation/home).
Its docs say too many requests yield a 403 and a 5-minute wait; what I received was a 429 with the same 58-byte body as FTS (https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Published-Notice-OCDS-Search; measured).
Contracts Finder's ocid prefix is `ocds-b5fd17` with GUID-based process ids, and its OCP registry entry spans Nov 2016 – Sep 2026 by release date with 594,095 tenders and 439,536 awards (measured; https://data.open-contracting.org/en/publication/128).

## 4. Notice format

Each release is one notice: the release `id` is the notice id (`083685-2026`), the XML zip for 3 September holds 436 files and the API returned 436 releases for that day (docs §1; measured).
Release keys seen in 1,735 releases: `id`, `tag`, `date`, `ocid`, `initiationType` ("tender"), `language`, `parties`, `tender` (all 1,735); `buyer` 1,693; `awards` 1,119; `contracts` 831; `buyerID` 715; `bids` 317; `planning` 219; `description` 126; `links` 79; `relatedProcesses` 62 (measured).
All releases use the ocid prefix `ocds-h6vhtk-` (1,735 of 1,735), which the docs define as `ocds-h6vhtk-hhhhhh` with a zero-padded hex local id (measured; https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages).
The prefix follows OCDS's rule of a registered publisher prefix plus a local identifier (https://standard.open-contracting.org/latest/en/schema/identifiers/).

| Tag (4 weekdays, 1,735 releases) | Count |
|---|---|
| award / contract (always together) | 1,009 / 1,009 |
| tender | 306 |
| planning | 165 |
| tenderUpdate / awardUpdate / contractUpdate | 81 / 81 / 81 |
| planningUpdate | 42 |
| tenderCancellation | 29 |
| contractTermination | 11 |
| contractAmendment | 9 |
| implementation | 2 |

All twelve values are in the OCDS `releaseTag` codelist (https://standard.open-contracting.org/latest/en/schema/codelists/).
Updates and amendments are new releases under the same ocid: the process `ocds-h6vhtk-0510f8` has 12 releases (planning → tender → ten award/contract releases), and `ocdsReleasePackages/ocds-h6vhtk-06a958` returned 5 (measured).
An update release carries only its delta: `083685-2026` (tags award, contract) has `tender` = `{id, legalBasis, amendments, title, documents}` and `awards[0]` = `{id, amendments}` with no value, supplier or date (measured).
The parser must therefore merge releases per ocid itself; fetching a record package per process is one request each and is priced out by the rate limit (§1, §2).

Legal basis distinguishes the two regimes: `tender.legalBasis` is `{"scheme":"UKPGA","id":"2023/54"}` for Procurement Act 2023 notices and `{"scheme":"CELEX","id":"32014L0024"}` (or `32014L0025`) for PCR 2015 notices (measured).
Year-end pages show the switch: 2021–2024 100 % CELEX; 31 Dec 2025 72 UKPGA vs 23 CELEX; 3 Sep 2026 page 1 92 vs 8 (measured).
"On the 24th February 2025, the rules that shape how public bodies buy goods and services changed", and "whilst we are in the transition period, this will include all the notices that exist under previous regimes" (https://www.gov.uk/government/publications/procurement-act-2023-short-guides/contracting-authorities-an-overview-of-the-central-digital-platform-the-enhanced-find-a-tender-service-html).
The Act's notices are numbered UK1 pipeline, UK2 preliminary market engagement, UK3 planned procurement, UK4 tender, UK5 transparency, UK6 contract award, UK7 contract details, UK8 contract payment, UK9 contract performance, UK10 contract change, UK11 contract termination, UK12 procurement termination, UK13–UK16 dynamic market, UK17 payments compliance (https://www.gca.gov.uk/news/procurement-act-2023-notices-what-they-mean-and-how-to-use-them-procurement-essentials).
In OCDS the notice number appears as `documents[].noticeType` (a UK-extension field) beside `documentType`, in the section the notice belongs to (https://raw.githubusercontent.com/cabinetoffice/ocds_uk_extension/main/release-schema.json; measured).

| noticeType | documentType | section | release tags seen | Count (382 releases) |
|---|---|---|---|---|
| UK1 | plannedProcurementNotice | planning | planning | 3 |
| UK2 | marketEngagementNotice | planning | planning | 16 |
| UK3 | plannedProcurementNotice | planning | planning | 2 |
| UK4 | tenderNotice | tender | tender, tenderUpdate | 39 |
| UK5 | awardNotice | awards | award+contract | 5 |
| UK6 | awardNotice | awards | award+contract, awardUpdate+contractUpdate | 57 |
| UK7 | contractNotice | contracts | award+contract | 91 |
| UK10 | contractNotice | contracts | contractAmendment | 1 |
| UK15 | awardNotice | tender | award+contract | 3 |

(measured over the 3 September page and the year-end pages; UK8/9/11/12/13/14/16/17 did not occur in the sample.)
The UK extension's codelists add `implementationNotice` (UK9), `contractTerminationNotice`, `tenderCancellationNotice`, `marketEngagementNotice`, party role `removedSupplier`, reserved participation `smeVcse`, and classification schemes `UK_CA_TYPE`, `UK_CA_DEVOLVED_REGULATIONS`, `UKPGA` (https://raw.githubusercontent.com/cabinetoffice/ocds_uk_extension/main/codelists/+documentType.csv and siblings).
Its fields include `tender.aboveThreshold`, `tender.specialRegime`, `Value.amountGross`, `Period.durationInMonths`, `Document.noticeType`, `Organization.details.vcse`, contract `terminationRationaleClassifications` and `implementation.performanceFailures` (https://raw.githubusercontent.com/cabinetoffice/ocds_uk_extension/main/release-schema.json).
The extension changed on 2025-02-22, 2025-04-30, 2025-09-01 and 2026-01-13 (https://raw.githubusercontent.com/cabinetoffice/ocds_uk_extension/main/README.md).
Packages declare ten extensions: the OCDS EU profile, amendment rationale classifications, budget breakdown, contract completion, documentation, pagination, suitability, Links, the Cabinet Office UK extension and performance failures (measured package header).
The EU profile "describes how to express, in OCDS, the information in Tenders Electronic Daily (TED) notices" (https://standard.open-contracting.org/profiles/eu/latest/en/).
Language: `language` is `en` in all 2,017 releases inspected; Welsh exists only as a site UI option (measured; https://www.find-tender.service.gov.uk/Developer/Documentation).
Lots: `tender.lots` is present in 1,676 of 1,735 releases (2,210 lots over four days); in the 3 September page 31 of 118 lots carry a `value`, 42 a `contractPeriod`, all a `status`; awards point to lots via `relatedLots` (measured).
Amounts are OCDS `{amount, currency}` plus the UK `amountGross`, e.g. an award `{"amountGross":660000.0,"amount":660000.0,"currency":"GBP"}`; `tender.value` is present in 515 of 1,735 releases, `award.value` in 48 of 100 releases on the sample page (measured).
Dates are ISO 8601 with offset (`2026-09-01T00:00:00+01:00`, `2021-01-02T17:00:05Z`); periods use `startDate`/`endDate` (`tenderPeriod`, `enquiryPeriod`, `awardPeriod`, award `contractPeriod`, contract `period`, `dateSigned`) (measured).
`procurementMethodDetails` is free text, including "Below threshold - open competition", "Below threshold - without competition" and "Competitive flexible procedure" (measured).
Party `id` is `<scheme>-<id>` (`GB-PPON-PBZB-4962-TVLR`); roles seen: supplier 5,578; buyer 1,800; tenderer 829; reviewBody 223; procuringEntity 153; processContactPoint 61; centralPurchasingBody 31; mediationBody 28; reviewContactPoint 26; removedSupplier 1 (measured).
Addresses carry `country: "GB"` and NUTS-style `region` codes such as `UKD72` (measured).

## 5. Identifier schemes on parties

| `identifier.scheme` (4 weekdays) | All parties | Buyers | Suppliers + tenderers |
|---|---|---|---|
| GB-PPON | 5,606 | 1,308 | 4,402 |
| GB-COH | 1,523 | 156 | 1,726 |
| (none) | 738 | 225 | 263 |
| GB-NHS | 71 | 69 | 1 |
| GB-CHC | 26 | 22 | 5 |
| GB-UKPRN | 20 | 13 | 7 |
| GB-MPR | 8 | 6 | 2 |
| GB-NIC / GB-SC | 1 / 1 | 1 / 0 | 0 / 1 |

(measured; a party with several roles is counted once per role.)
`additionalIdentifiers` add GB-PPON 1,196, GB-UKPRN 18, GB-CHC 13, GB-SC 4, GB-MPR 2, GB-COH 1, GB-NHS 1; the dominant pairing is primary GB-COH with an additional GB-PPON (110 parties in 382 releases) (measured).
No `GB-VAT` identifier occurred anywhere in the sample (measured).
9.2 % of parties have no identifier today; before the Act it was nearly all of them — 0 of 70 parties on 31 Dec 2021, 3 of 163 in 2022, 21 of 201 in 2023, 5 of 98 in 2024 (measured year-end pages; https://data.open-contracting.org/en/publication/41 notes "Organization identifiers are not provided for most buyers and suppliers").
The PPON is "the unique identifier for the organisation. It will appear in every notice and is the way that information about that organisation is joined together digitally", issued on registration on Find a Tender (https://www.gov.uk/government/publications/procurement-act-2023-short-guides/contracting-authorities-an-overview-of-the-central-digital-platform-the-enhanced-find-a-tender-service-html).
Every one of 235 PPON values matches `^[A-Z]{4}-[0-9]{4}-[A-Z]{4}$` (measured).
`GB-PPON` is not registered in org-id.guide, whose list has 25 `GB-` codes without it (https://org-id.guide/download.json; https://org-id.guide/list/GB-PPON returns 404).
The other schemes are registered: GB-COH Companies House, GB-CHC Charity Commission (England and Wales), GB-SC Scottish Charity Register, GB-NIC Charity Commission for Northern Ireland, GB-NHS NHS ODS codes, GB-UKPRN UK Register of Learning Providers, GB-MPR Mutuals Public Register, GB-SRS Cabinet Office Supplier Registration Service, GB-LAE Local Authorities for England, GB-GOR Government Organisation Register (retired 2021) (https://org-id.guide/download.json).
gov.uk guidance names Companies House, the charity commissions, NHS ODS and UKPRN as the registers identifiers are sourced from (https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-procure-phase/guidance-central-digital-platform-and-publication-of-information-html).
Companies House numbers are 8 characters, digits or a letter prefix plus digits, and "Where a company number contains a prefix e.g. SC, NI, FC, the prefix should be provided in upper case", with prefixes including SC (Scotland), NI (Northern Ireland), OC/SO/NC (LLPs), LP/SL/NL (limited partnerships), FC (overseas), AC/SA/NA (assurance) (https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/809682/uniformResourceIdentifiersCustomerGuide.pdf; https://forum.companieshouse.gov.uk/t/company-number-format-and-suffixed-letters/4922).
Measured GB-COH shapes: 133 eight-digit, 6 `SC`, 3 `RC`, 3 `IP`, 2 `NI`, 1 `OC`, and 4 unpadded values of 6–7 digits, so zero-padding to 8 is needed before keying (measured).
`RC` (royal charter) and `IP` (industrial and provident) are issued by other registrars but appear under GB-COH in FTS data (measured; https://forum.companieshouse.gov.uk/t/company-number-format-and-suffixed-letters/4922).

Mapping onto tender-db: `organizations.country` is the register's jurisdiction, so every scheme above maps to `GB` (crates/store/src/canonical.rs; CONTEXT.md).
`identifier_kind` is `vat | national`; all FTS identifiers are register ids, so `national`, with the OCDS scheme kept in `organization_mentions.scheme` and the raw value in `raw_identifier` (crates/store/src/canonical.rs).
`canonical_key` today keys only all-digit bodies and has no GB arm, so PPONs and prefixed Companies House numbers would fall to E0 exact match (crates/ingest/src/crosswalk.rs).
Unit 2 needs a GB arm: normalise GB-COH to 8 uppercase characters (zero-pad digit-only values), key GB-PPON as its own series without cross-scheme merging, and record the COH↔PPON pairs from `additionalIdentifiers` as E2 evidence.
A VAT↔Companies House crosswalk is unnecessary because VAT numbers do not occur (measured).

## 6. Currency

| Currency (amount objects, 4 weekdays) | Count | Share |
|---|---|---|
| GBP | 2,147 | 98.9 % |
| AED | 22 | 1.0 % |
| USD | 2 | 0.1 % |

(measured; the 2021–2025 year-end pages were GBP only.)
GBP is already in tender-db's rate series; AED and USD go through `canonical_currency` like any TED currency (.scratch/tender-db/issues/342-sources-beyond-ted-and-doe.md).

## 7. Volume

| Day (2026) | Releases | Pages | Bytes | Bytes/release | Unique ocids |
|---|---|---|---|---|---|
| Tue 1 Sep | 457 | 5 | 5,703,699 | 12,480 | 433 |
| Wed 2 Sep | 371 | 4 | 4,404,333 | 11,871 | 352 |
| Thu 3 Sep | 436 | 5 | 4,461,883 | 10,233 | 398 |
| Fri 4 Sep | 471 | 5 | 10,271,752 | 21,808 | 438 |
| Sat 5 Sep | 7 | 1 | 87,569 | 12,510 | — |
| Sun 6 Sep | 1 | 1 | 18,974 | 18,974 | — |

(measured; the four weekdays total 1,735 releases and 24,841,667 bytes, 14,317 bytes per release uncompressed JSON.)

| Year | Highest notice id on 31 Dec | Notices published | XML zip sample day | Files |
|---|---|---|---|---|
| 2021 | 032542-2021 | 32,542 | 2021-06-16 | 141 |
| 2022 | 036737-2022 | 36,737 | 2022-06-15 | 127 |
| 2023 | 038048-2023 | 38,048 | 2023-06-14 | 161 |
| 2024 | 041642-2024 | 41,642 | 2024-06-12 | 167 |
| 2025 | 086608-2025 | 86,608 | 2025-06-11 | 332 |
| 2026 (to 6 Sep) | 084165-2026 | 84,165 (≈123,400 full-year pace) | 2026-06-10 | 466 |

(measured: notice ids are a zero-padded per-year sequence per the docs, so the year-end maximum is the year's count; zip file counts from the data.gov.uk daily archives.)
Volume roughly doubled after the Act went live on 24 February 2025: 135 notices on 26 February 2025 versus 332 on 11 June 2025 (measured zips).
Backfill: 319,742 releases × 14,317 bytes ≈ 4.6 GB uncompressed JSON, ≈ 0.5 GB gzipped (OCP's flattened all-time bulk is 214 MB compressed) (measured; https://data.open-contracting.org/en/publication/41).
Steady state: ≈123,000 releases/year ≈ 1.8 GB/year raw, ≈ 5 MB per weekday.
Storage: the DB is ~604 GiB on a 1.7 TB volume with ~846 GiB free (task brief); the TED archive alone is 176 GiB (docs/research/storage-lifecycle-2026-08.md).
The raw FTS backfill is under 1 % of free space; at TED's all-in cost of ~45 KB per notice (604 GiB / 14.24 M notices, crates/store/src/canonical.rs) the projected DB growth is ~14 GB for the backfill and ~5.5 GB per year.

## 8. Recommendation

Go: build the fetcher (unit 2).
The source is open (OGL v3), unauthenticated, documented, English-only, GBP to 99 %, one publisher prefix, one notice per release, ~5 MB a day, and five and a half years of history that TED does not have (§1–§7).

Three biggest risks:
1. Rate limiting is opaque and tighter than the docs suggest — 429s at every cadence tested from this egress — so the backfill is a one-to-two-day resumable job, and the fetcher must persist (window, cursor) progress and honour `Retry-After` on every call; the data.gov.uk XML zips are a rate-limit-free cross-check for counts (§1, §2).
2. Organization identity is weak in the history and FTS-local in the present: ~95 % of pre-2025 parties have no identifier, 9 % still have none, the PPON is not in org-id.guide, and Companies House values arrive unpadded and prefixed, so the crosswalk needs a GB arm and the matcher must not expect TED-grade identifier rates for 2021–2024 (§5).
3. Two regimes and delta releases: PCR 2015 (CELEX) and Procurement Act (UKPGA) notices coexist, the UK extension changed four times in a year, update releases carry only changed fields so the projection must merge per ocid, and Contracts Finder duplicates FTS for 2021–2025 with no machine link, which argues for FTS-only in unit 2 and Contracts Finder as a later, deduplicated unit (§3, §4).

Fetches that failed or were inconclusive: `https://www.gov.uk/guidance/find-a-tender-service-fts-notices-under-the-procurement-act-2023` (404, guessed URL), `https://www.find-tender.service.gov.uk/apidocumentation/api-how-to-guide` (404), `https://org-id.guide/list/GB-PPON` (404, consistent with the download list), `ocdsRecordPackages?updatedFrom=...` (400), `limit=500` and 31/41-day windows (429 on every attempt), and `https://www.legislation.gov.uk/uksi/2015/102/regulation/110` current version (revoked text; the 2021-01-01 point-in-time version was used instead).
Two gov.uk PDFs (the Companies House URI guide and the February 2025 Find a Tender factsheet) and the Cabinet Office PPON FAQ PDF could not be read by WebFetch and were extracted locally with pypdf.
