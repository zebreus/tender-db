# 373 — jobwatch applied a 26 h lookback to a window that reached back 18 h

Status: RESOLVED — fixed and installed 2026-09-09 (`2d38800`). Found during the ownership
check-in, while trying to confirm that the daily pipeline had run and noticing that
`/admin/jobs` recent list held nothing but this session's own refold jobs.

Kind: observability / operations (the watchdog reported "ok" about a window, not about the day)
Blocked by: —
Relates to: 224 (the same two watchdogs, reconstructed there — see "How it got in"), 313 (which
added the `?limit=` parameter this script needed and never used)

## The defect

`tender-db-jobwatch.sh` calls `GET /admin/jobs` with no `limit`, so the server returns its
default — the newest **20** runs. The script then filters those for `outcome != "ok"` within
`TENDER_JOB_FAIL_LOOKBACK_SECS`, default **26 h**. Those are not the same window, and on a busy
box the lookback is the larger of the two by a wide margin.

Measured on prod 2026-09-09:

| what | value |
| --- | --- |
| jobwatch's failure lookback | 26.0 h |
| how far back the default 20 entries actually reached | **17.7 h** |
| blind band | **8.3 h** |
| span of the same log at `?limit=200` | 134.1 h |
| share of the 20-entry window consumed by ONE session's refold jobs | ~40 % |

A daily that failed inside that band would leave `systemctl --failed` clean and the journal
saying `ok jobwatch: … 20 recent, last project → ok`. The watchdog exists for exactly that
event.

**Not currently hiding anything.** The only two non-ok runs in the deep window are 70.6 h and
107.3 h old (`scan-org-match-keys` plan-staleness refusal, `fold-provisional-echoes` p0 parity
abort) — both guard rails firing correctly days ago and handled at the time, both outside the
26 h lookback anyway. So this was a live exposure, not a live miss.

## How it got in

Issue 224 reconstructed both watchdog scripts after they vanished from the box, with behaviour
"inferred from the unit descriptions + `model::ingestion` job shapes" — there was no original to
copy. A reconstruction naturally writes the plain endpoint. Issue 313 had *already* added
`?limit=` precisely because "the depth used to be a hard-coded 20 that silently ignored this
parameter, which hid a day of history during an incident hunt" — the same failure mode, in the
same endpoint, fixed on the server side while the one client whose whole purpose is reading that
log kept asking for the default.

## The fix

1. **Ask for the depth it needs.** `?limit=$depth`, `TENDER_JOB_LOG_DEPTH` default 200 (the
   supervisor's `JOB_LOG_MAX`; ~5 days at the observed rate).
2. **Make saturation loud, not silent.** Depth alone just moves the threshold. So when the log
   comes back FULL *and* its oldest entry is younger than the lookback start, an older failure
   is provably invisible — the script now warns and exits non-zero rather than printing `ok`.
3. **State the span in the ok line**, so the number is legible without re-deriving it:
   `ok jobwatch: idle, 0 queued, 200 recent covering 134h, last project → ok`.

## Verification

Ran the modified script against the live endpoint before installing:

- depth 200 (new default) → `200 recent covering 134h`, exit **0**.
- forced depth 5 → `WARN … 5 entries reach back only 1h but the lookback is 26h`, exit **1**.
- forced depth 20 (the OLD behaviour) → `WARN … 20 entries reach back only 18h`, exit **1** —
  i.e. the new guard fires on exactly the silent condition this box was already in.

Installed to `/usr/local/bin` and dry-fired: green, with the wider coverage in the journal.

## Left undone

`ops/watchdogs/` has no test harness — these scripts are verified by running them against the
box. A fake `/admin/jobs` fixture plus a few assertions would let the saturation guard and the
lookback arithmetic be checked without prod, and would have caught this class of bug. Worth a
unit if the watchdogs grow again; not built now because the fix is three lines of curl and jq
and the box is the only real environment.
