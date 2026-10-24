# 592 — a reparse counts notices the current parser now quarantines but never names them (577 slice 2: 8 de-2.x notices)

Status: ready-for-agent — filed 2026-10-24 ~20:50Z (hourly audit) from 577 slice 2's reparse 2776: "re-parsed 255735 notices
across 114 packages (582195 members walked, 0 unmatched, 0 re-keyed, **8 now failing and left untouched**)". The job's own
comment (`supervisor.rs` ≈:19531) says a non-zero `now_failing` "means a parser regression, not progress". Nothing records
WHICH 8: `ingest::process::reparse_package` (`process.rs` ≈:719) only bumps `report.now_failing` on a
`Parse::Quarantined` and drops the reason; the journal carries no line for them. Their stored layer was left alone (the
conservative rule), so nothing was lost; but their parse is now frozen at whatever parser first stored them, and the next
parser regression of the same kind would be just as silent.
Next: unit 1 BUILD (small, rides the next code deploy) — in `reparse_package`, keep up to N (e.g. 50) `(fetch_id,
member_path, publication_id if known, quarantine reason)` per job and print them in the job summary and one journal line
each; then re-run a reparse over the de-2.x packages that hold them (or the whole de-2.x pair again, cheap once 591 ships)
and decide per reason: fix the parser, or accept a quarantine with the reason recorded.
Kind: observability / parser regression (8 of 255,735 de-2.x notices; no served change)
Relates to: 577 (unit 3 slices: slice 1 de-1.x had 0 now failing, slice 2 de-2.x 8), 591 (reparse delete), 244 / 272 (the
reparse job's honest summary), 247 (per-statement reparse costs).

## Verify

    ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=50"' | python3 -c "import sys,json;[print(r['job_id'],r['counts'][:400]) for r in json.load(sys.stdin)['recent'] if r['kind']=='reparse']"

- **open (2026-10-24)**: 2776 reads `8 now failing and left untouched` with no member named anywhere.
- **done**: a reparse summary names each now-failing member with its reason, and each of the 8 is either parsed again or
  carries a recorded, accepted reason.
