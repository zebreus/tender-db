# 565 — a Tender-scope filter on `/v1/lots` re-evaluates a lot only when the lot's own change row is written

Status: ready-for-agent — filed 2026-10-17 as unit U-a of issue 560's decision (workflow wf_3176baf8-d14). Next:
re-baseline the reach after 560's build ships (its A-EUR rows and D5's seq-less rows cover part of today's count), then
decide between A-wide (every surviving lot `changed` whenever the tender row is written) and an SSE-side fan-out.
Kind: correctness (SSE `added`/`removed` on `/v1/lots` under Tender-scope filters)
Relates to:
- 560 (a Tender value move with no lot row; its decision builds A-EUR and named this wider gap);
- 533 (closure, the same gap for `status`, closed in both arms);
- 493 (a lot's own value can move without a row; the cost argument against writing every lot);
- 547 (the SSE `Scope::At` probe at the probed version).

## Verify

    grep -c 'version_predicates' .scratch/tender-db/issues/565-*.md

- **done**: the chosen option is built, and an api test shows a `/v1/lots?winner=` (or `country=`, `cpv=`, `buyer=`,
  `bidder=`, `procedure_type=`, `currency=`, `status`/deadline) subscriber hearing `added`/`removed` for a lot whose
  Tender — not the lot — moved into or out of the filter.
- **open (2026-10-17)**: such a move writes no lot row, so the lot stream never re-evaluates the lot.

## What was found

`version_predicates` (crates/store/src/read.rs ~1268–1330) reads the Tender-scope filters — country, cpv, buyer,
winner, bidder, procedure_type, currency and the deadline behind `status` — at the Tender VERSION `(tid, seq)`, not per
lot. A lot stream re-evaluates a lot only when that lot's own change row is written. A version that moves only
Tender-scope facts writes a tender row and no lot row, so `/v1/lots` with one of these filters misses the move. Example:
a second contract award notice that names a winner on an already-closed Tender, with the lot facts unchanged, writes no
lot row, so `/v1/lots?winner=<org>` never hears the lot.

Starting measurements (560's A-wide design, 2026-10-14/15): W1 332 versions / 3,619 uncovered lots (56 versions with no
lot row at all), W2 3,628 / 17,012 (2,794). These include lots 560's A-EUR now covers and credit neither D5's seq-less
rule-L corrections nor 533's closure rows: re-baseline after 560's deploy before weighing A-wide's volume.

## Options

- **A-wide**: write lot `changed` for every surviving lot whenever the tender row is written. Exact, but O(lots) rows
  on every tender change; 493's cost argument (a lot's own value moving without a row measured 0 of 14,480, so it was
  left documented) weighs against it unless the re-baselined reach is large.
- **SSE fan-out**: on a tender row, the lot stream probes each lot of the Tender at that version (no extra stored rows;
  a read-side cost per tender event per lot subscriber).
- **Document**: keep the gap and name it in /docs' Live feed exceptions, if the re-baselined reach is negligible.
