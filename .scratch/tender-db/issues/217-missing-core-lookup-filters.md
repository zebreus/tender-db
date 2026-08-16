# 217 — the API can't be queried by the keys real consumers hold: publication_id, org identifier/name, tenderer/bidder

Status: PART A (notices) DEPLOYED & VERIFIED 2026-08-16 (serving rev `5ca7971`) — B, C, and A-tenders + a
fast path OPEN. `/v1/notices?publication_id=<id>` now resolves the official notice number (commits
`88d876a` + `5ca7971`); prod-verified: a source-paired lookup returns the matching notice, unknown → empty
page, and it is named in `ignored_filters` on `/v1/tenders` (honoured_params, issue 118).

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

**Open follow-ups (split out):**
- **A fast path for `publication_id`** — a query shape that seeks the composite index for the exact-match /
  ≤1-result case (drop the id-cursor pagination when `publication_id` is present), or a dedicated
  `notices(publication_id)` index. Would move the lookup from the isolated ~10 s walk to a ~ms main-pool
  seek. HIGH-ish (it is the primary lookup and 10 s is poor UX).
- **A-tenders** — `/v1/tenders?publication_id=` (an EXISTS over `tender_versions.publication_id`, also
  un-indexed-alone; same walk shape).
- **B** (org `identifier`/`name`) and **C** (`bidder`/`tenderer`) — untouched.

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
