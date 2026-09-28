# 437 — FTS award releases publish CPV and region only on `awards[].items`, which the parser drops

Status: RESOLVED-VERIFIED 2026-09-28 — verified after the FTS re-parse (1600) and the full fold (1616). Was: DEPLOYED 2026-09-27 14:40 UTC (rev `9dedf49`, commit `cef344b`); FTS re-parse queued as job 1599 (`reclaim_only`), folded by the one full fold. Was: **BUILT 2026-09-27** (see the foot): the walk reads every non-delta award's items the way
it reads the tender's, and the checklist maps the path. Uncommitted, not deployed. The standing rows need the gated
`fts:ocds-1.1` re-parse and fold. Filed 2026-09-27.
Kind: coverage (ingest — `crates/ingest/src/fts/parse.rs`'s item walk and `struct Award`, and the ADR-0004
checklist entry in `crates/ingest/src/fts/checklist.rs`)
Relates to: 386 (unit 2b built the checklist that recorded this path as Ignored on a false premise; its fixtures
028961-2025 and 083650-2026 are this issue's witnesses), 342 (the FTS crosswalk, `342-fts-plan` §3b item 3 set the
BT-262/263 rule this reuses), ADR-0004 (mapped-or-ignored)

## Verify

    curl -s --max-time 20 https://tenders.zebreus.click/v1/tenders/7956308 | python3 -c "import sys,json; d=json.load(sys.stdin); print(sorted((c['scheme'], c['code']) for c in d['classifications'] if c['scheme'] in ('cpv','nuts')))"

- **done**: `[('cpv', '48000000'), ('nuts', 'UK')]`. Tender 7956308 is 028961-2025 (issue 386), a UK7 whose only CPV and
  region are on its award item. Needs the `fts:ocds-1.1` re-parse and fold.
- **open**: `[]`. This is the expected output, not one I read: this session had no prod access. The parse layer has
  never emitted these rows, as the red test below pins.

## Observed (2026-09-27, prod, the lead session's read)

- **The newest 100 UK rows:** 52 carry no CPV, and **all 52 publish one on `awards[].items[].additionalClassifications`**.
  60 carry no region; **41 of those 60 publish one on `awards[].items[].deliveryAddresses[].region`**. The other 19
  publish no region anywhere, which is source reality.
- **A 2026-09-22..24 window, CPV coverage by form:** UK5 **0/20**, UK6 **3/119**, UK7 **1/363**.
- The UK5/UK6/UK7 award releases carry **no `tender.items`** at all. They publish the CPV and the NUTS region per
  award item.

The committed fixtures show the same shape: 028961-2025 (UK7) and 083650-2026 (UK6) each have `tender.items` = 0 and
one award item. The recorded pages hold two more, 083608-2026 (UK7) and 083253-2026 (UK5).

## Mechanism

- `fts/parse.rs` walked only `tender.items` for classifications (BT-262/263) and delivery places (BT-5071).
- `struct Award` had no `items` field, so serde dropped them without a trace.
- `fts/checklist.rs` recorded `awards[].items` as `Ignored("the award's items restate the tender's; classifications
  and places are read there")`. That premise is false for exactly the forms that publish no tender items. The census
  test passed because the path had a disposition, and a disposition is all the census checks.

## Built (2026-09-27)

Uncommitted, not deployed. `ops/check.sh` was not run. The focused suites below are green.

**The walk.** The per-item body is now `Walk::item`. It runs over `tender.items` first and then over every
non-delta award's `items`. `Award::is_delta` still decides "non-delta", so a UK15 `{id, amendments}` skeleton says
nothing here either. `struct Award` gained `#[serde(default)] items: Vec<Item>`.

**Scope.** The rule is the tender walk's:

- The item's own `relatedLot`, when that lot has a section.
- Otherwise, for an award item, the one lot its award names. This is the narrowness `inherited_periods` already
  applies, so a multi-lot award's lot-less item is procedure-wide.
- Otherwise the procedure.

An item that names a lot is never moved to its award's lot. Within each item, the first classification is BT-262
and the rest are BT-263. All four fixture award items name their lot.

**No double count.** The fold's fact set (`BTreeSet<Fact>`) already dedupes an identical `(field, scheme, code)` at
one scope. It keys on the role, though, so a code the tender item made BT-263 and the award item made BT-262 would
be served twice. The walk therefore keeps a `stated` set of `(scope, scheme, code)` for classifications and
`(scope, nuts, region)` for regions:

- A tender item only records into the set. A release without award items parses byte-identically to before.
- An award item skips anything already stated at its scope, in either role. The same set dedupes several awards
  restating each other.

**Checklist.** `awards[].items` is now `Mapped`, with the old premise's refutation in a comment. Its leaves mirror
`tender.items`:

| path | disposition |
| --- | --- |
| `relatedLot` | Mapped |
| `additionalClassifications` | Mapped |
| `deliveryAddresses` | Mapped |
| `id` | Ignored |
| `additionalClassifications[].description` | Ignored |
| `deliveryAddresses[].country` | Ignored |
| `deliveryAddresses[].countryName` | Ignored |

The census `every_published_fts_path_is_mapped_or_ignored_on_record` is still green.
`the_longest_entry_wins_and_containers_cover_their_subtrees` now also pins three of the new leaves.

**Tests.** Red first, then green:

- `fts::parse::tests::an_award_releases_items_carry_its_cpv_and_delivery_region` runs on the committed fixtures.
  Lot `1` of 028961-2025 gets `cpv 48000000` and `nuts UK`; lot `1` of 083650-2026 gets `cpv 80500000` and
  `nuts UKK15`. Exactly one of each, and nothing procedure-wide. It was red with `left: None`.
- `fts::parse::tests::award_items_scope_like_tender_items_and_restate_nothing` uses a synthetic release. An award
  restating the tender's items adds nothing. A partly-new award item adds only its new code, in its own role. A
  lot-less item of a single-lot award lands on that lot, and one of a multi-lot award lands on the procedure. A
  delta award's item is ignored. It was red with `left: ["72000000"]` against `["72000000", "30200000"]`.

Also green: all of `cargo test -p ingest --lib` (275, including the projection's round over these two fixtures,
`an_fts_contract_keeps_its_published_value_and_a_contractless_award_its_date`), `--test fts` (4, including the
checklist census), `--test text` (13), `--test project` (66) and `--test data_quality` (9).

**What reaching the standing rows takes:**

1. A **re-parse of the `fts:ocds-1.1` profile** and its fold. This is the same gated operation issue 386's jobs
   1585/1587 ran. New FTS ingests carry the fix from the first daily tick after deploy.
2. **Issue 434's mention refresh is NOT needed.** No organization fact changes. The CPV and NUTS rows fold through
   the tender layer (`tender_version_classifications`, lot-scoped).
3. Then the Verify above.

Rows that publish no region anywhere (19 of the measured 60) stay without one. That is the source, not a gap.

## Verify read 2026-09-28

`[('cpv', '48000000'), ('nuts', 'UK')]` on tender 7956308 (read 2026-09-28 16:0x UTC). **done.**
