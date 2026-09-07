# Recorded Find a Tender API responses (issue 342, 2026-09-07)

Fetched live by the unit-1 verifier at ~15 s spacing, headers in `p1.headers.txt`:

- `2026-09-03-p1..p5.json.gz` — `GET /api/1.0/ocdsReleasePackages?updatedFrom=2026-09-03T00:00:00&updatedTo=2026-09-03T23:59:59`, pages 1–5 via `links.next` (100/100/100/100/36 releases = the whole day, 436 releases, 398 ocids).
- `2026-09-03-n083685.json.gz` — `GET /api/1.0/ocdsReleasePackages/083685-2026`, an amendment-bearing UK15 release whose tender and awards carry only their deltas.
- `2026-09-03-dec2020.json.gz` — the December 2020 window: `releases: []` (FTS holds nothing before January 2021).
- `ocds_uk_extension-release-schema-2026-09-07.json` — the Cabinet Office UK extension's release schema as fetched that day.

Test fixtures under `crates/ingest/tests/fixtures/fts/` are cut from these; this directory is the untrimmed source.
