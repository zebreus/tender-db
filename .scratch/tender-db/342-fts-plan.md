# 342 unit 2 — Find a Tender (FTS) source: implementation plan

Written 2026-09-07 from `docs/research/uk-fts.md` (§2, §4–§6, §8), the unit-2 shape in
`342-sources-beyond-ted-and-doe.md:93-104`, and six subsystem maps. Three commits, dry-first. Line
refs are to the tree at `0b71ea7`. Everything gates through `ops/check.sh` (read `GATE-EXIT=`).

## 0. Data flow

API page (`GET {BASE}/ocdsReleasePackages?updatedFrom=&updatedTo=`, ≤100 releases, `links.next`)
→ staged verbatim under `<archive>/fts/<kind>/<period>.pages/` (resumable) → ONE zip
`fts/<kind>/<period>.zip`, one member `<release id>.json` per release → sha256 + `fetches` row
(`record_fetch`, as today) → `process::process(.., "fts", kind, period)` walks the zip (PK magic,
package.rs:166) → `profile::dispatch_with` JSON arm → `notices` row `UNIQUE("fts", publication_id,
content_hash)` → `process::parse_payload` → `fts::parse_payload` → `store::Parsed` (eForms field ids)
→ `project.rs` standard (non-legacy) path; `BT-04-notice` = ocid groups releases into one Tender.

## 1. Commit (a): `fts` in the fetch registry (fetcher + probe + backfill)

New `crates/ingest/src/fts/mod.rs` (+ `pub mod fts;` in `crates/ingest/src/lib.rs:7-23`):

```rust
pub const BASE: &str = "https://www.find-tender.service.gov.uk/api/1.0";
pub const FIRST_MONTH: (u16, u8) = (2021, 1);        // API holds nothing before 2021-01-02
pub const OVERLAP_SECS: i64 = 7200; pub const PAGE_PAUSE_SECS: u64 = 12; // §1: 15 s cadence saw no 429
pub struct Window { pub day: (u16, u8, u8), pub url: String }
pub fn window_url(base: &str, day: (u16, u8, u8), overlap_secs: i64) -> String;
    // `?limit=100&updatedFrom=YYYY-MM-DDTHH:MM:SS&updatedTo=YYYY-MM-DDT23:59:59` — UK wall-clock
    // strings exactly as the server interprets them (§2); NO tz conversion here.
pub fn day(base: &str, day: (u16, u8, u8)) -> fetch::Target;      // kind "daily", period YYYY-MM-DD,
    // url = window_url(day, OVERLAP_SECS), rel_path "fts/daily/YYYY-MM-DD.zip"
pub fn monthly(base: &str, (y, m): (u16, u8)) -> fetch::Target;   // kind "monthly", period YYYY-MM,
    // url = window_url(first day, 0), rel_path "fts/monthly/YYYY-MM.zip"
pub fn windows(base: &str, target: &fetch::Target) -> Vec<Window>;
    // daily → 1 window with overlap; monthly → one contiguous 1-day window per civil day
pub fn months_through(end: (u16, u8)) -> Vec<(u16, u8)>;   // FIRST_MONTH..=end (doe.rs:46 shape)
pub fn uk_offset(unix: i64) -> i64;  pub fn uk_civil_date(unix: i64) -> (u16, u8, u8); // GMT/BST, EU instants
pub fn release_id(release: &serde_json::Value) -> Option<&str>;
pub fn member_bytes(header: &serde_json::Value, release: &serde_json::Value) -> Vec<u8>;
    // single-release OCDS package {version, extensions, publisher, license, publicationPolicy,
    // releases:[release]}; `uri`/`links`/`publishedDate` DROPPED (page-specific). serde_json's
    // default BTreeMap order ⇒ byte-deterministic ⇒ the 2 h overlap re-yields the SAME
    // content_hash and dedups on identity. Never enable serde_json `preserve_order`.
```

`crates/ingest/src/fetch.rs`:
- `classify_status` :337-349: add `SERVICE_UNAVAILABLE` to the `Throttled` arm (FTS docs: 503 = 429).
- Extract the tail of `fetch()` (:114-142: hash-compare / `-vN` / rename / `record_fetch`) into
  `async fn land(db, target, existing: Option<&store::Fetch>, final_path, bytes: i64, sha256) -> Result<Outcome>`
  so both fetchers share the immutability rules.
- `pub(crate) async fn get_bytes(client, url) -> Result<Vec<u8>, Error>`: the `download()` retry
  loop (:281-315) without a file — 429/408/503 → `Retry-After` (int secs, cap 120) × ≤5, other 4xx
  permanent, 5xx/transport ×3.
- `pub async fn fetch_fts(db, client, archive_root, target: &Target, refetch: bool,
  on_progress: impl FnMut(&str /*day*/, usize /*pages*/, usize /*releases*/)) -> Result<Outcome, Error>`:
  1. `latest_fetch` + `!refetch` → `Unchanged` without HTTP (as `fetch()` :97-100).
  2. staging dir `<archive>/<rel_path minus .zip>.pages/` with `cursor.json` =
     `{"day":"YYYY-MM-DD","page":N,"next":url|null,"done":[...]}`; pages saved verbatim as
     `<day>-p<NNN>.json` BEFORE the cursor advances. Resume = read `cursor.json`, continue at `next`.
  3. per window: `get_bytes(window.url)`, parse, save, follow `links.next` until absent,
     `sleep(PAGE_PAUSE_SECS)` between pages; an `Err` mid-window leaves staging intact, job fails.
  4. assemble `zip::ZipWriter` (deflate) to `<final>.part`: members `<release id>.json` via
     `member_bytes`, sorted by id, first occurrence wins on a duplicate id; an empty window still
     yields a 0-member zip. 5. sha256 → `land(...)`; remove the staging dir on success.
- `fetch()` :90: guard `if target.source == "fts" { return Err(Error::Unsupported("paged source")) }`
  (new variant) so no caller can archive a raw page under the zip name.
- `pub async fn probe_fts_daily(db, client, archive_root, base, end: (u16,u8,u8),
  on_day: impl FnMut(&str, &Outcome)) -> Result<Vec<(String, Outcome)>, Error>`: clone of
  `probe_doe_daily` (:200-221) over `latest_fts_day(db)` (= `latest_fetch_period_max("fts","daily","")`),
  `fetch_fts(.., refetch=false)`; nothing registered → only `end`.
- `register_archive` :425 → `["ted", "doe", "fts"]`; `.pages` dirs are already skipped (:439).

`crates/app/src/supervisor.rs`:
- `Spec` :211-218: add `ProbeFts`; arm beside `ProbeDoe` :2851, body in `Box::pin(async move {..}).await`
  (CLAUDE.md stack rule): `end = fts::uk_civil_date(now_unix() - 86_400)`, progress `p.package = day`.
- `Spec::Fetch` arm :2632-2643: `if target.source == "fts" { fetch::fetch_fts(..) } else { fetch::fetch(..) }`;
  `RehashProbe` :2692: `source == "fts"` → finding `{outcome:"exempt"}`, `skipped += 1`, continue.
- `Supervisor.fts_base` (:130; `new` :946-960 = `fts::BASE`); `build_target` :9632-9655 gains
  `fts_base` + arms `("fts","daily") => fts::day(.., parse_ymd(period)?)`, `("fts","monthly")`;
  update callers :2636, :2694 and the test at :10106.
- `fetch_parts` :1949-1953: `Some("fts") => "fts"`. `enqueue_backfill` :1893-1916: arm `"fts"` =
  `range` → `months_between`, else `fts::months_through(current month)`; replace the `src` fallback
  :1916 with a 3-way match. Fan-out unchanged (monthly `Fetch` jobs + `Process` + `Project`).
- `enqueue_daily` :9493-9503: always push `("probe","fts daily (probe)",Spec::ProbeFts)` +
  `("process","fts daily (all)",Process{fts,daily,None})` (FTS publishes 7 days). `JobRequest.source` doc :841-846.

CLI: `bin/fetch.rs` `Source::Fts` (:20, :50-53), `--day`/`--month` → `fetch_fts`, `--backfill` = `months_through`; `bin/process.rs:38` adds `"fts"`.

Tests (a): `fts/mod.rs` unit — `urls_match_documented_patterns` (window string, overlap boundary
`2026-09-02T22:00:00`, rel_path, monthly window count, month rollover), `uk_offset` at both DST edges,
`member_bytes_is_deterministic_and_drops_page_fields`; `fetch.rs` unit — 503 → Throttled.
`tests/fetch.rs`: `fts_server(pages, throttle_once)` axum fixture on `/api/1.0/ocdsReleasePackages`
keyed by `cursor`, `links.next` on non-final pages, one `429 Retry-After: 1` when asked;
`fts_window_follows_links_next_into_one_zip` (members = ids, `Fetched` then `Unchanged`, staging gone),
`fts_window_resumes_from_staged_pages` (pre-staged p001 + cursor; zero hits on page 1),
`fts_walk_forward_catches_up_a_multi_day_gap`. `tests/register_archive.rs`: `fts/daily` +
`fts/monthly` register, `.pages` ignored. Supervisor `targets_are_built_per_source_and_kind`.

## 2. Commit (b): profile `fts:ocds-1.1`, release stored as the notice body

`crates/ingest/src/profile.rs`:
- `dispatch_with` :148-181: after the text-era check (:152) and BEFORE `from_utf8`/roxmltree (:156-167):
  `if first non-whitespace byte == b'{' { return one(dispatch_fts_ocds(member_path, bytes)); }`.
- `fn dispatch_fts_ocds(member_path, bytes) -> Record`: `serde_json::from_slice::<Value>` else
  quarantine `unparsable-json`; `version` string else `missing-ocds-version`; `releases.len() == 1`
  else `ocds-release-count`; `releases[0].id` string else `missing-publication-id` (profile `Some`).
  Notice: `publication_id = id` verbatim (`083685-2026`), `profile = format!("fts:ocds-{version}")`,
  `declared_version = Some(version)`, `content_hash = sha256(bytes)`, `span: None`.
- Header table :11-16 gains the row; `data_quality.rs:942 era_of`: `p if p.starts_with("fts:") => "FTS OCDS"`.
- `process.rs:18 parse_payload` unchanged in (b): `fts:` falls to `Parse::Pending` = identity-only rows
  (`parse_state='pending'`), the dry-first rung. `original_lang = ENG` lands with (c): it is a
  projection field (`tender_versions.original_lang`) fed by `BT-702(a)-notice`.

Fixtures `crates/ingest/tests/fixtures/fts/`: cut from the recorded page `scratchpad/fts_day1.json`
(3 Sep 2026 p1, 100 releases — copy it out of the session scratchpad FIRST) with `fts::member_bytes`:
a UK4 tender with lots, a UK6 award+contract with suppliers/value, a UK2 planning, the UK15 delta
`083685-2026`, one 2022 CELEX award recorded live (`ocdsReleasePackages/<id>`), and a 3-release trimmed
page `pages/2026-09-03-p001.json` for the assembly test. README section + count line (README.md:15).

Tests (b): `profile.rs` unit `fts_release_member_dispatches_as_a_notice` (id, profile, version),
`fts_member_without_version_quarantines`, `broken_json_quarantines_as_unparsable_json`; `tests/fts.rs`
`fts_zip_package_processes_end_to_end` (fixtures zipped as `fts/daily/2026-09-03.zip`, `record_fetch`,
`process::process(.., "fts", "daily", None)` → `members == notices`, `quarantined == 0`,
`notice_counts_by_profile == [("fts:ocds-1.1", n)]`, `parse_state='pending'`; `parsed` after (c)).

## 3. Commit (c): parser onto the notice model + crosswalk GB arm

New `crates/ingest/src/fts/parse.rs`: `pub fn parse_payload(profile: &str, bytes: &[u8]) -> store::Parse`
(gate `profile == "fts:ocds-1.1"`, else `Pending`); `process.rs:18` arm `profile.starts_with("fts:")`.
Root `Section{id:"PROCEDURE", kind:"Notice", parent:None}`; ordinals per (section, field) from 0;
amounts via `eforms::value::cents`, dates via `eforms::value::timestamp` (date-only → `has_time:false`).
Lenient: unknown keys ignored (the UK extension moved 4× in a year); quarantine only on unparsable
JSON, a missing `ocid`, or an unrepresentable amount (`unrepresentable-value`).

| OCDS (release) | section | field id / value |
|---|---|---|
| `ocid` | PROCEDURE | `BT-04-notice` `Id{value, is_ref:false}` = procedure_key |
| `date` | PROCEDURE | `OPP-012-notice` `Date` (published_at; no dispatch date) |
| `language` (`en`) | PROCEDURE | `BT-702(a)-notice` `Code{code:"en"}` → ENG; `lang:"en"` on every Text |
| first `documents[].noticeType` (tender/awards/contracts/planning), else `tag.join("+")` | PROCEDURE | `OPP-070-notice` `Code` (subtype; free text except X01) |
| `tender.legalBasis {scheme,id}` | PROCEDURE | `BT-01-notice` `Code{list:scheme, code:id}` (stored, not read) |
| `tender.title` / `tender.description` (else release `description`) | PROCEDURE | `BT-21-Procedure` / `BT-24-Procedure` `Text` |
| `tender.value.amount` (+`currency`) | PROCEDURE | `BT-27-Procedure` `Amount` + sibling `TED-VAL_TOTAL_TAX_BASIS` `Code{code:"excl"}`; `amountGross` only → Amount from it + `incl` |
| `tender.tenderPeriod.endDate` / `enquiryPeriod.endDate` | PROCEDURE | `BT-131(d)-Procedure` / `BT-13(d)-Procedure` `Date` |
| `tender.contractPeriod.startDate/endDate` | PROCEDURE | `BT-536-Procedure` / `BT-537-Procedure` |
| `tender.items[].classification` (CPV) first / rest + `additionalClassifications` | PROCEDURE or item's `relatedLot` | `BT-262-*` / `BT-263-*` `Classification{scheme:"cpv"}` |
| `tender.deliveryAddresses[].region` (`UKD72`) | PROCEDURE | `BT-5071-Procedure` `Classification{scheme:"nuts"}` |
| `tender.lots[]` | `Section{id: lot.id, kind:"Lot", parent:PROCEDURE}` | `BT-21-Lot`, `BT-24-Lot`, `BT-27-Lot` (+tax basis), `BT-536/537-Lot` |
| `parties[]` (every party) | `Section{id:"ORG-"+party.id, kind:"Organization", parent:PROCEDURE}`, never nested | `BT-500-Organization-Company` `Text{lang:"en"}` = name; `BT-514-Organization-Company` `Code` = `address.country` or `"GB"`; `BT-501-Organization-Company` `Id{scheme:Some(identifier.scheme), value:"<scheme>-<id>", is_ref:false}` (D6); `additionalIdentifiers[]` → further `BT-501` rows (ordinal ≥1; unread by `mentions()`, kept for the E2 follow-up) |
| roles `buyer`/`procuringEntity` | PROCEDURE | `OPT-300-Procedure-Buyer` `Id{value:"ORG-…", is_ref:true}` (role `Procedure-Buyer`, matches `LIKE '%uyer%'`) |
| `centralPurchasingBody` / `reviewBody` / `mediationBody` | PROCEDURE | `OPT-300-Procedure-CPB` / `OPT-301-Lot-ReviewOrg` / `OPT-301-Lot-Mediator` (is_ref) |
| `supplier`/`tenderer`/`removedSupplier` | — | no OPT-300 ref; resolved through the results graph below |
| `awards[]` with suppliers or value or status (delta-only `{id, amendments}` → nothing) | `RES-<award.id>[-<lot>]` `LotResult` per `relatedLots` entry (none → one, Tender-scoped) | `BT-142-LotResult` `Code` `selec-w` (active/pending) or `clos-nw` (unsuccessful/cancelled); `BT-13713-LotResult` `Id` = lot; `OPT-320-LotResult` `Id{TEN-…, is_ref}` per supplier; `OPT-315-LotResult` `Id{CON-…, is_ref}` |
| each `award.suppliers[n]` | `TEN-<award.id>-<n>` `LotTender`; `TPA-<award.id>-<n>` `TenderingParty` | `BT-720-Tender` `Amount` = `award.value` on n=0 ONLY (the fold sums winning bids); `BT-13714-Tender` `Id` = lot; `OPT-310-Tender` `Id{TPA, is_ref}`; `OPT-300-Tenderer` `Id{ORG-<supplier.id>, is_ref}` |
| `contracts[]` | `CON-<contract.id>` `SettledContract` | `BT-150-Contract` `Id`; `BT-145-Contract` `Date` = `dateSigned`; `BT-1451-Contract` `Date` = its award's `date`; `BT-3202-Contract` `Id{TEN-… of awardID, is_ref}` |
| `bids.statistics[]` | sub-section of the LotResult | `BT-759` `Number` + `BT-760` `Code` |
| `relatedProcesses[]`, `planning.budget`, `implementation`, `amendments[]` text | — | not mapped in unit 2 (OPP-090 needs 8-digit ids joined per source; rare: 62/1,735) |

Crosswalk `crates/ingest/src/crosswalk.rs` `canonical_key` (:68; arm match :142-343): new `"GB"` arm for
`kind == "national"` bodies (already alnum-uppercased): `GBCOH<x>`, `x` = `^\d{8}$` or `^[A-Z]{2}\d{6}$`
⇒ `e1("GB:coh", x)`; 6–7 digits ⇒ zero-pad ⇒ `e2("GB:coh")`; `GBPPON<12 alnum>` ⇒ `e1("GB:ppon")`
(own series, never cross-scheme); bare `^\d{8}$`/`^[A-Z]{2}\d{6}$` under GB (TED-era rows) ⇒
`e2("GB:coh")`; any other `GB<SCHEME>…` ⇒ `None` (E0 exact). Module tests (:346+):
`gb_coh_pads_to_eight_at_e2_and_keys_full_forms_at_e1`, `gb_ppon_is_its_own_series`,
`gb_other_registers_key_nothing`; `project.rs`: `normalise_identifier("GB-COH-SC123456", Some("GB"))` → `(GB, national, "GBCOHSC123456")`.

Tests (c): `tests/fts.rs` per fixture — `uk4_tender_maps_title_deadline_buyer_and_lots`,
`uk6_award_maps_winner_value_and_contract` (winner org id, `BT-720` cents, `excl`),
`celex_2022_award_has_tag_subtype_and_no_notice_type`, `delta_release_emits_no_result_round`,
`every_fts_fixture_parses`; a `project_fold_source.rs`-style fold test: two releases under one ocid
→ one Tender whose head carries the later award's winner and a resolved buyer role.

## 3b. Corrections to §3's crosswalk, from the fixtures (2026-09-08)

§3's table was written from the API docs and the OCDS schema. Five member fixtures cut from the live
3 Sep 2026 page say it is wrong in six places. Fix the table by these, not the other way round.

1. **`tender.contractPeriod` does not exist** — not in any member fixture nor either page. Contract
   periods live only at `tender.lots[].contractPeriod`, `awards[].contractPeriod` and
   `contracts[].period` (the last with a third sub-key `maxExtentDate`). So `BT-536/537-Procedure`
   has no source; keep `BT-536/537-Lot`.
2. **`tender.deliveryAddresses` does not exist** — delivery addresses are one level deeper, at
   `tender.items[].deliveryAddresses[]` (and `awards[].items[].deliveryAddresses[]`), each
   `{country?, countryName?, region?}` with all three optional. `BT-5071` must be read from the item,
   and its scope is the item's `relatedLot` when it has one.
3. **`tender.items[].classification` (singular) does not exist** — items carry only
   `additionalClassifications[]` (`{scheme:"CPV", id, description}`). The singular form does appear,
   but at `tender.classification`, and only in the page fixtures. So "first CPV = `BT-262`, rest =
   `BT-263`" cannot key off `classification` vs `additionalClassifications`; take the item's first
   additional as `BT-262-*` and the remainder as `BT-263-*`, and read `tender.classification` as
   `BT-262-Procedure` when present.
4. **`documents[]` is never release-level, and `noticeType` is not a tender/awards/contracts/planning
   word.** Exactly ONE document per release carries `noticeType`, and its value is a UK form code —
   `UK1`, `UK2`, `UK4`, `UK5`, `UK6`, `UK7`, `UK10`, `UK12`, `UK15` observed. That document hangs off
   `tender.documents`, `planning.documents`, `awards[].documents` or `contracts[].documents`
   depending on archetype, so the subtype read must search all four. Do NOT key off `documentType`:
   `awardNotice` maps to both UK6 (award) and UK15 (dynamic-market modification), and
   `contractNotice` here means a contract CHANGE (UK10), not a call for competition.
5. **`tender.value.amount` can be absent while `amountGross` is present** (083645 has
   `{amountGross: 0, currency:"GBP"}` and no `amount`) — the planned `amountGross`-only fallback is
   load-bearing, not defensive. Amounts also arrive as JSON floats (`32800.0` in 083650) alongside
   integers elsewhere, so the reader must accept both.
6. **Explicit JSON nulls, not omissions**, at `tender.lots[].description` (083645) and
   `contracts[].statusDetails` (_noid). A non-optional `String` there panics on real data.

Also confirmed as planned: `awards[]` really can be `{id, amendments}` and nothing else — 083685
carries **34** such skeletons with sparse ids — so the "delta-only → emit nothing" arm is the common
case for UK15, not an edge. Party identity is uniform: `identifier` is always
`{scheme:"GB-PPON", id:"<suffix>"}` with `party.id == "GB-PPON-" + identifier.id`, `legalName` and
`additionalIdentifiers` absent from every member fixture. Roles observed: `buyer`, `supplier`,
`removedSupplier` — none of `procuringEntity`/`centralPurchasingBody`/`reviewBody`/`mediationBody`
appear, so those arms land untested and must stay lenient. `bids.statistics` and `relatedProcesses`
have NO member-fixture coverage at all; `bids` appears once in page p002.

Do not map release-level **`buyerID`**: it is a UK extension field whose value is an empty ARRAY.

### A fetcher follow-up this uncovered

`_noid-2026-09-03-p001-000.json` is release 0 of page p001 with the `id` key simply absent (verified:
the two are deep-equal after deleting `id` from the page copy). `fts::release_id` therefore archived
it under `_noid/`. But the notice number IS in the file — the `noticeType`-bearing document's `id` is
`083674-2026`, which is exactly what the page-level `id` said. **That document's id equals the FTS
notice number in all five fixtures**, so it is a sound second fallback for `release_id` and would have
retired the `_noid/` path. Worth a separate small commit; not in scope for (c).

7. **`procedure_key` = ocid is NOT one Tender per ocid on the utilities register (issue 386 unit 1,
   2026-09-18).** Find a Tender's utilities qualification systems (CELEX 32014L0025) publish every
   participating utility's awards under the REGISTER's ocid — `ocds-h6vhtk-02874c` carried 29 distinct
   buyers on its first page — so §3's "one Tender per ocid" welded Scottish Hydro's and SSE's contracts
   under Anglian Water. The key election departs from §3 here: an FTS ocid whose releases carry two or
   more distinct buyer SETS is split per buyer at the plan's refused-key gate (threshold 2, since the
   buyer key is the publisher's own `GB-PPON` party id, not an org-layer duplicate), and the served
   `procedure_key` of each part reads `refused:<ocid>:<buyer key>` — the same arm issue 369 unit 5 gave
   TED's placeholder keys. A single-buyer chain of any length and a joint procurement repeating one buyer
   set still fold to one Tender under the bare ocid. Failure direction is CONTEXT.md's: a buyer that
   re-registers under a new id splits its own chain; nothing welds.

## 4. FTS-specific decisions

- D1 Package layout: one zip per (kind, period), one member per release `<release id>.json`, each a
  single-release OCDS package (header minus `uri`/`links`/`publishedDate`) — the profile is dispatched
  on the package header, the member is self-describing/OGL-attributed. Rejected: raw pages as members
  with `span` records — `process.rs:271-276` hard-routes `span` to the text parser, and page
  composition shifts under the overlap, breaking `member_path` stability (the reclaim key).
- D2 Kinds: `daily` = live poll (one window, 2 h overlap); `monthly` = backfill package assembled from
  contiguous 1-day windows (§2). Mirrors TED/DÖE so `Process{daily, None}` re-walks only live days.
- D3 Identity: `publication_id` = release `id` verbatim; `content_hash` = sha256 of the member (a
  re-published id with changed content is a second row, as TED versions); overlap and monthly-over-daily
  dedup on identity. `procedure_key` = ocid. No `BT-701`, no `Change`.
- D4 Deltas: OCDS deltas do NOT fit eForms change semantics (`Change` + same `BT-701` REPLACES the
  round, project.rs:3044/3428-3433) but do fit the fact fold: silent fields carry forward per Tender
  (project.rs:3395-3420). A delta folds as an ordinary notice under its ocid; delta-only awards emit no LotResult.
- D5 Subtype = first `documents[].noticeType` (UK1–UK17), fallback the tag set (`award+contract`)
  for CELEX-era notices; `original_lang` = ENG via `BT-702(a)-notice` `en`.
- D6 Identifiers: the `BT-501` value is the published composite `<scheme>-<id>` (FTS's own `party.id`
  shape: `GB-COH-SC123456`, `GB-PPON-PBZB-4962-TVLR`) → `(GB, national, GBCOHSC123456)`, scheme kept on
  the mention. Reason: `normalise_identifier`/`canonical_key` get no scheme, and bare values collide
  across GB registers (COH and UKPRN are both 8 digits).
- D7 Amounts: `amount` (net) → Amount + `TED-VAL_TOTAL_TAX_BASIS=excl`; `amountGross` only → `incl`;
  currency verbatim (GBP/AED/USD), EUR derived at fold time from `currency_rates`.
- D8 Rate limits: 429/408/503 honour `Retry-After` (cap 120 s = FTS's value), 5 attempts, then the
  job fails with staging intact; 12 s pause between pages. Progress = staging dir + `cursor.json` on
  disk (no schema change); the `fetches` row is written only for a complete window.
- D9 Backfill = `POST /admin/jobs {kind:"backfill", source:"fts"[, range:[YYYY-MM,YYYY-MM]]}` from
  2021-01; ~4,700 requests ≈ 11–16 h plus back-offs, resumable per month. Measure `2025-06` first.
  Empty windows register a 0-member zip so `MAX(period)` advances; the probe never refetches.

## 4b. Review corrections to this plan (2026-09-07, adversarial four-lens review of commit (a))

The plan as written had commit (a) push BOTH `fts daily (probe)` and `fts daily (all)` in
`enqueue_daily` (§1) and called the chain "dark-safe". Four independent lenses found the same
defect and it is corrected in the built commit:

- **The process push moves to commit (b).** With no JSON arm in `profile::dispatch_with`, every
  `<id>.json` member of the zip the probe just landed reaches `roxmltree::Document::parse` and is
  written as a `quarantine` row with reason `unparsable-xml` — a few hundred a day, in the bucket
  the dashboard calls "genuinely malformed XML", and nothing in the plan reclaimed them. Only the
  probe ships in (a); `current_packages(source, kind, None)` re-walks every FTS daily on the first
  tick after (b), so nothing is lost by waiting. **The backfill must not be run before (b) either**
  (its `Process{fts, monthly}` tail has the same shape); D9's "measure 2025-06 first" is therefore
  a step-7 action, after (b) is deployed.
- **The walk-forward is capped at `fts::PROBE_DAY_CAP` = 31 days per tick.** An FTS day is 4–5
  paced requests plus any back-off where a DÖE day is one download, and the probe kind cannot be
  cancelled; uncapped, a watermark left far behind would hold the single job runner for hours and
  park `fetch-rates`, `project` and the fold behind it. The watermark advances per landed day, so
  the remainder is the next tick's work.
- **A release without a usable id is archived, not thrown.** Failing the package was deterministic:
  that day would fail every tick and the watermark would never advance past it. Such a release now
  lands under the reserved `_noid/` member prefix, where (b)'s profile layer quarantines it as a
  missing publication id with the bytes intact.
- **Staging older than the registered package is debris.** A crash (or a failing `remove_dir_all`)
  between the `fetches` row and the cleanup leaves a `.pages/` whose `done` list would make a later
  refetch skip the windows it must re-walk; `fetch_fts` now discards a staging dir whose cursor
  predates the registered fetch and keeps a newer one (an interrupted refetch) to resume.
- **503 → Throttled is corpus-wide, deliberately.** It is not FTS-scoped: a TED or DÖE 503 now
  waits up to 150 s (480 s with `Retry-After`) instead of failing after 6 s. Three lenses raised it;
  it is HTTP-conformant, bounded by the same cap TED's 429s already carry, and more likely to
  succeed on a transient outage. Accepted and recorded here rather than scoped.
- **Tests sharpened**: the 429 test now times the wait (proving the server's `Retry-After` was
  honoured, not our 15 s fallback), the request-count assertions sum requests instead of counting
  distinct keys (which was vacuous), and the throttle-exhaustion arm, the `_noid/` archive, the
  staging-debris discard and the per-tick cap each got a test.

## 4c. The release no document parser can hold (found 2026-09-07 building commit (b))

Live release `083529-2026` of 3 September 2026 carries
`"maximumLotsBidPerSupplier": 1e9999` — the publisher writing "no limit". That is
valid JSON and `serde_json::Value` REFUSES it ("number out of range"), because a
`Value` number must fit an `f64`. Python's parser accepts it as infinity, which is
why the unit-1 research never saw it, and why the recorded pages were needed.

Parsed as a document, one field failed the page, which failed the day,
deterministically, on every tick: the watermark would never have advanced past
3 September 2026. That is the SAME failure shape the `_noid/` decision was
introduced to prevent one level up, arriving through a different door — which is
the tell that the rule was too narrow. The rule now is:

**Never build a generic document over a publisher's JSON. Name the fields you
read.** `fts::Page` decodes exactly four things — the header's five fields for the
member, `links.next`, a release's `id`, and the package `version` — and every
release stays raw bytes. A value we cannot represent is a value we never looked
at. The fetcher and the profile layer both go through it, and commit (c)'s parser
must too: read named paths, never `Value`.

Two consequences worth recording. The member is now the publisher's own bytes
spliced under a fixed header order, so **risk 5 below is retired** — a member is
what was served, not our re-rendering of it. Against that, the old re-serialisation
normalised key order and whitespace; the archive now depends on the API rendering a
given release identically across the 2 h overlap, which the unit-1 re-check
observed (the same page re-fetched byte for byte). If that ever drifts the symptom
is an extra archived version, not lost data.

## 5. Risks and open questions (default in bold)

1. Label/prefix folds in `normalise_identifier_with` (project.rs:4681, issue-359 vocabulary) or
   `idgate::condemns` may mangle `GBCOH…`. **Pin with the §3 unit test first; if it fails, thread
   `Mention.scheme` into a `normalise_identifier_scoped` rather than changing the value form.**
2. TED-era GB orgs (bare `12345678`) will not E0-match `GBCOH12345678`. **E2 bridge in the GB arm
   (candidate edges only); count the pairs after the 2025-06 run.**
3. COH↔PPON `additionalIdentifiers` as E2 evidence: the parser cannot write `org_candidate_edges`
   (rules there are E3/E4). **Keep the pairs as ordinal ≥1 `BT-501` rows; follow-up issue for an `e2-fts-altid` writer.**
4. UK NUTS codes (`UKD72`) are not in NUTS 2021. **Emit as `nuts`; check the place fact on the
   2025-06 measure; fall back to a Text fact if the code table rejects them.**
5. Member bytes are a re-serialisation, not bytes-as-served (first deviation from architecture.md:130).
   **Accept; document in `fts/mod.rs` and architecture.md; staged raw pages are deleted after assembly.**
6. Limiter variance (§1) can fail a daily probe after 5×120 s. **Accept; next tick resumes from staging; keep FTS out of the issue-222 re-probe loop.**
7. OGL attribution ("Contains public sector information licensed under the Open Government Licence
   v3.0.") must appear where FTS data is exposed. **README + `/v1` source description; no code gate.**

## 6. Ordered task list

1. Preserve the fixture source: copy `scratchpad/fts_day1.json` into `crates/ingest/tests/fixtures/fts/`
   (trimmed page + cut members). AC: files exist, README rows written.
2. `fts/mod.rs` builders + `uk_offset` + `member_bytes` with unit tests. AC: `cargo test -p ingest fts` green.
3. `fetch.rs`: 503 → Throttled, `get_bytes`, `land`, `fetch_fts` with staging/resume, `probe_fts_daily`,
   `register_archive` list, `fetch()` guard. AC: `tests/fetch.rs` fts cases + `register_archive.rs`
   green; existing fetch tests unchanged.
4. Supervisor: `Spec::ProbeFts`, `Fetch` routing, `fts_base`, `build_target`, `fetch_parts`,
   `enqueue_backfill`, `RehashProbe` exemption, `enqueue_daily`; CLI arms. AC: `ops/check.sh`
   `GATE-EXIT=0` (server feature); `targets_are_built_per_source_and_kind` covers fts.
5. Commit (a): stage named files only, `git diff <file>` before each add; push `HEAD:main HEAD:<handover>`.
6. `profile.rs` JSON arm + `dispatch_fts_ocds`, `era_of`, `bin/process.rs` whitelist, fixtures README.
   AC: dispatch unit tests + `fts_zip_package_processes_end_to_end` (pending rows) green.
7. Commit (b). Deploy; `POST /admin/jobs {kind:"fetch", source:"fts", package_kind:"monthly",
   period:"2025-06"}` then `process fts monthly`. AC: notice count within 1 % of the data.gov.uk
   daily zip file counts for June 2025 (§2's limit-free cross-check), `quarantined == 0`, staging gone.
8. `fts/parse.rs` + `parse_payload` arm + per-fixture parse tests + fold test. AC: every fixture
   `Parsed`; fold test shows one Tender per ocid with buyer role and winner.
9. Crosswalk GB arm + `normalise_identifier` pin test. AC: crosswalk tests green, must-NOT panel (:479) untouched.
10. Commit (c). Deploy; `Reparse{profiles:["fts:ocds-1.1"]}` over 2025-06, then `project`. AC:
    `parse_state='parsed'` ≥99 % of the month, `parse_quarantined` listed by reason, tenders ≈ unique
    ocids, `head_value_eur_cents` non-NULL for GBP, identifier share on mentions ≈ §5 (≈90 % post-Act).
11. Backfill `source=fts` (2021-01 → current); watch job_log for `throttled` errors and re-enqueue
    failed months (resume is free). AC: 69 monthly packages; releases ≈ 319,742 + 2026 YTD. The daily
    chain entries ship dark-safe in step 4 (`ProbeFts` on an empty registry fetches only yesterday).
12. Docs in one commit: CONTEXT.md:100-116 (source, OGL attribution, cadence), docs/architecture.md:41,
    49-56,130 (identity + assembled-zip note), docs/operations.md:160-163,292-307,627-628,641-642,
    docs/research/SUMMARY.md entry for uk-fts.md, README.md:15-16, ADR-0003 "Verified" line (no
    notice-level TED↔FTS overlap). Record the 2025-06 numbers on issue 342; set it `ready-for-agent`
    for the Contracts Finder and E2-edge follow-ups.
