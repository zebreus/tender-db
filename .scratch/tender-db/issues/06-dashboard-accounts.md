# 06 — Dashboard v1 + accounts

Status: ready-for-agent
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
