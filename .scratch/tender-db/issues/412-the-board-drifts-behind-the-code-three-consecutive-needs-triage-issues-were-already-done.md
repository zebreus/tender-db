# 412 — the board drifts behind the code: three consecutive `needs-triage` issues were already built, deployed and verified, and none of them said so

Status: ready-for-agent — found 2026-09-17 by working the `needs-triage` queue three issues deep and finding the same thing each time. Not a code defect; a defect in the thing CLAUDE.md calls the source of truth.
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
