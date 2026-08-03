# 103 — a shrinking rewrite orphans `lots` / `lot_results` / `bids` / `contracts` rows

Status: open — KNOWN WART, not reachable today. Surfaced while implementing issue 99; deliberately not
fixed in that batch.
Kind: correctness (latent) / data hygiene
Blocked by: —
Relates to: 99 (the forced rewrite that made this worth writing down), 93 (`retire_tender_tx`, which
does delete these tables), 85/98

## The wart

`delete_version` clears the twelve version-keyed satellites **and** the `tender_versions` row, but not
the four **tender-scoped entity** tables:

```rust
// delete_version deletes: tender_version_* (12) + tender_versions
// it does NOT delete:     lots, lot_results, bids, contracts
```

That is deliberate and load-bearing. Those four carry surrogate ids referenced by
`tender_version_lots`, `tender_version_lot_results`, `tender_version_bids` and
`tender_version_contracts`, and the write path re-links them by natural key
(`UNIQUE(tender_id, lot_key)` etc., via the `lot_lookup`/`bid_lookup`/… prepared statements). Leaving
them in place is exactly why a rewrite reuses the same lot/bid/contract ids and is therefore
**byte-identical** rather than merely equivalent — which is what issue 99's
`an_epoch_forced_rewrite_reproduces_identical_content` gate depends on.

The wart is the other direction: if a rewrite produces **fewer** entities than the stored version had —
a lot dropped, a bid removed — the old row stays in `lots`/`bids`/… with nothing referencing it. It is
invisible to the API (every read path joins through the version-keyed satellites) but it is dead weight
in the table and in the `UNIQUE` index, and it means `SELECT COUNT(*) FROM lots` overstates reality.

## Why it is not reachable today

A shrinking rewrite requires the projection to produce fewer entities for the same notice set — i.e. a
projection-logic change that *removes* mappings. Every change so far has been additive: issue 85 added
DE-1.x facts, issue 98 adds only party roles and touches no lot/bid/contract structure. The issue-99
epoch re-fold rewrites the whole DE-1.x cohort, but produces a superset, so nothing is orphaned.

It becomes reachable the first time a mapping is **removed or narrowed** — which the 86/48/88 semantics
batch could plausibly do.

## Fix sketch

Either delete the four entity tables' rows for the Tender before rewriting and let the write path
recreate them (simple, but renumbers surrogate ids and would break byte-identity — so it would have to
be paired with the ids being derived rather than surrogate), or sweep entities not referenced by any
surviving version after a rewrite (`DELETE FROM lots WHERE tender_id = ? AND id NOT IN (SELECT lot_id
FROM tender_version_lots WHERE tender_id = ?)`), which preserves ids and is the cheaper, safer shape.

The second is preferred. It should run only on the rewrite path, and its gate is the same one issue 99
uses: a forced rewrite under unchanged logic must still be byte-identical, so the sweep must be a no-op
when nothing shrank.

## Note

Recorded because it is precisely the kind of latent wart that gets discovered as a mysterious count
discrepancy months later. It costs nothing to know about, and the fix is cheap when it is actually
needed rather than speculative now.

## The invariant currently HOLDS corpus-wide — measured 2026-08-03

Issue 115's cross-Tender decoration bug (`summarise` keyed on `lot_id` alone rather than
`(tender_id, seq, lot_id)`) was reachable **only** if some satellite row carried a `lot_id` belonging to
a different Tender — i.e. only if this issue's orphaned rows actually existed. That made 103 the
mechanism that would decide whether a shipped defect was live or latent, so it was measured directly
against the serving database.

Full walks of all three lot-scoped satellites joined to `lots`:

```sql
SELECT COUNT(*) FROM <satellite> s JOIN lots l ON l.id = s.lot_id
 WHERE s.lot_id IS NOT NULL AND l.tender_id <> s.tender_id;
```

| satellite | rows with a foreign `lot_id` |
|---|---|
| `tender_version_texts` | **0** |
| `tender_version_amounts` | **0** |
| `tender_version_dates` | **0** |

**Not one satellite row anywhere in the corpus references a Lot belonging to another Tender.**

Two consequences worth recording:

1. **This wart is still latent, as the Status line says — now with a corpus-wide measurement behind it
   rather than an argument.** No shrinking rewrite has yet orphaned a row that another Tender's version
   then picked up.
2. **The `a39d53a` fix for 115 is therefore PREVENTIVE, not remedial.** It closed a path that the data
   could not currently walk. That is the right thing to have shipped — the guard belongs in the code
   regardless — but the record should not imply production was serving cross-Tender values. It was not.
   Independently corroborated end-to-end: 1,700 served lots across 11 pages spanning 10–117 distinct
   Tenders each, recomputed against the pre-fix three-column semantics, showed zero divergence on both
   the buggy build and the fixed one.

**This is a measurement of today, not a guarantee.** The moment a shrinking rewrite lands without the
sweep described above, the invariant can break — and 115's decoration is no longer the thing that would
notice, because it now matches on all three columns. So the value of this number is that it bounds the
past, not the future: it says the fix was preventive, and it says nothing about whether this wart stays
harmless.
