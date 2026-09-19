# 198 — dashboard: per-BT coverage report (stored values by field id)

Status: backlog (Lennart 2026-08-14: "idk if we need that, but if you want it create an issue")
Kind: observability / dashboard panel
Relates to: ADR-0002 (all business terms, no omissions), the completeness harness in
crates/ingest/tests/eforms.rs, issue 40 (ledger pattern for making claims inspectable)

## Verify

    curl -s --max-time 20 https://tenders.zebreus.click/api/dashboard | python3 -c 'import sys,json; d=json.load(sys.stdin); print([k for k in d if "bt" in k.lower() or "field" in k.lower()])'

- **done**: a key naming per-field (per-BT) coverage on the dashboard payload — the panel exists
- **open**: `[]` (read 2026-09-19) — no per-BT coverage panel; the claim stays enforced at build time only

## What

The "every business term is represented" claim is currently enforced by the build-time
harnesses (every SDK field has a mapping decision; every xpath folds into the index) and
the runtime quarantine gate — but it is not *inspectable* from the dashboard: nobody can
see, per BT id, how many stored values exist across the corpus. A per-BT coverage panel
would make the claim auditable at a glance and surface odd distributions (a BT with zero
values across millions of notices is either genuinely never published or silently
mis-claimed — today only the first explanation is checkable without SQL).

## Sketch

- One bounded aggregate per value table: `SELECT field_id, count(*) GROUP BY 1` over
  notice_texts/codes/amounts/dates/integers/numbers/ids/classifications (union, cached in
  the coverage cell like the other heavy measures, refreshed behind the issue-53 job gate).
- Render grouped by BT family (BT-xxx / OPT-xxx / OPA-xxx / UBL-xxx / SDK01-xxx / DE1-xxx),
  the synthetic prefixes visibly separated from SDK-declared ids.
- The Decision::label() plumbing ("the value tables a decision writes into, for the
  completeness report") already anticipates this report — the enum knows each field's
  table.

Not urgent (Lennart is lukewarm); pick up when the dashboard is next touched or an audit
needs it. Bounded: one query per table + a panel.
