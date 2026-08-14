# 202 — a corrupt UTF8 twin suppresses its readable ISO: 2005-04-09 lost whole

Status: DIAGNOSED (fix design below)
Kind: coverage gap, ~900–1,000 notices, fully recoverable from the archive
Relates to: 201 (found attributing the unreadable-zip bucket), 181 (the supersedence policy)

## What (verified on the box, 2026-08-14)

The 2005-04-09 daily package carries BOTH `EN_20050409_070_ISO_ORG.ZIP` (valid PK magic,
readable) and `EN_20050409_070_UTF8_ORG.ZIP` (corrupt: no EOCD). The ISO-vs-UTF8
supersedence is decided from ENTRY NAMES alone (`PackageContext::from_entry_names`), so
the readable ISO was skipped as "superseded" and the UTF8 twin then died
`unreadable zip bundle` — the whole day has ZERO notices in the corpus
(`member_path LIKE '%20050409%'` → 0) while every neighbor day has ~900–1,100.
The other 7 unreadable-zip rows are non-EN siblings / a cf companion of covered days —
duplicates, documented-keep candidates once this fixes.

## Fix design

In reprocess mode the held set already names the members that FAILED: pass it into
`PackageContext` so a held (= previously failed) EN UTF8 member does not count as a
superseding twin — the readable ISO then dispatches and the day reclaims from it. The
corrupt UTF8 row then either skips by policy (superseded-by-ISO-fallback) or stays as a
documented corrupt-at-source keep whose content survives via ISO. Plain-ingest behavior
unchanged (single-pass walk cannot know a later member is corrupt); the reclaim path is
where recovery happens, which is exactly the reclaim program's job. Test: synthetic
package with a corrupt UTF8 EN and a readable ISO EN — plain process quarantines the
UTF8 whole; a reprocess of the bucket ingests the day from ISO.
