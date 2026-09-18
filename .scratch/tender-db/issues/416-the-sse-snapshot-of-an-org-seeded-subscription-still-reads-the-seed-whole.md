# 416 — the SSE snapshot of an org-seeded subscription still DISTINCTs the org's whole participation slice per page (the REST pages walk it in windows since 388)

Status: ready-for-agent — found 2026-09-18 13:20Z while closing issue 388: the REST list pages for `winner`/`bidder` seeds now walk the `(organization_id, tender_id)` index in windows (`lots_seeded_page`, `tenders_seeded_page`), but the SSE snapshot (`crates/app/src/v1/sse.rs`, `read_items` with `Scope::Page`) still takes `tender_from`'s / `lot_seed_predicates`' `(SELECT DISTINCT tender_id FROM <table> WHERE organization_id = ?)` shape, id-ordered, once per snapshot PAGE. Filed rather than folded into 388 because the snapshot is a one-shot pass with its own contract (a short page ends the snapshot), so the windowed walk's short-page-while-more cursor cannot be dropped in without changing how the snapshot decides it is done.
Kind: performance (SSE snapshot of a prolific org's subscription)
Relates to: 388 (the windowed walk and its numbers), 408 (b) (the bounded walk deliberately left the SSE snapshot unbanded for the same reason — a short page is the snapshot's end signal), 55 (the paged snapshot), 223
Blocked by: nothing

## Observed

Not measured live — reasoned from the shapes 388 measured. A subscription to `/v1/tenders/events?winner=357` (3,734 tenders) or `/v1/lots/events?bidder=357` (137k lots) snapshots through `read_items(collection, Scope::Page { after, limit: snapshot_page })`, and each page of that is the pre-388 statement: the org's whole index slice DISTINCTed (3.55M / 3.26M rows for org 357), sorted by id, `LIMIT`ed. Before 388 that shape cost 1.25–3.7 s a page on the REST list; the snapshot pays it per snapshot page, so a 137k-lot subscription's snapshot is on the order of a thousand such pages.

## Why it matters

The SSE transport is the documented way to mirror a filtered view. For an ordinary org the snapshot is a handful of pages and fine; for the prolific class 388 was filed on it is minutes of the isolated pool per subscription, and every reconnect after `cursor_expired` repeats it.

## Repro

1. `grep -n "read_items(collection, &reader, &filter, scope)" crates/app/src/v1/sse.rs` — the snapshot's page read.
2. `grep -n "fn tender_from\|fn lot_seed_predicates" crates/store/src/read.rs` — the DISTINCT seed both take when `participation_seed` is set.

## Verify

    grep -c "seeded_lots\|seeded_tenders\|lots_seeded_page\|tenders_seeded_page" crates/app/src/v1/sse.rs

- **done**: `1` or more — the snapshot routes an org-seeded subscription through the windowed walk
- **open**: `0` (read 2026-09-18: `0`)

## Done when

- An org-seeded subscription's snapshot pages through the windowed walk (`lots_seeded_page` / `tenders_seeded_page`), with the snapshot's own end condition read from the walk's `next`/`examined_to` (`None` = done) rather than from page length — the same distinction 408 (b) drew for the bounded walk.
- The snapshot's item ORDER for the lots stream may change to `(tender, lot)`; the docs say a snapshot is a set followed by a `live` marker, so state whether any consumer contract depends on id order before changing it, and say so in `/docs` if it does not.
- A test drives an `?winner=` subscription on the chain fixture through the snapshot and gets the same item set as before, ending with the `live` marker.
