# 180 — the 6,341 unknown-token `unparsable-xml` rows have a stated hypothesis and no owner

Status: needs-triage
Kind: data-quality investigation (suspected-gap bucket)
Blocked by: —
Relates to: 73 (stated the hypothesis, then closed as duplicate of 36+41), 137 (measured the split), 30 (classification)

## Why

The dashboard's still-held `unparsable-xml` bucket is 8,400 (public `/api/dashboard`, 2026-08-10).
2,059 of it is the DTD population, fully attributed and owned (issues 84/139: 1,898 + 154 + 7).
The other 6,341 is exactly the two unknown-token details issue 137 measured:

| detail | outstanding |
|---|---|
| unknown token at 1:1 | 4,441 |
| unknown token at 1:3 | 1,900 |

Issue 73 characterized these as "likely genuinely non-XML/corrupt, leave quarantined" — but also
stated the falsifiable alternative: *"If 'unknown token at 1:1' is non-XML text-era files
mis-dispatched: fix the profile routing."* Issue 73 then resolved as a DUPLICATE of 36+41 (the DTD
story), so the hypothesis lives only inside a closed issue. A population this size classed
`SuspectedGap` on the dashboard needs an open owner, per the investigate-then-fix discipline that
covered the two big buckets (35/36).

## What

1. Sample both details from quarantine (bounded `/v1/sql`, member bytes from the archive tars —
   the issue-141/143/144 method) and establish what the payloads actually are.
2. If non-XML/corrupt by evidence: reclassify the reasons as benign-by-evidence (the reason must
   prove non-notice, issue 30's bar) or document them as a named keep, and give them a ledger row
   so the dashboard tells the story.
3. If mis-dispatched text-era files: fix the routing, reclaim, ledger row as usual.

The two details may have different answers (1:1 = starts with garbage; 1:3 = starts with a BOM or
short prefix?) — attribute them separately, not as one bucket.
