# Instruments, gates and watchdogs

How to build a check you can believe. This is the team rule, adopted 2026-08-04, after a single day
produced **eight instrument bugs and every one of them failed permissive**. Companion to
[`prod-box-reads.md`](prod-box-reads.md), which governs *what* you may read; this governs *whether the
thing reading it can be trusted*.

## The asymmetry, which is the whole reason this file exists

**A restrictive bug announces itself. A permissive bug is indistinguishable from success.**

If a check wrongly refuses, you find out in minutes — you are blocked, so you investigate. If a check
wrongly permits, nothing happens. The run proceeds, the number looks plausible, the guard reports clear,
and the defect survives until somebody points an *independent* instrument at the same fact.

That is not a bias in the bugs. It is a bias in the *discovery*: permissive failures are the ones that
do not generate the evidence of their own existence. So they accumulate, and they accumulate silently,
and a day's worth of them all point the same way.

**Worse: a permissive bug tends to agree with what you expected.** Three of the eight below produced
evidence that *supported the hypothesis under investigation* — a sampler whose numbers climbed during a
study of unbounded duration, an occupancy count inflated in the direction of the conclusion. **Agreement
is where nobody looks.**

## The ledger — 2026-08-04, all eight permissive

The pattern is only believable with the instances.

| # | instrument | the bug | what it reported when broken |
| - | ---------- | ------- | ---------------------------- |
| 1 | `thread_cpu.sh` sampler | associative subscript in arithmetic context → cumulative, not delta | numbers **climbed** — looked exactly like a query getting slower, i.e. it corroborated the thing being investigated |
| 2 | Tier A precondition | aggregate CPU threshold | **passed while a Class B scan ran** — an abandoned read sits at ~24% of a core, under the threshold |
| 3 | `thread_cpu.sh` occupancy | keyed by thread *name*, not tid | duration **inflated** by the number of same-named busy threads (up to 4× for `slow-read-exec`) — again toward the conclusion |
| 4 | CPU-delta precondition | bash `$12` is `$1` followed by `2`, not field 12 | every delta **0** → `busy=0` always → "box is clear", unconditionally |
| 5 | probe interlock | `is-active` on a `--service-type=oneshot` unit, which sits in `activating` for its whole `ExecStart` | **blind for the entire run it existed to cover** — and the same form gated a sampling loop, so the loop body never ran *and the abort inside it never armed* |
| 6 | `pid_busy_pct` | returned `0` on four distinct measurement failures | `0` means idle means **go** |
| 7 | restart watcher | instantaneous `D\|R` run-state | read **clear five seconds before the query stopped**; a thread momentarily in `S` between I/O waits reads idle |
| 8 | run harness | `timeout N ssh host cmd` | bounded the **client**, not the work — a remote `sqlite3` ran 90 minutes after its parent died |

Not one refused when it should have.

## The audit question

Before trusting any check, ask: **"what does this report when it is broken?"**

If the answer is *go / clear / healthy / 0*, it is dangerous **however correct it is when it works**.
Number 6 is the purest case: a function written specifically to eliminate permissive failures, which
returned `0` — idle, i.e. proceed — on every failure to measure. The bug was not in the logic. **It was
in the default, and a default is exactly what nobody re-reads.**

## A check whose answer arrives anyway can be dead and still look fine

This is *why* a permissive check goes unnoticed, and it is not covered by any of the above.

A waiter loop was written to block until a background job finished. Its pattern matched its own command
line, so it exited instantly, every time — it never once did its job. **Roughly twenty-five of them ran
before anybody noticed, and the reason nobody noticed is that the harness's own completion
notifications supplied every answer independently.** The check was dead. The workflow was fine. **Dead
and alive were indistinguishable, because the answer arrived regardless.**

The shape recurs everywhere: a row-count gate that stays green because the counts happen to be right
for unrelated reasons; a test whose subject is served by a component the deploy would have exercised
anyway. In every case the tell is not that the check is *wrong* — it is that **the answer arrives
whether or not the check works.**

**The test:** *if this check broke completely, would anything look different?* If the answer is no, it is
**decorative** until you cut every other source of its answer and confirm it can still produce **both**
a pass and a fail on its own.

**A gate that cannot be made to fail in isolation is not a gate.** Note what this asks that
[`prod-box-reads.md`](prod-box-reads.md)'s rule does not: nobody here estimated anything, and no
judgement was misapplied. The check was simply redundant with a channel nobody thought of as a channel.

## "Measure the artifact" has grades

Not a binary. Each step is closer to the thing, and a check can be one grade closer and still not close
enough:

    load average          a correlate of "are the slots busy"      — wrong for ~45 min on 2026-08-04
    instantaneous state   a correlate of "is it still working"     — wrong by 5 s, and a decision hung on it
    CPU delta over an interval / repeated occupancy                — the artifact

Number 7 was *not* a careless choice — run-state is a real improvement on load average. It was still a
correlate, and a restart authorisation was premise-bound on it. **Ask which grade you are on, not whether
you are "using /proc".**

## Self-test every predicate both ways

*(owner: sdk-vendor, whose `activity.sh` is the shared primitive)*

**A predicate only ever observed saying "no" is indistinguishable from one hard-wired to say "no".** So
exercise both arms and keep the evidence: the positive case *and* the negative, and for a guard, the
refusal as well as the pass.

This applies hardest to the things you least want to fire. An abort that has never aborted is not a
proven abort — fire it deliberately against a throwaway target before it guards anything real. Two of
today's guards were adopted on the strength of never having complained.

**Both-ways is per-axis.** Exercising a check on a pass case and a fail case proves discrimination
*only along the axis the fixture varies*. A confound on an axis the fixture holds **constant** sails
through clean, because the check was never asked about it: a query exercised both ways on
*introduced-vs-not* still false-positived on multi-version carry-forward, because every fixture row was
single-version. So the question is not "did I test both ways" but **"which axis does my fixture hold
constant?"** — that is where the untested confound is.

## A guard nested inside the loop it guards has no independent existence

Number 5 is the one to remember, because it is qualitatively worse than the rest: the others
*misreported*, this one **disarmed a safety mechanism**. The sampling loop and the abort shared a single
condition, so when the condition was wrong the guard did not fail — **it evaporated**, silently taking
the abort with it, and a 455 GB scan ran on prod with no watchdog.

**Put the abort outside the loop it watches, and preferably outside the process.** An external watchdog
that reads the target's state and can stop it is structurally unable to disappear along with the thing
it guards.

## Fail loud, fail closed

* **Loud:** a check that cannot measure must say so — a distinct `ERROR` arm, never folded into "clear".
  *"df returned nothing; this watch did NOT run"* is worth more than a silent pass.
* **Closed:** when the harness is uncertain, refuse rather than proceed. A firing test with a
  fires-on-anything threshold must assert its target exists *before* arming, or a mistyped unit turns a
  possible misfire into a guaranteed one.
* **Prove liveness.** A watchdog that prints only on breach is silent when healthy *and* silent when
  dead, and those must not look alike. Emit a heartbeat carrying the live reading, so absence of output
  means **gone**.

## Select by naming, not by matching

Four of today's failures — `pkill -f`, `pgrep -f`, `is-active`, `--state=active` — **selected by
matching, and every one matched the wrong thing.** Twice, `pkill -f` matched the agent's own command
line and killed its shell mid-command.

For anything destructive this is not style, it is safety: `systemctl stop tdb-firetest-rd.service` names
one unit and has no expression to get wrong. Prove the absence of patterns mechanically
(`grep -nE "pkill|pgrep| -f |--all"`) rather than by eye — "there are no patterns here" is exactly the
kind of claim that reads true and isn't.

## Three operational rules

1. **Bound work where it runs.** `timeout N ssh host cmd` bounds the client; `ssh host 'timeout N cmd'`
   bounds the work. And verify the bound reaches the *grandchild* — killing a wrapper while its
   `sqlite3` child survives relocates the orphan one level down while looking fixed.
2. **Capture cost before a kill.** `cputime`, `/proc/PID/io`. A kill is a one-way door on the evidence:
   on 2026-08-04 an orphan's CPU and bytes-read became permanently unknowable the moment it was killed.
3. **Identity, not state, for "untouched".** `ActiveState=active` after an event cannot distinguish
   *never touched* from *stopped and restarted in the gap*. An unchanged `MainPID` can.

## Calibrate against a case where the instrument could disagree

**Every one of the eight was found by pointing something independent at the same fact. None was found by
reading the code.**

So calibrate against a case whose true answer you know by other means, and prefer one where a
*subtly* wrong tool would visibly diverge: a GIL-serialised spinner is a weaker test than real parallel
threads, precisely because the GIL makes the arithmetic tidy. Check the reading against reality before
trusting it — the interlock in number 5 was caught by asking "it says 0; is that true?" while a probe had
been running for 650 seconds.

Beware environments that make the correct and incorrect versions look **equally** broken: number 4 was
first tested under `zsh`, which does not word-split, so both forms returned empty and read as "my parsing
is wrong somewhere" rather than "the wrong form is silently permissive". **A test environment that cannot
distinguish them is worse than no test, because it produces a confident wrong diagnosis.**

## Related

* [`prod-box-reads.md`](prod-box-reads.md) — what may be read on the prod box, and why the rule is
  category-and-boundedness rather than size. Same reasoning: a rule that asks anyone to *estimate* is a
  rule that reintroduces the failure it prevents.
* Issue 28's **state, not event** requirement — a gate must assert the world is currently right, not that
  something once happened. That is this file's problem one layer up: how a *correct* instrument stops
  being true.
* Issue 120 — the ANSWER / PATH / COST split. Choose the instrument for the property that changed; this
  file is about whether the instrument you chose is honest.
