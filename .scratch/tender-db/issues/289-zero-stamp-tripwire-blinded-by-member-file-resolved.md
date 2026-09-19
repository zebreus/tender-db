# 289 — the "reclaim stamped NO ledger rows" tripwire is suppressed for any per-record stranding inside a partially-resolved member file

Status: RESOLVED-DEPLOYED (board sweep 2026-09-19) — `member_family_still_held` and the shared `log_zero_stamp` verdict are in the served tree (`110d527`); the tripwire has stayed quiet since (the hourly check reads 0 `reclaim stamped NO ledger rows` lines in every 24 h window). Was: RESOLVED-IN-CODE 2026-08-26 (owner) — built exactly per the pre-verified sketch: `member_family_still_held` (the family clause with inverted resolution filter) + a shared `log_zero_stamp` verdict at both alarm sites: no resolved family row → the original loud line; partially resolved with held residue → the same grep-able prefix with a distinguishing '(partially-resolved file — held siblings remain, issue 289)' marker; fully resolved → silent (181/196/200 stay benign). Test `a_zero_stamp_under_a_partially_resolved_file_is_not_silenced` pins the stranded shape (old gate true = suppression proven, new probe true = marker fires — observed live on stderr) and the fully-resolved quiet case. Full gate green (66 suites). DEPLOYED (rev ae31cbd, /health green). RESOLVED-DEPLOYED. (2026-08-26, owner — adversarial reclaim review; not yet re-verified line-by-line by the owner)
Kind: observability (the monitoring signal this bug family is caught by)
Severity: LOW (no data corruption; degrades the standing OPERATE check)
Relates to: 139 (the address-miss class the tripwire exists to catch), 288 (same review)
Found by: the 2026-08-26 adversarial reclaim/quarantine review.

## Verify

    grep -c 'partially-resolved file' crates/store/src/lib.rs

- **done**: a positive count — the distinguishing marker exists, so a per-record stranding under a partially-resolved file prints the loud line with its marker instead of being silenced (the served rev is this tree)
- **open**: `0` — the marker is gone; a stranding inside a partially-resolved file is silent again

## The gap

The zero-stamp alarm (`[store] reclaim stamped NO ledger rows`) is gated by
`member_file_resolved` (lib.rs ~1904-1928), which returns true when ANY sibling
row of the member file (`file`, `container`, or `file#%`) already carries
`reprocessed_at` OR `skipped_at`. In a multi-record file the FIRST resolved
record permanently satisfies the predicate, so a LATER record whose reclaim
legitimately stamps zero rows — a genuine issue-139-style address miss
(member_path/content_hash drift) — is silently not logged. The OPERATE check
greps for this exact line every firing; the suppression makes a real stranding
invisible to it.

## Scenario

File F with records F#1..F#1000. F#1 reclaims (file row resolved). F#742's held
row was written under an address no stamp now matches; its reclaim stamps zero
rows — a real stranding — but `member_file_resolved(F)` is true (F#1), so no
line is logged.

## Fix direction

Narrow the gate from file-level to row-level: suppress the alarm only when the
RECORD's own row (its `member_path#ordinal`, or the exact address the stamp
targeted) is already resolved — not when any sibling is. Or log at a lower
severity ("zero-stamp under a resolved file") so the signal survives without
re-introducing the noise the gate was built to remove. Verify the gate's
original purpose (what noise it suppressed) before choosing; then pin with a
test: a zero-stamp reclaim for an unresolved record inside a partially-resolved
file MUST log.
## Pre-verification (2026-08-26, owner — ultracode loop, adversarial agent + owner read)

CONFIRMED at both alarm sites (lib.rs:1799-1808 parsed arm, :1876-1883 fresh-record
arm; gate fn `member_file_resolved` :1904-1929). The suppression is FAMILY-level and
permanent: the gate's query returns true if ANY of (file row, container row, any
`#<ordinal>` sibling) carries either outcome stamp, and resolved rows are terminal —
so the first resolved record blinds the alarm for every later record of that file,
forever. `stamp_reclaimed` discards per-address counts (sums into one u64), so the
sites cannot currently tell WHICH addresses matched.

**Key constraint for the fix:** checking only the attempted addresses would regress
issue 200 (post-segmentation ordinals resolve at addresses the reclaim never
attempts — the reason the `LIKE 'file#%'` clause exists, 106 false alarms). The
property that separates a genuine stranding (this issue) from the three documented
benign shapes (181 COR file row / 196 container / 200 shifted ordinals): **a
stranding leaves an UNRESOLVED family row behind; the benign shapes leave none.**

**Fix sketch (next unit):** add `member_family_still_held` (same family clause as
`member_file_resolved`, inverted filter: both stamps NULL), and split the guard
three ways at both sites: unresolved family → the existing loud line; resolved
family BUT held residue remains → the same `reclaim stamped NO ledger` prefix (so
the OPERATE grep still catches it) with a distinguishing `(partially-resolved
file — held siblings remain, issue 289)` marker; fully resolved, nothing held →
silent (the benign shapes). The marker keeps the irreducible ambiguity honest: a
sibling legitimately awaiting its own reclaim can make a benign zero look stranded.
Red-first test: in-module tests (lib.rs test mod — helpers `seed_fetch`/
`tiny_parsed`/`held_notice`; model tests at :6855/:6908/:6969), asserting the gate
predicates directly (the alarm is an eprintln). Companion assertion on the COR-shape
test pins that 181/196/200 stay silent.
