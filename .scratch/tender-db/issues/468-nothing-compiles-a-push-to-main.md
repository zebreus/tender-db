# 468 — nothing compiles a push to main: issue 254 deferred CI until a second committer, and main now has three

Status: done (unit 1, drilled 2026-10-05; the main-red issue step LANDED 2026-10-07 — its green path runs on every push to main; the red path is drilled by the next real red, or by a `ci-drill` red merged to main never) — Unit 1 landed 2026-10-05 (`.github/workflows/compile.yml`, uncommitted at the time of writing; see "Unit 1 — landed" below). NEXT: the owner's red/green drill on a `ci-drill/468` branch, then record its run ids and wall times here. The `main-red` issue open/close step is not yet in the workflow. (Filed 2026-10-01 from the owner's board survey, workflow wf_4eac8781-4d0, verified by an adversarial pass.)
Kind: risk (process: a non-compiling `main` reaches every other agent; prod stays gated)
Relates to: 254 (chose the deploy gate over a workflow until "a second committer"), 300 (the 2026-08-30 escape),
260 (the gate's flags and its single feature resolution), 414 (what the gate compiles), 24 (the repo's only
workflow), 459 (the deploy gate, which this sits beside and does not replace)

## What is wrong

Nothing outside the committing session compiles what lands on `main`. Read 2026-10-01 at `3994975`, which is
origin `main`:

- `.github/workflows/` holds one file, `uptime-check.yml`, a scheduled curl of `/health/deep`. The public Actions
  API lists that workflow and GitHub's dynamic Copilot reviewer, and nothing else.
  `actions/runs?event=push&branch=main` answers `total_count` 0, so no workflow has ever run on a push.
- There is no hook either. `core.hooksPath` is unset, `.git/hooks` holds only the `*.sample` files, and
  `.claude/settings.json` has no `hooks` key. A local hook would guard only its own worktree anyway.
- `flake.nix` `checks` defines a clippy gate (`nix/package.nix:161–166`, with
  `--workspace --all-targets --features tender-db/server`) and the VM smoke test. No script, runbook or workflow
  runs `nix flake check`, yet `docs/architecture.md:157` and `:167` and `CONTEXT.md:194` still describe both as
  CI.

What does compile guards prod, not `main`. `ops/check.sh` runs when the committing agent runs it, over that agent's
own tree. `deploy.sh` (at `3994975`) runs that gate unless a marker covers the rev (`:73–80`), then `nix build`s on the
box (`:171`). Issue 254 picked the deploy gate over a workflow and named the gap it leaves: "What it does NOT do is
catch a red tree that someone else pushes, since it only guards the deploy — if this repo ever gets a second
committer, the workflow becomes worth adding beside it."

That condition now holds. The 82 commits on `main` since 2026-09-30 00:00 UTC (up to `3994975`, public API) come from
three committers: two Claude sessions (70 and 11 commits, counted by their `Claude-Session` trailers) and the owner
(`f40d5e5`). CLAUDE.md's Committing section is written for several agents sharing one worktree.

Both recorded escapes came after 254 closed on 2026-08-20. Both broke code behind the `server` feature. Each one
got a new CLAUDE.md paragraph (lines 52–62) and no mechanism. The times below are author dates from the public
GitHub API:

| | reached `main` | what did not compile | red until | caught by |
|---|---|---|---|---|
| 2026-08-24 | `9142c9f`, 22:02 UTC | a `Filter` initializer in `crates/app/src/v1/mod.rs` lacked `country_seed` ("server feature did not compile") | `6cf7573`, 22:14 | the box's `nix build` at deploy |
| 2026-08-30 | `4f9a97b`, 01:23 UTC | a `supervisor.rs` test read `queued[].spec`, which `QueuedJob` does not carry; the gate's GATE-EXIT=101 was misread as green | `4c2a611`, 02:12 | the Unit-4 adversarial panel (`300-stage4-implementation-plan.md:317`) |

`v1` and `supervisor` are two of the eight modules that `crates/app/src/lib.rs:9–25` compiles only under `server`,
and the second break was in test code. A check without `--features tender-db/server` would have passed both
breaks. A check without the test targets would have passed the second.

The cost falls on the other agents. Anyone who pulls a red `main` gets a gate that fails on someone else's change.
A gate takes 823 s warm (458, 2026-10-01) and about 35 minutes from clean (CLAUDE.md). A compile error also prints
no `test result` line, which is exactly how the 08-30 failure was misread as green.

## Proposed fix

Add a push-triggered workflow that compiles what the gate compiles but runs nothing. The file is
`.github/workflows/compile.yml`, with `name: compile`:

- **Triggers:** `push` to `main`, plus `workflow_dispatch` for the drill and for re-runs.
  `concurrency: compile-${{ github.ref }}` with `cancel-in-progress: true`, because only the newest push matters.
  `permissions: contents: read`.
- **Toolchain:** stable. The box builds with fenix `stable`, pinned by `flake.lock` (`nix/package.nix:19–25`). If
  the two ever disagree, the box's `nix build` decides.
- **Flags:** the gate's own, `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`
  (`ops/check.sh:19`).
- **Cache:** `Swatinem/rust-cache@v2`. Per its README, it is keyed on the rustc release and a hash of the
  `Cargo.lock`/`Cargo.toml` files. It caches `~/.cargo` and the dependency artifacts. The workspace crates are not
  cached, and unused dependencies are pruned before each save.
- **The step:**

      cargo check --locked --all-targets -p model -p store -p ingest -p tender-db --features tender-db/server

The owner's note suggested `--workspace --tests`. The spelling above differs for these reasons:

- **`--features tender-db/server` is the point of the check.** Without it, `accounts`, `admin`, `coverage`, `ledger`,
  `plan_capture`, `supervisor`, `v1` and `webhooks` are not compiled at all. CLAUDE.md's 2026-08-31 experiment passed a
  `supervisor.rs` full of garbage, and both escapes above were in these modules.
- **It uses the gate's package set in a single invocation.** This is the list in `ops/check.sh:151` (at `3994975`), so
  features resolve once and the same way the gate resolves them (issue 260: 41 crates under turso resolve
  differently per package). `--workspace` names the same four members today, because `Cargo.toml` excludes
  `crates/vendor/turso`. Spelling the list out keeps the two commands identical.
- **`--all-targets`, not `--tests`.** The gate's `cargo test` also compiles examples ("to ensure they compile", per
  the cargo docs): `crates/ingest/examples/diag139.rs` and `crates/store/examples/crash_probe.rs`. `--tests` skips
  them. `--all-targets` is also how the flake's clippy check spells it.
- **`check`, not `test`.** It emits metadata and links no test executables, so the graveyard of 200–350 MB test
  binaries that issue 260 prunes never forms. A runner starts empty, and only the cache persists between runs. The
  check needs no wasm target and no nix store, which answers 254's worry about a cold runner failing on the wasm
  toolchain. It does not compile doctests and it runs no tests. Running the suites stays the gate's job, and the
  deploy gate is unchanged.
- **`--locked`.** A manifest edit pushed without its `Cargo.lock` change fails here instead of resolving silently on
  the runner.

**Make a red run visible.** 254 expected a workflow to fail by being "ignored after the third red run", so the
result has to reach someone. A red run already marks the commit with a failed check. In addition, when a run on
`refs/heads/main` fails, it opens one `main-red` issue in the GitHub repository unless one is already open, naming
the commit and the run, and the next green run closes it. This is the open/close step of `uptime-check.yml`, which
issue 24 drilled end to end (run 36571026751, issue #2). The workflow then needs `issues: write`.

**Pushing the file.** A push from a session registers a workflow that is triggered by push or dispatch: `f9cf205`
(author `Claude`) added `uptime.yml`, and that workflow's `workflow_dispatch` drill ran. Issue 24's rule to edit
workflows through the API applies to cron schedules, and this workflow has none.

**The pin is a drill.** It is the CI form of CLAUDE.md's 2026-08-31 experiment:

1. After the workflow lands, cut a throwaway branch from `main`.
2. Append a line of garbage to `crates/app/src/supervisor.rs` on that branch.
3. Dispatch `compile` on the branch and require `conclusion: failure`, with the error reported in `supervisor.rs`.
4. Delete the branch.

A workflow that has lost `--features tender-db/server` passes this drill green, so the drill pins the footgun
itself. Record the drill's run id here, together with the wall time of the first cold run and the first warm run.

Not in scope:

- **The wasm client** (`--features web`, target `wasm32-unknown-unknown`). Neither the gate nor this check compiles
  it. Only the box's `dx bundle --platform web` does (`nix/package.nix:133`).
- **`nix flake check` on a runner** (clippy plus the VM smoke test). It needs a cold nix build of the whole graph,
  so it is a later unit if it is wanted at all. When `compile.yml` lands, correct `docs/architecture.md:157` and
  `:167` and `CONTEXT.md:194` to say what actually runs.

## Verify

    curl -sS 'https://api.github.com/repos/zebreus/tender-db/actions/runs?event=push&branch=main&per_page=1' | jq -c '[.total_count, (.workflow_runs[0] | if . then [.name, .head_sha[:7], .conclusion] else null end)]'

This uses the public GitHub API with no token. An output of `[null,null]` means the rate limit answered, not
that the issue is in either state.

- **open** (2026-10-01 12:21 UTC, `main` at `3994975`): `[0,null]`. No workflow has ever run on a push to `main`.
- **done:** `[N,["compile","<sha>","success"]]`, with N ≥ 1 and `<sha>` the head of `main`
  (`git ls-remote --heads origin main`). A `"failure"` in the third slot means the check works and `main` is red,
  so fix `main` first.

## Unit 1 — landed (2026-10-05)

`.github/workflows/compile.yml` (`name: compile`):

- **Triggers:** `push` to `main`, `push` to `ci-drill/**` (so the drill never touches `main`), and
  `workflow_dispatch`. `concurrency: compile-${{ github.ref }}`, `cancel-in-progress: true`.
  `permissions: contents: read`. `ubuntu-latest`, `timeout-minutes: 60`.
- **Toolchain:** `dtolnay/rust-toolchain@stable`. The repo pins no `rust-toolchain*` file; the box's fenix
  `stable` (flake.lock) is the reference.
- **Env:** the gate's `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`.
- **Cache:** `Swatinem/rust-cache@v2`.
- **Step:** `cargo check --locked --all-targets -p model -p store -p ingest -p tender-db --features tender-db/server`.
- **System deps:** none installed. Every dependency is pure Rust (rustls, miniz_oxide flate2, turso; no
  `build.rs` needing pkg-config/openssl/protobuf), and nix/package.nix's only `nativeBuildInputs`
  (dioxus-cli, binaryen) serve the wasm bundle, which this check does not build.
- **Not yet:** the `main-red` issue open/close step from "Make a red run visible" (needs `issues: write`). A red
  run marks the commit with a failed check today; nothing else notifies.
- **Docs:** `docs/architecture.md` (Deployment, Testing strategy) and `CONTEXT.md` (Deployment decision) no
  longer call the flake checks CI; they name `compile.yml` and say `nix flake check` is run by nothing.

No cargo command was run locally for this unit (disk); the YAML was parsed with `python3 -c 'import yaml…'`.

**NEXT — the drill, done by the owner** (it needs a push):

1. `git switch -c ci-drill/468 origin/main`, append a line of garbage to `crates/app/src/supervisor.rs`,
   commit, push. Expect `compile` red with the error reported in `supervisor.rs`.
2. Revert the garbage (push the clean tree to the same branch). Expect `compile` green.
3. Delete the branch. Record both run ids here, with the wall time of the first cold and first warm run.

The issue closes when the drill is recorded and the Verify block's `done` line holds for the head of `main`.

## Drill (2026-10-05, done)

- **Green.** Runs 37343942721, 37343942954 and 37343943476 compiled `c430a5b` on `claude/cool-sagan-5bk0rc`,
  the handover branch and `main`. Each took about 2 min 20 s from a cold cache. `claude/**` was added to the
  triggers so the session branches go red before `main` does.
- **Red.** `2ae804f` appended a line of non-Rust to `crates/app/src/supervisor.rs`, pushed to
  `claude/cool-sagan-5bk0rc` only. Run 37351247405 failed in the `cargo check` step with "could not compile
  `tender-db` (lib) due to 6 previous errors", about 30 s after a warm cache. The revert `34d0637` was pushed
  after that run finished; this workflow's `cancel-in-progress` would otherwise have cancelled it.
- **Open.** The `main-red` issue open/close step needs `issues: write`.
- **Done.** `actions/checkout` bumped to `@v5` (Node 24).

## The main-red step — landed (2026-10-07)

`compile.yml` gains `issues: write`; the check step is `continue-on-error` with an id, a step on
`refs/heads/main` opens one `main-red` issue (naming the commit and the run) unless one is open —
commenting "still red" on it otherwise — and a green run on main closes it; a last step fails the run
when the check failed, so the commit is still marked red. A cancelled run (superseded push) touches no
issue. The `uptime-check.yml` pattern, which issue 24 drilled. The red path is not drilled on main on
purpose: pushing garbage to main to test it would be the very escape this guards against; the YAML was
parsed with `python3 -c 'import yaml…'` and the green path runs on this commit's own push.
