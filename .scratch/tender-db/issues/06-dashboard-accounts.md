# 06 — Dashboard v1 + accounts

Status: resolved
Blocked by: 05

Goal: the product's face: data coverage + quality on the dashboard, and the
account lifecycle.

Scope:
- Dashboard (dioxus, `/`): coverage per Source/profile/year (notice counts
  vs the research ground-truth table vendored as data), quarantine count +
  drill-down, unchained-awards and junk-org-id metrics, import lag (last
  fetched/processed package vs today).
- Accounts: register (username+password, argon2id), login (session cookie),
  API token create/revoke (`tdb_` prefix, SHA-256 at rest, shown once),
  delete account. Server functions under `/_dash`; lost password = lost
  account (documented at sign-up).
- AGPL §13: source link at API root response and dashboard footer.
- Tests: auth round-trip, token auth extractor on a gated probe endpoint.

Acceptance: dashboard renders real coverage from an ingested day; account
lifecycle works end-to-end in the integration test and manually.

## Answer

Delivered: accounts in `store` (`crates/store/src/accounts.rs`), the account
lifecycle and dashboard measurement in the app library
(`crates/app/src/accounts.rs`, `crates/app/src/coverage.rs`), the bearer-token
extractor in the public API (`crates/app/src/v1/auth.rs`), and the UI
(`crates/app/src/ui.rs`, `/` = dashboard, `/account`, `/tenders`).

**Dashboard panels** (`/`, polled every 15 s — a plain poll, deliberately not
new SSE plumbing):

- *Contents* — the canonical counts (Tenders incl. island, versions, Lots,
  Organizations canonical/provisional, mentions, changes) plus the cursor.
  "tenders (island)" is the unchained-notice metric and
  "organizations (provisional)" the junk-org-id metric the issue asked for; both
  already fall out of `canonical_counts()`, so they are rows here rather than
  panels of their own.
- *Import lag* — age of the newest fetched package and of the newest ingested
  Notice, separately, because fetching and processing are separate stages.
- *Quarantine* — headline count, breakdown by reason, and a drill-down of the 50
  newest (reason, profile, member path, detail as a tooltip).
- *Coverage* — notices held per (source, profile, year) against the vendored
  ground truth, with a ratio. Years group by the *package period* the notice came
  from, not by a parsed date: coverage is a question about packages.

**Ground truth** is vendored at `crates/app/data/ted-notice-counts.csv`
(`include_str!`), transcribed from `docs/research/ted-access-channels.md` §6.
Note: that section's rows sum to **13.31 M**, not the "≈12.9 M" its own prose
quotes beneath them — the rows are what we transcribed, and a unit test pins the
sum so a future edit to either cannot drift silently.

**Accounts**: username + password only, argon2id (crate defaults = OWASP
parameters) in `spawn_blocking`, PHC string at rest. Two separate credentials —
a DB-backed session cookie (`HttpOnly; Secure; SameSite=Lax; Path=/`,
`Secure` droppable via `TENDER_DB_INSECURE_COOKIES=1` for local http dev) for the
dashboard, and `tdb_<64 hex>` bearer tokens stored as SHA-256 for the API. Tokens
are shown once; the revoke list keeps revoked rows as the audit trail. Login
verifies a dummy hash for unknown usernames so timing does not enumerate
accounts. Deleting an account deletes its tokens and sessions in one
transaction (explicit child deletes, not `ON DELETE CASCADE` — the order is ours,
not the engine's).

**Deviations from the issue text and the research doc, deliberate:**

- Server functions live under `/api` (the existing dioxus namespace), not
  `/_dash`. `docs/architecture.md` updated to match.
- No `axum-extra` cookie layer. api-layer.md §5 recommends it (and names version
  0.12.6, which does not exist for axum 0.8 — 0.10.3 is what the lock has). The
  session cookie is one header in each direction, so it is read from `HeaderMap`
  and written with dioxus's own `SetHeader<SetCookie>`; the bearer extractor
  reads `Authorization` from `Parts` directly. Zero new HTTP dependencies.
- `argon2` is the only new workspace dependency for credentials: it re-exports
  `password_hash`, which supplies the PHC codec *and* the OS RNG the token and
  session secrets are drawn from. `futures-timer` was added to the app crate for
  the dashboard's refresh poll (one `Delay` future that works in both the wasm
  client and a native render).
- `v1::AppState::new` now takes `(Arc<Db>, Arc<Readers>)` and derives the cursor
  watch itself — token authentication needs the writer for the best-effort
  `last_used_at` touch.

**Gated probe**: `GET /v1/me` → `{"user":{id,username,created_at}}`, 401
otherwise. `AuthUser` also implements `OptionalFromRequestParts`, so issues 07
and 08 get both `user: AuthUser` and `user: Option<AuthUser>` for free.

**AGPL §13**: every dashboard page footers a link to `/_source`.

**Tests**: 5 store round-trips (hash+verify incl. corrupt-hash denial, token
lookup by hash + revoke + cross-account isolation, session expiry and logout,
delete-cascades), 3 app unit tests (cookie flags/parsing, credential policy,
ground-truth parse, number/age formatting), and 4 integration tests over a real
socket (register→login→mint→`/v1/me` 200→revoke→401; delete-account closes the
gate; cross-account isolation; the dashboard measures an empty database
honestly).

## Comments

2026-07-20 — Production UX defect: the dashboard flashed/reloaded every few
seconds, unusable. Root cause: both `DashboardPage` (15 s) and `IngestionPanel`
(3 s) polled by calling `.restart()` on a `use_server_future(..)?` resource.
`use_server_future`'s `?` re-suspends the whole subtree the instant the
resource turns `Pending` (it checks `state == Pending`, ignoring the retained
old value), and `.restart()` sets it `Pending` on every tick — so each poll
swapped the rendered panels for the suspense fallback for a frame. Because
`IngestionPanel` renders inside `DashboardPage`, its 3 s poll blanked a large
chunk of the page.

Fix (commit 11f46f5): a `use_polled(period, fetch)` hook that keeps the last
value. The first load still flows through `use_server_future` — the server
render carries the data, the client hydrates without a refetch, and that first
load is the only moment a loading state may show — and every refresh after that
runs in a background `use_future` that writes a `use_signal`; the component
renders the freshest signal value, falling back to the transported seed. The
resource is never restarted, so nothing ever re-suspends. (The account / token /
webhook panels already used the non-suspending `use_resource` + `.read()`
pattern, so they never flashed; left as-is.)

Polish shipped alongside: ingestion running-job elapsed + notices/s throughput
(server-clocked via new `Ingestion.measured_at`), queue-depth count, full-width
progress bars; coverage folded into one collapsible row per (source, profile)
era with held/published/ratio in each summary (scales to 34 years); a System
panel with service revision (new `Dashboard.service_rev`, measured server-side),
change cursor, and import lag.

Verification: client/wasm build + `cargo clippy --features web` clean; new unit
tests for `duration` and `coverage_by_era`. Full server gate
(`cargo test --workspace --features server`, `nix build .#tender-db`) and the
production deploy are pending — the shared worktree's `ingest` crate is
mid-refactor by another agent (issue 15 results layer) and does not currently
compile; will run gates + deploy once it does and the supervisor queue is idle.
