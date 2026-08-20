# 101 — surface the DE-1.x award-winner gap in the data-quality panel, not only the quarantine ledger

Status: DONE 2026-08-20 — section 3 of the data-quality report now carries `with winner` and its rate
against the notices that materialised a result, so "a result block with nobody in it" is one line
rather than an inference across two sections. Gated by a test and falsified
Kind: observability / honest disclosure
Blocked by: —
Relates to: 100 (the gap being disclosed), 27 (the data-quality report), 40 (the resolution ledger), 98/99

## Why

Shipping 98+99 puts an honest disclosure of the DE-1.x award-winner gap into the quarantine ledger's
`diagnosis` (issue 100 is the defect; the ledger entry names it in caps and cites the issue). That is the
right home *for now* — it sits next to the "Resolved" claim it qualifies, which is exactly where a reader
who might be misled is standing.

But it is an odd sole home. The ledger answers *"what failed to parse, and was it fixed?"*. The winners
gap is not a parse failure: those notices parse, project, and render. It is a **completeness** gap — which
is the data-quality report's question (issue 27: *"how complete is the data we did import?"*). A user
asking "can I trust DE award data?" reads the coverage/quality panel, not the quarantine table, and today
that panel would show DE-1.x award notices with lot results and contracts and simply no winners, with
nothing saying why.

There is also a shelf-life problem: when the 241 residual are eventually reclaimed the ledger entry's live
counts go to zero, and an entry whose counts are zero is exactly the one a reader skims past — taking the
winners disclosure with it.

## What

Surface the gap where completeness is measured:

- `data_quality`'s per-era table already reports `winner` density per profile (`FIELDS`, ingest/
  src/data_quality.rs). The three `eforms:eforms-de-1.*` rows will read ~0% for winners while their
  title/buyer/cpv read high — a shape that currently looks like an unexplained anomaly.
- Attach a known-gap annotation to those rows: winners not resolved for this dialect, cause, issue 100.
  The report is "descriptive, not pass/fail" by design, so this is an annotation, not a threshold.
- The dashboard's coverage/quality panel should render the annotation next to the number, so the 0% is
  read as *disclosed and understood* rather than *broken and unnoticed*.

## Note

Deliberately additive. It changes no projection behaviour and gates nothing — the ship gate for 98+99 is
the ledger disclosure (verified live by the verification suite's section I). This is about the disclosure
outliving the ledger row it currently lives in, and reaching the panel where the question is actually
asked.


## How it landed (2026-08-20)

Not as prose beside the numbers — a note saying "DE-1.x winners are broken" goes stale the day it is
fixed, and a reader who skims past a zero-count ledger entry (this issue's own shelf-life argument)
will skim past a stale note too. As a **column**, which measures itself:

    == 3. Results materialisation (notices whose PUBLISHED type announces a result → lot_results) ==
      era                     award-notices  with lot_results  density  no block parsed  with winner  named

`with winner` counts the versions that resolved one; `named` is its rate **against the notices that
materialised a result**, not against every award notice — a notice with no result block is already
counted one column left, and dividing by it twice would blame the winner chain for a missing block.
The test pins that choice with a 1,000/500/250 row that must read 50 %, not 25 %.

What it makes visible, on today's stored report, without any inference:

    eforms-de-1.1   60,059 award notices → 60,037 results (100.0%) → ~3% named   (issue 100)
    eforms-de-2.1   61,938 → 61,931 (100.0%) → ~80% named
    sdk-0.1        138,950 → 138,950 (100.0%) → ~5% named                        (issue 257, filed today)

The middle row is the control that makes the other two legible: the same report, the same era family,
a working chain. Before this column, getting there meant multiplying section 1's `winner` share by an
era's award fraction in your head — which is exactly how I nearly mis-read sdk-0.1 as "1.1 %, probably
its ceiling" until I looked at section 3's denominator (issue 231's note, now issue 257).
