# 02 — Notice extraction, profile dispatch, quarantine skeleton

Status: ready-for-agent
Blocked by: 01

Goal: packages from the archive are opened, each notice file is identified
and dispatched to a mapping profile, and Notice identity rows exist — with
quarantine as the only failure mode.

Scope:
- `store`: `notices` table (source, publication_id, content_hash, profile,
  declared_version, package ref, file path/offset, ingested_at) with the
  identity key (source, publication_id, content_hash); `quarantine` table
  (notice ref/raw ref, profile, reason, detail, first_seen, reprocessed_at).
- `ingest`: package walker (tar.gz for TED) + per-file dispatch on root
  element + CustomizationID → profile id (text / ted-export-r208 /
  ted-export-r209 / eforms:<customization>); unknown → quarantine.
- Publication-id extraction per era (OJS number; eForms BT-701+publication
  number). No field mapping yet.
- `process` CLI: walk archive → notices + quarantine; idempotent re-runs.

Acceptance: processing the fetched day from issue 01 yields one notice row
per file with correct profile split (compare counts against
docs/research/ted-access-channels.md era findings), zero silent drops
(files == notices + quarantined).
