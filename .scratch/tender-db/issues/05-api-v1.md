# 05 — Public API v1: REST + changes + SSE

Status: resolved
Blocked by: 04

Goal: the read side goes live in-process: `/v1` REST over the canonical
layer, the changes poll endpoint, and SSE snapshot+diff with resume.

Scope:
- `app`: axum sub-router merged beside dioxus (`/v1`), reader-connection
  pool from `store`.
- `GET /v1/tenders` (+`/{id}`), `/v1/lots`, `/v1/organizations`,
  `/v1/notices` — cursor-paginated JSON, filters: source, country, CPV
  prefix, buyer, status, value range, kind. `GET /v1/changes?since=`.
- SSE on collection endpoints via `Accept: text/event-stream`: watch
  doorbell, snapshot-in-read-tx (capture N) → added events → live marker →
  diff loop with per-subscription (old,new) predicate evaluation
  (docs/architecture.md); Last-Event-ID resume; reset event; keep-alive
  15s; X-Accel-Buffering: no; 5 streams/IP cap.
- Per-IP rate limiting (tower_governor) on `/v1`.
- Integration test: in-process server over a fixture-ingested DB; REST
  shapes, SSE snapshot/diff/resume sequences asserted.
- Update the VM smoke test to assert `/v1/tenders` answers.

Acceptance: integration tests green; manual `curl` against a locally
ingested day shows tenders, changes, and a live SSE diff when a new notice
is ingested mid-stream.

## Answer

Delivered: the reader side of `store` (`crates/store/src/read.rs`) and the
public API (`crates/app/src/v1/`), merged beside dioxus in one process.
`nix build .#tender-db` produces a bundle that serves the dioxus shell at `/`,
the dashboard server functions at `/api/…` and the whole public API at `/v1`
from the same binary — verified by running the built artifact, not just the
tests.

### Endpoints

| | |
|---|---|
| `GET /v1` | service info + AGPL §13 source offer + current cursor |
| `GET /v1/tenders`, `/v1/tenders/{id}` | list + full detail (satellites, parties, lots, version chain) |
| `GET /v1/lots`, `/v1/organizations`, `/v1/notices` | cursor-paginated lists |
| `GET /v1/changes?since=&limit=&entity=` | `{events, last_cursor, more}` |
| `GET /health` | `{ok, rev, database, cursor}` — what `deploy.sh` probes |
| `GET /_source` | the §13 written offer, naming the running revision |

Filters (`source`, `country`, `cpv`, `buyer`, `status=open|closed`,
`min_value`, `max_value`, `kind`, `tender`) are shared across collections.
Pagination is `?cursor=&limit=`, the cursor an opaque string that happens to be
the last row id; `more`/`next_cursor` agree by construction. Per-IP rate
limiting (tower_governor 0.8, 10 rps / burst 50) covers `/v1` only — `/health`
must answer under load and `/_source` is a licence obligation.

### One filter, three evaluations

The design decision worth recording: the filter predicates are built **once**,
as SQL, and evaluated in two *scopes* — `Page` (current state of everything
matching) and `At { id, seq }` (did this one version match?). The REST list,
the SSE snapshot and the SSE diff's `(old, new)` probe all go through the same
`read_items`, so a filtered stream cannot drift from the filtered list it
started from. There is no second, in-Rust copy of the predicate to keep in
step, which is what docs/architecture.md's "evaluate each subscription's
predicate against (old, new)" would otherwise have cost.

That also gives the *subscription's* view of an event rather than the log's: a
Tender edited **out of** a filtered set is a `removed` for that client and an
`added` for one whose filter it moved into, from the same change row.

### SSE conformance

Subscribe → snapshot-in-one-read-tx (capturing `N` inside it) → `added` events
→ `live` marker with `id: N` → diff loop on `cursor > N`. Keep-alive 15 s,
`X-Accel-Buffering: no`, `Cache-Control: no-store`, cursor as the SSE `id:`,
`Last-Event-ID` (or `?cursor=`) resume, 5 streams/client (429 beyond, slot
released on disconnect).

**One correction to the recipe.** The doorbell must be *drained before it is
waited on*, not after. A resuming client's backlog was committed before it
connected, so no `watch` notification will ever fire for it — an implementation
that waits first hangs forever on exactly the case resume exists for. The loop
is therefore drain → wait → drain, and the test
`a_resumed_subscription_gets_exactly_what_it_missed` is what caught it.

### Tests

`crates/app/tests/api.rs`, 12 tests: the real router over a real Turso file
ingested from the real fixture chain, over a real socket. REST shapes, every
filter, pagination walking a collection exactly once, detail + 404, the change
feed, and the SSE protocol — snapshot→live, a diff arriving from a notice
ingested *mid-stream*, an exact resume, a filtered stream staying silent for a
non-matching change, and the 5-stream cap releasing slots. Green under both
`cargo test --workspace` and `--features tender-db/server`; clippy silent; the
VM smoke test now asserts `/v1/tenders` and `/health`.

### Three gaps, deliberately left

1. **AGPL §13 is a written offer, not a tarball.** `/_source` returns the
   revision and how to request the Corresponding Source. Embedding a source
   bundle via `include_bytes!` would bloat the binary and the build; serving
   the git repo needs a public URL that does not exist yet. **Lennart: resolve
   at deploy time** — either publish the repo and point `SOURCE_OFFER`
   (`crates/app/src/v1/mod.rs`) at it, or keep the offer and be reachable.
2. **The client key is a forwarded header, not the peer IP.**
   `dioxus::server::serve` uses `into_make_service()`, so `ConnectInfo` does
   not exist and the peer address is unavailable to us. Production is behind
   nginx (where the peer address would be the proxy's anyway), so
   `X-Forwarded-For`/`X-Real-IP` is the correct key there — but nginx **must**
   set them, and direct callers share one bucket. Recovering true peer IPs
   means serving our own listener instead of dioxus's, which costs the dev
   hot-reload loop.
3. **SSE on `/v1/notices` snapshots and then stays silent.** Notices are the
   raw import layer and have no canonical change rows; the diff loop has no
   `entity_kind` to select. Honest, but worth a line of API docs.

Minor: event `id` fields are integers (our canonical ids), where
api-layer.md §3 sketched opaque strings. Cursors *are* strings, as that
lesson intended; entity ids stay integers to match the REST payloads.
