# 38 — Minimality cleanup from the architecture review

Status: needs-verification (32c99ea)

Fresh-eyes review (2026-07-21) found the boundaries clean but today's
rapid supersessions left un-swept leftovers. All verified zero-caller by
word-boundary grep; re-verify before each deletion:

1. Delete the superseded per-notice read path (issue-19 batching
   replaced it with parsed_chunk): Db::parsed_notice (canonical.rs:617),
   private value_queries() (canonical.rs:1762, near-duplicate of
   all_value_queries), Db::parsed_notices (canonical.rs:594).
2. Delete Db::apply_tender singular wrapper (canonical.rs:921) — dead.
3. Drop the duplicate change-log reader Db::changes_since
   (canonical.rs:1662) — production uses read::changes_since; repoint
   the ingest tests at that.
4. crates/ingest/Cargo.toml: remove the unused direct `model` dep
   (reached transitively via store). Leave app's wasm-bindgen pin.
5. Unify duplicated helpers: one epoch-seconds helper (currently 5×
   unix_now/now_unix across ingest/app — pick one name, one home);
   one civil-date helper (days_from_civil in supervisor.rs:751 +
   eforms/value.rs:196, inverse fetch::civil_date).
6. Tighten over-exposed pub: accounts.rs TOKEN_PREFIX + prefix_hint,
   canonical.rs Applied::add.
7. Award-linkage SQL duplication (Db::award_linkage vs data_quality's
   DENSITY_WITH_SQL): judge on the spot — if a shared formulation is
   clean, do it; if it couples store to the CLI awkwardly, document the
   mirror-comment as deliberate and leave it.

Acceptance: all suites + clippy green after each step; no behaviour
change (deletions + moves only); diff is pure minus except the helper
unification.
