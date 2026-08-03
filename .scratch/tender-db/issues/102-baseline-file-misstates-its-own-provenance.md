# 102 — the verification baseline file misstates where its numbers came from

Status: open — POST-SHIP follow-up, deliberately not fixed before the 98+99 deploy
Kind: observability / record accuracy (verification tooling)
Blocked by: — (waiting only on `issue98-de1x-org-refs` being unfrozen after the ship)
Relates to: 98 (the suite this lives in), 99, 85

## Defect

`de1x_verify.sh --baseline` writes a provenance header that always names `$BASE_URL`, even when the
numbers were read from a snapshot file:

```
# pre-fold baseline captured 2026-08-02T13:42:28Z from http://127.0.0.1:8080
PRE_TENDERS=7895452
...
```

That file was produced by `TDB_SNAPSHOT=/data/db/snapshots/tender-db-1785661162.db`. The app at
`127.0.0.1:8080` was never consulted, and — this is the part that matters — at the moment of capture it
was serving a **different** state from the snapshot the numbers came from.

Two lines are wrong, not one:

- **`from <BASE_URL>`** should name the snapshot path in snapshot mode.
- **`pre-fold`** is hard-coded, but this capture is the *post-85* state, which is the *pre-98/99* state.
  The word is only correct relative to an unnamed fold.

## Why it matters more than a cosmetic

The whole purpose of the file is to say *"these are the counts of a specific layer state at a specific
time"* — it exists so `EXPECT_NO_REGROUPING` can prove nothing regrouped between two named states. A
provenance line that names the wrong source defeats the one job the file has. And the near-miss it
enables is real and already happened once in this batch: the box carried a **pre-85** baseline whose
header looked identical in shape to the post-85 one, and running the re-verify against it would have
produced a false NO-GO indistinguishable from the genuine regrouping signal (see the 98 thread). A
correct provenance line is what lets a reader tell those two files apart at a glance.

## Fix

In the `--baseline` block:

```bash
src="${TDB_SNAPSHOT:-$BASE_URL}"
echo "# baseline captured $(date -u +%FT%TZ) from $src"
```

plus drop the hard-coded "pre-fold", or make it a parameter naming the fold the baseline precedes
(`BASELINE_LABEL="pre-98/99"`), so the file states which transition it is the "before" of.

Consider also recording the snapshot's size/mtime, so a baseline can be tied to a specific snapshot file
rather than just a path that may later be overwritten or pruned.

## Why not fixed now

`issue98-de1x-org-refs` is frozen at `aab6065` so proj-fix has a stable head to rebase onto, and the
deploy is gated behind that. A one-line cosmetic change is not worth reopening a frozen branch during a
ship. The generated file on the box is annotated by hand in the meantime, so the operator running the
re-verify sees accurate provenance; this issue makes the *tool* produce it.
