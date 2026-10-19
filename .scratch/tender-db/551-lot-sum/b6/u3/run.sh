#!/bin/sh
# Issue 551 B6 U3: how the Rust-vs-port parity of path W v2 + G ran on 2026-10-19 (HEAD ab9aa22), every step in the
# order it ran. The harness is TEMPORARY: harness/insert.py puts the two `mod u3_harness;` blocks and the two harness
# files into crates/ (asserted, exact edits; never `git checkout`), the cargo runs use the gate's flags and package
# set, and insert.py takes all of it out again; crates/ must then equal HEAD (`git diff --quiet HEAD -- crates/` and an
# empty `git status --porcelain -- crates/`, checked twice: outputs/crates_restored_{1,2}.txt).
# Not done, as instructed: ops/check.sh, git add / commit, any box access, any GET (every input was in a recorded
# layer, outputs/check.log; api_requests.log records none).
set -u
REPO=/home/user/tender-db
U3=$REPO/.scratch/tender-db/551-lot-sum/b6/u3
IN=$U3/inputs
OUT=$U3/outputs
GATE_FLAGS="CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0"

# 1. inputs (offline; the port b6/rule_v2_g.py sha256 23cbd850... is loaded unmodified, only its layer lists are set).
#    `check`: notice contents no layer holds (0 for every population). `export`: per population {pop}_in.tsv (Rust
#    input; b3api / b3rec are B3's own api_in.tsv / rec_in.tsv copied verbatim, and b3api is re-generated as
#    b3api_regen.tsv, byte-identical), {pop}_port.jsonl (decide under the four guard sets; f3502's equal U2's
#    decisions.jsonl.gz per id and guard set) and f3502_proj_in.jsonl. `proj`: the projection inputs of b3api / c563 / s61.
python3 -I -B $U3/export_u3.py check > $OUT/check.log 2>&1; echo "CHECK-EXIT=$?" >> $OUT/check.log
(time python3 -I -B $U3/export_u3.py export $IN) > $OUT/export.log 2>&1; echo "EXPORT-EXIT=$?" >> $OUT/export.log

# 2. the temporary harness in (crates/ clean and both files at HEAD's sha256 first: outputs/pre_insert.txt).
cd $REPO
python3 -I -B $U3/harness/insert.py insert

# 3. run 1: every decision population x 4 guard sets, plus the projection of the 3,502. Output to a file; read the
#    echoed GATE-EXIT value (0), never a filter's exit code.
env $GATE_FLAGS \
  U3_JOBS="$IN/b3api_in.tsv=$OUT/b3api_rust.tsv;$IN/b3rec_in.tsv=$OUT/b3rec_rust.tsv;$IN/c563_in.tsv=$OUT/c563_rust.tsv;$IN/s61_in.tsv=$OUT/s61_rust.tsv;$IN/f3502_in.tsv=$OUT/f3502_rust.tsv" \
  U3_PROJ_IN=$IN/f3502_proj_in.jsonl U3_PROJ_OUT=$OUT/f3502_proj_rust.jsonl \
  cargo test -p model -p store -p ingest -p tender-db --features tender-db/server u3_harness -- --ignored --nocapture \
  > $OUT/run1.log 2>&1; echo "GATE-EXIT=$?" >> $OUT/run1.log

# 4. projection inputs for the 562 / 61 / 563 (the decision record's "562 + 61 + 3,502"), then run 2: the projection
#    harness on each (U3_JOBS unset, so the store harness does nothing; 0 crates recompiled).
python3 -I -B $U3/export_u3.py proj $IN --pop b3api c563 s61 > $OUT/proj_export.log 2>&1; echo "PROJ-EXPORT-EXIT=$?" >> $OUT/proj_export.log
for pop in b3api s61 c563; do
  env $GATE_FLAGS U3_PROJ_IN=$IN/${pop}_proj_in.jsonl U3_PROJ_OUT=$OUT/${pop}_proj_rust.jsonl \
    cargo test -p model -p store -p ingest -p tender-db --features tender-db/server u3_harness -- --ignored --nocapture \
    > $OUT/run2_proj_${pop}.log 2>&1; echo "GATE-EXIT=$?" >> $OUT/run2_proj_${pop}.log
done

# 5. first comparison (it found the records' 3 first-check differences and the sorted-key chain order; see
#    u3-result.md), then the two harness-side fixes in export_u3.py / compare_u3.py (the records' port side reads a
#    withheld amount as the port's reader does; projection rows carry `served_chain`), then the re-export. The Rust
#    and projection inputs must come out byte-identical to what runs 1 and 2 read (outputs/run1_inputs_sha/).
sha256sum $IN/*.tsv $IN/*.jsonl > $OUT/run1_inputs_sha/before_reexport.sha256
(time python3 -I -B $U3/export_u3.py export $IN) > $OUT/export.log 2>&1; echo "EXPORT-EXIT=$?" >> $OUT/export.log
python3 -I -B $U3/export_u3.py proj $IN --pop b3api c563 s61 > $OUT/proj_export.log 2>&1; echo "PROJ-EXPORT-EXIT=$?" >> $OUT/proj_export.log
grep -E '\.tsv|_proj_in' $OUT/run1_inputs_sha/before_reexport.sha256 | sha256sum -c --quiet && echo "RUST AND PROJECTION INPUTS UNCHANGED"

# 6. the comparison (exit 0 iff 0 disagreements and every invariant holds).
python3 -I -B $U3/compare_u3.py $IN $OUT > $OUT/compare.log 2>&1; echo "COMPARE-EXIT=$?" >> $OUT/compare.log

# 7. the harness out again, and the byte-identity proof (check 1 right after removal, check 2 after the results were
#    written).
python3 -I -B $U3/harness/insert.py remove
git diff --quiet HEAD -- crates/; echo "diff-quiet exit $?"
git status --porcelain -- crates/
for f in crates/store/src/canonical.rs crates/ingest/src/project.rs; do
  echo "$f work $(sha256sum < $f | cut -c1-64) HEAD $(git show HEAD:$f | sha256sum | cut -c1-64)"
done

# 8. the result files.
python3 -I -B $U3/make_result.py
