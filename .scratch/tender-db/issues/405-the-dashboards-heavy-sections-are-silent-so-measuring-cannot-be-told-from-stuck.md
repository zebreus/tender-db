# 405 — the dashboard's heavy sections log nothing on success, so "Measuring…" cannot be told from "stuck"

Status: ready-for-agent — unit 1 (the success-path timing line) LANDED 2026-09-16, see the foot; the two skip paths and the served age are still open. Found 2026-09-16 while trying to read issue 400/401's live acceptance off the deployed dashboard and finding the panel they changed simply absent, with nothing anywhere saying whether that was normal.
Kind: defect (observability — `crates/app/src/coverage.rs`, the background refresher's `publish`/`refresh_into`)
Relates to: 37 (the sectioned `None` default — "measuring…" rather than a false `0`, which is the right rendering and is exactly what makes the silence ambiguous), 61 (the coverage regression that moved the refresher onto its own thread and added the change-gate), 53 / 42 (why the heavy sections are gated behind `heavy_write_active` at all), 400 and 401 (whose acceptance this blocked for ~15 minutes)
Blocked by: nothing

## Observed

After the `c40cccc` deploy restarted the service at ~11:44Z, `/api/dashboard` served
`"coverage": null` and `"pipeline": null` while `counts`, `quarantine` and `award_linkage` were all
populated. `journalctl -u tender-db` carried **no `coverage:` line at all** for the whole period.

There was no way, from outside or from the box, to distinguish:

- the heavy `notices` GROUP BY still running its first pass (which is what it was), from
- the refresher thread wedged on a pinned reader, from
- the change-gate wrongly deciding nothing had been written, from
- `TENDER_DISABLE_COVERAGE` set by someone.

Measured by polling: coverage appeared between 11:57 and 11:58Z, so the first heavy pass after a
restart takes on the order of **13 minutes**. That is a perfectly reasonable duration for the scan.
The defect is that the number is not knowable to anyone.

## Why it is silent

`publish` (`coverage.rs:259`) logs on the `Err` arm only:

    Ok(v) => set(&mut cell.write().expect("coverage snapshot"), v),
    Err(e) => eprintln!("coverage: {section} refresh failed, keeping last value: {e}"),

So the entire success path — every 60 s, every section, forever — is mute. The only other `eprintln!`
in the module is the `TENDER_DISABLE_COVERAGE` notice. A section that has never succeeded and a
section that succeeds every minute produce byte-identical logs: none.

`coverage` is also published LAST of the four heavy sections, which is why it is reliably the one
missing: a reader sees three populated panels and one "measuring…" and has no reason to think that is
ordering rather than failure.

## Why it matters

It cost a live acceptance today: issues 400 and 401 both change the coverage/pipeline panels, and the
panels were blank for the quarter-hour after their deploy with nothing to consult. The honest options
at that moment were "wait and hope" or "file a defect against a system that was working" — and the
second nearly happened.

More generally this is the standing shape of an alarm nobody can calibrate: the first time the
refresher really does wedge, the symptom will be the one that has been ordinary for months.

## Done when

- Each heavy section logs its duration on SUCCESS, once per measurement, in the style the rest of the
  codebase already uses (`[data-quality] window 14/35 …: 64.1s for 16 queries`).
- The skipped paths say so at least once rather than silently returning: `heavy_write_active` and the
  change-gate are both correct behaviour and both indistinguishable from a hang. A rate-limited line
  (first skip after a measurement, not every 60 s) is enough.
- Ideally the age is SERVED, not just logged: a `measuring since <age>` on the section would let the
  UI say "measuring for 12 min" instead of "measuring…", which is the form a reader actually needs.
  That is a model change (`Dashboard`), so it is a second unit.


## Unit 1 landed 2026-09-16 — the success path says how long it took

`timed(section, measure)` wraps each of the four heavy measurements and prints
`coverage: <section> measured in <n>s`, on the error path too (`(failed)` suffixed) because how long
something took to fail is the difference between a timeout and a refusal, and `publish` says only
that it failed. The style matches `[data-quality] window 14/35 …: 64.1s for 16 queries`, which is the
line that made the data-quality job's 88 minutes legible this morning.

Not tested, deliberately: it is an `eprintln!` with no observable surface, like every other timing
line in this codebase. The testable version is the third bullet below — serving the age — and that is
where a test belongs.

### Still open

- **The two skip paths.** `heavy_write_active` and the change-gate both `return` silently, and both
  are correct behaviour indistinguishable from a hang. A rate-limited line (announce on the
  TRANSITION into skipping, not every 60 s) needs a small piece of state threaded through
  `refresh_into`, which has eleven call sites — mechanical, but it is its own commit.
- **Serve the age.** `measuring since <age>` on the section so the UI can render "measuring for
  12 min" instead of "measuring…". That is a `Dashboard` model change and the unit that actually
  fixes the reader's problem; the log line only fixes the operator's.

### Live, and it corrected the issue's own arithmetic — 2026-09-16, rev `acf5103`

The first sequence after the deploy:

    14:12:12  coverage: quarantine    measured in  18.6s
    14:12:49  coverage: award-linkage measured in  37.2s
    14:15:52  coverage: counts        measured in 182.6s
    14:15:59  coverage: coverage      measured in   7.4s

Two corrections to what is written above, both from the instrument's first output:

- **The pass is ~4 minutes, not ~13.** The 13 minutes in the "Observed" section is wall-clock from
  restart to the panel appearing, measured by polling from outside; it includes service startup and a
  box that had just finished an 88-minute data-quality run. The sum of the four measurements here is
  246 s.
- **`counts` dominates, not `coverage`.** `refresh_into`'s own gate comment calls
  `coverage`/`pipeline` "the one full `notices` `GROUP BY`, the heaviest read". On this pass it was
  the CHEAPEST of the four at 7.4 s, against 182.6 s for `counts`. Caveat, stated rather than
  glossed: the four run in sequence, so `coverage` reads `notices` right after `counts` has walked
  it, and a cold first pass may divide differently.

That second point is a standing claim in the code that the measurement does not support, and it is
now checkable across restarts instead of being settled by a comment. If it holds up over a few cold
starts, the gate comment should be rewritten and the ORDER reconsidered — publishing the 7-second
section before the 3-minute one would put the panel a reader is waiting for on screen first.

Unit 1 is done. The two skip paths and the served age remain open above.
