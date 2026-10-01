# 480 — UK Contracts Finder (below-threshold notices, 2015 →) is not ingested

Status: ready-for-agent — filed 2026-10-01 from closing 342, whose 2026-09-07 decision named Contracts Finder as "a
later, deduplicated unit" beneath FTS. The first unit is research: measure the feed, find a deduplication key
against FTS, and decide go or no-go with numbers. Start it after 477's top-up lands, so the FTS half it deduplicates
against is complete.
Kind: new source (UK, below threshold)
Relates to: 342 (FTS, the above-threshold half; its decision chose the UK as the first market beyond TED and DÖE),
477 (FTS completeness), 448/452/453 (GB organization identity, which Contracts Finder suppliers would share)

## What is missing

FTS covers the UK above the procurement thresholds from 2021. Below the thresholds, and for the whole UK before FTS
existed, the official channel is **Contracts Finder**. 342 recorded its OCDS feed from 26 Feb 2015, an open licence,
and the same Companies House identifiers. tender-db holds none of it, so a UK buyer's smaller contracts, often the
majority by count, are invisible. A UK supplier's history reads only its above-threshold wins.

342's decision recorded the hard part: **Contracts Finder duplicates FTS from 2021 to Feb 2025 with no machine
link.** Notices above the threshold were published to both, so a naive import would double-count every
above-threshold UK procedure of that period.

## First unit: research (one firing), then decide

1. The API and bulk channels: release schema, paging (check for the same cursor defect as 477: never assume
   `links.next` is complete), rate limits, history depth, volume per year, and the licence statement to serve
   (`data_sources`, issue 446).
2. **The deduplication key against FTS for 2021–2025-02.** Look for a published cross-reference (an FTS notice id in
   the release, a shared ocid prefix, a `relatedProcesses` link). If there is none, measure a deterministic match
   (buyer identifier + title + publication date ± 1 day + value) on a sample of known-duplicated pairs, and count
   its precision before trusting it. ADR-0003 governs cross-source merges: a strong explicit cross-reference only.
   A fuzzy match is not a merge key; at most it is a "skip as duplicate" signal, and it needs its own decision.
3. The decision, recorded here with numbers: go (profile, parser and backfill as for FTS), go for 2025-02 → only (after
   the Procurement Act unified publication, where the duplication may end), or no-go.

## Verify

    curl -s https://tenders.zebreus.click/v1 | jq -c '[.data_sources[].source]'

- **open** (2026-10-01): no Contracts Finder entry (`ted`, `doe`, `fts`).
- **done:** a decision recorded here. If go, a `contracts-finder` entry (or equivalent) in `data_sources`.
