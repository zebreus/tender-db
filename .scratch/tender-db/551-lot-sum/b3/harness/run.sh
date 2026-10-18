#!/bin/sh
# Issue 551 B3: how the temporary harnesses were run (2026-10-18). Insert the three-line `mod b3_harness;` blocks named
# in each .rs file's header, copy the two .rs files next to canonical.rs / project.rs, run, then remove all of it again
# (crates/ must equal HEAD: `git diff --quiet HEAD -- crates/`). Inputs from b3/export.py, b3/records.py.
R=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b3/run
L=/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b3
: > $R/proj_empty.jsonl
B3_IN=$R/api_in.tsv B3_OUT=$R/api_rust.tsv B3_PROJ_IN=$R/proj_in.jsonl B3_PROJ_OUT=$R/proj_rust.jsonl \
  CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo test -p model -p store -p ingest -p tender-db --features tender-db/server b3_harness -- --ignored --nocapture \
  > $L/run3_api.log 2>&1; echo "GATE-EXIT=$?" >> $L/run3_api.log
B3_IN=$R/rec_in.tsv B3_OUT=$R/rec_rust.tsv B3_PROJ_IN=$R/proj_empty.jsonl B3_PROJ_OUT=$R/proj_empty_out.jsonl \
  CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo test -p model -p store -p ingest -p tender-db --features tender-db/server b3_harness -- --ignored --nocapture \
  > $L/run3_rec.log 2>&1; echo "GATE-EXIT=$?" >> $L/run3_rec.log
