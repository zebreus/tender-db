# Issue tracker: Local Markdown

Issues and specs (you may know a spec as a PRD) for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01` — never a single combined tickets file
- Triage state is recorded as a `Status:` line near the top of each issue file (see `triage-labels.md` for the role strings)
- Comments and conversation history append to the bottom of the file under a `## Comments` heading
- Every issue carries a `## Verify` block — one free-to-run command and what it prints in BOTH states (see below); `ops/board-verify.sh` runs them over the open issues

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `.scratch/<effort>/map.md` — the Notes / Decisions-so-far / Fog body.
- **Child ticket**: `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`/`prototype`/`grilling`/`task`); a `Status:` line records `claimed`/`resolved`.
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `.scratch/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.

## Filing a new issue when several agents work in parallel

Two conventions, both learned the same day from two collisions inside one hour.

**Numbers: file in your own band.** The tracker has no allocator, so two agents each taking "the next
free number" within the same minute collide silently, and both commits look correct in isolation. Bands
are assigned per agent (e.g. 130+, 160+, 190+). Non-contiguous is fine; collision-free matters more.

**Investigations: announce before you file.** Bands fix *number* collisions. They do not stop two people
filing the *same finding* from different ends — and a follow-up arising at the seam between two agents'
work belongs to neither band, which is exactly when it happens. One line to the team before opening the
issue ("filing the 51-row residual as issue N") costs nothing and prevents it.

**If it happens anyway: pick an owner, do not defer.** Both collisions ended with each agent politely
standing their own issue down in favour of the other's, briefly leaving two issues pointing at each other
and none canonical — worse than the duplicate, because a duplicate gets worked twice while a mutual
pointer gets worked never. Whoever notices second should *propose* which survives, not defer to the
other, and then make both files point the same way.

## Parking an issue: defer against a signal, not a number

Measured 2026-09-01/02, by re-reading every parked issue on the board:

| issue | how it was parked | state on re-read |
| --- | --- | --- |
| 48 | "residual: 2,168 length-3 + 151 length-10" | **stale** — a fold had cleared it, 0 rows left, 2 weeks unnoticed |
| 68 | "bound memory on the 8 GB box" | **stale** — the box has 62 GiB, and the reduction it revisits was undone elsewhere |
| 169 | "measured: /data at 39%, 1.1T free" | **stale** — 58% / 709G, and the downgrade rationale was void |
| 92 | "until the `longest_chain ≥ 4,000` tripwire flags" | **fine** — gauge computed weekly, read 3,282 flat |
| 95 | "we do not know why; the path is no longer used" | **fine** — no premise that can rot |

**Three of five had gone stale, and the mechanism is visible in the table.** The
three that rotted were parked against **a number written down at a moment in
time**. The two that held were parked against **a computed signal** (92) or
against **no measurable premise at all** (95).

A number in a Status line is a photograph. It stops being true the moment the
system moves, and nothing tells anyone — the issue just sits there looking
decided. Issue 169's photograph was the worst case: it had downgraded a storage
risk, and by the time it was re-read the volume had gone 39% → 58% with the old
warning threshold leaving about nine days between first alarm and full.

So, when parking:

- **Prefer a condition the system evaluates itself.** "Deferred until
  `longest_chain ≥ 4,000`" survives indefinitely because a weekly job recomputes
  it. "Deferred, currently 3,282" does not.
- **If you must record a number, record how to re-take it.** One line — the job,
  the query, the file to stat — turns a photograph into an instruction. Every
  stale entry above cost a fresh investigation to re-derive something its author
  could have written in a sentence.
- **Say what would change the answer.** 95 is parked well precisely because it
  says the path is no longer live; a reader knows immediately what would reopen
  it.
- **Re-read the parked pile periodically.** None of the three above announced
  itself. They were found by going and looking, which nothing on the board asked
  anyone to do.

## `## Verify`: one command, both outputs

Measured 2026-09-17/18 (issue 412), by checking every `needs-triage` issue from the 2026-09-15
review fan-out against prod BEFORE working it: **nine of ten were already done.** Three (395, 385,
398) had been fixed by adjacent work that never read the issue. Three more (390, 391, 396) had their
closure — "all five units built, deployed and verified", "Status: RESOLVED-VERIFIED" — written into
the BODY by the owner while line 3 still said `needs-triage`; a grep for that shape across the board
found two more of the same week (400, 401) under `ready-for-agent`. Two mechanisms, one symptom: the
Status line is a photograph, and nothing re-takes it. Picking work by Status from that queue was
wrong nine times in ten, and each miss cost most of a firing to re-derive a state that was already
true.

So every issue carries a `## Verify` block:

    ## Verify

        <one command, on a single indented line>

    - **done**: <what it prints when the issue is done>
    - **open**: <what it prints while the issue is open>

The rules that make it a check rather than a story:

- **One command.** If the state needs three calls, the issue will drift, because nobody runs three
  calls to decide whether to re-read something. Pick the one that distinguishes the states; the rest
  of the evidence stays in `## Repro`.
- **Both outputs.** `## Repro` states the open output only. The verify line states what "done" prints
  too, so a reader — or the script — can tell which state it is looking at without re-deriving the
  issue. Record the date you last saw each output, so the next reader knows how old the photograph is.
- **Free to run.** No token, no data pages, no box load: a public `curl`, a bounded read per
  `prod-box-reads.md`, or a `grep` over a served document. A verify that needs permission will not
  be run, and then it is not a check.
- **A multi-unit issue verifies its LAST open unit**, and says which; the closed units' lines can
  stay in the body as prose.

`ops/board-verify.sh [--all] [NNN ...]` extracts each open issue's command, runs it, and prints the
output beside the stated done/open lines; it also names the open issues that carry no `## Verify`
yet. It decides nothing — the reading is the owner's — but it turns re-triage from a reading
exercise into a scroll. Run it at the top of a triage pass and before picking work by Status.

Worked examples, retrofitted 2026-09-18: 395 (a public-API window whose first row moves from the
month's last day to its first), 385 (two exemplar deadlines), 398 (a `grep -c` over the served
OpenAPI for the sentence the decision put there — the DATA reads the same in both states, so the
prose is the only thing a check can look at).

