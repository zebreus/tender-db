# 412 — the board drifts behind the code: three consecutive `needs-triage` issues were already built, deployed and verified, and none of them said so

Status: **DONE 2026-09-18** — all three `## Done when` bullets met: the `## Verify` convention is written into `docs/agents/issue-tracker.md` (395/385/398 retrofitted as the worked examples, plus the seven issues closed by this sweep), `ops/board-verify.sh` runs the blocks over the open issues (first cut, run today), and the 2026-09-15 fan-out queue is fully swept — NINE of ten were already done, 393 is the one live issue. Residual: 37 open issues carry no `## Verify` yet; the script names them every run and they are retrofitted on touch. Was: ready-for-agent — found 2026-09-17 by working the `needs-triage` queue three issues deep and finding the same thing each time. Not a code defect; a defect in the thing CLAUDE.md calls the source of truth.
Kind: operational (issue tracker hygiene — `.scratch/tender-db/issues/`, `docs/agents/triage-labels.md`)
Relates to: 395 / 385 / 398 (the three instances, all closed 2026-09-17 after verification), `docs/agents/issue-tracker.md` (the tracker's own contract), `docs/agents/triage-labels.md` (the state vocabulary that is not being applied on the way OUT of a state)
Blocked by: nothing

## Observed

Three firings, three `needs-triage` issues picked as "highest value", three times the work was
already done:

| issue | filed | what the board said | what prod said |
| --- | --- | --- | --- |
| **395** TED 2025-06 never fetched | 2026-09-15 | `needs-triage` | registry contiguous (402/402), June window starts on day 1, `fetch_complete` already a contiguity test, funnel already names holes. One acceptance item genuinely open. |
| **385** F14 corrigendum dates | 2026-09-15 | `needs-triage` | mapping units built 2026-09-15, carriers refolded, all three exemplars serving corrected values, API windows correct |
| **398** reclaimed quarantine object | 2026-09-15 | `needs-triage` | the triage fork was decided and all three prose sites — including the LIVE `/v1/openapi.json` — carry the answer |

In each case the work had been done by an adjacent unit: 395 by the 401/402 funnel work, 385 by its
own units 1–2 plus a refold, 398 by the 370 "served claims" cluster. None of them wrote back to the
issue that prompted them.

## Why it matters

The cost is not embarrassment, it is **misallocated firings**. Each of the three cost most of an
hourly firing to re-derive a state that was already true, and in two of them the re-derivation was
only worth it because it produced something new (395's scheduled check; 385's finding that its own
census metric can never reach zero). That is luck, not method. A fourth could have been pure waste.

The deeper cost is that CLAUDE.md says *"the board stays the source of truth"* and the board is
currently a **lagging indicator** of the code. An agent picking work by Status is picking by a
signal that is wrong roughly three times in three.

And it cuts the other way too, which is worse: if `needs-triage` can silently mean "done", then
`DONE` can silently mean "regressed". Nothing distinguishes a status that was written once from a
status that is still true.

## The shape of the fix, and what NOT to do

The tempting fix is a rule — "always update the issue you touched". That rule already exists
implicitly and is exactly what failed three times, because the agent doing the 401 funnel work did
not know 395 existed. **A rule that depends on remembering an issue you never read will fail the
same way.**

What would actually work is a check, in the house style of `ghost-census` / `member-twin-census` /
`registry-contiguity` — something that fires and names, rather than a discipline that must be
recalled:

- **A staleness sweep over the board.** For each issue not in a terminal state, when was its file
  last modified against when its named code paths last changed? An issue whose `Kind:` names
  `crates/app/src/coverage.rs` and whose file predates that file's last commit is a candidate for
  re-reading. Cheap, and it points at exactly the three above.
- **Or, cheaper and probably better first**: a `## Verify` block on each issue — the ONE command
  whose output distinguishes open from done. 395's would be the registry count, 385's the exemplar
  deadline, 398's a `curl`. Then re-triage is a script, not a reading exercise, and the issues that
  already carry a `Repro` section are most of the way there.

The second is the smaller unit and the one to try first. Most of these issues already have a
`## Repro` with exactly the right command in it; what is missing is the convention that its FIRST
line is machine-runnable and that its expected output is stated for BOTH states.

## Done when

- A convention is written into `docs/agents/issue-tracker.md`: every issue carries a one-command
  `## Verify` whose output distinguishes open from done, and the three issues above are retrofitted
  as worked examples.
- Something runs it. A `board-staleness` job in the weekly tick that executes each open issue's
  `Verify` line and reports the ones whose output no longer matches "open" — or, if that is too
  clever for a first cut, a `/triage` step that does it by hand over the `needs-triage` queue and is
  recorded as having run.
- The remaining `needs-triage` issues (387, 390, 391, 392, 393, 396, 399) are each checked against
  prod BEFORE being worked, and any that are already done are closed with their evidence. Three of
  ten were stale; the base rate matters and should be measured rather than assumed.

## Recorded because the pattern is mine too

Two of my own issues this week did the same thing one level down: 404's own fix was broken (issue
411) and its write-up said the opposite until it was read against the schema, and my FK inventory on
404 was wrong until I re-derived it. The common failure is **writing down what should be true
instead of what was checked**, and a board that is never re-checked preserves those errors
indefinitely. The three issues above are the same failure with a longer feedback loop.

## Comment — 2026-09-17: the sweep this issue asked for, run. The base rate is 7 in 10.

`## Done when`'s third bullet — check the rest of the `needs-triage` queue against prod BEFORE
working it — done for the ones with a runnable repro. Each took one `curl`.

| issue | claim | live verdict |
| --- | --- | --- |
| 392 `/v1/changes` never validates `since` | cursor ahead echoed back | **FIXED** — `"reset":"cursor_ahead"`, `last_cursor:"0"` |
| 399 `/v1/changes` has no `ignored_filters` | filters silently dropped | **FIXED** — `"ignored_filters":["source","country","status"]` |
| 387 unit 1 `name_prefix` upper bound dropped | `яп` → 1 match / 99 non-matches, `more:true` forever | **FIXED** — 1 item, all matching, `more:false`; the `zzzzп` control returns 0 with `more:false` |
| 393 legacy party identity | raw source roles, transliterated names | **STILL LIVE** — tender 8414191 serves `ECONOMIC_OPERATOR_NAME_ADDRESS` and `APPEAL_PROCEDURE_BODY_RESPONSIBLE` as roles; org 9954048 is `Perifereia Attikis - Geniki Dieythynsi…`, a Latin transliteration of Greek |

With 395, 385 and 398 from the previous three firings, that is **seven of ten** `needs-triage` issues
from the 2026-09-15 fan-out already resolved, and **one confirmed open**. 390, 391 and 396 are not
yet checked (their repros are multi-step rather than a single call).

**The base rate is the finding.** Picking work by `Status` from this queue has been wrong ~70 % of
the time. That is not a tracker with a few stale rows; it is a signal that is worse than useless for
prioritisation, because it systematically points at the *finished* work — the fan-out filed 14 issues
in one batch on 2026-09-15 and the subsequent fixes were done by whoever was in that code, not by
whoever owned the issue.

**And the sweep is cheap.** Four issues, four `curl`s, under two minutes — against roughly three
firings spent re-deriving 395, 385 and 398 the long way. That ratio is the argument for the `## Verify`
convention proposed above, and it is now measured rather than asserted.

### Next, concretely

- 393 is the one confirmed-open issue of the four and is where the next firing should go.
- 390, 391 and 396 still need a verdict; their repros want more than one call, which is itself
  evidence for the `## Verify` bullet — an issue whose state cannot be established in one command is
  an issue that will drift.

## Verify

    test -x ops/board-verify.sh && grep -c 'one command, both outputs' docs/agents/issue-tracker.md && ops/board-verify.sh 2>/dev/null | tail -1

- **done**: `1`, then `N verify line(s) run; no \`## Verify\` on: …` — the trailing list is the residual, and it should only shrink (37 on 2026-09-18)
- **open**: nothing, or `0` — no script, or no convention

## Comment — 2026-09-18: the sweep finished, the convention written, the script built. Closing.

**The last three, plus 387.** 390, 391 and 396 each carried their own closure in the body —
"Issue 390 is complete — all five units built, deployed and verified", "Status: **RESOLVED-VERIFIED
2026-09-16**" twice — under a line 3 that still said `needs-triage`. Re-read live today at rev
`ba9eb1f`, every acceptance table holds (the per-unit rows are on each issue). 387's body listed the
three prod reads it still owed; taken today, all three met. So the fan-out queue, complete:

| issue | live verdict | how it went stale |
| --- | --- | --- |
| 395, 385, 398 | done | (a) fixed by adjacent work that never read the issue |
| 392, 399 | done | (a) — fixed while working 390 unit 1 |
| 387 | done | (c) built, three reads owed "at the next idle window"; the window came, nobody wrote back |
| 390, 391, 396 | done | **(b) the owner wrote the closure at the BOTTOM and never touched line 3** |
| 393 | **live** | — (and its own line 3 still said `needs-triage` after I had shipped unit 2 and measured unit 3: mechanism (b), mine, flipped today) |

**Nine of ten.** The base rate is now measured over the whole queue, not four of it.

**Mechanism (b) is scriptable, so I scripted it.** A grep over every issue whose Status line is
non-terminal and whose body contains `RESOLVED-VERIFIED` / `is complete` / `VERIFIED LIVE` /
`Status: **DONE` gave 13 candidates: the three above, **two more from the same week that no hand
sweep had reached — 400 and 401, both `## RESOLVED-VERIFIED 2026-09-16 … Closing.` under
`ready-for-agent`** — and eight false positives (REOPENED issues citing their own past closure,
multi-unit issues with one unit verified, one `Relates to:` line). Both flipped today with their
evidence. Five issues in one week, then, went on reading as open after their owner had written
"verified" — that is not a discipline problem, it is a place-of-writing problem: the closure went
where the evidence goes (the bottom) and the state lives where the reader looks (line 3).

**And the vocabulary is why the grep needed a hand pass.** Counting the first word after `Status:`
across the board: `DONE` 61, `RESOLVED` 49, `resolved` 41, `CLOSED` 40, `RESOLVED-VERIFIED` 31,
`RESOLVED-DEPLOYED` 12, `FIXED` 11, and some seventy further spellings with one to six issues each
(`SETTLED`, `REPAIRED`, `RESOLVED-DIAGNOSED-HONEST`, `CLOSED-SUPERSEDED-DELIVERED`, …). The five
labels in `triage-labels.md` are applied on the way INTO a state and almost never on the way out.
Decision: not rewriting ~300 closed issues. New closures spell `DONE <date>`; the script classifies
by an explicit OPEN list (the five labels plus `REOPENED`/`open`/`BACKLOG`/`PARKED`/`DORMANT`) and
treats every other spelling as closed. Recorded in `triage-labels.md`.

**What shipped, against `## Done when`:**

1. *A convention in `issue-tracker.md`* — the `## Verify` section: one command on one indented line,
   both outputs stated, free to run, the last open unit of a multi-unit issue. 395/385/398 carry the
   retrofits as worked examples; 390/391/396/387/393/400/401 got blocks as they were closed or
   re-stated. `triage-labels.md` gained the on-the-way-out rule: line 3 moves FIRST.
2. *Something runs it* — `ops/board-verify.sh [--all] [NNN …]`: extracts each open issue's command,
   runs it (60 s cap), prints the output beside the stated done/open lines, names the open issues with
   no block. It decides nothing. Run today: 1 open issue with a block (393, output equals its stated
   open state), 37 without. The weekly-tick job the bullet also offered is deliberately NOT built:
   it would mean the supervisor executing shell lines out of markdown, and the by-hand step the
   bullet allowed as a first cut is what a triage pass needs anyway.
3. *The remaining queue checked before being worked* — done, above; 9/10.

**Residual, named so it is not mistaken for done:** 37 open issues have no `## Verify`. They are
retrofitted on touch, not by a job — writing a verify line means re-deriving the issue's state, which
is the firing's work anyway, and the script prints the list every run so it cannot go unnoticed.
