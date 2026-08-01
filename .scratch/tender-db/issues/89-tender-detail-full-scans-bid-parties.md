# 89 — `/v1/tenders/{id}` full-scans `tender_version_bid_parties` on every request

Status: FIXED in working tree (`a572544`), awaiting deploy + reindex — DISCOVERED 2026-08-01 (proj-fix, during the post-recovery verification pass)
Kind: correctness / performance (missing index — public API outage)
Blocked by: —
Relates to: 82 (the same shape on `tenders_current_published`), 83, the `tender_version_parties_version` fix at canonical.rs:205-210

## Symptom

`GET /v1/tenders/{id}` times out (>60s, no response) for **every** tender id. Reproduced on prod rev
`484e9204` against ids 1, 4,000,000 and 8,105,902. The list endpoint is unaffected
(`/v1/tenders?limit=3` returns in 0.088s), so this is specific to the detail endpoint.

## Root cause

`read::tender_detail` reads each satellite by `(tender_id, seq)`, including (read.rs:794-800):

```sql
SELECT p.bid_id, p.role, p.organization_id, o.name
  FROM tender_version_bid_parties p JOIN organizations o ON o.id = p.organization_id
 WHERE p.tender_id = ? AND p.seq = ?
```

`tender_version_bid_parties` (canonical.rs:348-361) has **no PRIMARY KEY** and exactly one declared
index — `tender_version_bid_parties_org` on `organization_id`, which serves org→bids, not
tender→parties. So the table has **no index usable for this read** and every detail request
full-scans it.

It is the only satellite in that state:

| table | serves `(tender_id, seq)` |
|---|---|
| texts / amounts / dates / classifications / parties | `_version` index |
| lot_results | PK `(tender_id, seq, lot_result_id)` |
| bids | PK `(tender_id, seq, bid_id)` |
| result_winners | PK `(tender_id, seq, lot_result_id, organization_id)` |
| result_stats | `_version` index |
| **bid_parties** | **nothing** |

This is precisely the gap canonical.rs:205-210 records closing for `parties` — *"the by-version index
its siblings all carry: parties was the lone satellite without one"*. `bid_parties` was missed by
that fix, so it inherited the title.

## Why it surfaced now, not earlier

Pre-existing, but latent while the table was small. The recovery rebuild folded the reclaimed corpus
into it, and a full scan of the grown table crossed the request timeout. Same shape as issue 82: a
missing index that only becomes an outage at full-corpus scale.

## Fix (`a572544`)

A tenth `DEFERRED_TENDER_INDEXES` entry:

```rust
("tender_version_bid_parties_version", "tender_version_bid_parties(tender_id, seq)"),
```

**Deferred rather than schema-batch, and NOT for the const's usual reason.** `(tender_id, seq)` is
append-mostly, not a random key, so it does not meet the issue-60 criterion the other nine meet. It
is deferred for the **boot-path** reason: a schema-batch `CREATE INDEX` would rebuild it over the
whole table at every `Db::open`, re-creating the multi-hour boot that issues 82/83 just removed. The
code comment says so explicitly, so it is not later "corrected" into the schema batch.

The pre-existing reindex op builds it — `build_tender_indexes()` is a `CREATE INDEX IF NOT EXISTS`
loop, so a follow-up reindex skips the nine present and builds only this one.

## Validation

After deploy + reindex: `GET /v1/tenders/{id}` returns promptly for a sample of ids across the id
range, and the response renders the full detail (texts, amounts, dates, classifications, parties,
lots, bids, results). Assert all 10 `DEFERRED_TENDER_INDEXES` present in `sqlite_master`.

## Note

Found by attempting the routine post-recovery spot-check (verification task #2), which is what a
"render 3-5 tenders fully" check is for — the endpoint had been down for an unknown period with
nothing reporting it, because nothing exercises it automatically. Worth considering whether the
health check should touch one detail read, so a satellite losing its index surfaces as a health
failure rather than at the next manual spot-check.
