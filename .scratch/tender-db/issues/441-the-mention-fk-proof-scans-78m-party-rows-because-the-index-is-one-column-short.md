# 441 — deleting one organization mention scans all ~78M party rows: turso's FK proof uses only an index of the FK's exact shape, and the party tables' index is one column short

Status: ready-for-agent — BUILT 2026-09-27 (gate pending at filing); deploys with the 434–440 bundle; the deploy's
auto-Reindex builds the two new indexes AHEAD of the resumed job 1596. Filed 2026-09-27 from the hourly OPERATE step:
job 1596 (the r209/r208 re-parse for 393/397) stalled at fetch 28.
mine to take.
Kind: performance (write path), root cause of a cost three issues measured and routed around
Relates to: 247 (measured the ~10 s mention delete, deferred the FKs, concluded "the enforcement path rather than a
missing index" — wrong, see below), 248 (the mention keep-set, built to AVOID the 2.2 s delete), 100 (added the
one-column `tender_version_parties_mention` for the re-parse's party DELETE), 352 (the merge loops' per-delete FK
proof, bracketed off rather than explained), 393 (unit 1 drops `TRANSLITERATED_ADDR` sections — the re-parse that hit
this), 434 (the fold now refreshes stale mentions instead of relying on the keep-set)

## Observed (2026-09-27 ~15:50 UTC, prod)

Job 1596 (`reparse ted-export-r209,ted-export-r208`) sat at package 4/163, fetch 28 (`2024-02.tar`), for over an
hour: `members_done` moved 64 in 50 s (~1.3 notices/s). The job thread was pinned at 100 % CPU and reading ~100 MB/s
through `pread`. `/metrics` over 30 s: 18 notices re-parsed, and the whole +36.2 s landed in
`tender_db_reparse_clear_statement_seconds_total{stmt="organization_mentions"}`, about 2 s per notice.

That statement is `DELETE FROM organization_mentions WHERE notice_id = ? AND section_id = ?`. It runs only for a
mention whose section the new parse no longer creates, and issue 393 unit 1's `Rule::Ignore` for
`TRANSLITERATED_ADDR` makes exactly those: every legacy notice carrying the transliteration block drops one mentioned
section. The windows measured on 393 put the block on 4.6–9.2 % of buyer tenders, so hundreds of thousands of notices
× ~2.2 s means days of re-parse.

## Root cause (read in `turso_core` 0.7.2, confirmed by the compiled program)

`tender_version_parties` and `tender_version_bid_parties` carry `FOREIGN KEY (mention_notice_id, mention_section_id)
REFERENCES organization_mentions`. With foreign keys ON (the writer), deleting a mention compiles a child probe per
party table. `translate/fkeys.rs` `emit_fk_parent_key_probe` picks a child index only when
`ix.columns.len() == child_cols.len()` and every column matches in order. Otherwise it falls back to
`table_scan_match_any`, a `Rewind` over the whole child table.

The party tables had `tender_version_parties_mention (mention_notice_id)`, one column where the FK has two. `EXPLAIN`
of the mention DELETE on a local schema showed `OpenRead tender_version_bid_parties` + `Rewind` and `OpenRead
tender_version_parties` + `Rewind`. After the change it shows `OpenRead …_mention_key` + `Found`.

So 247's reading ("reads of the same shape seek, so it is the enforcement path, not a missing index") was half
right. The enforcement path is what ignores the index, but only because the index has the wrong shape.
`defer_foreign_keys` only moves the same scan to COMMIT, which is why it "did not move" anything.

## The fix (built)

- `DEFERRED_TENDER_INDEXES`: `tender_version_parties_mention_key` and `tender_version_bid_parties_mention_key` on
  `(mention_notice_id, mention_section_id)`. These are new names, because the missing-index check compares names.
  They replace the narrow pair: the prefix still serves the re-parse's notice-wide party DELETE (`SeekGE`, pinned by a
  test).
- `RETIRED_TENDER_INDEXES`: the narrow pair, dropped by `build_tender_indexes` only once each replacement exists (no
  box is ever without one), and by `strip_tender_indexes` before a rebuild's fold (so it is never maintained through
  one).
- Foreign-key enforcement stays ON. The proof is kept and made cheap; nothing is bracketed off.
- Comments in `reparse_notice` / `clear_parsed` that called the cost un-indexable now say what it was.
- Tests (`crates/store/tests/mention_fk_probe.rs`, reading the compiled program, not a timing):
  - `a_mention_delete_proves_its_foreign_keys_by_index`: opens both `_mention_key` indexes, no `Rewind` of either
    party table, and `FkCounter` still compiled, i.e. enforcement is on. It fails on the old definitions: the
    exploratory run printed the two `Rewind`s.
  - `the_notice_wide_party_delete_still_seeks`
  - `the_narrow_mention_indexes_are_retired_once_replaced` (build and strip)

Cost on prod: two bulk index builds over ~78M and ~67M rows, under the 240M `MAX_AUTO_INDEX_ROWS` cap. The deploy
queues them as the auto-Reindex ahead of the resumed 1596, which restarts at fetch 28 (its cursor is the last finished
package).

## Verify

    ssh -o BatchMode=yes root@zebreus.click 'curl -s https://tenders.zebreus.click/metrics | grep -E "reparse_notices_total|clear_statement_seconds_total\{stmt=\"organization_mentions\"\}"'

Read it twice, 60 s apart, while a re-parse that drops mentioned sections runs (1596 on the r209/r208 legacy era).

- **done**: seconds per re-parsed notice attributable to `organization_mentions` well under 0.1 (was ~2), and
  `/admin/jobs` shows 1596 advancing packages at minutes each, not hours
- **open**: ~2.0 s per notice (read 2026-09-27 15:52 UTC: +18 notices, +36.2 s in 30 s)
