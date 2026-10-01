# 478 — an FTS release that publishes `items[].deliveryAddresses` as a map, not a list, is quarantined whole

Status: **DONE 2026-10-01** — deployed at `b26cf3a`, reclaimed by reprocess 1829 (`1 reclaimed, 0 still held`) and projected by 1830. The Verify reads done at 20:48 UTC: `[]`, and the dashboard's outstanding quarantine fell from 13 to 12.
Was status: ready-for-agent — DEPLOYED 2026-10-01 at `b26cf3a` (built `91b6d59`, review fixes `91d089d`: a repeated ordinal key quarantines, and the real member is a fixture). Reprocess 1829 `1 reclaimed, 0 still held`, and project 1830 wrote its tender. Notice `002109-2021` is now `parsed` (id 47015164). NEXT: the Verify, once the dashboard's quarantine snapshot refreshes (it still listed the row at 20:3x).
Was status: ready-for-agent — filed 2026-10-01 13:5x UTC from FTS chunk 9 (process 1808: `21043 parsed, 1 quarantined`).
The first unit is the parser: accept a map-shaped `deliveryAddresses` (its values in key order) with a fixture test,
then reprocess the one notice.
Kind: ingestion (a source dialect the parser does not accept)
Relates to: 342 (FTS), ADR-0004 (strict ingestion: an unrepresentable shape is quarantined whole, and stays
reprocessable)

## What is wrong

The FTS backfill's last chunk quarantined one release. The dashboard's quarantine `recent` reads:
`unparsable-json`, profile `fts:ocds-1.1`, member `002109-2021.json`, `invalid type: map, expected a sequence at
line 69 column 45`.

The member (13,499 bytes, in `/data/archive/fts/monthly/2021-02.zip`) publishes, at lines 66–73:

```json
"items": [
    {
        "id": "1",
        "deliveryAddresses": {
            "1": {
                "region": "UK"
            }
        },
        "relatedLot": "1"
    }
],
```

OCDS defines `deliveryAddresses` as an array of Address objects. This release has a map keyed by the address's
ordinal instead. The value is still one address with one field, so the information can be represented. Only the
container is off-schema, and serde refuses it, which quarantines the whole notice.

It is 1 of the 314,156 FTS releases held on 2026-10-01. Other map-for-array spellings of the same dialect may exist
in other fields and simply not be among the releases we hold yet (see 477, ~14,000 missing).

## Proposed fix

- In `crates/ingest/src/fts/parse.rs`, deserialize `deliveryAddresses` with a helper that accepts a sequence, or a
  map whose values it takes in key order, numeric keys numerically. Record the map form in the notice's parse notes
  if the parser keeps such notes; it must never be silently different from a sequence. Use the same helper for any
  other OCDS array field the FTS model reads, if the source is seen to spell those as maps too (grep the archive
  for `": {\n *"1": {` before generalising).
- Fixture: this release (`002109-2021.json`) under the FTS fixtures. The test asserts it parses, with one delivery
  address of region `UK`.
- Then reprocess: `{"kind":"process","source":"fts", …}` scoped to the 2021-02 package (or a reparse of the
  quarantined member through the reclaim path), then `project`. The quarantine count for `fts:ocds-1.1` goes to 0.

## Verify

    curl -s https://tenders.zebreus.click/api/dashboard | python3 -c "import json,sys; q=json.load(sys.stdin)['quarantine']; print([r['member_path'] for r in q['recent'] if r['profile']=='fts:ocds-1.1'])"

- **open** (2026-10-01): `['002109-2021.json']`.
- **done:** `[]`.

## 2026-10-01 20:3x UTC — deployed and reclaimed

- `list_or_ordinal_map` reads an ordinal-keyed map in key order, numeric keys numerically, and refuses a repeated
  key. The real member `002109-2021` is a fixture.
- Job 1829: `1 package(s): 1 reclaimed, 0 still held`; job 1830 projected it. `/v1/sql` reads notice 47015164
  `parsed`.
- The dashboard's `quarantine.recent` still showed the row a minute later. `recent_quarantine` filters
  `reprocessed_at IS NULL`, so this is the memoized snapshot, not the data. Re-read next firing.

## RESOLVED-VERIFIED 2026-10-01 20:48 UTC

The dashboard's quarantine snapshot refreshed: `quarantine.recent` holds no `fts:ocds-1.1` row, and `outstanding` reads
12 (13 before). Notice `002109-2021` is parsed (id 47015164).
