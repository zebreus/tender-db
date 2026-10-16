# 562 — a whole-Tender rewrite of `source` or `kind` (a DÖE Tender turning `ted`) is invisible to an SSE subscription on it

Status: ready-for-agent — filed 2026-10-16 from issue 547's build review. Next: decide the shape (a per-version source
read, or a seq-less tender `changed` when the label flips), then build.
Kind: correctness (SSE `added`/`removed` on `source` and `kind`)
Relates to:
- 547: listed `kind` and `source` as "probably harmless"; the build found they are rewritten for the whole Tender and
  left them on `t.*` (comment in `tenders_query`).
- 462: `kind` is re-derived from the fold-first notice.
- 495 / ADR-0017 D3: a stale refold (compare On) already emits a seq-less correction row, which the in-place path
  classifies `(false, false) => removed` correctly; an ordinary current-epoch fold does not.

## Verify

    grep -c 'fn a_ted_twin_joining_a_doe_tender_reaches_a_source_subscription' crates/app/tests/api.rs

- **done**: 1 (the test passes: a `source=doe` tape hears `removed` and a `source=ted` tape `added` when a TED twin
  joins), OR a `## Decision` below keeps the gap, documented in CHANGELOG.
- **open (2026-10-16)**: 0; the CHANGELOG 547 entry names the gap.

## What was found

`canonical.rs` (~22446-22457) rewrites `tenders.source` (`primary_source`: `ted` once any notice is TED) and
`tenders.kind` in place. `tenders_query` emits `AND t.source = ?` / `AND t.kind = ?` on both `Scope::At` probes, so
seq and seq − 1 read one label. Reproduced in the 547 review: CHAIN[0] as a DÖE notice, then CHAIN[1] as TED for the
same procedure → the Tender goes `doe` → `ted`, pages agree, `/v1/changes` holds only `tender 1 changed version 2` and
`lot 1 changed version 2`; the `source=doe` tape got nothing (10 s), the `source=ted` tape got `changed` (not `added`).
`kind` was not run; same mechanism.

## Units

1. **Decide (S).** A: source at a version = `ted` if any version up to that seq was caused by a TED notice (an
   EXISTS over `tender_versions` ⋈ `notices` keyed `(tender_id, seq <= ?)`; read its plan); kind at a version from the
   fold-first notice up to it. B: when the fold flips the label, emit a seq-less tender (and lot) `changed`, which the
   in-place path classifies by the head alone.
2. **Build (S–M)**, with the api test the Verify names.
