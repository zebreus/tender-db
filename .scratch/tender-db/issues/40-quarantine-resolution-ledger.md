# 40 — Quarantine resolution ledger on the dashboard

Status: ready-for-agent

User request (Lennart, 2026-07-21): as quarantine categories get fixed
and reprocessed, their counts go to zero and the story disappears. The
dashboard should keep the decisions visible and traceable: what each
category was, how it was diagnosed, how it was fixed, and what came
back.

Design:
- Quarantine rows already survive reprocessing (reprocessed_at) — so
  "ever held / reclaimed / outstanding" per category is derivable live.
- The narrative is source-controlled knowledge: a curated ledger file
  in the app (e.g. crates/app/data/quarantine-ledger.*) keyed by
  reason (+ optional detail pattern), each entry: category, class at
  diagnosis, diagnosis summary (one sentence), fix reference (issue +
  rev), resolved date. Adding a ledger entry becomes part of landing a
  quarantine fix (note it in docs/agents or the issue template).
- Dashboard: a "Resolved categories" section of the quarantine panel —
  rows persist at zero outstanding, showing category, diagnosis, fix
  ref, reclaimed N (live: reprocessed count matching the key),
  resolved date. Open categories keep their current rows; the ledger
  section grows over time into the audit trail.
- Seed the ledger with what's already known: issue 31's two fixes
  (r208 @REASON, text RP) once reprocessed, and later 35/36.

Acceptance: a category driven to zero still tells its story on the
dashboard (diagnosis, fix, reclaimed count, date); ledger entries are
source-controlled and reviewable; no request-path scans (counts via
the background refresher).
