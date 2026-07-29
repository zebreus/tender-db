# Bulk quarantine recovery reclaims all buckets, then folds once

ADR-0004 keeps unparseable notices in a strict quarantine — raw payload retained,
`reprocessed_at` NULL, never dropped — so they can be reclaimed once the parser
gains the missing capability (a vendored SDK version, a serializer quirk, an
xpath grammar extension). By mid-2026 that backlog was ~2.4M notices (~14.6% of
the corpus): mostly eForms whose SDK version we had not vendored
(`unknown-customization`), plus the text-era `OC` field bucket and a 2008-05
DTD-XML bucket. This ADR records how we fold that backlog in — the reclaim
mechanism (issue 76) plus the batch strategy — after the first attempt at
per-bucket incremental folding hit two scaling walls.

The reclaim mechanism re-reads a quarantined member's archived package, re-parses
it, and writes the parsed layer in place, flipping `parse_state` to `parsed`,
`projected` to 0, and setting `reprocessed_at`. It is one `Spec::Reprocess`
admin job per bucket, keyed by `reason` + optional `detail LIKE` + optional
`profile`, isolated on the job runtime, bounded-memory, and resumable by the same
per-package cursor as any process job (ADR-0007).

**Decision: reclaim every bucket first (parse layer only), then fold the whole
recovered corpus with one `rebuild:true`, rather than folding each bucket as it
reclaims.** A `reclaim_only` flag on the reprocess job omits the trailing
incremental projection, leaving the reclaimed notices `projected = 0` for the
final rebuild to fold in bulk.

The first bucket (SDK 1.7, ~300K) was reclaimed with the default trailing
*incremental* fold, and that exposed why incremental is the wrong tool here. The
incremental projection (issue 58) was built for a ~15K daily delta: it is
`O(delta)` in RAM (it loads the whole delta's parsed layer to build the plan) and
its Phase-2 re-reads each touched tender's notices with ~10 cold random seeks per
notice. At 300K that OOM-walked into swap, and at the read side a 300K delta is
~13M random seeks (~overnight per bucket). A full rebuild avoids both: it streams
the parsed layer sequentially through the bucketed sequential fold (Phase2::
Buckets), bounded RAM, and one sequential ~254GB pass beats ~13M random seeks by
a wide margin. So the ~1M+ reclaimed notices fold once, sequentially, in
~low-hours instead of ~days of incremental folds — and reclaims run back-to-back
with no per-bucket fold between them.

Two scaling fixes were required along the way and are load-bearing for any future
bulk reclaim: an index on `quarantine.notice_id` (issue 80 — the per-member
reclaim flag was full-scanning the 2.4M-row table, `O(held × 2.4M)`, which
cliffed a dense package to ~1 member/s), and bounding the incremental fold's
Phase-1 by streaming it in id-ordered chunks while keeping grouping and the fold
global for byte-identity (issue 81 — kept because the daily incremental still
uses it).

The final rebuild renumbers every tender in fold order and re-emits the CDC
`changes` feed, which `reset_tender_layer` appends to rather than replacing. With
no feed consumers yet and this rebuild establishing the recovered baseline, the
rebuild takes an opt-in `clear_changes` that DROP+recreates the feed first, so it
emits one clean generation reflecting the final layer instead of stacking a fresh
~50-60M-row generation of stale-id events onto the accumulated ~80M. The one
internal consumer, the coverage refresher, only reads the cursor as a
change-detector (it never resumes from it), so the reset triggers a single
self-healing re-measure.

Rejected alternatives: per-bucket incremental folds (the natural default, and
what the trailing project does) — `O(delta)` RAM and `O(delta)` random-seek
Phase-2 make it OOM-prone and days-slow at bulk scale, and it re-folds the same
tenders once per bucket. Suppressing the rebuild's change-events instead of
cleaning the feed — wrong, it would leave reclaimed tenders with no `added`
baseline for future consumers. Building the read-side blob piggyback (issue 63,
which turns a rebuild's Phase-2 from ~low-hours into ~minutes) before running the
recovery — it saves only a few hours on this one-shot, is the most
byte-identity-critical change in the system, and couples to the resume invariant
the live recovery depends on; it is deferred to a focused post-recovery session.

Consequences: recovery is a fast reclaim phase (bounded by archive read I/O —
reading whole monthly tars to reach sparse held members, the honest cost, ~a day
for the full backlog) followed by one rebuild. The tender layer is absent for the
reclaimed notices until that rebuild runs — acceptable because the parse layer is
the source of truth and the projection is a pure function of it. `reclaim_only`
and `clear_changes` are recovery-shaped flags, not part of the steady-state daily
pipeline, which stays incremental (small delta, already fast). The reclaim-side
read amplification (walking tars for sparse members) is the reclaim analog of
issue 63 and is a candidate future optimization (store held members' tar
byte-offsets to seek instead of walk).
