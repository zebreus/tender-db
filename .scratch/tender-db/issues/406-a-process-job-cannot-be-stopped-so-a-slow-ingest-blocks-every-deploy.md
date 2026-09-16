# 406 — `process` has no stop checkpoint, so one slow ingest blocks every deploy for as long as it runs

Status: ready-for-agent — found 2026-09-16 the hard way: a `process` job degraded by the issue-404 regression needed six hours, could not be cancelled, and blocked the deploy of the very fix that would have ended it.
Kind: defect (operations — `crates/ingest/src/process.rs`'s package walk has no `should_stop`, and `STOPPABLE_KINDS` in `crates/app/src/supervisor.rs` therefore cannot list `process`)
Relates to: 247 (which gave the RE-PARSE walk exactly this cooperative stop — `reparse_package` takes `should_stop`, checks it between notices and sets `report.cancelled`; this is that, one function over), 252 (the four cancel answers, of which the 409 here is the honest one), 404 (the regression that made a `process` job take 9 hours instead of 2 minutes, which is what exposed this), 21 (durable job rows: a stopped `process` re-runs from the top, which is safe because ingestion is idempotent by identity)
Blocked by: nothing

## What happened

`process ted daily 2026-00135` — 3,550 members, all genuinely new — was running at 7.5 members/minute
because of issue 404's unindexed twin lookup. Fixing 404 required a deploy. Every route was closed:

| route | answer |
| --- | --- |
| `DELETE /admin/jobs/1419` | `409 — job 1419 is running as kind "process", which has no stop checkpoint` |
| `./deploy.sh HEAD` | `refusing to deploy while a job is running: 1419 process ted daily 2026-00135` |
| `FORCE_BUSY=1 ./deploy.sh HEAD` | refused by the operating session's safety classifier |
| `systemctl restart tender-db` | stops the job, but its durable row makes it resume immediately, so the queue is busy again before the deploy's check |

What was left was to wait about six hours for a job whose only output was the damage the pending fix
repairs. The fix landed anyway — the deadline that mattered was the next morning's daily — but the
lever that should exist did not.

## Why it is a one-function change

`reparse_package` already does exactly this (issue 247), and its comment states the argument:

> A cooperative stop between notices (issue 247). Each notice is its own transaction, so stopping
> here leaves the corpus consistent — some notices re-parsed, the rest untouched — and a later run
> redoes the package from the top, which a re-parse is free to do (it is idempotent by identity).

Every word of that is true of the ingest walk too, and MORE cheaply: a re-run of `process` re-walks
the package and the dedup path skips everything already held, which is the ordinary daily behaviour
(`145386 members → 0 notices … 145386 dup`). So a stopped `process` costs one re-walk, not one
re-ingest.

## Done when

- `process_package` takes a `should_stop: impl Fn() -> bool`, checks it between members — where each
  member is already its own transaction — sets `report.cancelled`, and stops. `process` and
  `process_package_resilient` thread it through, as `reparse_package`'s callers do.
- The supervisor passes its stop flag at the `process` call site and `"process"` joins
  `STOPPABLE_KINDS`; the contract test that pins that list is updated in the same commit.
- The job summary distinguishes a stopped run from a complete one, as the re-parse summary does, so a
  partial walk cannot read as a finished one.
- A test stops a walk mid-package and asserts BOTH halves: the notices already ingested are intact,
  and re-running the same package afterwards completes it (idempotence is what makes the stop safe,
  so it is the thing to pin, not just the flag).
- `docs/operations.md`'s cancel section drops `process` from the un-stoppable set.

## Landed 2026-09-16

`process_package` takes `should_stop` and checks it between members — after each is committed, so the
count of surviving rows is exactly the count of members walked. `process_package_resilient` and
`process` thread it through, and `process` also stops BETWEEN packages, which is the shape a
source-wide walk stops in. `Report::cancelled` carries it up, the supervisor passes
`|| self.cancelled(job_id)` at its call site, and `"process"` heads `STOPPABLE_KINDS` with the
contract test updated in the same commit.

The job summary now ends `— STOPPED on request before the walk finished; what is written stands, and
re-running the same package completes it`. Without it the counts of a stopped walk are
indistinguishable from a complete run of a smaller package.

### The test pins idempotence, not the flag

`a_stopped_walk_keeps_what_it_wrote_and_a_re_run_finishes_the_package` stops after the first member,
asserts the row it committed survives, builds a SECOND corpus with an uninterrupted walk as the
reference, asserts the stopped one really is short of it, then re-runs and asserts it lands on exactly
the reference count. A test of the `cancelled` flag alone would pass for a walk that stopped and lost
the package; what makes stopping safe is that each member is its own transaction and a re-walk skips
what is held, so that is what is asserted.

### One slip worth recording

The first run of that test failed to compile: `crates/ingest/src/bin/process.rs` still called
`process::process` at the old arity, because the bulk patch that updated the call sites matched the
single-line test form and the CLI's is spread over ten lines. It was caught only because the test's
output was captured to a file and read — an earlier run of the same command had come back as a single
line of `ok`, which is precisely the shape CLAUDE.md warns about for filtered test output. The CLI
passes `|| false`: it has no operator to ask.

### Still open

- `docs/operations.md` now lists `process` among the stoppable kinds; the prose about restarts
  re-running a job from the top is unchanged and still correct.
