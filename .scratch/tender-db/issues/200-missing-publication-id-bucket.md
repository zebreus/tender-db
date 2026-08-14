# 200 — missing-publication-id: 108 old text-era rows + 2 new eForms-adjacent rows

Status: RESOLVED (2026-08-14) — bucket emptied: 108 reclaimed, 1 re-held under its true reason, 0 remain
Kind: quarantine bucket diagnosis
Relates to: 195 (the 2 new rows surfaced in its reclaims via the issue-87 rewrite)

## What (measured via /v1/sql, 2026-08-14)

110 outstanding `missing-publication-id` rows: 108 profile `text`, detail NULL (the old
bucket — text-era records without an ND line, never diagnosed) and 2 that re-held under
this reason during the 195 reclaims (their members re-parse to records lacking a
publication id — need member extraction to see whether the id is genuinely absent in the
source or lost by the parser). No ledger entry names this population yet — add one
(resolved or outstanding) once diagnosed.

## Next

Group the 108 by member/package vintage; extract the 2 new members (query: reason =
missing-publication-id, attempts > 0).

**2026-08-14 ~09:2x CEST box time (orchestrator) — DIAGNOSED + fix deployed (rev ac484f5),
reclaim running.** The whole bucket is ONE mechanism: 2010-era structured-form notices
quote the buyer's file reference under "IV.3.1) reference number" — "3.131/2010",
"331.08/01" — alone on an INDENTED line, which matched the <digits>.<digits>/<digits>
record-marker shape (the check trimmed leading whitespace). Each such line split a real
notice mid-body; the ND-less tail held as missing-publication-id. Measured on the full
2010-03-23 EN daily: 1,488/1,488 real markers at column 0, the only 2 indented matches
both false; every era fixture agrees. Fix: the marker anchors at column 0
(profile.rs is_record_marker, trim_ascii → trim_ascii_end) + unit and segmentation tests.
Reprocess reason=missing-publication-id enqueued — the merged fuller records enter as
versions of their publication ids; ordinals after each merge shift down one, so most old
#N rows resolve via the by_member address coincidence; any stranded tail-position rows
stay visible in this bucket for a follow-up sweep. Ledger entry after the drain verifies.

**2026-08-14 ~09:5x CEST box time (orchestrator) — RESOLVED.** Reprocess (rev ac484f5): 15
packages, 108 reclaimed (merged fuller records entered as versions), 123,569 already-parsed
dedups, 1 member re-held under unclaimed-content (an in-record orphan line — issue 199's
family, tracked there). The missing-publication-id reason is now EMPTY — verified via the
outstanding-by-reason readout. Ledger entry "2010 records split by quoted reference
numbers" added.

**2026-08-14 ~10:1x CEST box time (orchestrator) — rider: the drain fired 106 benign
fresh-record alarms** (old rows resolved under coincidental ordinals; every row terminal,
bucket verified 0). member_file_resolved now reads a resolved record-SIBLING of the same
file as benign-zero evidence (store commit "resolved record siblings are a benign
zero-stamp"); a file with no resolved row anywhere still alarms. Deploys with the pending
ledger entry once the queue idles (mpi trailing fold in the issue-192 slow-plan phase with
the daily queued behind it).
