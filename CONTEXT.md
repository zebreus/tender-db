# tender-db

An API, database, and dashboard for public procurement data, aggregated from
multiple official publication sources into one queryable dataset. Users answer
arbitrary questions about historical and live procurement: market analysis,
spotting upcoming opportunities, public-spending transparency, competitor
analysis.

## Language

**Tender**:
A public procurement opportunity as tracked by tender-db, independent of which Source published it.
_Avoid_: bid, contract (those are different stages of procurement)

**Lot**:
A subdivision of a Tender that is awarded separately; the unit Bids are placed on.

**Bid**:
An offer submitted by an Organization for a Lot (the eForms "tender" / TEN- entity — the word "tender" never means an offer inside tender-db).
_Avoid_: offer, and especially eForms' own use of "tender"

**Notice**:
A single publication event at a Source about a Tender (contract notice, corrigendum, award notice, …) — the raw imported record. Its identity is the Source's publication identity plus a content hash; declared version numbers (BT-757) are advisory only, as real TED chains have gaps, missing v01s, and cross-type version sequences (docs/research/ted-empirical-checks.md).

**Source**:
An external publication platform tenders are imported from (TED, national portals).
_Avoid_: feed, provider

**TED**:
Tenders Electronic Daily, the EU's official journal for above-threshold procurement — the primary Source.

**eForms**:
The EU standard (Regulation 2019/1780 + SDK) defining the structure of TED notices; SDK 1.15 defines 1256 context-specific fields from 357 distinct business-term ids (counts vary per SDK version — see docs/research/eforms-data-model.md).

**Business Term (BT)**:
One atomic eForms field definition (e.g. BT-05 "Notice Dispatch Date"); v1 must represent all of them, no omissions, accounted per SDK version.

**Organization**:
A canonical profile of a company or authority appearing across Tenders (as buyer, bidder, winner, subcontractor); one profile per real-world entity, not per notice mention. Its `country` is the jurisdiction of the register its identifier lives in, because it keys identity: a SIREN published under an overseas-department code (`RE`, `GP`, `MQ`, `GF`, `YT`, `PM`, `BL`, `MF`, `WF`) is French, a Y-tunnus under `AX` Finnish, a CVR under `GL` Danish, an orgnr under `SJ` Norwegian, and the row carries the register's code (issue 358). Territories with a register of their own (`NC`, `PF`, `FO`, `AW`, `CW`, `SX`, `BQ`) keep theirs. The OrganizationMention keeps the code the notice published.
_Avoid_: company, authority (those are roles/kinds of Organization, not separate concepts)

**OrganizationMention**:
One appearance of an organization in one Notice (the eForms ORG- entity, whose ID is notice-local) — the immutable evidence canonical Organizations are built from.

## Relationships

- A **Tender** has one or more **Lots**. Lot identifiers are stable across a
  procedure's notices except inside framework/DPS call-off rounds, where some
  buyers redefine them per round — lot references are resolved defensively,
  per notice version. Award rounds are repeated award notices under one
  procedure; there is no separate "round" entity (verified empirically).
- A **Tender** is documented by one or more **Notices**; each Notice comes
  from exactly one **Source**. A Tender usually has one Source, but when a
  strong explicit cross-reference proves two Sources publish the same
  procedure, their records merge into one Tender (ADR-0003).
- **Organizations** participate in Tenders in roles (buyer, bidder, winner, …).
- A **Notice** contains **OrganizationMentions**; an **Organization** groups
  the mentions resolved to one real-world entity. Mentions are auto-merged on
  exact official identifiers (registration number, VAT id), and identifier-less
  mentions sharing a normalised name and country resolve to one `provisional`
  profile (issues 234 and 351) — `provisional` means "no official identifier",
  not "one mention". Mentions are never destroyed by merging.

## Decisions

### Product
- Four faces: REST-style query API structured around Tenders and Lots, live
  subscriptions (SSE on every endpoint, Firestore-like), arbitrary read-only
  SQL queries, and a dashboard showing data coverage and data quality.
- One monotonic change cursor (from the canonical version sequence) drives
  everything live: SSE streams (initial snapshot event, then
  added/changed/removed diffs, resumable via Last-Event-ID), a poll endpoint
  for "what changed since", and webhooks.
- Webhooks are in v1, minimal: account holders register a URL that receives
  POSTed change events with retries.
- Basic query endpoints are unauthenticated. Accounts (username + password
  only, no email verification, created on the dashboard) gate webhook
  registration and the SQL endpoint.
- The SQL endpoint requires an account and is enforced read-only via a layered
  gate: single-SELECT statement allow-list (parser-based), a `query_only`
  connection, timeout-by-drop, result-size caps, per-user rate limits — turso
  has no read-only open flag or authorizer, so the allow-list is the primary
  wall (docs/research/turso-capabilities.md).
- SSE is anonymous with a per-IP stream cap (~5); the change log is kept
  indefinitely but cursors may expire (documented reset path). Webhook
  secrets are stored plaintext (one-box threat model); delivery is https-only
  to public addresses (dev-mode escape hatch). Opening rate posture is
  generous (≈10 rps/IP, SQL 2 concurrent + 300/h per token). Lost password =
  lost account (no email exists by design). (All resolved 2026-07-19.)
- Metadata only: the PDF/document attachments of tenders are out of scope for
  now.
- The dashboard owns the account lifecycle: register, login, generate API
  tokens, delete account. API tokens authenticate the account-gated API
  features (SQL endpoint, webhooks).
- License: AGPL-3.0-or-later. The running server links its own source (AGPL
  §13) at the API root and dashboard footer.
- Notice content is public business data; no privacy-driven exposure
  restrictions apply (assessed by Lennart's lawyer, 2026-07-19).

### Data
- Sources: TED first, plus oeffentlichevergabe.de (Datenservice Öffentlicher
  Einkauf) as the German Source — anonymous CC0 bulk exports (eForms-DE XML,
  OCDS, CSV) back to 2022-12; chosen after a full portal survey
  (docs/research/german-portals.md) specifically to keep the model
  source-agnostic. service.bund.de was deep-dived and REJECTED as a Source —
  "do not ingest — not now, and probably not later either"
  (docs/research/service-bund-de.md §9, decision C22).
- TED history spans three format eras (tagged text 1993–2010, TED_EXPORT XML
  2011–2024, eForms 2023→, mixed per-file during the transition); importers
  dispatch a mapping profile per file (text / r208 / r209 / eforms, plus
  per-CustomizationID eForms profiles incl. eForms-DE and DÖE sdk-0.1, which
  is a permanent ~40%-of-volume dialect, not a transition artifact). Legacy
  chains link via OJ notice numbers (transitive edges — a missed link splits
  a Tender, never wrongly merges); chains break at the eForms boundary. See
  docs/research/ted-access-channels.md and ted-legacy-mapping.md.
- Near-real-time comes from TED alone (daily package by 09:30 CET Mon–Fri);
  oeffentlichevergabe.de is strictly T+1 and serves as a daily reconcile.
- No geographic focus, and not even locked to public procurement long-term;
  TED-primary is a bootstrapping choice because its data structures are well
  documented.
- All TED notice types are in scope, and all eForms Business Terms must be
  representable in v1 — no omissions.
- The schema is idiomatic, normalised SQL: links between structured records
  are resolved to real foreign keys, not left as embedded identifiers.
- The schema is fully hand-designed; the eForms SDK's fields.json is used only
  as a mechanical completeness checklist, never as a schema generator
  (ADR-0002).
- Both historical backfill and continuous near-real-time updates, with change
  detection, are first-class from the start.
- Two-layer model: append-only Notices (raw + parsed) as source of truth, plus
  a versioned canonical layer whose every version is traceable to a Notice and
  queryable with plain SQL (ADR-0001).
- Canonical Organization profiles (buyers and bidders) are a deliberate
  product feature, not just normalisation.
- Every Notice yields a Tender even without linkage identifiers: sources
  publishing island notices (DÖE sdk-0.1 numeric channel, ~40% of German
  volume) produce single-notice Tenders, upgradeable by re-projection if
  linkage ever appears (resolved 2026-07-19).
- Representation (resolved 2026-07-19): money as INTEGER cents + currency
  code; timestamps as UTC + original offset; codelist labels English-only
  (schema supports more); Reviews stay notice-layer-only in v1, Parts are
  Lots with a kind flag, BRIN notices become minimal Tenders of a distinct
  kind.
- Ingestion is strict: a notice with any unmapped content is quarantined whole
  (raw payload kept, reason recorded, reprocessable), never partially or
  silently imported (ADR-0004). The quarantine count is the dashboard's
  headline data-quality metric.
- Fetching and processing are separate stages: the fetcher only downloads and
  stores raw, versioned, unprocessed payloads; the processor parses them into
  Notices and the canonical layer. An unmappable field never forces a
  re-download.
- Fetch idempotency is hash-based on our side: TED serves no ETags, checksums,
  or Last-Modified, and daily packages may be rewritten until 09:30 CET on
  publication day.

### Infrastructure
- Persistence: a single SQLite file via Turso (pure-Rust, no system
  dependencies); WAL, foreign_keys, busy_timeout, synchronous=NORMAL, STRICT.
- Stack: Dioxus 0.7 fullstack (Axum server + WASM dashboard), packaged and
  deployed via the Nix flake; organised as a Rust workspace so model, ingest,
  and app layers are encapsulated as separate crates.
- True monolith: API, dashboard, importer, SSE, and webhooks are one process
  on one server owning the SQLite file exclusively (ADR-0005). Within the
  process: one writer connection, N parallel reader connections; connection
  pragmas (foreign_keys=ON — it defaults OFF — busy_timeout, synchronous)
  are applied per connection.
- Disk: the parsed DB does not fit the VPS's 75 GB — multilingual text
  satellites are ~86% of it (22.3 KB/notice measured on a real month;
  docs/research/pilot-sizing.md) — so a Hetzner volume is required before any
  backfill. The raw archive lives on the filesystem as the fetched packages;
  the DB stores (package, filename, sha256) references, never raw XML blobs.
- Backups: pause writer → `wal_checkpoint(TRUNCATE)` → file copy (~20 s at
  10 GB), verified offline with integrity_check + row counts. `VACUUM INTO`
  is forbidden at scale (OOM, docs/research/turso-scale.md). Bulk loads go
  PK-only-then-index. Turso 0.7.0 trap: an abandoned half-done write
  statement poisons the open transaction — the writer must ROLLBACK after
  any dropped write future.
- Deployment (resolved 2026-07-19): the VPS stays Ubuntu, running the
  flake-built bundle under a hardened systemd unit (ADR-0006); the NixOS
  module + VM smoke test remain as CI and as a distributable. Public
  hostname: tenders.zebreus.click. Storage: a 1 TB Hetzner volume (grown from
  500 GB on 2026-07-22) carries DB + raw archive. No off-box backups for now —
  accepted risk, under re-decision: the measured recovery cost is ~1–1.5 days
  (canonical layer from archive) to ~4–6 days (DB/volume loss, incl. re-fetch),
  not the "roughly a day" originally estimated, and user state (accounts,
  tokens, webhooks — <1 MB) is NOT rebuildable from anything (see
  docs/research/dr-premise-2026-08.md; snapshots were removed 2026-08-06).
- Heavy scraping runs on the provisioned Hetzner VPS (1 Gb/s) — also the
  production target — never on the dev machine (~100 kB/s uplink). Access is
  via `ssh root@zebreus.click`; run any command expected to take more than a
  few seconds inside tmux on the VPS, so it survives a dropped connection.

## Example dialogue

> **Dev:** "TED sent a new notice — do I update the **Tender** directly?"
> **Domain expert:** "No. The **Notice** is the immutable record from the
> **Source**; the **Tender** is what we derive from all its Notices. And
> careful: if the notice mentions 'tenders received', those are **Bids**."

## Flagged ambiguities

- "Tender" was overloaded (eForms uses it for a submitted offer) — resolved:
  in tender-db a **Tender** is always the opportunity/procedure; the eForms
  TEN- entity is a **Bid**. Importers translate at the boundary.
- Completeness promise (resolved 2026-07-19): "everything the source era
  publishes, nothing silently dropped" — era-scoped per-profile checklists,
  strict quarantine within each profile's universe (ADR-0004 amendment).
  Backfill: full history 1993→, text era header-only and English-only for
  now (model stays multilingual); parsed DB stores EN + original language,
  raw archive keeps everything.
- Cross-source field precedence for merged Tenders (ADR-0003) resolved
  (2026-07-21, owner sign-off under transferred product authority): per field
  class — DÖE is the richer original for German content (national codes, future
  DEX fields), TED owns publication identity; shared eForms fields resolve
  through the supersession fold (publication-date order, dispatch fallback,
  TED-last source tiebreak), as implemented in the projection.
