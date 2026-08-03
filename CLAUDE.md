# tender-db

## Agent skills

### Structuring long-running work

When pursuing a long-running goal, don't hold the plan in your head — put it on the issue tracker and work from there. Capture the goal as a spec/PRD (`/to-spec` or `/to-prd`), break it into independently-grabbable issues (`/to-issues`), and hand those issues to subagents and teammates as their work units — one issue per agent, `Status:` and `Blocked by:` lines coordinate who does what. Use `/triage` to move issues through their states, and `/wayfinder` when the effort is too big or foggy for one session.

### Committing

Commit when you have completed an issue or a meaningful unit of work. Multiple agents work on this worktree in parallel, so never stage with `git add -A`/`git add .` — always stage the individual files you changed, and inspect the commit afterwards (`git show --stat`) to confirm it contains only your files.

Staging a **named** file is not enough when another agent is editing that same file: `git add <file>` takes their uncommitted hunks too, and they land under your commit message. This has happened (`5e59ee5` carries a co-worker's feature its message never mentions — the work was intact, the provenance wrong).

`git add -p` would be the fix elsewhere, but it is interactive and unavailable here. So: **`git diff <file>` immediately before staging**, and read it. If it contains hunks you did not write, another agent is mid-edit — commit your other files and coordinate rather than sweeping theirs in. For sustained work on a contended file, take a separate worktree instead.

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary — the five canonical role names used as-is. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` at the repo root plus `docs/adr/`. See `docs/agents/domain.md`.
