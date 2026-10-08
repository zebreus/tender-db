**Fold write path for `tender_version_lots`: how lot rows are written, how kept versions work, what the fold has in hand, and the epoch and backfill options (issue 490)**

Nothing was edited, and nothing was run on the box. All citations are against /home/user/tender-db at its current state.

The short answers:
1. Lot rows have exactly one production writer: `write_version` buffers them and `Pending::flush` inserts them.
2. Adding a derived column to `tender_version_lots` does not make kept versions look changed, and they do not need rewriting. The fold never compares satellite contents. Existing rows stay NULL until a fold rewrites them, so a backfill is still needed.
3. Everything the read-time pick needs is in memory where lot rows are built. Two things need care: a naive per-version `ScalePartners::of_chain` call is quadratic, and the pick's tie-break depends on row order.
4. Bumping `PROJECTION_EPOCH` does not by itself rewrite anything. The proven backfill is one all-profile `refold` (about 10.5 h, measured on 2026-10-07).

---

## 1. Where `tender_version_lots` rows are built and inserted

- **Schema:** `crates/store/src/canonical.rs:217-224`. The columns are `(tender_id, seq, lot_id, kind)`, with primary key `(tender_id, seq, lot_id)`. There is no other index.
- **Where rows are built:** `Db::write_version`, `canonical.rs:28974`.
  - `:28997-29000` builds one `EurContext { rates: self.rates_lookup(), date: civil_date(v.published_at) }` per version. This is the same conversion that writes `tender_version_amounts.eur_cents`.
  - `:29003-29012` loops `for lot in &v.lots`. It resolves `lot_id` through `lot_identity` (`:29004`), pushes four values into `pending.version_lots` (`:29005-29010`), then calls `write_facts(..., Some(lot_id), &lot.facts, ..., &eur)` (`:29011`).
  - That call writes the lot's amount rows. Each `Fact::Amount` becomes one row with `eur.cents(cents, currency)` (`write_facts`, `:29217`, amount arm at about `:29240-29258`). No amount fact is ever dropped.
- **Buffer:** `struct Pending { version_lots: Vec<Value>, … }` at `:31023-31046`. It holds rows in order, four values per row.
- **Insert:** `Pending::flush`, `:31052-31055`:
  `flush_rows(conn, "INSERT INTO tender_version_lots(tender_id, seq, lot_id, kind) VALUES ", 4, &mut self.version_lots)`.
  - It is called once per batch of `WRITE_BATCH` tenders, just before `COMMIT`, in `apply_tenders` (`:15113`).
  - `flush_rows` is at `:31078`. With `MAX_BATCH_BIND = 900`, 4 columns gives 225 rows per statement; 7 columns would give 128.
- **Only one caller:** `write_version` is called only from `apply_tender_tx` (`:15228`).
- **Other writers:** `grep` finds no `UPDATE tender_version_lots` anywhere in the crates. Rows are removed only by `delete_version` (`:15446`), `reset_tender_layer` and `clear_canonical` (drop and recreate), and the retire path (`RETIRE_TABLES`, `:29440`).
- **Raw-SQL test fixtures:** about 15 test files insert `tender_version_lots` rows directly with a four-column list (for example `crates/store/tests/lot_value_election.rs`, `lot_summary_equivalence.rs`, `lots_*`). A new nullable column won't break those inserts. It will read NULL there, though, so once `summarise` reads the column those fixtures need values or `summarise` needs a fallback.

## 2. Kept prefix, the state key, the early return, and whether a derived column disturbs them

All of this is in `apply_tender_tx`, `canonical.rs:15148-15291`.

- **State key** (`:15162-15188`). It is the stored chain of causing notice ids plus `tenders.projection_epoch`, and nothing else.
  - `stored_chain` (`:15432-15444`) reads only `SELECT caused_by_notice_id FROM tender_versions … ORDER BY seq`. On a rebuild it is skipped and the chain is empty.
  - `stale = stored_epoch != PROJECTION_EPOCH` (`:15178`).
  - `keep` is 0 if stale. Otherwise it is the longest common prefix of the stored ids and `p.versions[i].caused_by_notice_id` (`:15179-15188`).
- **Unchanged-chain early return** (`:15189-15194`). If `!stale && keep == stored.len() && keep == p.versions.len()`, it increments `tenders_unchanged` and returns. This happens before any delete, write or `head_update`.
- **Partial rewrite:**
  - Versions `keep+1..=stored.len()` are deleted in reverse (`:15219-15222`). `delete_version` (`:15446-15470`) deletes from 14 tables, including `tender_version_lots`.
  - Versions `keep..` are rewritten through `write_version` and `append_version_changes` (`:15225-15232`).
  - The orphan sweep runs only if the chain shrank (`:15253-15266`). Its kept-prefix helper (`:15484`) reads only `lot_id` from `tender_version_lots`.
  - `head_update` (`:15273-15289`) writes `current_*` and stamps `PROJECTION_EPOCH`. `head_value_eur_cents(&p.versions, &self.rates_lookup())` runs at `:15282`.
- **Change rows** come from `append_version_changes(v, previous)` (`:29303`). That diffs the in-memory `TenderVersion` structs, never stored columns. A derived column therefore creates no change rows.

**A derived column does not make kept versions look changed.** No code path compares satellite content, so the early return and the value of `keep` are the same with or without the column.

**Kept versions do not need rewriting for correctness:**
- Each version is the cumulative left fold of notices `1..=N` (`crates/ingest/src/project.rs:5382-5468`). Facts and lots carry forward (`:5385-5407`), and rounds accumulate (`:5427-5433`).
- The per-lot pick for version N reads only versions `1..=N`, plus the rates at N's publication date.
- Kept means the same notice ids at the same positions under the same epoch. So version N's elected value cannot change when version N+1 arrives.
- The objection in the `ScalePartners` doc comment (`canonical.rs:2804-2812`) was about a per-`Fact` marker computed over the whole chain, which a later notice would flip. A per-version value bounded at `seq <= N` does not have that problem.

**The real consequence is NULLs, not invalidation.** Rows written before the column exists stay NULL until a fold rewrites them:
- An untouched tender early-returns and keeps NULL in every version.
- A daily fold that appends version N+1 to an old tender fills only N+1. Versions `1..=N` stay NULL, which matters for historical `Scope::At` reads (the SSE diff, `crates/app/src/v1/sse.rs:426-469`).
- The read paths that use the head version (`current_seq`: the stream, containment and `v_lots`) are correct once the head version has been written by the new code.

**Measured evidence.** Issue 484 hit exactly this (`.scratch/tender-db/issues/484-*.md:3`):
- Fold 2940 walked all 8.78M Tenders but wrote only 1.62M. The other 7.16M early-returned and kept `is_buyer` NULL.
- Only the all-profile `refold` 2952/2953, which stamps every tender stale, rewrote all of them: 8,780,780 written, 0 unchanged (`484-*.md:646`).

**NULL is ambiguous.** It can mean "nothing elected" or "not yet written". `summarise` cannot switch to reading the column until that is resolved. The options are:
- a full backfill plus a completeness flag in `projection_state` (the pattern of `currency_presence_complete` at `crates/store/src/lib.rs:841-856` and `tender_links_complete` at `:866-874`);
- a non-NULL "written" marker per row;
- a per-tender epoch check (see section 4).

## 3. What is in hand where lot rows are written, and whether the exact `summarise` pick can be computed there

**Available:**
- The full chain `p.versions` (`TenderProjection.versions`, `canonical.rs:3367-3373`). It is always the whole chain, aligned position by position with the stored chain, which is what the `keep` comparison relies on.
- The loop at `:15225-15232` already has `i` and `p.versions[..=i]` in scope; it computes `previous` from them.
- `write_version` currently receives only `v: &TenderVersion`, so it needs one new argument: the chain prefix or a prepared partner set.
- Rates: `self.rates_lookup()` (`lib.rs:1149`, an `Arc` snapshot reloaded at the start of each projection, `rates.rs:218-220`). It is already used in `write_version` (`:28998`).
- Per lot: `lot.facts`, and the version's own amounts at tender and lot scope.

**What the read-time pick does** (`crates/store/src/read.rs:3910-4093`):
- **Candidates** (`:3995-3997`) are lot-scope amounts of version N with `quality IS NULL`. In the fold that is `lot.facts` entries `Fact::Amount` with `quality == None`.
- **Skips:**
  - `sentinel_amount(cents)` on the published figure (`:4022`; `canonical.rs:3015`).
  - The ceiling `eur > IMPLAUSIBLE_EUR_CENTS`, applied only when a conversion exists (`:4032`). A missing conversion keeps the candidate.
- **Scale rule:** `ScalePartners::refuses_amount(field, currency, cents, eur)`, applied only when `eur` exists (`:4078-4082`).
  - Partners are every amount of versions `1..=N`, of any field, quality or scope, plus every lot award with both cents and currency.
  - Corroboration uses version N's amounts at all scopes. Lot awards never corroborate.
  - This is exactly `ScalePartners::of_chain(&p.versions[..=i])` (`canonical.rs:2853-2880`).
  - The query that loads the inputs only runs when a candidate reaches €1 bn (`read.rs:4045-4065`). That gate changes nothing, because `refuses_amount` returns false below `SCALE_ERROR_MIN_EUR_CENTS` anyway (`canonical.rs:2898`).
- **Choice:** the maximum of the published `cents`, compared across currencies, not EUR. Ties keep the first row seen (strict `>` at `read.rs:4086-4087`).
- **What "first row seen" means:** the version's rows come back through `tender_version_amounts_version(tender_id, seq)` (`canonical.rs:296`) in rowid order, which is insertion order. That is the iteration order of the `BTreeSet<Fact>`. `Fact::Amount` sorts by `field`, then `cents`, `currency`, `tax_basis`, `quality` (`canonical.rs:2509-2526`).
  - So iterating `lot.facts` with strict `>` reproduces the tie-break exactly.
  - Doing this at fold time also removes a hidden dependence on scan order. The `Pending` doc (`:31009-31021`) claims row order is unobservable, and this tie-break contradicts that.

**Points to preserve exactly.** Each of these differs from `head_value_eur_cents` (`canonical.rs:2650-2709`), so the lot pick cannot simply reuse the head election:
- (a) Lot scope only.
- (b) The maximum is taken over published cents, not EUR.
- (c) Unconvertible candidates are admitted.
- (d) A conversion that rounds to EUR 0 is not declined. The head declines it at `:2707`.
- Consequence: a stored `value_eur_cents` for the picked row can be NULL or 0. That matters if it is to back `/v1/lots?min_value=`.

**Cost trap.**
- Calling `of_chain(&p.versions[..=i])` for every rewritten version is quadratic, and worse in practice: facts and rounds are cumulative, so each prefix rescans every earlier version.
- The legacy mega-chain is 2,983 versions with 8.9M stored lot-result rows (`rates.rs:478-487`). That would mean billions of hash inserts.
- Fix: keep one running partner set across the loop.
  - First add the in-memory kept prefix `p.versions[..keep]` (no I/O).
  - Then, for each written version, add its amounts and lot awards. The set only grows, so union semantics hold.
  - Rebuild only the `head` map for each version.
  - `ScalePartners`' fields are private, but `impl Db` is in the same module (`canonical.rs:8336`, against the struct at `:2813`), so a `reset_head()` or direct field access is legal.
  - The cost is then proportional to the rows the fold already writes.
- `head_value_eur_cents` keeps its single whole-chain `of_chain`.

**Where to compute it:** inside the lot loop at `:29003-29011`, using the `eur` already built. Each lot then gets `(value_cents, value_currency, value_eur_cents)` appended to `pending.version_lots`, and the column count at `:31055` goes from 4 to 7.

## 4. `PROJECTION_EPOCH`: what a bump does and costs, and how earlier columns were backfilled

**The constant:** `canonical.rs:1302` (currently 3). Its contract is at `:1247-1301`.
- A bump makes every stored tender stale (`:15178`). Each tender then takes the `keep = 0` path: all versions deleted and rewritten, all change rows re-emitted, head restamped.
- **A bump queues no work.** "The rewrite set is the `projected = 0` marking, never the epoch" (`:1264-1266`). Nothing in `crates/ingest/src` reads `projection_epoch`.
- So a bump only changes what happens to a tender a fold is already processing. From then on every tender the daily touches gets a full-chain rewrite instead of an append, with change events for every version. Untouched tenders still need a refold.
- **Policy:**
  - A bump must be paired with a scoped refold (`:1264`).
  - Bump only for cross-profile logic changes (`:1283-1291`). Issue 179 measured 6h02m and 14.2M version writes for a 2.69M-notice cohort.
  - The change feed is append-only, so the noise is permanent (`:1267-1269`).
  - Precedents shipped without a bump: issue 471's scale rule (`docs/operations.md:564`), issue 484's `is_buyer` (`:1545`) and the 479 backfill (`:869`).
- **What a bump would buy here:** a per-tender discriminator. `projection_epoch >= 4` would mean "every version written with the column", which settles the NULL ambiguity per tender.
  - The all-profile `refold` already stamps every tender stale, and the rewrite then stamps the current epoch. So the bump adds only the discriminator and the extra daily rewrite cost.
  - A `projection_state` completeness flag gives the same discriminator without the bump.

**How earlier columns were added and backfilled:**
- **Adding the column:**
  - Add a line to `MIGRATIONS` in `lib.rs:691`. The array is declared `[&str; 30]`, so its length must change.
  - A nullable column with no default is metadata-only and O(1) (`is_buyer` on 127.8M rows, `lib.rs:766-771`; proven by `crates/store/tests/alter_add_column_cost.rs`).
  - It must ship in the same commit as the column in canonical.rs's `CREATE TABLE`. That is issue 372's lesson: the fix was inert on prod because tests only see fresh databases (`lib.rs:734-742`).
  - Rebuilds keep ALTER-added columns on `tender_version_lots`, because `drop_and_recreate` copies the DDL from `sqlite_master` (`canonical.rs:10464-10483`). Only `tenders` has a hard-coded `CREATE` that must be edited (`:10505-10520`).
- **SQL walk backfills:** `backfill_current_deadline` (`lib.rs:3690`) and `backfill_current_title` (`lib.rs:3740`). Both walk `tenders` primary-key windows, checkpoint between batches and are idempotent.
  - The value equivalent, `backfill_current_value_eur`, was deleted (`lib.rs:3776-3793`, issue 375). `sentinel_amount` is a digit walk, and copying it into SQL would be a second implementation of the rule.
  - `crates/store/tests/head_election_agreement.rs:354` pins the head columns to exactly two writers.
  - A lot-value column should get the same single-writer pin. A SQL backfill is ruled out by that precedent.
- **Rust walk without a refold:** `rederive_eur_window` (`rates.rs:452-555`) and `backfill-original-lang` (`lib.rs:3884`).
  - These window on the `tenders` primary key, read each table by its `(tender_id, seq)` prefix, join in Rust (the mega-chain lesson) and update by key.
  - A lot-value walk could call the same pick function the fold calls.
  - It would still be a second writer, and it would have to read amounts in rowid order to keep the tie-break.
- **Riding an all-profile `refold`** (the runbook at `docs/operations.md:869-895`; the `is_buyer` rollout at `:1545-1556`):
  - It requeues every notice, stamps every tender stale and runs `project` on the full-corpus path.
  - The latest run was refold 2952 and project 2953: 14,896,923 notices, 8,780,780 Tenders, 07:50→18:16 UTC, 10 h 26 m.
  - That was the last all-profile refold, and the batched changes it carried (484, 489, and 479 before it) are already done. Issue 490 would need a new one. That is the "issue-484 drain shape" the issue refers to.
  - Disk: about 13.2M `tender_version_lots` rows on prod (`read.rs:1460`) gaining three narrow columns, plus the usual WAL and change-event volume.

**Two side effects of storing the value:**
- **A rate correction makes stored lot values stale.** Today `rederive-eur` updates `eur_cents` in place, and `summarise` reflects that at once. `rederive-eur` only stamps the affected tenders stale (`supervisor.rs:5815-5822`), and since the planner ignores the epoch, they re-elect only when their notices next come through a fold. A stored lot value would then lag in the same way `current_value_eur_cents` already does.
- **A future change to the pick becomes a drain, not a deploy.** The refold must stamp tenders stale (`refold-notices` does, `supervisor.rs:5492`) to reach kept versions. That is the pattern of the 471 drain at `operations.md:572-603`.

**Tests to update:**
- The fold-equivalence and golden digests name `tender_version_lots` columns explicitly (`crates/ingest/tests/project_golden.rs:95`, `crates/ingest/tests/project.rs:5232`). The new columns stay unpinned unless they are added there.
- The `MIGRATIONS` array length must be bumped.