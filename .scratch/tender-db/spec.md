# tender-db v1 — spec

Owner: Claude (full ownership per Lennart, 2026-07-19). Requirements source:
CONTEXT.md (binding); design: docs/architecture.md; evidence:
docs/research/SUMMARY.md. This spec adds only the acceptance criteria.

## Definition of done (v1)

1. **Data**: full TED backfill 1993→ (text era header-only, EN files) and
   DÖE backfill 2022-12→, continuously updated (TED daily after 09:30 CET,
   DÖE T+1), with per-profile completeness checklists green and quarantine
   visible on the dashboard. Cross-source UUID merges active.
2. **API**: `/v1` REST (tenders, lots, organizations, notices, changes),
   SSE snapshot+diff with Last-Event-ID resume on collection endpoints,
   account-gated read-only SQL endpoint, minimal webhooks. Anonymous access
   per decided limits.
3. **Dashboard**: coverage per Source/profile/year vs measured ground truth
   (yearly-counts from research), quarantine + unchained-awards + junk-org-id
   quality metrics, account lifecycle (register, login, tokens, delete).
4. **Deployed**: tenders.zebreus.click serving publicly from the VPS
   (Ubuntu + systemd + nginx TLS), data on /data, importer running on
   schedule, AGPL source link at API root + dashboard footer.
5. **Verified**: `nix flake check` green; integration tests over fixture
   packages; production spot-checks (known notices resolve correctly through
   API for every era); API counts vs TED Search API counts for sample days.

## Explicit non-goals (v1)

PDF attachments; fuzzy org matching; webhooks beyond minimal; OpenAPI/utoipa;
multilingual labels beyond EN; off-box backups; Reviews/E5 as canonical
entities; heuristic text-era body extraction.

## Issues

Implementation slices live in `issues/` (tracer bullets — each lands a
working end-to-end increment). Dependency edges via `Blocked by:` lines.
