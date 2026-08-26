# 289 — the "reclaim stamped NO ledger rows" tripwire is suppressed for any per-record stranding inside a partially-resolved member file

Status: DIAGNOSED (2026-08-26, owner — adversarial reclaim review; not yet re-verified line-by-line by the owner)
Kind: observability (the monitoring signal this bug family is caught by)
Severity: LOW (no data corruption; degrades the standing OPERATE check)
Relates to: 139 (the address-miss class the tripwire exists to catch), 288 (same review)
Found by: the 2026-08-26 adversarial reclaim/quarantine review.

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
