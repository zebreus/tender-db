# 200 — missing-publication-id: 108 old text-era rows + 2 new eForms-adjacent rows

Status: in-progress (fix deployed, reclaim running)
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
