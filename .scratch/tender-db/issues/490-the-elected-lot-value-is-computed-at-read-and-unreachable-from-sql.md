# 490 — the elected lot value is computed at read time and unreachable from /v1/sql

Status: ready-for-agent — DEPLOY A LIVE (`9e2e80d`); BACKFILL RUNNING (refold 2066 done, project 2067 planning at 07:2x UTC); unit 3c on main (`d68ff16`, gate green); DEPLOY B READY ON `claude/cool-sagan-5bk0rc` ONLY (`960272c`, gate green, reviewed). Keep deploy B OFF main until the backfill checks pass. NEXT, after project 2067: (1) the completeness read (`projection_epoch <> 4` in 100k windows; triage 3 vs 0); (2) the m490 compare (stored `value_*` == REST, 0 differences); (3) data-quality `{"kind":"data-quality","dry_run":false}` and section 16 against dq 2954 (expect issue 491's text rows out, nothing else). Then push deploy B to main and deploy it, and confirm the partial index built and the /docs recipe's timing.
Kind: SQL surface / performance / coherence
Relates to: 471 (the exact-10ᵏ rule, unit 4(a)), 389 (lot-level election made coherent with the
tender head, on the REST surface only), 372 (`quality = 'withheld'`), 50 (the analyst views)

## What is true today

**The tender value is stored.** `head_value_eur_cents` (canonical.rs) runs in the fold when it
writes a tender's head pointer. That covers the sentinel test, the ceiling, the exact-10ᵏ
`ScalePartners` rule and the max over tender and lot amounts. The elected figure lands in
`tenders.current_value_eur_cents`, indexed `(current_value_eur_cents, id)`. `/v1/sql` and
`/v1/tenders?min_value=` read that column and pay nothing for the rule. Measured on the box
2026-10-08, server side through `/v1/sql`:

| Query | Time |
|---|---|
| top 5 tenders by `current_value_eur_cents` | 3 ms |
| `COUNT(*)` over a €1 m–€10 m range (984,406 tenders) | 56–66 ms |
| point read by id | < 1 ms |

**The lot value is not stored.** The REST lot rows (`/v1/lots` and the lots of
`/v1/tenders/{id}`) pick their value in memory at read time in `read.rs::summarise`. It takes the
max-cents candidate over the version's amounts after the sentinel, ceiling and (since 471 unit
4(a)) `ScalePartners::refuses_amount` tests. The rule's extra inputs, the chain's amounts and lot
awards up to the version, are queried only when a candidate reaches the €1 bn gate. The REST
cost is small, but it is spent on every read, not once.

**`/v1/sql` has no elected lot value at all.** `v_lots` carries id, tender, key, kind, seq and
title, and no value. An analyst gets a lot's value by aggregating `v_tender_amounts` /
`tender_version_amounts` themselves. Those tables hold every published figure, including the ones
the fold and the REST pick refuse. That is deliberate (refused is not deleted, and the `quality`
vocabulary stays at the source's own `'withheld'`; see the `ScalePartners` doc comment). So a
SQL consumer's natural `MAX(eur_cents) … GROUP BY lot_id` serves the ×10ᵏ error that the
tender head and the REST lot row both refuse. To reproduce the REST figure in SQL, the consumer
would have to re-implement a rule that reads the whole chain. `/v1/sql` is the main way the data
is consumed, so this is the wrong way round. The fast surface has the incoherent figure, and the
coherent figure costs work on every REST read.

## Direction (to decide in unit 2)

Store the elected lot value once, at fold time, the way the tender head already is. Then expose
it on `v_lots` (e.g. `value_cents`, `value_currency`, `value_eur_cents`). `summarise` would then
read a column instead of re-deriving it.

Why it is likely sound: the per-lot pick for version N reads versions 1..=N only
(`seq <= ?` in `summarise`). A later notice therefore cannot change version N's figure, and
storing it on the version row (`tender_version_lots`) does not hit the objection the
`ScalePartners` doc raises against a per-`Fact` `quality` marker (the rule reads the whole chain,
so a later version would rewrite kept ones). Check before building:

- the fold's state key and the unchanged-chain early return. A new derived column must not make
  every kept version read as changed.
- the deadline fallback and the title pick in `summarise`. Store only the value; the rest stays.
- the backfill cost: a refold of all profiles, about 14.9 M notices, the issue-484 drain shape.
- whether an indexed `v_lots.value_eur_cents` should also back `/v1/lots?min_value=`, which
  today filters on the TENDER head column.

## Units

1. Measure the gap on a window: current lots whose naive SQL max differs from the REST pick.
2. Decide the storage shape against the checklist above. Record the decision here.
3. Build it with tests (fold writes it, the view exposes it, `summarise` reads it, and REST and
   SQL agree on a refused-figure fixture). Then gate, deploy, backfill, and do a closing read.

## Unit 1 — measured (2026-10-08)

The reader's full report is in `490-values/reader-measure-2026-10-08.md`. The raw rows and scripts are in
`490-values/m490/` (JSON gzipped). The scripts rebuild `summarise`'s pick from the SQL rows; it matched the
served REST value on **6,297 of 6,297** lots.

| window (tender ids) | era | lots | with a lot amount | naive SQL ≠ REST |
|---|---|---|---|---|
| 224000–224499 | eForms 2025 | 1,222 | 539 | 8 (zero 6, one-unit 1, scale rule 1: 224156 LOT-0002 €40 bn) |
| 1000000–1000499 | eForms | 1,245 | 333 | 8 (zero 6, one-unit 2) |
| 8784500–8784999 | FTS 2026 | 1,046 | 350 | 37 (one-unit £1.00 on 4 tenders) |
| 6941300–6941799 | TED r209 2021 | 1,783 | **0** | 0 |
| 6290000–6290499 | TED r209 2018–19 | 1,001 | 45 | 0 |
| **all** | | 6,297 | 1,267 | **53 (4.2 %); 41 remain even after `quality IS NULL AND cents > 0`** |

Also found:

- REST never falls back to a tender-scope figure for a lot. 3,176 of the 6,297 lots have no lot amount
  while their tender has one, which is why `/v1/lots?min_value=` stays on the tender head (decided below).
- The 2021 r209 era projects no lot-scope amounts at all. Its lot figures exist only as
  `tender_version_lot_results.awarded_cents`, which is never elected. 6941544's £80 bn ×1000 award is still
  served raw by `v_lot_results` / `v_awards`. That is a separate gap, not covered by a `v_lots` column.
- Read cost today: 6 of 515 Tenders whose current version has lot amounts reach the €1 bn gate that makes
  `summarise` load the chain on every read.

## Unit 2 — decision (2026-10-08)

The design is `490-values/design-2026-10-08.md`. It was adversarially reviewed for correctness
(`review-0`) and for operations (`review-1`); the reader reports sit beside it. Both reviews confirm the
election maps one-to-one onto `summarise` (ties in Fact order, unconvertible winners kept, no tender
fallback, LotsGroup/Part, later chain changes). Their findings are about rollout. Decided:

1. **Storage.** `tender_version_lots.value_cents`, `value_currency` and `value_eur_cents`, all nullable.
   They are stored per version because SSE `Scope::At` serves non-head versions, and version N's pick
   reads only versions 1..=N, so kept versions stay valid. The fold is the only writer.
2. **One rule.** `canonical::elect_lot_value`, beside `head_value_eur_cents`. `ScalePartners` gains
   `add_version_figures` / `set_head`, so the fold builds a running partner set (`of_chain` is rebuilt from
   them, and a test pins running == `of_chain` per prefix). `summarise` calls the same function in deploy A
   and reads the columns in deploy B.
3. **`PROJECTION_EPOCH` bumps** (ops review #3). The all-profile refold rewrites every Tender anyway, and
   epoch 4 then means "written by 490 code". A bounded primary-key window read of `projection_epoch <> 4`
   is the completeness check. That covers a stopped refold, a rollback, and a restored backup. Cost: every
   whole-corpus walk until the backfill completes becomes that backfill.
4. **Two deploys.**
   - **Deploy A** ships the columns, the fold write, the epoch bump, and `summarise` routed through
     `elect_lot_value` with no output change.
   - **Then:** an all-profile refold, sized by a fresh profile list (correctness review F1), with no deploy
     in its window.
   - **Deploy B** ships `summarise` reading the columns, the `v_lots` columns (not in A: ops #8), and a
     deferred PARTIAL index `tender_version_lots(value_eur_cents, tender_id, seq, lot_id) WHERE
     value_eur_cents IS NOT NULL`. turso 0.7.2 uses it only with the literal term, so the documented recipe
     carries it (ops #7).
5. **Preconditions for deploy B:**
   - `rederive-eur` also requeues the causing notices of the Tenders it changed (F2; today it only stamps,
     and nothing re-folds a stamped Tender).
   - The completeness check reads 0.
   - The section-16 head band is unchanged against a before-image saved ahead of deploy A (ops #5).
   - The m490 windows compare stored == REST.
6. **`/v1/lots?min_value=` stays on the tender head.** A lot-level bound would drop the 50 % of lots that
   carry no lot figure. That would be a contract change, which nobody has asked for.
7. **Sizing** comes from `sqlite_stat1` for `tender_version_lots`, not from `MAX(rowid)`. The table holds a
   row per lot per version, so 13.2 M (the lots count) is only a floor (ops #1). Record `df` before and
   after the refold.
8. **Runbook additions** for `docs/operations.md`:
   - The refold's `expect:1` sizing call queues a real project (ops #4).
   - Pass conditions must hold across a stopped or resumed refold (ops #2).
   - After a rollback below deploy A, redo the refold before deploy B (ops #9).
   - A future change to `sentinel_amount`, `IMPLAUSIBLE_EUR_CENTS` or `ScalePartners` needs a stamping
     refold (F4; add them to the `PROJECTION_EPOCH` contract text).

## Units (revised)

- **3a, deploy A.** `ScalePartners` refactor, `elect_lot_value`, the fold write, the migration, the epoch
  bump, and `summarise` routed through the shared function. Tests:
  - running partners == `of_chain`;
  - the fold writes the columns;
  - REST is unchanged on the fixtures (sentinel, ceiling, ×1000 partner, withheld, unconvertible, tie);
  - head election is unchanged.
- **3b, backfill.** Save the section-16 before-image, size the refold, run the all-profile refold, then do
  the completeness read and the m490 compare.
- **3c, `rederive-eur`.** Requeue the changed Tenders' notices.
- **3d, deploy B.** `summarise` reads the columns, the `v_lots` columns, the partial index, `/docs` and the
  SQL recipe, and `operations.md`.

## Unit 3a — deployed; 3b — backfill started (2026-10-08)

- **Commits.** `a929893` (fold write, shared `elect_lot_value`, running rule, epoch 4), `f895fd9` (golden
  re-blessed) and `92a6c9c` (review fixes). The review workflow `wf_b8d81d38-d7c` found the blocker
  already fixed by `f895fd9`. Its one major, the untested kept-prefix seed and post-loop `set_head`, is
  fixed: each new test fails with its target line removed. The golden capture showed only the epoch
  line moving before the digest was widened. `value_eur_cents` stores NULL for a conversion that rounds
  to 0 (decided: issue 378's rule).
- **Gate.** `GATE-EXIT=0`, 145 suites, on `92a6c9c`. **Deploy A** shipped `9e2e80d`: tree-equal outside
  `.scratch`, built on the box from the pushed rev.
- **Before-images.**
  - The head band: data-quality job 2063 was a DRY RUN (it needs `dry_run:false`), so the band
    before-image is dq 2954's section 16 (`471-values/section16-dq2954-2026-10-08.txt`). It predates
    issue 491's drain, so its expected departures are 491's text rows plus daily drift.
  - REST lot values: m490, captured before deploy A.
  - Disk: `/data` 850 G used, 809 G free.
- **Pre-flight.** 0 unprojected notices. The profile list, walked fresh, is the same 24 profiles as job
  2952. Sizing call 2064 aborted at 14,896,923 notices; its trailing project 2065 was a no-op.
- **Refold 2066 + project 2067** were enqueued at 05:37:58Z.

## Units 3c and 3d — built (2026-10-08)

- **3c: `rederive-eur` re-queues the causing notices of the Tenders it changed.** Commit `d68ff16`,
  tested by `rederive_eur_requeues_the_notices_of_the_tenders_it_changed`, gate green, on main. Not
  deployed yet; it rides with deploy B.
- **3d (deploy B): `summarise` reads `tender_version_lots.value_*`** with one PK-prefix seek per version.
  - `v_lots` appends the three columns.
  - A deferred PARTIAL index `tender_version_lots_value_eur`.
  - A `/docs` recipe (CROSS JOIN, lot table first, literal `IS NOT NULL`), pinned by its plan in
    `the_lot_value_recipe_drives_from_the_partial_index`.
  - Column notes, including exact `v_lots` notes that send ranges to the base table.
  - `openapi` `Lot.value`, and the runbook's deploy-B steps.
  - Tests that hand-built amounts for the reader now go through the fold, or store the value as the
    fold would.
  - Commits `8f71775`, `160f65c` and `960272c`, reviewed by `wf_a689b740-7c8`. The review's blocker was
    already fixed by `160f65c`; its major (a plain JOIN in the recipe) is fixed and pinned. Gate:
    `GATE-EXIT=0`, 145 suites.
- **Follow-up filed:** issue 493. A lot whose elected value moves emits no lot change. This predates 490.

