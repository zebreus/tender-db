# tender-db

An API, database, and dashboard for public procurement data, aggregated from
multiple official publication sources into one queryable dataset. Users answer
arbitrary questions about historical and live procurement: market analysis,
spotting upcoming opportunities, public-spending transparency, competitor
analysis.

**Live instance:** <https://tenders.zebreus.click> · **API docs:**
<https://tenders.zebreus.click/docs> · **Dashboard:**
<https://tenders.zebreus.click/>

A **Tender** is a public procurement opportunity, independent of which **Source**
published it. It is derived from one or more **Notices** — the immutable
publication events at a Source (TED, the German Datenservice Öffentlicher
Einkauf, …). A Tender has **Lots** (separately-awarded subdivisions); **Bids**
are the offers Organizations place on Lots (never called "tender" here, despite
eForms' own naming). See [CONTEXT.md](CONTEXT.md) for the full domain language.

## The four faces

1. **REST query API** over Tenders, Lots, Organizations and Notices — filtered,
   cursor-paginated JSON under `/v1`.
2. **Live subscriptions** — Server-Sent Events on every collection endpoint
   (snapshot, then a resumable diff feed), plus a `/v1/changes` poll and
   account-registered **webhooks**.
3. **Read-only SQL** — account holders run a single `SELECT` against the public
   schema at `/v1/sql`.
4. **Dashboard** — data coverage, data-quality (quarantine), ingestion progress,
   and the account/token lifecycle.

Basic query endpoints are unauthenticated. An account (username + password only,
created on the dashboard — [there is no email or password reset](CONTEXT.md))
gates the SQL endpoint and webhook registration; API tokens (`tdb_…`,
`Authorization: Bearer`) authenticate those.

## Quickstart (API)

Every example runs against the live instance. Responses are JSON; the change
cursor is an **opaque string** — never do arithmetic on it. Counts grow while
the historical backfill runs.

Service info (also the AGPL §13 source offer):

```sh
curl -s https://tenders.zebreus.click/v1
```

**List tenders**, with filters (`source`, `country` NUTS prefix (alpha-2 at the
country level, e.g. `DE`), `cpv` prefix, `buyer`/`winner` organization id,
`status=open|closed`, `min_value`/`max_value` in EUR cents (compared against
the derived EUR-at-publication-date value), `currency` ISO-4217, `kind`; plus
`lang=de` to prefer a language for the picked titles — among the languages the
publisher wrote: the eForms era is served in the notice's own language, since
TED's per-language renderings are machine translations outside the bulk feed and
are not ingested). Page with `limit` (max 1000) and the returned `next_cursor`:

```sh
curl -s "https://tenders.zebreus.click/v1/tenders?country=DE&status=open&limit=5"
curl -s "https://tenders.zebreus.click/v1/tenders?cpv=45&min_value=100000000&limit=20"
```

**Tender detail** — one Tender with its lots, texts, amounts, parties, awards
(`lot_results`), bids, contracts and version history:

```sh
curl -s https://tenders.zebreus.click/v1/tenders/14327
```

Other collections take the same filters:

```sh
curl -s "https://tenders.zebreus.click/v1/lots?limit=5"
curl -s "https://tenders.zebreus.click/v1/organizations?country=ESP&limit=5"
curl -s "https://tenders.zebreus.click/v1/notices?limit=5"
```

**Poll for changes** since a cursor (`since=0` from the beginning). Each response
carries `last_cursor` and `more` — loop until `more` is false:

```sh
curl -s "https://tenders.zebreus.click/v1/changes?since=0&limit=100"
```

**Live feed (SSE)** — send `Accept: text/event-stream` to any collection
endpoint. You get one `added` event per matching row, a `live` marker, then
`change` events forever. `curl -N` disables buffering:

```sh
curl -N -H "Accept: text/event-stream" \
  "https://tenders.zebreus.click/v1/tenders?country=DEU"
```

Resume exactly where you left off with the last cursor you saw (browsers send it
automatically as `Last-Event-ID`; for curl pass it as a header or `?cursor=`):

```sh
curl -N -H "Accept: text/event-stream" -H "Last-Event-ID: 193000" \
  https://tenders.zebreus.click/v1/tenders
```

**SQL** (needs a token — create one on the dashboard). One `SELECT`, read-only,
against the public tables and views. See with the views (`v_tenders`, `v_lots`,
`v_lot_results`, `v_organizations` — unfiltered peeks and whole-corpus
aggregates), query with the tables: the engine applies a `WHERE` or `JOIN` on a
view only after building the whole view, so a filtered view read is refused up
front with the base-table join to use instead. Discover the schema at
`/v1/sql/schema`; every table's notes say which join reaches it fast:

```sh
curl -s https://tenders.zebreus.click/v1/sql/schema

# a whole-corpus aggregate over a view is fine (3 s)
curl -s -X POST https://tenders.zebreus.click/v1/sql \
  -H "Authorization: Bearer tdb_…" \
  --data 'SELECT source, count(*) FROM v_tenders GROUP BY source'

# anything filtered goes to the tables (17 ms for a point read)
curl -s -X POST https://tenders.zebreus.click/v1/sql \
  -H "Authorization: Bearer tdb_…" \
  --data 'SELECT t.id, t.current_title AS title, v.published_at
            FROM tenders t JOIN tender_versions v
              ON v.tender_id = t.id AND v.seq = t.current_seq
           WHERE t.id = 93601'
```

**Webhooks** — register an https URL to receive signed change batches
([Standard Webhooks](https://www.standardwebhooks.com/): verify the
`webhook-signature` header). The signing secret is returned once:

```sh
curl -s -X POST https://tenders.zebreus.click/v1/webhooks \
  -H "Authorization: Bearer tdb_…" \
  -H "content-type: application/json" \
  -d '{"url":"https://example.com/hooks/tenders"}'
```

Full endpoint, filter, SSE, SQL and webhook reference:
<https://tenders.zebreus.click/docs>.

## Architecture (five lines)

- Raw packages are **fetched** to an immutable archive, then **processed** into
  an append-only **Notice** layer (parsed, or quarantined whole if any field is
  unmapped) — fetch and process are separate stages.
- A deterministic **projection** builds the versioned **canonical** layer
  (Tenders/Lots/Bids/Organizations), every version traceable to the Notice that
  caused it.
- One monotonic **change cursor** is written with each canonical write and
  drives everything live: SSE, the poll endpoint, and webhooks.
- It is a **true monolith** (ADR-0005): API, dashboard, importer, SSE and
  webhooks are one Axum + Dioxus process owning one SQLite/Turso file.
- Deeper: [docs/architecture.md](docs/architecture.md), domain language in
  [CONTEXT.md](CONTEXT.md), decisions in [docs/adr/](docs/adr/), evidence in
  [docs/research/](docs/research/) (`SUMMARY.md` is the index).

## Build & develop

The repo is a Rust workspace (`model`, `store`, `ingest`, `app`) built as a
Dioxus 0.7 fullstack app. On NixOS/Nix, the flake carries the toolchain:

```sh
nix develop                          # dev shell with rust, dx, wasm target
nix develop --command dx serve       # dashboard + API, hot reload (http://localhost:8080)
nix develop --command cargo test --workspace --features tender-db/server
nix build .#tender-db                # the production bundle (server binary + public/)
```

`store`, `ingest` and the server API only compile under the `server` feature, so
run tests with `--features tender-db/server`. The fetch/process/project CLIs are
dev tools for scratch databases only — the production database is opened solely
by the running server (Turso is single-process).

## Deployment & operations

Production is one Hetzner VPS running the flake-built bundle under a hardened
systemd unit; `./deploy.sh [ref]` pushes, builds **on the VPS**, atomically
switches the symlink and health-checks. Full runbook (layout, backfill via the
`/admin` API, backups, recovery): [docs/operations.md](docs/operations.md).

## Licence

**AGPL-3.0-or-later** — full text in [LICENSE](LICENSE). As AGPL §13 requires,
the running server offers the corresponding source of the exact revision it runs
at <https://tenders.zebreus.click/_source> and in the dashboard footer.
