# 04 — Canonical projection + versioning + change log

Status: ready-for-agent
Blocked by: 03

Goal: parsed notices project into the versioned canonical layer (ADR-0001)
and every canonical write appends to the change log — the cursor spine
exists.

Scope:
- `store`: `tenders` + `tender_versions(tender_id, seq, caused_by_notice_id,
  published_at, …)` + version-keyed satellites (lots, texts, amounts,
  classifications, parties); `organizations` + `organization_mentions`;
  `changes(cursor AUTOINCREMENT, entity_kind, entity_id, version_seq, op,
  changed_at)` written in the same transaction. Current-state SQL views.
- `ingest`: deterministic projection: group parsed notices by BT-04 (TED)
  / notice UUID chains (DÖE later) / island rule (single-notice Tenders);
  new notice ⇒ new tender_version; defensive lot resolution per version
  (FA/DPS relabeling); org mentions always, canonical org merge only on
  exact normalised official identifiers (plausibility gates from
  ted-legacy-mapping.md).
- Rebuildability: a `reproject` CLI that rebuilds the canonical layer from
  the notice layer from scratch (appending new versions/cursor rows).
- Tests: fixture chains (CN → corrigendum → CAN) assert version sequences,
  change rows, island tenders, org mention dedup.

Acceptance: the fetched day projects into Tenders with correct version
counts; reproject is idempotent-in-content (same canonical state, new
cursor rows); `cargo test` green.
