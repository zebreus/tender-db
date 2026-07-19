# 12 — DÖE source: fetcher, eForms-DE + sdk-0.1 profiles, cross-source merge

Status: ready-for-agent
Blocked by: 04

Goal: oeffentlichevergabe.de is a live second Source and German procedures
merge across Sources.

Scope:
- Fetcher: monthly + completed-day exports (eforms.zip), T+1 schedule,
  registry rows; archive under /data/archive/doe/.
- eForms-DE profile: SDK-DE deltas (+4 fields, national codelists,
  E-subtypes, DE→EU version fallback table per
  docs/research/eforms-de-profile.md); DEX satellite table.
- sdk-0.1 profile: the committed empirical path inventory as checklist
  (numeric + uuid channels); numeric-channel notices become single-notice
  Tenders; unknown new paths quarantine + inventory extension workflow.
- ADR-0003 merge: exact notice/procedure UUID equality merges DÖE+TED
  Tenders; per-field-class precedence; merge is a projection concern
  (re-projection can undo).
- Fixtures from the VPS sample months.

Acceptance: one DÖE day ingests; a known DÖE↔TED pair (e.g. 373130-2026)
resolves to ONE Tender with TED publication identity + DÖE national codes;
below-threshold islands appear as Tenders.

## Comments

2026-07-19 — The **fetcher half only** is done (`crates/ingest/src/doe.rs` +
a `doe` subcommand on the fetch CLI). No parsing, no profiles, no merge —
those remain the bulk of this issue.

Landed:
- `doe::monthly(base, year, month)` and `doe::day(base, date)` building the
  `eforms.zip` export URLs; source `doe`, kinds `monthly`/`daily`, rel_paths
  `doe/monthly/YYYY-MM.zip` and `doe/daily/YYYY-MM-DD.zip`.
- `doe::months_through(end)` — the backfill walk from `FIRST_MONTH` (2022-12).
- CLI: `fetch doe --day YYYY-MM-DD | --month YYYY-MM | --backfill`. Backfill
  skips closed months via the registry without downloading and always
  re-fetches the current month, which keeps accumulating at T+1.

Verified against the live API on the VPS (real fetches into /data/archive):
- `--day 2026-07-18` → Fetched, 1,829,595 B, 567 entries, `<uuid>-NN.xml`.
- `--month 2026-06` → Fetched, 93,887,752 B, 23,398 entries. Both counts
  match docs/research/german-portals.md exactly; ZIPs pass `testzip()`.
- Re-run of both → Unchanged; `--refetch` re-downloads and still Unchanged
  (hash match).
- 400s (today, future day, 2022-11) → `Outcome::Rejected`, exit 0, ~0.07 s.

One change outside the DÖE files was needed in `fetch.rs`: `download()`
retried *every* non-404 error three times with backoff, so a permanent 400
became a ~6 s retry loop. Client errors are now never retried, and 400 maps
to the new `Outcome::Rejected` alongside `NotFound`. This is the root fix
rather than special-casing DÖE, and it applies to TED equally.

Not done here, deliberately: a full `--backfill` run was not executed.
It is ~44 requests / ~3 GB, and german-portals.md §9 lists "undocumented
rate limits" as an open question to clear with
support@datenservice-oeffentlicher-einkauf.de first.

Blocker found for whoever takes the parsing half (see also issue 05/14):
the production DB `/data/db/tender-db.db` can no longer be opened by current
code — `Parse error: invalid expression in CREATE INDEX: parse_state`. The
store's migrations are `CREATE TABLE/INDEX IF NOT EXISTS` only, so a DB
created before issue 03/04 never gains the new `notices` columns and then
fails on the index over the missing column. Fresh DBs are fine. Needs either
a real column-migration path or a documented recreate step. My verification
therefore registered into `/data/db/doe-verify.db`; the archive files under
/data/archive/doe/ are real and will re-register (hash-idempotent) once the
production DB opens again.
