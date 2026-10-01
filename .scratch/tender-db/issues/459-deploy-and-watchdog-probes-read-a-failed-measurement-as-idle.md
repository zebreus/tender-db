# 459 — the deploy and watchdog probes read a failed measurement as idle, and the deploy's test gate is bound to HEAD, not to the rev it ships

Status: ready-for-agent — units 1–3 BUILT 2026-10-01 (workflow `wf_2cb1e277-797`: an implementer, three adversarial reviewers (fail-open, deploy-safety, tests-both-ways) whose 15 defects were all reproduced and fixed, and a mutation check where 26 of 27 mutations turn a harness red). `ops/test-gate-marker.sh` passes 62 cases and `ops/watchdogs/test-watchdogs.sh` 117; the Verify reads done locally (`jobwatch=1 snapshot=1 driftwatch=1`). NEXT: install the watchdogs on the box (`install.sh`), then the first real deploy proves deploy.sh's new gate and probe.
Was status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the queue probe: one probe script with named answers, shared by `deploy.sh` and the snapshot, that lets a caller proceed only on a named answer (`idle`, or `down` when nothing listens) and never on a failed or empty one, pinned by a wrong-secret case in `ops/watchdogs/test-watchdogs.sh`.
Kind: operability (instrument discipline: ops checks that fail permissive)
Relates to: 245 (landed the deploy's queue probe), 254 (the test gate and its marker), 420 (the snapshot's quiescence
gate), 373 (jobwatch's previous permissive bug, and the offline harness it built), 165 (driftwatch is its only
automated watch), `docs/agents/instrument-discipline.md`

## What is wrong

`docs/agents/instrument-discipline.md` asks one question of every check: "what does this report when it is
broken? If the answer is *go / clear / healthy / 0*, it is dangerous however correct it is when it works" (§ The
audit question). Its ledger #6 is this exact shape: a function that "returned `0` on four distinct measurement
failures", so "`0` means idle means **go**". § Fail loud, fail closed asks for "a distinct `ERROR` arm, never folded
into 'clear'". Four ops checks and the deploy's test gate break that rule. All were read at HEAD `9b44528`, with no
uncommitted changes to any of the files. The three watchdog scripts in `/usr/local/bin` on the box have the
same sha256 as the repo copies (read 2026-10-01).

### The probes

| check | where | it cannot measure when… | what it reports then | what that costs |
|---|---|---|---|---|
| `deploy.sh` queue probe | `deploy.sh:84-90` | ssh fails (outer `\|\| true`); the secret file is missing or empty (`[ -n "$S" ] \|\| exit 0`); curl times out (`--max-time 10`; jq on empty input prints nothing and exits 0); `/admin/jobs` answers 403/404/500 (JSON with no `.current`, or non-JSON, then `\|\| true`) | `BUSY=''`, which the script reads as idle | the deploy goes on to `systemctl restart tender-db` (`:198`), which re-runs the running job from the top (`:55-59`, issue 245) |
| snapshot `running_job()` | `ops/watchdogs/tender-db-snapshot.sh:42-47, 49` | any curl failure, or any non-200 (the `['current']` lookup throws and prints `''`); a missing secret skips the whole wait gate (`:49`) | idle | a reflink of the live DB mid-fold, the "mid-transaction page soup" its header (`:16-18`) waits to avoid. The comment at `:36-37` justifies proceeding only for an app that cannot be reached |
| jobwatch | `ops/watchdogs/tender-db-jobwatch.sh:46, 50, 92-99` | `/admin/jobs` answers a JSON 403 or 404. curl has no `-f`, the JSON passes `jq -e .` (`:50`; the message at `:51` assumes a bad secret gives non-JSON), and `.recent`/`.queued` are null | `ok jobwatch: idle, 0 queued, 0 recent covering 0h, last ? → ?`, exit 0 | a failed daily or a wedged job goes unreported for as long as the probe is denied |
| driftwatch | `ops/watchdogs/tender-db-driftwatch.sh:30-33, 43-46` | the network fails, or the answer is not a release list (a GitLab 404, an API path change) | `WARN drift: … UNWATCHED this run`, exit 0, plain stdout (info priority) | it never reaches `systemctl --failed` or `journalctl -p err`, so a dead probe looks exactly like "no release yet" until 165's 2026-12-02 deadline |

The 403 and 404 are real answers of this server. `deny()` (`crates/app/src/admin.rs:620-631`) returns a JSON 404
`not found` when `TENDER_ADMIN_SECRET` is unset, and a JSON 403 `bad or missing operator secret` on a mismatch. Both
go through `error()` (`:633-636`). On the box the service reads the secret once at start, from
`EnvironmentFile=/root/tender-admin-secret` in the `admin.conf` drop-in. The scripts read the same file on every run.
So if the secret is rotated in the file without a restart, every probe gets a 403. If `admin.conf` is removed, every
probe gets a 404.

**Reproduced 2026-10-01 11:48 UTC** with the repo copies (byte-identical to the installed ones) against the public
`/admin/jobs` with a wrong secret. That endpoint answers `{"error":{"message":"bad or missing operator secret","status":403}}`:
- jobwatch printed `ok jobwatch: idle, 0 queued, 0 recent covering 0h, last ? → ?` and exited 0. For comparison, the
  box's own jobwatch at 11:29 UTC printed `ok jobwatch: running fetch #1796 (364s), 2 queued, 200 recent covering
  92h, last fetch → ok`. The 403 path never reads the queue at all.
- the snapshot (`TENDER_SNAP_DRY=1`) printed `dry run: would snapshot /data/db/tender-db.db into /data/db/snapshots
  (keep 2)` and exited 0. It did the same with a secret file that does not exist.
- driftwatch, with `TENDER_DRIFT_API` pointed at that 403, printed `WARN drift: SDK-eforms-de release probe answered
  unparseably — upstream is UNWATCHED this run` and exited 0. Pointed at a closed port, it printed the `(network)`
  line and exited 0.
- deploy.sh's jq expression, run on that 403 body, prints an empty line, so `BUSY` is empty.

Driftwatch's exit 0 was a deliberate choice: "a flaky mirror must not sit in `systemctl --failed` masking a REAL
drift alarm" (`:17-20`). The record does not support it. The journal holds 42 runs since 2026-08-21, all
`ok drift`, and 0 `WARN drift` lines.

The repo already has the right shape. `tender-db-diskwatch.sh:24` reads `${used:-100}`, so a `df` that returns
nothing counts as a full disk.

The offline harness cannot catch any of this. Its fixture server (`test-watchdogs.sh:40-56`) never reads
`x-admin-secret` and always answers 200. Its only negative cases are an unreachable port and a missing secret file
(`:153-160`), and it has no driftwatch case. This is the "Both-ways is per-axis" trap in instrument-discipline.md: the
fixture holds authentication constant, so a script that ignores the answer's status passes every case. `install.sh`
gates the install on this harness (`:30-37`, issue 373), so a fail-open script gets through that gate too.

### The gate binding

- **deploy.sh certifies HEAD and ships `$REV`.** `REV=$(git rev-parse "$REF")` (`:33`). The skip (`:75-77`) compares
  `target/.tests-green` with `HEAD_SHA`, and otherwise `./ops/check.sh` runs on the working tree (`:80`). Nothing
  compares the marker, or the tree the gate tested, with `REV`, which is the commit pushed (`:129`) and built (`:166`).
  1. Say HEAD is green and `origin/main` is a different commit. `./deploy.sh origin/main` prints "Suites already green
     at <HEAD>" and ships `origin/main` untested. That is the command the script's own refusal tells the operator to
     run (`:45`), and `docs/operations.md:134` uses it too.
  2. Say the tree is dirty. check.sh tests the uncommitted edits, prints `tree DIRTY — no marker written` and exits 0
     (`check.sh:161-162`). deploy.sh then ships the committed `REV`. `docs/operations.md:55-57` warns against this in
     prose ("certifies a tree nobody will deploy"). The script does not enforce it.
- **check.sh records the HEAD at the end of the run.** `:138` records only the start time. `:157-159` check for a
  clean tree and write `git rev-parse HEAD` only after cargo has finished. If another agent commits in this shared
  worktree during the ~12-minute run, HEAD moves and the tree is clean again by the end. The marker then names a
  commit that cargo may not have compiled, and the next `./deploy.sh` at that HEAD skips the suites. Nothing
  downstream catches it, because `nix/package.nix:60` and `:86` set `doCheck = false`.
- **The other direction costs every deploy.** Any commit moves HEAD off the marker, and most commits on this board
  touch only `.scratch/`: 16 of the last 20 at `9b44528`. So the operator judges by eye that "only `.scratch/`
  changed" and deploys with `SKIP_TESTS=1`. The handover makes this the rule (`HANDOVER-2026-10-01.md:86-87`). Both
  of 2026-10-01's restarts did it: the 426 deploy at `500da94` (07:49 UTC) and the 455 deploy at `f40d5e5` (10:04
  UTC). 455's record shows the manual `git diff --stat 9551a23 HEAD -- . ':!.scratch'`. When a judgement by eye
  replaces a gate, the gate is permissive, and the script could make that same diff itself.

## Proposed fix

The root cause is one thing in five places: a failure to measure shares its representation with the healthy answer
(an empty string, a null, exit 0). The fix gives failure its own answer, so that proceeding requires a positive
result. This is ledger #9's lesson in instrument-discipline.md: make the bad state unrepresentable rather than add
a check that catches it.

### Unit 1 — one queue probe with named answers (`deploy.sh` and the snapshot)

- Add `ops/watchdogs/tender-db-queue-probe.sh`. It reads the secret, runs
  `curl -sS --max-time 10 -o <tmp> -w '%{http_code}'`, and prints exactly one line:
  - `idle`: HTTP 200, and the body has a `current` key whose value is null.
  - `busy <id> <kind> <params>`: HTTP 200, and `.current` is an object.
  - `down`: curl exit 7 (connection refused). Nothing listens, so nothing writes. This is issue 420's reasoning,
    kept as a named arm rather than a default.
  - `error <what>`: everything else, named: no secret in the file, curl exit N, `HTTP 403 bad or missing operator
    secret`, or HTTP 200 without a `current` key.
- `deploy.sh` pipes the repo's copy over ssh (`$SSH "$VPS" bash -s < ops/watchdogs/tender-db-queue-probe.sh`), so it
  never depends on what is installed. It acts on a `case`:
  - `idle|down` proceeds.
  - `busy\ *` refuses, as today.
  - `*` refuses with `could not measure the queue: <answer, or "no answer, ssh exit N">`.
  
  `FORCE_BUSY=1` overrides both refusals. An ssh failure, an empty answer and any output nobody foresaw all land in
  `*`.
- The snapshot calls the installed probe (add it to `install.sh`'s list). `idle` and `down` snapshot. `busy` waits,
  as issue 420 built. `error` polls like `busy`, because a timeout under load can clear. If the budget runs out on
  `error`, the snapshot prints `ERROR snapshot: could not read the queue (<what>) — NOT snapshotted` and exits 1,
  not 0 like the busy skip. A missing secret becomes an `error`, no longer a skipped gate.
- **Pin.** The fixture server in `test-watchdogs.sh` checks `x-admin-secret` against the fixture secret. It answers a
  mismatch with the real `deny()` body (403 `{"error":{"status":403,"message":"bad or missing operator secret"}}`)
  and has a mode for the 404. New cases:
  - "a wrong operator secret is an error, not idle": the probe prints `error … 403 …`.
  - "the snapshot does not snapshot on a wrong secret": exit 1, and no `dry run` line.
  - "…nor without a secret file": exit 1.
  - "a down app is snapshotted": port 1, the named `down` arm.
  
  The fixture tests the exact bytes that `deploy.sh` pipes, so the only deploy-side logic left untested is the
  `case`.

Not in this issue: the probe runs once, before the push and a build that takes minutes. A job that starts in
between (the 07:35 UTC daily tick) is still restarted. That is a correct measurement taken at the wrong moment, not
a failed one.

### Unit 2 — bind the gate to the rev it ships

- `ops/check.sh`: record `start_head=$(git rev-parse HEAD)` and the tree state before cargo starts. Write the marker
  as **`start_head`**, and only if both hold:
  1. the tree outside `.scratch/` was clean at the start and is clean at the end;
  2. HEAD has not moved outside `.scratch/`, i.e. `git diff --quiet "$start_head" HEAD -- . ':!.scratch'`.
  
  Otherwise write nothing, and say which condition failed (`HEAD moved <a> → <b> outside .scratch/ during the run`).
- `deploy.sh`: the marker covers the ship when `m=$(cat target/.tests-green)` names a commit and
  `git diff --quiet "$m" "$REV" -- . ':!.scratch'` holds. Then:
  - If the marker covers `REV`, skip the suites. The dirty-tree condition goes: the marker is a fact about two
    commits, and the box builds `REV`, not the working tree.
  - If it does not, and `git diff --quiet HEAD "$REV" -- . ':!.scratch'` fails, refuse before spending 12 minutes:
    the gate tests the checked-out tree. Tell the operator to check out `$REF`, or to deploy from a fresh checkout
    of the SHA (`docs/operations.md`).
  - Otherwise run `ops/check.sh`, then require that the marker covers `REV`. If it does not, refuse: `the gate ran
    but wrote no marker covering <REV>`. This catches a dirty tree and a HEAD that moved.
  
  Past the gate step there are only two ways forward: a marker whose tree equals `REV` outside `.scratch/`, or an
  explicit `SKIP_TESTS=1`. `./deploy.sh origin/main` is then gated on what it ships. A `.scratch/`-only commit after
  a green gate deploys without `SKIP_TESTS=1`, which covers 426's. 455's also changed `.claude/settings.json`, so it
  would still re-gate. Widen the exclusion only with a stated reason.
- The `.scratch/` exclusion is sound only while no build or test reads under `.scratch/`. A grep on 2026-10-01 found
  no `include_str!`/`include_bytes!` from it and no path literal into it under `crates/`.
- **Pin.** Put the two predicates in one file that both scripts source (`ops/gate-marker.sh`: `marker_covers <rev>`
  and the check.sh end-of-run condition). Add `ops/test-gate-marker.sh`, which builds a throwaway git repo in a
  tempdir and asserts four cases:
  - a marker at `REV` covers it;
  - a marker plus a `.scratch/`-only commit covers it;
  - a marker plus a `crates/` change does not;
  - a code commit landing between start and end means no marker.
  
  `deploy.sh` runs it before reading the marker, the way `install.sh` runs `test-watchdogs.sh` before installing.

### Unit 3 — jobwatch and driftwatch exit non-zero when they could not look

- jobwatch: take the HTTP status (`-w '%{http_code}'`). Anything other than a 200 whose body has `current`, `queued`
  and an array `recent` prints `ERROR jobwatch: /admin/jobs answered HTTP 403: bad or missing operator secret —
  jobs NOT checked` and exits 1. Delete the wrong premise in the message at `:51`.
- driftwatch: a failed or unparseable probe prints `ERROR drift: … UNWATCHED` and exits 1. With 42 clean runs and no
  failures, the flaky-mirror argument has nothing behind it. If the mirror ever does prove flaky, the answer is to
  alarm on the age of the last good probe (a stamp file), not to go back to exit 0. Rewrite the header (`:17-20`)
  and `ops/watchdogs/README.md:15` to match.
- **Pin.** New `test-watchdogs.sh` cases:
  - "a wrong operator secret fails jobwatch": the fixture's 403, exit 1, `HTTP 403`.
  - "driftwatch fails on an answer that is not a release list": `TENDER_DRIFT_API` at the fixture's 404, exit 1.
  - "driftwatch fails when unreachable": port 1, exit 1.
- Then `sudo ./install.sh` on the box, which runs the harness first.

## Verify

    for s in jobwatch snapshot; do TENDER_ADMIN_URL=https://tenders.zebreus.click TENDER_ADMIN_SECRET_FILE=<(echo TENDER_ADMIN_SECRET=not-the-secret) TENDER_SNAP_DRY=1 TENDER_SNAP_WAIT_MIN=0 bash ops/watchdogs/tender-db-$s.sh >/dev/null 2>&1; printf '%s=%s ' $s $?; done; TENDER_DRIFT_API=https://tenders.zebreus.click/admin/jobs bash ops/watchdogs/tender-db-driftwatch.sh >/dev/null 2>&1; echo driftwatch=$?

This runs the repo copies of the three watchdog scripts against the public `/admin/jobs` with a wrong secret. That
endpoint answers a JSON 403: no real secret, no data, GETs only. `TENDER_SNAP_DRY=1` stops the snapshot before it
writes anything. The command reads units 1 (the snapshot, through the shared probe) and 3 (jobwatch and driftwatch),
which is the last unit. Unit 2 has no free read; `ops/test-gate-marker.sh` is its pin.

- **open** (2026-10-01 11:48 UTC): `jobwatch=0 snapshot=0 driftwatch=0`. All three report idle, ok or a WARN on a
  measurement that failed.
- **done**: all three non-zero (`jobwatch=1 snapshot=1 driftwatch=1` if the ERROR arms exit 1). After `install.sh`,
  the `/usr/local/bin` copies hash the same as the repo's.

## 2026-10-01 — built (units 1–3)

An implementer wrote all three units and their pins. Three reviewers then attacked it, each through one lens:
fail-open paths, deploy safety (a simulated deploy in a throwaway repo with stub ssh), and whether the tests fail
both ways. They found 15 defects. The fixer reproduced every one and fixed them all. What changed beyond the spec:

- **driftwatch reads every tag on the page.** It used to read only `releases[0]`, but upstream publishes patches to
  older lines after newer ones. It alarms on any `major.minor` above the vendored line, and an unparseable tag is an
  ERROR. The live feed reads `ok … 10 releases read, highest 1.14.4`.
- **The probe's `down` needs proof.** It requires a loopback URL (`--noproxy`), plus `systemctl show tender-db`
  reporting `MainPID=0`. A curl exit 7 alone can come from a proxy, a wrong port or an unroutable host. The app binds
  its port before it serves (`dioxus-server-0.7.9/src/launch.rs:140`), so startup is never read as `down`.
- **One `queue_verdict` function** (`ops/watchdogs/tender-db-queue-verdict.sh`, 17 table cases) judges the probe's
  line together with its exit status, for both `deploy.sh` and the snapshot.
- **The gate marker is `gate-v2 <sha>`.** A pre-459 bare-SHA marker covers nothing, so the first deploy after this
  re-gates once. `gate_begin` stamps the start time. The marker is refused if HEAD moved outside `.scratch/`, if the
  tree was dirty outside `.scratch/`, or if any tracked or non-ignored file outside `.scratch/` was written during the
  run, which catches an edit made and reverted mid-gate. Every git call runs with replace objects, optional locks
  and fsmonitor off, and assume-unchanged or skip-worktree files count as dirty.
- **`deploy.sh`'s whole gate decision is `gate_deploy_step`** (`skip | run | refuse-head | refuse-dirty`),
  table-tested in both directions with REV ≠ HEAD. `deploy.sh` runs `ops/test-gate-marker.sh` before trusting a
  marker, and prints its failing lines.
- **jobwatch** requires `current`, an array `queued` and a non-empty array `recent`, with a numeric `finished_at` and
  a string `outcome` on every run. The `// 0` defaults are gone.

Left as known limits:
- A write that keeps an old mtime (`cp -p`, `touch -d`) gets past the quiet-run check.
- The order inside `check.sh` (`gate_begin` before cargo) is not pinned.
- The admin secret still reaches curl on the box's command line (unchanged).
