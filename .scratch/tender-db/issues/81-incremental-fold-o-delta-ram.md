# 81 — incremental project fold is O(delta) in RAM (OOM on a large reclaim)

Status: fixed + green (team-lead reviews, deploys before folding the big reclaim buckets)
Kind: performance / bounded-memory-principle violation
Relates to: 58 (incremental projection), 76/77 (reprocess), bounded-memory-principle
Found: 2026-07-29, SDK 1.7 reprocess trailing project (job 2) — 299,829 reclaimed

## Symptom (run-driver)

After the reprocess reclaimed ~300K notices, the trailing `project rebuild=false`
(the incremental fold of that delta) climbed RAM ~780M/min MONOTONIC — RSS 3.2GB+
into swap, memAvail dipping to 1.7G. The full REBUILD folds 12.4M with flat RAM
(it streams), so the streaming path is bounded; the INCREMENTAL path was not.

## Root cause

`project_incremental_inner` loaded the ENTIRE delta into RAM before folding — four
O(delta) structures, all holding full content:
- `changed_parsed = parsed_by_ids(changed)` — every changed notice's full parsed
  layer (sections + all value rows).
- `extra_parsed = parsed_by_ids(extra_ids)` — same for every touched-EXISTING
  Tender's notices.
- `rows: Vec<PlanRow>` sized `changed + extra` — the whole plan in RAM.
- `mentions: Vec<Mention>` — every changed notice's org mentions.

Fine for a ~15K daily delta (the path's design point); GBs for a 300K reclaim, and
OC (577K) would be ~2×. The full rebuild avoids this by streaming Phase 1 to the
on-disk plan; the incremental path short-circuited that by building the plan whole.

## Fix — stream Phase 1, keep grouping + fold GLOBAL

Grouping (SQL, on-disk) and Phase 2 (already batched) were already bounded — only
Phase 1 (the parsed-layer read + plan build) held O(delta). So:

- Pass 1 (streamed, id-ordered chunks): read the changed notices' idents — legacy
  check + collect new keyed keys — without holding their parsed layer.
- Compute the touched existing Tenders + their notices; the plan set is
  `changed ∪ touched-existing`, in one global id order.
- Pass 2 (streamed, id-ordered chunks): build the ONE plan a chunk at a time
  (`insert_plan` per chunk), resolving mentions per chunk. Peak RAM = one chunk.
- Then ONE global `build_plan_groups` + the batched Phase-2 fold — unchanged.

Because grouping and the fold stay global, the output — surrogate ids included — is
BYTE-IDENTICAL to the old whole-delta path; only the parsed-layer read is chunked.
Peak RAM is flat vs delta size (one 50K chunk's parsed layer + the resolver's
org cache + the touched-id lists), well under 4GB even for OC 577K.

## Validation

- New `incremental_chunked_is_byte_identical_to_single_pass`: folds a multi-Tender
  delta (incl. a new keyed Tender whose two notices span chunks) with chunk=1 and
  asserts the canonical layer is byte-identical to a single-pass run, and that the
  cross-chunk Tender did not split.
- The existing incremental output-identity tests (incremental == full non-rebuild)
  stay green; full store+ingest+app suites green incl. project_equivalence,
  project_resume, project_fold_source, project_golden, project_memory.

Deploy before folding the big reclaim buckets (1.10 255K, OC 577K, DE 218K).
