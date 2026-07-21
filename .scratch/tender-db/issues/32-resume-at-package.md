# 32 — Restarted process jobs resume at the last incomplete package

Status: ready-for-agent

A restarted `process (all)` job re-walks every package from 1993 to find
where it left off. Identity-dedup makes that correct but expensive: the
2026-07-21 bad8dda restart spent ~90 min skip-scanning the 2004-2010 era
(huge bundles) at 0 notices written — user-visible as "stuck at 0.0/s",
and every deploy during a long backfill pays it again.

Fix: the durable queue (issue 21) already persists the job row across
restarts — extend it with a progress cursor (last fully-completed
package period, updated as each package finishes, same write
transaction as the package's job-progress bump). recover() re-enqueues
the job with the cursor; the walker starts from the first package AFTER
it. Correctness: a package is only recorded complete after its last
member committed, so resuming after it never skips anything; the
partially-done package re-runs and dedups (seconds, not hours).
`rebuild`-style full re-walks stay available by enqueuing without a
cursor (a fresh enqueue never inherits one).

Acceptance: kill -9 mid-backfill at package N, restart → job resumes at
N (not package 1), first status line shows the resumed position;
existing recovery tests still green.
