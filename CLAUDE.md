# tender-db

## Agent skills

### Structuring long-running work

When pursuing a long-running goal, don't hold the plan in your head — put it on the issue tracker and work from there. Capture the goal as a spec/PRD (`/to-spec` or `/to-prd`), break it into independently-grabbable issues (`/to-issues`), and hand those issues to subagents and teammates as their work units — one issue per agent, `Status:` and `Blocked by:` lines coordinate who does what. Use `/triage` to move issues through their states, and `/wayfinder` when the effort is too big or foggy for one session.

### Testing

Run the suites through `ops/check.sh`, not raw `cargo test`. The script prunes the
superseded test binaries and builds with debuginfo off (issue 260) — raw `cargo test`
skips both and fills this container's disk in a session (measured three times on
2026-08-20 and again on 2026-08-23; the recovery each time was `cargo clean`, ~28 GiB).
For a single focused test mid-iteration, plain `cargo test -p <crate> <name>` is fine —
just run `ops/check.sh` before committing so the pruning happens and the truncation
traps its header documents don't eat a failure.

**`cargo check -p tender-db` DOES NOT COMPILE `crates/app/src/supervisor.rs`.**
`lib.rs` gates that module (and `admin`, `coverage`, `ledger`, `v1`, `webhooks`)
behind `#[cfg(feature = "server")]`, which is not a default feature. So a bare
`cargo check -p tender-db` returns **exit 0 on a file full of syntax garbage** —
verified on 2026-08-31 by appending literal nonsense and watching it pass. An
hour of "compiles clean" for supervisor edits was worth nothing that firing.
Use `--features server` for any ad-hoc check of those modules, or just run
`ops/check.sh`, which enables it. The same footgun applies to `cargo test`:
`--lib` tests in those modules are silently filtered out, not run.

**Adding a `Spec` arm to `supervisor::run_spec` can blow the stack of a test you
never touched.** It is a 62-arm async match, so every arm's locals live in ONE
future. On 2026-09-01 adding a census arm made
`an_execute_without_an_expected_count_is_refused` abort with `stack overflow`
(SIGABRT) — a test with no relation to the change, which reads as a mystery
regression. Wrap a big arm's body in `Box::pin(async move { ... }).await` so its
frame goes on the heap; the five census arms already do.

Never pipe `ops/check.sh` or a gating `cargo` command through `tail`/`head`/`grep`
in a background or chained command: the pipeline reports the FILTER's exit code and
a red suite reads as green (issue 254's trap; it re-bit on 2026-08-24 and a
non-compiling commit reached main — only the deploy's own nix gate stopped it).
Redirect to a file and echo `$?` instead — and then READ the echoed `GATE-EXIT=`
VALUE, not the harness/wrapper's own exit line, and not a `test result: FAILED`
grep (a COMPILE error prints no test-result line at all). This re-bit on
2026-08-30: GATE-EXIT=101 sat in the output file while `tail -1` showed the
wrapper's "[exited with code 0]" and a non-compiling test reached main (caught
by the Unit-4 adversarial panel before deploy).

### Committing

Commit when you have completed an issue or a meaningful unit of work. Multiple agents work on this worktree in parallel, so never stage with `git add -A`/`git add .` — always stage the individual files you changed, and inspect the commit afterwards (`git show --stat`) to confirm it contains only your files.

Staging a **named** file is not enough when another agent is editing that same file: `git add <file>` takes their uncommitted hunks too, and they land under your commit message. This has happened (`5e59ee5` carries a co-worker's feature its message never mentions — the work was intact, the provenance wrong).

**Push by explicit ref, not by branch name.** When HEAD sits on a handover branch,
`git push -u origin main` pushes the STALE local `main` and reports nothing — six
commits sat on `claude/tender-db-handover-i691vo` and the box while `origin/main`
stayed behind on 2026-09-04, until `git ls-remote --heads origin` showed the gap.
Push `git push origin HEAD:main HEAD:<handover-branch>` and confirm with
`git ls-remote --heads origin main`.

`git add -p` would be the fix elsewhere, but it is interactive and unavailable here. So: **`git diff <file>` immediately before staging**, and read it. If it contains hunks you did not write, another agent is mid-edit — commit your other files and coordinate rather than sweeping theirs in. For sustained work on a contended file, take a separate worktree instead.

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary — the five canonical role names used as-is. Every state is the owner's (the agent's) to set; an issue whose next step is a decision is `ready-for-agent`, and the decision is that step — nothing on the board waits for a person. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` at the repo root plus `docs/adr/`. See `docs/agents/domain.md`.

### Reading the production box

Metadata **and** bounded ⇒ free. Anything reading data pages gates on the team lead's word and runs
against a snapshot, never the serving DB. See `docs/agents/prod-box-reads.md`.
