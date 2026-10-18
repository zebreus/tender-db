# Issue 551, unit 4b B3: port parity result (2026-10-18)

**Verdict: PASS.** The Rust v2 (`awards_path_check` with path W on) and the port agree on every row of every
population. There are 0 disagreements on ship or refuse, S, currency, n, the first refusing check, `vouched_by` and
the derived value. Projection parity is clean on every B1 field. The invariant "every v2 fire is a v1 fire" holds.
`crates/` is byte-identical to HEAD. A verifier re-ran the whole thing independently and got byte-identical outputs.

The pass comes with one finding for B4 and B5: the 567/568/570 drain moved the port's own outcomes. There are 18 new
ships and 0 lost. The EUR 100 m band is now 20 heads, M2's 15 plus 5 new ones. See "Finding for B4/B5" below.

Machine-readable copy: `B3-result.json` (same directory).

## Inputs and pins

| item | value |
|---|---|
| repo HEAD | `b16383a` (B1 `3c81386`, B2 `d4b8d76`) |
| `crates/store/src/canonical.rs` | sha256 `5c0aa239b128ffe4bae95468af886ccf3ad12e063beadfcb2b6c9b7cfc28bb5e` (equals HEAD and `pre_insert.sha256`) |
| `crates/ingest/src/project.rs` | sha256 `eee59a98310eddb930de905c2af585106ea5b1483c9645385bda943e3a1f4968` (equals HEAD and `pre_insert.sha256`) |
| committed port `unit4b/rule_v2.py` | sha256 `39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce` (untouched) |
| `570-lot-keys/remap_port.py` (option B) | sha256 `abe2b78ae5a0506dd327a412f9753c77a8d2a448fba080d4d73c0aae083bd503` |
| `568-r208-lots/readback-u8/combined_port.py` | sha256 `5363593c663c651bcbf125464ccfd24b96d45b56e1834086352f7d8903d7673c` |
| **B3 port copy `b3/rule_v2_b3.py`** (the port of record for B5) | sha256 `5400bc555f15ba09e852afe132009a900a2e5a3634e7546ff703fc9a8a89ade5` |
| **B3 fixture builder `b3/gen_b3.py`** | sha256 `2e9635bae0a49b7924780d7829d549ec9edf5180a69c37d9b6062aa83cc6d9de` |

**Tender reads.** Every Tender was re-read after the drain. That was 562 GETs of `/v1/tenders/{id}`, all made
between 17:09:53Z and 17:23:58Z on 2026-10-18, after job 2617 finished at 15:21Z. There were 0 failures.

**Notice contents.** All 4,416 version notices came from the caches, in this order: the M1 cache, then B2's bundle,
then the u570 cache. None was missing, so no notice was fetched. A refold does not change notice contents.

**The 4,745 records.** These come from `551-lot-sum/results-slim.jsonl`. Each side is fed the same recorded rows.

## Port changes (copies only; every edit is an asserted replacement in `make_port_copy.py`)

1. **570 remap.** `pub_no` now reads the notice's own lot section whose remap target is the served key. It does this
   with `combined_port.remap_chain` (`remap_port.remap` plus K3, both sha256-pinned) over the version chain in fold
   order. The fixture builder's B1 carry lands each section on its target key.
   - This edit is needed. 66 Tenders hold 364 remapped notices, and without the edit the port moves 20 decisions
     among them.
   - 6 of those 20 are Rust ships: 6569442, 7139761, 7153588, 7475504, 7722197 and 7789396.
2. **568 unit 8.** No edit was needed. The port reads the served `lot_results`, where a refused block is `lot: null`.
3. **R10 at every size**, per the M2 fallback that passed.
4. **FTS legacy fix.** `CP.is_legacy` now ignores `TED-VAL_TOTAL_TAX_BASIS`, which `fts/parse.rs` writes on OCDS
   notices.
5. **Offline layered reads.** The copy reads the post-drain Tenders first, then the B3 notices, the M1 cache and
   B2's bundle.
6. **`facts()` split.** It is now `facts_from(x, contents, v1)`, so the records can be fed in.

## Decision parity (store harness, `awards_path_check` with path W on)

| population | compared | agree | port ships | Rust ships | v1 fires (port / Rust) | v1 fires pre-drain |
|---|---|---|---|---|---|---|
| labelled 279 | 279 | 279 | 30 | 30 | 276 / 276 | 279 |
| out-of-sample 6 | 6 | 6 | 2 | 2 | 6 / 6 | 6 |
| walked 277 (M1 54 + fallback 223) | 277 | 277 | 41 | 41 | 273 / 273 | 276 |
| records 4,745 | 4,745 | 4,745 | 0 | 0 | 6 / 6 | n/a |
| **total** | **5,307** | **5,307** | **73** | **73** | **561 / 561** | |

- No Tender is missing from the Rust export (`not_exported` is empty).
- **What `compare.py` compares on every row:** ship, first refusing check, S, currency, n and v1-fires. On ships it
  also compares `vouched_by` (field, notice, cents) and the derived value (kind, cents, currency, lots).
- **The verifier tested the comparison by mutation.** Changing one field at a time was caught every time, including
  a dropped row and a change to a v1 check on the records side.

### Invariant: every v2 fire is a v1 fire

The invariant holds on every population. It was checked three ways, with 0 violations each:
- every Rust ship is a v1 fire in the port;
- every Rust ship was a v1 fire in the pre-drain (M1/M2-era) reads;
- every port ship is a v1 fire in the port.

The fourth check, "Rust ship ⊆ Rust v1", is circular: the harness derives Rust's v1 from the first check. So the
verifier also diffed `awards_lot_sum` between `3c81386` and HEAD. The v1 guard conditions are unchanged; the diff
only splits `copy_suspect` into `over_cap` and `refused`. R1–R10 run only after G4.

## Disagreement list and dispositions

| # | what | where | count | disposition |
|---|---|---|---|---|
| 1 | decision (ship/refuse, S, currency, n, first check, `vouched_by`, derived) | every population | **0** | none needed |
| 2 | Rust ships where the port refused (the parity protocol says read these under the M2 bar) | every population | **0** | none needed |
| 3 | Rust refuses where the port shipped (the parity protocol says drop these from the 30) | every population | **0** | none needed |
| 4 | known B2 name-level differences (`vouched_by` order, `lot_number_tokens` ASCII digits, title tie order, the B1-carry vs award-block R2 number) | 73 ships | **0** | did not occur |
| 5 | projection: `LotState.kind` | 122 rows in 18 Tenders | 122 | **Input difference, no decision effect.** See below. |
| 6 | projection, first pass only: 75 result rows in 15 FTS Tenders | export input | 75, then 0 | **Fixed in the input**, not in Rust or the port. See below. |

**Row 5, `LotState.kind`.** The port says `ProcurementProjectLot` and the fold says `Lot`. All 18 Tenders mix DE1
notices with eForms ones: 49224, 255403, 448013, 521716, 677680, 680425, 750804, 806550, 822558, 866035, 866431,
911043, 912466, 936207, 985336, 989268, 1079142 and 1094914.
- The cause is that the fold's `normalise_de1` renames the kind, while the port reads the raw one.
- Kind is not a B1 field, and those rows' lot numbers agree.
- Path W reads the served kind at the head, and the remap does not run on DE1 notices, so no decision is affected.

**Row 6, the FTS first pass.** `export.py`'s profile inference and `combined_port.is_legacy` both read
`TED-VAL_TOTAL_TAX_BASIS`, which the FTS parser writes on OCDS notices, as a sign of legacy TED. Both now ignore it.
The tables in this file come from the fixed run.

## Projection parity (ingest harness)

B1's real `normalise_de1` → `NoticeState::read` → `bind_organizations` → `fold` was fed each parsed
`/v1/notices/{id}/content`. That is the same sequence `apply_plan_batch` uses. The output was compared field by field
with the port's facts that feed R1, R2, R3 and R10, on the 562 API Tenders (279 + 6 + 277).

| field | rows compared | agree |
|---|---|---|
| `LotState.published_lot_no`, every version (4,416 version lot sets) | 26,764 | 26,764 |
| `LotState` key sets per version | 4,416 | 4,416 |
| head lot keys vs served `lot_details` | 562 | 562 |
| `LotResultState.lot_key` (567 + 568-u8 + 570 vs served) | 6,699 | 6,699 |
| `LotResultState.published_lot_no` | 6,699 | 6,699 |
| `LotResultState.group` | 6,699 | 6,699 |
| `TenderVersion.stated_totals` incl. `lots_awarded` (multiset) | 1,737 | 1,737 |
| version chain vs served versions | 562 | 562 |

- The verifier confirmed by mutation that each comparison catches a change.
- **570 remap coverage.** Across the 562 there are 364 remapped notices (in 66 Tenders), 303 refused silent-merges
  and 1 refused injective: 74 Tenders in all. All of them agree with the Rust fold.

## Coverage gaps (not disagreements)

**Winners and bids are not compared.** R1 and R9 read them.
- Award cents match on all 6,699 result rows.
- The harness binds organisations synthetically, so it gives 58 rows more winners than the served data.
- The decision side reads the served winners on both sides, so no outcome is affected. The projection of winners and
  bids is simply not independently verified.

**The 4,745 records mostly test v1 guard parity.**
- The recorded rows carry no B1 facts.
- Both sides give 0 ships and 6 v1 fires, all refused: R5.1 ×5 and R2d ×1. The 6 are 585917, 5995030, 6450467,
  6450469, 6450473 and 8426318.
- The other first checks are W/T' 3,372, W/K 696, W/G0 632, W/G1' 20, V 15, W/G5 2, W/G3 1 and W/G6 1.

**7511890 is not in B3's populations.** docs/operations.md's 570 block names it, so the verifier checked it
separately. It used one fresh GET (`scratchpad/b3/verify/raw/tenders/7511890.json`), the u570 cached notices, and the
harness while it was in place. Port and Rust agree:
- Both decide W/T' (not a v1 fire), with S PLN 339,546.48 and n = 4.
- 22980479 RES-1 sits on LOT-3 with lot number "3", taken from the notice's own LOT-1 section moved onto LOT-3.
- The section that printed 6 (its own LOT-3) lands on LOT-6, which carries "6".
- The port's `INV[22980479]` is {LOT-3: LOT-1, LOT-4: LOT-2, LOT-6: LOT-3}.

**Spot check by the verifier.** Ten random agreeing ids (seed 5510318) were checked against the notices: 6330626,
7265996, 871648, 7001924, 5747154, 1043054, 6862752, 536585, 401929 and 8469517. S, currency and n match a manual
latest-award sum on all 10, and each first refusing check or vouch is borne out by the notice text.

## Finding for B4/B5: the drain moved the port's own outcomes

Against the M1/M2-era outcomes (`results/B3-movers.json`), there are **18 new ships and 0 lost**.
- 12 came from the drain itself: the 567 refold, 568 plus unit 8's guards, and served lot keys.
- 6 came from reading 570's remap (port edit 1).

| id | population | pre-drain | now | S now | vouched by |
|---|---|---|---|---|---|
| 5985466 | labelled, wrong/double_count (on old S EUR 124.79 m, n 4) | R2a | ship | EUR 96,003,380.66, n 3 | TED-VAL_TOTAL |
| 6569442 | labelled, neutral/other | R2c | ship (remap) | CHF 244,681,010.00, n 29 (old n 21) | TED-VAL_TOTAL |
| 6595330 | labelled, right | R2c | ship | EUR 2,295,195.00, n 2 | TED-VAL_TOTAL |
| 6603588 | labelled, wrong/double_count (on old S EUR 117.22 m) | R2a | ship | EUR 154,611,342.00, n 7 | TED-VAL_ESTIMATED_TOTAL |
| 6679375 | labelled, right | R2c | ship | EUR 238,500,000.00, n 3 | TED-VAL_TOTAL |
| 7139761 | labelled, right/other | R2c | ship (remap) | EUR 166,731,929.69, n 13 (old EUR 101.15 m, n 11) | TED-VAL_ESTIMATED_TOTAL |
| 7542368 | labelled, right | R2c | ship | EUR 179,840,748.93, n 3 | TED-VAL_TOTAL |
| 5862601 | walked | R2a | ship | HUF 1,683,175,196.00, n 19 | TED-VAL_TOTAL |
| 5888594 | walked | R2c | ship | HUF 91,683,772.00, n 2 | TED-VAL_TOTAL |
| 6317191 | walked | R2a | ship | CHF 61,378,150.00, n 6 | TED-VAL_TOTAL |
| 7153588 | walked | R2c | ship (remap) | EUR 19,128.00, n 4 | TED-VAL_TOTAL |
| 7177672 | walked | R2c | ship | HUF 315,979,996.00, n 4 | TED-VAL_TOTAL |
| 7298633 | walked | R2c | ship | PLN 3,583,769.75, n 9 | TED-VAL_TOTAL |
| 7398935 | walked | R2c | ship | EUR 14,446.00, n 2 | TED-VAL_TOTAL |
| 7475504 | walked | R2c | ship (remap) | BGN 35,531,694.00, n 4 | TED-VAL_ESTIMATED_TOTAL |
| 7722197 | walked | R2c | ship (remap) | EUR 179,360.00, n 4 | TED-VAL_TOTAL |
| 7789396 | walked | R2c | ship (remap) | BGN 28,400.60, n 6 | TED-VAL_ESTIMATED_TOTAL |
| 8371658 | walked | R2c | ship | EUR 14,936,120.00, n 4 | TED-VAL_TOTAL |

**The band (S ≥ EUR 100 m) is now 20 heads, not M2's 15.**
- It contains all of M2's 15: 683951, 707424, 726354, 1102571, 1147190, 1148297, 5278856, 5842187, 6017838, 6057925,
  6195903, 6370901, 7001924, 7011414 and 8426318.
- It adds 5: **6569442** (EUR 222.48 m), **6603588** (EUR 154.61 m), **6679375** (EUR 238.50 m), **7139761**
  (EUR 166.73 m) and **7542368** (EUR 179.84 m).
- None of M2's 15 left the band.
- 5842187 (EUR 101.03 m) sits within 2 % of the threshold.

**v1 no longer fires on 6 ids** after the drain. None of them was a ship, before or now.

| id | pre-drain first check | first check now |
|---|---|---|
| 5571139 | R2a | W/G1' |
| 5669744 | R3 | W/G0 |
| 6852738 | R2c | W/G3 |
| 6969157 | R2c | W/G1' |
| 7372694 | R2c | W/G1' |
| 7495109 | R2a | W/G1' |

**Consequences:**
- B5's stop rule "a band set different from M2's" would trip on these 5 new band heads.
- 6603588 and 5985466 now ship, though both were labelled double_count on their pre-drain sums.
- So the 5 new band heads need an M2-bar read before B5's band stop rule can be evaluated. Arguably all 18 new ships
  do. That means the adversarial band re-read, with 0 wrong of any class allowed. B4's operations.md text names the
  band and the bar, so this read should come before B4.

## Harness location and runs

**Record copies** are in `.scratch/tender-db/551-lot-sum/b3/harness/`. While the runs were going, each was included
by a three-line `#[cfg(test)] #[path = ...] mod b3_harness;` block. That block and the file have both been removed.
- `canonical_b3_harness.rs` (sha256 `deff4f18…0712`) ran from `crates/store/src/`, included from `canonical.rs`.
  Its test is `canonical::b3_harness::b3_parity`.
- `project_b3_harness.rs` (sha256 `c833efd7…0e43`) ran from `crates/ingest/src/`, included from `project.rs`. Its
  test is `project::b3_harness::b3_projection`.

**`run.sh`** ran both with the gate flags and the gate package set:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p model -p store -p ingest -p tender-db --features tender-db/server b3_harness -- --ignored`.
Output went to a file, and the run then echoed `GATE-EXIT=$?`.

| run | log | both harness tests | GATE-EXIT |
|---|---|---|---|
| implementer, records smoke | `scratchpad/b3/run1.log` | ok | 0 |
| implementer, first full run (before the FTS input fix) | `scratchpad/b3/run2.log` | ok | 0 |
| implementer, final API run | `b3/results/run3_api.log` | ok | 0 |
| implementer, final records run | `b3/results/run3_rec.log` | ok | 0 |
| verifier, API run | `scratchpad/b3v/run_api.log` | ok | 0 |
| verifier, records run | `scratchpad/b3v/run_rec.log` | ok | 0 |
| verifier, 7511890 | `scratchpad/b3v/run7511890/run.log` | ok | 0 |

- `ops/check.sh` was not run, as instructed. Nothing was committed.
- **The verifier's outputs are byte-identical to the implementer's** (cmp):
  - `api_rust.tsv` (sha256 `005a4b5a…e090`)
  - `rec_rust.tsv`
  - `proj_rust.jsonl`
  - `compare.json` = `B3-parity.json` (sha256 `38187799…f7f1`)
  - all five regenerated inputs
  - `rule_v2_b3.py` and `gen_b3.py`, which are exactly what `make_port_copy.py` writes

## Byte-identity proof (`crates/`), re-checked by the reporter at HEAD `b16383a`

```
$ git status --short crates/            -> (empty)
$ git diff --stat crates/               -> (empty)
$ git diff --quiet HEAD -- crates/      -> exit 0
$ sha256sum crates/store/src/canonical.rs  == git show HEAD:crates/store/src/canonical.rs | sha256sum
  5c0aa239b128ffe4bae95468af886ccf3ad12e063beadfcb2b6c9b7cfc28bb5e (both)
$ sha256sum crates/ingest/src/project.rs   == git show HEAD:crates/ingest/src/project.rs | sha256sum
  eee59a98310eddb930de905c2af585106ea5b1483c9645385bda943e3a1f4968 (both)
$ find crates -name '*b3*'              -> (none)
```

`LOT_SUM_AWARDS_PATH` is still `false`, because `canonical.rs` is unchanged.

## Records to land before B4 (no code changes)

1. **Port of record.** Record in issue 551 that the remap-aware copies are the port of record: `b3/rule_v2_b3.py`
   (`5400bc55…ade5`) and `b3/gen_b3.py` (`2e9635ba…d9de`). B5's cohort check uses them, not `unit4b/rule_v2.py`.
   They exist only in the untracked `b3/` directory, so they must be committed together with
   `make_port_copy.py`, the harness copies and the scripts.
2. **7511890.** Record its result (above) in the issue, and add it to B5's id cohort.
3. **Coverage notes.** Note in the issue that projection parity does not cover R1/R9's winners and bids, which come
   from served data. Also note that the 4,745 records mainly test v1 guard parity. Optionally, make `compare.py` exit
   non-zero on any decision disagreement before B5 reuses it.
4. **New band heads.** Read the 5 new band heads (6569442, 6603588, 6679375, 7139761, 7542368) under M2's adversarial
   band bar, and preferably the other 13 new ships under the reader protocol. Record the result as B5's reference
   band set instead of M2's 15.

## Files

`.scratch/tender-db/551-lot-sum/b3/` contains:
- the port copies and their patcher: `rule_v2_b3.py`, `gen_b3.py`, `make_port_copy.py`;
- the scripts: `fetch.py`, `export.py`, `records.py`, `compare.py`, `movers.py`;
- `harness/`;
- `results/` (`B3-parity.json`, `B3-movers.json`, `api_port.jsonl.gz`, `api_rust.tsv`, `rec_port.jsonl.gz`,
  `rec_rust.tsv.gz`, `proj_rust.jsonl.gz`, `run3_api.log`, `run3_rec.log`).

The raw reads and run inputs are under `/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/`:
- `b3/`, the implementer's: `raw/tenders`, `bundle/`, `run/`, `run1.log`, `run2.log`, `pre_insert.sha256`;
- `b3v/`, the verifier's: the regenerated run, `compare.json`, `run7511890/`, and the mutation tests in `mut/` and
  `mutp/`.

These are session-scoped and will be lost. `b3/results/` in the repo, still uncommitted, holds everything the
verdict rests on.
