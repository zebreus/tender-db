# 52 — Server RSS grows monotonically across the backfill (possible leak)

Status: DORMANT (verified 2026-08-16, owner sweep). The backfill is done; steady-state RSS today is 0.6 GB (measured /proc, after a day that included a 7.9M-row backfill job and three index builds) — no leak in steady state. The growth pattern only ever manifested during multi-day bulk loads, so this bites again only if issue 28's full reprocess is scheduled; re-open then and profile first (issue 19's lesson).
Severity: LOW (no OOM risk for job 1; investigate)

Observed during the 2026-07-21 backfill (rev 62f7255): the server RSS
climbs roughly linearly with packages processed — ~300 MB early
(2013-05) → 797 MB by pkg 38/158 (2016-06), ~20 MB/package. The
supervisor processes one package at a time and each package's working
state should be released after it commits, so monotonic growth suggests
something is retained: candidates — a per-package accumulator not
cleared, the reader-pool / prepared-statement caches, turso page cache
(may be benign/bounded), or the mention/org maps in the walker.

Not urgent: extrapolated to the ~120 remaining packages that's ~+2.4 GB
→ ~3.2 GB on an 8 GB box — no OOM for job 1. The projection (job 5) is
the memory-sensitive phase (issue 19 chunks it); confirm RSS there too.

Task: profile where the retained memory goes (a heap snapshot across a
few package boundaries, or bisect by disabling caches), confirm whether
it plateaus or is unbounded, and fix if it's a real retention bug. If it
turns out to be bounded turso page cache, document that and close.

Acceptance: RSS growth explained — either bounded/benign (documented) or
a retention bug fixed so RSS plateaus across the backfill.

## Verify

    ssh -o BatchMode=yes root@zebreus.click 'grep -E "VmRSS|VmHWM" /proc/$(systemctl show -p MainPID --value tender-db)/status | tr -s " " | paste -sd " "; ps -o etimes= -p $(systemctl show -p MainPID --value tender-db)'

- **done** (dormant holds): resident set a few GB at most between the weekly chains and the peak (`VmHWM`) not climbing week over week — read 2026-09-26: `VmRSS: 4050972 kB VmHWM: 25203364 kB`, 583,141 s up (one weekly data-quality + org-merge chain in the window); 09-19 read `VmRSS: 525632 kB` two hours after a restart
- **open** (reopen): RSS climbing monotonically across a multi-day single job, the 2026-07 backfill shape — or the `VmHWM` peak growing from one weekly chain to the next on the same corpus, which would say a job holds more each week rather than the same working set

## 2026-09-26 08:2x — a week of uptime: 4 GB resident, 25 GB peak, box at 62 GB

Read by the owner sweep after seven days without a restart (the 09-19 deploy of `135a469` was the last):
`VmRSS 4,050,972 kB`, `VmHWM 25,203,364 kB`, `VmSwap 0`, 77 threads; `free -g` says 62 GB total, 49 GB
available, 42 GB in page cache. The 09-19 reading of 0.5 GB was two hours after a restart; this one sits
behind the Sunday chain (data-quality 6068 s, then build-org-match-keys over 6.6M rows, org-merge-health,
scan-org-match-keys over 4.2M keys), which is where the 25 GB peak almost certainly comes from — those
jobs hold whole-key maps in memory by design (issue 300's stage 4), and the queue runs them one at a
time. The 4 GB that stays is the allocator keeping arenas after the peak plus turso's page cache, not
the monotonic climb-inside-one-job this issue was filed on, so DORMANT holds. Two numbers to keep
reading, now both in the Verify line: if `VmHWM` is higher next Sunday on the same corpus, a job holds
more each week; if `VmRSS` keeps stepping up between chains, the arenas are not being reused. Neither
needs anything today on a box with 49 GB to spare. Not filing a separate issue — the weekly sweep of
this Verify block is the watch.
