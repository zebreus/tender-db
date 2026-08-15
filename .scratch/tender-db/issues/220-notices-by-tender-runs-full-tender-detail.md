# 220 — /v1/notices?tender=X runs a full tender_detail (~17 satellite queries) and discards all but the version chain

Status: needs-triage — MEDIUM, CONFIRMED (code) 2026-08-15. Filed from the API performance review (subagent).
Kind: performance (wasted per-request work on the main reader pool)
Blocked by: —
Relates to: 115/116 (the tender_detail satellite reads this over-uses), 120 (main vs isolated pool)

## Defect

`tender_notices` — the handler for `GET /v1/notices?tender=X` — calls `read::tender_detail`
(`crates/app/src/v1/mod.rs:765`) and then reads only `detail.versions[].caused_by_notice_id`
(`mod.rs:768`). Every other thing `tender_detail` fetches is thrown away.

`tender_detail` (`crates/store/src/read.rs:927-1058`) issues ~17 satellite/results queries per call —
texts, amounts, dates, classifications, parties, all lots + `summarise`, lot_results, winners, stats, bids,
bid_parties, contracts. For the notice list only the version→notice mapping is needed, which is a single
query: `SELECT caused_by_notice_id FROM tender_versions WHERE tender_id=? ORDER BY seq`.

## Cost scenario

On the **main/fast** reader pool (`state.readers.get()`, mod.rs:764 — the pool live `/v1/tenders` traffic
shares), a normal tender wastes ~14 indexed satellite reads per request. A high-lot tender is far worse:
`lots_of` reads the tender's entire lot set (whole-corpus max ~2,604 lots) plus `summarise`'s three
full-slice reads, and `results_of` reads all lot_results/bids/contracts with their org joins — thousands
of rows fetched and discarded to return a handful of notice ids, all while holding a main-pool reader.

## Fix direction

In `tender_notices`, replace the `tender_detail` call with a direct `tender_versions` read (its
emptiness doubles as the 404 check — every real tender has ≥1 version), then the existing per-notice
lookups. Turns ~17 queries into ~2 + N-distinct-notices, and stops a `?tender=` notice list from doing
award/lot/party work it never uses.

## Verification

- Pick the fattest tender: `SELECT tender_id, COUNT(*) c FROM tender_version_lots GROUP BY tender_id
  ORDER BY c DESC LIMIT 1`. Compare `curl -w '%{time_total}' /v1/notices?tender=<id>` before/after — the
  gap is the wasted satellite work. (Bounded read; run queue-idle.)
- Behavior unchanged: same notice ids, same order, same 404 on an unknown tender id.
