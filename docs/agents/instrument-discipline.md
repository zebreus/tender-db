# Instruments, gates and watchdogs

How to build a check you can believe. This is the team rule, adopted 2026-08-04, after a single day
produced **nine instrument bugs and every one of them failed permissive**. Companion to
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

**Worse: a permissive bug tends to agree with what you expected.** Three of the nine below produced
evidence that *supported the hypothesis under investigation* — a sampler whose numbers climbed during a
study of unbounded duration, an occupancy count inflated in the direction of the conclusion. **Agreement
is where nobody looks.**

## The ledger — 2026-08-04, all nine permissive

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
| 9 | probe self-test **precondition** | wrapper ignored an argument it did not recognise and ran the payload | executed a **455 GB unconfined scan on prod for ~13 min** — and preconditions run *before* `systemd-run`, so it had no `MemoryMax`, no `IOWeight`, no abort, no `RuntimeMaxSec` |

Not one refused when it should have.

**Number 9 is a different species and worth separating.** The first eight *misreported*. The ninth took
an **unrequested action**: the check meant to *precede* the protections ran *instead of* them, so every
guard bypassed itself through the thing that was supposed to gate it. A permissive misreport costs you a
wrong belief; a permissive *action* costs you the thing the guard was protecting.

Its fix is also the one worth copying, because it is not a check: `"") run;; *) refuse exit 2`. Reaching
the expensive path requires **saying nothing**, which is deliberate — never **saying anything**, which is
a typo. Adding a check that *catches* the bad argument would have been one more permissive-failure
candidate; making the bad argument **unrepresentable** removes the class. Prefer structure over a guard
wherever the structure exists.

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

## Declare the config, not just the input

The suite already insists a check must prove its **input** is the input it thinks it is —
snapshot path, size, mtime, age, and how it was resolved. It said nothing about the check's own
**settings**, and that gap cost a real bug.

`systemd-run` does not pass the environment through: only what is explicitly `--setenv`'d reaches
the unit. A probe forwarded two variables and not a third, so `GATE_LABEL` was accepted on the
command line, appeared to be in use, and **evaporated at the boundary** — the gate inside fell back
to a default and wrote another cadence's state file. No error, no warning; the variable was simply
absent. It was caught only because the verdict line reported `repeat=yes` under a label with no
prior state, where `unknown` is the only possible correct answer.

**The rule: an input a check merely BELIEVES it received is not an input it has VERIFIED.** That
applies to the settings exactly as it applies to the data, and the discipline had been applied to
one and not the other.

**The fix is not to forward more carefully.** The obvious repair — enumerate the variables and
`--setenv` each — is correct on the day it is written and rots immediately: the next variable added
silently fails to cross, the same defect one variable later. So instead, **print the configuration
actually in effect**:

```
-- config: label=demo-label state=/var/lib/…/demo.last max_age=30h fail_on_repeat=0
```

With the variable set, the line names it. With it dropped, the same line names the default. **The
divergence is on screen, not inferred from a surprising number three messages later.** This does not
prevent the drop; it makes the drop announce itself, which is the achievable goal.

The generalisation past config: wherever a value crosses a boundary — a process, a unit, an SSH
transport, a job queue — the receiving side should state what it got. A sender's belief about what
it sent is not evidence.

*(sdk-vendor, 2026-08-05, from the GATE_LABEL boundary bug. The rot argument — that an enumerated
forward carries its own expiry — is proj-fix's.)*

## A conclusion can outlive its premise

The hardest defect in this file to see, because **nothing about it looks wrong on re-reading.**
It was derived correctly, it was true when written, and it simply stopped applying — usually
because something changed several exchanges away, in a different part of the design, by someone
who had no reason to think of it.

The instance: `FAIL_ON_REPEAT=0` was recommended for the cheapest tier, because that tier was
going to run every five minutes against an input that refreshes daily — under which repeat
detection fires constantly and correctly means nothing. Sound. Then the live-detection job moved
into the app, the tier's cadence became once-per-snapshot, its cadence matched its input's, and
the recommendation became exactly wrong: a repeat check that never fires on the tier whose input
is freshest. **The advice did not change and did not need correcting; the world under it moved.**

**The tell is that there is no tell.** A wrong conclusion contradicts something. A stale one is
internally consistent, cites real reasoning, and survives review by anyone checking whether it
follows — because it does follow, from a premise that is no longer the case.

**The only thing that catches it is re-deriving rather than re-reading.** Ask what the advice
depends on, then check whether that is still true — not whether the advice still sounds right.
Guidance that states its premise out loud ("because this runs faster than its input changes")
is far cheaper to re-derive than guidance that states only its conclusion, which is an argument
for writing the *because* into every recommendation that will outlive the conversation.

*(proj-fix, 2026-08-05 — the fourth distinct failure mode this one design produced: wrong
premise, unenforced conjunct, an inference never noticed, and a valid conclusion whose premise
moved. They share no fix. The only thing that caught all four was someone re-deriving instead of
re-reading.)*

## Put the interpretation where the number is met

A caveat that reaches the reader *after* they have formed a judgement is not a caveat; it is an
excuse. Three times in one day the deciding factor was **where** an interpretation lived, not
whether it existed:

* the shortfall rule, in the **abort message** rather than the tracker;
* the resolution mode (`pinned` vs `newest`) and effective config, in the **verdict line** rather
  than a commit message;
* "this first red run is an inventory, not an incident", in the unit's **`Description=`** and first
  journal line rather than an issue filed afterwards.

Whoever opens a red verdict at 07:40 on day one forms their model of what the thing *is* in that
moment. *"The detector just inventoried a layer nobody had checked"* and *"the pipeline broke last
night"* produce very different reactions to identical output, and only one of them is available if
the framing lives somewhere the reader is not.

*(proj-fix, 2026-08-05.)*

## A classification can fail permissive too

Not only instruments. A **taxonomy** fails permissive when its frame is wrong, and it is harder to see,
because **every case still lands in some bucket — so nothing looks wrong.**

Issue 117 classified paginated reads by selectivity: *dense*, *matches-late*, *matches-nothing*. All
three are properties of where matches **start**, so the scheme silently assumed a filter keeps matching
once it begins. Nothing ever measured that. Prod's `?kind=registration` does not: 120 rows in ~8.1M, all
inside id deciles 1–2. It was not misclassified — it was **unclassifiable**, and no bucket existed for
"matches early, then stops". The resolution built on it ("both real values are dense, so only an
adversarial nothing-match is slow") was sound reasoning *inside a frame nobody was checking*, which is
why it survived review.

**The tell is not a case the scheme rejects — it is a case it cannot distinguish.** A taxonomy that
rejects an input is telling you something; one that accepts everything may only be telling you it has a
bucket for everything. So ask of a classification the same audit question this file asks of a check:
*what would it look like if the frame were wrong?* If the answer is "the same", the scheme is not
carrying evidence.

The generalisation: an assumption about the **corpus** wearing the clothes of a property of the
**schema**. "A two-value column has two dense values" is a claim about data; nothing in the schema said
it, and nothing re-checked it when the data moved. Same shape as the `notices_fetch_id` comment that was
true at 3.5M rows and still there at 27.4M.

*(sdk-vendor, 2026-08-05, from issue 117's correction — added to a doc authored by run-driver.)*

## Assert the transition, not the state, when absence is sometimes correct

The vacuity hole is *"0 violations is trivially true of an empty table"*. Its inverse is just as easy to
ship and gets the check disabled faster: **asserting a state that is legitimately absent under some
correct conditions.**

The instance: a detector for "the canonical layer was emptied" is obvious to write as
`EXISTS(SELECT 1 FROM tenders)`. But a fresh install has no tenders, and neither does a rebuild before its
first fold — both correct. An absolute assertion fires on both, and a check that cries wolf on known-good
states is switched off within a month, at which point it protects nothing.

The fix is to assert the **transition**: *this run must not have emptied a layer that was populated*,
compared against the state the run itself observed. Same for any "X must be present" check where X is
built rather than given.

Test for it the way you test for vacuity — ask what correct states the assertion would reject. If the
answer is "none I can think of", enumerate the lifecycle instead: first install, first build, restore,
migration. Absence is usually legitimate somewhere in there.

## State acceptance criteria as explicit conjunctions

An acceptance criterion written as prose cannot be audited clause by clause. Written as a conjunction,
each clause is independently checkable — and an **unenforced** one shows.

The instance: a 593k-row prod write was authorised on "the dry-run reports 593,010 marked and 0
guard-rejected". The code enforced the first clause (`found != expect` aborted) and merely *printed* the
second; the execute path proceeded whatever the data-loss count said. Code, test and design doc had all
been written carefully, and the gap survived all three. It became visible the moment the lead restated
the criterion as an explicit **AND**, because then the question "which clause does the code check?" has a
list to check against rather than a sentence to re-read.

Two properties make this worth doing by default:

* **The clauses are usually independent, and that is easy to miss.** Here the scope could hold exactly the
  expected number of markable rows *and* data-loss rows beside them — so a matching count was no evidence
  at all about the second number. Prose ("the counts should look right") hides that; a conjunction forces
  you to ask what each conjunct rules out on its own.
* **An unenforced conjunct degrades to a promise.** It is then kept by whoever remembers to read the
  second number, on the night the window opens. That is the same "encode the rule in the instrument, not
  the process" argument this file is about, applied to the criterion rather than the check.

So: write the criterion as `A AND B AND C`, then for each conjunct name the line of code that refuses
when it fails. If there isn't one, the criterion is partly aspirational — which is worth knowing before
the run, not after.

*(proj-fix, 2026-08-05, from the issue-84 backfill: the gap was mine, in my own operation, and the
lead's phrasing is what exposed it.)*

## Related

* [`prod-box-reads.md`](prod-box-reads.md) — what may be read on the prod box, and why the rule is
  category-and-boundedness rather than size. Same reasoning: a rule that asks anyone to *estimate* is a
  rule that reintroduces the failure it prevents.
* Issue 28's **state, not event** requirement — a gate must assert the world is currently right, not that
  something once happened. That is this file's problem one layer up: how a *correct* instrument stops
  being true.
* Issue 120 — the ANSWER / PATH / COST split. Choose the instrument for the property that changed; this
  file is about whether the instrument you chose is honest.
