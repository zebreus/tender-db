# 202 — a corrupt UTF8 twin suppresses its readable ISO: 2005-04-09 lost whole

Status: RESOLVED (2026-08-14, two rounds — see bottom)
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

**2026-08-14 ~19:2x CEST box time (orchestrator) — fix deployed (rev b4a38c5), recovery
running.** The dispatch context now excludes ledger-held 'unreadable …' whole bundles from
the supersedence decision, on plain ingest AND reclaim (store.unreadable_bundle_members;
whole-bundle paths only, record/file-level holds never match). End-to-end test: first walk
loses the day + holds the bundle, second walk ingests from ISO. Recovery = process ted
monthly 2005-04 (running, job 1) + fold (queued); expect ~900 notices for 2005-04-09.
After it verifies: ledger entry for the unreadable-zip class (1 recovered day + 7 sibling
duplicates, documented keep).

**2026-08-14 ~19:5x CEST — round 1 recovery FAILED, round 2 deployed and VERIFIED.**
Round 1 (b4a38c5) recovered 0 notices: the en_utf8_text/en_utf8_cf flags were
package-global, and in the 2005-04 MONTHLY every other day's readable UTF8 kept the flag
true — the excluded day's ISO stayed suppressed. (The synthetic test used a single-day
daily, where the flag flips; it could not see this.) Round 2 (b3f67b6): supersedence is
keyed PER PUBLICATION DAY — PackageContext holds HashSets of YYYYMMDD tokens, an ISO is
superseded only when ITS day ships a non-held UTF8 twin of the same delivery class. The
end-to-end test now carries a second day with a readable UTF8 (the monthly shape).
Recovery verified on the box: job 689 `process ted monthly 2005-04` → 932 parsed,
0 quarantined; `member_path LIKE '%20050409%'` → 932 notices. Ledger entry
"Corrupt zip bundles in the TED archive (EOCD missing)" added (8 rows stay held as the
record of the corrupt bytes; content covered — EN day via ISO twin, 7 non-EN siblings
via the language policy).
