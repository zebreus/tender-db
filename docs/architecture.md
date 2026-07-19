# tender-db architecture

The design the implementation follows. Domain language: CONTEXT.md. Decisions
with rationale: docs/adr/. Evidence: docs/research/ (SUMMARY.md is the index).

## Data flow

```
fetch ──► /data/archive (raw packages, immutable, hash-idempotent)
             │ extract + per-file profile dispatch
             ▼
         Notice layer (identity + parsed relational form; quarantine)
             │ projection (deterministic, rebuildable)
             ▼
         Canonical layer (versioned Tenders/Lots/Bids/Organizations…)
             │ every write appends to…
             ▼
         Change log (the cursor spine)
             │
             ├─► REST /v1 + SSE (snapshot+diff) + poll + webhooks
             ├─► SQL endpoint (read-only layered gate)
             └─► Dashboard (coverage, quality, accounts)
```

One process owns all of it (ADR-0005): the importer is a background task of
the server — the ingestion Supervisor (job queue + scheduler + progress
state), triggered by schedule or by the `/admin` API (preshared operator
secret, TENDER_ADMIN_SECRET). The fetch/process/project CLIs are dev tools
for scratch databases only; no external process ever opens the production DB
(turso is single-process). Fetching and processing are separate stages — a
parse bug never forces a re-download; a re-projection never re-parses raw
XML unless asked.

## Crates

- `model` — shared serde domain types (wasm + native). No IO.
- `store` — Turso persistence: schema, migrations, writer/readers, change
  log, quarantine. One writer connection behind a mutex (ROLLBACK after any
  abandoned write future — turso 0.7.0 poisons the tx otherwise), N reader
  connections; pragmas per connection (foreign_keys=ON is OFF by default).
- `ingest` — fetchers (TED, DÖE) + era/profile parsers + projection. Owns
  the completeness checklists.
- `app` (`tender-db`) — Dioxus fullstack binary: public axum API under
  `/v1`, SSE, SQL endpoint, webhooks delivery, dashboard (server functions
  under `/api`), and the importer scheduler.

## Notice identity and profiles

A Notice is keyed by (source, publication_id, content_hash) — TED publication
number / DÖE uuid+version; declared versions (BT-757) are advisory
(ted-empirical-checks.md). Every notice file is dispatched to a mapping
profile by its root element + CustomizationID:

`text` | `ted-export-r208` (incl. R2.0.7, defence) | `ted-export-r209` |
`eforms` (per (SDK-version, national-profile) sub-profiles incl. eforms-de
and the DÖE sdk-0.1 empirical inventory).

Each profile = a parser producing (mapped fields, explicit ignore hits) with
XML handled by roxmltree, matching namespace-URI + local name. Anything
neither mapped nor ignore-ruled quarantines the whole notice (ADR-0004,
era-scoped). Completeness is enforced by tests: per profile, walk its
authority inventory (fields.json / XSD element inventory / committed path
inventory) and assert every entry has a mapping or a documented exclusion.

## Canonical layer and versioning (ADR-0001)

Version-row pattern: per aggregate an identity table (`tenders`) plus a
versions table (`tender_versions(tender_id, seq, caused_by_notice_id,
published_at, …payload)`) and satellites keyed by (tender_id, seq): texts
(lang, field, value — EN + original only), amounts (INTEGER cents +
currency), classifications, parties, withheld fields, legacy-only fields.
Current state = max(seq) per entity (exposed as SQL views). Time-travel =
filter on published_at ranges. Lots live under the tender version (lot ids
resolved defensively per version — FA/DPS rounds relabel them). Organizations:
`organizations` (canonical) ← `organization_mentions` (per notice, immutable);
merging only on exact normalised official identifiers with plausibility gates.

Projection is deterministic and rebuildable: canonical = f(notice layer,
merge rules). Re-projection appends new versions; it never rewrites history.

## Change cursor ↔ backfill (resolved design)

The cursor is **ingestion order**: `changes(cursor INTEGER PRIMARY KEY
AUTOINCREMENT, entity_kind, entity_id, version_seq, op, changed_at)`, written
in the same transaction as the canonical write. Properties:

- Never renumbered. Re-projections and merges append new change rows.
- Backfill emits change rows uniformly — no special mode. The initial
  backfill runs before subscribers exist; later bulk imports are just busy
  periods (SSE clients resume by cursor; webhook slots batch).
- Domain time (published_at) and learn time (cursor) are independent axes;
  clients wanting domain order sort by published_at, the live feed is learn
  order. This is what makes out-of-order historical ingestion harmless.

## SSE (filtered, Firestore-like)

A subscription = a `/v1/...` collection query + its filter params (the same
indexed predicates as REST: source, country, CPV prefix, buyer, status,
value range, kind). Protocol: subscribe watch → snapshot in one read tx
(capture cursor N, stream `added` events) → `live` marker → diff loop
`cursor > N`. Per change, the server loads (old, new) version rows and
evaluates each active subscription's predicate on both: new-only → `added`,
both → `changed`, old-only → `removed`. Predicates are cheap in-memory
checks; subscriptions are capped (5/IP anonymous). Cursor is the SSE id
(Last-Event-ID resume); a cursor below retention or across a reset gets an
explicit `reset` event. Poll endpoint and webhooks consume the same log
(webhook endpoint = stored cursor position, advanced on 2xx).

## API surface

- `/v1/tenders`, `/v1/tenders/{id}`, `/v1/lots`, `/v1/organizations`,
  `/v1/notices`, `/v1/changes?since=`, `/v1/sql` (POST, account), SSE via
  `Accept: text/event-stream` on collection endpoints. JSON, cursor-paginated.
- SQL gate layers: turso_parser single-SELECT allow-list → `query_only`
  reader → 10s timeout-by-drop → 10k rows/10MB streaming caps → per-token
  limits (2 concurrent, 300/h). Never enable ATTACH on serving handles.
- Auth: argon2id passwords; `tdb_` tokens stored as SHA-256; session cookie
  for the dashboard; `AuthUser`/`Option<AuthUser>` extractors.
- Standard-Webhooks signatures; retries 30s→daily; disable after sustained
  failure. Secrets plaintext (one-box threat model); delivery https-only to
  public addresses (dev flag relaxes).

## Storage layout (VPS)

- `/data/archive/<source>/...` — fetched packages exactly as downloaded,
  named by source convention, with a `fetches` registry table (url, sha256,
  fetched_at, path). Idempotency by hash; TED daily packages re-fetched if
  the hash changes before 09:30 CET finality.
- `/data/db/tender-db.db` — the Turso file (TENDER_DB env).
- Backups: none for now (accepted risk; everything rebuildable). The
  checkpoint+copy runbook lives in docs/research/turso-scale.md when needed.

## Deployment (ADR-0006)

Ubuntu VPS. Build via `nix build .#tender-db` (on the VPS — 1 Gb/s), run as
a hardened systemd unit (mirror of nix/module.nix flags), nginx + certbot in
front terminating TLS for tenders.zebreus.click (SSE: proxy_buffering off /
X-Accel-Buffering: no). The NixOS module + VM smoke test stay as CI and as a
distributable for NixOS users; production parity lives in the Ubuntu unit.

## Testing strategy

- Unit: store round-trips, profile parsers against committed fixture
  notices (real, one per profile per notice type).
- Completeness: per-profile checklist tests (the ADR-0002/0004 guarantee).
- Integration: end-to-end ingest of a fixture package → canonical → API
  assertions, in-process.
- `nix flake check`: clippy + VM smoke test (bundle serves, API answers).
- Production verification: era-ladder spot checks + API-vs-package counts
  (the Search API count assertion from ted-access-channels.md) after
  backfill.
