# 254 — nothing runs the test suite but me, and I was reading grep's exit code

Status: CLOSED 2026-08-20 — the doctests are fenced, and the gap they rotted in is closed too:
`ops/check.sh` runs every crate the way that crate has to be run, and `deploy.sh` will not deploy a
tree it has not seen pass.
Kind: test-suite hygiene, self-inflicted
Relates to: 244 (the slices whose doc comments rotted), 251 (the other two)

## What happened

`cargo test -p ingest` had been failing for several commits and I did not notice. Nine doctests,
all failing to compile. None of them were tests: they were indented blocks of measured payload in
doc comments — award-label counts, verbatim prod notice bodies, the form-era XML shape — and
rustdoc compiles an indented block as Rust unless told otherwise.

    test crates/ingest/src/text/parse.rs - text::parse::AWARD_LABELS (line 73) ... FAILED
    test crates/ingest/src/text/parse.rs - text::parse::parse_money (line 232) ... FAILED
    test crates/ingest/src/project.rs   - project::AMOUNT_ELEMENT (line 391) ... FAILED
    ... 6 more

Fixed by fencing all nine as ```text (`678a753`, and project.rs in the commit before it).
`cargo test --workspace --doc` now reports 0 tests in all four crates, exit 0.

## Why I could not see it

Two independent reasons, and both are still live.

**1. I read grep's exit code, not cargo's.** Every run this session looked like

    cargo test -p ingest 2>&1 | grep -E "test result|error|FAILED" | head -30

The pipeline's status is `head`'s, which is 0 whenever it wrote anything, and `head -30` cut the
output before the doc-test section. A suite reporting `FAILED` scrolled past as `exit code 0`.
This is the same class of mistake as the earlier `grep -c` incident, where a trailing grep exiting
1 on zero matches made a green 24/24 suite report as failed — the failure mode is symmetric and
the cause is identical: **a pipeline's exit status describes its last command, not the test run.**

The habit to keep: redirect to a file, read the exit code of cargo itself, THEN grep the file.

    cargo test -p ingest > /tmp/t.log 2>&1; echo "EXIT=$?"; grep -E "test result" /tmp/t.log

**2. There is no CI.** No `.github/` in the repo at all. Nothing runs the suite except me, from
this session, by hand. Everything the board says about test coverage rests on that.

## What to do about the gap

Not decided, and deliberately not decided in a hurry — a workflow that runs `cargo test
--workspace` on push is easy to add and easy to make useless (a 10-minute cold build on a runner
with no warm nix store, failing on the app crate's wasm toolchain, ignored after the third red
run). The honest options:

1. **A pre-push check on the box** — the VPS already has the warm nix store and builds the flake
   on every deploy. A `deploy.sh` step that refuses to deploy a tree whose `cargo test --workspace`
   is red would catch this at the only moment that matters, without a new CI surface.
2. **A GitHub workflow** for the fast crates only (`model`, `store`, `ingest` — the app crate is
   the slow one and its tests are the thinnest), which is where every parser slice lands anyway.
3. **Both**, with the workflow as the notice and the deploy gate as the enforcement.

Option 1 is the one I would take: it needs no new infrastructure, it gates the thing that actually
reaches users, and it cannot be ignored. Filed rather than done because a deploy gate wants its own
measurement — how long `cargo test --workspace` takes on the box, and whether it can run while the
job queue is idle without disturbing the serving DB.


## A third way the suite lies, hit the same day (2026-08-20)

Checking that section 6 of the report did not break the app crate, I ran

    cargo test -p tender-db --lib

and got `test result: ok. 0 passed; 0 failed` — which I nearly read as a pass. The app crate's
server-side modules are behind `#[cfg(feature = "server")]`, so that invocation compiles the WASM
half and finds none of the ~83 tests.

`.cargo/config.toml` documents this trap in a 15-line comment and provides `cargo test-app` for it.
I had read that file before. I still ran the wrong command, because the wrong command's output looks
like the right command's output.

That is three green-looking zeros in one session — a pipeline's exit code, a truncated doc-test
section, and a feature-gated crate — and it settles the shape of the fix in the section above:
**option 1, a deploy gate**, but the gate must run the SAME commands a person would forget:

    cargo test -p model && cargo test -p store && cargo test -p ingest && cargo test-app

with cargo's own exit code, unpiped. A gate that runs `cargo test --workspace` would reproduce the
third trap exactly, since `--workspace` compiles the app crate without `server` too.


## The gate, as landed (2026-08-20)

`ops/check.sh` — four commands, cargo's own exit code, nothing piped:

    cargo test -p model
    cargo test -p store
    cargo test -p ingest
    cargo test-app            # the alias; `-p tender-db --lib` runs ZERO tests

On success over a CLEAN tree it writes `target/.tests-green` with the commit sha. Not written for a
dirty tree — a green run over uncommitted edits says nothing about the commit, and a marker that can
lie is worse than none. `target/` is gitignored, because this is a local fact rather than a
repository one.

`deploy.sh` runs it before anything touches the VPS, and skips it when the marker already names
`HEAD` and the tree is clean — so the ordinary flow (check, commit, deploy) pays for the suites once.
`SKIP_TESTS=1` overrides for the deploy that cannot wait; it announces itself in the log, the same
shape as `FORCE_BUSY=1`.

Measured on its own first run: **316 s** for all four crates from a warm target directory, and it
refused to write the marker because the tree was dirty — both halves of the contract, verified by
running it rather than by reading it. Five minutes is the whole cost of the gate, and zero on the
ordinary path where the marker already names HEAD.

Chosen over option 2 (a GitHub workflow) for the reason the section above gives: the gate cannot be
ignored and needs no new infrastructure. What it does NOT do is catch a red tree that someone else
pushes, since it only guards the deploy — if this repo ever gets a second committer, the workflow
becomes worth adding beside it.
