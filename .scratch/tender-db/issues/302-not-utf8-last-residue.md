# 302 — name the last not-utf8 residue (~21 unexplained rows in the terminal 208)

Status: CLOSED 2026-08-27 (owner, verified against the live ledger) — the premise was
stale: every not-utf8 row already carries a named terminal class. See the accounting
below.
Kind: data quality (quarantine terminal-state completeness)
Relates to: 181 (drained the bucket to terminal 208: "early-1999 members + ~21
rows to name"), 196, 288/289 (ledger honesty on the same surface).

The not-utf8 bucket is terminal at 208 rows: the early-1999 members are explained
(pre-UTF8 encoding era, held by design), but ~21 rows were never individually
named. One bounded pass: fetch each row's member bytes (snapshot/archive read),
classify (encoding? truncation? genuine garbage?), and either reclaim (if any are
recoverable with an encoding fix) or write the per-class explanation into the
ledger so the panel's terminal number is fully accounted. Success = every one of
the 208 attributable to a named class.

## 2026-08-27 — closed against the live ledger (owner, bounded /v1/sql pass)

The success bar is already met, and was met the day 181 closed. Measured today:

- `reason='not-utf8'`: **1,700 rows total — 0 outstanding, 0 unnamed.**
  77 reclaimed; 1,623 skipped, every one with a named class:
  `text-era-non-english` 1,571 + `text-era-iso-superseded-by-utf8` 52.
- Every skip stamp dates to 2026-08-13 07:44–21:49 UTC — i.e. 181's own
  close-out classification pass covered the WHOLE bucket, residue included.
- The "~21 to name" was 181's own recorded accounting delta ("187
  early-1999-named members (record-tally) vs 208 rows" — records vs ledger
  rows), not 21 unexamined payloads; the panel's 208 counted then-outstanding
  rows, since driven to 0.

Nothing recoverable is being held: outstanding = 0. If per-row byte-level
re-verification of the two skip classes is ever wanted, that is an archive
read pass, but the ledger completeness this issue asked for exists.
