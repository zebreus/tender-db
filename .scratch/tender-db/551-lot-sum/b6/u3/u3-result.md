# Issue 551 B6 U3: Rust v2+G against the port, parity result (2026-10-19)

**Verdict: PASS.** The Rust twins and the port of record agree on every Tender of every population under all four
guard sets. They were compared on 37,732 (Tender, guard set) pairs: 9,433 Tender rows times 4 guard sets. There are 0
disagreements on ship or refuse, S, currency, n, the first refusing check (R10 for every guard refusal) or `vouched_by`.
On `vouched_by`, Rust's total is always a member of the port's kept vouches, and it is the port's first kept vouch on
every ship. The head-value twin also agrees with the check twin on all 37,732 pairs.

**Projection parity is clean** on every field the guards read, on the 3,502, the 562, the 61 and the 563. The only
difference is `LotState.kind`, which `normalise_de1` renames for DE1 sections (`ProcurementProjectLot` becomes `Lot`).
B3 recorded the same class. Its count is 122 on the 562, as in B3, and 71 on the 3,502.

**`crates/` is byte-identical to HEAD `ab9aa22`.** This was checked twice, after the harness was removed.

**Two harness-side encoding bugs were found and fixed.** Neither touched the port or the production code.
- In the records' port input, 3 ids differed only in the name of the first refusing check.
- The comparison input put the projection's version chain in sorted-key order.

Both are explained below. Machine-readable copy: `u3-result.json`. Exact steps: `run.sh`.

## Inputs and pins

| item | value |
|---|---|
| repo HEAD | `ab9aa22` (U3 guards f41c809 + bcc501a). `canonical.rs` sha256 `42c9b2b5…dbcd` and `project.rs` sha256 `66b7076f…70f5` before and after, equal to HEAD |
| port of record `b6/rule_v2_g.py` | sha256 `23cbd8508a3a…` (matches `b6/SHA256SUMS`). Loaded unmodified: only its layer lists are pointed at each population's reads |
| fixture builder | B3's `b3/gen_b3.py` `2e9635ba…`, through `b3/export.py` `2203cab0…` (its `emit`, `profile_of`) and `b3/records.py` `124382c2…` (its `v1_of`, `detail_of`). `gen_b3`'s `R` is rebound to `rule_v2_g`, so each fixture is built from exactly the facts the port decides on |
| rates | the m551 10-17 set: `8e925cef…`, `47e94127…`, `48e2c9f1…` |
| U2 decisions | `b6/u2/decisions.jsonl.gz` `0add92e3…`. The port's decisions on the 3,502 equal U2's on every id, under all four guard sets, including `vouch_candidates` (3,502 / 3,502) |
| GETs | none. `outputs/check.log`: 0 tenders and 0 notices missing from the layers. `api_requests.log` lists no request |

## Populations

| population | reads | Tenders | notice contents |
|---|---|---|---|
| **b3api**: B3's 562 API ids (279 labelled, 6 oos, 277 walked) | B3's re-reads (`scratchpad/b3/raw/tenders`), notices in B3's order | 562 | 4,416, all from the M1 cache |
| **b3rec**: B3's 4,745 unit-1 records | `unit3/data/ctx.json.gz` (stored rows) | 4,745 | none (as in B3) |
| **c563**: the 563 cohort (B3's 562 + 7511890) | B5's re-reads (`scratchpad/b5/raw/tenders`, path W on), notices in B5's order | 563 | 4,420 |
| **s61**: the 61 B5 investigation heads | the served snapshots the readers read (`b5/investigate/served`, path W on) | 61 | 372, from the B6 layer |
| **f3502**: the 3,502 | the fresh B6 layer (`scratchpad/b6/tenders`, path W off) | 3,502 | 15,537: 15,246 from the B6 layer, 291 from the M1 cache |

- **Rust inputs, b3api and b3rec.** These are B3's own `api_in.tsv` and `rec_in.tsv`, copied verbatim. The b3api
  file was also re-generated from `rule_v2_g` on B3's reads, and the result is byte-identical (sha256
  `37a738e2…`).
- **Rust inputs, other populations.** Each fixture is built from the served rows: lot results with winners, award,
  decision, published number and group flag; bids with parties; amounts; lots; one lots-only version per other
  results-round notice; and the port's stated totals.
- **Served-aware v1 on c563 and s61.** These were read with path W on (B5's `cohort_parity.port_decision`,
  operations.md step 3). On a head served `lot-sum-awards`, the port API reader's `W/G0-served` reason, which
  compares against today's served head, is read as `W` before `facts_from` runs. The Rust twin reads chain facts and
  never a served head. This touched 73 heads in c563, which are B5's 73 ships, and all 61 in s61. No other head was
  touched.

## Decision parity

Each row lists compared / agree, then ships port / Rust, under v2, v2+single, v2+cover and v2+both.

| population | v2 | v2+single | v2+cover | v2+both | disagreements |
|---|---|---|---|---|---|
| b3api 562 | 562/562, 73/73 | 562/562, 72/72 | 562/562, 73/73 | 562/562, 72/72 | 0 |
| b3rec 4,745 | 4,745/4,745, 0/0 | 4,745/4,745, 0/0 | 4,745/4,745, 0/0 | 4,745/4,745, 0/0 | 0 (after fix 1; 3 ids x 4 before it) |
| c563 563 | 563/563, 73/73 | 563/563, 72/72 | 563/563, 73/73 | 563/563, 72/72 | 0 |
| s61 61 | 61/61, 61/61 | 61/61, 60/60 | 61/61, 60/60 | 61/61, 60/60 | 0 |
| **f3502 3,502** | 3,502/3,502, **3,502/3,502** | 3,502/3,502, **3,468/3,468** | 3,502/3,502, **3,489/3,489** | 3,502/3,502, **3,467/3,467** | **0** |
| (the 61 inside f3502) | 61/61, 61/61 | 61/61, 60/60 | 61/61, 60/60 | 61/61, 60/60 | 0 |
| **total** | | | | | **0 of 37,732** |

- **v1 fires agree** (port / Rust): 555 / 555 in b3api and c563, 6 / 6 in b3rec, 61 / 61 in s61, 3,502 / 3,502 in
  f3502.
- **The guard refusal sets are identical on both sides.**

  | population | R_single | R_cover | R_excess (cover beyond single) |
  |---|---|---|---|
  | f3502 | 34 ids (U2's `R_single.json` exactly) | 13 ids, 12 of them also in R_single | {7202436} |
  | s61 and the 61 in f3502 | {6465510} | {6465510} | none |
  | b3api and c563 | {6595330} | none | none |

  6595330 is the case where G-single refuses and G-cover keeps.
- **Invariants hold on both sides for every id.** Every guarded ship is a v2 ship. Every guard refusal is named
  `R10`. A v2 refusal keeps its first check under every guard. S, currency and n are the same under all four guard
  sets. Every v2 ship is a v1 fire.
- **No ship's `vouched_by` moves under any guard,** on either side.
- **6465510 behaves as U1's T1 says.** v2 ships EUR 162,020.92, n = 2, vouched by TED-VAL_TOTAL 16,129,032 on
  20772908. G-single, G-cover and both refuse it at R10, and the head-value twin keeps EUR 70,340.42.

## Disagreements and their causes

1. **b3rec: 120725, 155057 and 514883, under all four guard sets, first check only.** The port said `W/K` and Rust said
   `W/T'`. Both refuse, with S none and n = 0. **Cause: an encoding gap in the port side's input. Fixed.**
   - **What changed in Rust.** Each record's head has a withheld tender-scope `result_value`, stored as the -100
     placeholder, and a withheld lot award carrying the same -100. Since 5d3fc52 (B5, 179732), Rust refuses a withheld
     tender-scope award copy at `W/T'`. When B3 ran, Rust and the port both said `W/K`. B3's recorded Rust output
     differs from today's Rust on exactly these 3 ids and no others.
   - **Why the port said `W/K`.** `b3/records.py` gives `merge_rule.serve` the stored entry, -100 included. Its
     `is_head_award` then matches placeholder to placeholder and files the figure as a copy, so T' passes and K
     refuses.
   - **Why the port of record never sees that input.** It reads a withheld amount through `rule_v2_g._entry`: the API
     serves it with no value, which `_entry` reads as cents -1. That never matches a -100 award, so the port files a
     procedure figure and refuses at `W/T'`, as Rust does. 5d3fc52's own comment says so.
   - **Fix, port-side input only.** In `export_u3.py`'s records path, every withheld amount is now given to
     `merge_rule` at -1, as `_entry` encodes it (`api_withheld`). The Rust input stays B3's verbatim file.
   - **Effect of the fix.** It re-encodes 7 records. Exactly these 3 move (`W/K` to `W/T'`); the other 4 are
     unchanged. With B3's encoding, the port still equals B3's recorded `rec_port.jsonl` on all 4,745 records. The
     pre-fix comparison is kept as `b3rec[B3 port encoding]` (`u3-compare.json` → `populations_record_only`; port file
     `inputs/b3rec_port_b3enc.jsonl`).
2. **Projection: "version chain" on 832 of the 3,502. Cause: a harness bug in the comparison input. Fixed.**
   - **The bug.** B3's `compare.projection` reads the served chain from the key order of `port_lots_all_versions`.
     `export_u3.py` writes its rows with `sort_keys=True`, which sorts those keys numerically as strings.
   - **The evidence.** On all 832, the chain is the same set of notices in a different order. The fold's order equals
     the API's `versions[].seq` order on every Tender.
   - **The fix.** Rows now carry `served_chain` (a list), and `compare_u3.py` re-keys them in that order before
     calling B3's unmodified function. Result: 0 chain differences in all four projection populations.
3. **Projection: `LotState.kind`, 71 rows on the 3,502, 122 on the 562 and on the 563. Recorded, not a bug.** The
   fold's `normalise_de1` renames DE1 `ProcurementProjectLot` to `Lot`. B3 recorded the same 122 on the same 562. No
   guard and no B1 field reads the kind.

No other disagreement occurred, and no name-level `vouched_by` difference occurred: Rust's vouch is the port's first
kept vouch on all 14,747 shipping (Tender, guard set) pairs.

## Projection parity

B1's real `NoticeState::read` → `fold` on the parsed notice contents, compared with the port's facts. The comparison is
B3's `compare.projection`, unmodified.

| population | Tenders | version notices / lot rows | result rows (notice, key) → lot_key, published_lot_no, group | stated totals, multiset incl. `lots_awarded` | head lot keys | differences |
|---|---|---|---|---|---|---|
| **f3502** | 3,502 | 15,537 / 100,158 | 25,741, all equal; key sets equal on every Tender | 6,257 / 6,257, equal | equal | `LotState.kind` 71 |
| b3api | 562 | 4,416 / 26,764 | 6,699, all equal | 1,737 / 1,737 | equal | `LotState.kind` 122 |
| s61 | 61 | 372 / 3,115 | 579, all equal | 144 / 144 | equal | none |
| c563 | 563 | 4,420 / 26,788 | 6,705, all equal | 1,739 / 1,739 | equal | `LotState.kind` 122 |

- **What this shows about the guards' inputs.** The result-row key sets per round notice are what G-single counts
  (`round_results`) and what G-cover reads through `on_lot`. Their equality, with the stated totals' kinds equal,
  means the real fold gives the guards the rows the fixtures copy.
- **The deviation risk recorded under "B6 U3 (build part)".** On 6465510's notice 20772908, the real fold projects
  one result, RES-1, with lot_key null, published number "4" and group false. It also projects TED-VAL_TOTAL as an
  Award of 16,129,032 EUR with `lots_awarded` = []. Both equal the port's facts and the fixture. This is the
  decision's "20772908-type notices" check.
- **No fold drift on the 562 since B3.** The 562's projection input is byte-identical to B3's `proj_in.jsonl`. Today's
  fold output on it is byte-identical to B3's `proj_rust.jsonl`.

## Port-side cross-checks

| check | result |
|---|---|
| port v2 (rule_v2_g on B3's reads) vs B3's `api_port.jsonl` | 562 / 562 equal (ship, first, S, currency, n, vouched_by) |
| port v2 on records, B3's encoding, vs B3's `rec_port.jsonl` | 4,745 / 4,745 equal. With fix 1: 4,742 equal, and the 3 differ only in first check `W/T'` |
| Rust v2 (guards NONE) today vs B3's `api_rust.tsv` | 562 / 562 equal |
| Rust v2 (guards NONE) today vs B3's `rec_rust.tsv` | 4,742 equal. The 3 are 5d3fc52's `W/T'` |
| port v2 vs `served_lsa_rows.json` (ship, S, currency, n; served vouched_by a member of the kept vouches) | s61 61 / 61 and f3502 3,502 / 3,502, with the first vouch equal on all |
| port v2 vs B5's `cohort_parity_result2.json` | 563 / 563, 73 ships |
| port refusal sets vs U2's `R_single.json` / `R_excess.json` | equal (34; {7202436}) |

## Coverage limits

- **b3rec cannot exercise the guards.** The records carry no stated totals, so R10 refuses every v1 fire whatever the
  guard set: 0 ships, 6 v1 fires. Like B3's records run, they mainly test the v1 guards.
- **Scale rules read the head only in the fixtures.** The fixtures' chain is the head version plus lots-only versions
  (`gen_b3.build`), so ScalePartners comes from the head. This is the port's own API simplification, and it is gated
  at EUR 1 bn.
- **R1 and R9 winners and bids come from the served rows on both sides.** Projection parity compares winner counts
  only.
- **Projection parity is a separate harness from decision parity.** It does not run end to end through the twin. The
  two harnesses agree on their shared inputs (the projected rows equal the fixture's rows). The end-to-end protection
  remains drain stage 1's D1 on 6465510.

## Harness and runs (TEMPORARY; record copies in `harness/`)

- **Store harness.** `harness/canonical_u3_harness.rs`, test `canonical::u3_harness::u3_parity`. It is derived from
  B3's harness. It runs `awards_path_check_guards` and `head_value_with_awards_path_guards` under
  `R10Guards::ALL` (NONE / SINGLE / COVER / BOTH) and writes 4 lines per Tender.
- **Ingest harness.** `harness/project_u3_harness.rs`, test `project::u3_harness::u3_projection`. It is B3's harness
  plus gzip content reading and the `U3_PROJ_*` variables.
- **Insertion.** Both were inserted and removed by `harness/insert.py`, which makes exact, asserted edits and never
  runs `git checkout`.
- **The command.** Every cargo run used the gate's flags and package set:
  `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server u3_harness -- --ignored --nocapture`.
  Output went to a file and the run echoed `GATE-EXIT=$?`.

| run | log | what | GATE-EXIT |
|---|---|---|---|
| 1 | `outputs/run1.log` | build 3 m 04 s. Store harness: 5 populations x 4 guard sets (23.3 s). Ingest harness: projection of the 3,502 (89.1 s) | **0** |
| 2 | `outputs/run2_proj_b3api.log` | projection of the 562 (0 crates recompiled) | **0** |
| 2 | `outputs/run2_proj_s61.log` | projection of the 61 | **0** |
| 2 | `outputs/run2_proj_c563.log` | projection of the 563 | **0** |

- **Inputs did not change between the Rust runs and the final comparison.** After the two harness fixes, the export
  was re-run. Every Rust input (`*_in.tsv`) and every projection input (`*_proj_in.jsonl`) came out byte-identical to
  the files runs 1 and 2 read (`outputs/run1_inputs_sha/before_reexport.sha256`). The final comparison
  (`outputs/compare.log`, COMPARE-EXIT=0) therefore compares exactly what the Rust read.
- **Not done, as instructed.** No `ops/check.sh`, no git add or commit, no box access, no GET. Disk was 5.8 GB free
  before and after. The builds overwrote the gate's own artifact family (same hashes, e.g. `ingest-a0bb5cc3f6079573`),
  and no extra target directory was used.

## Proof that `crates/` was restored

The full output of each check is in `outputs/crates_restored_1.txt` (15:19:34Z, right after removal) and
`outputs/crates_restored_2.txt` (15:20:55Z, after the results). Both report the following.

```
$ git diff --quiet HEAD -- crates/          -> exit 0
$ git status --porcelain -- crates/         -> (empty)
crates/store/src/canonical.rs  work 42c9b2b5038a563c600c358d73017a3cb29a741950f75944b9feb14dcda1dbcd = HEAD
crates/ingest/src/project.rs   work 66b7076f30776dcc3f224e5787bdf5356f58fbd8c6b49d06bab1dc0757d170f5 = HEAD
crates/store/src/canonical_u3_harness.rs, crates/ingest/src/project_u3_harness.rs: absent; no 'u3_harness' anywhere under crates/
```

The pre-insert state (`outputs/pre_insert.txt`) had the same two sha256 values and a clean `git status -- crates/`.

## Files (`b6/u3/`)

- **Scripts.** `export_u3.py` (inputs and port decisions), `compare_u3.py`, `make_result.py`, `run.sh`, and
  `harness/` (the two `.rs` record copies and `insert.py`).
- **`inputs/`.**
  - `{pop}_in.tsv`: the Rust inputs.
  - `{pop}_port.jsonl`: the port under the four guard sets.
  - `b3rec_port_b3enc.jsonl`: the records' port side in B3's encoding.
  - `{pop}_proj_in.jsonl` and `{pop}_projport.jsonl`: projection inputs and port facts.
  - `export_meta.json`: layers, digests and per-population statistics.
- **`outputs/`.**
  - `{pop}_rust.tsv` and `{pop}_proj_rust.jsonl`: the harness outputs.
  - `u3-compare.json`: every comparison row and cross-check.
  - The logs and the two restoration proofs.
- **Large files are gzipped.** Every `.tsv` / `.jsonl` was gzipped after the run (`gzip -n`). `u3-result.json`
  `files` holds each plain file's sha256 and `files_gz` each `.gz` file's. Run `gunzip -k` before re-running
  `compare_u3.py`.
- **Not to be committed.** Per the decision record (D1), the regenerable 3,502 fixtures should not be committed.
