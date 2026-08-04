# Issue tracker: Local Markdown

Issues and specs (you may know a spec as a PRD) for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01` — never a single combined tickets file
- Triage state is recorded as a `Status:` line near the top of each issue file (see `triage-labels.md` for the role strings)
- Comments and conversation history append to the bottom of the file under a `## Comments` heading

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
