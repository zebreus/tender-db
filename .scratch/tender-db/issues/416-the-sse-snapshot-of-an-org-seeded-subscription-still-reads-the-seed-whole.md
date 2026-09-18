# 416 — the SSE snapshot of an org-seeded subscription still DISTINCTs the org's whole participation slice per page (the REST pages walk it in windows since 388)

Status: **DONE 2026-09-18** — FIXED, gated (127/127) and DEPLOYED at `ddf7a0c` 14:06 UTC, read live at 14:07 (foot): the snapshot of `/v1/lots?bidder=357` delivered **18,500 events in eight seconds** where the old shape delivered one page of 500, and the tenders snapshot of the same org completed — 1,446 rows and the `live` marker — inside the same eight seconds where it had reached 1,000. Was: ready-for-agent — found 2026-09-18 13:20Z while closing issue 388.
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

    grep -c "read_page" crates/app/src/v1/sse.rs

- **done**: `2` or more — the snapshot pages through the list's own read (`read_page`), which carries the windowed walk
- **open**: `0` — the snapshot reads `read_items` pages and ends on page length (read 2026-09-18 13:20: `0`; at `ddf7a0c`: `4`)

## Done when

- An org-seeded subscription's snapshot pages through the windowed walk (`lots_seeded_page` / `tenders_seeded_page`), with the snapshot's own end condition read from the walk's `next`/`examined_to` (`None` = done) rather than from page length — the same distinction 408 (b) drew for the bounded walk.
- The snapshot's item ORDER for the lots stream may change to `(tender, lot)`; the docs say a snapshot is a set followed by a `live` marker, so state whether any consumer contract depends on id order before changing it, and say so in `/docs` if it does not.
- A test drives an `?winner=` subscription on the chain fixture through the snapshot and gets the same item set as before, ending with the `live` marker.

## FIXED, DEPLOYED and READ LIVE 2026-09-18 — `ddf7a0c`

**What changed (`crates/app/src/v1/sse.rs`).** The snapshot loop no longer reads `read_items`
pages in id order and stops on a short page. It calls `read_page` / `isolated.read_page` — the
REST list's own page, which carries every cursor grammar and both seeded walks (388) inside it —
with the band left open (`i64::MAX`), and ends when the read hands back no next page. For every
unseeded shape that is exactly the old behaviour: a full page continues from its last id, a short
page is the last. For an organization-seeded subscription it is the windowed walk, so the seeded
lots snapshot now streams in `(tender, lot)` order; the docs' snapshot-order sentence says so.
Pinned by `an_org_seeded_subscription_snapshots_through_the_windowed_walk`: a two-row snapshot
page over the chain fixture, lots and tenders — the snapshot's `added` set is the list's one-page
answer, in its order, once each, with no SSE id inside the snapshot, then `live`.

**Live, org 357 as bidder, an eight-second bounded read of the event stream, before and after:**

| stream | before (`62b3ce5`, 14:00) | after (`ddf7a0c`, 14:07) |
| --- | --- | --- |
| `/v1/lots?bidder=357` | **500 events** (one snapshot page; the second did not arrive in time), no `live` | **18,500 events**, in `(tender, lot)` order, unique, still streaming |
| `/v1/tenders?bidder=357` | 1,000 events, no `live` | **1,446 events and the `live` marker** — the whole snapshot |

That is the pre-388 page (the org's 3.26M-row slice DISTINCTed per page, ~4 s each) against a
page that costs a page. The boot logged no deferred-index or REFUSING line and no error.

**Not changed.** The unseeded snapshot's shape and end condition (a short page is the last —
`i64::MAX` keeps the band open, so 408 (b)'s bounded walk still does not apply to the snapshot);
the `buyer` seed (id-ordered on both streams, as in 388); the docs' "read in a single consistent
transaction" sentence about the snapshot, which has not been true since the paged snapshot (55)
took one reader per page — noted here, not fixed here.

