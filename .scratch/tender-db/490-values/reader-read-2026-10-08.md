Read-path report for issue 490 (lot value). No files were edited and nothing was run on the box.

## 1. How `summarise` picks a lot's value

**Where it runs.** `crates/store/src/read.rs:3910` `summarise(conn, rows, lang)`. It is called from:
- `lots` (read.rs:3131)
- `lots_page` (read.rs:3217)
- `lots_previous_shape`, the oracle (read.rs:3255)
- `lots_seeded_page` (read.rs:3817)
- `tender_detail` → `lots_of` (read.rs:2899, 4182-4186)

`lots_identity` (read.rs:3142, the SSE match check) does not call it.

**Row key.** Rows are keyed `(tender_id, seq, lot_id)` (read.rs:3921-3922). This is exactly the primary key of `tender_version_lots`. It runs one set of queries per distinct `(tender_id, seq)` in the page.

**Candidates (read.rs:3989-4000).** These are lot-scoped only. There is no fallback to tender-scoped amounts, unlike the deadline.
```sql
SELECT s.lot_id, s.cents, s.currency, s.eur_cents, s.field FROM tender_version_amounts s
 WHERE s.tender_id = ? AND s.seq = ? AND s.lot_id IS NOT NULL AND s.quality IS NULL
```
Schema (canonical.rs:265-292): `cents`, `currency` and `field` are NOT NULL; only `eur_cents` can be NULL. Two consequences:
- The `cents.unwrap_or(i64::MIN)` arm (read.rs:4086) is dead code.
- The four-way `if let` before the scale rule (read.rs:4079) in effect means "only when `eur_cents` is not NULL".

**Exclusions, in order:**
1. **Withheld rows:** `quality IS NULL` in the SQL (issue 372).
2. **Sentinels** (read.rs:4022): `sentinel_amount(cents)`, canonical.rs:3015. It refuses:
   - negatives, 0, 1 and 100;
   - an all-nines run of 9 or more digits in the major unit;
   - a run of 9 or more identical digits in the cents.
3. **Ceiling** (read.rs:4031-4034): refused only if `eur_cents` is not NULL and is above `IMPLAUSIBLE_EUR_CENTS` (1e13, canonical.rs:2726). The comment explains why an unconvertible amount is not refused: the lot row serves the published figure.
4. **Exact-10ᵏ scale rule** (read.rs:4045-4083, issue 471 unit 4(a)):
   - **Gate:** the chain is loaded only if some candidate has `eur_cents >= SCALE_ERROR_MIN_EUR_CENTS` (1e11, canonical.rs:2762). Without the gate, `refuses_amount` returns false anyway.
   - **Chain query** (read.rs:4054-4064):
     ```sql
     SELECT seq, field, cents, currency FROM tender_version_amounts WHERE tender_id = ? AND seq <= ?
     UNION ALL
     SELECT seq, NULL, awarded_cents, awarded_currency FROM tender_version_lot_results
      WHERE tender_id = ? AND seq <= ? AND awarded_cents IS NOT NULL AND awarded_currency IS NOT NULL
     ```
     It takes any field, any `quality` and any scope.
   - **Partners:** every row with seq ≤ N (`add_partner`, floor above 1,000 cents).
   - **Corroboration ("head"):** every amount row at seq = N, any scope and any quality, via `add_head_amount`. Lot awards never corroborate.
   - **Refusal:** `refuses_amount(field, currency, cents, eur)` (canonical.rs:2898), applied only where `eur_cents` is not NULL. It mirrors `ScalePartners::of_chain(chain[..=N])` (canonical.rs:2853).

**Order and ties (read.rs:4084-4091).**
- The winner is the highest **raw published cents, compared across currencies**. It is not ranked by EUR.
- Ties use strict `>`, so the first row in scan order wins. With only the `(tender_id, seq)` index, scan order is rowid order.
- The fold inserts rows in this order (canonical.rs:28997-29011, 29215-29250): tender facts first, then each lot in `v.lots` order, and within a lot in `BTreeSet<Fact>` order — `(field, cents, currency, tax_basis, quality)`. So among tied maxima the lowest `field` wins, then the lowest `currency`.
- The comment at canonical.rs:29211-29213 says row order is "unobservable". That is false for this tie.

**What ends up on `LotRow` (read.rs:383-404).**
- `value_cents` and `currency` come from the same winning row. Both are `None` when no candidate survives.
- There is no EUR field. `eur_cents` is read only for the ceiling and the scale gate.
- JSON (json.rs:127, via `money` at json.rs:80-85) is `{cents, currency}`, or null unless both are present.

**How this differs from the stored tender election** (`head_value_eur_cents`, canonical.rs:2650-2709):

| | Lot pick | Tender head |
|---|---|---|
| Ranking | raw cents | EUR |
| Unconvertible amount | kept | dropped (`?`) |
| Conversion rounds to €0 (issue 378) | kept (e.g. HUF 1.48 is served) | dropped |
| Scope | lot only | tender and lot |
| Ceiling and scale rule | same predicates, but only when `eur_cents` exists | same predicates |

The tender row's displayed pick (read.rs:2040-2050) also has an explicit tie rule, `s.cents DESC, s.currency`. The lot pick has none.

## 2. REST surfaces that serve or filter on a lot value

**`GET /v1/lots`** (route mod.rs:242, handler mod.rs:1602, then `collection` → `read_page` mod.rs:1222-1272)
- The page goes through `lots_seeded_page` (winner/bidder seed) or `lots_page`; the isolated pool (isolate.rs:273) runs the same path. The served value is the **lot pick**.
- `min_value`/`max_value` (`version_predicates`, read.rs:1182-1186) compare the **tender head column**:
  - stream shape: `(SELECT tt.current_value_eur_cents FROM tenders tt WHERE tt.id = l.tender_id)` (read.rs:3575-3576);
  - containment (`tender=`) and `At` shapes: `t.current_value_eur_cents` (read.rs:3374).
- This is documented at read.rs:757-760 and in docs.rs:172 and 741-766.
- `currency` (read.rs:1201-1211) is an EXISTS over any amount row of the version, tender or lot scope. It does not read the pick.
- `sort`/`order` return 400 on `/v1/lots` (mod.rs:1603, `reject_sort` mod.rs:1711-1716). `/v1/tenders` sorts only by id, published_at or deadline (mod.rs:1462-1470). **No endpoint sorts by value.**

**`GET /v1/tenders/{id}`** (mod.rs:1756-1770 → `tender_detail` read.rs:2765)
- `lot_details` (json.rs:465) is the **lot pick**: `lots_of` uses `Filter{tender, lang}` with `Page{0, 20000}` on the containment shape, then `summarise`.
- The tender's own `value` is the **head column**, looked up by `eur_cents = t.current_value_eur_cents`.
- `amounts` is the raw rows, refused ones included.

**SSE on `/v1/lots`** (sse.rs)
- The snapshot goes through `read_page` (sse.rs:202, 212), so it uses `summarise`.
- The diff match check uses `lots_identity` at `Scope::At{id, seq}` and `seq-1`, with no `summarise` (sse.rs:425-433).
- The `include_data=true` payload uses `read_items(Scope::At{id, seq})` (sse.rs:468-474). The `At` arm binds an explicit seq (read.rs:3347-3349), so **`summarise` can run at a version that is no longer the head**. A stored value therefore has to be stored per version, not per lot.
- A quirk that is already there: at `At` scope `min_value` still reads the current head column, even when probing `seq-1`.

**Elsewhere**
- `/v1/tenders` list rows carry only a lot count (read.rs:2058).
- Webhooks and `/v1/changes` carry no lot values: `read_items` is called only from mod.rs, sse.rs and isolate.rs.
- `/v1/sql`: `v_lots` (canonical.rs:1086-1096) has no value. `v_tender_amounts` (canonical.rs:1182-1188) has raw rows with no exclusions.

## 3. The smallest stored set that gives bit-identical output

**Minimum.** Two nullable columns on `tender_version_lots` (primary key `(tender_id, seq, lot_id)`, the same key `summarise` uses):
- `value_cents INTEGER`
- `value_currency TEXT`

Every `LotRow` comes from a `tender_version_lots` row (read.rs:3323-3329, 3337-3357, 3547-3555), so these columns cover every row served.

**NULL meaning.** The pair is all or nothing, because the winning row always has NOT NULL cents and currency. Both NULL means no lot-scoped candidate survived.

**Unconvertible winner.** The pair is still set; it is not gated on a rate. An optional third column, `value_eur_cents`, would be that same row's `eur_cents` and NULL in this case. REST does not need it. It is what `v_lots` and any lot-level index or filter would read.

**What the fold-time pick must reproduce exactly:**
- the candidate filter from §1;
- raw-cents ranking;
- ties broken by the first row in Fact order;
- no rule against a conversion that rounds to €0;
- the ceiling and the scale rule only where `eur_cents` is not NULL;
- the scale rule built over `chain[..=N]`, with the head taken as version N, all amounts and lot awards included.

Version N reads only versions 1..=N, so appending a version never changes an earlier version's pick.

A caution on `value_eur_cents`: it would be the EUR of the raw-cents winner, not the highest EUR among the candidates. Switching the ranking to EUR would change output for lots with amounts in several currencies. The fixture in `lot_summary_equivalence` (lot 4) pins the current tie behaviour.

**Hazards a stored value adds:**
- **Rollout ambiguity.** Before the backfill finishes, both NULL means "not elected yet" as well as "no candidate", and the same is true of every test fixture that inserts rows with raw SQL. Fix it with a marker column (e.g. `value_elected INTEGER NOT NULL DEFAULT 0`, falling back to today's derivation when it is 0), or by gating the read switch on the backfill and rewriting the fixtures. New columns must be nullable or defaulted so existing raw `INSERT INTO tender_version_lots` statements in tests keep working.
- **`rederive-eur` changes the inputs in place.** It UPDATEs `eur_cents` on every version, head or not (rates.rs:437-560; supervisor.rs:5792-5830). The ceiling, the 1e11 gate and `value_eur_cents` all depend on that column. The job only stamps the affected tenders stale. The stored pick is refreshed only if the refold rewrites `tender_version_lots` for all versions; the fold slice should confirm this, including the unchanged-chain early return.
- **Fold cost.** Rebuilding `of_chain(chain[..=N])` for each version is O(N²) on long chains (rates.rs mentions a 2,983-version chain). Build it incrementally, or lazily only when a candidate reaches the €1 bn gate, as `summarise` does.

**Read-side change.** Either:
- read `SELECT lot_id, value_cents, value_currency FROM tender_version_lots WHERE tender_id=? AND seq=?` per version inside `summarise`, or
- select `vl.value_*` in the identity queries. On the stream shape these must be correlated subqueries beside the `kind` one, because `lots` must stay the only table in FROM (read.rs:3385-3402).

Either way the amounts query and the chain query (read.rs:3989-4092) go. Titles and deadlines stay.

## 4. Tests that pin the pick (must stay green)

**Store tests that insert rows with raw SQL** (they break if the stored value is not populated):
- `lot_value_election.rs`:
  - `a_lot_whose_only_figure_is_an_exact_zero_serves_no_value`: zero; zero beside a real figure; control; negative; over the ceiling; unconvertible `XXX` still served; withheld.
  - `the_lot_payload_and_the_value_filter_agree_about_a_zero`: `max_value` reads the head column.
- `lot_summary_equivalence.rs::set_based_lot_summary_agrees_with_the_correlated_subqueries`: lot 4 is 500 GBP vs 500 USD → GBP (raw cents, first row, `eur_cents` NULL); lot 6 is SEK; lot 7 is None/None; plus the orphan-lot guard.
- `lot_summary_cost.rs:138`, `#[ignore]`: "every lot decorated with a value".
- `lots_filter_fixture.rs:270-290`: `lots` vs `lots_previous_shape`, value fields included. Its amounts are all tender-scoped (line 92), so the value comparison already compares NULLs only.

**Store tests that go through the fold:**
- `head_election_agreement.rs`:
  - `a_refused_lot_figure_is_not_served_as_the_lots_value` (:226), the lot half of 471, with a below-gate control;
  - `a_scale_slip_partnered_only_by_an_earlier_version_is_refused_by_the_fold_and_the_row` (:186);
  - `a_tender_whose_only_amount_is_refused_serves_no_value` (:161);
  - `the_list_row_serves_the_amount_and_deadline_the_fold_elected` (:107);
  - `the_read_layer_reuses_the_election_rather_than_repeating_it` (:316), a structural check;
  - `the_head_columns_have_exactly_one_writer_that_decides_them` (:354). It counts only `current_value_eur_cents =`, so a new lot-column writer would not be caught; a twin guard may be wanted.
- `head_pointer_equivalence.rs:~228-250`: shipped vs oracle vs banded `LotRow` equality.

**Other `summarise` picks that must not regress:**
- `lot_deadline_scope.rs`: 9 tests, :104-319.
- `original_lang_pick.rs`: lot title at :125 and :165.

**Other files that call `read::lots`** and insert into `tender_version_lots` with raw SQL (so they depend on the schema accepting those inserts):
- `seeded_lots_page.rs`, `lots_country_seed.rs`, `lots_open_head_seed.rs`, `tenders_shortcircuit.rs`;
- `lot_prod_profile_probe.rs`, `lots_drive_probe.rs`, `currency_presence.rs`.

**Unit tests in canonical.rs:**
- `sentinel_amounts_are_negatives_and_all_nines_runs` (:31633)
- `the_head_column_does_not_assert_a_price_of_zero` (:31706)
- `a_figure_exactly_ten_to_the_k_above_a_sibling_is_not_elected` (:31750), including the `refuses_amount` assertions at :31984-31993.

**App tests (`crates/app/tests/api.rs`):**
- `a_lot_priced_at_zero_is_served_as_no_value_and_agrees_with_the_filter` (:4318). It **UPDATEs `tender_version_amounts` after the fold** and expects both `/v1/lots` and `lot_details` to follow. It will fail with a stored value unless the test also clears or re-elects the stored columns.
- The money-shape check at :687-694 needs a non-null `{cents, currency}` on `/v1/lots`.
- `an_open_lot_shows_the_procedure_deadline_that_opened_it` (:4405).

**Plan tests that change only if the identity SELECT or the lots `min_value` predicate moves:**
- read.rs `head_pointer_plan_tests` (:5047+), with lots `min_value`/`max_value` cases at :5071-5072;
- the lib.rs `lots_statement` plan tests (:7801-8140);
- `honoured_params_match_the_emitted_sql` and `FILTER_CLASSIFICATION` (read.rs:757-760).