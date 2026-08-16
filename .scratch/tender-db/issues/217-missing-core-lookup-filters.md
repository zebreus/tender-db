# 217 — the API can't be queried by the keys real consumers hold: publication_id, org identifier/name, tenderer/bidder

Status: RESOLVED — every capability DEPLOYED & VERIFIED 2026-08-16/17. A (notices `5ca7971`+`7386914`),
A-tenders (`e3825d8`+`3a658ec`+`1e57a04`, below), B (`7eb2495`), B-name (`5fa2c6c`), C (`00f2f48`).
The one optional residual — a `(country, name_norm, id)` composite for companioned name search — waits
for demand, tracked in the B-name section.

**A-tenders DEPLOYED & VERIFIED 2026-08-17 (serving rev `1e57a04`).** `/v1/tenders?publication_id=`
resolves the official number to the TENDER it caused, through any version (a corrigendum's number still
finds the procedure). Seeded FROM off the new deferred `tender_versions_publication (publication_id,
tender_id)` (auto-reindex 711, ~30 s); the seed is EXACT so it doubles as the predicate and takes
precedence over the participation seeds. Two findings en route, both prod-measured:
(1) the tenders flatten repeats the notices one — `source=ted` alongside the number drove
`tenders_source_id` (8M-row slice), a 35 s timeout; fixed the same way (`retain_publication_companions`:
NO `t.`-column predicate rides the seeded SQL, source/kind/date-bounds post-filter in Rust; the
statement seam pins it). (2) the seed SUPPRESSES isolation (issue-212 pattern) — every companion is
bounded by the seed's handful. Final numbers on the MAIN pool: bare 1.8 ms, source-paired 1.7 ms,
kind+winner companioned 1.8 ms, absent 1.6 ms; main list pages 14-20 ms throughout.
Bonus catch (own commit `e3825d8`): `reset_tender_layer`'s bare CREATE TABLE lacked `current_deadline`
(216 added it via migrate() ALTER only) — a from-scratch rebuild would have broken mid-fold; the column
now lives in the schema batch and the recreate DDL, red-lit by the rebuild-index test.

**Finding worth keeping:** the filter is NOT the index seek this issue assumed. The dedup index is
`UNIQUE(source, publication_id, content_hash)`, but the list path's `ORDER BY id LIMIT` pagination makes the
planner drive off the id PK / `notices_source_id` and FILTER by `publication_id`, so a sparse value (≤1
match) walks the table to fill the page — **~10 s measured in prod, WITH or without `source`** (issue-117
class). First deploy (`88d876a`) wrongly kept `source+publication_id` on the main pool, a 10 s walk on the 8
shared REST readers; `5ca7971` isolates every `publication_id` lookup (issue 120), verified: during an 8.5 s
lookup, concurrent `/v1/tenders` stayed at 0.48 s. So the capability exists and is shed-safe, but slow.

**C (bidder) DEPLOYED & VERIFIED 2026-08-16 (serving rev `00f2f48`).** `/v1/tenders?bidder=<org>` /
`/v1/lots?bidder=<org>` — an EXISTS over `tender_version_bid_parties` (role `tenderer`, verified against
prod), mirroring `winner`; isolated with an index-served `reachable()` probe, honoured on tenders/lots,
named ignored elsewhere. Prod: an unknown bidder short-circuits to an empty page; a PRESENT org walks the
isolated pool and **times out a 35 s client — but so does `winner` on the same org** (measured), so bidder
is exactly consistent, not a regression. That shared present-value walk (issue-117 Class B, all of
buyer/winner/bidder) is now its own issue **223**.

**B (org `identifier`) DEPLOYED & VERIFIED 2026-08-16 (serving rev `7eb2495`).**
`/v1/organizations?identifier=<value>` (pair with `kind` for the scheme) — served by a new deferred index
`organizations_identifier_id (identifier, id)`, so the `(filter, id)` paginated read seeks with no sorter.
The auto-reindex detector enqueued the build on open (`703`, ~38 s); prod-verified: a present VAT
(`RO42283735` → org 2) returns in **7.8 ms**, `identifier + kind` in 6 ms, an unknown value in **0.8 ms**
(a seek that finds nothing, not a walk). Honoured on organizations, named ignored elsewhere. This is the
front door to the winner/buyer/bidder reverse-lookups (issue 223): a consumer holding a company's VAT can
now resolve its canonical id, then its participation history.

**A fast path DEPLOYED & VERIFIED 2026-08-16 (serving rev `7386914`).** Two steps, both prod-measured:
(1) deferred index `notices_publication_id_id (publication_id, id)` (auto-reindex 704 built it in 25 s) —
`publication_id=` alone dropped ~10 s → **1.0 ms**; (2) the pairing stayed a 7.8 s walk because the
planner flattens the compound WHERE and drives from `notices_source_id`, so `notices_query` now emits
publication_id as the ONLY identity predicate and `source`/`kind` post-filter in Rust over the ≤handful
of matched rows. Final numbers: alone 1.0 ms, source-paired **0.5 ms**, kind-paired 0.5 ms, absent
0.6 ms, wrong-source correctly empty. Notices de-isolated per the 88d876a rule (measured first); the
isolation test inverted with full history. Finding worth keeping: turso flattens same-table FROM-subquery
and IN-seed shapes (the issue-223 trick does NOT transfer when both sides are one table) — post-filtering
in Rust over a near-unique key is the deterministic fix.

**Open follow-ups (split out):**
- **A-tenders** — `/v1/tenders?publication_id=` (an EXISTS over `tender_versions.publication_id`, also
  un-indexed-alone; same walk shape).
- **B-name** — DEPLOYED & VERIFIED 2026-08-16 (serving rev `5fa2c6c`). `/v1/organizations?name_prefix=`:
  Unicode-case-insensitive (umlauts included — Rust `to_lowercase`, never SQL's ASCII `lower()`), name-
  ordered with a `(name_norm, id)` keyset cursor. The `name_norm` column is resolver-written, migrated
  O(1), and was backfilled over ALL 24,614,285 orgs in 667 s (job 710); `organizations_name_norm_id`
  auto-built in 34 s. Prod: `siemens` 1.7 ms, `müller` 1.2 ms, cursor pages clean. One measured residual:
  a `country`/`kind` COMPANION flips the planner onto the companion's index (4.9 s scan over the DE
  slice) — correct, now routed ISOLATED (verified: main pool at 17 ms during a companioned search); a
  `(country, name_norm, id)` composite would make the pairing seek if demand appears. Empty prefix 400s.
- **C** (`bidder`/`tenderer`) — DONE (issue 217-C, serving rev `00f2f48`; see above).

Was: needs-triage — HIGH, CONFIRMED (code) 2026-08-15. Filed from the API completeness review (subagent).
Three related capability gaps, grouped because each is "add a filter to an existing collection" and they
share a theme; split out if one grows. Each section is independently actionable and severity-tagged.
Kind: completeness (missing query filters / lookups)
Blocked by: —
Relates to: 118 (accepted-but-ignored filters), 50 (sql-analyst-surface — today the only way to do these),
216 (the other half of the list-query vocabulary gap)

`Params` (`crates/app/src/v1/mod.rs:452-472`) has `deny_unknown_fields`, so every gap below is a hard
**400**, not a silent ignore — a consumer literally cannot pass these.

## A. No `publication_id` filter — the official TED/OJS notice number resolves to nothing (HIGH)

- `publication_id` is serialized on every tenders row (json.rs:68), notices row (json.rs:111) and version
  (json.rs:202), but is a filter on neither collection: `tenders_query` filters only source/kind/version
  predicates (`read.rs:849-915`), `notices_query` only source/profile (`read.rs:1732-1753`).
- Consumer scenario: "I have TED notice `2026/S 123-456789`; give me its tender / its parsed data via the
  public API" — impossible without a token and hand-written `/v1/sql`. The publication id is the real-world
  external key printed on every notice; not accepting it is a glaring hole.
- Fix: add an exact-match `publication_id=` filter to `/v1/tenders` and `/v1/notices` (the column is already
  indexed for ingestion dedup), or a `/v1/notices?publication_id=` lookup.
- **Index correction (2026-08-16, owner):** the dedup index is `UNIQUE(source, publication_id, content_hash)`
  (`lib.rs:138`) — a **source-leading composite**, so `publication_id=` ALONE is NOT a clean seek (the
  leading `source` column is unconstrained). `source=? AND publication_id=?` seeks it cleanly (and a TED
  number implies its source), so the cheap path is to pair them; `publication_id=` alone should either route
  to isolation (issue 120) or get a dedicated `notices(publication_id)` index via the deferred-index builder
  (issue 111). Same question on the tenders side — `tender_versions.publication_id` has no standalone index —
  so this section is a real index-infrastructure task, not a one-line filter add. Split it out when built.

## B. Organizations only fetchable by internal id — no lookup by official identifier value or name (HIGH/MED)

- `organizations_query` filters only `country`, `identifier_kind` (via `kind`), and `buyer` (= `o.id`)
  (`read.rs:1664-1688`). The `identifier` value, and `name`, are returned (json.rs:97-99) but not queryable;
  `/v1/organizations/{id}` (mod.rs:749-759) is internal-id only.
- Consumer scenario: "Find the organization with VAT `DE123456789`" or "organizations named 'Siemens'" —
  impossible via REST; `?kind=VAT` narrows to the scheme but cannot match the value. The canonical Org layer
  exists to dedup real-world entities by their official identifier; not being able to query by it defeats the
  point for an external consumer.
- Fix: add `identifier=` (paired with the existing `kind`) and a `name` prefix/contains search to
  `/v1/organizations`.

## C. No participation reverse-lookup beyond buyer and winner (MED)

- `version_predicates` implements only `buyer` (party role `%Buyer%`) and `winner` (`result_winners`)
  (`read.rs:590-605`). There is no `tenderer`/`bidder`/`subcontractor` filter, though those roles are
  modelled in `tender_version_parties` / `tender_version_bid_parties` and surfaced in tender detail's
  `parties[]` / `bids[].parties[]`.
- Consumer scenario: "All tenders where org 9 submitted a bid (won or not)" — unreachable; `?winner=9`
  under-counts (winners only) and there is no `?tenderer=9`. Bidder/competitor history is a standard
  procurement-analytics question.
- Fix: add a `bidder=`/`tenderer=` filter as an EXISTS over `tender_version_bid_parties`/`_parties` by role,
  mirroring the existing `winner` predicate — and give `reachable()` a matching absent-value probe so a
  sparse/absent org id short-circuits (see 219).

## Verification

- `GET /v1/tenders?publication_id=<known>` and `GET /v1/notices?publication_id=<known>` return the matching
  row(s); an unknown id returns an empty page (not 400, not a walk).
- `GET /v1/organizations?identifier=DE123456789&kind=VAT` returns the org; `?name=siemens` returns matches.
- `GET /v1/tenders?bidder=<org>` returns tenders that org bid on, a superset of `?winner=<org>`.
