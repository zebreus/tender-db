# 302 — name the last not-utf8 residue (~21 unexplained rows in the terminal 208)

Status: BACKLOG (filed 2026-08-26 on Lennart's request — remaining quarantine work)
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
