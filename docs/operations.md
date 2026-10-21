# Operations runbook

Production is a single Hetzner VPS, `root@zebreus.click`, serving
<https://tenders.zebreus.click>. It runs Ubuntu, not NixOS — the app is built by
the flake and run under a hand-written systemd unit that mirrors
`nix/module.nix`'s hardening flags (ADR-0006). Design context: `CONTEXT.md`.

## Layout on the box

| Path | What |
| --- | --- |
| `/opt/tender-db/repo.git` | bare repo; `git push vps main` from the dev machine lands here |
| `/opt/tender-db/src` | working clone the build runs from |
| `/opt/tender-db/app` | symlink → the live nix store path (atomic switch on deploy) |
| `/opt/tender-db/app-result` | `nix build` result link (the most recently built bundle) |
| `/opt/tender-db/deployed-rev` | git rev of the running deploy |
| `/data/db/tender-db.db` | the Turso database, on `/data` (local NVMe, see below) |
| `/data/archive/<source>/…` | raw fetched packages, immutable |

`/opt/tender-db/` also holds research artifacts from the exploration phase
(sample packages, SDK checkouts, scan scripts). They are not part of the
deployment; leave them alone.

**The hardware, and how to re-read it** — these numbers move, so read them
rather than trusting a copy (this one is from 2026-10-01):

| what | 2026-10-01 | re-read with |
| --- | --- | --- |
| cores | 32 | `nproc` |
| memory | 62 GiB | `free -g` |
| disks | two local NVMe drives (Micron 7450, ~1.92 TB each, which `lsblk` prints as 1.7T: its sizes are binary, TiB); no network volume | `lsblk -d -o NAME,SIZE,MODEL` |
| `/data` (DB + archive) | `/dev/md3`, software RAID on the NVMe, 1.7T in `df -h` (1.78 TB, `total_bytes` 1,780,595,036,160), 74 % used | `df -h /data` (binary units), or `/health/deep` → `.checks.disk` (bytes) from anywhere |
| `/` (system, nix store) | `/dev/md2`, 120 G, 75 G used | `df -h /` |
| clock | `Europe/Berlin`: CEST +02:00 in summer time, CET +01:00 in winter time (next switch 2026-10-25) | `timedatectl` |

## Deploy

From a clean checkout on the dev machine:

```sh
./deploy.sh          # deploys HEAD (what is checked out)
./deploy.sh <ref>    # deploys any ref
```

> **A deploy builds from a COMMITTED SHA in a CLEAN tree — never the shared working
> tree.** *"From a clean checkout"* above is the rule, not a stylistic preference, and
> this is what it prevents.
>
> Several agents share this worktree. On 2026-08-05 a deploy was authorised at
> `80e2365`; `HEAD` was `80e2365`, and the tree was **not** — two files carried
> uncommitted work, and they were the two files the deploy shipped
> (`crates/app/src/supervisor.rs`, `crates/store/src/lib.rs`). A teammate was mid-edit
> on the very constant the deploy's review had cleared. Building "what's here" would
> have put un-committed, un-reviewed code into production, and a deploy is the one
> action where that is unrecoverable.
>
> So, before staging:
>
> ```sh
> git status --short crates/     # MUST be empty
> git log --oneline -1           # the SHA you are deploying, read from output
> ```
>
> If the tree is dirty, deploy from a fresh checkout of the SHA instead — never
> `git stash` someone else's work to clear the path.
>
> **Two corollaries, both learned the same day.** A pre-deploy test run against a dirty
> tree certifies a tree nobody will deploy — the green describes the wrong artifact, so
> run the gate on the committed tip. And **a review clearance is bound to the artifact it
> was given against**: if the code changes after a review, the clearance does not
> automatically follow it, and the reviewer has to say so. See
> [`agents/instrument-discipline.md`](agents/instrument-discipline.md).
>
> Since issue 459 the script enforces the first corollary itself: its test gate is bound
> to the commit it ships, not to the working tree (see
> [The test gate and the queue probe](#the-test-gate-and-the-queue-probe-issues-245-254-459)).

The script pushes the ref to the VPS bare repo, builds `#tender-db` **on the
VPS** (it has the 1 Gb/s uplink and the warm nix store — never build the bundle
over the dev machine's ~100 kB/s link), atomically switches `/opt/tender-db/app`,
restarts the service, and health-checks the public URL. A failed build never
touches the symlink, so the old bundle keeps serving. The ssh connection carries
keepalives (`ServerAliveInterval=15`, `ServerAliveCountMax=4`) so a dropped TCP
link fails the deploy cleanly instead of hanging forever on a dead socket
(2026-07-21 incident).

Build times, warm store (the normal case): a **server-code-only** change
rebuilds in **~4.6 min**; a change that touches the **dependency graph**
(`Cargo.lock`, a new crate, a toolchain bump) is **~10.5 min**. A first build on
a *cold* store compiles the whole Rust + wasm toolchain graph and takes far
longer (tens of minutes). If a deploy might outlive your
connection, run it inside tmux on the box.

**Deploys are one at a time and never move production backwards** (f8bed0b —
two concurrent deploys raced once and the slower one would have regressed the
live rev). The VPS-side critical section (build → symlink switch → restart)
takes a `flock` on `/opt/tender-db/deploy.lock`; a second deploy while one holds
it aborts immediately with

```
another deploy holds /opt/tender-db/deploy.lock — aborting
```

Inside the lock it also refuses a regression: if the target rev is an ancestor
of the currently deployed rev (recorded in `/opt/tender-db/deployed-rev`) it
exits with `refusing regression: <rev> is an ancestor of deployed <rev>`. So
redeploying an older commit is a deliberate act — check out or revert to a
descendant rather than pointing `deploy.sh` at the old one. (For an emergency
revert to a *known-built* older bundle, use the symlink rollback below, which
bypasses the build and the guard.)

The health check at the end probes `GET /health` (see [Ingestion](#ingestion));
it must return `"ok":true` throughout, since ingestion runs in-process and the
readers keep serving over WAL during a load. The script polls for up to **120 s**
(one probe/second) before it declares the deploy failed — that grace exists
because the **first open after a schema change migrates**: additive `ALTER`s plus
`CREATE INDEX IF NOT EXISTS` build once over the whole table (tens of seconds on
the multi-million-row `notices`), which can hold `/health` past a minute. That is
startup work, not failure — the migrating-open pause clears itself once the index
is built and every subsequent open is instant.

The deploy also writes the rev into a systemd drop-in
(`/etc/systemd/system/tender-db.service.d/rev.conf`,
`Environment=COMMIT_SHA=<rev>`) and reloads before the restart. The app reads
`COMMIT_SHA` at runtime (`crates/app/src/v1/mod.rs`, `rev()`), so `/health`,
`/v1`, `/_source`, and the dashboard's System panel report the actual deployed
revision — while the `nix build` never sees the rev and stays reproducible. A
plain local build (no `COMMIT_SHA` in the environment) reports `dev`.

**The AGPL §13 source offer is a link built from that rev** (issue 463): `/_source` and
`/v1`'s `source_offer` point at `https://github.com/zebreus/tender-db/tree/<rev>`
(`v1::source_offer`; a `dev` build, or any rev that is not a full sha, links the
repository root). A public repository was declined on 2026-07-21 and the offer was a
written one; the repository was published on 2026-08-08, but until issue 463 the offer
still named its own page as the place to ask. The link is true only while GitHub has the
rev, and the deploy pushes only to the box, so `deploy.sh` first asks `rev_published`
(`ops/published.sh`): it fetches `https://github.com/zebreus/tender-db` **by URL** into
`refs/published/heads/*` and refuses a rev that no branch there contains — `is on no
branch of https://github.com/zebreus/tender-db`, naming the push to run (`git push origin
HEAD:main`, or the URL when `origin` is something else). Never `origin` itself: in a local
clone of the shared tree `origin` is that tree, whose branches carry every unpushed commit,
and the first version of the check passed exactly that case. A failed fetch refuses too.
`deploy.sh` runs its offline pin `ops/test-published.sh` first, and the app's
`source_tests` pin `ops/published.sh`'s URL to `v1::REPOSITORY`. `FORCE_UNPUBLISHED=1`
skips the check, and `/_source` then links a 404 until the push lands.

### The test gate and the queue probe (issues 245, 254, 459)

Before it pushes anything, `deploy.sh` answers two questions, and goes ahead only on a
positive answer to each — never on a failure to find out.

**Were the suites green on the tree it ships?** `ops/check.sh` writes
`target/.tests-green` (gitignored, a local fact) as `gate-v2 <sha>`, naming the HEAD **at
the start** of a green run, and only when all three hold: the tree outside `.scratch/`
was clean at the start and the end, HEAD did not move outside `.scratch/` during the
run, and no file outside `.scratch/` that git tracks or could track was written during
it (an edit reverted mid-run leaves both ends identical while cargo compiled the edit;
the file's mtime is what still says so). Otherwise its closing line says why no marker
was written (`tree DIRTY outside .scratch/ at the start`, `HEAD moved <a> → <b> outside
.scratch/ during the run`, `files outside .scratch/ changed during the run (first:
<path>)`). A marker in the pre-459 format (a bare SHA) covers nothing, so the first
deploy after 459 re-gates once. "Clean" counts untracked files against the committed
`.gitignore` files only — `.git/info/exclude`, `core.excludesFile` and
`status.showUntrackedFiles` cannot hide one — and a file flagged assume-unchanged or
skip-worktree is never clean, because `git status` cannot see its edits. `deploy.sh`
then:

| situation | what it does |
|---|---|
| the marker's tree equals `$REV`'s outside `.scratch/` | skips the suites (`Suites already green at <marker>, whose tree equals <rev>'s …`) |
| no such marker, and the checked-out HEAD differs from `$REV` outside `.scratch/` | refuses at once — the gate can only test the checked-out tree. Check out the ref, or deploy from a fresh checkout of the SHA |
| no such marker, and the tree is dirty outside `.scratch/` | refuses at once — `check.sh` would test uncommitted edits and write no marker. The refusal lists what is dirty; when it is only untracked files (a log redirected into the checkout is the usual one) it says to move them out, not to start a fresh checkout |
| otherwise | runs `ops/check.sh`, then refuses unless the marker it left covers `$REV` (`the gate ran but wrote no marker covering <rev>`) |
| `SKIP_TESTS=1` | skips the gate, and says so |

So a `.scratch/`-only commit after a green gate deploys without `SKIP_TESTS=1` — the
by-eye `git diff --stat <marker> HEAD -- . ':!.scratch'` is what the script now does
itself — and `./deploy.sh origin/main` is gated on `origin/main`, not on whatever HEAD
happens to be green. Any change outside `.scratch/`, including `.claude/` or `docs/`,
re-gates; widen the exclusion only with a stated reason. The exclusion is sound only
while no build or test reads under `.scratch/`. The predicates live in
`ops/gate-marker.sh`, which both scripts source — the table above is its
`gate_deploy_step`, one word per row — and `deploy.sh` runs their offline pin
`ops/test-gate-marker.sh` before it reads the marker, and prints its failures if it
fails. Keep logs and redirected output **outside the checkout**: an untracked file in it
is a dirty tree to the gate, so `ops/check.sh > gate.out` in the repo root writes no
marker.

**Is a job running on the box?** A restart re-runs the running job from the top (issue
245). `deploy.sh` pipes `ops/watchdogs/tender-db-queue-probe.sh` from the checkout to the
box (`ssh … bash -s <`), and the probe answers one line, which
`queue_verdict` (`ops/watchdogs/tender-db-queue-verdict.sh`, shared with the snapshot)
judges together with ssh's exit status — `idle` with a non-zero exit, two lines or a
trailing blank are not `idle`:

| answer | the deploy |
|---|---|
| `idle`, exit 0 | proceeds |
| `down`, exit 0 — a loopback refusal **and** systemd says `tender-db.service` is loaded with `MainPID=0` | proceeds — nothing is running to re-run |
| `busy <id> <kind> <params>`, exit 0 | refuses: `refusing to deploy while a job is running` |
| `error <what>`, no answer, or anything else | refuses: `could not measure the queue: <what>` |

A refusal alone is not `down`: curl exits 7 the same way when the app is up and the
probe asked the wrong place (a changed `PORT`, a proxy, a non-loopback address), so with
a running process it is `error curl exit 7 … not stopped`.

`FORCE_BUSY=1` overrides both refusals. The probe it replaced printed an empty string for
idle and for every failure to measure, so ssh trouble, a missing secret, a timeout or the
admin API's JSON 403 all read as an idle box. `error HTTP 403 bad or missing operator
secret` means `/root/tender-admin-secret` no longer matches the secret the service read
at start (it reads the file once, from the `admin.conf` drop-in); `error HTTP 404 not
found` means the service runs without `TENDER_ADMIN_SECRET`. The weekly snapshot reads
the queue through the same probe (`ops/watchdogs/README.md`).

### Waiting for a deploy: track the PID, not a `pgrep` pattern

A deploy runs 20–40 minutes, so it is natural to background it and poll. The
obvious poll is wrong:

```sh
until ! pgrep -f "deploy.sh origin/main"; do sleep 30; done   # NEVER EXITS
```

**The waiter's own command line contains the pattern, so `pgrep` matches the
waiter itself.** It reports "still running" forever, including long after the
deploy has finished — and the failure is silent, because "still running" is
exactly what a healthy in-progress deploy looks like. On 2026-09-11 seven of
these accumulated, each waiting on the next, while the log they were guarding had
already moved on. `pkill -f` with the same pattern has the matching bug plus one
more: it kills itself mid-sweep.

Capture the PID when you start it and wait on that:

```sh
log="${TMPDIR:-/tmp}/deploy-$(date +%s).log"   # OUTSIDE the checkout (below)
nohup ./deploy.sh > "$log" 2>&1 &
DEPLOY_PID=$!
while kill -0 "$DEPLOY_PID" 2>/dev/null; do sleep 30; done
```

The log goes outside the checkout because the shell creates it before `deploy.sh`
starts: a `deploy.log` in the repo root is an untracked file, so the test gate reads the
tree as dirty and refuses (issue 459).

If the PID is already lost, `ps -eo pid,args | grep -E "[d]eploy\.sh"` finds it —
the bracket around the first letter is what keeps the grep out of its own results,
the same trick for the same reason.

**Read the deploy's own verdict, not the process exit.** The last lines are
`OK  <url>/health -> 200, database ok` and `OK  deployed rev: <sha>`; confirm the
sha against `git ls-remote --heads origin main` and against `/health`'s `rev`.

Rollback: point the symlink at a previous store path and restart.

```sh
ssh root@zebreus.click 'ln -sfnT /nix/store/<old-path> /opt/tender-db/app.new \
  && mv -T /opt/tender-db/app.new /opt/tender-db/app && systemctl restart tender-db'
```

Old store paths survive until `nix store gc` runs; `nix profile`-style
generations are deliberately not used — the symlink is the whole mechanism.

## Service management

```sh
systemctl status tender-db
systemctl restart tender-db
systemctl stop tender-db
systemctl is-enabled tender-db      # enabled → survives reboot
```

Unit: `/etc/systemd/system/tender-db.service`, plus drop-ins under
`/etc/systemd/system/tender-db.service.d/` (`systemctl cat tender-db` shows the
merged result). Runs as the `tenderdb` system user (not `DynamicUser`, unlike
the NixOS module — the data in `/data` must outlive restarts).
`ProtectSystem=strict` with `ReadWritePaths=/data/db /data/archive`: the process
can write nowhere else, so a new state directory needs a unit edit, not just a
`mkdir`. The base unit sets `IP=127.0.0.1`, `PORT=8080`, `TENDER_DB`,
`TENDER_ARCHIVE`; the operator secret comes from the `admin.conf` drop-in — see
[Ingestion](#ingestion).

Anything writing to `/data` outside the service (a manual fetch run, say) must
leave the files owned by `tenderdb`, or the service loses access:

```sh
chown -R tenderdb:tenderdb /data/db /data/archive
```

## Ingestion

Ingestion runs **inside the server process** (ADR-0005, issue 16): the
Supervisor is a background task that owns the writer for its jobs while the
readers keep serving over WAL, so a production load has **zero downtime** and no
external process ever opens the DB (turso is single-process). You never stop the
service to load data.

Two ways jobs start:

- **Scheduler** — at 09:35 Europe/Berlin it enqueues the daily pipeline: on
  Mon–Fri a TED probe forward + re-fetch of the current day (the 09:30 CET
  finality window) and a `process`; every day a DÖE completed-day fetch (T+1,
  yesterday's date) + `process`; every day an FTS probe for the previous UK
  civil day (`fts daily (probe)`: a walk of cursorless windows with a 2 h
  overlap, split wherever a page is full — issue 477 — ~12 s between any two
  requests, across jobs too; it fetches every day after the newest monthly's
  last day that holds no daily, at most 14 a run, and is cancellable at any
  request) + `process fts daily (all)`; then `fetch-rates` (the ECB file, up to yesterday's
  fixing), `rederive-eur-recent` and one `project` that folds whatever landed.
  `rederive-eur-recent` (issue 504) re-derives the EUR values of the Tenders whose head was
  published in the last 8 days. The fold converts a version published today at yesterday's rate,
  because the ECB publishes at about 16:00 CET, after this chain; the next morning's pass corrects
  it. It announces what moved (ADR-0017 D5) and re-queues it for the `project` that follows.
  (The trailing `snapshot` job was removed with the backup feature,
  2026-08-06.) No operator action needed. Confirm a run fired by looking for a
  `probe` job (and the trailing
  `fetch`/`process`/`project`) with that morning's `started_at` in
  `GET /admin/jobs` → `recent[]`, or on the dashboard's Ingestion panel. The
  scheduler is a plain in-process timer (no cron/systemd timer), so it only fires
  while the service is up. A tick that passed while the process was not running —
  a box that was down at 09:35, or a deploy that restarted the service across it —
  is served at the next start: `catch_up_missed_tick` (issue 245) runs today's
  daily at once unless a successful `probe` finished at or after today's tick
  (TED's on a weekday) or one is still queued, and logs `[scheduler] the 09:35
  Berlin tick passed unserved …`. Only TODAY's tick is caught up; a day the
  process was down for entirely is not re-run, so re-drive that one via `/admin`
  if its sources need it. 09:35 is Berlin wall-clock: 07:35 UTC in summer time
  (CEST), 08:35 UTC in winter time (CET). The EU switches on the last Sunday of
  March and of October at 01:00 UTC, next on 2026-10-25
  (`berlin_offset_follows_the_eu_dst_rule` pins the rule).
- **`/admin` API** — for manual loads, backfills and reprocessing. Gated by a
  preshared operator secret in `TENDER_ADMIN_SECRET`, sent as the
  `X-Admin-Secret` header and compared in constant time. **Unset ⇒ the whole
  `/admin` surface answers 404** (the feature is simply absent); a wrong secret
  is 403.

> ⚠️ **The `fetch` / `process` / `project` CLIs are dev tools for scratch
> databases only.** Never run them against the production DB: turso is
> single-process, so a CLI cannot open the file while the service is running, and
> stopping the service to run one is exactly the downtime this design removes.
> Everything below goes through `/admin` instead.

### The operator secret

Live setup on the box (never committed): the secret is an `EnvironmentFile`
holding one `KEY=VALUE` line, kept out of the main unit so it never appears in
`git` or `systemctl cat` of the checked-in unit.

```
/root/tender-admin-secret                          # mode 600, root-owned:
    TENDER_ADMIN_SECRET=<64 hex chars>

/etc/systemd/system/tender-db.service.d/admin.conf # the drop-in that wires it:
    [Service]
    EnvironmentFile=/root/tender-admin-secret
```

systemd reads the `EnvironmentFile` as root at start, before dropping to the
`tenderdb` user, so the `0600 root` file stays unreadable to the service user
and everyone else. First-time setup:

```sh
# On the VPS:
printf 'TENDER_ADMIN_SECRET=%s\n' "$(openssl rand -hex 32)" > /root/tender-admin-secret
chmod 600 /root/tender-admin-secret
mkdir -p /etc/systemd/system/tender-db.service.d
printf '[Service]\nEnvironmentFile=/root/tender-admin-secret\n' \
  > /etc/systemd/system/tender-db.service.d/admin.conf
systemctl daemon-reload && systemctl restart tender-db
journalctl -u tender-db -n5   # logs "admin: /admin API enabled"
```

Rotating it is an edit of the file + `systemctl restart tender-db`; the secret
lives only on the box. Unset it (remove the drop-in) and the entire `/admin`
surface goes back to answering 404.

### Driving it (examples)

`GET /admin/jobs` returns the running job's live progress, the queue, and the
recent-run log — the same shape the dashboard's Ingestion panel renders.

The examples run **on the box**, where the secret lives, through `/root/aj.sh`. From
the dev machine, do NOT wrap one in `ssh root@zebreus.click '…'`: the POST examples
single-quote their JSON, so the outer quotes pair with the inner ones and the JSON
reaches the box without its double quotes (`{kind:project}`, a 400). Double-quote the
JSON inside the ssh quotes instead,
`ssh root@zebreus.click '/root/aj.sh /admin/jobs "{\"kind\":\"project\"}"'`, or paste
the example unchanged into `ssh root@zebreus.click bash -s <<'EOF'` … `EOF`.
`aj.sh <path>` GETs, `aj.sh <path> '<json>'` or `aj.sh <path> @<file>` POSTs, and
there is no method word.
It is `ops/aj.sh`, installed by `ops/watchdogs/install.sh` and pinned by
`ops/watchdogs/test-watchdogs.sh` (issue 464); it prints the server's body as it came
and exits 1 on any status but 2xx. `tender-admin` (`ops/admin.sh`) is the CLI with
named commands (`jobs`, `queue`, `enqueue`, `cancel`, `raw <METHOD> <path>`).

Both read the secret file the way `admin.conf` makes systemd read it: one
`TENDER_ADMIN_SECRET=<hex>` line, whose VALUE is the header. A raw curl must strip it
the same way, `SECRET=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' /root/tender-admin-secret)`;
the whole line sent as the header, as `$(cat /root/tender-admin-secret)` gives it, is a
403.

```sh
# What is the importer doing right now?
/root/aj.sh /admin/jobs | jq

# Fetch one TED daily, then process + project it (three sequential jobs).
/root/aj.sh /admin/jobs '{"kind":"fetch","source":"ted","package_kind":"daily","period":"2026-00136"}'
/root/aj.sh /admin/jobs '{"kind":"process","source":"ted","package_kind":"daily","period":"2026-00136"}'
/root/aj.sh /admin/jobs '{"kind":"project"}'

# Cancel a job. Two verbs reach the same handler (issue 250) — the POST form exists
# because a DELETE is unreachable from some operating sessions.
tender-admin cancel 41                              # POST /admin/jobs/41/cancel
tender-admin raw DELETE /admin/jobs/41 </dev/null   # the same handler
#
# Four answers (issue 252):
#   200 {"state":"dropped"}   it was queued and is gone
#   200 {"state":"stopping","kind":…,"in_flight":"whole-corpus: unmapped_fields (480/483)","checkpoint":…}
#                             it is running and its kind checks the stop flag — BETWEEN work
#                             items. `in_flight` names the item it must finish first (issue 382):
#                             a whole-corpus query can take an hour, and only a restart ends
#                             that one early. Read it before reaching for the restart.
#   409                       it is running as a kind with NO stop checkpoint — the
#                             honest refusal. The stoppable set is STOPPABLE_KINDS in
#                             supervisor.rs (a contract test pins it): process,
#                             reparse, data-quality, project, merge-provisional-orgs,
#                             org-merge-health, r2-census, r3-census,
#                             match-org-identifiers, build-org-match-keys,
#                             scan-org-match-keys, org-edge-census,
#                             case-review-backlog, fold-org-countries,
#                             fusion-census, rehoming-packet,
#                             satellite-orphans, drop-orphan-satellites,
#                             anchor-wall-census, xb-packet, … — plus an FTS
#                             `fetch` (issue 450: `stoppable` in supervisor.rs) and
#                             `audit-fts-ids` (issue 477 unit 3) and
#                             `backfill-org-name-variants` (issue 542; a restart
#                             resumes it after its last committed window).
#                             Issue 543 unit 3 added three:
#                             - `reprocess` stops before every package and between
#                               quarantine families inside one: a whole-file or
#                               whole-container row's records finish together,
#                               because the first of them resolves the row. Each
#                               reclaim is its own commit, and a fresh enqueue
#                               with the same params re-derives the rest;
#                             - `refold-fields` stops between carrier-sweep
#                               windows, or after the sweep, and then writes
#                               nothing. Once its re-queue starts, the re-queue
#                               and the stamp run to the end together;
#                             - `rederive-eur` stops before every window. The
#                               committed windows stand, and the next
#                               `rederive-eur` starts again at the first tender
#                               (~10 min; only a job recovered after a restart
#                               resumes from its watermark).
#                             A TED/DÖE fetch is one download and answers 409.
#                             Every other kind is in UNSTOPPABLE_KINDS beside it,
#                             each with the reason it answers 409: short, one
#                             statement or download, a one-off, or deliberate
#                             (refold-denied-schemes, disk-census). A test fails on
#                             a pushed kind in neither list or in both (issue 543
#                             unit 2), so a new kind's cancel answer is a choice.
#                             To end a running 409 kind anyway: a service restart
#                             with TENDER_DROP_JOBS=<id>, or the restart recovers
#                             it at the FRONT of the queue.
#   404                       no such job
# A cancelled data-quality run stores NOTHING: a half-measured report would read like a
# whole-corpus one, so the previous report stands.

# Backfill a DÖE monthly range (fans into one fetch per month + process + project).
/root/aj.sh /admin/jobs '{"kind":"backfill","source":"doe","range":["2024-01","2024-12"]}'

# Backfill FTS (issue 342). With no range: every month from 2021-01 through the
# previous UK month. A range reaching into the running month is refused (issue 477:
# a registered monthly is never re-walked; the daily probe fetches those days). Each
# monthly walks one 1-day window per civil day and is rate-limited. The API's
# `links.next` cursor LOSES rows (issue 477; issue 449's stuck cursor is the same
# defect), so the walk never follows it: a window whose cursorless page is full is
# split in two and both halves asked, down to two seconds (a one-second window is a
# 400). A 2021 month is ~80 paced requests, a 2026 month ~400. A span still full at
# two seconds is DENSE (issue 477 unit 1b): when its page holds ONE notice, the notice
# is completed from `/ocdsRecordPackages/{ocid}` over its run of consecutive ocids
# (about run length + 6 extra paced requests: 21 for 2023-11-14's 15-ocid notice), and
# the job row reads `<Outcome> · N dense span(s) completed: M ocid(s), R record
# request(s)` (the daily probe's row ends the same way). It still fails as `malformed`
# with staging intact on a page of several notice ids, a run past 500 ocids, a hole in
# a run, another notice dated inside the span, or a record that contradicts the page.
# A staged record (`fts/<kind>/<period>.pages/<span>-r<ocid>.json`) is never asked
# again, so a failure the error names as "staged as …" repeats on every re-enqueue
# without a request: once the cause is understood, delete that file and re-enqueue.
# A throttled month
# FAILS and keeps its staged span pages under fts/monthly/<YYYY-MM>.pages/, so
# re-enqueueing it resumes, it does not restart. Read job_log for `throttled` errors
# and re-enqueue those months. A running FTS fetch can be cancelled (issue 450): the
# walk reads the flag before every request, ends `CANCELLED at a checkpoint` with
# nothing landed, and keeps its staged span pages, so re-enqueueing the same month
# resumes where it stopped.
/root/aj.sh /admin/jobs '{"kind":"backfill","source":"fts","range":["2025-07","2026-08"]}'

# Re-fold every notice carrying a section of a KIND (issue 237) — the cheap cohort:
# notice_sections_kind is indexed, so this is an index read, unlike refold-fields.
/root/aj.sh /admin/jobs '{"kind":"refold-sections","profiles":["GroupComposition"]}'

# Re-fold an explicit, small notice-id list (issue 58's step-3 exerciser). Capped
# at 1,000 ids: a longer list is a cohort and wants `refold`/`refold-fields`.
/root/aj.sh /admin/jobs '{"kind":"refold-notices","notices":[123,124,125]}'

# Re-fold every notice carrying one of these FIELD ids (issue 88's twin of `refold`; the
# list rides the `profiles` key). The carrier sweep walks EVERY notice value table in full
# (texts, codes, classifications, amounts, dates, integers, numbers, ids) — two of them took
# 46 min on 2026-09-12 over ~31.7 M notices — and only then writes, gated on `expect` (±25 %).
# `"tables": [...]` narrows the walk when the channel is known (a date lives in notice_dates);
# a name outside the eight is refused at enqueue. Until 2026-09-12 the sweep opened only
# texts + amounts and answered 0 carriers for a date field — the same answer a typo gives.
# SIZE FIRST: `"expect": 1` makes the job enumerate, report the count in its abort message
# and write nothing, which is the dry run this job otherwise lacks. Then the real run with
# that count; a `project` is queued behind it automatically.
# The sweep reads through the reader pool, one borrow per 500k-id window (issue 549), so
# the heartbeat, admin, webhook and enqueue writes proceed during it; queued jobs still
# wait their turn. A sizing run (`expect: 1`) holds no writer at all. A wet run still holds
# the writer for its re-queue and stamp phase, so do not expect it to journal zero waits.
# A cancel stops the sweep before its next window (issue 543 unit 3), or after the sweep:
# the job answers "STOPPED by cancel …", says how far the sweep got (the table and notice
# id, or that it had finished), and has re-queued and stamped nothing. A re-run sweeps from
# the start. Once the re-queue starts, a cancel waits for it and its stamp to finish, so the
# issue-179 pair lands whole.
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-LOT_TITLE","TED-LOT_DESCRIPTION"],"expect":1}'
#   → error "refold-fields aborted: 345203 notices carry [...], expected ~1 (nothing was written)"
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-DATE_OF_CONTRACT_AWARD"],"tables":["notice_dates"],"expect":1}'

# Drop a still-queued job (a RUNNING one is asked to stop via /cancel above).
tender-admin raw DELETE /admin/jobs/42 </dev/null

# Read the newest stored data-quality report (issue 230). The measurement is a
# weekly job — Sunday 03:10 Berlin, ~36 min over 32 id windows — and this is
# where its body lands. `age_seconds` is served so a stale report cannot be
# mistaken for a current one.
/root/aj.sh /admin/reports/data-quality | jq -r .body
/root/aj.sh /admin/reports/data-quality | jq '{computed_at, age_seconds}'
```

The same through the CLI: `tender-admin raw GET /admin/reports/data-quality </dev/null | jq -r
.body`. A kind nothing has computed yet answers 404, not an empty report — "not
measured" and "measured as zero" are different claims and the report is careful
about the difference (its own text banners any section it could not measure).

**Section 16, the head-value band (issue 471 unit 1).** Every Tender whose ELECTED head
(`tenders.current_value_eur_cents`) is at or above €10 bn, one row per Tender, grouped by
the currency the figure was published in; `IMPLAUSIBLE_EUR_CENTS` (€100 bn) caps it from
above. Section 10 ranks the tail by repetition, so a figure that occurs once never shows
there; this is where it shows. Each row carries the head notice and its era, the published
and EUR figures, and three signals: the exact-10ᵏ (k ≥ 3) partner (the largest figure of
the same Tender and currency, in any version, the head is exactly 10ᵏ above — the sharp
scale-error signal; figures ≤ 10.00 as published are issue 380's placeholders and never
count), the ratio to the smallest sibling (for reading only), and the newest F14 value
corrigendum in the chain (II.1.5, II.1.7, II.2.6, V.2.4 — the 2014-directive numbering
only, so on an r2.0.8-era chain `—` means not checked). The elected row is the one the
read layer serves (same `cents DESC, currency` tiebreak); a Tender whose head column
matches no amount row at its head version (a `rederive-eur` move awaiting its refold) is
listed under "elected row NOT FOUND" rather than dropped. Bounded: a top-down range seek
on `tenders_current_value_eur` (~330 rows on 2026-10-01) plus per-Tender seeks, never a
scan; a safety cap of 1,000 Tenders prints LISTING FULL before the rows, and what it cuts
is the band's lowest heads. It adjudicates nothing — it is the listing 471's units 4 and 5
decide on. The stored report is TEXT only (`/admin/reports/data-quality` serves
`{kind, computed_at, age_seconds, body}`); the JSON form `.head_value_band.rows` exists only
from `bin/data-quality --json`, which runs every query over `/v1/sql` under that endpoint's deadline. Check it landed — the
summary line, not the header, which also prints when the query failed:
`/root/aj.sh /admin/reports/data-quality | jq -r .body | grep -cE 'Tender\(s\) in [0-9]+ currenc'`
must print `1`, and the same with `grep -c 'UNMEASURED — the .band_listing'` must print `0`.

**F14 value corrections (issue 489).** A TED F14 corrigendum block whose `TED-SECTION` is II.1.5
(the total estimated value), II.1.7 (the total awarded value) or II.2.6 (a lot's estimated value)
now moves that figure, the way 385's `NEW_VALUE.DATE` moves the deadline. The block's
`TED-NEW_VALUE.TEXT` must read as exactly one figure plus an ISO currency code (any EU grouping, a
label ending in `:` skipped when it holds no digit), and its `TED-OLD_VALUE.TEXT` as one figure. The fold
applies it ONLY where the chain carries that old figure in that field: at tender scope for
II.1.5 / II.1.7, and for II.2.6 on the one lot carrying it (no lot, or two lots with the same figure,
and nothing moves). A late F14 that names a figure the chain no longer carries is dropped. V.2.4
(a contract's value) is not mapped. The project line counts it as
`; issue-489 F14 value corrections: N admitted (estimated …, result …, lot …), … unread, … ambiguous,
… contract refused`. "Admitted" is counted at plan time; the fold's old-figure check can still drop one.
To re-apply after a change, size first, then run it for real (the handler queues its own `project`;
do not queue a second one):
`{"kind":"refold-fields","profiles":["TED-NEW_VALUE.TEXT"],"tables":["notice_texts"],"expect":1}`
aborts and prints the carrier count (191,493 on 2026-10-06), then send the same with that count as
`expect`. 156,263 Tenders, routed to the whole-corpus bucketed path.

**The exact-10ᵏ election rule (issue 471 unit 4(a)).** The head election
(`head_value_eur_cents`, through `ScalePartners` in `crates/store/src/canonical.rs`) now refuses a
positive amount F when (1) some positive figure P of the same Tender and currency, above 10.00 as
published, has F = P × 10ᵏ exactly with k ≥ 3 — P may sit in ANY version, in its amounts or in a
results round's lot award — and (2) no amount of the head version with a DIFFERENT `field` carries
the same figure (lot awards never corroborate: they are the same RES / BT-720 element as
`result_value`), and (3) F's EUR conversion is at least €1 bn (`SCALE_ERROR_MIN_EUR_CENTS`; it was
the €10 bn band floor until the below-band measurement of 2026-10-06 adjudicated the €1–10 bn
decade whole — 65 refused heads, 64 scale errors, one genuine: 8287294's GBP 4 bn framework, the
recorded false positive; the data-quality report's `BAND_FLOOR_EUR_CENTS` stays €10 bn). The head falls to
the next admitted figure, which is not a correction: under 471 alone 6988280
fell to a €1 bn placeholder (since issue 492's ×100 rule that lot figure falls too), 6803400 to £50 M. The rule is computed in the election, not stored
as a `quality` marker (the marker vocabulary stays `'withheld'`), so nothing in the parsed or
version layers changes and the read layer's elected row follows by `eur_cents` with no edit of its
own. Section 16's partner column uses the same floor and exponent (`SCALE_PARTNER_FLOOR_CENTS`,
`SCALE_ERROR_MIN_EXPONENT`). The per-LOT value goes through the same predicate. Since issue 490 the
fold elects it (`canonical::elect_lot_value`, with the running rule over the chain up to the version)
and stores it in `tender_version_lots.value_*`, which the lot row reads. So 224156's LOT-0002 serves no
value rather than its refused ceiling, and a rule change reaches stored lot values only through a
stamping refold (see "The elected lot value and its epoch-4 backfill").

**Who changes, and when.** No `PROJECTION_EPOCH` bump: a Tender is re-elected the next time the
fold rewrites it. Because of (3), and because the election only removes candidates and takes the
max, the rule can move ONLY a Tender whose elected value is already at or above €1 bn, and
only downwards. Nothing below €1 bn moves, ever, under this rule: no adjudication reaches it, and
there the slip often runs the other way (a small figure typed in thousands, a 1,000.00 placeholder
beside a genuine €1 M ceiling). Lowering the gate again is owed to a measurement of the next
decade first (issue 471; the €1–10 bn one is `.scratch/tender-db/471-values/below-band-*`).

**The €1 bn extension's drain (2026-10-06).** The €1–10 bn decade is ~3,600 Tenders, past the
1,000-id cap, so its drain is NOT the whole decade but the 65 measured heads
(`.scratch/tender-db/471-values/below-band-refuse65.tsv`, ids in column 1): the election moves a
Tender only when its elected head is refused, and the measurement's query
(`below-band-query.sh`) computes exactly that predicate. Cohort: `SELECT t.id,
v.caused_by_notice_id FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq =
t.current_seq WHERE t.id IN (<the 65>)`, then `refold-notices` + `project` as below. Expected: each of
the 65 heads drops (8287294 to GBP 4 m); one whose next figure is another unpartnered €1 bn+ slip
(514188's 94 lots) stays above €1 bn and is a finding for unit 5, not this rule; and re-running `below-band-query.sh` over the decade lists no
uncorroborated partner row.

**Other jobs will apply it to band Tenders they touch.** Until the drain below has run, any
`rederive-eur`, `reparse`, `refold-*` job or `PROJECTION_EPOCH` bump that rewrites a band Tender
re-elects it under this rule, so that job's before/after can show band value DROPS its own issue
never caused. Attribute them here: a band Tender that fell, whose old head figure has an exact
10ᵏ (k ≥ 3) same-currency partner in section 16, is this rule's, not that job's. Run the drain
first and the question does not arise.

```sh
# 1. Deploy in a queue gap (never under a running project).
/root/aj.sh /admin/jobs | jq '.recent[:5], .queued'
# 2. The cohort: every band Tender's head notice. A range seek on tenders_current_value_eur
#    plus a PK join, ~330 rows (bounded /v1/sql, on the team lead's word, never retry a 408):
#    SELECT t.id, v.caused_by_notice_id FROM tenders t
#      JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq
#     WHERE t.current_value_eur_cents >= 1000000000000
#    Refolding the WHOLE band is deliberate: it is under the 1,000-id cap, a Tender the rule does
#    not touch re-elects the same head, and it needs no second copy of the rule to pick rows.
# 3. Re-queue them (one job; the ids are notices.id, not publication numbers), then fold:
/root/aj.sh /admin/jobs '{"kind":"refold-notices","notices":[<the caused_by_notice_id column>]}'
/root/aj.sh /admin/jobs '{"kind":"project"}'
#    The refold-notices message must say re-queued N, stamped N tender(s) with N the cohort size.
```

**Expected effect** (the 22 adjudicated rows, 2026-10-06): the 18 agreed scale errors still in
the band leave it (6941544, 4972513, 8452561, 5592948, 6988280, 577127, 6581010, 4685893, 8822396,
5094790, 224156, 568960, 404296, 4785037, 6577862, 6721266, 6852637, 1163733), and so does the
split row 6803400 — 19 rows. **8400892 stays** at £10.8 bn (corroborated: `estimated_value` and
`result_value` both carry it). 4578779 and 4581663 were already drained by unit 3. Verify:
`/v1/tenders/5592948` serves £9,000,000 (the corrigendum's figure, reached without 4(b)),
`/v1/tenders/8400892` is unchanged, and in the next data-quality run's section 16 the only one of
job 2019's 22 partner rows still listed is 8400892. A band row with a partner that IS still listed
is corroborated, so read it; a row that leaves the band and is not one of the 19 is a finding too,
because unit 6's re-read is where a false positive the sample could not show would surface.

**The ×100 extension (issue 492, 2026-10-09).** k = 2 is refused too, under the same €1 bn gate,
unless the figure is a framework total. That means: a procedure figure, over a head version of
two or more lots, whose ×100 partner is only ever a lot figure, and whose lots sum to between F/10
and F. There is no corroboration exemption at k = 2. Its drain is a finder job, not a typed
id list. It re-queues every Tender whose stored head value, or any stored lot value of any version,
is at least €1 bn. The rule can lower only those, and a Tender it does not touch compares
identical (ADR-0017) and costs a compare:

```sh
/root/aj.sh /admin/jobs '{"kind":"refold-value-band"}'                  # dry: counts Tenders and notices
/root/aj.sh /admin/jobs '{"kind":"refold-value-band","dry_run":false}'  # stamps and re-queues them
/root/aj.sh /admin/jobs '{"kind":"project"}'
```

Only lots count toward (b) and (d): a LotsGroup's figure is a total over lots it groups, and a
PIN's parts carried in the head beside the CN's lots count only in a head with no Lot at all. A
withheld figure or a sentinel is no figure of its lot. A tender-scope figure equal to a lot award of
the head is that award's copy, a lot figure. A stored LOT value has its own exemption, the sibling lot: kept when the version has two or more lots,
its ×100 partner is only ever a figure of OTHER Lots (never procedure-level, never this lot's own,
never a Part's or a LotsGroup's), and the version has a procedure figure in the currency at least
as large that the election admits (a lot award's copy is no procedure figure). It applies to the
stored lot value of a Lot or a LotsGroup only: a Part's own figure never gets it, and neither do
the head election's lot candidates, since that procedure figure already outranks them. It was decided on the 11 stored lot values of €1 bn or more with
a ×100 partner (2026-10-09; `.scratch/tender-db/492-x100/lot-adjudication-*`): 9 slips, 2 genuine
(8748271, 8811221, both kept by it). The fold elects version N's lot value over versions 1..=N,
so a slip whose partner first appears in a LATER version stays on the earlier version's row
(524394, 952611, 8591463, 8730855, 8821990): a lot ranking over `tender_version_lots` filters to
the head version.

Verify, in order:
- The `refold-value-band` message: `stamped N` with N the dry run's Tender count.
- The `project` message's `compare: N verified, M corrected`: ADR-0017 D7's R3 completeness,
  N + M equal to the stamped count (more if other Tenders were pending).
- Re-read the adjudicated ids in `.scratch/tender-db/492-x100/`. Of the €1–10 bn 56, the 45 slips
  fall, the 9 frameworks stay, and 8819939 falls (the recorded wrong refusal). Of the €10 bn and
  over nine, the 6 slips fall (751664, 5545591, 6409799, 6640498, 7490161, 8810872), 8595426 and
  8618327 stay, and 4871119 falls to no head (contested, recorded on issue 492).
- The lot rows that moved: the `lot changed` correction rows of the cohort's Tenders. Expected:
  292242, 627800, 1003919, 8715174, 474292 and 8819939 lose their lot value (NULL) and their heads
  fall to the procedure figure (8819939 is the recorded wrong refusal); 8748271 and 8811221 keep
  theirs. Any other lot that fell from €1 bn or more
  is a finding, a read against its notice.

**The residual rule (issue 505, 2026-10-09).** A Lot's figure of €1 bn or more is refused, with no
partner needed, when it is exactly 10ᵏ (k ≥ 2) times what an admitted procedure figure of its
version leaves after the OTHER Lots' figures in the same field and currency (`ScalePartners`'
`residual_slip`; at least one other Lot must state that field, the residual must be at least P/100,
and k ≤ 6; P is judged with the slipped lot at its residual). It catches a lot slip whose true value appears nowhere in the chain: 8784848's
£60 bn lot 1 is 100× the £600 m its £1 bn procedure leaves after lots of £150 m and £250 m. A ratio
alone was no signal (23 heads at ≥ 10× adjudicated: 18 slips, 5 genuine, no ratio band separates
them); this shape had no false positive, and over every version holding a stored lot value of
€1 bn or more it fires on 5 Tenders, all slips (`.scratch/tender-db/505-lot-over-procedure/`).
Drain as for the ×100 extension (`refold-value-band` dry, wet, `project`). Expected: 8784848's head
falls from €72 bn to £1 bn, 553044's to €26.958 m, 5748163's and 1120720's to their procedure
figures; 915781's lot 5 loses its stored value (its head is already the procedure figure). Nothing
else moves; a sixth Tender is a finding. The rule never raises a value outside this cohort: the
slip it counts at its residual when judging P is one that survived the election before, so its
Tender held a value of €1 bn or more.

**The lot-sum rule (issue 537, 2026-10-16).** A procedure figure F of €1 bn or more (not a lot
award's copy) is refused, with no single partner needed, when it is 10ᵏ (k = 2…6) times a sum S of
its own head version's lots over n ≥ 2 of them, to one cent per lot after scaling
(`|F − 10ᵏ·S| ≤ 10ᵏ·n`; `ScalePartners::lot_sum_match`, S built by `canonical::LotSums`). On the
award side S is each Lot's latest award, or BT-709 or BT-660 per lot, and refuses unconditionally;
on the estimate side S is each Lot's largest BT-27 or BT-271, and refuses only under guard T (a
figure of the chain, in F's currency, recorded at procedure scope at least once, other than F,
within [S/10, 10·S]). Lots of kind Lot only; withheld figures, sentinels and placeholders at or
below 10.00 are no figure of their lot. 492's framework exemption wins; there is no corroboration
exemption. A LotsGroup's figure equal to a refused F falls with it (the slip in a second slot); a
lot award's tender-scope copy and a Lot's or Part's own figure are outside the rule. Decided on
unit 1's 48 matched heads (`.scratch/tender-db/537-lot-sum/`): it refuses 22, every one a head
slip. Drain as for the ×100 extension (`refold-value-band` dry, wet, `project`). Expected: those
22 heads leave the €1 bn+ band (3,750 → 3,728 on unit 1's population, less any of the four
below); 369057, 380921, 425446, 392980, 251619 and 706887 fall to no value, 165437 to
€20,202,891.70, and the rest as the synthesis's "falls to" table says.

Four more heads may move, none adjudicated. Unit 1 summed LotsGroup rows into S and the build
leaves them out, which adds two estimate-side matches outside the 48. 74787 has one version and
no procedure figure but F, so it never gets T and stays. 143300 (€1 bn in two fields over lot
estimates summing to €1 m, k = 3, n = 4) is kept today only by 471's corroboration exemption
against its €1 m group figure, and this rule has none: it falls to €1 m if either earlier contract
notice (00379665-2024, 00382458-2024) stores a procedure figure within [€100,000, €10 m]. Read
those two versions' amounts before the drain (a bounded `/v1/sql` read) and judge it as a possible
23rd. Likewise 164283, 905123 and 8636199, head slips the synthesis kept as `no T` from the readers'
reads, have earlier versions: a stored procedure figure there that gives T refuses them too
(rightly, but it moves the count). Any other head that moved is a finding, read against its notice.
Then re-read the Verify of issue 537 and section 16's `lot sum` column: a listed award-side or `T`
row is a finding (492's exemption keeps it, or it was not refolded) unless its head is a lot
award's tender-scope copy or a Lot's or Part's own figure, which the rule does not judge; a `no T`
row is the guard's keep.

**The derived lot-sum head (issue 551, 2026-10-16).** Where no procedure figure is admitted, the head
election serves the SUM of the head version's lots instead of one figure or null, on two paths
(`canonical::derive_lot_sum`; decision and evidence: `.scratch/tender-db/551-lot-sum/`). Path W was
withdrawn by unit 4a (2026-10-17, below), switched on again at unit 4b's B5 (2026-10-18) as v2: the v1 guards below
plus R1–R10, every awards sum vouched by a buyer-stated total ("Path W v2", below), switched off again on 2026-10-19
when B5's pre-registered stop rule tripped ("B5's stop rule tripped", after B5's runbook), and is **on again since
issue 551 B6** (`canonical::LOT_SUM_AWARDS_PATH = true`) as v2 with two guards on R10's vouching total, G-single and
G-cover ("Path W v2+G", after "B5's stop rule tripped").
- **path V, `lot-sum-vouched`**: a procedure figure F that 537, 471 or 492 refused is 10ᵏ times an electable lot
  sum S (each Lot's latest award, else its largest BT-27 or BT-271; a partner-only BT-709/BT-660 match only names
  k), every refused F names the same S, S is not one figure repeated on every lot with F ≈ 10ʲ of it (V1),
  0 < S ≤ €100 bn, and S is above the figure the max election serves (V0, added at unit 3's review);
- **path W, `lot-sum-awards`** (v1's guards; withdrawn by unit 4a, on as v2 at B5 with R1–R10 after G4, see "Path
  W v2" below, off when B5's stop rule tripped on 2026-10-19, on again as v2+G since B6, see "Path W v2+G"): every procedure figure is a sentinel or absent, today's head is an award copy or
  null, and the lots' latest awards (one currency, two or more lots, every one a typed Lot) pass G0/G1'/G2/G3/G4/G5
  (no repeated total, no lot that totals the others, estimates within 10×, above today's figure, above €0.00 and
  under the cap) and G5' (no award copy a scale rule refused, and no summed award one refuses at its own lot, copy
  or none; added at unit 3's review and widened in its third round: a ×1000 award would put the slip itself in the
  sum) and G8b (no two summed lot keys spelling one lot number, added in unit 3's second review round: a chain
  re-keyed from TED's `LOT-1..n` to eForms' `LOT-0001..n` with both families typed and awarded would sum each lot
  twice; `LOT-1`, `LOT-0001`, `Lot 1` and `1` all read lot 1, a key like `1a` or `2/E` reads none). G8b reads the
  key's spelling: TED's `LOT-k` is the notice's k-th lot section in document order, not its published LOT_NO
  (kept only as the `TED-LOT_NO` value), so it lines up with eForms' numbers only where the notice describes
  every lot from 1 in order. A misaligned re-keying passes it (TED `LOT-1`/`LOT-2` for lots 3 and 5 beside eForms
  `LOT-0003`/`LOT-0005` reads four lots), and distinct lots can be refused (harmless: today's head stays).
  Estimates and framework maxima are never summed on this path.

The fold writes S's EUR value to `current_value_eur_cents` and the marker beside it (`current_value_derivation`,
`current_value_cents`, `current_value_currency`, `current_value_derivation_detail`), all four NULL on an
elected head and written or cleared with every head write; never an amount row. Neither path is reachable from
`refold-value-band`'s cohort: since 537's drain every path-V head sits below €1 bn or null, and path W raises
heads from below. The drain is its own finder job, `refold-lot-sum` (ADR-0017 D6 route R3 by
tender id; no epoch bump):

```sh
/root/aj.sh /admin/jobs '{"kind":"refold-lot-sum"}'                               # dry: walks the cohort, writes nothing
/root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","dry_run":false,"expect":N}'    # N = the dry run's Tender count
```

- **The cohort** (`canonical::lot_sum_cohort_sql`, read in 100k tender-id windows on the reader pool, the stop flag
  read before each): a head version with a tender-scope figure converting to €1 bn or more (path V's evidence: every
  scale rule is gated there), or with awards above 10.00 on two or more distinct lots and no tender-scope figure that
  is neither an award's copy, nor partner-only, nor in the sentinels' range (≤ 1, = 100, or ≥ 111,111,111). Both legs
  over-select; the fold decides. Every read is a seek (plan-probed 2026-10-16: tenders PK range, then
  `tender_version_amounts_version` and `tender_version_lot_results`' PK).
- **Size** (unit 1's random sample, 22 of 994 heads in the cohort): about **195,000 Tenders** corpus-wide (95 %
  Wilson 129,000–293,000), about 1.7 notices each. The walk itself reads every head (8.8 M Tenders, a few seeks each);
  the dry run's elapsed time is its measurement. The fold then costs **~2–2.5 h**, not the 6–10 minutes first
  estimated from project 2564's rate: project 2575 (the 2026-10-17 drain, 194,404 stamped) took 2 h 18 m, the
  incremental pass-1 identity stage alone 2,513 s over 598,710 changed notices (3.1 per Tender, not 1.7), the
  sharded pre-pass ~1,630 s, and the touched expansion grew the fold to 219,278 Tenders at ~3,000 a minute
  (multi-lot heads). Run it in a queue gap outside the daily ingest. The looser cohort (any two awarded lots) is
  ~460,000.
- **The wet run** needs `expect` (the dry run's count) and aborts before any write when the walk finds more than a
  quarter off it; it stamps the cohort epoch-stale, re-queues its notices, and queues `project` behind itself.
- **Expected moves** (the 2026-10-16 drain's, before unit 4a): path V about 16 heads (537's 15 below, plus 928196
  through 492: EUR 46,797,460.35); path W about 21,200 (95 % 9,700–46,200), each an understatement of 1.5× to 569×
  raised to its awards sum (the 2026-10-17 drain, project 2575: 15,352 corrected). Every other cohort Tender compares
  identical (`verified`). From the 4a deploy to B5's this job raised no path-W head (it re-elected them back: "Path W
  withdrawn" below); for a run after the B5 deploy the expected moves are those of "Path W v2" below; from the
  stop-rule deploy (2026-10-19) to B6's it again raised none ("B5's stop rule tripped", below); from B6's deploy on,
  the moves of "Path W v2+G" below (run by `tender_ids`, never the corpus walk).

Verify, in order:
- The wet message's `stamped N` equals the dry count; the `project` message's `compare: N verified, M corrected`
  sums to it (D7's completeness; more if other Tenders were pending). M is the expected moves above, give or take;
  for a run between unit 4a and B5, the M of "Path W withdrawn" below; after B5, the M of "Path W v2"; after the
  stop-rule deploy, the M of "B5's stop rule tripped"; after B6's, the M of "Path W v2+G" (|P| = 3,467 on the 3,502).
- The done line: `/v1/tenders/7427765` serves EUR 3,000,000.00, 5943121 EUR 1,321,579.92 and 369057
  EUR 62,378,736.29, each with `value_derivation.method` = `lot-sum-vouched`, `rule` 537. The other path-V pins:
  251619 EUR 24,338,758.08, 380921 PLN 7,247,040.00, 392980 EUR 35,740,042.72, 425446 EUR 1,549,007.19, 706887
  PLN 70,560,792.75, 1024214 EUR 3,400,000.00 (`summed` BT-271-Lot), 5779108 EUR 2,120,000.00, 6342034
  EUR 10,836,754.45, 6921980 EUR 24,930,336.11, 7044132 EUR 2,385,992.00, 7829010 EUR 2,000,000.00, 8514592
  DKK 179,248,937.00. Path W's six pins (6450467, 6450469 and 6450473 EUR 3,574,764,979.28, 8426318
  GBP 3,000,000,000.00, 585917 EUR 250,000.00, 5995030 PLN 1,549,511.60, `lot-sum-awards`) read right on
  2026-10-17 and were withdrawn with path W (unit 4a's refold, below). Under v2 (B5) three served again, vouched
  (8426318, 585917, 5995030), and the Lyon-Turin trio states no total and keeps EUR 1,660,661,682.94; B5's stop
  rule's drain back (below) returned the three to their pre-551 figures, and B6's drain serves their sums again, with
  the same `vouched_by` (neither guard moves a pin: "Path W v2+G", D3).
- Unchanged, with `value_derivation` null: 8440393, 7487018 (a total repeated on every award), 5316712 (one lot
  restates the framework), 6885102 (the sum is below its head), and the estimate-only heads 4930240, 8822313,
  4925871, 175096 and 5625591.
- **The post-drain bounded read** (unit 2's open question; path W's wrong-direction rate rests on 10 fires, 0 farther,
  95 % upper bound 28 %): read every derived awards head of €100 m or more (a range seek on the value index,
  `WHERE current_value_eur_cents >= 10000000000 AND current_value_derivation = 'lot-sum-awards'`, LIMIT 200) and a
  uniform sample of about 50 path-W changes (the `tender changed` correction rows of project's run, or 50 random
  100k-id windows of `tenders` read by primary key), each against its notices, before the issue is called done.
  A sum that is farther from the notice's total than the figure it replaced is a finding, and so is a sum that
  counts one lot twice: G8b refuses two keys spelling one lot number, but not one lot under keys it cannot read as
  numbers (a re-keyed chain from `Lot A` to `LOT-0001`), nor one lot under two numbers. TED's `LOT-k` is an
  ordinal, the notice's k-th lot section, so do NOT read its digits as the lot number: compare each summed TED key
  against that notice's `TED-LOT_NO` value for the section (the version's lot fields), and treat TED `LOT-1`/`LOT-2`
  beside eForms `LOT-0003`/`LOT-0005` as possibly the same two lots until the LOT_NOs say otherwise. The read
  compares each head's summed lots against the notice's lot list. It also flags each path-W head whose summed award
  another figure of the head repeats at the same cents, typically that lot's own estimate in the head version's
  lot amount rows (a `(tender_id, seq)` seek): 471 counts the repeat as corroboration, so G5' passes a ×1000-or-more
  slip that HAS an exact partner, while the head election refuses that same lot figure at its lot (a v1 estimate of
  €1.5 m, then LOT-1 estimated and awarded €1.5 bn beside LOT-2's €0.8 m award, serves €1.5008 bn on a null head).
  Every such head is €1 bn or more (the scale rules' gate), so the range seek above reaches it; read the lot's
  earlier versions for a 10ᵏ partner. 537's lot-sum rule never judges a lot award, so a slip ×10ᵏ a sum of other
  lots' figures, with no single 10ᵏ partner, is summed too and shows only in this read.

**Path W withdrawn (issue 551 unit 4a, 2026-10-17).** The post-drain bounded read FAILED (wf_dc85fa7e-c00,
evidence `.scratch/tender-db/551-lot-sum/postdrain/read_result.json`): path W's sum was wrong on 9 of a uniform 50 of
project 2575's 15,352 corrections (18 %, 95 % ~9–31 %) and on 96 of the 229 derived awards heads of €100 m or more
(42 %; 9 of the 21 at €1 bn or more), every wrong upheld by a skeptic. Classes: `not_additive` 41 (framework ceilings
or per-supplier maxima summed across lots sharing one envelope), `double_count` 28 (one lot under two keys, a
re-issued award beside its original), `farther` 21 (farther from the notice's total than the figure it replaced),
`slip` 11, other 4. Decided (fail closed): `canonical::LOT_SUM_AWARDS_PATH` is `false`, so `derive_lot_sum` never
returns an awards sum and such a head keeps the max election (an award copy, one figure, or null). Path V is
unchanged. The path, its guards and its pins stay compiled and tested through `head_value_with_awards_path` (unit 4b,
a redesign measured on the 279 labels, turns it back on only at ≤ 1 wrong in a fresh uniform 30 and 0 farther in the
€100 m band). The marker columns, the SSE At probe's lot legs and the cohort SQL stay. (Unit 4b's v2 met that bar
under its "vouched everywhere" fallback and B5 switched it back on: "Path W v2", after this block. This block is
also B5's route back if B5's confirmation fails, and B5's stop rule took it on 2026-10-19 with the drain scoped by
`tender_ids`: "B5's stop rule tripped", after B5's runbook. Issue 551 B6 switches path W on again with two guards:
"Path W v2+G", after that.)

The refold back is the same job and cohort as the drain (the cohort is path W's shape on stored rows, so it holds
every stored `lot-sum-awards` head, the daily folds' since 4e5eaf7 included), after the 4a deploy, in a queue gap
outside the daily ingest:

```sh
/root/aj.sh /admin/jobs '{"kind":"refold-lot-sum"}'                               # dry: expect ~194,000 (2573 found 194,404)
/root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","dry_run":false,"expect":N}'    # N = this dry run's count; queues project
```

- **Cost**: the dry walk ~3 minutes (2573: 170 s); `project` **~2–2.5 h** (2575: 2 h 18 m, above).
- **Expected moves**: about 15,352 `corrected` back (2575's path-W changes), plus any path-W head the daily folds
  derived since the 4e5eaf7 deploy; every path-V head and every other cohort Tender compares identical (`verified`).
  Each move is announced as a correction (ADR-0017 D3, as the drain's were).

Verify, in order:
- The wet message's `stamped N` equals the dry count, and the `project` message's `compare: N verified, M corrected`
  sums to it (D7); M ≈ 15,352 (the 2575 count, give or take the daily folds' path-W heads).
- The 6 path-W pins serve no derivation (`value_derivation` null): 6450467, 6450469 and 6450473 EUR
  1,660,661,682.94, 8426318 GBP 2,000,000,000.00, 585917 `null`, 5995030 PLN 2,721.60 (their pre-551 figures).
- A bounded read of the value band finds no `lot-sum-awards` row: a range seek on `tenders_current_value_eur`,
  `SELECT id FROM tenders WHERE current_value_eur_cents >= 10000000000 AND current_value_derivation = 'lot-sum-awards'
  LIMIT 200` (the 229 heads the failed read found sat there), returns nothing. Plan-probe it first; there is no index
  on `current_value_derivation`, so never run the predicate without the value range.
- The 16 path-V pins are unchanged (the done line and the list above: 7427765 EUR 3,000,000.00, 5943121
  EUR 1,321,579.92, 369057 EUR 62,378,736.29, …, each `lot-sum-vouched`, 928196 rule 492), and the must-stay heads
  still serve no derivation.

**Path W v2 (issue 551 unit 4b, B4/B5, 2026-10-18).** `canonical::LOT_SUM_AWARDS_PATH = true` from B5's deploy (54b9068)
until **2026-10-19, when B5's stop rule switched it off again** (`false`; "B5's stop rule tripped", after B5's
runbook; v2's code, R1–R10 and `vouched_by` stayed, tested through `head_value_with_awards_path`), and `true` again
since issue 551 B6, with two guards on R10's candidates ("Path W v2+G", after "B5's stop rule tripped"): path W is v1's T', K
and G0–G8b unchanged, then ten refusal families R1–R10 (`canonical::awards_v2_check`, each a named pure function; the
first refusing check is named in `rule_v2.py`'s vocabulary by `lot_sum_decision`). Every v2 fire is a v1 fire: nothing
above is relaxed or re-keyed, and G8b stands. Any one refusal keeps today's head (an award copy, or null). A "summed
award" is a lot's latest award by notice id, as `LotSums` builds it. The facts are B1's, carried in the projection
only (no schema change): `LotResultState.published_lot_no` / `group`, `LotState.published_lot_no`,
`TenderVersion.stated_totals`.
- **R1**: a summed lot's result names two or more winners who are not all tenderers of ONE LotTender of that lot in
  that round (a consortium on one LotTender, or a TED award block flagged AWARDED_TO_GROUP, is one tender; no
  LotTender rows, two winners and no group flag refuse).
- **R2**, published lot numbers (TED LOT_NO / LOT_NUMBER, never TED-ITEM; eForms BT-13713; FTS the OCDS lot id), read
  as tokens: (a) two summed keys share a token; (b) one award's number lists two or more lots; (c) one key's award
  rounds name different numbers; (d) the summed awards come from two or more notices and one has no number.
- **R3**: a round notice's award-type stated total in S's currency (BT-161, BT-118, BT-1118, TED `VAL_TOTAL` on a
  CAN-type form) is exceeded 1.01× by the summed awards of the keys that notice awards; with no such total anywhere,
  S above 10× the chain's largest estimate.
- **R4**: a summed award at least 10× its own lot's largest BT-27-Lot (eForms and FTS lots; TED lot estimates carry
  no lot, a recorded gap).
- **R5**: the fifteen shape checks (`R5.1`–`R5.15`: a closed or winnerless result, a later closed result on the lot,
  an award ≈ the sum of the lot estimates or of one round's or one kind's lot figures, an award above its own bids,
  BT-709 ratios, award/estimate ratios, equal bids or n × one ceiling, S below 1,000.00 in its own currency, equal
  estimates).
- **R6**: two summed keys awarded in different notices with the same winner set and the same normalised lot title.
- **R7**: a summed award 10× or a tenth of an earlier valued award on the same key, same currency.
- **R8**: the largest summed award at least 10× the second largest.
- **R9**: the summed result's round publishes LotTenders, but none for the summed lot (an award resolved through
  another lot's tender).
- **R10, at EVERY size** (the "vouched everywhere" fallback): some stated total T in S's currency, from any notice of
  the chain, with |T − S| ≤ 1 % of S, equal to no single summed award and not a single-lot notice's own award. Kinds:
  BT-161, TED `VAL_TOTAL`, BT-118, BT-1118, BT-27-Procedure, TED `VAL_ESTIMATED_TOTAL`, TED `VAL_RANGE_TOTAL`
  LOW/HIGH (`TED-LOW` / `TED-HIGH`); never a modification's `VAL_TOTAL_BEFORE/AFTER`, never TED `VALUE` /
  `VALUE_COST`. BT-118 / BT-1118 are read here only: never an amount row, never elected (506 stands). The first
  vouching total in chain order is the detail's `vouched_by {field, notice_id, cents}` (`lot-sum-awards` detail only;
  path V's stored detail is byte-identical to unit 3's, so the 16 path-V heads produce no corrections).

How it was measured before any of it was served (the bar is the decision's and was not relaxed):
- **The shadow walk (M1, wf_66e388ca-ef6)**: the failed drain's 15,352 path-W corrections (project 2575, `/v1/changes`
  cursor 955146761 → 955254530) minus the 279 labelled ids, in the salted order `((id XOR 0x551B4) × 2654435761) mod
  2³², id`, each scored on public-API facts with the port `unit4b/rule_v2.py` (sha256 39be6d1e…). v2 (R10 from EUR
  10 m, the amendments' option (a)) shipped 30 of 54 walked; **M2 FAILED** its bar, 27 right / 3 wrong (wf_88da8542-c46). **The fallback**,
  "vouched everywhere" (R10 at every size), continued the same walk to 277 walked: 30 vouched ships read **29 right /
  1 wrong** (wf_03469a70-c1b; bar ≤ 1 wrong of 30) and the €100 m band 15/15 right (bar: 0 wrong of any class). A
  reader and an adversarial skeptic on every head, a third reader on disagreement. Evidence:
  `.scratch/tender-db/551-lot-sum/unit4b/` (`M1-walk.md`, `walk.jsonl`, `M2-result.json`, `M2-fallback-walk.md`,
  `walk_fallback.jsonl`, `M2-fallback-result.json`). Projected fires 15,352 × 30/276 ≈ **1,669**.
- **B3 port parity (wf_643ad3a2-3ba, after the 567/568/570 drains)**: the Rust v2 against the port on 5,307 rows (the
  279 labelled, the 6 out-of-sample px2 fires, the 277 walked, the 4,745 unit-1 records): **0 disagreements** on
  ship/refuse, S, currency, n, the first refusing check and `vouched_by`; projection parity clean on 562 Tenders (B1's
  real fold vs the port's facts); every v2 fire a v1 fire. **Port of record from B3 on: `b3/rule_v2_b3.py`** (sha256
  5400bc555f15…, the 570-remap-aware copy, R10 at every size) with `b3/gen_b3.py` (2e9635bae0a4…). 7511890 was
  checked separately (both W/T', S PLN 339,546.48, n = 4). Evidence: `551-lot-sum/b3/B3-result.md`.
- **The drains moved the port's own outcomes**: 18 new ships, 0 lost; all 18 read right (read18, wf_24a65e6a-e63,
  `b3/read18/read18.md`). **The reference band set is 20** (S_eur ≥ €100 m): 683951, 707424, 726354, 1102571, 1147190,
  1148297, 5278856, 5842187, 6017838, 6057925, 6195903, 6370901, 7001924, 7011414, 8426318 (M2's 15) and 6569442,
  6603588, 6679375, 7139761, 7542368. **No lot-key drain may land before B5's band seek**: 4 partial-sum ships
  (6679375, a band head, 7153588, 7398935, 7722197) leave out an award on a lot-null row, and keying those rows would
  move their S off the vouching total (6679375 would leave the band). If one lands anyway, re-run the B3 port diff and
  read any band change before B5.

**B5's runbook** (a queue gap outside the 07:35 UTC daily chain; every step's message recorded in issue 551):
1. **Deploy** the flip (`ops/check.sh`, read `GATE-EXIT=`, then the usual deploy). From the deploy on, the daily folds
   derive v2 heads for the Tenders they touch.
2. **The id cohort first** (the parity protocol: "B5 then drains an id cohort first and confirms 0 ship/refuse
   disagreements with rule_v2.py before the corpus refold"). Build the list, deduplicated, from
   `.scratch/tender-db/551-lot-sum/`:
   - the 279 labelled heads: the keys of `postdrain/read_result.json`'s `final`;
   - the 277 walked ids: `id` of every row that has one (skip the `meta` and `end` rows, as `b3/fetch.py`'s `ids()`
     does) of `unit4b/walk.jsonl` (M1's 54) and `unit4b/walk_fallback.jsonl` (the fallback's 223);
   - the px2 fires, B3's out-of-sample 6: 585917, 5995030, 467176, 294968, 7256284, 5977060 (population `oos-6` in
     `b3/results/B3-parity.json`; `unit4b/evidence.tar.gz` `score_work/px2_prep.py` names them);
   - 7511890 (`b3/B3-result.md`'s 570 check).

   563 ids (B3's 562 plus 7511890), under the 5,000 cap. **Before the wet run**, copy the port's tender cache aside
   (`b3/fetch.py`'s `RAW/tenders`: B3's reads, on which `b3/results/api_port.jsonl.gz` decided the 562); step 3's
   re-read overwrites it in place. Then:

   ```sh
   /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","tender_ids":[ID,…]}'                               # dry: "N of 563 named Tender(s) …"
   /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","dry_run":false,"expect":N,"tender_ids":[ID,…]}'    # same list; queues project
   ```

   The dry message counts the named ids the cohort predicate admits and lists (the first 20 of) those it does not:
   an id it does not admit cannot move (v2 ⊆ v1 ⊆ the predicate), so it is reported, not re-queued. The `project`
   is small (563 Tenders and their touched expansion): minutes. Expected `corrected`: about 73, the B3 ships of the
   562 (30 labelled, 2 px2, 41 walked; 7511890 is W/T'), give or take what the daily folds already re-elected.
3. **Cohort parity** before anything else. Re-read the cohort with `b3/fetch.py tenders`, then `b3/fetch.py notices`
   (the contents of any notice published since B3), ≤ 1 GET per 1.5 s. Both scripts, like `b3/export.py`, take their
   ids from `ids()`, which reads only `unit4b/M1-port-check.json` and the two walk files (B3's 562): add 7511890 to
   that list, or decide it separately as B3's verifier did (`b3/B3-result.md`; expected W/T', so no derivation). The
   b3 scripts cache under this session's scratchpad (`fetch.py`'s `SP`, `rule_v2_b3.py`'s `SCRATCH` / `B3_RAW`);
   from another session, point those at a fresh cache first.
   - **Compare the served state with the port of record** (`b3/rule_v2_b3.py`). No script does this yet:
     `b3/compare.py` compares the port with B3's removed Rust-harness TSVs, not with what is served, so write the
     comparison beside it over `rule_v2_b3.py`'s `facts()`, `v1_eval`, `v2_refusals` and `v2_decision`. A served
     `value_derivation.method = lot-sum-awards` is a ship; anything else is a refusal.
   - **The port must not judge G0 against the drained head.** `v1_eval` checks G0 against the served `value` and
     refuses (`W/G0-served`) when that is not the port's own max election. After the drain every ship serves its S,
     so the port as it stands refuses every ship. For a head served `lot-sum-awards`, the port's ship is: v1's reason
     `W` or `W/G0-served`, no R1–R10 refusal (`v2_refusals`), and the served `value` {cents, currency} and
     `value_derivation.lots` equal to the port's S, currency and n. For every other head, `v2_decision` as it stands
     (its served head is still the max election, so `W/G0-served` there is a real refusal).
   - Cross-check with B3's outcomes (`api_port.jsonl.gz`, decided on the reads kept aside in step 2). Explain any id
     whose ship/refuse moved since B3 by a notice published since, or count it as a disagreement.
   - **Bar: 0 ship/refuse disagreements, and S, currency and n equal on every ship**, or stop: switch off (step 7).
     A Rust ship the port refuses is first read under M2's bar. A `vouched_by` that names another total the port also
     counts (`facts()['vouches']`) is B2's recorded name-level difference (the first vouching total in B1's order,
     where the port groups by field), not a stop.
4. **The corpus**: `refold-lot-sum` dry, then wet with `"expect"` = that dry count (no `tender_ids`; ~194,000 Tenders,
   the walk ~3 min), its `project` ~2–2.5 h (2575: 2 h 18 m).

   ```sh
   /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum"}'                               # dry: ~194,000 (2584 found 194,411)
   /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","dry_run":false,"expect":N}'    # N = this dry count; queues project
   ```

   **Stop rule**: the two drains' `corrected` together (step 2's plus this one's; every other cohort Tender compares
   identical) outside **835–3,338** (0.5×–2× of the fallback's projected 1,669) means switch off and investigate
   (step 7). So does a pin off, or one of the 20 band heads missing (step 5). A band row outside the 20 is a stop and
   read, not a switch-off: go no further until it is read under M2's band bar (step 6), as B3's five new band heads
   were (read18). Switch off only if it reads wrong.
5. **Verify**: the wet `stamped N` equals the dry count and the `project`'s `compare: N verified, M corrected` sums
   to it (D7).
   - 585917 serves EUR 250,000.00, 5995030 PLN 1,549,511.60 and 8426318 GBP 3,000,000,000.00, each with
     `value_derivation.method` = `lot-sum-awards` and `vouched_by` (BT-118-NoticeResult on 25416405; TED-VAL_TOTAL
     PLN 1,546,390.00 on 19612533; TED-HIGH on 20083570).
   - The Lyon-Turin trio 6450467, 6450469 and 6450473 serves EUR 1,660,661,682.94 with no derivation (R10: no
     stated total).
   - The 16 path-V pins are unchanged (the done line and its list above; each `lot-sum-vouched`, 928196 rule 492,
     the detail byte-identical), and the must-stay heads (8440393, 7487018, 5316712, 6885102, 4930240, 8822313,
     4925871, 175096, 5625591) serve no derivation.
   - A bounded seek of the value band finds **exactly the 20**: `SELECT id FROM tenders WHERE current_value_eur_cents
     >= 10000000000 AND current_value_derivation = 'lot-sum-awards' LIMIT 200` (plan-probe it first: a range seek on
     `tenders_current_value_eur`; there is no index on `current_value_derivation`, so never run the predicate without
     the value range). One of the 20 missing is a stop: switch off (step 7). Any other row (a daily fold since the
     deploy can derive one) is a stop and read: step 6 reads it before anything goes on.
6. **Confirmation read** (M2's bars, M2's amended protocol: a reader and an adversarial skeptic on every head, a third
   reader on disagreement): 30 path-W corrections of the two drains, drawn by a **NEW salt** recorded in issue 551
   before the draw (not 0x551B4, and none of the 279 labelled or 277 walked ids), at most 1 wrong; plus every band
   `lot-sum-awards` row outside the 20 (step 5), 0 wrong of any class (farther, double_count, not_additive, slip,
   other; neutral allowed). This also covers heads folded after 2026-10-17, which the walk could not see. Either bar
   missed is a failure (step 7).
7. **On failure**, unit 4a's route ("Path W withdrawn", above; what the switched-off election serves is pinned in
   `path_w_switched_off_keeps_the_max_election_and_path_v`): set `LOT_SUM_AWARDS_PATH = false`, gate, deploy,
   `refold-lot-sum` dry → wet (`expect`) → its `project`, and verify a bounded seek of the value band finds no
   `lot-sum-awards` row and the 16 path-V pins are unchanged. **Taken 2026-10-19** (step 4's stop rule), with the
   drain scoped by `tender_ids`: next block.

**B5's stop rule tripped (issue 551, 2026-10-19).** Steps 1–4 ran on 2026-10-18/19: the cohort drains 2622 (74
corrected) and 2625 (1), then the corpus drain, its `project` 2627 (251,526 stamped; 248,097 verified + 3,429
corrected). The two drains' `corrected` together, 3,429 + 75 = **3,504**, is outside the pre-registered 835–3,338
(the corpus alone is above 3,338): about 2.1× the projected 1,669. The 3,504 corrections fall on 3,503 distinct
Tenders (179732 was corrected by 2622 and corrected back by 2625; it serves `null`), and a bounded PK read of all
3,503 found **3,502 serving `lot-sum-awards`** (issue 551's B5 record), so the excess is path-W fires, not other
corrections. Nothing has been read wrong yet; the switch-off is the rule (step 7), and the investigation (why 2.1×:
the issue's first decomposition is the post-lot-key-drain rate, 41 of 277 walked against M1's 30, plus 1,154 fires
outside 2575's population; and the pre-registered confirmation read, step 6) follows before any switch back on.
- **Code**: `canonical::LOT_SUM_AWARDS_PATH = false`. v2's code (R1–R10, the `vouched_by` detail) and
  `refold-lot-sum`'s `tender_ids` stay; the v2 pins hold through the test-only `head_value_with_awards_path` (the
  route back on), and `path_w_switched_off_keeps_the_max_election_and_path_v` compares production with it. Path V is
  byte-identical. From the deploy on, no fold derives `lot-sum-awards`.
- **The drain back** (after the deploy, in a queue gap outside the 07:35 UTC daily chain): `refold-lot-sum` scoped by
  `tender_ids` to the served `lot-sum-awards` heads: the `corrected` ids of 2622, 2625 and 2627 (their
  `tender changed` correction rows) that still serve it, **3,502 Tenders** (179732 is left out: it serves `null`, and
  would compare identical if listed), under the 5,000 cap, so one run. Path W was off from 4a's deploy until B5's,
  and no other fold had run since B5's deploy by 2627's end (02:35Z; issue 551), so the list is complete. If a daily
  chain project ran between B5's deploy and the switch-off deploy, add each id from its `tender changed` rows that
  serves `lot-sum-awards` (two runs if the list passes 5,000). The whole cohort instead (unit 4a's route) would plan
  the 14.9 M notices again, about 5 h (2627: 4 h 48 m), into the 07:35 UTC daily chain.

  ```sh
  /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","tender_ids":[ID,…]}'                               # dry: "N of 3,502 named Tender(s) …"
  /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","dry_run":false,"expect":N,"tender_ids":[ID,…]}'    # same list; queues project
  ```

  The evidence the investigation reads, which the drain back clears, is snapshotted already (65d33fc):
  `.scratch/tender-db/551-lot-sum/b5/investigate/served_lsa_rows.json` holds the served `value` and
  `value_derivation` (`vouched_by` included) of all 3,503 corrected ids. **Before the wet run**, snapshot the same for
  any id added beyond those 3,503 (from a daily fold, if one ran).
- **Expected moves**: one `corrected` per listed id that still serves `lot-sum-awards` (about 3,502; the PK read
  found no other kind of correction), each back to the max election (an award copy, one figure, or null) and
  announced as a correction (ADR-0017 D3); every other listed id compares identical.
- **Verify**: the wet `stamped N` equals the dry count and the `project`'s `compare: N verified, M corrected` sums to
  it (D7). 585917 serves `null`, 5995030 PLN 2,721.60 and 8426318 GBP 2,000,000,000.00, each with no derivation (their
  pre-551 figures); the Lyon-Turin trio keeps EUR 1,660,661,682.94. A bounded seek of the value band (the plan-probed
  range seek of step 5) finds no `lot-sum-awards` row. The 16 path-V pins are unchanged (each `lot-sum-vouched`, the
  detail byte-identical), and the must-stay heads serve no derivation.
- **Done** (2026-10-19 03:31–03:40Z): deployed 36ca974 (rev 9529345); dry 2628, wet 2629 and `project` 2630: 3,502
  corrected, 0 `lot-sum-awards` left on the PK reads and the band. Switched on again by B6: next block.

**Path W v2+G (issue 551 B6).** `canonical::LOT_SUM_AWARDS_PATH = true` again, with R10's two candidate guards both on
(`R10_G_SINGLE = R10_G_COVER = true`, so `R10Guards::PRODUCTION` is both, pinned in `the_production_guard_set_is_both`).
Decision and evidence: issue 551's "B6 — decision" (wf_834aaac1-932, recorded before any new fetch or read) and
`.scratch/tender-db/551-lot-sum/b6/`. Why B5's stop rule tripped was our process: B3 had recorded 41 ships of 277
walked after the 567/568/570 lot-key drains (M1: 30), and the window was never re-based on it (15,300 × 41/277 ≈
2,265 against the 2,348 seen inside 2575's population). From now on any B3-type rate change re-registers the window.
The investigation read (61 heads: 59 right, 1 neutral, 1 wrong) found one wrong, 6465510: a single-lot award notice's
own TED-VAL_TOTAL vouching a sum it is no part of. R10's "not a single-lot notice's own award" clause compares the
total with the stored award row, and that row is stored lot = null / awarded = null, so it could not see it.
- **The guards** (`canonical::r10_g_single_refuses`, `r10_g_cover_refuses`): each only removes R10 candidates, so a
  v2+G fire is a v2 fire, and with no candidate left R10 refuses under its own name. **G-single**: a stated total
  from a notice whose round in the version has exactly ONE LotResult (keyed or not) does not vouch. **G-cover**: a
  total of kind Award (BT-161, BT-118, BT-1118, a CAN-type form's TED-VAL_TOTAL) or Range (TED-LOW / TED-HIGH)
  vouches only if its notice names at least one summed key on R3's accessor (keyless results excluded); estimate
  totals are unchanged. A guard can move `vouched_by` to the next candidate in chain order. Path V is
  byte-identical.
- **Measured before the flip** (units U1–U5): the port of record `b6/rule_v2_g.py` (sha256 23cbd850…) on fresh
  fetches of the 3,502 served heads reproduces B5's v2 on 3,502 / 3,502 (ship, S, currency, n, `vouched_by` by
  membership). The guards refuse 35: v2+single 3,468, v2+cover 3,489, **v2+both P = 3,467** (`b6/u2/P.json`, ids
  sha256 a2619fd9…). The Rust v2+G equals the port on 37,732 (row, guard set) pairs, with 0 disagreements and
  projection parity clean. U4 adopted both guards by the rule fixed before the reads (adopt iff ≥ 10 % of the read
  set is non-right): G-single 3 wrong of the first 30 of its 34 refusals (6465510, 6582856, 7179291; exactly the
  bar), G-cover 1 of 1 beyond G-single (7202436). U5's fresh read of 30 from P (salt `551-b6-fresh-`, every read id
  excluded) read 30 right (GO), and the B-band re-read set was empty. The zero-drift probe on 0787596
  (`refold-lot-sum` over the 3,502: 3,502 verified, 0 corrected) puts the store at the switched-off code's fixed
  point, so the drain's D1/D2 measure path W alone.
- **Accepted residuals** (point 13): 5816379's superseded-vouch shape (about 1 in 60, errors about 1.5×) stays
  unguarded and is filed; the partial sums (lot = null awards outside S: 6679375, 7153588, 7398935, 7722197 (read18)
  and 6779279, 6850401, 7698260 right; 7749755 neutral) stay. **Subscribers see the |P| heads' third correction
  since B5** (on at B5's drain, off at its drain back, on again at B6's). For the 2,334 of P inside project 2575's
  15,300 corrected ids (`b6/u2/P.json` v2+both ∩ `b5/investigate/p2575_ids.json`), unit 3's drain (2575) and unit
  4a's drain back (2586) make it the fifth. Each correction is a `version: null` `changed` row for the Tender and its
  lots (ADR-0017 D3).
- **Timing** (point 10): flip no earlier than the 10-21 post-chain gap (10-20 only if issue 560 records U-d PASS and
  closed on the 10-20 morning and issue 562 unit 5 has its cursor ranges). Never clock-driven: a missed bar waits for
  the next gap. Do not start unless everything can finish at least 2 h before the next 07:35 UTC chain (D7).
- **Before the deploy** (U6, in the gap): re-run the zero-drift probe on the deployed rev (expect 0 corrected);
  refresh the rates and record their sha256; enumerate L_changed (every Tender with a change row from 958787061 to
  the cursor read right after the flip deploy, excluding 2630's own 3,502 head-revert rows, plus the whole refolded
  sets of 541, 571 and 545), refetch L_changed ∩ 3,502 and re-predict them into P; record P's sha256, the band set
  B_P and the pins; snapshot the served `value` and `value_derivation` of every listed id. The switch-off twin
  (W = false) is built and gated on the currently deployed base before each gap.
- **The drain** (point 9): `refold-lot-sum` by `tender_ids`, never the corpus walk, dry → wet (`expect`) → its
  `project`, in runs of at most 5,000. Stage 1: the 61, the pins, 7511890, every id the guards refuse, and a salted
  300 of P (one run); check D1–D4. Stage 2: the rest of the 3,502; check D1–D5. Then the L_changed runs; check D6.

  ```sh
  /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","tender_ids":[ID,…]}'                               # dry: "N of M named Tender(s) …"
  /root/aj.sh /admin/jobs '{"kind":"refold-lot-sum","dry_run":false,"expect":N,"tender_ids":[ID,…]}'    # same list; queues project
  ```

- **Stop rules** (D1/D2 on a PK-snapshot diff of the named ids; a fire in the touched expansion is judged under
  D5/D6). **D1** the ids corrected to `lot-sum-awards` equal P exactly, each with its predicted S, currency, n and
  `vouched_by`; the 35 the guards refuse compare identical. An unpredicted fire or a different S: switch off. A
  predicted id that does not fire: stop and explain within the gap, else switch off. **D2** corrected = |P| exactly
  (tolerance 0). **D3** pins: 585917 EUR 250,000.00, 5995030 PLN 1,549,511.60, 8426318 GBP 3,000,000,000.00, each
  `lot-sum-awards` with its recorded `vouched_by`; the Lyon-Turin trio keeps EUR 1,660,661,682.94 with no
  derivation; the 16 path-V pins byte-identical; the 9 must-stay heads with no derivation; 6465510 does not serve
  `lot-sum-awards`. Any one off: switch off. **D4** the plan-probed band seek (≥ EUR 100 m) equals B_P; a missing
  head is a switch-off, an extra row is read at 0 wrong of any class (899272, within 2 % of EUR 100 m, is
  stop-and-explain). **D5** any id serving `lot-sum-awards` outside P ∪ L_changed (PK-read every id in the project's
  change-row range): switch off. **D6** L_changed runs: new fires above max(2 × projection, projection + 10), the
  projection being dry-admitted × 3,502/251,526, stop and read; the delta read (all new fires if 30 or fewer, else a
  salted 30, `551-b6-delta-`) uses U5's sequential bar, and failing it is a switch-off. **D7** time, as above.
- **Switch-off route** (point 11): redeploy the pre-gated W = false twin, then `refold-lot-sum` with `tender_ids` =
  the drain ids ∪ every id with a change row since the flip cursor that serves `lot-sum-awards` (PK-read; runs of at
  most 5,000), then verify that the band seek and the PK reads show no `lot-sum-awards` row. The corpus walk
  (about 5 h) is the alternative, in a gap away from 07:35 UTC.
- **After the flip** (point 12): a 14-day watch after each 07:35 UTC chain: the band seek for new `lot-sum-awards`
  rows ≥ EUR 100 m, each read at 0 wrong of any class, else switch off; the daily count of new heads against the
  fire rate the L_changed runs observed on the chain's folded Tenders (more than 3× is stop and read); on day 7 a
  salted 30 (`551-b6-daily-`) of heads first derived by a daily fold, at U5's bar. **Standing rule**: no drain or
  deploy that changes path-W inputs (lot keys, winners, org merges, rounds, stated totals, published lot numbers)
  while path W is on without re-running the port diff on the served set first. Pins: 6908248 (right only while
  22266591's RES-2..10 stay lot = null) and the partial sums 6679375, 7153588, 7398935, 7722197, 6779279, 6850401,
  7698260.

**Result-level framework values as scale partners (issue 506, 2026-10-10).** An eForms award
notice's framework maximum and re-estimate per lot result (BT-709, BT-660, and their eForms-DE 1.x
spellings) are stored on the Lot their result names through BT-13713 as `result_framework_maximum` /
`result_framework_reestimate` — dropped when the result names no Lot of the notice, never lot-null.
They are PARTNER-ONLY (`store::PARTNER_ONLY_AMOUNT_FIELDS`, `electable_amount_field`): every scale
rule sees them as lot figures of their lot, no election elects them, they never corroborate, and
they stay out of 492's lot sums and 505's tables. BT-118 / BT-1118 (the notice totals) stay
unmapped. Decision and census: `.scratch/tender-db/506-result-amounts/`. Drain, after the cohort
read predicts the moves (`cohort.py`): Pass A `refold-value-band` dry, wet, `project` (expected:
395737 falls to BGN 7,348,200, 627219 to €6,666,666.67, plus the adjudicated list; anything else
that moved is a finding); Pass C `refold-fields` over `DE1-FrameworkMaximumAmount` on
`notice_amounts` (the bare eForms-DE 1.x root framework maximum, aliased in the second 506 commit and
the one piece that can RAISE a head), sized with `expect:1`, then wet, every moved head read against its
notice; then Pass B `refold-fields` over the four field ids on `notice_amounts`, sized with `expect:1`
first (storage completeness: no served value may move — B assumes C has run, or a de-1.1 chain carrying
both shapes would rise inside B).

**The open head after issue 533 (`refold-open-head`).** `tenders.current_closed_by_notice` (the
closing rule, `canonical::closed_by_notice`) is fold-maintained and reads NULL on every row folded
before the deploy; only the open head (`current_deadline > now`, ~42k on 2026-10-14) can answer
`status` differently, since a past or absent deadline is Closed already. Re-derive it through the
fold (ADR-0017 R3; no SQL backfill, issue 375), right after the deploy (until it has run, an open
head folded before the deploy keeps its pre-533 answer) and queued after any running refold:

```sh
/root/aj.sh /admin/jobs '{"kind":"refold-open-head"}'                  # dry: counts Tenders and notices
/root/aj.sh /admin/jobs '{"kind":"refold-open-head","dry_run":false}'  # stamps and re-queues them
/root/aj.sh /admin/jobs '{"kind":"project"}'
```

Enqueue the wet run and its `project` **immediately after the deploy**, clear of the 07:35 UTC daily
chain (check `/admin/jobs` for a queued daily first), so every daily `fetch`/`process`/`project`
queues behind that `project`. Why: a pre-deploy open head whose rule already closes it holds a NULL
flag, so pages and the SSE head probe serve it open. A daily fold that appends another closing
version to it writes the flag but no correction row (the transition row compares the rule over the
prefix with the rule over the chain, never the stored NULL; a `project` that folds the stamp and the
append together skips the head-differs path for the same Tender), and SSE, reading the rule at
seq−1 and the flag at the head, closed on both sides, drops the row as `(false, false)`. A
`status=open` subscriber then keeps it open and is never sent `removed` (a `status=closed` one gets
`changed` where `added` was due), and the refold, reaching it later, finds the head equal and writes
nothing. Folded by refold-open-head's `project` first, the flip is a seq-less correction row, which
SSE delivers. The window is accepted, not fixed in the fold (step 0d: 153 open heads close under the
rule, each also needing a closing notice appended inside it); `closed_by_notice.rs`'s
`a_head_folded_before_the_deploy_reads_its_null_flag_until_its_next_fold` pins it.

Verify: the wet message's `stamped N` equals the dry count, and the `project` message's
`compare: N verified, M corrected` sums to it (D7's completeness; more if other Tenders were
pending). The flips are rule T/L correction rows (a lot closes only with its Tender, so a lot's
flip rides its Tender's). Then three field refolds, each sized first with `"expect":1` (the abort
message names the carrier count; nothing is written) and run with that count as `expect`:

```sh
# 1(b): past-deadline DÖE award notices stop serving their own date
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["SDK01-ProcurementProjectLot-TenderingProcess-TenderSubmissionDeadlinePeriod-EndDate"],"tables":["notice_dates"],"expect":1}'
# 2a fill, eForms: BT-766 and its national spellings -> `dps` rows (heavy)
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["BT-766-Lot","BT-766-Part","DE1-ProcurementProjectLot-TenderingProcess-ContractingSystem-ContractingSystemTypeCode","SDK01-ProcurementProjectLot-TenderingProcess-ContractingSystem-ContractingSystemTypeCode"],"tables":["notice_codes"],"expect":1}'
# 2a fill, r2.0.9: the DPS marker (K1's legacy leg) and 2b's two no-award reasons, Integer markers
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-DPS","TED-PROCUREMENT_DISCONTINUED","TED-PROCUREMENT_UNSUCCESSFUL"],"tables":["notice_integers"],"expect":1}'
```

Until the last two have run, `dps` rows and the `discontinued`/`unsuccessful` reasons are complete
only for the open head and for Tenders folded since the deploy (the sandbox notes say so); `status`
does not wait on them, since a past-deadline head is Closed by its deadline. Post-deploy reads:
issue 533's `### Unit 2 order`, step 5.

**Publisher-escaped titles and descriptions (issue 538 unit 3).** The fold decodes a publisher's
extra escaping in every `title` and `description` fact, Tender and lot, every era
(`ingest::project::escapes::decode_publisher_escapes`): numeric character references and the five
XML names (ASCII case-insensitive, so the text era's `&Amp;` too), repeated while one remains, at
most 3 passes; then a decoded `<br>` / `<br/>` / `<br />` becomes a line feed and a decoded
U+E000–U+F8FF becomes `•`. It changes a text only where a reference was decoded. The notice layer
is untouched (nothing is re-parsed), and so are organization names (deferred to the issue's unit 4,
behind 524's wet runs) and the other text fields. No `PROJECTION_EPOCH` bump (ADR-0017 R3): the
cohort is the carriers of the field ids the projection reads as a title or description, all in
`notice_texts` (`TEXTS` rows mapped to `title`/`description` in `crates/ingest/src/project.rs`, plus
`OJ_HEADING_FIELD`; a row added there joins this list):

- eForms (TED SDK 1.x, eForms-DE 2.x) and FTS: `BT-21-Procedure`, `BT-21-Lot`, `BT-21-Part`,
  `BT-21-LotsGroup`, `BT-24-Procedure`, `BT-24-Lot`, `BT-24-Part`, `BT-24-LotsGroup`;
- eForms-DE 1.x: `DE1-ProcurementProject-Name`, `DE1-ProcurementProjectLot-ProcurementProject-Name`,
  `DE1-ProcurementProject-Description`, `DE1-ProcurementProjectLot-ProcurementProject-Description`;
- DÖE sdk-0.1: the same four with `SDK01-` for `DE1-`;
- r2.0.8, r2.0.9 and internal-ojs: `TED-TITLE`, `TED-TITLE_CONTRACT`, `TED-CONTRACT_TITLE`,
  `TED-TITLE_QUALIFICATION_SYSTEM`, `TED-TITLE_DESIGN_CONTACT_NOTICE`,
  `TED-TITLE_RESULT_DESIGN_CONTEST`, `TED-TITLE_NOTICE_BUYER_PROFILE`, `TED-LOT_TITLE`,
  `TED-LOT_DESCRIPTION`, `TED-SHORT_DESCR`, `TED-SHORT_CONTRACT_DESCRIPTION`,
  `TED-SHORT_DESCRIPTION_CONTRACT`, `TED-NEW_VALUE.TEXT`, and `TED-TI_DOC` (the OJ heading, read as
  the title only when a notice maps none of its own);
- text era: `TXT-TI`, `TXT-TX`, `TXT-AB`.

Only a Tender whose title or description holds an `&` can move, but a field id says nothing about
the value, so the carriers are close to every notice of each era (nearly every notice publishes a
title). Size each family with `"expect":1` (the abort names the count; nothing is written), then
send it with that count as `expect` (the handler queues its own `project`), the eForms family first
since it carries the class:

```sh
# eForms-structured: TED eForms, eForms-DE 1.x/2.x, FTS, DÖE sdk-0.1
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["BT-21-Procedure","BT-21-Lot","BT-21-Part","BT-21-LotsGroup","BT-24-Procedure","BT-24-Lot","BT-24-Part","BT-24-LotsGroup","DE1-ProcurementProject-Name","DE1-ProcurementProjectLot-ProcurementProject-Name","DE1-ProcurementProject-Description","DE1-ProcurementProjectLot-ProcurementProject-Description","SDK01-ProcurementProject-Name","SDK01-ProcurementProjectLot-ProcurementProject-Name","SDK01-ProcurementProject-Description","SDK01-ProcurementProjectLot-ProcurementProject-Description"],"tables":["notice_texts"],"expect":1}'
# legacy TED and the text era
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-TITLE","TED-TITLE_CONTRACT","TED-CONTRACT_TITLE","TED-TITLE_QUALIFICATION_SYSTEM","TED-TITLE_DESIGN_CONTACT_NOTICE","TED-TITLE_RESULT_DESIGN_CONTEST","TED-TITLE_NOTICE_BUYER_PROFILE","TED-LOT_TITLE","TED-LOT_DESCRIPTION","TED-SHORT_DESCR","TED-SHORT_CONTRACT_DESCRIPTION","TED-SHORT_DESCRIPTION_CONTRACT","TED-NEW_VALUE.TEXT","TED-TI_DOC","TXT-TI","TXT-TX","TXT-AB"],"tables":["notice_texts"],"expect":1}'
```

Cost: if a count comes out at its eras' whole notice count, the cohort is those eras, and a `refold`
over their profiles is the same requeue without the `notice_texts` sweep; either way the `project`
behind it is a fold of millions of notices (the full fallback past `LEGACY_CLOSURE_CAP`; the 397
all-profile fold measured ~442 min), so run it in one quiet window, off the daily tick, and batch it:
these cohorts contain 533's three deferred field-refold cohorts above (BT-766 and DÖE award notices
are eForms-structured notices with a title, the r2.0.9 markers legacy ones), so the two drains are
one fold, and its `compare: … corrected` count is the sum of both changes. The cost is accepted
knowingly. A finder over stored text would be as complete and far smaller: the decode is the
identity on a text with no decodable reference and nothing else in the fold reads these values, so
the Tenders whose `tender_version_texts` title or description holds `&#` or one of the five names
(a windowed read on the reader pool), stamped and re-queued the way `refold-value-band` does, are
the whole cohort. It needs a new job arm, so it is not built while the batch with 533 holds: that
fold is owed anyway, though this cohort makes it several times larger (533's three are ~3.5 M
notices; the carriers here are close to all ~31.7 M). If the batch falls through, build the finder
rather than run this cohort alone. Expected for 538 alone: about 1–2 % of eForms Tenders corrected
(unit 1: 0.9–2.1 % per window), 0.2–0.35 % of r2.0.8 and r2.0.9 Tenders, about none elsewhere. Size
change rows by corrected Tenders, not one row each: a corrected Tender writes ADR-0017 D3's rows, one
`tender changed` (rule T) plus a `lot changed` per rule-L lot, which is every lot of its head version
when a head text moved, so a 10-lot eForms Tender writes 11. A corrected Tender that neither an `&`
in its stored title or description nor another change folded in the same run (533's, or any
deployed rule not yet refolded) explains is a finding. Verify, in order:
- D7's R3 completeness: each wet `refold-fields` message reports `stamped N`, the distinct Tenders
  of that job. The `project` messages' `compare: X verified, Y corrected`, summed, must be at least
  the largest N (more if other Tenders were pending). It would equal the sum of the N for disjoint
  cohorts, but these overlap (533's three lie inside the two families, and a Tender stamped by two
  jobs is folded once), so the decisive half is a bounded window read that finds no
  `projection_epoch = 0` residue. Until both hold the drain is not complete: a stopped `project`
  (it is stoppable) is re-run, not read as done.
- Issue 538's `## Verify` (8561858 prints `False Gara europea … modalità multilotto, per
  l’affida`), 8561112's description reads `souscription & la gestion`, 8561120's lot descriptions
  end in a line feed rather than `&amp;lt;br/&amp;gt;`, and `/v1/notices/45886825/content` still
  holds `&#224;` (the notice layer keeps what the XML's one decode left).
- Only after both: retire the `issue-538 refold` sentence of `/docs` and the OpenAPI through
  `RETIRED` (`crates/app/src/v1/docs.rs`).

**Legacy award blocks by their published lot number (issue 567, 2026-10-17).** `read_legacy_results`
(`crates/ingest/src/project.rs`, `LegacyLotNumbers`) puts a legacy award block on the lot section that
publishes the block's own `TED-LOT_NO` (else `TED-LOT_NUMBER`): the exact number first, then, only where no lot
publishes it exactly, a unique normalised one (`normalise_lot_number`: trimmed, a leading `Lot`/`Los`/`Lote`/`Lotto`
and a `nr`/`no`/`n°` after it dropped, leading zeros dropped, so `Lot 01` is `1`). A number two lot sections publish
names neither, and a number no lot publishes leaves the award on the Tender. The walker's `TED-ITEM` ordinal keys
the lots only in a notice where no lot section publishes a number; an award block that names no number is placed by
its own ordinal there, or where every lot section's number is its ordinal (the pre-567 placement, kept: in the
unit-1 sample 27 such blocks in 6 notices, the 19 whose award title names a lot on that lot). Before, ITEM and LOT_NO went into
one first-claim-wins map and an award's last reference won, so swapped, skipped or rotated numbering (a lot
section printing a number that is not its ordinal) could put a winner or value on another lot: 10.6 % of 2020–23
and 4 % of 2016–19 multi-lot legacy CANs (unit 1, `.scratch/tender-db/567-lot-mapping/`). Ten or more lot sections
are no trigger alone (a notice whose every lot prints its ordinal mapped right at any count), but there the store's
primary-key value order (`LOT-10` before `LOT-9`) decided which section won such a collision.
The rule reads no value order. The parsed layer is right, so this is a refold (ADR-0017 R3, no epoch bump).

The cohort is the `TED-LOT_NO` carriers: every notice whose mapping can move carries one, in the lot sections whose
numbers decide (swapped, skipped, rotated, or not their ordinals) or on the award block (a normalised match). R2.0.8
award forms carry no lot section, so nothing there resolves before or after, and `TED-LOT_NUMBER` stays out.
`refold-fields` sweeps field ids, not section kinds, so it cannot keep only the award-side LOT_NO: contract notices
and corrigenda that carry one ride along and compare `verified`.

```sh
# size: aborts naming the carrier count, writes nothing
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-LOT_NO"],"tables":["notice_ids"],"expect":1}'
#   → error "refold-fields aborted: N notices carry [\"TED-LOT_NO\"], expected ~1 — check the field ids (nothing was written)"
# wet, with that N; the handler queues its own `project` (do not queue a second)
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-LOT_NO"],"tables":["notice_ids"],"expect":N}'
```

- **Size (estimated 2026-10-17)**: about 1.4 M carriers (95 % ~0.96–2.0 M): 19 of a uniform 60 `ted-export-r209`
  notices carry `TED-LOT_NO` (random ids 18.0–23.8 M kept on profile, 132 public-API reads; 4,490,549 r209 notices):
  in lot sections (17), on award blocks (8), once in an F14 change block. Fewer Tenders are stamped than notices
  re-queued (550's 1,899,575 carriers stamped 1,530,259).
- **Cost**: the sweep reads `notice_ids` alone, on the reader pool. The `project` is about 550's (2531: 4.3 h for
  1.53 M Tenders), so 3–4 h: a corpus-scale writer, run in a queue gap clear of the 07:35 UTC daily chain and of a
  running refold; stoppable (issue 543), a stopped `project` is re-run, not read as done.
- **Expected moves**: the misattributed Tenders among the era's multi-lot CANs (6 of the 60 above, so ~450 k with a
  wide interval) at unit 1's 4–10.6 %: tens of thousands corrected (order 2–5 × 10⁴). A few gain a lot where only the
  normalised number matched (2 of 694 sampled CANs: awards numbered `01`, `06`…`09` against lots `1`…`72`).
  Nothing else moves in the 694 cached legacy CANs re-run through the port
  (`.scratch/tender-db/567-lot-mapping/new_mapping.py`): 51 blocks change, 45 from the wrong lot to the lot their
  LOT_NO names (the 15 notices below) and 6 from no lot to it. Each corrected Tender writes ADR-0017 D3's rows
  (`tender changed`, plus `lot changed` per rule-L lot).
- **The head value can ride along (rare)**: 492's framework exemption, 537's lot-sum refusal and 551's
  `lot-sum-vouched` sum the awards by `lot_key` (`LotSums::by_currency` keeps one award per lot), so a lot that
  carried two awards counted one and the right lot none. Where a procedure figure of EUR 1 bn or more meets such a
  collision, the head's `value` / `value_derivation` can flip by 10^k either way (kept ↔ refused, vouched ↔ not).
  Expect none to a handful; each rides on a 567 lot move, so the fresh 30 reads it as 567's. 551 unit 4b's shadow
  walk read the pre-567 lots: its B3 parity re-runs after this refold (551's own note).

Verify, in order:
- D7's completeness: the wet message's `stamped N` and the `project` message's `compare: X verified, Y corrected`
  summing to at least N (more if other Tenders were pending).
- The 15 misattributed notices of unit 1, each award on the lot its LOT_NO names (`curl -s
  https://tenders.zebreus.click/v1/tenders/<tender> | jq -c '[.lot_results[] | [.key, .lot]]'`; tender: the blocks
  that move, was → is):
  5984445 (19411290, 305919-2017): RES-9 LOT-10 → LOT-9 · 6352468 (20128178, 501322-2018): RES-2 LOT-1 → LOT-4 ·
  6439745 (20296603): RES-4 LOT-3 → LOT-4 · 6445785 (20451472, 258495-2019): RES-3 LOT-1 → LOT-4 · 6565774
  (20852084): RES-9 LOT-10 → LOT-9 · 6397695 (21072522): RES-2 → LOT-11, RES-6 → LOT-2, RES-7 → LOT-3, RES-10 → LOT-7,
  RES-14 → LOT-16, RES-15 → LOT-18 · 6980424 (21769762): RES-2 LOT-5 → LOT-8 · 6997164 (21871346): RES-4 and RES-6
  → LOT-9, RES-5 and RES-10 → LOT-13, RES-9 and RES-14 → LOT-7, RES-12 and RES-15 → LOT-5 · 7080411 (21923286): RES-4
  LOT-1 → LOT-4 · 7006083 (21943329): RES-1 → LOT-2, RES-2 → LOT-7, RES-3 → LOT-9, RES-4 → LOT-4, RES-7 → LOT-6 ·
  7590694 (23000792): RES-6 → LOT-7, RES-7 → LOT-6 · 7153808 (23074890): RES-7, RES-24 and RES-26 → LOT-3, RES-9,
  RES-20 and RES-31 → LOT-5, RES-10 → LOT-6, RES-11 → LOT-7, RES-13 → LOT-9, RES-17 → LOT-20, RES-19 → LOT-22, RES-23 →
  LOT-12 · 7693726 (23256169): RES-8 LOT-11 → LOT-8 · 7644019 (23319363): RES-2 LOT-1 → LOT-3 · 7697322 (23413021):
  RES-7 → LOT-7, RES-8 → LOT-8, RES-9 → LOT-9. Every other block of these Tenders stays where it was (45 moves in all).
- A fresh uniform 30 corrected Tenders: note the `/v1/changes` cursor before and after the `project`, draw 30
  salted-uniform `tender changed` rows with `version: null` between them, and read each Tender's
  `lot_results[].lot` against its award notice's `/v1/notices/<id>/content` (one GET per 1.5 s), each award block
  where the rule above puts it. The block's reference is its LOT_NO, else its LOT_NUMBER, else its ITEM; every value
  of it must name the same lot, or the block is on none.
  - Where some lot section prints a LOT_NO or LOT_NUMBER: the block's LOT_NO (else LOT_NUMBER) goes on the lot
    section that prints it exactly, else on the one lot section that prints it once normalised; on none where two
    lot sections print it or none does. A block that prints no number goes on the lot at its ITEM ordinal only
    where every lot section's number is its ordinal (normalised), else on none.
  - Where no lot section prints a number: the lots are keyed by their ITEM ordinals, and the block's LOT_NO (else
    LOT_NUMBER, else ITEM) is read against those, exact then normalised (RES-2 with LOT_NO 3 goes on LOT-3). A LOT_NO
    that no ordinal matches leaves it on none: there is no fall-back to its ITEM.

  Bar: 30 of 30. A wrong one, or a correction neither 567 nor another deployed rule awaiting its refold explains,
  is a finding.

**R2.0.8 awards by their contract notice's lot number (issue 568, 2026-10-17).** An R2.0.8 award form repeats
`AWARD_OF_CONTRACT` with a `LOT_NUMBER` and publishes no lot section, so 567's same-notice map is empty there and
every block resolved to no lot. `fold` (`crates/ingest/src/project.rs`, `match_inherited_lots`, with
`LegacyLotNumbers::inherited` beside 567's map) now resolves such a block against the lots the award's version
inherits from the notices folded before it, when the round is pushed and before `lots_awarded` reads the rounds,
on both fold paths (streaming and `BucketRow::to_notice_state`). The rule (`.scratch/tender-db/568-r208-lots/
decision.md`, steps 1–9):
- only a result with `lot_key` null and its own printed number (first `TED-LOT_NO`, else first `TED-LOT_NUMBER`;
  only a legacy block has that pair), and only in an award notice with no lot section of its own (one with lot
  sections keeps 567's resolution, its null included);
- against the inherited lots of kind `Lot` that print a number (the latest section's `TED-LOT_NO`, else
  `TED-LOT_NUMBER`, never `TED-ITEM`): the exact trimmed number first, the `normalise_lot_number` form only where
  no lot prints it exactly; one lot names the key, two or none leave the block on the Tender;
- G1: two or more numbered blocks that all normalise to one number, on a Tender whose inherited lots print two or
  more numbers, match none (5476747's CAN 18543084 prints `1` on all three); a single-lot Tender is exempt;
- G2: once a notice folded before the award republished an inherited lot under another normalised number, or
  without one, no later award is matched (a lot that printed none and gains one is not renumbered). Since issue
  570 it compares at the key the remap below puts the section on, so a swap the remap joins by number renumbers
  nothing; a lot kept at its position under a dropped or other number (an unnumbered section, a refused notice)
  still does;
- frozen: the key is set on the award's own version and carried as resolved; `published_lot_no` stays the
  block's; never the key's ordinal, `TED-ITEM`, a title, corrigendum prose, `CONTRACT_NUMBER` or the position.

The parsed layer is right, so this is a refold (ADR-0017 R3, no epoch bump). The cohort is the `TED-LOT_NUMBER`
carriers: every R2.0.8 award block that can move prints it, and so does the R2.0.8 CN lot annex it matches (those
CN-only Tenders ride along and compare `verified`). `TED-LOT_NO` stays out: 567's refold covered it, and R2.0.9
CANs keep their same-notice resolution.

```sh
# size: aborts naming the carrier count, writes nothing
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-LOT_NUMBER"],"tables":["notice_ids"],"expect":1}'
#   → error "refold-fields aborted: N notices carry [\"TED-LOT_NUMBER\"], expected ~1 — check the field ids (nothing was written)"
# wet, with that N; the handler queues its own `project` (do not queue a second)
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-LOT_NUMBER"],"tables":["notice_ids"],"expect":N}'
```

- **Order**: after 567's drain is read back (2589/2590), clear of the 07:35 UTC daily chain and of a running
  refold, and **before 551's B3 parity**: legacy award lot keys feed path W's sums, so a B3 run before this drain
  re-runs after it, for the reason it waits for 567.
- **Baseline first**: before the wet run, snapshot the ≥ EUR 1 bn value band and the path V `lot-sum-vouched`
  heads; read (e) below against them.
- **Size and cost**: carriers likely about 0.5–0.9 M, so the `project` probably falls back to FULL (issue 305's
  500 k closure cap): about 4.3 h, like 2531 and 2590. Stoppable (issue 543); a stopped `project` is re-run, not
  read as done.
- **Expected moves** (an assumption, not a measurement: 0.8–1.0 M R2.0.8 award notices): about 130–160 k Tenders
  corrected (100–195 k), 1.1–1.4 M award blocks from `lot: null` to a lot. The dry run's carrier count and the wet
  run's `stamped N` replace the estimate. Each corrected Tender writes a `tender changed` plus a `lot changed` per
  touched lot (5284511 alone about 140), order 10⁶ change rows in all: lot subscribers (SSE, webhooks) see the
  burst.
- **The head value can ride along**: 537's award-side scale refusal and 551's path V see R2.0.8 award lot sums
  for the first time, so a ≥ EUR 1 bn head can flip kept ↔ refused or vouched ↔ not. Expect a handful at most;
  each must be a 568 move (e). `LotSums` keeps one award per lot, so a lot with several awards sums its latest.

Verify, in order:
- (a) D7's completeness: the `project`'s `compare: X verified, Y corrected` summing to at least the wet run's
  `stamped N`, and a bounded window read finds no `projection_epoch = 0` residue.
- (b) The issue's Verify: 4748635 serves `[LOT-1, LOT-2, LOT-3, LOT-5, LOT-6, LOT-7, LOT-8, LOT-9]` (`curl -s
  https://tenders.zebreus.click/v1/tenders/4748635 | jq -c '[.lot_results[].lot]'`).
- (c) The 77 measured Tenders (the 71 of `.scratch/tender-db/568-r208-lots/port_result.json` and the 6 of
  `extra_port.jsonl`), through `/v1/tenders/{id}` at one GET per 1.5 s: every block on the port's lot (647 + 8);
  5476747's three, the 26 no_match, the 6 no_number and 4634188 RES-2 stay `null`; nothing else moved.
- (d) A fresh uniform 30 corrected Tenders from the `project`'s `tender changed` rows (note the `/v1/changes`
  cursor before and after; `version: null`), at least 10 from outside 2012–2015, each moved block adjudicated by
  title or figure against its notices' `/v1/notices/<id>/content`. **Bar: 30 of 30 with no wrong block**; one
  wrong reopens the decision.
- (e) The band and path V diff against the baseline: every head that moved is a Tender whose R2.0.8 awards gained
  keys, each read as 568's.
- (f) Status: R2.0.8 Tenders whose served `status` changed should be about none (533's `closed_by_notice` can flip
  in the projection where every result is a no-award, but 2010–2019 deadlines are past).

**568 unit 7's refusal guards (built in unit 8 to the amended rule, 2026-10-18).** The read-back failed on 8327340:
one R2.0.8 award block for several lots (`обособени позиции 5, 6, 15, 16`) prints one of them, and the rule above put
the whole award on that lot. `match_inherited_lots` now runs four guards (`unit7_refusals`,
`.scratch/tender-db/568-r208-lots/u7/decision-u7.md`, "The rule" as changed by its "Amendment (unit 8,
2026-10-18)", which wins, R1 and R2 of its round 2 included) over the blocks the rule matched, on both fold paths.
An R2.0.8 award whose own title names several lots, names another lot than both its printed number and the lot that
number finds, or carries another lot's title, or that prints several lot numbers, stays on the Tender:
- the evidence is the block's own `TED-CONTRACT_TITLE` / `TED-TITLE_CONTRACT` / `TED-TITLE` (every value, directly
  on the `RES-n` section, never its Organization children, never `CONTRACT_NUMBER`), **read per language** (the
  value's `lang`, `""` for none), and its own lot references, read by `read_legacy_results` (`AwardText`: the
  designators `lot_designators` reads per language, the `title_key`s, and whether it prints several numbers), kept
  per result key on `NoticeState.award_text`, which rides the bucket frame (`BucketRow.award_text`) and is never on
  the round: fold input only, nothing new is stored;
- N is the set of numbers the inherited `Lot`s print (normalised); **with fewer than two, no guard fires**; Dn is a
  language's designators whose head is in N;
- G4, block level: a matched block **contradicts** when some language names a lot in N and no language names its
  printed number as a head or **shares a head** with the designators of its own lot's `title`/`description` (R1: a
  compound counts by its head, so `Pakiet 16 zad.2` agrees with a lot described `Pakiet 16 - pozycja nr 2`). It
  stays on the Tender. Contradictions are counted over every matched block first (a G3 or G6 block counts). **With
  two or more** the notice numbers awards by a contract counter: every other matched block stays on the Tender too,
  **except a title-confirmed one** (every language naming a lot in N names exactly its printed number, and only
  plain designators: R2, a compound such as OOS 5416534 RES-16's `Pakiet 16 zad.2` printed 16 confirms nothing).
  One contradicting block refuses only itself; a block that matched no lot never counts (5218711's 5/6/7). R1's
  recorded cost (risk 4): a block naming another item of its lot's own pakiet (5267810's `Pakiet 8 poz. 1` printed
  9 on LOT-9 `pakiet 8`) agrees at block level and is held only by its notice's other contradictions;
- G6: the block prints two or more numbers (5001424's `LOT_NUMBER` 1-6); G3: one language names two or more
  distinct heads in N (a compound counts by its head: `Pakiet 8 poz. 6, Pakiet 8 poz. 7` names 8); G5: none of its
  title keys is its lot's, and one that is not the Tender's is exactly one other numbered lot's (4854639's
  `Snagging` on LOT-4, 4975428 RES-2). The title key is the reference's exactly: lower-cased, NFKD with combining
  characters dropped (`title_key_nfkd`, generated from Python's `unicodedata`: `ł`, `ß`, `ø` stay, `й` → `и`), one
  leading designator dropped only where `. : ) - – —` follows it (`Pakiet 2` keeps `pakiet 2`);
- the reader (`lot_designators`, amended): only a tight dash joins numbers (`Część 1 – 50 miejsc` names 1); `Los`
  is a lot word only capitalised or upper-case (Spanish `de los 2`); a compound takes `zad.` too and a list
  (`Pakiet 16 zad.1`, `Pakiet 8 poz. 6 i 7`: one designator each); `pakiet 8 pozycji` is a count; a position word
  after another clause is none; a list's last number before a quantity word (`szt`, `ans`, `lat` …) is dropped;
- a refused block keeps `lot_key` null and its `published_lot_no`, and is never re-keyed to the lot its title names;
  frozen at the award's version like the rule above. 567's same-notice keys, eForms, FTS and 570's remap are
  untouched (the guards run only where `match_inherited_lots` does: an award notice with no lot section).

Measured (decision-u7, amendment with R1 + R2): in sample 11/13 multi-lot and 53/61 other-wrong blocks caught, 0
false refusals in 1,762 (≤ 0.21 %), the 77 measured Tenders untouched; out of sample (`u8-oos/`, 50 Tenders) 100/100
caught, 0 of 319 (≤ 1.15 %). The residual: 5 of 230 uniform Tenders keep a wrong block (2.2 %, 0.71–5.00 %), 2 of
them 568's own (4854639 RES-9/10, 4968281 RES-1: 0.87 %, 0.11–3.1 %), 3 inherited (4861483 issue 364's weld,
5364153 on its parent lot, 5267907 identical placeholder lots). The build equals the amended port (`g3_port.py`
`apply_amended`, its defaults) on every block of every cached chain, full notices folded through both paths: 2,094
in-sample + 432 out-of-sample blocks, of which the port matched 1,837 + 419 (64 + 100 refused) and 240 + 1 neither
side matches, 0 divergences; the remaining 17 + 12 sit on award notices with their own lot sections (567's
same-notice keys), which the guards never move. `lot_designators` and `title_key` equal the reference
(`u7/lot_designators.py`) on all 53,428 distinct text values of those caches. **Refold: ONE refold together with
570**, over `TED-LOT_NO` + `TED-LOT_NUMBER` (the 570 block's commands below; every block a guard can refuse sits
on a notice carrying one of them). Baseline first (`drain/baseline-pre568u7-570.json`: the ≥ EUR 1 bn band and the
16 path-V heads). Expect about 7 k Tenders touched (3–15 k), about 3.1 k fully back on the Tender (0.6–8.9 k),
about 59 k blocks back to `lot: null`; 8327340-style lot sums shrink back (709,431.63 BGN off its LOT-5).

Verify, after that refold (decision-u7 unit 11 as amended), beside 570's own items:
- (a) D7 as above; (b) 8327340 serves `lot: null` on 17821481 RES-1/2/5/6/7 and 17830774 RES-1/2, and 17821481
  RES-3 → LOT-24, RES-4 → LOT-32, 17840150 RES-1 → LOT-2;
- (c) the salted 150's 57 refused blocks serve `null` (5051007 RES-4..19, 5267810 RES-2..32, 8364864 × 2,
  5020611 × 4, 5001424 × 1, 4854639 RES-7/8, 4975428 RES-2), while 5051007 RES-1..3 keep LOT-1..3 and 5267810
  RES-1 keeps LOT-1; its other 915 matched blocks unchanged except 570's named movers;
- (d) the 77: 692 of 692 unchanged (570's port names any 570 mover);
- (e) **the combined drain's fresh 30**, drawn before any id is read: salt `sha256("568u8-fresh30-"+id)` ascending
  over the combined wet run's changed set (its `version: null` `tender changed` rows in the cursor window),
  excluding the salted 150, the first read-back's fresh 30 (`readback/fresh30.json`), the 77, 8327340 and the out-of-sample 50
  (`u8-oos/draw.json`); strata: **at least 10 outside 2012–2015, and at least 8 changed Tenders outside
  `readback/population.json` that serve an R2.0.8 block on a lot** (the blocks 570's narrower G2 newly matches,
  never sampled). **Bar:** (i) parity: every block's served lot equals the amended port's replay with 570's remap
  composed in (`../570-lot-keys/remap_port.py` option B in `g3_port.replay`'s carry), 0 divergences; (ii) at most 2
  of 30 Tenders carry a judged-wrong block the port also keeps, of any shape; (iii) a wrong block of a new shape is
  recorded and filed, not a reopen: reopen only on a parity break or on 3 or more of 30. P(pass of (ii)) is 0.973 at
  the pooled 2.2 % (0.812 at its 5.0 % upper bound; 0.950 at the in-sample 5/180). The G2 stratum, the out-of-era 10
  and a same-pakiet offset notice without other contradictions (R1's risk 4) are what (ii) would detect;
- (f) band / path V diff against the baseline: each mover read as 568-u7's (a lot sum shrinking back) or 570's.

**Legacy lots joined by printed number across a chain (issue 570, 2026-10-18).** TED's legacy notices key their lot
sections `LOT-k` by position in the notice, and the fold merged a Tender's lots, and put each notice's award blocks,
by that key. Where two notices of one Tender list their lots differently (a CAN listing a subset, packages split
across two award notices, a swap), one key held two lots: 8375276's eight 2018–2020 award notices put 147 blocks
on the other lot. `fold` (`crates/ingest/src/project.rs`, `legacy_lot_remap` beside `match_inherited_lots`, with
568's `LegacyLotNumbers::inherited` / `lookup`) now maps each later legacy notice's lot sections onto the Tender's
existing keys by printed number before the lot carry, on both fold paths (streaming and
`BucketRow::to_notice_state`; `NoticeState.legacy` rides the bucket frame). The rule
(`.scratch/tender-db/570-lot-keys/decision.md`, steps 1–9):
- only a legacy notice (`is_legacy_profile`) with lot sections, folded after an earlier notice published some; the
  first lot-publishing notice (the anchor) keys its lots by its own section ids, and no key is ever renamed or
  invented (every key is a carried one or the section's own id);
- a `Lot` section printing a non-blank number goes to the one carried `Lot` printing it (`published_lot_no`, the
  latest section's `TED-LOT_NO`, else `TED-LOT_NUMBER`): exact trimmed first, `normalise_lot_number` only where no
  lot prints it exactly; any other section (no number, no match, ambiguous, not a `Lot`) keeps its own key;
- the section's facts, kind and number are carried at its target; its place country is read by its own key and
  applied there; its round's `lot_results` and `bids` keys (567 resolved them to the notice's own sections) follow
  it; `published_lot_no` is never touched;
- K1: two sections landing on one key (a number printed twice, or a matched section landing on the key another
  keeps) leave the whole notice on its own keys, as before 570; K2: so does a numbered section that matches nothing
  where a carried lot printing another number sits (8433088's `1` against the CN's `1-4`, 6016286's `2.` against
  `2`); K3: no notice is remapped once a non-legacy notice has published a lot section (eForms `LOT-0001` and TED
  `LOT-1` stay apart, so 551's G8b input is unchanged); K4: nothing is remapped where no carried `Lot` prints a
  number (7025760, 7465548);
- 568's G2 reads the remapped carry (above); frozen: a notice's map reads only the notices folded before it.

The parsed layer is right, so this is a refold (ADR-0017 R3, no epoch bump). Lot key strings and lot ids are
unchanged on every replayed chain (0 minted or swept in 211, `port_result.json`); the content under some ids moves
back to the identity their first notice gave them. The cohort is the `TED-LOT_NO` and `TED-LOT_NUMBER` carriers:
every remapped section prints one, and so does every anchor lot it can match (`TED-LOT_NUMBER` adds only the R2.0.8
chains with two lot-publishing notices, 0 of 71 in 568).

```sh
# size: aborts naming the carrier count, writes nothing
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-LOT_NO","TED-LOT_NUMBER"],"tables":["notice_ids"],"expect":1}'
#   → error "refold-fields aborted: N notices carry [...], expected ~1 — check the field ids (nothing was written)"
# wet, with that N; the handler queues its own `project` (do not queue a second)
/root/aj.sh /admin/jobs '{"kind":"refold-fields","profiles":["TED-LOT_NO","TED-LOT_NUMBER"],"tables":["notice_ids"],"expect":N}'
```

- **Order**: after 568's drain is read back (2593/2594), in a queue gap clear of the 07:35 UTC daily chain and of a
  running refold. **Before 551's B5 flip**: path W's R2 reads `LotState.published_lot_no` at the round notice's
  version, and the port (`unit4b/gen_fixtures.py`, `rule_v2.py`'s `pub_no`) assumes a served key is the notice's
  own section id, which a remapped notice breaks (7511890's 22980479 RES-1 serves on LOT-3, whose own LOT-3 prints
  6): both apply `remap_port.remap`, a B3 run before this drain re-runs after it, and B5 waits for both.
- **Baseline first**: before the wet run, snapshot the ≥ EUR 1 bn band, the path V heads and the 211 port Tenders
  re-fetched after 568 (`drain/baseline-pre570.json`); read (c), (d) and (f) below against it.
- **Size and cost**: about 2.1 M carriers (567's ≈ 1.4 M `TED-LOT_NO` plus 568's dry 2591, 719,732
  `TED-LOT_NUMBER`), so the `project` falls back to FULL (issue 305's closure cap): about 4.3 h, like 2590.
  Stoppable (issue 543); a stopped `project` is re-run, not read as done.
- **Expected moves** (95 % intervals, 204 sampled multi-lot legacy CANs): 3.9 % (1.7–7.6 %) of multi-lot legacy CANs
  change, about 22 k Tenders (9–42 k; the wet run's `corrected` replaces the estimate); 2.0 % of their served
  results move. Each corrected Tender writes a `tender changed` plus a `lot changed` (`version: null`) per head lot,
  ≈ 250 k rows (100–500 k). A ≥ EUR 1 bn head can ride along where a moved award changes a lot sum (path V, 537,
  492); each must be a 570 move.

Verify, in order:
- (a) D7's completeness: the `project`'s `compare: X verified, Y corrected` summing to at least the wet run's
  `stamped N`, and a bounded window read finds no `projection_epoch = 0` residue.
- (b) The issue's Verify: 8375276 serves LOT-1 "Lahve a savičky" and LOT-2 "Kojenecká voda", every block whose
  notice printed LOT_NO 1 on LOT-1 and every block printing 2 on LOT-2 (147 moved: 120 + 27).
- (c) The 8 changed sample Tenders and the 5 changed exhibits of `.scratch/tender-db/570-lot-keys/port_result.json`:
  every block on the port's B lot (32 + 207), lot key sets unchanged; 8433088, 6016286, 7465548, 7112657, 5939039,
  5317587, 5540787, 6120045 and 7025760 equal the baseline.
- (d) The other 196 sample Tenders equal the baseline (one GET per 1.5 s).
- (e) A fresh uniform 30 corrected Tenders from the `project`'s `version: null` `tender changed` rows in its cursor
  window, drawn before any id is read (`sha256("570-fresh30-" + id)`): each moved block on the key whose head number
  equals its own, each moved section's title agreeing with its target (or untitled), no lot added or removed.
  **Bar: 30 of 30**; one wrong reopens the decision.
- (f) The band and path V diff against the baseline: every head that moved is a corrected Tender, read as 570's.
- (g) `lot removed` rows in the window attributable to 570: expected 0.

Job payloads (`crates/app/src/supervisor.rs`, `JobRequest`): `{kind:
fetch|process|project|backfill|daily|reprocess|reindex|analyze|refold|refold-fields|
refold-notices|refold-sections|reparse|data-quality|backfill-titles|mark-skipped-siblings|clear-rebuild-flag,
source?, package_kind?, period?, range?, rebuild?, refetch?, profiles?, notices?,
expect?, dry_run?}` (the `snapshot` kind was removed 2026-08-06).
Defaults: `source` `ted`, `package_kind` `daily`.
`process`/`project` accept no period to run the whole source (`process` with no
`period` re-parses every archived package of that source). `refetch:true`
re-downloads a known package (finality re-check). `project` with `rebuild:true`
drops and re-derives the whole canonical layer. `backfill` needs a `source`; for
`ted` it also needs a monthly `range` (`["2024-01","2024-12"]`), for `doe` the
range is optional (defaults to the whole 2022-12→now archive), and for `fts`
it is optional too (defaults to 2021-01→the previous UK month; a range reaching
into the running month is refused, because a registered monthly is never
re-walked and the daily probe, which fetches every day after the newest monthly
that holds no daily, covers it — issue 477; a single FTS `fetch` of a day or
month that has not ended in UK time is refused for the same reason). A backfill fans
into one `fetch` per month, then one whole-source `process`, then one `project`,
so progress and cancellation stay per-package. Jobs run **one at a time** in
enqueue order — the writer is single anyway — so a fetch → process → project
sequence lands in order.

### The FTS id audit and `audit-fts-ids` (issue 477 unit 3)

FTS notice ids are a per-year sequence, `NNNNNN-YYYY`, so a year's highest id held says how many
notices the year has issued. Every id below it that no notice row carries is either **lost by a fetch**
(the API serves it by id) or **never published** (the API answers 404 or a package with no release:
an id issued and withdrawn before publication). The 2021-01 … 2026-08 top-up left 1,180 such ids on
2026-10-03. `audit-fts-ids` asks the API for each one by id
(`GET /ocdsReleasePackages/<id>`; the 2026-10-04 run found all 1,180 absent, list in
`.scratch/tender-db/477-fts/absent-ids-2026-10-04.txt`; **the weekly Sunday tick re-runs it** wet with
`max_ids: 300`, last behind the scans, so each week's new never-published ids are accounted for) and records the answer in the `publication_audit` table, one row
per id:

| verdict | answer | what happens next |
|---|---|---|
| `absent` | 404 / 410, or a package with `releases: []` — recorded only while a held id answers (the control, below) | leaves the denominator; a current-year absent is re-asked once 30 days old, a closed year's only with `recheck_absent_days` |
| `present` | a 200 carrying a release of the id; `published` / `published_day` / `ocid` recorded | the report lists the packages to refetch for it; never re-asked (a refetch recovers it, not a probe) |
| `error` | anything else: a 200 with an **empty body** (FTS sent three for `04196f`'s existing record, unit 1b), a 5xx or other 4xx after the retries, a throttle that outlived them, a transport error, a body that is not a release package, releases of other ids only, or an absent a failed control demoted | asked again on the next run |
| `quarantined` | not asked: an archived member the processor quarantined carries the id (`NNNNNN-YYYY[~hash].json`) | accounted for (the fetch did not lose it) and still in the denominator; never asked; a reclaim makes it held |

- **Held ids come from the DB, not the zips**: `notices.publication_id` of source `fts`, an index range
  of the identity key. A member quarantined without a notice row is read from `quarantine` (one scan
  per run, never by the dashboard) and recorded `quarantined` without a request — as `present` it
  would never close, because the refetch dedups by hash and adds no row. A `present` id that survives
  its refetch is worth checking against `quarantine` by member name.
- **A 404 counts only while a known id answers.** Before the first request, after every 50, and once
  more at the end when absents were recorded since, the run asks the year's highest HELD id. If it does
  not answer `present` (a wrong base, an API path change, an outage page served as 404) the run halts
  and the absents since the last passed control are demoted to `error`. ~2 % more requests.
- **Cost and pacing**: one request per due id on the process-wide FTS clock, 12 s apart (shared with
  any FTS fetch). ~1,200 ids ≈ 4 h, and jobs run one at a time, so **run it capped**: `max_ids: 300`
  is ≈ 1 h; enqueue it well clear of the 07:35 UTC daily chain and repeat until `due` is 0. A cancel is
  read before each request, but a throttled request can retry for up to ~10 min (5 × 120 s) first.
- **Wet by default.** It writes only its own ledger and report (no notice, Tender or org), and a dry
  run that asked the API would spend the same hours to discard the answers. `dry_run: true` counts what
  is missing and due **without a request**, which is the cheap first look.
- **Stoppable and resumable.** It reads the stop flag before every request. Every answer is written
  as it arrives, and a run asks only ids with no row or an `error` row (plus stale absents), so a
  cancel, a crash or a restart loses nothing and the next run carries on. Five consecutive `error`s,
  or a failed control, **halt** the run: the job FAILS with the reason, the report is stored, and a
  re-run resumes.
- **Nothing is enqueued.** A present id's day cannot always be located: the API's windows select on a
  hidden publication instant, not the release `date` (`009921-2021`, dated 2021-07-27, is listed on
  2021-05-07). So the report names, for each present id, the packages that hold its nearest held
  neighbours (where it was listed) plus the package of its release day (the monthly while monthlies
  reach that month, else the daily), and `enqueue` carries the exact bodies: one `fetch` with
  `refetch:true` per package, one `process` per kind, one `project`. A month is 82–390 paced requests,
  so read the list before enqueueing it.

```bash
# The cheap first look: missing, quarantined and due per year, no request, nothing written.
/root/aj.sh /admin/jobs '{"kind":"audit-fts-ids","dry_run":true}'
# The audit, capped (≈ 12 s per due id, 300 ≈ 1 h); repeat until the report's `due` is 0.
/root/aj.sh /admin/jobs '{"kind":"audit-fts-ids","max_ids":300}'
# Re-ask every year's absents last asked ≥ 30 days ago (the current year's are by default).
/root/aj.sh /admin/jobs '{"kind":"audit-fts-ids","max_ids":300,"recheck_absent_days":30}'
# The report: the invariant per year, then the residue.
/root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq '{missing, due, probed, absent, present, errors, quarantined, controls, stopped, halted, unaccounted, complete}'
/root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq -r '.years[] | "\(.year) highest \(.highest) held \(.held) absent \(.absent) quarantined \(.quarantined) present \(.present) error \(.errors) unchecked \(.unchecked)"'
/root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq -r '.present_ids[] | "\(.id)  \(.published_day)  \(.packages | join(", "))"'
/root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq -r '.absent_ids | length, .[]'
# The recovery. FIRST read the bodies (each fetch is a month's 82–390 paced requests):
/root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq -r '.enqueue[]'
# Then, only if every line is wanted, enqueue them in order; `|| break` stops the chain at a
# refused POST, so a refused `fetch` is never followed by its `process` and `project`.
/root/aj.sh /admin/reports/audit-fts-ids | jq -r .body | jq -r '.enqueue[]' > /tmp/fts-refetch.txt
while read -r body; do /root/aj.sh /admin/jobs "$body" || break; done < /tmp/fts-refetch.txt
```

**The dashboard reads it.** The coverage grid's FTS cells and the import funnel's FTS row carry the
id-based denominator: `published` = each year's highest id held less its absent ids, the ratio is the
year's distinct ids held over that (an id with several releases counts once), and the funnel shows
` · N ids unaccounted` and denies `fetch complete ✓` while any id below a year's highest is neither
held, quarantined nor shown absent. The Held column of an FTS cell is the distinct ids of that id-year
(not notice rows, which repeat an id per release and are bucketed by package year), so Held, Published,
the ratio and the era header all divide ids by ids. New never-published ids appear daily, so the count
climbs between audits; a re-run of the audit (it asks only the new ones) brings it back to 0.
**`complete` is bounded by the highest id HELD, not the highest issued**: for the current year the ids
above it are the daily probe's; for a closed year (2021–2025) ids a fetch lost after the year's last
held id — its final days — are invisible to this check. Nor can it see a second release of a held id.

### Reading a `project` job's `counts` line (issues 318, 364, 448)

A finished `project` writes one summary line to its job row (`GET /admin/jobs` →
`recent[].counts`), and the run's gate tallies ride it. The service's stderr does
reach the journal (the `[project] …` heartbeats are `eprintln!`; issues 61/63 once
said otherwise), but the journal is size-capped and rotates, while the job row is
durable and is what `/admin/jobs`, `jobwatch` and the issues' Verify lines read:

```
<n> notices → <t> tenders (<i> islands), <v> versions; <w> tenders written, <u> verified unchanged, <r> relabelled (issue 562)
; <the issue-318 genericness-wall suffix, when the wall was consulted>
; issue-448 alias asked A bound B refused R (poisoned …, no owner …, veto …, names …, generic …)   ← only when the altid alias was asked about a company-number merge
; issue-469 register alias asked A bound B refused R (poisoned …, no owner …, shared …, veto …, names …, generic …)   ← only when it was asked about a register merge
; issue-364 previous-publication citations: N admitted, M refused (prior-information …, buyer-profile …, periodic-indicative …, qualification-system …, DPS …, undeclared …, unknown kind …)
; issue-364 edges refused by the cited notice's own type: K (prior-information …, buyer-profile …, periodic-indicative …, qualification-system …, DPS …, unknown kind …)
; issue-481 tender links joined: J (previous-notice … of which cross-source …, logical-notice …, matched …); refused: R (not-earlier …, fan-in …, not-one-to-one …, keyed-weld …, oversized …); deferred: D; largest component: L key(s)
; issue-385 F14 corrigendum dates: …
; issue-479 procedure type: eforms F folded / U unmapped / N none, r209 …, text …   ← one entry per profile family the run planned
```

`<r> relabelled` is printed on every line, 0 included. It counts the Tenders whose `source` or `kind` the fold
moved in place (a DÖE Tender turning `ted` when its TED twin joins) and announced with ADR-0017 D3's seq-less set,
rule T plus every lot of the head version: ordinary folds, the unchanged-chain early return, and stale refolds
under `TENDER_REFOLD_COMPARE=off` or `shadow`. A stale refold under the acting compare (`on`, the default) announces
its label moves inside the compare's own correction set instead, so they count under the compare tally's
`corrected`, not here: an `on` refold that moved labels reads `0 relabelled`. Issue 562's post-deploy paired read
checks this number against the day's seq-less `tender changed` rows.

The two issue-364 lines are two different gates and read differently:

- **`previous-publication citations`** is the read-time gate — what the CITING
  notice declared its citation to be (the r2.0.8/r2.0.9 `.PREV_KIND` rows). About
  19 % refused is the measured, expected shape; the same total arriving as
  `undeclared` means an era publishes a slot the gate has not been taught. Absent
  when the run planned no legacy citation at all.
- **`edges refused by the cited notice's own type`** is the grouping-time gate —
  what the CITED notice IS by its own document-type code (`TXT-TD`,
  `TED-TD_DOCUMENT_TYPE`, `TED-NAT_NOTICE`). It is the only line that can count
  the text era, whose `TXT-RN` declares no kind, and it is absent when nothing was
  refused. A resumed grouping (fold index already on disk) reports nothing here:
  the refusals happened in the run that built it.

`unknown kind` non-zero on either line is a vocabulary gap in the code, not a
corpus fact — look at it before reading the split as right.

The **issue-448 alias** line counts PPON-first mentions of suppliers the altid arm merged
(`match-org-identifiers` rule `altid`): `bound` went to the company-number org, `refused`
minted or bound as they would without the alias, by cause. `names` is the one to read
first: a supplier renamed since the merge, or a publisher writing another company's name
beside the PPON. A bind the generic wall could not check (`… with the generic wall unable to
answer`) went through leniently. The diag log's `[issue 448]` lines say the same on every
fold, zeros included, with how many aliases the fold was armed with. Since issue 518 the
bar's name corroboration reads the owner's MENTION names (its newest 4,096 mentions, at most
256 distinct names, read once per owner per fold) beside its designated names, as the
planner does, so a PPON-first spelling the merge was corroborated on no longer refuses
`names`; the finish line's `owner mention names read for N owner(s), M cut by the cap` says
how often the cap binds. The consortium/legal-form `veto` still reads the designated names.

The **issue-469 register alias** line is the same alias for PPONs a REGISTER merge folded
(a UKPRN, charity, MPR, SC or NIC org kept, issue 469 U5): `bound` went to the one org
carrying the ledger's exact (`GB`, kind, identity) triple. `shared` means that triple
stands on two or more rows at the fold's open, or under a `wrong`/`related` identifier
verdict: the alias never picks one, and the mention mints. `no owner` means no row carries
the triple — the register org was merged away since, a rebuild met the PPON-first notice
before the register-first one, or the normaliser changed. A PPON the ledger merged into two
targets of any kind (two company numbers, two identities, or one of each) is `poisoned`.
Stop rule S5: register refusals above register binds halt further register wet runs (an
alias defect); the next dry plan's `registry_regrown` names the pairs that split again.

The **issue-513 nhs-body alias** speaks in the diag log only (not on the job row — the fold's
`Report` copies sit in `run_project`'s poll-frame budget, issue 467): `[issue 513] resolver
nhs-body alias ARMED …` at the fold's open (keys armed from the `nhs-body` ledger, rows a
`keep` verdict dropped, keys poisoned by two bodies, keys owned by a standing identifier-NULL
row outside the body and so not armed — D4), and `[issue 513] resolver nhs-body alias: …` at
its close, zeros included: reviewed-spelling mentions `asked`, `bound` to the standing
survivor, bound to a survivor `minted` from the ledger (a rebuild meeting the body's uncoded
history first), `refused` (poisoned key, shared/withheld PPON target) and bound by 234 as
without the alias, and the orgs the `GBNHS<code>` was re-recorded on. Stop rule S4: refusals
above binds halt the campaign; the next dry run's `nhs_body_regrown` must hold no `name`, `ppon` or
`identity` entry (since issue 519 U4 an `identity-stray` entry is listed apart, never a halt: the job
line reads `… N keys regrown (S4), M identity-stray listed (no halt)`, and N counts the halting kinds only).
Since issue 519 U4 two `[issue 519]` lines sit beside them, zeros included: `… child triple arm
ARMED …` at the open (child triples and their arms, one per code that folded the child, from the
identity-loser ledger rows; triples poisoned by a broken anchor chain; or `NOT armed` when the
fold's rules carry no child gate) and `… child triple arm: …` at the close — child-coded mentions
`asked` (their identifier IS a folded child's `GBNHS<C>` triple), `bound` to the survivor standing
or to one `minted` from the ledger, and refused by name (no arm's `child_check` admits the name),
`ambiguous` (two do), withheld child triple, or shared/withheld PPON target — each refusal resolved
as without the arm, so a mis-publisher stands on its own `GBNHS<C>` row (S4 `identity-stray`).

The **issue-481** line is the grouping's Tender-link step (the `tender_links` ledger below;
ADR-0003's 2026-10-02 amendment, ADR-0011). `joined` counts links that put two group keys into one
Tender, per rule: `previous-notice` (OPP-090, ADR-0011's edge, which now resolves a DÖE citation of
a TED number too — those are the `cross-source` part, a population ADR-0011 never measured),
`logical-notice` (a TED eForms notice's BT-701 is the id DÖE publishes the same notice under) and
`matched` (a reviewed match, issue 481 unit 3). `refused` are the guards: `not-earlier` (an OPP-090
naming a notice that is not strictly earlier), `buyer-disjoint` (issue 481 unit 2b: an OPP-090 whose
citing and cited notices both name buyers and share none — a copied placeholder number that happens
to be a real notice of another buyer, like `00123456-2026`, SCB's notice; compared on each buyer's
identifier AND names, widened to one buyer's measured spellings (see "What overlaps" below), notice
against notice, and a notice with no parsed buyer never refuses), `fan-in` (cross-Source OPP-090s from two or more
procedure-keyed components into one TED notice: a PIN several procedures cite, or a colliding key),
`not-one-to-one` (one logical id carried by notices of two components, or one notice matched to
notices of two), `keyed-weld` (a same-notice or matched link would put two procedure-keyed Tenders
together: issue 482's colliding BT-04s already weld, and a new rule must not add to it) and
`oversized` (past 64 components). All but `not-earlier` and `buyer-disjoint` are expected at zero
or near it on well-formed data, so read any non-zero one before the joins. `buyer-disjoint` is
copied placeholders, plus whatever the token comparison gets wrong: a buyer renamed between notices,
or written with no identifier in common and a name that differs beyond case, punctuation and
accents. The backfill's `buyer_disjoint_samples` show which. `deferred` counts links the
incremental fold could not judge yet: one its plan held only one end of (the far end is re-queued),
a guarded join beside such an end, and — until the ledger is attested complete (below) — every
fan-in- or weld-guarded join. A non-zero count is a join one fold late, never a lost one and never
one a full fold would refuse; after the attestation the link closure keeps it at zero. `largest
component` is the biggest component the step built, in group keys: the check for an issue-482 hub
welded through previous-notice links, which have no cap — a jump from single digits is the thing
to look at. The line is absent when nothing joined, was refused or deferred.

### Routing a re-derivation: the cheapest complete route first (ADR-0017 D6/D7)

Before reaching for a refold, take the FIRST route that applies. Issue 490's epoch-4 refold took 11 h
and 70.6 M change rows; its value could have been an in-place backfill (issue 495).

- **R0, deploy only.** The read path, serialization or performance changed.
- **R1, requeue only.** The fix moves grouping or chain membership; the chain compare catches it.
- **R2, in-place backfill** (no fold, no planning). Only when ALL hold:
  - E1: the value is a pure function of the stored canonical rows, reference tables
    (`currency_rates`) and the causing notice's own value rows. No resolver, section or mention
    state (484's `is_buyer` fails this).
  - E2: one implementation. The walk calls the fold's own function, injected across the crate
    seam as 340's `normalize_lang` is, and a fixture pins fold(new) == fold(old) + backfill.
  - E3: no entity or version is minted or removed.
  - E4: every stored value derived from the moved one is re-derived too, or its Tenders are stamped
    and requeued (as `rederive-eur` does).
  - E5: version N depends only on versions 1..=N.
  - E6: the fold writes the same value from the deploy on.

  Mechanics: tender-PK windows, one `BEGIN IMMEDIATE` and one checkpoint per window, a
  `projection_state` watermark and a completion flag. A walk that moves a served value writes ADR-0017
  D5's correction rows in its window transaction. The template, with its checklist, is the module doc
  of `crates/store/src/inplace.rs`. Its worked example is `rederive-eur` (issue 495 unit 5), whose E2
  fixture is `a_rederive_eur_walk_then_its_fold_equals_a_fold_under_the_new_rates`. Precedents: 340,
  306/375, 371; 490's lot value would have qualified.
- **R3, scoped-stale refold** (requeue + stamp 0, no epoch bump): the fold output changes for a cohort
  a finder can enumerate (`refold` by profile, `refold-fields`, `refold-sections`, `refold-notices`,
  tender ids). 479 and 484 were this.
- **R4, epoch bump + all-profile refold.** Only when no finder can enumerate the cohort, or
  completeness must survive a rollback below the deploy or a restore from an older backup (the epoch
  is the one marker that lives in code). Batch corpus-wide fixes into one fold.

**Completeness by route.**

- **R4:** the epoch in bounded 100k-id windows (as in 490's runbook below).
- **R3:** the stale-0 residue in the same windows. Not restore- or rollback-proof: after either,
  re-run the scoped refold.
- **R2:** the watermark at `MAX(id)` plus the completion flag, which the read path gates on. After a
  rollback below the deploy, clear the flag and re-walk.

Since ADR-0017 unit 4 (issue 495, 2026-10-09), R3 and R4 **compare before they write**. Each stale Tender's
prefix of unchanged causing notices is re-derived and compared table by table with what is stored.
- Only what differs is written. Identical tables keep their rows and rowids.
- An unchanged head gets its epoch stamped alone.
- A corrected Tender is announced by ADR-0017 D3's seq-less correction rows: one `tender changed`, then a `lot
  changed` per rule-L lot. Its history is not replayed.
- Versions past the prefix are written and announced as before.

**Reading the compare on the counts line.** The `project` job's counts carry `compare (issue 495): N tenders
verified, M corrected; tables … skipped / … rewritten; rows …; K correction rows; … us per row` (in shadow, K
is planned rather than written). The fold's last journal lines print it too, on the full and the incremental path,
with a per-table `identical/compared` split.
- `verified`: nothing in the compared prefix moved, so the Tender's epoch was stamped. Versions appended past the
  stored chain, if any, were still written and announced by their transition rows.
- `corrected`: the Tender got its share of the `K` correction rows. That share is one `tender changed` plus a `lot
  changed` per rule-L lot: every lot of the new head when the head moved, every surviving lot when stored versions
  were dropped uncompared (a mid-chain insert or a cut).
- A filtered SSE subscriber (`/v1/lots?winner=…`) gets an event for each corrected entity, `removed` for one outside
  its filter (ADR-0017 D4, as for organization merges). A corpus-wide correction is therefore a burst of such
  events: say so before running one.

**Stop rule.** Before a refold, run its cohort in shadow on a sample (a `refold` of one small profile with a
runtime drop-in `Environment=TENDER_REFOLD_COMPARE=shadow`) and read the planned correction rows. If the real run's
correction rows land outside 0.5×–2× of that prediction per Tender, stop and investigate before the next one: the
compare and the logic change disagree about what moved.

**Kill switch.** `TENDER_REFOLD_COMPARE=off` (a drop-in, then a restart) restores the full rewrite and the history
replay (the issue-179 cost below). `shadow` compares and counts but still rewrites everything. The compare refuses
itself, and logs it once, when `tender_version_bid_parties_version` is missing (a rebuild defers it).

### The procedure type and its backfill (issue 479)

Every Tender version carries at most one `procedure` classification (scheme `procedure`, field
`procedure`, Tender scope, `lot_id` NULL) in eForms' `procurement-procedure-type` vocabulary — the
value `/v1/tenders?procedure_type=` filters on and every row echoes as `procedure_type`. The fold
elects it per notice (`project::elect_procedure_type`) from each era's own field, in this order:
the code fields `BT-105-Procedure` (TED eForms, DÖE eForms-DE 2.x, FTS),
`DE1-TenderingProcess-ProcedureCode` (through the DE-1.x alias) and
`SDK01-TenderingProcess-ProcedureCode`; then the r208/r209 form's own section-IV checkbox where it
names one type (`project::PROCEDURE_MARKERS`: `PT_OPEN`, `PT_RESTRICTED`, `PT_COMPETITIVE_DIALOGUE`,
`PT_INNOVATION_PARTNERSHIP`, the with-call and the without-call markers, Integer-1 rows in
`notice_integers`); then TED's code `TED-PR_PROC`, the internal-ojs era's `TED-PROC` and the text
era's `TXT-PR` through the closed table in `project::procedure_type` (1 open, 2/3 restricted,
4 neg-w-call, T neg-wo-call). The marker outranks the PR code, so a `PR_PROC 4` notice whose form
ticked a without-call box folds `neg-wo-call`, and a PR code the table does not map (`V`, `C`, `G`)
still folds when its form ticked a box that names the type.

Everything else emits nothing: **every PR/PROC code but 1/2/3/4/T** — `9`/`Z` and the text era's
`0`/`7`/`8` (not procedure types), and `6`, `B`, `C`, `E`, `F`, `G`, `N`, `V` or any code the census
surfaces (unclassified until it does) — an eForms code off the list (`comp-tend`), an sdk-0.1
free-text label. A notice that emits nothing keeps the type an earlier notice of its Tender stated,
so the served type is the latest MAPPED one. German national codes are stored as published: the
eight eForms-DE `us-*` codes, and any well-shaped sdk-0.1 `de-*` code.

**The fold's `counts` line reports coverage.** The `issue-479 procedure type` suffix reports, per
profile family (`eforms`, `eforms-de-2x`, `eforms-de-1x`, `doe-sdk01`, `fts`, `r209`, `r208`,
`internal-ojs`, `text`), how many planned notices folded a code (F), published only unmapped values
(U), or published none (N) — counted where `Ident::read` runs, through the same
`elect_procedure_type` the fold calls, so the line cannot disagree with what was written.
`CONFLICTING` counts mapped values that disagree with the elected one; on `r208`/`r209` that is the
PR × marker cross-tab's disagreement column (a PR 4 with a without-call marker counts one), so read
it rather than alarm on it. Two blind spots:
- **`fts` U is structurally 0.** FTS's parser (`fts::parse::procedure_type`, issue 465) drops every
  `procurementMethodDetails` label off its own closed table before the notice layer, so the
  Procurement Act routes (`Competitive flexible procedure`, `Direct award`, `Award under
  framework`, the `Below threshold - …` routes) and unknown labels arrive as nothing and land in
  `fts` N, not U. FTS coverage of those labels needs the raw releases, not this line.
- **A resumed fold prints no suffix.** The tally is filled in Phase 1, which a resumed run skips
  (it reports zeroes, and `rows()` hides empty families). If the backfill fold is stopped at a
  checkpoint and resumed, the `issue-479` suffix of the FIRST, stopped run's counts line is the only
  record, and it covers only what that run planned before it stopped — partial if the stop fell
  inside Phase 1. Note that in the issue when it happens; the per-profile coverage then comes from
  the census reads below, not from the fold.

**Census before the backfill (do not skip).** Read 2026-10-05 (issue 479 "Legacy census"): the PR 4
gate held, and `6`/`B` → `neg-w-call`, `V` → `neg-wo-call`, `C` → `comp-dial`, `G` → `innovation`
joined the table; `E`/`F`/`A`/`N`/`Z` and the text-era letters stay unmapped. Re-read it only if the
table is touched again. (Before it the table was frozen from fixtures alone.) A table change
after the backfill costs a SECOND full fold (~7.5 h) and a second change event per Tender, so read the
legacy cross-tab first. Bounded PK-window reads through `/v1/sql`, two or three windows per legacy
profile, in a low-traffic window, each on the team lead's word, never retrying a 408. Window starts
come free from `/v1/notices?publication_id=…` (e.g. 100002-2019 → 20320242, 100002-2014 → 17806795,
100002-2003 → 2114278, 114238-2008 → 27161440):

```sql
-- per profile × PR code × procedure marker, over one 50k notice-id window
SELECT n.profile, c.field_id, c.code, COALESCE(i.field_id, '-') AS marker, COUNT(*) AS notices
  FROM notice_codes c
  JOIN notices n ON n.id = c.notice_id
  LEFT JOIN notice_integers i
         ON i.notice_id = c.notice_id AND i.field_id LIKE 'TED-%PT\_%' ESCAPE '\'
 WHERE c.notice_id BETWEEN :start AND :start + 50000
   AND c.field_id IN ('TED-PR_PROC', 'TED-PROC', 'TXT-PR')
 GROUP BY 1, 2, 3, 4
```

Read it for: (a) **the PR 4 gate** — PR 4 rows with NO marker (r208 forms that print none) are the
only ones still folded by the code alone; if a window shows without-call markers on more than 1 % of
PR 4 notices, the unmarked remainder is suspect too, and PR 4 should fold only through its marker
(change the `"4"` arm to `None`) BEFORE the backfill; (b) **every unmapped code** with its marker
distribution — a code that co-occurs ≥ 99 % with one marker gets that marker's type in the table
(`V` ↔ `PT_AWARD_CONTRACT_WITHOUT_CALL` → `neg-wo-call`, `C` ↔ `PT_COMPETITIVE_DIALOGUE` →
`comp-dial`, `G` ↔ `PT_INNOVATION_PARTNERSHIP` → `innovation`), which matters only for the notices
whose form printed no marker; (c) the share of PR codes off the table, for the record. Freeze the
table from that (a code change, the gate, a deploy), then run the ONE backfill below. If the census
is skipped on purpose, say so in the issue: any table change it would have caught then costs another
full fold.

**Backfill: one full fold, no re-parse, no `PROJECTION_EPOCH` bump.** Every era already stores its
procedure field in the notice layer (FTS since issue 465; the markers since the r209 walker), so
nothing is re-parsed; the DE-1.x alias, the sdk-0.1 arm and the markers are fold-side. New ingests
carry the type from the deploy. Existing Tenders get it only when a fold rewrites them, and every era
publishes the field, so the cohort is the corpus: `refold-fields` on the field ids would first sweep
the value tables (~46 min) and then requeue ~all notices, and either way the legacy delta is far
over `LEGACY_CLOSURE_CAP` (500,000), so the incremental fold takes the full fallback (job 1616:
~442 min). One `refold` over every profile skips the sweep:

```sh
# 0. Free disk first: ~14 M new classification rows (one per version that has a code) plus two
#    index entries each, order 1–1.5 GB. The fold also emits a change event per rewritten Tender.
df -h /data   # the DB is /data/db (the unit's WorkingDirectory)
# 1. The census above is read and the table frozen from it (deployed).
# 2. Off the daily tick, on a queue read idle by hand (issue 459), and batched with any other
#    corpus-wide fold change pending at the time so the corpus pays ONE fold (397's 1595–1597).
#    Batched with, pending (2026-10-05): issue 484 unit 3's `is_buyer` backfill — it has no job of
#    its own and rides the next all-profile fold (see "The buyer-equal winner flag" below).
/root/aj.sh /admin/jobs | jq '.recent[:5], .queued'
# 3. Size it: expect=1 aborts with the real count and writes nothing. The profiles are exact
#    profile strings (`/v1/notices?kind=` spells them: every `eforms:eforms-sdk-*`,
#    `eforms:eforms-de-*`, `fts:ocds-1.1`, `ted-export-r209`, `ted-export-r208`,
#    `internal-ojs`, `text`); the count must come out at the corpus notice count.
/root/aj.sh /admin/jobs '{"kind":"refold","profiles":[<every profile>],"expect":1}'
# 4. The real run with that count. It requeues every notice, stamps every Tender stale (the
#    issue-179 pair) and queues `project rebuild=false`, which takes the full fallback (~7.5 h).
/root/aj.sh /admin/jobs '{"kind":"refold","profiles":[<every profile>],"expect":<count>}'
# 5. Read the trailing project job's counts line: the issue-479 suffix is the per-era coverage
#    (fts U is 0 by construction; a stopped-and-resumed fold prints no suffix — see above).
```

Rejected: a bespoke writer inserting `procedure` rows directly — it would re-implement supersession
and carry-forward outside the fold, a second writer of one table, to save a fold that has a runbook.

**Verify** after the fold: `curl -s https://tenders.zebreus.click/v1/tenders/8576017 | jq -c
'[(.classifications // [])[] | select(.scheme == "procedure") | .code]'` gives `["open"]`;
`/v1/tenders?procedure_type=open&limit=1` answers with `ignored_filters: []`; and one tender per era
(TED eForms tender 2 `neg-w-call`, the tenders of r209 100002-2019 and r208 100002-2014 `open`, the
internal-ojs CN of 115165-2008 `open`, the text-era award 100002-2003 `[]` unless chained to a CN,
DÖE 1542904 `[]` — a free-text label). Then time a rare code (`?procedure_type=innovation&limit=10`):
the read seeds from the `(scheme, code)` index under `COUNTRY_SEED_CAP` entries
(`read::procedure_seed_viable`, list reads only — the SSE diff's single-tender reads never seed); a
slow answer means the crossover is wrong for this shape, and the 408 band still bounds the walk.

### The elected lot value and its epoch-4 backfill (issue 490)

Since issue 490 the fold stores each version's lot value in `tender_version_lots.value_cents`,
`value_currency` and `value_eur_cents`. It is elected by `canonical::elect_lot_value`, the same
function the REST lot row runs.

**Deploy A** (the fold write plus `PROJECTION_EPOCH` = 4) leaves every existing row NULL.
- REST still derives the value at read time, so nothing REST serves changes.
- `/v1/sql` does show the three columns at once. They are mostly NULL, and their column notes say
  why.

The columns are filled by one all-profile refold. **Deploy B** then switches `summarise` to read
them, exposes them on `v_lots`, and adds their index. Do not ship deploy B before every check below
is green.

**What the bump costs until the refold has run.** Every Tender is epoch-stale.
- Each Tender the daily fold touches is compared and rewritten only where it differs (ADR-0017 D1). Before
  issue 495 unit 4 it was rewritten in full (`keep = 0`) and re-emitted its history to `/v1/changes`, the
  issue-179 cost.
- Any whole-corpus walk (the bucketed fallback at 100k+ planned notices, a large reparse or refold)
  rewrites all ~8.8 M Tenders. If one is due anyway, let it BE the backfill and run the completeness
  check after it. Otherwise hold it until the refold.

```sh
# 0. Before-images, taken BEFORE deploy A. Deploy A re-elects the head of every Tender the daily
#    fold touches, through head_value_eur_cents_with.
#    - df -h /data
#    - The latest data-quality section 16 (the head band).
#    - The table size. tender_version_lots holds a row per lot per VERSION, so the lots count
#      (13.2 M) is only a floor. MAX(rowid) is a high-water mark, and sqlite_stat1 is refused by
#      /v1/sql. Sample primary-key windows and scale up:
echo "SELECT COUNT(*) FROM tender_version_lots WHERE tender_id BETWEEN 4000000 AND 4099999" | /root/sq.sh
# 1. The queue idle by hand, with NOTHING unprojected. The `expect:1` sizing call below queues
#    a real `project` behind its own abort (jobs 2041/2042 in issue 484). If notices are
#    unprojected, that project folds them first, which can be hours on the bucketed path.
/root/aj.sh /admin/jobs | jq '.current, .queued, .recent[:3]'
# 2. The profile list, built fresh. A profile ingested since the last list is otherwise missed
#    (correctness review F1). Walk `SELECT profile FROM notices WHERE profile > ? ORDER BY profile
#    LIMIT 1` (the notices_profile index) from '' until it returns nothing.
# 3. Size it, then run it with the count. No deploy may land in the run's window (about 10.5 h:
#    requeue, stamp, then `project rebuild=false` on the full fallback).
/root/aj.sh /admin/jobs '{"kind":"refold","profiles":[<every profile>],"expect":1}'
/root/aj.sh /admin/jobs '{"kind":"refold","profiles":[<every profile>],"expect":<count>}'
```

**Completeness is the epoch, not the counts line.** A stopped or restarted `project rebuild=false`
cannot resume its plan. The next run re-plans and reports a large `unchanged` count, so "written ==
stamped" is not a pass condition. Instead:

- Take the top of the range with `SELECT MAX(id) FROM tenders`.
- Read 100k-id windows (issue 397's measured size; `COUNT(*)` does not yield, and a 408 is never
  retried) until every window answers 0.
- For a window that does not answer 0, list its stale ids and refold just those Tenders.

```sh
echo "SELECT COUNT(*) FROM tenders WHERE id BETWEEN 1 AND 100000 AND projection_epoch <> 4" | /root/sq.sh
echo "SELECT id, current_seq FROM tenders WHERE id BETWEEN 1 AND 100000 AND projection_epoch <> 4 LIMIT 20" | /root/sq.sh
```

Any job that stamps Tenders epoch-stale (`rederive-eur`, the resolver, `refold-notices`) stamps 0. So
`<> 4` over-reports and never under-reports. Triage a residue by epoch:

- **3** means untouched since deploy A. Refold it.
- **0** is one of two cases:
  - Re-stamped after its 490 write, so its values are present and a requeued notice is in flight.
  - **Stuck**: every causing notice is no longer `parse_state = 'parsed'` (quarantined on a reparse,
    or pending). The stamp joins notices by profile with no parse-state filter, but the requeue takes
    only parsed ones. If the trailing project took the INCREMENTAL path, such a Tender keeps its
    pre-490 rows. The project's log line `INCREMENTAL → FULL fallback BEFORE identity pass` says
    which path it took. Probe a stuck Tender's lot rows for a NULL value beside an admissible lot
    amount.

**Before deploy B, also:**

- The m490 windows (`.scratch/tender-db/490-values/m490/`, five 500-tender windows) compare the
  stored `value_*` with the REST lot `value` lot by lot: 0 differences.
- The section-16 band is the before-image apart from daily drift.
- `rederive-eur` requeues the causing notices of the Tenders it changed (issue 490 unit 3c). Until
  then a rate correction moves the stored value only at the Tender's next fold. Do not let the
  compare straddle a `fetch-rates` or a `rederive-eur`.
- Since issue 495 unit 5, each `rederive-eur` window commits its moved `eur_cents`, the stamp, the
  re-queue, the watermark and ADR-0017 D5's correction rows together. The job summary counts the
  correction rows it announced; run `project` afterwards, and the fold announces whatever its election
  then moves.
- Since issue 543 unit 3, a cancel stops `rederive-eur` before its next window. The committed windows
  stand, and the stop clears the watermark: the next `rederive-eur` walks every tender again, so a rates
  repair made after the stop reaches the whole corpus. A window whose rates have not changed rewrites
  nothing. Only the same job, recovered after a restart, resumes, and from its OWN cursor (the job
  progress each window records after it commits; since 543's follow-up, not the stored watermark any run
  writes); a fresh enqueue always starts at the first tender.

**After deploy B:**

1. Wait for the boot-time Reindex to build the partial index before timing any value query. Until it
   exists, a lot value range is a scan of the table. `/v1/sql` refuses `sqlite_master`, so confirm it
   another way:
   - the `reindex` job in `/root/aj.sh /admin/jobs` finished `ok` after the deploy; or
   - the boot line `supervisor: N deferred index(es) missing (...)` no longer names
     `tender_version_lots_value_eur` (`journalctl -u tender-db`).
2. Time the `/docs` recipe (top 20 lots at or above EUR 10 m) and a narrow band with a LIMIT.
3. Do not compare a lot `COUNT(*)` over a wide band with the tenders' 56–66 ms. Every index entry
   of every version probes `tenders`, so it is not index-only.

**A rollback below deploy A** after the refold has run:

- The epoch-3 binary reads every stored epoch 4 as stale. Each Tender it folds is rewritten in full
  with NULL lot values and stamped 3.
- After deploy A returns, those Tenders are stale again and rewrite on their next fold. The ones
  nothing touches stay NULL.
- The epoch check above lists exactly them. Refold them before deploy B.
- Never run a whole-corpus walk on the rolled-back binary: it would rewrite the whole corpus to
  NULL.

**A future change to the election** (`sentinel_amount`, `IMPLAUSIBLE_EUR_CENTS`, `ScalePartners`)
reaches stored lot values only through a stamping refold, as it already did for the head column.
- The 471-style drain picks Tenders by the head band. That covers every lot a rule that refuses
  MORE can move.
- It does not cover a rule that admits more.
- It does not cover the stored values of older versions, which SSE `Scope::At` serves.

### The Tender-link ledger and `backfill-tender-links` (issue 481)

`tender_links` holds every link that joins notices across procedure keys, keyed by notice (never by
Tender, whose ids are retired on merge). `declared` rows are written by the plan build for each
planned non-legacy notice and diffed every time it is planned: `opp-090` (target Source always TED)
and `logical-notice` (TED BT-701 → every DÖE `<id>-<digits>` version). A reference nothing holds
keeps one row with `b_notice_id` NULL, found by name when its target arrives. `matched` rows are
written and undone only through `Db::write_matched_links` / `Db::delete_matched_links` (unit 3),
committed 5,000 links per transaction. A link resolves among PARSED notices only, as a full plan
holds them: a reference to a pending or quarantined notice stays unresolved until it parses. The
ledger survives `reset_tender_layer` and is off `/v1/sql`; so is `tender_key_merges`, the grouping's
record of every procedure key a link merge folded into another key's Tender (an absorbed key names
no Tender, and the daily finds a later notice under it through this row).

**`projection_state.tender_links_complete`** attests that every parsed notice has its declared rows.
It is 0 on prod after the deploy (the corpus was planned before the ledger) and set by a finished
WET `backfill-tender-links` or a completed full plan build, never cleared. While it is 0 the daily
holds every fan-in- and weld-guarded join back (`deferred`): a carrier with no row is one the
closure cannot reach, and a guard counting part of a component would admit what a full fold
refuses. ADR-0011's same-Source previous-notice joins are unguarded and go ahead.

**A ledger write or delete outside a fold re-queues both notices** (`notices.projected = 0`, the key
the incremental change-set reads), in the same transaction as the row, so it reaches the next DAILY
fold; a written row whose notices already share a Tender re-queues nothing. **The daily fold applies
links**: after the touched expansion and the legacy closure it walks the ledger from every planned
notice (rows they state, rows naming them by id, unresolved rows naming them by publication id or
version stem, and the changed notices' own links resolved from their parse) and plans each Tender it
reaches whole, to a fixpoint. Past 500,000 added notices (the legacy closure's cap) it falls back to
the full path, loudly (`INCREMENTAL → FULL fallback: link closure exceeds cap … (issue 481)`).

```sh
# Dry (the default): what the ledger lacks, per rule, and the sampled pairs to read by hand.
/root/aj.sh /admin/jobs '{"kind":"backfill-tender-links"}'
/root/aj.sh /admin/reports/tender-link-backfill | jq -r .body | jq '{notices, declaring, would_merge, not_earlier, buyer_disjoint, would_split, requeued, rules}'
/root/aj.sh /admin/reports/tender-link-backfill | jq -r .body | jq -r '.samples[] | "\(.rule)  \(.a)  \(.b)"'
# Issue 481 unit 2b: the buyer census — joins the guard refuses, and welds the next fold splits.
/root/aj.sh /admin/reports/tender-link-backfill | jq -r .body | jq -r '.buyer_disjoint_samples[] | "\(.rule)  \(.a)  \(.b)"'
/root/aj.sh /admin/reports/tender-link-backfill | jq -r .body | jq -r '.would_split_samples[] | "\(.rule)  \(.a)  \(.b)"'
# Wet: write the rows and re-queue the would-merge and would-split pairs; the next `project` (the daily) applies them.
/root/aj.sh /admin/jobs '{"kind":"backfill-tender-links","dry_run":false}'
```

`backfill-tender-links` (**dry default**; stoppable between windows; report `tender-link-backfill`,
stored only by a run that finished) is the one-shot for the corpus planned before the ledger existed.
It walks every parsed non-legacy notice by id, 5,000 notices and at most 500,000 ids per window,
loading only the link fields (BT-701, OPP-090 and the DE-1.x spelling folded onto them) with one
range read of `notice_ids` per window, derives the links with the plan's own `declared_links` and
diffs them with the producer's own diff. Wet, each window's rows and re-queues are ONE transaction on
the writer (a few thousand index inserts), then a WAL checkpoint. Per rule it counts `declared`,
`present` (already on the ledger), `resolved`, `unresolved`, `would_merge` (rows with a target,
written now or already held, whose notices fold into different Tenders today: the joins the next
fold makes, before its weld guards; they re-queue both notices, a held row included, since a
daily before the attestation wrote it and held its join back; the `not_earlier` and
`buyer_disjoint` ones below do not, nor issue 486's `shared_kind`), `cross_source` (of `would_merge`,
the rows citing another Source: every `logical-notice` row, and the DÖE→TED OPP-090s ADR-0011 never
measured — read this one for `opp-090` before the wet run) and `stale` (declared rows the notice no
longer declares, deleted, both notices re-queued). Only `would_merge` rows (less `not_earlier`
and `buyer_disjoint`), `would_split` rows and `stale` rows re-queue anything.

**The buyer census (issue 481 unit 2b).** Every resolved `opp-090` row is judged the way the fold
judges it, in the fold's order. The census computes the fold's input from both notices' full parse
(DE-1.x folded first, as the plan does) through the plan row's own derivation: the tolerant buyer
tokens, the procedure key and the publication instant. It counts three things per rule, `opp-090`
only (`logical-notice` reads 0):
- **`not_earlier`**: rows in `would_merge` whose target is NOT strictly earlier than the citing
  notice. The fold refuses these by direction before it reads a buyer (ADR-0011 guard 3), so they
  re-queue nothing and are in no sample list. Job 1882's copied placeholder (Älvkarleby kommun
  citing SCB's newer `00123456-2026`) is one; before the unit 2b review it read as a join.
- **`buyer_disjoint`**: rows in `would_merge` whose target is strictly earlier and whose two
  notices are buyer-disjoint. The next fold refuses these joins, so they re-queue nothing and stay
  out of `samples`. Up to 30 are listed in `buyer_disjoint_samples`.
- **`would_split`**: rows whose notices share a Tender TODAY (an earlier full projection joined
  them before the guard existed) but are buyer-disjoint, with the target strictly earlier and the
  two notices under different procedure keys. A shared BT-04 keeps them in one group, so those are
  not counted. The next fold that plans these pairs splits them. The wet run re-queues both notices
  so that fold is the next daily. Up to 30 are listed in `would_split_samples`. The count is an
  upper bound: another admitted path between the two notices keeps them together. A split retires
  nothing: the cited notice keeps the Tender (it named it, being earlier), and the citer's key gets
  a Tender of its own and loses its `tender_key_merges` row.

**What overlaps.** A buyer's tokens are its identifier and its names, and two notices overlap when
they share one. Each is widened to the spellings one buyer is measured to publish (unit 2b review),
since a token can only make two notices overlap:
- the identifier as its E1 cross-walk key when it has one (every SIRET of one SIREN, as AP-HP
  publishes five; a NIP bare or as a `PL…` VAT; an organisationsnummer and its `SE…01` VAT), else
  as published. Pad-derived (E2) keys are not used, since padding has collided across entities;
- every name the buyer published (each language variant, not just the first), lower-cased with
  punctuation folded and Latin accents dropped (`HOPITAUX` meets `hôpitaux`), under the
  identifier's register country (`RE` meets `FR`).

Since unit 2c (job 1893's census: ~12 of 30 `would_split` samples were one buyer named two ways)
two notices also overlap on:
- one **resolved organization** among their buyer-side mentions (the org layer after merges;
  the fold and the census read the same recorded mentions);
- one **raw identifier** (letters and digits, upper case) whatever its scheme, placeholder classes
  excepted;
- the **contract signatory** beside the buyers (a ministry signing for its hospital);
- an **agency's principal** (`… B.V. namens Stichting Prisma` meets `Stichting Prisma`; one agency
  buying for two schools does not meet itself, nor the agency's notices in its own name);
- a name that is a **whole-word prefix** of the other's name or equals its **head**, the text before
  the first separator, of three content words or more (`ARPAS` / `ARPAS - Agenzia…`, two
  `Servicio Andaluz de Salud. …` units), but never two names that only start alike, never the
  name of a kind of body (`Centre hospitalier universitaire (CHU) de …`, `Azienda sanitaria
  locale - …`, `Zarząd Dróg Wojewódzkich w …`, `Stadt …`), and never for a buyer with no country;
- an **acronym** and its spelled-out name (`ICS` / `Institut Català de la Salut`).

Still disjoint, and so refused: one buyer that changed or respelled its name beyond that, with no
identifier in common, written in another co-official language (`Servizo Galego de Saúde` against a
`Servicio Gallego de Salud` area), or abbreviated mid-name (`im. M.Nenckiego PAN`); a CAN filed by
another body than the CN's buyer; and a group PIN or qualification system (the DB group's
deadline-shortening PIN, ÖBB-Holding's), which is cited by many procedures and is not one of them
(since issue 486 such a notice is refused by its own type first, buyers or not, when its subtype
says what it is; a PIN used as a call for competition, `10`-`14`, is not).

**The shared-publication refusal (issue 486).** A previous-notice reference whose citing OR cited
notice is a shared publication by its own type is refused by the fold, same-Source or cross-Source,
after the direction check and before the buyer guard and the fan-in count. It is counted
`shared-kind` on the `issue-481` job-row line (`refused: N (not-earlier …, shared-kind …,
buyer-disjoint …`). The type is the plan row's `shared_kind`. For eForms rows (new in 486) it comes from the subtype
(`OPP-070`, DE-1.x folded onto it): `1`-`3` buyer profile, `4`-`6` PIN only and `7`-`9` PIN to shorten time limits
(`5` and `8`, the utilities ones, read periodic indicative), `15` qualification system. A notice with no subtype falls
back to the notice type (`BT-02`, or sdk-0.1's `SDK01-NoticeTypeCode`): `pin-buyer`, `pin-only`, `pin-rtl`, `qu-sy`.
For legacy rows it comes from the `TD` code (issue 364 unit 6: `0`, `A`, `P`, `M`, `B`, `O`, `Q`, `Y`). The
refusal reads both. **So an eForms OPP-090 that cites a LEGACY PIN, periodic indicative, buyer-profile,
qualification-system or DPS notice is refused too, where before 486 it joined.** The 364 gate itself is unchanged: it
still refuses legacy OJS edges in the union-find, and ledger edges are judged only here. These eForms subtypes are NOT
flagged:
- `10`-`14`, the PIN used as a call for competition: the call that opens one procedure, which its award cites as a CAN
  cites its CN. A DPS opened by one therefore stays joined, as a DPS opened by a contract notice (`16`/`17`) does.
- contract notices, results, VEAT and modifications.
- the transport PIN `T01` (`pin-tran`).
- the national `E`/`T`/`X` subtypes, including `E1`/`E2`, which are national planning notices but unmeasured.

A flagged notice keeps its own Tender, together with the notices under its own BT-04, and each citer stays on its own
procedure's Tender. The cost: an award citing its own PIN (only, or to shorten time limits) under ANOTHER BT-04 no
longer joins it. A shared BT-04 still joins them. This follows ADR-0011's "a weld is worse than a missing link".

The census applies the same rule. `shared_kind` counts the would-merge pairs it refuses (nothing re-queued).
`would_split_shared` counts the would-split pairs: one Tender today, under different keys, split by the next fold
because an end is a shared publication, whatever the buyers (both ends re-queued). `would_split_shared_kinds` breaks
the latter down per kind (the cited end's kind, else the citing end's), each with its own sample of pairs, so a PIN's
splits read apart from a qualification system's. A grouping plan from before 486 has no `plan_eforms_shared_kind`
marker and is rebuilt, not resumed.

**Since units 1a/1c (deployed `9f9db13`) the outright refusal above is the qualification system only**
(`store::link_refuses_shared_kind`): dry 1982 would have split 10,277 PIN pairs, mostly one procedure announced early.
**The PIN fan-in (issue 486 unit 1b).** A previous-notice reference, same- or cross-Source, into a PIN, periodic
indicative or buyer-profile notice (`store::link_fans_in_shared_kind`, eForms or legacy) is refused when two or more
keyed components cite that notice's component, counted after the other same-Source previous-notice unions. The PIN's
own component counts as a citer when a non-PIN notice in it cites the PIN, for example a contract notice that reuses
the PIN's BT-04. Every reference in is refused, and the PIN becomes a Tender of its own. A PIN cited by one procedure
still joins it. The refusal is counted `pin-fan-in` on the `issue-481` line, after `buyer-disjoint`. It is judged
after the buyer guard and before 481's cross-Source fan-in. A join waits (`deferred`) while the plan may not see
every citer. A refusal never waits. When a second procedure cites a PIN that has already joined the first, the next
daily's link closure walks from the newcomer to the PIN and on to every ledger row naming it. It refuses both
references and splits the first procedure's Tender. Full and daily give the same result
(`a_pin_joins_its_one_citer_and_stands_alone_once_a_second_procedure_cites_it`). The census judges the same rule.
Its caller reads every citer of a PIN-kind target off the ledger (`tender_links_b`) plus the window's own rows, and
counts their distinct procedure keys (`LinkEndpoint::pin_citer_keys`), pooled over the PIN-kind notices under the
PIN's own BT-04 (an amendment's citers add up with the PIN's, as in the fold, whose node is the key; siblings are read
from the Tender under that key). A would-merge pair it refuses is `pin_fan_in`
(per rule and total; nothing re-queued). A would-split pair is `would_split` + `would_split_shared` under the PIN's kind
in `would_split_shared_kinds`. The census counts KEYS where the fold counts components, so it can miss either way: two
keys that a re-tender chain joins count twice, and a sibling PIN that another key's Tender absorbed is not found. The
fold is the verdict; read the dry run as an estimate.
**A 1:1 PIN join can wait.** Before 1b a same-Source reference into a PIN always joined. Now a join into a PIN-kind
notice waits (`deferred`) under 481's conditions: the ledger not attested complete (`tender_links_complete`), or a
one-ended link beside it. A later fold that sees every citer joins it. Before deploy, confirm the flag is set on prod;
after the wet run, read the daily's `deferred` count.

**The 486 re-queue on prod** reuses this job's wet run; there is no new job. The welds are `would_split_shared` pairs,
for example Tender 202112's 234 versions behind qualification-system notice 24716938.
```sh
/root/aj.sh /admin/jobs '{"kind":"backfill-tender-links"}'      # dry
/root/aj.sh /admin/reports/tender-link-backfill | jq -r .body \
  | jq '{would_split, would_split_shared, shared_kind, requeued, kinds: (.would_split_shared_kinds | map_values(.pairs))}'
/root/aj.sh /admin/reports/tender-link-backfill | jq -r .body | jq '.would_split_shared_kinds'   # the per-kind samples
```
Decide from the dry run before the wet run:
- **Expected:** `NOTICE_QUALIFICATION_SYSTEM` in the hundreds to low thousands (Tender 202112 alone is ~234 pairs).
  The PIN kinds can be larger, because PINs are widely cited, but each was a weld only if the citer sits under ANOTHER
  BT-04.
- **Per kind, open about 10 of its samples on TED.** Count each pair that is genuinely one procedure (an award of the
  PIN's own planned contract) as a correct merge the split undoes.
- **Go:** total `would_split_shared` ≤ 5,000 and, per kind, no more than about 1 sample in 10 a correct merge.
- **Stop:** total > 5,000, or one kind's samples mostly correct merges. Do not run wet. Report the per-kind counts and
  samples on issue 486 and decide per kind (a kind can come out of `SHARED_EFORMS_SUBTYPES` in a follow-up unit).
  The fold refuses on every daily that re-plans such a component and on any full rebuild, whether or not the wet run
  happens. So a kind that must not split has to leave the table in code; skipping the wet run does not protect it.
- The wet run also re-queues whatever 481 `would_merge` joins and `stale` rows are still pending. Read those counts
  too, because the next daily absorbs all of them. The re-queued notice count is `requeued`.
```sh
/root/aj.sh /admin/jobs '{"kind":"backfill-tender-links","dry_run":false}'   # wet: re-queue both ends
# the next daily `project` splits them; verify:
curl -s https://tenders.zebreus.click/v1/tenders/202112 | jq '.versions | length'
```

**Read the census before the wet run.** Each sample names both publication ids. For every
`buyer_disjoint` and `would_split` sample, open both notices on TED and check whether they are one
procedure.
- A copied placeholder, or two different procedures, is the guard working.
- One procedure whose buyer was renamed or respelled between the notices, with no identifier in
  common, is a false refusal. A false refusal would also split a correct Tender on the next daily
  (`would_split`). Count these before the wet run, and hold the wet run if they are not rare.
- The guard already runs on every daily once this binary is deployed. Holding the wet run delays
  the splits and the re-queue, but it does not stop them.

The census costs one full parse per endpoint of a resolved `opp-090` row. Job 1882 found 181,222
resolved rows, which is at most ~360k parses on top of the 3.87M link-field reads. Expect the dry
run to take noticeably longer than job 1882's 585 s, and read its job row for the real figure. Idempotent: a re-run, or a notice a fold planned
since the deploy, reads as `present`. A wet run that walks to its target attests the ledger complete. Expected on prod: `logical-notice` resolved rows in the hundreds of thousands (every
TED/DÖE twin pair, most of them already one Tender by BT-04) and `would_merge` near the calibration's
~2,650 above-threshold islands, plus the cross-source OPP-090 joins (unmeasured). **Read the samples
before the wet run**: each pair names both publication ids, so a sample can be checked on the two
portals by hand. After the wet run the next daily `project` is a normal daily plus the would-merge
pairs' Tenders; read its `issue-481` line (largest component, refusals, `deferred: 0`).

**Disk before the wet run**: a ledger row costs about 229 bytes, table and indexes together (measured
2026-10-02: 200,000 unresolved `logical-notice` rows, 11,174 pages of 4 KiB). The report carries the
run's own figure (`ledger_bytes`; the summary's `~N MB of ledger rows`); at the expected ~2.2M
`logical-notice` rows plus ~250k `opp-090` rows that is about 0.56 GB. Check the free space against it
before queueing the wet run.

**One-time renames, not data loss.** A pre-ledger OPP-090 Tender named after an island member (the
old representative was simply the earliest member) is renamed after its keyed member the first time
a fold plans it: the island-named Tender is retired with a `removed` event and the keyed one minted.
A key the 2026-08-20 re-projection absorbed has no `tender_key_merges` row until a fold plans its
Tender (or the next full projection writes them all); until then a later notice under it still
folds apart, exactly as before the deploy.

**Expected runtime on prod: about 15–45 minutes wet, roughly half that dry** — a reasoned estimate,
not a prod measurement. Measured 2026-10-02 on a synthetic corpus at prod's id-row density (89
`notice_ids` rows per TED eForms notice, as 17,906 rows over 201 notices read on prod): 0.73 ms per
notice dry and 1.55 ms wet in the gate's O0 test build, ~3.7 s of writer per 4,500-notice window.
turso's CPU paths run ~10–30× faster in the release build, which puts the ~3M TED eForms notices (one
resolve seek and about one row each) and the other non-legacy notices (no link, one shared range
read per window) at minutes of CPU; the rest is cold-disk seeks into `notices`' UNIQUE and the
per-window checkpoints (~1,600 windows). The writer is held for one window's inserts at a time
(well under a second in release). The dry run's own job row is the real measurement: read its
duration before queueing the wet one.

### `procedure-key-census`: buyer-disjoint clusters under one UUID (issue 482)

The fold groups notices by procedure key and checks nothing else for a UUID key (BT-04): issue 369's
gate reads shaped keys only. A reused UUID welds unrelated procedures (Tender 1110706: a German
DÖE/TED pair and a Bulgarian award 15 months later). This census counts how often, before a guard is
chosen (refuse the key, as 369 does, or split the Tender at a buyer-disjoint edge).

```sh
/root/aj.sh /admin/jobs '{"kind":"procedure-key-census"}'
/root/aj.sh /admin/reports/procedure-key-census | jq -r .body | jq '{tenders, notices, notices_without_buyers, undecidable, split, split_with_buyerless, split_same_key, interleaved, sequential, singleton_minorities, max_clusters, cluster_counts, cross, buckets: (.buckets | map_values(.tenders))}'
# 30 samples per bucket: tender, key, gap, and each cluster's dates, Sources, publications and first buyer.
/root/aj.sh /admin/reports/procedure-key-census | jq -r .body | jq -r '.buckets["several-jurisdictions"].samples[] | "\(.tender_id) \(.procedure_key) gap \(.gap_days)d span \(.span_days)d", (.clusters[] | "   \(.notices)× \(.first_published)..\(.last_published) +\(.gap_days)d \(.sources|join(",")) \(.jurisdictions|join(",")) \(.buyer)  \(.publications[:4]|join(" "))")'
# The hub shape: the 30 Tenders with the most clusters.
/root/aj.sh /admin/reports/procedure-key-census | jq -r .body | jq -r '.hubs[] | "\(.tender_id) \(.notices) notices, \(.clusters_total) clusters"'
```

**Read-only** (no dry flag; it writes nothing but its report), stoppable between windows, report
`procedure-key-census` stored only by a run that finished. It walks the Tender layer by id in windows
of 5,000 ids (`Db::uuid_keyed_tender_versions`: a primary-key range on `tenders`, the key's shape
pre-filtered in SQL, a seek per Tender into `tender_versions`) and keeps the Tenders whose key is a
genuine UUID (`refused:`, `island:`, `ojs:` and FTS `ocds-` keys never qualify) with two or more
notices. Each notice is fully parsed (DE-1.x folded first) and read through the link guard's own
buyer side (`GuardSide`, issue 481 units 2b/2c: resolved organization, raw identifier, signatory,
agency principal, whole-word prefixes and heads, acronyms), and two notices overlap exactly when the
guard would say so (`store::buyer_tokens_disjoint`). A notice joins a cluster when it overlaps ANY
member, so a joint procurement whose awards each name one of the CN's buyers, or a central
purchasing body's framework naming its call-off buyers on the CN, is one cluster.

A notice naming no buyer is unknown, never decisive: it joins no cluster and splits nothing
(`notices_without_buyers`; `undecidable` counts Tenders with fewer than two notices naming a buyer;
a split Tender lists its buyerless notices under `without_buyers`). A Tender with two or more
clusters is `split`, and lands in one bucket of each axis:
- **`one-jurisdiction` / `several-jurisdictions` / `unknown-jurisdiction`**: several when two
  clusters name buyers of known register jurisdictions and share none (a cross-border joint
  procurement inside one cluster does not count; a DE cluster beside a BG one does); unknown when no
  pair says several and some cluster's buyers name no country (BT-514); one only when every pair of
  clusters shares a known jurisdiction;
- **`one-source` / `several-sources`**: several when two clusters' Sources are disjoint (a DÖE-only
  cluster beside a TED-only one); a TED notice colliding with a DÖE/TED pair (1110706) is one Source.
  Each cluster lists its `sources`;
- **`gap-le-90d` / `gap-le-1y` / `gap-gt-1y`**: the split rule's axis. Each minority cluster's
  `gap_days` is the distance between its time range (`first_published`..`last_published`) and the
  largest cluster's (0 when they overlap); the Tender's `gap_days` is the smallest of them, so
  `gap-gt-1y` means every minority cluster is over a year from the largest;
- **`span-le-90d` / `span-le-1y` / `span-gt-1y`**: secondary, first to last notice over all of them,
  buyerless ones included (`tender_versions.published_at`).

Beside the buckets: `interleaved` (some minority cluster's time range overlaps the largest's: one
platform or authority running procedures side by side under one key, the gate shape) against
`sequential` (a later procedure reusing the key, the split shape); `singleton_minorities` (every
cluster but the largest is one notice: the island a split would cut out). A sample lists at most its
20 largest clusters (`clusters_total` counts them all) and 40 buyerless notices
(`without_buyers_total`).

`split_same_key` counts the split Tenders whose every notice carries the Tender's own key (the BT-04
reuse itself); a cluster's `other_keys` counts its notices under another key, which an issue-481 link
(OPP-090, BT-701) joined, so a sample with `other_keys` > 0 may be a link weld the daily has not split
yet rather than a reused UUID. `cross` counts jurisdictions × Sources × gap together (`several-jurisdictions/one-source/gap-gt-1y`),
`cluster_counts` the cluster count (`2`, `3`, `4-5`, `6-10`, `11+`), and `hubs` the 30 Tenders with
the most clusters (Tender 42726's shape: 36 notices under 8 buyers). Each bucket keeps a uniform
30-Tender sample (bottom-30 by a hash of the Tender id, so a re-run gives the same sample).

**What to decide from it.** A collision bucket (several jurisdictions, more than a year apart) that
is large and clean says the guard is a split at the disjoint edge; a hub-heavy one-jurisdiction
bucket where the clusters are one authority's separate procedures says a refusal of the key (369's
gate). Read the samples on the portals before deciding.

**Expected cost: about 2–3 hours on prod**, a reasoned estimate from bounded reads. 20 bounded
`/v1/sql` windows of 2,000 Tender ids (2026-10-02) found the UUID-keyed Tenders packed at the bottom
of the id space, where the 2026-08-20 re-projection minted them: 1,948–1,957 of every 2,000 ids up to
~1.16M, none sampled between 1.175M and 8.79M, and the dailies' new mints at the top (8.805M: 1,166
of 2,000, almost all with one notice). 62–64 % of them have two or more notices, ~3.2 notices each,
so the census parses about **2.4M notices** in ~735k Tenders. A full parse plus the mention read
costs ~3 ms a notice on prod: job 1903's endpoint census added ~1,030 s over job 1882's 585 s for at
most ~360k endpoints. The ~1,760 windows themselves are primary-key range reads, empty above 1.16M
(minutes in all). It holds no writer, but it DOES hold the single job worker for its whole run: the
daily's `process`/project jobs and fetches queue behind it, so queue it right after a daily finishes.
It keeps no checkpoint: a restart (a deploy) re-runs it from Tender id 0. Out of scope: a key whose
Tender a link merged under a non-UUID key (`plan_group_merge` to a shaped or `ojs:` key) is not walked;
a whitespace-padded UUID is (the SQL pre-filter trims, as `is_uuid` does). Clustering is pairwise per Tender, skipping pairs already
joined; the largest Tender in the sampled windows had 372 notices. Read the job row's duration for the real
figure.

The report also lists every Tender with 3 or more clusters (`hub_tenders_total`, `hub_tender_ids`
ascending, at most 10,000): what the fold's UUID-hub gate (below) refuses, and what
`requeue-uuid-hubs` re-queues. A report stored before issue 482 unit 2 has neither field.

### The UUID-hub gate and `requeue-uuid-hubs` (issue 482 unit 2)

The grouping (`Db::build_plan_groups`) refuses a **UUID** procedure key (a genuine `8-4-4-4-12` uuid,
not placeholder-shaped: those are issue 369's) whose planned notices fall into **3 or more**
buyer-disjoint clusters. Clusters are the census's: issue 481's buyer-guard token sets
(`plan_notice.buyer_guard`) clustered transitively by `store::buyer_clusters` over
`store::buyer_tokens_disjoint`. A notice naming no buyer joins no cluster and never counts. Two clusters are
not acted on (the 2026-10-03 census read: their precision is unmeasured). Joint procurements and a
central purchasing body's framework overlap, so they are one cluster and stay one Tender.

A refused key behaves like 369's: it is in `plan_refused_key`, and each notice under it groups as
`refused:<key>:<label>`. The label is the `buyer_key` of the cluster's first member (smallest notice
id), or `#g<token>` when no notice in the cluster has one, so one buyer's notices (a DÖE/TED pair
included) still fold together, a re-plan names the cluster the same, and a later notice writing the
buyer another way never renames it. A buyerless notice becomes its own Tender
`refused:<key>:#n<notice_id>` (369 makes it an `island:`; the key prefix lets the daily find it). The
link step treats that `#n` group as an island, not a keyed member, so a DÖE twin's logical-notice link
rejoins it. A legacy notice with an OJS self number is left to the legacy closure, as in 369's arm, and
does not count toward the clusters. The fold job row says what it refused:
`; issue-482 uuid hubs refused: K key(s), N notice(s) split into C buyer cluster(s)` (silent at
zero). The journal logs `[project] group step uuid-hubs: … (largest N notices) …` on every grouping,
including the zero; `largest` is the biggest candidate key's notice count, the clustering's input
size (clustering is per token digest, linear in tokens).

**The daily sees a hub key whole.** The gate's cluster count is not monotonic (a bridging joint notice
can collapse clusters), so the incremental plan has to hold every notice under a hub key. A bare-key
Tender is touched whole by its key. A hub key's Tenders are found by the key's `refused:` prefix
(`Db::refused_family_tender_ids`, a range seek on `tenders_procedure_key`). So are the keys an
issue-481 link merged one of its groups into: `tender_key_merges` records `refused:<key>:<label>` →
`<to_key>` rows, which it used to drop. A Tender holding a hub key's group that the plan reaches some
other way pulls in all of that key's Tenders too, through `ingest::project::refused_sibling_closure`,
run after the link closure and to a fixpoint, with the link closure's cap and full fallback. It reads
the Tender's own `refused:` name AND the `tender_key_merges` rows pointing at it (by `to_key`, index
`tender_key_merges_to`): a hub cluster absorbed into another key's Tender is reached when a new notice
arrives under that other key. Pinned by
`a_hub_cluster_merged_into_another_keys_tender_keeps_the_hub_whole_on_the_daily`. When a key crosses
the threshold on a daily, its old Tender is retired (`removed` events, no ghost) and one Tender is
minted per cluster. Pinned by `a_uuid_hub_splits_per_buyer_cluster_on_full_and_daily_folds` (ingest,
`project_incremental.rs`), which compares the full and daily folds step by step.

**369's and 386's refused keys are planned per buyer.** Their gates count distinct buyer sets, which
only grow, so an incremental grouping keeps a placeholder/FTS key that already has a `refused:<key>:`
Tender (or merge row) refused (`[project] group step refused-keys (seeded): …`), and a changed notice
pulls in only its own buyer's `refused:<key>:<buyer_key>` Tender (or the Tender a link merged it into).
A common placeholder key reused EU-wide is never planned whole. Before unit 2, a new notice under such a
key was planned alone, read as one buyer, and founded a bare-key Tender that a full fold never makes.
Pinned by `a_split_placeholder_key_is_planned_per_buyer_on_the_daily`. Residual: a re-parse that
shrinks such a key below its threshold leaves it refused on the daily until the next full re-plan.

**Existing hubs split only when a fold plans them.** A daily touches a hub only when a new notice
arrives under its key. The ~150 hubs the 2026-10-03 census found (Tender 430681: 789 notices, 319
Swiss buyers) are re-planned by re-queueing their notices:

```sh
/root/aj.sh /admin/jobs '{"kind":"procedure-key-census"}'            # ~2–3 h; stores hub_tender_ids
/root/aj.sh /admin/reports/procedure-key-census | jq -r .body | jq '{hub_tenders_total, n: (.hub_tender_ids|length)}'
/root/aj.sh /admin/jobs '{"kind":"requeue-uuid-hubs"}'                # dry (the default): counts
/root/aj.sh /admin/jobs '{"kind":"requeue-uuid-hubs","dry_run":false}' # wet: projected = 0 on their notices
# the next daily (incremental) fold re-plans them whole and splits them
```

`requeue-uuid-hubs` reads `hub_tender_ids` from the stored `procedure-key-census` report. It
refuses when no report is stored, or when the report predates unit 2: re-run the census first. It
re-queues every notice of those Tenders (`tender_versions` seeks, then issue 323's re-queue).
That is a few thousand notices, a normal daily-sized plan. A full re-plan (`project` with
`rebuild`) applies the gate too, without the re-queue.

The list is **Tender-scoped**, the gate **key-scoped**, so they do not coincide. The census clusters
every notice of a Tender, including another key's notices a 481 link welded in, so a listed Tender can
reach 3 clusters only through those and stay one Tender after the re-queue (wasted work, harmless).
And the census walks only Tenders whose surviving key is a UUID: a hub whose groups a link merged
under a non-UUID key is never listed, and only a full re-plan splits it. So "verify the split" means
the gate's own count: the fold job row's `uuid hubs refused` and the journal's `group step uuid-hubs`.

### `buyer-role-census`: contractors, review bodies and platforms in the buyer slot (issue 483)

Award notices sometimes name the contractor, a review body or a platform vendor in the buyer role
(`Procedure-Buyer`, legacy and sdk-0.1 `buyer`). Every buyer-based guard trusts that role (481's link
guard, 482's hub gate, 369's key election), and the served `parties[]` names it. The 482 two-cluster
read found it in 16 of 45 false splits (16698: buyer and tenderer SWAPPED — the contractor Ratio Web
Sp. z o.o. as buyer and signatory, the real buyer Instytut Adama Mickiewicza as tenderer; 533381: the
Tribunal Català de Contractes; 438807: the UZP appeals department, with the real buyer POLREGIO only
receiving tenders and paying; 198229: European Dynamics). This census sizes it before a fix is
chosen: demote the role at projection, or only ignore it in the guards.

```sh
/root/aj.sh /admin/jobs '{"kind":"buyer-role-census"}'              # stride 10 (default): one window in ten
/root/aj.sh /admin/jobs '{"kind":"buyer-role-census","stride":1}'   # every window (the better part of a day)
/root/aj.sh /admin/reports/buyer-role-census | jq -r .body | jq '{stride, windows_read, windows, notices, notices_with_buyers, buyer_mentions, flagged_notices, decisively_flagged_notices, no_clean_buyer, classes: (.classes | map_values({decisive, mentions, notices, no_clean_buyer, every_buyer}))}'
# Source × subtype × class, with the denominators:
/root/aj.sh /admin/reports/buyer-role-census | jq -r .body | jq -r '.cells | to_entries | sort_by(-.value)[:60][] | "\(.value)\t\(.key)"'
/root/aj.sh /admin/reports/buyer-role-census | jq -r .body | jq -r '.read | to_entries | sort_by(-.value)[:40][] | "\(.value)\t\(.key)"'
# Procedure type (BT-105) × class, and which name-list entries dominate:
/root/aj.sh /admin/reports/buyer-role-census | jq -r .body | jq -r '.procedures, .patterns | to_entries | sort_by(-.value)[:40][] | "\(.value)\t\(.key)"'
# The samples of one class: publication, flagged name, why, the notice's other buyers.
/root/aj.sh /admin/reports/buyer-role-census | jq -r .body | jq -r '.classes["buyer-tenderer-swap"].samples[] | "\(.publication) \(.subtype) \(.procedure_type) \(.flagged)  <\(.basis|join("; "))>  clean_left=\(.clean_buyer_left)  others: \(.other_buyers|join(" | "))"'
```

**Read-only** (no dry flag; it writes nothing but its report), stoppable between windows and between
chunks, report `buyer-role-census` stored only by a run that finished. It walks the parsed notices by
id in windows of 20,000 ids, 1,000 notices per read (`Db::parsed_window`: the projection's own
chunk read, bounded to the window), and reads one window in every `stride` (default 10, `0` reads as
1). The windows it reads are evenly spread over the id space, so over every era and Source; the counts
are **of the sample**, so multiply by `stride` for a corpus estimate. Each notice is fully parsed
(DE-1.x folded first) and its organization roles read from its own id-refs (a nested Organization's
inner half lands on its outer one, as the projection binds it; sdk-0.1's `ContractingParty` /
`WinningParty` sections are buyer / winner). Each mention's resolved organization comes from
`organization_mentions`.

Each buyer mention gets every class that applies (`basis` says what each matched):

| class | when | decisive |
|---|---|---|
| `contractor-same-section` | its own Organization is also referenced as a winner, tenderer, main or subcontractor on the notice, in ANY lot | no |
| `contractor-org-same-name` | another section with the same resolved organization AND the same folded name is such a party | no |
| `contractor-org-other-name` | the same resolved organization under a DIFFERENT name is such a party (a resolver fusion — shared switchboard ids, the PL823 stub, bare DE ids — or an in-house award to an Eigenbetrieb sharing its authority's id) | no |
| `contractor-name` | no organization match, but its folded name equals such a party's | no |
| `buyer-tenderer-swap` | the roles swapped (16698, 299165): the buyer carries a commercial legal form (`COMMERCIAL_FORMS`: sp. z o.o., S.A., GmbH, s.r.o., …) and is not public-shaped, while a tenderer that is not itself a buyer is public-shaped (`PUBLIC_STEMS`: instytut, uniwersyte…, gmina, stadt, ministry, …) with no commercial form | no |
| `swap-legal-form` | the weak half: a commercial buyer and a tenderer with no commercial form that is not public-shaped either (a person, an association, an unsuffixed name) | no |
| `real-buyer-elsewhere` | it holds no buyer-shaped role (tender receipt / evaluation, additional information, paying, financing, signatory, documents provider; legacy `tender-receipt`, `further-information`, `specifications-provider`) while another organization, neither buyer nor contractor, does (438807). Its `basis` names that organization: the buyer a demote could recover | no |
| `review-body-name` | its name is a known review body (`NAME_PATTERNS` in `role_census.rs`: KIO, UZP Departament Odwołań, Vergabekammer, ÚOHS, Förvaltningsrätten, Tribunal Català, TACRC, TAR, tribunal administratif, …) AND the notice agrees: `real-buyer-elsewhere` holds, or another buyer mention is not a review-body name | yes |
| `review-body-name-alone` | the name alone, or with only its own review(-adjacent) role: a court, KIO or ÚOHS buying in its own name is its own review body and looks exactly like this | no |
| `review-body-role` | the notice's own review-body role (eForms `Lot-ReviewOrg`, `Part-ReviewOrg`, `ReviewBody`; legacy `ADDRESS_REVIEW_BODY`, `APPEAL_PROCEDURE_BODY_RESPONSIBLE`, `RESPONSIBLE_FOR_APPEAL_PROCEDURES`) names the same organization or name | no |
| `review-info-role` | it is the appeals-information body or mediator (eForms `Lot-`/`Part-ReviewInfo`, `Lot-`/`Part-Mediator`; legacy `ADDRESS_REVIEW_INFO`, `mediation-body`, `appeal-information`) | no |
| `esender` | it is the notice's eSender / procurement service provider (`Procedure-SProvider`): a buyer sending its own notices | no |
| `docs-provider` | it is the documents provider (`Lot-`/`Part-DocProvider`, legacy `specifications-provider`) | no |
| `platform-name` | its name is a known platform vendor (European Dynamics, EU-Supply, Mercell, Vortal, cosinex, subreport, DTVP) | yes |

A buyer mention is **clean** when no decisive class flags it. **`no_clean_buyer`** counts notices
naming buyers of which none is clean: the notice's only buyer is wrong, and demoting the role would
leave it buyerless. Per class: `mentions`, `notices`, `no_clean_buyer` (of its notices) and
`every_buyer` (its notices whose every buyer mention carries this class, the measure for a
non-decisive class), plus 30 samples (bottom-30 by a hash of the notice id, the same whatever the
stride or window; each with its BT-105 `procedure_type`). `read` counts notices read per
`source/subtype` (`legacy` for the legacy TED profiles, `-` for none), the denominators of `cells`
(`source/subtype/class` and `source/subtype/no-clean-buyer`). `procedures` counts notices per
`procedure type/class` (in-house and negotiated-without-call awards as their own cells), `patterns`
per `class/list label` for the name-list classes (which entries dominate, e.g. a short token like
`kio`, `tar` or `kofa` matching real buyers).

Decisions in the classes. **Which are decisive was set by the first run** (2026-10-03, job 1942,
stride 10: 1,470,018 notices read, 3,151 decisively flagged under the first rules, 30 samples per
class read; evidence in `.scratch/tender-db/483-roles/`): only the corroborated `review-body-name`
and `platform-name` held up.
- **The contractor classes count, they do not decide.** In every sample it was the CONTRACTOR slot
  that held the buyer (Gobierno Vasco named as its own supplier by a legacy text notice, Stadt
  Hilden, Kent County Council, SPMS: a winner block repeating the authority, an in-house award),
  never a contractor in the buyer slot. The buyer mention is the right one.
- **The swap is read from legal forms**, because in 16698 and 299165 neither organization holds
  both roles, so no contractor class sees it. Not decisive: 2-3 of 30 samples were real swaps (a
  Ziviltechniker GmbH as buyer and Stadt Köln as tenderer); the rest were company buyers awarding
  to a public institute (PKP PLK and the Instytut Kolejnictwa, Hrvatske ceste and Institut IGH,
  Dresdner Verkehrsbetriebe and TU Dresden).
- **The eSender is not decisive.** 2,351 of the first run's 2,993 no-clean-buyer notices were
  buyers sending their own notices (Sprinkenhof, a Berlin Senatsverwaltung, Gmina Cieszyn, the
  Département de l'Aube). A platform vendor in the slot is `platform-name`.
- **A review-body name is decisive only when corroborated by ANOTHER party**: a recoverable real
  buyer (`real-buyer-elsewhere`: a Vergabekammer beside the Staatliches Bauamt, the High Court of
  Ireland beside the OPW) or another, non-review buyer. Its own review role does not corroborate:
  KIO, ÚVO, ÚOHS and the tribunaux administratifs buying for themselves are their own review body.
  The bare `tar` matched a Hungarian village (Tar Község); TAR is listed only with its region.
- **The review-body role, review information and the documents provider are not decisive.** A
  buyer writing its own name into those blocks (common in UK and IE notices) is the normal case or a
  mis-tag of THAT role. Counted and sampled to size them.
- **Left out of the name lists:** `Commissione` (the European Commission buys), the Polish Urząd
  Zamówień Publicznych as a whole (it also buys for itself; its `Departament Odwołań` is listed). A
  list entry is data: a row in `NAME_PATTERNS`, written in its folded form (a test holds each to its
  own fold), matched as whole words.
- **Not yet read** (issue 483, deferred): whether a contractor is the winner or a losing tenderer;
  whether the tenderer of a swap is a buyer on other notices (needs an org-level role index).

**Expected cost.** About 3 ms a notice for the parse and the mention read (issue 482's measure on
`parsed_by_ids`; the census reads contiguous windows, which should cost less). So stride 10 parses a
tenth of the corpus in a few hours, and stride 1 takes the better part of a day. It holds no writer,
but it DOES hold the single job worker for its whole run, so queue it right after a daily. It keeps no
checkpoint: a restart re-runs it from notice id 0. Read the job row's duration for the real figure.

**What to decide from it.** Read `no_clean_buyer` with the swap and `real-buyer-elsewhere` classes
in view: before they existed the census could not see 16698's shape at all, and a small number would
have read as "demote is safe". A large `no_clean_buyer` (the notice's only buyer is wrong) says demote
at projection only where the real buyer can be recovered (`real-buyer-elsewhere`'s basis, the swap's
tenderer), and otherwise make the guards ignore the flagged role. A `contractor-*` or swap mass
concentrated in a few Sources × subtypes (award notices from one publisher) points at a parser fix for
that publisher. Read 30 samples per class on the portals first.

### Void-lot party names and `refold-void-names` (issue 510)

A party slot that says the lot was not awarded — `Infructueux`, `Lot déclaré infructueux`, `Sans suite`,
`Desierto`, `Lotto deserto`, `Niet gegund`, `Unieważniony`, `Not awarded`, and TED's multilingual
templates in every language their notices print (`Kontrakten ikke tildelt.`, `Los wurde nicht vergeben`,
`This lot was unsuccessful.`, `entfällt — Los nicht vergeben`, `Zuschlag nicht vergeben`: issue 525) — is
not a party.
`ingest::partyname::not_a_name` decides it (one list for every era and for the text parser; decision and
measurements in `.scratch/tender-db/510-void-names/unit1-decision.md`), and the fold applies its
`VoidLot` class in every era and role: such a party mints no organization, carries no role, result winner
or bid party, and a legacy or sdk-0.1 result left with no real winner reads `clos-nw` (the void phrase
used to be the "winner" and manufacture `selec-w`). eForms keeps the publisher's BT-142. Issue 511
widened the rule to placeholders and blank slots (next section); a void phrase beside an award
(`Lot 1) Sarl Bremond. Lot 2) Infructueux`) is an `AwardSummary` the fold keeps.

A refold of a notice that still has a recorded void mention RETIRES it (`Db::retire_mentions`): the
mention and the party / bid-party rows naming it are deleted and their Tenders stamped stale, in one
transaction that also writes ADR-0017 D5's correction rows for the deleted party rows (the fold's
compare reads after the delete, so a Tender whose only void footprint was a party row would otherwise
change silently); the fold's line reads `N non-name mention(s) retired, M correction row(s) announced
— issues 510/511` and queues the orphan sweep. `refold-void-names` finds the standing ones:

```sh
/root/aj.sh /admin/jobs '{"kind":"refold-void-names"}'                             # dry: counts
/root/aj.sh /admin/reports/void-names-refold | jq -r .body \
  | jq '{orgs, mentions, notices, notices_by_profile, identified, legal_form, long, exempted: (.exempted|length), top: .top[:10]}'
/root/aj.sh /admin/jobs '{"kind":"refold-void-names","dry_run":false,"expect":N}'  # wet + the fold
```

The walk reads every organization by id window and nets each head name that carries a void phrase
(`partyname::mentions_void`, never SQL `LIKE`). The net is a name test; the split is the fold's own
PARTY decision (issue 525). The fold drops a party only when every published copy of its name is a
non-name (void, placeholder or blank), or when it has four or more copies and at least half read void.
TED's multilingual winner slots print two dozen copies, a bilingual award two. So a void head says
nothing about the party: `Lot not awarded` heading 22 copies the lexicon reads as names is KEPT, and
served as a winner. The walk therefore reads every recorded mention of the net's orgs with its section
(`Db::mention_sections_of_orgs`, through `organization_mentions_org`) and each mentioned notice's
stored parse (`parsed_by_ids`, 200 per read), and asks `ingest::project::fold_parties`: the fold's
`drop_decision` with the fold's own inputs (the parse as the fold reads it, a DE-1.x one through
`normalise_de1`'s aliases; the sdk-0.1 flag from the profile; the section map; the nested-Organization
aliases). A mention is re-queued only when the fold drops its section, outer or
nested inner half (since issue 525 the fold retires both keys), or when the walk cannot read it (no
parse, no such party section). Every other mention is listed in `exempted` with the fold's reason
(`party kept: 6/23 names void`, `…, an award summary among them`, `…, a gated identifier`, `party kept:
no name`) and up to three of its notices. Read a kept void template's copies there
(`/v1/notices/<id>/content`): the ones no list holds belong in `partyname`'s `VOID_TEMPLATES`, exact
values only (a template that goes on is national free text, never measured as a prefix). A net org mentioned nowhere keeps the head rule (`partyname::is_void_lot`: counted, for the
orphan sweep; else exempted as `names an award`). What `partyname::names_an_award` keeps (a per-lot
summary such as `Lot 1) Sarl Bremond. Lot 2) Infructueux`, a company form, an awardee word) is an
award summary the fold never drops, so it lands in `exempted` too. Dropping such a "name" would turn
the awards it lists into `clos-nw` (the first dry run, job 2164, stopped on exactly these). So every
mention of the net's orgs the paired fold will retire is re-queued, and a dry re-walk after the fold
lists no notice. The re-queued mentions are a subset of what that fold retires, not all of it: it
retires every dropped party key of each re-queued notice, of every class and org, nested inner halves
included, and whatever else its delta drops. A fold `retired` count above the dry run's mention count
is expected. One below it means mentions re-queued only because the walk could not read their parse,
or a corpus that moved between the runs: read those before the next drain. Before issue 525, the split was `is_void_lot` on the head name, the fold's rule for ONE name.
It re-listed 37 orgs / 40 mentions the fold keeps after every drain (dry 2351, wet 2352, fold 2353 `0
non-name retired`, dry 2354). The net still reads the head name only, not every `organization_names`
copy (about 39M rows, several times the org walk): a party the majority rule drops while its head
copy reads as a name stays outside the net, so add the template's copies to the lexicon when one
shows up. `long` lists every void name over 120 characters (summaries the exemption missed) and
`distinct` the void names by fold. An org with mentions on both sides is in `top` and in `exempted`.
Issue 520 U4: a mention whose party the fold keeps under its award summary's one winner
(`partyname::award_winner`, `… puis attribué à la Société Arrow Génériques suite à …` → `Arrow
Génériques`) is re-queued too and listed under `extracted` (`{org, name, winner, notice}`, counted on the
job line, its notices in `notices` and `expect`); the paired fold rebinds each to the winner's
organization and retires none (`mentions_rebound`, and the fold row's `extracted winners (issue 520)`
sample). An organization that carries an identifier is exempted instead, its reason naming the winner:
the resolver binds its mention by the identifier, so no rename moves it to another organization (a
paired fold over a re-queued sibling notice still rewrites that mention's name and its void-net
variants to the winner: `mentions_refreshed`, not `mentions_rebound`). The fold row's `extracted
winners` counts only the heads it renames on mentions no identifier binds, so read `mentions_rebound`
as what moved. Before that wet run (U5) read
every row against 520's unit-1 App. B and stop on one not there: `jq -r '.extracted[] | [.org, .winner,
.notice] | @tsv'` on the report body, and `jq -r '.exempted[] | select(.reason | test("issue 520")) |
[.org, .reason] | @tsv'` for the identified ones.
Issue 520 U6: the same path also reads a winner named before a void tail
(`partyname::void_tail_winner`: five reviewed rows only, read whole on the fold — `BGE Guyane 16 rue Lt
Becker 97300 Cayenne / La consultation a été déclarée sans suite pour les parts non couvertes des besoins` →
`BGE Guyane`, and two `GRPT … (mandataire) …, suite à procédure négociée après AO infructueux` → the
consortium). Two review panels broke every general shape, so a new such summary stays as published until its
row is reviewed and added to `A2_REVIEWED`. Its rows land under `extracted` too. Read each one before the
wet run, the same way.
Before the wet run read three lists:
`notices_by_profile` against the census shape (r208 ≈ 8.5k, text ≈ 0.7k, internal-ojs ≈ 150, r209 ≈ 140,
eForms ≈ 50 notices), `identified` (matches carrying an identifier) and `legal_form` (matches whose name
also holds a commercial legal form — the design lists them instead of guarding on them). **Stop and add
an exemption** if an identifier-bearing or legal-form org reads as a company. Wet needs `expect` (the dry
run's notice count) and aborts, writing nothing, when it finds more than a quarter above it. Then read the
fold's `retired` count (≈ the dry mention count), and run `sweep-orphan-orgs` dry → read → wet (above
10,000 orphans the auto sweep only records the plan).

### Placeholder and blank party names and `refold-placeholder-names` (issue 511)

A party slot that names no organization is not a party either: a pointer (`See section VI.3`, `siehe
oben`, `Véase perfil del contratante`, `See Contracts Finder Notice for full supplier list`), a withheld
or missing name (`Withheld Section94 Supplier`, `keine Angabe`, `TBC`, `Unknown`), a plural (`Various
suppliers`, `Diverse Firmen`) — the `Placeholder` class — or an empty slot (`N/A`, `nie dotyczy`,
`néant`, `no procede`, `.`, `0`) — the `Blank` class. `ingest::partyname::not_a_name_in` decides it
(decision, lexicon and guards in `.scratch/tender-db/511-placeholders/unit1-decision.md`), and the fold
drops the party in every role and era, the buyer slot included: no organization, no role, no result
winner, no bid party. What a legacy or sdk-0.1 result left with no real winner reads depends on the
class: `Placeholder` → `selec-w` with no winner (the text says a winner exists); `Blank` → no marker, so
the evidence rule decides (a value selects, no date closes, a date alone stays NULL; sdk-0.1 stays NULL
unless coded). eForms keeps the publisher's BT-142 and the Bid. A placeholder buyer leaves the notice
with no buyer tokens and no buyer key — the designed "names no buyer" — and its signatories are not read
on their own.

What the fold keeps, by design: an `AwardSummary` (a pointer followed by a name, `siehe oben: NEB
Betriebsgesellschaft mbH`; a stem inside a list or beside a company form); a `Blank` party carrying an
identifier the gate admits that does not echo the non-name (guard 3: `0` with a CVR, a DE platform GUID
on `.`); `ingen` / `inget` / `nd` outside a Danish, Swedish, Norwegian (resp. Polish, Italian) notice or
in a brand's case (`InGen`); and a stem that does not open the name (`Siemens – see section VI.3`).

Every fold's job row carries `dropped parties (issue 511): N void, N placeholder (N with an identifier:
<sample>), N blank` — the daily rate, and up to 20 identified placeholders (`notice/section 'name'
[identifier]`). **Between the deploy and the wet run, read that sample on every daily fold**: look up the
organization each identifier keys (`organizations_identity`) and stop if one is a real supplier — that
shape needs an identifier guard on `Placeholder` first. A small dip in `data_quality`'s winner-chain
rate is expected (withheld results become `selec-w` with no winner, dated blank ones NULL).

`refold-placeholder-names` is `refold-void-names` generalised by class (one fn, `run_refold_non_names`).
It still splits its net by the head name: it re-queues every hit but an award summary, and the fold
decides the rest. Issue 525's party split is the void walk's alone so far:

```sh
/root/aj.sh /admin/jobs '{"kind":"refold-placeholder-names"}'                             # dry, straight after deploy
/root/aj.sh /admin/reports/placeholder-names-refold | jq -r .body \
  | jq '{orgs, mentions, notices, notices_by_profile, by_class, buyer_rows, read_before_wet,
         identified: (.identified|length), identified_blank: (.identified_blank|length),
         non_exact: (.non_exact|length), scoped: (.scoped|map_values(length)), exempted: (.exempted|length)}'
/root/aj.sh /admin/jobs '{"kind":"refold-placeholder-names","dry_run":false,"expect":N}'  # wet + the fold
```

Since issue 511 unit 5 a SECTION-REFERENCE POINTER is `Placeholder` too (`partyname::section_reference_pointer`):
a name made of nothing but closed-list words — a pointer in any language the dry plan and TED's
template hold (`siehe`, `see`, `wie`, `jak w`, `cf`, `voir`, `vedasi`, `zie`, `όπως`, `véase`, …), locators
(`Ziffer`, `pkt`, `rubrique`, `punto`), positions (`above`, `ci-dessus`), the notice (`der Bekanntmachung`)
and form titles or section I's labels — around form section references (`VI.4.1`, `I.1)`): `siehe VI.4.1`,
`As in I.1 above`, `jak w pkt VI.4.1) ogłoszenia`, and the award heading alone in a winner slot, `V.1) AWARD
AND CONTRACT VALUE:`. A body's name beside the pointer keeps it a name (`Krajowa Izba Odwoławcza – jak
wyżej w pkt VI.4.1)`, `Referat II.3`), and so does a company named in list words where a body's name
stands (the unit 5 review): after a separator past the reference (`See section VI.2: Avis` stays the
`AwardSummary` it was), in a lot-numbered winner slot (`I.4) Avis`, `I.3) Point P`: a label closes on
labels), or a locator that points at no reference (`Point P (voir I.1)`). Dry 2283 re-run through it
(`.scratch/tender-db/511-placeholders/u5-build/simulation.md`): 1,319 `name` folds / 19,993 orgs / 53,302
mentions become `Placeholder`.

The walk's net (`partyname::mentions_placeholder`) is the raw-name arm, any placeholder or blank stem at
any word boundary and every whole value, scoped words included, the section-reference pointers, and every
name the fold's TED translation echo can sweep (`echo_shaped`): a form section number in a pointer-led
copy of twelve words or fewer with no legal form. The echo drops such a copy with its party's bare
pointer, and a non-English original makes that copy the party's head name. Unit 4's echo arm took any
short name with a section number (dry 2283: 25,333 orgs, 1,277 buyer rows, `Stadtverwaltung
Kaiserslautern - Stabstelle IV.1 …`); unit 5 narrowed the echo and its arm to pointer-led copies, and
the same re-run puts the net at about 22,450 orgs and 114 buyer rows. The echo still seeds only from
unit 4's bare placeholders (`partyname::echo_seed`), never from a pointer only unit 5 classes (a
bilingual award's FI `Katso kohta VI.4.1` would sweep its SV copy naming the review body), and sweeps
only a copy a `SECTION_POINTERS` word opens — so a TED pointer party that survives the canary means an
opener or a title is missing from `partyname`'s lists (Croatian, TED's 24th language from 2013 on, is
reconstructed there from the form labels, not read off a notice). The net is a superset of the fold's
drops by contract (pinned name by name and party by party). Every hit but an `AwardSummary` is
re-queued, so the fold — not the walk — decides the anchor, the language scope and guard 3, and a hit
the fold keeps costs a refold and nothing else. `exempted` lists the award summaries, never re-queued.
Its index reads (`tender_version_parties_org_role`, `tenders_procedure_key`, `tender_versions_notice`)
are deferred indexes: **never run it during a full rebuild**. The two role-split readers send `IN` lists
512 ids wide (`IN_CHUNK`); `the_placeholder_plan_readers_seek_the_org_role_index_at_the_chunk_width` pins
`SEARCH … USING INDEX tender_version_parties_org_role` at that width (511's plan-width rule). Run the dry
straight after the deploy, or
add the intervening folds' `retired` counts back (those folds retire placeholder mentions and their auto
sweeps take the one-mention tail).

1. **Dry, then read** (mandatory, all of them; `read_before_wet` names the lists):
   - `non_exact`: every distinct fold that is not exactly a lexicon entry (stem hits with more text,
     mid-string hits the anchor refuses, keep-rule and summary names), with classes, spellings, orgs,
     mentions and roles. If it ever proves unreadable, the floor is 200 random plus every one with
     winner, Tenderer or buyer rows, and reading less than all is recorded with its reason;
   - `identified` (placeholders with an identifier) and `identified_blank` (blank slots with an
     identifier, `echo: false` = guard 3 keeps it), with the identifier values — 23186509's VAT among them;
   - `legal_form` and `digit_only` (the zeros the raw-name arm takes; any other digit-only name is a name);
   - `scoped`: `in_scope` (DK/SE/NO/FI for `ingen`/`inget`, PL/IT for `nd`, by the org's country as a
     proxy), `out_of_scope` (expected kept: 1188404 FR, `InGen`) and `unknown` (no country).

   **Stop and add a keep test** if any of them reads as a real party. **Stop** if orgs fall below ~6,450
   or mentions below ~37,000 after the add-back (the predicate or the walk regressed), or if `buyer_rows`
   exceeds 1,000 (the net reached real buyers).
2. **Buyer canary.** `buyer_notices` (each with its Tender and procedure key, as `Ident` reads it) and
   `key_families` (each key's Tenders, refused splits included, and their notices;
   `key_families_over_cap` names a family past 200 Tenders for a read by hand) are the pre-image. Run
   `refold-notices` on the buyer notices (cap 1,000), let its fold run, and re-read the same rows with
   bounded `/v1/sql` reads. **Stop** if more than 10 % of the buyer notices changed Tender or any key
   family changed beyond them; read each change before the main run. **Unit 4 adds two notices**:
   12516947 (ORG-2) and 19530163 (ORG-10), TED's pointer winner in 24 languages. After their fold,
   `SELECT organization_id, name FROM organization_mentions WHERE notice_id = ? AND section_id = ?` (a PK
   seek) must return no row. The echo test's PL, SK, SL, GA, BG, RO and HR strings are reconstructions,
   and one real copy that writes the number otherwise keeps the whole party. **Stop** if either survives.
3. **Wet.** `expect` is the dry run's notice count (it aborts, writing nothing, above +25 %); its paired
   fold's `retired` must reach the dry re-queued mentions minus the `identified_blank` mentions with a
   gated identifier and the `scoped.out_of_scope` / `unknown` mentions — below that floor, stop and read
   the gap. Above the dry mentions is a read trigger, not a stop (touched-expansion notices, identifier-
   bound mentions on real-headed orgs): read the excess, starting with the fold's identified sample.
   `dropped` (void + placeholder + blank) is at least `retired`.
4. **Sweep.** The fold queues `sweep-orphan-orgs` (auto); above 10,000 orphans it only records the plan
   (read `protected`, then run wet).

### The buyer-role demote and `refold-buyer-roles` (issue 483 unit 2)

**At projection**, a buyer mention a decisive census class flags (`review-body-name`, `platform-name`
above) is not served as the Tender's buyer. The rule is `role_census::buyer_fix`, over the census's
own verdicts (`judge`), so the census measures exactly what the projection demotes:

- a **clean buyer mention is left** on the notice: the flagged mention loses its buyer role (KIO,
  a tribunal administratif or European Dynamics beside the real buyer);
- **none is left** and `real-buyer-elsewhere` holds: the first other party (section order) in a
  STRONG buyer-shaped role (tender receipt or evaluation, additional information, paying) is
  promoted to the dialect's buyer role (`Procedure-Buyer`, legacy / sdk-0.1 `buyer`)
  and the flagged mention dropped — 438807's POLREGIO, the Staatliches Bauamt beside a
  Vergabekammer, the OPW beside the High Court. **Never promoted**, the next eligible party is
  tried instead (`role_census::promotable`): a portal or platform label (`NameList::Platform` /
  `Portal`: "Digitaal via TenderNed", Negometrix, achatpublic, …), the eSender
  (`Procedure-SProvider`), a review body by role or by name, a party whose only buyer-shaped role
  is the documents provider or the financing party, a nameless party, **the contract signatory
  alone** (swapped notices file the winner there: CAMFIL POLSKA signing for NCBJ, Wackler for the
  BImA), **a company that does not pay or finance** (a commercial legal form and no public stem:
  the suppliers and tender agents — Roche Diagnostics Polska, Braun GmbH, PSI BV, a Rechtsanwälte
  GmbH — of the first dry run, job 1954; POLREGIO S.A. pays), and a "name" of more than 16 words
  (a legacy free-text sentence). No eligible party: nothing changes — correct or unchanged;
- **nothing recoverable** (Mercell alone): the role is served as published, AND stays in the guards
  (one verdict for both; the 2026-10-03 Decision's guard-only drop was reversed by unit 2).

The demoted mention keeps its other roles (a Vergabekammer stays the review body). One verdict feeds
both readers: `NoticeState::read` (the served `parties[]`, `v_tender_buyers`, organization
statistics) and `buyer_side_mentions` (369's buyer key, 481's guard tokens and sections, 482's hub
key; the procedure-key census reads the same). Both fold paths call them, so the full and the daily
fold agree by construction. The verdict is parsed-side (no resolved organizations: the plan row is
read before Phase 1), and gated cheaply: only a notice whose BUYER's name holds a review-body or
platform pattern reads its mentions a second time. A demoted mention that also signs the contract
leaves the guards' signatory side too.

**A full rebuild must not straddle the deploy.** A plan built before unit 2 carries buyer keys and
guard tokens read off the raw slot; the plan DDL now creates the marker table `plan_buyer_demote`
and `plan_is_complete` refuses to resume a plan without it (the rebuild plans again).

**Existing rows** change only when a fold re-derives them. `refold-buyer-roles` finds them:

```sh
/root/aj.sh /admin/jobs '{"kind":"refold-buyer-roles"}'                 # dry (the default): finds and counts
/root/aj.sh /admin/reports/buyer-role-refold | jq -r .body | jq '{dry_run, named_mentions, candidates, fixed_total, requeued, stamped}'
/root/aj.sh /admin/reports/buyer-role-refold | jq -r .body | jq -r '.fixed[] | "\(.publication)  drop \(.dropped|join(" | "))  promote \(.promoted|join(" | "))"' | head -40
/root/aj.sh /admin/jobs '{"kind":"refold-buyer-roles","dry_run":false}' # wet: projected = 0 + epoch-stale Tenders
# the next daily (incremental) fold re-plans exactly those notices, with the Tenders they sit in
```

Read the dry report's `promoted` column before going wet: a promote names the organization the
notice will serve as buyer, and that cohort (not census-1943's samples, which bound resolved
organizations where the projection compares by folded name) is the validation set. The job is a
heavy-write kind dry or wet, so the dry walk too holds the heavy-write belt (coverage refreshes skip)
for its length: run it in a queue gap.

**Only the fixed notices are re-queued, not their whole Tenders**, and that is enough: the daily
moves a re-queued notice between Tenders in both directions. A Vergabekammer CAN refused its CN's
OPP-090 link (buyer-disjoint) joins the CN once the Bauamt is promoted, and its old Tender is
retired; a CAN that joined its CN only through a shared KIO buyer splits out once KIO is dropped,
while the CN (unchanged verdict, not re-queued) keeps its Tender alone. Pinned by
`refold_buyer_roles_moves_a_notice_between_tenders_on_the_daily` (ingest, `project_incremental.rs`:
the full and the daily path byte-identical at every step, and equal to a fresh rebuild).

It walks `organization_mentions` in 250,000-notice-id strides (the `refold-denied-schemes` shape:
`name` has no index, so a fixed stride costs the same at any hit rate) and narrows in three steps:
mentions whose name holds a pattern (a big superset — every Polish notice naming KIO as review body);
of those, notices whose version SERVES that organization as a buyer (`tender_versions_notice`, then
the version's parties: the pre-unit-2 projection served the raw slot, so only these can change); of
those, notices whose parse gives a non-empty `buyer_fix`. A wet run re-queues them
(`unmark_projected_by_ids`) and stamps their Tenders epoch-stale (`stamp_stale_for_notices`: a
re-queue alone leaves each chain identical and the fold early-returns, issue 179). Stoppable between
strides; a stopped run stores no report (a wet run's finished strides stay re-queued). Job 1943
predicts ~1,400 decisively flagged notices corpus-wide (139 at stride 10): ~1,000 drop a flagged
mention beside a clean buyer, ~350 promote a recovered buyer, a few tens keep their role (nothing
recoverable, or a portal label). The walk is about one pass of the mentions table. Re-running it after the daily finds nothing: what the fold re-derived is no longer served
from the raw slot. A full re-plan (`project` with `rebuild`) applies the demote too, without the job.

**Verify** after the daily: `/v1/tenders/438807` names POLREGIO S.A. as buyer, not the UZP appeals
department, and a re-run of the dry job reports `0 whose buyer role the demote changes`.

### The buyer-equal winner flag `is_buyer` (issue 484 unit 3)

A winner the award notice names as its OWN buyer is flagged, not dropped:
`tender_version_result_winners.is_buyer = 1`. The verdict is the census's
(`role_census::buyer_equal_winners`): the winner mention and a buyer mention of the SAME notice are
one Organization section (nested halves folded), or their names fold equal (case, Latin accents) and
are non-empty and not a withheld-name placeholder (`N/A`, `Unknown`, `Confidential`, …:
`role_census::NON_NAME_FOLDS`) — the census's `contractor-same-section` ∪ `contractor-org-same-name` ∪
`contractor-name` (the census classes still count the placeholder pairs; the flag does not). A
text-era `<Authority>, <Unit>` in-house entry (1200610's `Staffanstorps kommun, Städservice`) is
minted under the authority's name by the parse, so it is flagged — on purpose: the notice publishes
no other name for it. The same resolved organization under another name is NOT enough
(`contractor-org-other-name`: an Eigenbetrieb or a kommun's Städservice shares its authority's id and
is a real supplier). Buyers are read after the 483 demote (a demoted review body is no buyer, a
promoted real buyer is one). Parsed-side, in `NoticeState::read`, so the full and the daily fold agree
by construction; a result's org is flagged only when EVERY section that bound it there is
(`LotResultState::buyer_winners`, per round, carried forward with its round).

- **Values:** 1 or NULL, never 0. NULL is "not buyer-equal" AND "written before unit 3"; both are
  served and counted exactly as before the change. The column came by `store::MIGRATIONS` at open
  (nullable, no default: metadata-only, O(1) on the 127.8M rows).
- **Served:** `"is_buyer": true` on `lot_results[].winners[]` and on the derived `winner` /
  `Tenderer` party (key absent otherwise); `v_lot_results.winner_is_buyer` / `v_awards`.
- **Counted:** `?winner=` (REST, SSE, webhooks) excludes flagged rows (`w.is_buyer IS NULL` in the
  per-row EXISTS and in the seeded walks' head-level role clause). The seed window and `org_reachable`
  are unchanged supersets: a window of only flagged tenders pages short with a cursor. `bidder=`,
  data-quality's `winner` coverage and `tender_db_dq_winner_named_rate` are unchanged on purpose.
- **Repairs:** the org-merge collapse (`repoint_org_references`, every merge rule) keeps a moved
  row's flag, and when a loser's duplicate collapses onto a survivor row on the same result the
  survivor stays flagged only if BOTH were (`clear_collapsed_buyer_flags`: the fold's every-section
  rule — a fused org bound by a non-buyer section is no buyer win). `repair_placeholder_orgs_batch`
  copies the source row's flag through its repoint, with the same both-flagged rule on a collision.
  The rehoming repairs update `organization_id` only. A merge can still CREATE a new equality the
  flag does not see (an in-house unit fused into its authority); that is the recall side, read at
  the next re-fold.
- **Parties:** the detail's `winner` / `Tenderer` party is flagged at read when every winner row of
  its organization on the party's own results (its notice's results on its lot; all of the
  notice's results when it has no lot) is flagged — never on (notice, org) alone, which would flag
  Morsø's road unit beside the buyer's own section.

**Rollout.** No `PROJECTION_EPOCH` bump (that would stale 8.7M Tenders, issue 179) and no job of
its own:

```sh
# 1. Deploy in a queue gap (never under a running project; the migration runs at open).
/root/aj.sh /admin/jobs | jq '.recent[:5], .queued'
# 2. Daily: every tender the daily rewrites is judged on write. Spot-check a recent PK window
#    (bounded /v1/sql, never retry a 408):
#    SELECT COUNT(*) FROM tender_version_result_winners
#     WHERE tender_id BETWEEN <recent lo> AND <recent hi> AND is_buyer = 1
# 3. Backfill: the NEXT all-profile `refold` another change batches (the issue-479 runbook above
#    lists it under "batched with"). Any profile-scoped refold before then backfills that profile.
#    Afterwards read the flagged rows per profile and compare with census 1981 × stride
#    (2 + 135 + 128 notices at stride 10: expect ~2,650 notices, several rows per multi-lot one).
```

**Verify** after the backfill: the tender of 3002722 serves `"is_buyer": true` on its winner, and
`?winner=<its Consejería org>` no longer lists it; `/v1/tenders/2959772` (Montte) and 8822638
(Microsoft) carry no flag; Morsø (24210321) as its published section names say — flagged only where
the winner IS the buyer's section or its name, never on the road unit under its own name.

### Reading a `process` job's `[process]` lines (issue 407)

Every package walk prints one journal line when it completes:

    [process] ted daily 2026-00180: 3424 members → 3424 notices (0 dup) in 25.1s (136.4 members/s, floor 8.8)

Two regimes, an order of magnitude apart, and never compared with each other: a **pure-dedup walk**
(`0 notices`, every member already held) runs at 1,700–15,000 members/s; a **writing walk** at
34–680. The clause after the rate is the guard: `floor N` is the median members/s of the last 30
writing walks of the same (source, kind) on this box divided by 10 (`store::jobs::RATE_FLOOR_DIVISOR`,
calibrated 2026-09-19 against a week of real lines whose natural spread was 5.3×; issue 404's
regression ran at 0.12), `floor pending n/5` means the history is still too short to have one, and
`— RATE ALARM (issue 407): under floor …` is the collapse. An alarm is repeated in the job's summary
(`; N RATE ALARM(S) (issue 407, ted daily): 2026-00180 at 0.1 members/s under floor 8.8`), because the
journal scrolls and the summary is what `/admin/jobs` keeps. Walks under 100 members are recorded
but never judged (fixed cost, not throughput); a stopped walk is neither. The rows live in
`package_rates` (notice layer, survives every rebuild).

The same ledger carries the **clean-walk watermark** (issue 419): a walk that declined nothing and
quarantined nothing is CLEAN, and a package whose newest walk was clean at its current fetch id cannot
yield anything on another walk. The tick's `… daily (all)` and `… monthly (all)` jobs skip those —
the summary says `; skipped N clean package(s) at their current fetch` — so the morning walk touches
the new package instead of every daily ever fetched (58 TED dailies, 127 s, on 2026-09-18). A
re-fetched package (TED's finality-window refetch) walks again; a package with policy-skipped members
keeps walking until an arm claims them; one with quarantined members until `reprocess` empties it.
`period=<one>` walks that package regardless.

### Sizing a `reparse`, because packages are the wrong unit (measured 2026-09-10)

`reparse` takes `profiles` and an optional `packages` cap, and the cap tempts you to think in
packages. Two behaviours make that misleading, both measured on prod:

- **A legacy re-parse above 500,000 un-projected notices forces a FULL corpus re-projection.**
  The legacy OJS closure's scoped-incremental path caps there (issue 305); above it the run
  announces `INCREMENTAL → FULL fallback BEFORE identity pass … re-projecting the whole corpus`
  and re-plans all ~14.4 M notices. A 20-package `ted-export-r209` run produced **879,331**
  notices and tripped it. **Size against 500,000 notices, not against packages** — at the
  measured density (~44,000 r209 notices per package) that is ~11 packages per chunk.
- **Package size is wildly uneven, and the first ones are not representative.** Packages 1–2
  walked 128,234 archive members for **283** matching notices in 55 s; packages 1–20 walked
  1,312,001 for **879,331** in 95 min. Extrapolating a per-package cost from the front of the
  archive is off by two orders of magnitude.

Also worth knowing before a small probe: **`reparse` stamps tenders epoch-stale by PROFILE, not by
the ids it touched.** Re-parsing 283 r209 notices stamped 2,131,375 tenders. That is deliberate
(a stale stamp forces a rewrite that recomputes identical content, a missed one silently loses the
re-parse), but a one-package probe does not have a one-package blast radius.

**There is no dry `reparse` (checked 2026-10-04, issue 484).** `reparse` replaces each matched
notice's parse in place and reports only counts (`re-parsed`, `unmatched`, `re-keyed`, `now failing`);
it has no `dry_run`, and nothing diffs the old layer against the new one (names gained/lost per notice).
So a parser change's effect is measured AROUND the wet run, not by it:
- **before**: the standing census that sees the defect (for winner/party changes, the
  `buyer-role-census` `contractor-*` classes — issue 483's job 1942 is a baseline) and the exhibit
  notices' stored parse via the public `GET /v1/notices/{id}/content`;
- **probe**: one package with `reclaim_only:true`, chosen with `after` so it holds an exhibit, then
  re-read that exhibit's `/content` — it serves the new parse before any fold;
- **wet**: the whole profile with `reclaim_only:true` — **from the profile's floor (no `after`)**, not from
  the probe's `after`: continuing from the probe skips every package below it (issue 484 left text
  packages 186–281 un-re-parsed that way, 2026-10-04) — then ONE `project` (above 500,000 un-projected
  notices it is the FULL fallback anyway);
- **after**: the same census at the same stride, and the issue's Verify. Read it per direction: a
  fix that removes junk can also READ more (issue 484's successor bound added 28% winners on the
  1993 fixture), so totals may rise while the defect class falls — count the totals before and
  after (a bounded read, or the census totals) rather than reading one net delta.

**Do not edit the working tree while a backgrounded `./deploy.sh` is running.** Its test gate runs
`ops/check.sh` LOCALLY, against the working tree as it is when the gate reaches it — not against the
committed ref it is deploying. On 2026-09-16 a deploy of `87d6a6d` failed on
`a_placeholder_publication_number_loses_to_the_notices_own_id`, a test that does not exist at that
commit: the gate had picked up the next issue's half-finished edits, including a predicate
temporarily short-circuited for a red-first check. It **failed closed** — nothing reached the box —
which is the right direction, but the deploy is wasted and the error names a commit that is not the
one being built, which reads as a mystery. Either let a deploy finish before starting the next edit,
or run `ops/check.sh` to green on a clean tree first (the deploy then skips its own gate when
`target/.tests-green` covers the commit it ships — see
[The test gate and the queue probe](#the-test-gate-and-the-queue-probe-issues-245-254-459)). Since
issue 459 an edit or a commit outside `.scratch/` during the gate's run — even one reverted before
it ends — leaves no marker, and the deploy refuses rather than shipping a green that describes
another tree.

**Read the `unmatched` and `re-keyed` counts before calling a re-parse complete (issue 290).**
`reparse_notice` finds its target by `(source, publication_id, content_hash)`. The hash is the same
bytes and is stable; `publication_id` is parser-EXTRACTED. So a parser change that also moves how
`publication_id` is derived makes the lookup miss its own targets, and those notices **keep their
old parse** while the run reports success — `unmatched` is documented as benign ("a package walk can
yield records the selection did not name"), which is exactly what makes it silent.

Two counters now make it legible, and the job summary calls each out when it is nonzero:

| counter | means | what to do |
|---|---|---|
| `re-keyed` | the row was found by `(source, content_hash)` after the full identity missed, its layer WAS replaced and its new `publication_id` adopted | nothing, if a re-key is what the run was for. Otherwise the parser changed identity derivation by accident — find out why |
| `unmatched` on a run that expected few | the row was not found by either key | **stop.** Either those members were never ingested (fine) or the derivation moved AND the hash is ambiguous (two notices of that source share the bytes), and those notices still carry their old parse |

The fallback is deliberately narrow: it fires only when the content hash names EXACTLY ONE notice of
that source, so it can never merge two notices on the strength of duplicate bytes, and it never
applies to the reclaim path, where a missing identity should mint rather than adopt.

**A re-parse that CHANGES identity derivation must not overlap the daily ingest (issue 404).**
The two jobs race for the same rows, and before the ingest path learned to adopt a moved key they
raced destructively: on 2026-09-16 issue 394's guard deployed at 07:00Z, a re-key campaign was
walking the 7,177 standing carriers in chunks, and `process doe daily (all)` ran at 07:58Z in the
middle of it. For every member the campaign had not yet reached, the new derivation named a key the
corpus did not hold, so `INSERT OR IGNORE` minted a SECOND row over the same archived bytes — 281 of
them — and the later chunk then matched the twin, which is why the last two chunks could report
`0 unmatched, 0 re-keyed` while the cohort stood still. The ingest path now adopts a moved key
instead of minting (issue 404), so the overlap no longer duplicates, but the jobs still race for the
same rows and the campaign is slower for it. So:

- **Either the campaign takes the queue** — pause the daily (or run it to completion first and
  enqueue the chunks back to back, checking `/admin/jobs` between them) —
- **or the derivation change ships AFTER the cohort is drained**, which is the ordering that never
  had the race to begin with.

Check `/admin/jobs` for a running or queued `process … daily` before enqueueing the first chunk, not
just before the last one: the daily is scheduled, so an idle queue now is not an idle queue in forty
minutes.

### Re-parsing the July-2011 `@FMTVAL` cohort (issue 471 unit 3)

**What changed.** The TED_EXPORT parser (`r209/parse.rs`, `Rule::Amount`, shared by `ted-export-r208`,
`ted-export-r209` and `internal-ojs`) no longer adopts an amount's `@FMTVAL` blind. When the element text
is an unambiguous number (`value::display_cents`: space/NBSP thousands, `20 550,54`, `13260.00`; a lone
`1.234` is a guess and is NOT read) and the two disagree:

- attribute = text × an exact EVEN `10^k`, `k >= 2` (the attribute the larger — the measured shape of
  TED's 2011 generator defect: `<VALUE_COST FMTVAL="49700000000000000">49 700`) → the TEXT is stored as an
  ORDINARY amount (no `quality` marker: electable and served like any other figure — owner decision
  2026-10-06), and the raw attribute is kept beside it in the parse layer (same section and ordinal, field
  id + `.FMTVAL_MISMATCH`) as the record of the correction; that row reaches no canonical fact. Compared
  in `i128`, so a 10¹²/10¹⁴ attribute that overflows the stored integer is still recognised;
- any other disagreement (text larger, odd `k`, not a power of ten) → the attribute is stored EXACTLY as
  before; the text is kept beside it as `.FMTVAL_TEXT`, parse-layer evidence only. That
  class was never measured outside the sampled 2011–2014 r208 months, so it changes no served value.

Rows whose text agrees, or is not an unambiguous number, read exactly as before. The legacy lot results
(`tender_version_lot_results.awarded_cents`) take the adopted text's figure too.

**Scope.** Standing rows change only on re-parse. The measured defect sits in 2011-04 … 2011-08 (the
July packages 2011-07-08 … 07-30 carry ~5,900 of the scaled attributes, plus four 46/69-element batches on
2011-04, 2011-05-26, 2011-06-03 and 2011-08-02; the notices are `R2.0.7` forms under the
`ted-export-r208` profile), so re-parse those packages only — not the era's ~15 M amounts. A later
whole-profile reparse (r208 or r209, for any other issue) is safe: outside the measured shape the
attribute is read exactly as before. The change does not touch `publication_id` derivation, so it cannot re-key and does not race the
daily ingest (issue 404); `re-keyed` must read 0.

1. **Before.** Read the exhibit: `curl -s https://tenders.zebreus.click/v1/tenders/4490098 | jq -c .value`
   (the stored rows are the attributes: 4.97×10¹⁸ cents for the 10¹² one, above
   `IMPLAUSIBLE_EUR_CENTS`, and 497,000,000 cents for the 10² `FMTVAL="4970000"`, so the served head
   is not 49,700 EUR) and its notice's stored parse,
   `/v1/notices/12376354/content`.
2. **Find the packages.** `fetches` is small, so this is a bounded read through `/v1/sql`:
   `SELECT id, kind, period, path FROM fetches WHERE source = 'ted' AND period >= '2011-04' AND period < '2011-09' ORDER BY id`.
   Fetch ids are INGESTION order, not publication order (the `after` doc in `supervisor.rs`), so
   check the five ids are contiguous before using one cap for all of them.
3. **Probe one package** — the one holding the exhibit (`2011-07`): with `F` its fetch id,
   `/root/aj.sh /admin/jobs '{"kind":"reparse","profiles":["ted-export-r208"],"after":F-1,"packages":1,"reclaim_only":true}'`
   (write `F-1` as the number). Then re-read `/v1/notices/12376354/content`: it serves the new parse
   before any fold — every `TED-…VALUE_COST` 4,970,000 / 5,000,000 cents, each with a
   `….VALUE_COST.FMTVAL_MISMATCH` text beside it holding the raw attribute, and no `.FMTVAL_TEXT` row.
4. **Wet.** ALL FIVE months in one job when the five ids are contiguous: `after` = the LOWEST id − 1,
   `packages` = 5. `reparse` walks every r208-holding fetch with id > `after` in fetch-id order and the cap
   truncates that list, so `packages` = 4 from the lowest id would re-do 04–07 and NEVER reach 08 (07 sits
   in the middle). Re-doing the probed 07 is harmless (a re-parse is idempotent). If the ids are not
   contiguous, one job per package (`after` = its id − 1, `packages` = 1). Five monthly packages of r208
   notices stay well under the 500,000-notice FULL-fallback line above, but `reparse` stamps every r208
   Tender stale by PROFILE, so the following fold is the cohort's, not five packages'. Read `unmatched`
   (expect ~0) and `re-keyed` (must be 0) per the table above, and check the job covered all five fetch ids.
5. **Fold.** ONE `{"kind":"project"}` after the last chunk.
6. **After.** `/v1/tenders/4490098 | jq -c .value` is the corrected text figure: the fixture projection
   of its member (`an_fmtval_scaled_by_ten_to_the_k_yields_to_its_text_and_is_elected`) elects
   `result_value` = 4,970,000 cents (49,700.00 EUR), and no amount of that notice is above €1M. A corrected
   figure is an ordinary amount, so supersession and election treat it like any other — no head is nulled
   by this change, and a Tender whose latest notice is in the cohort keeps that notice's (now correct)
   figure. `/v1/notices/12376354/content` still serves each raw attribute as a
   `….VALUE_COST.FMTVAL_MISMATCH` text beside its amount. There is no standing count of
   `.FMTVAL_MISMATCH` rows yet — the parse-layer scan is not a bounded `/v1/sql` read — so record the
   exhibit, the job counters and section 16's band before/after in issue 471.

### The organization-layer jobs (issues 300, 311-317)

These are their own family: censuses that measure, merge arms that write, and
review machinery that records verdicts. **Every writing one defaults to
`dry_run: true`** — a forgotten flag must mean the harmless thing — and the
merge arms run the T4 ladder (dry census → recorded plan → capped wet →
uncapped wet), so a wet run refuses unless its dry plan is on file.

| kind | writes? | notes |
|---|---|---|
| `org-merge-health`, `r2-census`, `r3-census` | no | the standing measurements; `org-merge-health` is weekly and carries the Stage-5 `null_country` bucket (issue 300) plus three tripwires read against its previous report: `parser_vs_stock.alarms` (issue 325), `gln_9110.shared_one_country` (327) and `name_growth.alarms` (300 tripwire 2: a top-100 org gaining ≥20 distinct names, a new entrant at ≥50, or `ge6` growing >5 %). Any alarm shows in the job's summary line as `… ALARM(S): …`; the first run after a deploy that adds a block has no baseline for it and stays quiet |
| `match-org-identifiers` (`rule: r2` / `e0` / `r3`) | YES | the merge arms; `max_groups` caps a run. Since issue 359 the R2 arm denies a group whose named members share no core token (`denied_names`, listed for review in `r2-merge-plan`); since issue 362 a reviewer's verdict in `org_merge_verdicts` overrides that (`denied_verdict`, `admitted_verdict`, `verdict_stale`). Since issue 514 unit 2 the `e0` arm's VAT-group wall skips a GB mention raw that `registry_mention_key` keys to the group's own register identity (W6: a label spelling of the number that formed the group; `wall_strip_admitted` / `wall_strip_denied` in `e0-merge-plan` count its footprint both ways), lists its name and wall denials (`denied`, `denied_wall`), and an `e0` dry run also writes `e0-merge-listing` (every planned group's key and members, uncapped up to 10,000). Every `e0` WET run is bound to that listing: it merges only a group listed with exactly its live members and counts the rest `held_unplanned`; it refuses without a listing, and a `truncated` listing falls back to parity alone (the summary says which). Since issue 549 a DRY `r2`/`e0`/`r3` plan reads through the reader pool instead of holding the writer, so a verdict POSTed while it runs can land mid-plan, and the plan then mixes state from before and after it: post verdicts BEFORE the dry run, so the plan you review reflects them. A wet run still plans under the writer |
| `match-org-identifiers` (`rule: altid`) | YES (wet; dry by default) | issue 448: the Companies House ↔ PPON pairs FTS parties publish in `additionalIdentifiers` (the fold keeps only a party's first identifier, so a supplier published PPON-first somewhere stands as a second org). Re-harvested from the FTS notices' `BT-501` rows every run and gated as E2 evidence: v2 gate, consortium veto, GB legal form head against head, the mention-evidence wall and a PPON org naming another GB key (no verdict overrides those), then a `keep`/HIGH `merge` verdict under (`GB`, `GB:altid`, `<coh>~<ppon>`), then the conflict flags, name-key corroboration and the generic wall. Corroboration reads each org's names from its mentions OUTSIDE the pair's witness notices (a side made only of witness mentions uses those), because a witness's own party name is recorded on the org its first identifier binds and would otherwise vouch for itself; a pair only the designated names join is listed `witness-only`, and names whose GB legal forms conflict across the two sides (plc against Ltd, whichever names the heads carry) list as `form-conflict` (issue 448 unit 1b). Stores `altid-merge-plan`: `both_distinct` is the issue's split count, `pairs` the sorted planned keys, `plan` / `denied` / `conflict_listing` list pairs in the merge-verdict shape and `no_target_sample` samples the pairs with no standing org. Since issue 469 unit 1 it also MEASURES (never plans) every other GB register a party publishes beside a keyed PPON: `registry_pairs` by scheme (`GB-NHS`, `GB-UKPRN`, `GB-CHC`, `GB-MPR`, …), each with `literal_pairs`, `pairs`, `no_identity`, `already_one`, `both_distinct`, `no_target_registry` / `no_target_ppon` / `no_target_both`, `multi_target`, `ppon_beside_two_values` and `ppon_beside_coh` (the charitable-company shape). A side is held by the orgs carrying its identity — for the register, the (kind, value) `project::normalise_identifier` mints, matched exactly — as their own identifier or as a merged one (`organization_merged_identifiers`: a PPON the 448 arm folded into a company-number org), plus every org a GB party's mention bound to on any FTS notice where that side came first; a register value that mints no identity (`no_identity`: a letter-only NHS code such as `QWO`, which the resolver binds by name) is held only through those mentions, so a code bound by name to several provisional orgs reads `multi_target`. A one-sided pair whose standing org's mentions carry the other side counts `already_one`. `registry_sample` lists up to 30 `both_distinct` pairs per scheme in R2's listing shape (`country`, `scheme`, `key` = `<register literal>~<PPON key>`, `members` = the register org then the PPON org, each `org_id` / `kind` / `identifier` / `name`, and up to three witness `notices`); the job's counts line ends `other registers paired with a PPON (issue 469: merged only under a HIGH merge verdict): GB-NHS n of m two orgs (…), …`. Since issue 513 unit 1 a DRY run also stores `registry_nhs_listing` (cap 2,000, `registry_nhs_listing_truncated`): every `GB-NHS` ↔ PPON pair whatever its class, in value order, each with the `literal`, its uppercase alphanumerics (`value`), the `identity` the normaliser mints (null when none: a letter-only code, or `R0B`, whose only digit is a zero), unit 1's `class`, the `ppon` key, up to five `registry_holders` and three `ppon_holders` (`org_id`, `kind`, `identifier`, `name`, `provisional`; `*_holder_count` the totals) and up to three `witness_publications` — read offline against the NHS ODS API, never into a count or the plan (a wet run's residual omits it). **Since issue 469 U4 the run also PLANS those splits, as their own graph** (on a dry run and, since U5, on a wet run's live re-plan; the company-number counts cannot move, the planner runs after them). Each scheme's `both_distinct` pairs are gated in this order — the graph gates first, so a 448 survivor holding the PPON reads `triple`, not `loser-incoherent`; all are structural (no verdict overrides them): `registry-shape` (`crosswalk::registry_pair_key` keys only GB-UKPRN / CHC / MPR / SC / NIC literals of the register's own shape, as the identity the resolver mints — so every NHS pair lands here until U7), `single-owner` (each side held exactly, as its own or a merged identifier, by one org not withheld by an issue-452 verdict — a side only name-bound mentions hold never plans; since U5 (D3) the register org must also carry the identity as its OWN `organizations.identifier` — country `GB`, kind `national` — because the register alias replays a merge only through that exact row, so a holder through a merged identifier alone is denied here rather than merged and then regrown), `triple` (any GB party published the PPON beside a company number, or the PPON's holder carries one of its own or as a merged identifier), `cross-register` (the PPON is partnered with two or more register values, any scheme, across EVERY GB party — the consortium agent that states one member's number per notice; unit 1's per-party `ppon_beside_two_values` cannot see it), then `gate` (v2 on both orgs' identifiers), `consortium`, `legal-form`, `evidence-wall` and `loser-incoherent`, the last two reading each org's mentions through the company-number AND the register channel (`crosswalk::registry_mention_key`), so two different UKPRNs are two registrations; then a `keep`/HIGH `merge` verdict under (`GB`, `GB:altid`, `<registry scheme>:<identity>~<PPON key>`, e.g. `GB-UKPRN:GBUKPRN10007798~PHDQ2359NZMP`, members ascending); then the judgment a HIGH verdict stands in for: the conflict flags `registry-multi-ppon` (the register value beside two or more PPONs) and `party-ambiguous`, the shared name corroboration over witness-free names (`form-conflict`, `witness-only`, `uncorroborated-disjoint` / `-overlap`) and the generic wall. Stored fields: `registry_pairs_planned` (the planned keys, sorted, uncapped), `registry_gates` per scheme (`candidates` = unit 1's `both_distinct` = `planned` + `denied` + `conflicts`, with every gate's count), `registry_plan` / `registry_denied` (conflicts included; caps 20,000 / 500, `*_truncated`), and the graph-level `registry_cross_register_ppons` (with `registry_cross_register_denied_ppons`) and `registry_triple_ppons`. Each listing entry is in the merge-verdict shape — `country` `GB`, `scheme` `GB:altid`, `key`, `members` ascending with `side` `registry` / `ppon` — plus `registry_scheme` (the publisher's, since `scheme` is the verdict's), `registry`, `registry_literal`, `ppon`, `ppon_literal`, `gate`, `keep` (the register org: U5's survivor), `loser`, `witnesses` (k) with up to three `witness_publications`, `publishers` (p: distinct buyer orgs of the witness notices), `self` (witness notices whose buyers include either org), `coherent` (the designated names corroborate), `ppon_partners`, `registry_partners`, `registry_names` / `ppon_names`, `corroborated_by`, `cooccurring` and (U5) `verdict_cohort`, the cohort of the HIGH merge verdict that admitted the pair (null for a pair planned on corroboration alone, which a wet run holds) — k, p, `self`, `coherent` and `cooccurring` are listed, never gated. The counts line gives each scheme's plan beside its split count — `GB-UKPRN n of m two orgs (plan a; denied b registry-shape, c triple, …; d conflicts; e verdict-admitted)`, empty clauses left out — and ends with the cross-register and triple PPON counts. **The register family's wet run (issue 469 U5).** The same wet job merges both families — there is no selector: `{"kind":"match-org-identifiers","rule":"altid","dry_run":false[,"max_groups":N]}` — re-planning the register graph live under the writer beside the company-number one. The register family is held to the stored plan's `registry_pairs_planned` by its OWN parity (max(2% of that set, 5); a stored plan with no `registry_pairs_planned` is an empty set, so every live register pair is deferred and more than five abort), checked after the company-number parity: either family over tolerance aborts before any write (`altid register parity abort: …`). A PPON org that is the loser of two live pairs of EITHER family, or one pair's loser and another's keep, is contested and merges in neither (`contradictory` / `registry_contradictory`). **A register pair merges only under an admitting HIGH, unapplied, members-exact merge verdict** (D1): register-confirmed pairs are posted as HIGH verdicts under the cohort prefix `altid-registry-confirmed-` (`store::ALTID_REGISTRY_CONFIRMED_COHORT_PREFIX`), reviewer-plus-challenger pairs under `altid-registry-<date>`; a planned pair with no verdict is held, counted `registry_unconfirmed`, never merged. `max_groups` caps each family on its own (D2), so a capped run samples the register family whatever the company-number backlog. Each register merge folds the PPON org into the register org (`repoint_org_references`, so the PPON becomes a merged identifier of the register org), stamps the verdict applied, and writes `org_merge_log` rule `e2-altid` with flat evidence and NO `coh` key: `scheme` `GB:altid`, `registry_scheme`, `registry_kind`, `registry` (the identity: the exact triple the alias replays), `registry_literal`, `ppon`, `ppon_literal`, `keep_id` / `loser_id` (the two orgs' identifiers; the issue-460 backfill reads `$.loser_id`), `name_key` (empty: a verdict stood in), `witnesses`, `publishers`, up to three `witness_notices`, `verdict` (`admitted`) and `confirmed` (`register` for a cohort under the prefix, else `verdict`). The register family's denied two-org pairs get open `e2-altid`/`E2` edges too, evidence `registry_scheme` / `registry` / `registry_literal` / `ppon` / `ppon_literal` / `registry_org` / `ppon_org` / `status` / `witnesses` / `publishers` / `registry_partners` / `ppon_partners` (an org pair a company-number edge already took this run keeps that one). The re-recorded residual's register fields are the run's own (the U4 carry-forward is gone): `registry_pairs_planned` = the reviewed register pairs still unmerged (cap, stop, contest, no verdict yet), `registry_plan` those entries, `registry_deferred_pairs` the live keys the stored set lacked (a continuation does not count them as drift again), `registry_denied` / `registry_gates` live, and `registry_live_plan_pairs`, `registry_expected_pairs`, `registry_deferred_unreviewed`, `registry_expected_not_live`, `registry_contradictory`, `registry_unconfirmed`, `registry_merged_this_run`, `registry_edges_written`, plus `registry_merged`: each merge's `key`, `keep`, `loser`, `verdict_cohort` and the exact ledger `evidence` (stop rule S4's read: `org_merge_log` is not on `/v1/sql`'s allow-list). The wet message adds `; register (issue 469 U5): merged m of n reviewed live pairs (u awaiting a HIGH verdict, c contradictory), held against e stored: …`, absent when the run held and planned no register pair. Every run also reports `registry_regrown` (S5): ledger-merged register keys whose pair is no longer `already_one` (a re-minted PPON org makes the PPON side two holders, `multi_target`), and the counts line then ends `; n merged register pair(s) split again (S5)`. U6's runbook: confirm offline → post the verdicts → dry → capped wet (`max_groups` 20) → read `registry_merged` → full wet. Refuses like r3 while a key build is in flight or the keys epoch drifted, and a wet run also while `org_match_keys` is empty. **Wet (issue 448 unit 2): dry first, always.** A wet run reads the latest `altid-merge-plan`'s `pairs` as the reviewed set (refused with no stored plan, and while an org FK index is missing, like r2), re-plans live under the writer with the same gates, and aborts before any write when the live plan and the stored set differ by more than max(2% of the stored set, 5) pairs (symmetric difference; the error names both counts). It merges only live ∩ stored pairs, in sorted key order, 50 per transaction, at most `max_groups` of them (of each family, since issue 469 U5): the PPON org folds into the company-number org (always the survivor of a company-number pair), `org_merge_log` rule `e2-altid` with flat evidence (`coh`, `ppon`, both literals, `keep_id`/`loser_id`, `name_key`, `witnesses`, up to three `witness_notices`, `verdict`), change events, and an admitting HIGH verdict stamped applied. A live pair the stored plan lacks is **deferred** (`deferred_unreviewed`/`deferred_pairs`), never merged: the next dry plan lists it for review. The two-org pairs a gate denied become open `e2-altid`/`E2` rows in `org_candidate_edges` (evidence `status` = the gate; never deleted; `state='merged'` once merged); `org-edge-census` counts them apart (`e2_altid`, `e2_altid_merged`) and neither it nor `xb-packet` joins them into components. The run re-records `altid-merge-plan` as the RESIDUAL (`residual_of_wet_run: true`, `pairs` = the reviewed pairs still unmerged), so a capped or stopped run continues under parity. The `plan` listing carries EVERY planned pair (issue 448 unit 3, the campaign's prerequisite: `ALTID_PLAN_LISTING_CAP` = 20,000, far above any realistic plan, `plan_listing_truncated` if it ever binds; the `denied` and `conflict_listing` listings keep R2's 500), each with up to three witness publication ids, so a reviewer reads every pair a wet run would merge. Every two-org listing also carries what the pair is judged by (unit 3b): `coh_names` / `ppon_names`, the names each side's corroboration read (the first 8 in key order, `coh_name_keys` / `ppon_name_keys` the totals; a head name is a first-seen election that one stray mention can set), `corroborated_by`, the name pair that cleared the wall (planned pairs only), and `cooccurring` / `cooccur_publications`, the notices other than the pair's own witnesses where both orgs are distinct parties (a wrong pair shows it, and so does a true pair beside a publisher that gave one supplier the other's identifier). `plan_cooccurring` counts the planned pairs that co-occur; it gates nothing. **The resolver alias (issue 448 unit 3).** Every `project` fold (incremental, full and rebuild) arms its mention resolver from the `e2-altid` ledger: each merged PPON key maps to the company-number KEY it was merged with (identity to identity, never org ids, so it survives a rebuild's renumbering); a pair under a `keep` verdict (`GB`, `GB:altid`, `<coh>~<ppon>`) is dropped, and a PPON the ledger merged into two company numbers is poisoned and never aliased. A mention whose raw identifier keys to that PPON, and which misses both the exact triple and the canonical key (its org was folded away), binds to the org that now owns the company number, at the arm's own bar and never looser: neither key poisoned, a standing owner, no consortium name on either side, the same GB legal form head against head, the planner's shared name predicate (`altid_corroborates`) between the mention's names and the owner's head and satellites — and, since issue 518, the owner's mention names (its newest 4,096 mentions, at most 256 distinct names, read once per owner per fold), as the planner reads names; the consortium and legal-form vetoes stay on the head and satellites — and the generic wall through the resolver's memo (lenient when the wall is disabled for the run, counted as bound with the wall unable to answer). A bind is never cached, so each repeat re-earns the bar with its own name. A refused mention mints or binds exactly as it would without the alias; as before 448, the PPON org it mints then takes the later mentions of that PPON, and the next dry plan lists the pair again. In a rebuild, a PPON-first notice older than the supplier's first company-number-first notice finds no owner yet and mints (`no owner`). The alias logs `[issue 448]` to the diag log on every fold (armed with how many aliases, then asked / bound / refused by cause, zeros included) and adds `; issue-448 alias asked … bound … refused …` to the `project` job's counts line when it was asked anything. Since issue 460 a bind also records the PPON as a merged identifier (`organization_merged_identifiers`, rule `altid-alias`) of the org it bound to when that org carries none, once per (PPON, org) a fold, each an `organization changed` (the counts line then says `N PPON(s) recorded as merged identifiers`): after a from-archive rebuild, which empties that table, the alias binds a merged PPON straight to the company-number org, no PPON org is minted and no merge ever writes it back, so this is where `?identifier=<PPON>` comes back. **The register alias (issue 469 U5).** The same arming reads the ledger's register rows (`registry_scheme`, `registry_kind`, `registry`; a row with a `coh` reads exactly as before) and maps each merged PPON to its (kind, identity); a `keep` verdict under the register key `<registry scheme>:<identity>~<ppon>` drops it, and a PPON the ledger aimed at two targets of any kind is poisoned. A PPON-first mention that misses the exact and the canonical lookups binds through the exact (`GB`, kind, identity) row — a register literal keys nothing, so there is no canonical map to find it by — only when exactly one row carries the triple at the fold's open and no `wrong` / unapplied `related` identifier verdict names it (else refused `shared`; no row: `no owner`), then at 448's bar unchanged: one code path for both target kinds, with issue 518's owner names. Its asks are counted apart (`AltIdAliasCounts::registry`; the armed diag line says how many PPONs alias to a register identity and how many register targets bind nobody, the finish line gives the register asks by cause, the counts line adds `; issue-469 register alias …`), and a bind records the PPON as a merged identifier exactly as a company-number bind does (issue 460). A pre-U5 binary skips register ledger rows (they carry no `coh`), so rolling back after a register wet run leaves those PPONs unaliased: they regrow unless the merges are unwound (ADR 0003). **No wet run until unit 3 is deployed AND the full `plan` listing of the dry plan the wet run will hold to has been reviewed** (issue 448 unit 4's campaign). The deploy alone lifts nothing: without the alias the next FTS fold re-mints every PPON-first supplier a merge folded, and without the review nobody has read what the run merges |
| `match-org-identifiers` (`rule: nhs-body`) | YES (wet, since issue 513 U4; dry by default) | issue 513 U2: the census a reviewed NHS body verdict is read from (decision "unit 2, second": a letter-only ODS code such as `GB-NHS-RTH` mints no identity, so each body stands as its name-bound spelling clusters plus its PPON org, and U4 will fold them under a reviewed, members-exact verdict). Re-harvests every FTS party (the altid walk): a GB party whose mention's raw identifier is `GBNHS` + three characters (`nhs_ods::nhs_tail`) is a coded party, and the org its mention bound is a holder of that tail; so is the org of a GB party publishing the tail behind a first identifier that is no keyed PPON (`GB-COH-…, GB-NHS-DQF`: a later-coded party, `later_coded_parties`); each tail maps to its allowlisted ODS code (`nhs_ods::survivor_code`, a unique O↔0 spelling recorded `folded`) or is listed off the allowlist. A code's holders are EVERY such org plus the holders of each PPON a GB party published beside the code (the triple as an own or merged identifier, less issue-452-withheld orgs, and the orgs PPON-first parties bound), uncapped (`holder_ids`). Stores `nhs-body-plan` (`dry_run`, `generic_wall_readable`, `ods_version`): `codes[]`, each with its `ods` record (name, role, successors, `legal_end`), `tails`, `coded_parties`, `coded_notices_after_legal_end`, `ppons`, `holder_ids`, `holders[]`, `eligible` and `verdict`; per holder `via` (`party`, `party-later`, `ppon`, joined by `+`), `coded_parties`, `later_parties`, `ppons`, the row (`provisional`, `country`, `identifier_kind`, `identifier`, `name`, `name_norm`), `merged_identifiers`, `mentions` (exact) with `mentions_read`/`mentions_capped` (the newest 4,096 through `organization_mentions_org`), `raw_identifiers` and `name_norms` (64, `names_capped`) over them, `other_codes`, `check` (`nhs_ods::check` of the head name: `admitted`, `tie`, or the refusal, with its `rival`), `key_owners_elsewhere` (standing identifier-NULL rows on its 234 key outside the body; D4), the flags `under_review` (any review table but an nhs-body verdict), `generic`, `consortium` (the head name), `consortium_names` (consortium-shaped mention spellings, 8: listed, never denied on), `successor_name`, `minted_identity`, `holder_of`, `contested`, and `denied`: the structural gates no verdict overrides (D6) — `gone` (the org row went since the harvest), `minted-identity` (an own identifier that is no PPON), `generic`, `consortium` (the head name only), `successor-name` (the head name is a successor's ODS name under a legal end), `contested` (two codes' merge verdicts name the org). Beside them: `off_allowlist[]` (cap 2,000: tail, `value`, `parties`, `holders`, `ppons`, `top_names`, the table's row for the tail when it has one), `countryless_parties` / `non_gb_parties` (a tail at any ordinal) / `other_nhs_values` (a GB-NHS row with no three-character tail) with samples, `coded_share_by_year` (the listed holders' mentions read and how many are coded, per notice year), the `denied` counts, `eligible_holders`, `multi_code_orgs`, `contested_orgs`, `key_owned_elsewhere`, and every cut's `*_truncated` flag (codes 2,000, holder entries 10,000). **Verdicts** are posted under (`GB`, `GB:nhs-body`, `<code>`) through `POST /admin/merge-verdicts`, members ascending (ONE member allowed: a body already one org records its code) and, for a `merge`, `reviewed_holders` — every holder the reviewer read. The census reports each code's `verdict.status`: `admitted` only for a HIGH, unapplied merge whose `reviewed_holders` IS the live holder set and whose members are live, standing, undenied, uncontested and (but for the survivor) own their 234 keys; otherwise `stale` (with `newcomers` / `departed`: a newcomer holder voids the review), `members-not-live`, `member-gone`, `member-denied`, `member-key-owned-elsewhere` (D4: a standing identifier-NULL row outside the body owns a LOSER's 234 key; since U6 the survivor the fold would keep is exempt, as the alias arms loser keys only), `contested`, `not-high`, `no-reviewed-holders`, `holders-unlisted` or `applied`; `keep` holds the body. Counted `admitted_verdict`, `verdict_stale`, `verdict_keep`. Refuses like altid while a key build is in flight or the keys epoch drifted; an empty `org_match_keys` runs dry and the counts line says the generic wall was blind. **Since U4** the census also lists `admitted_codes` (the codes whose verdict it admits, ascending), every `nhs-body` ledger row verbatim (`nhs_body_merged`, evidence parsed back to JSON — `org_merge_log` is not on /v1/sql; `ledger_rows`, cut at 5,000) and S4's `nhs_body_regrown` (a loser's 234 key standing on an identifier-NULL row other than its live survivor, `kind: name`, or a loser literal carried by a row again, `kind: ppon`; no `name`, `ppon` or `identity` entry may stand on any dry run — since issue 519 U4 the job line's `keys regrown (S4)` counts those halting kinds and `identity-stray listed (no halt)` the rest; `survivor` is resolved as the unwind resolves it — the evidence's identity and the keep's chain must agree, else `null`). **Wet** (`dry_run: false`, `max_groups` caps the bodies) REQUIRES the stored plan's `admitted_codes` (a plan from before U4 has none and refuses: re-run the dry plan), the org FK indexes, and a non-empty `org_match_keys`. It re-plans under the writer and holds the live `admitted_codes` to the stored set within max(2%, 5) bodies (S2) or aborts before any write; a live-admitted code the stored plan lacks is `deferred_codes`, never folded. Per admitted body (code order): the survivor is its SAME PPON org (the member whose own identifier is a `GB:ppon`; of several, the one holding a PPON published beside the code, else the body is `held` as `survivor-ambiguous`), or with no PPON member its largest cluster by mentions as a provisional anchor. A body whose LOSERS include a PPON org is `held` as `loser-ppon-unaliased` (no alias re-binds a loser's PPON yet, so its next PPON-first mention would mint the org again); since issue 519 U4 a body whose identity child would fold into a provisional anchor is `held` as `child-into-anchor` (the census status's backstop); held bodies stay in the residual. One transaction per body: each loser's references repointed onto the survivor (rule `nhs-body`; a loser PPON becomes a merged identifier), the row deleted, an `org_merge_log` row rule `nhs-body` with identity-keyed evidence (`code`, `ods_name`, `target` `ppon` with `target_identity`/`target_identifier_kind`/`target_country`/`target_ppon` or `anchor` with `anchor_name_norm`/`anchor_country`, `keep_name`, `keep_id`/`loser_id` literals, `loser_name`, `loser_name_norm`, `loser_names`, `loser_name_norms`, `loser_country`, `loser_ppon`, `cohort`, `job_id`), `organization removed`/`changed` and the tenders' `changed`; then `GBNHS<code>` written to `organization_merged_identifiers` on the survivor (so `?identifier=GBNHS<code>` answers it) and the verdict stamped applied. A one-member body only records its code. Every row since issue 519 U4 also says `code_recorded` (whether that cohort's fold wrote `GBNHS<code>`); a CHILD member (issue 519 U4) folds the same way — an identity child as an IDENTITY loser (`loser_kind: identity`, `loser_identifier_kind`, `child_code`/`child_codes`/`child_rel`/`child_periods`, and EMPTY `loser_name_norm`/`loser_name_norms`, so no name path reads the row), its own `GBNHS<C>` repointed onto the survivor as a merged identifier (`?identifier=GBNHS<C>` answers it); a zero-only cluster as a name loser with the child fields, its `GBNHS<00x>` recorded on the survivor (lookup only; a rebuild loses it). The stored plan is then the RESIDUAL (`admitted_codes` = the reviewed bodies still unfolded, `residual_of_wet_run: true`, `live_admitted_codes` beside it, `folded[]`, `held[]`, the moved counts); its `codes[]` is the live census from BEFORE the folds, so each entry carries `folded` (true for a body this run folded — its `holders` and `verdict.status` are pre-fold; a fresh dry run reads the post-fold state). A body whose transaction FAILS after an earlier body committed stops the run there: the residual (that body and every later one) is stored, `fold_error` names it, and the job fails naming the committed codes — a re-run continues under parity; a failure before the first commit wrote nothing and stores nothing. Every fold arms the **nhs-body alias** from those rows (with the altid alias): a GB identifier-less mention whose 234 key is a loser's own head key binds the survivor before issue 234's probe — the PPON target through its exact triple, its canonical key or an altid ledger move of its PPON, minted from the ledger (`keep_name`) when absent (a rebuild); an anchor through its own 234 key — and records `GBNHS<code>` on it once per run. A key two bodies claim is poisoned, a key a standing identifier-NULL row owns is not armed (D4), a shared or withheld PPON triple binds nobody; never by code. Since issue 519 U4 the identity-loser rows arm its **exact-triple arm**, asked at the head of the IDENTIFIED branch before `org_of` (else the first refused mint would claim the child triple for every later mention): a mention whose identifier is exactly a folded child's (GB, national, `GBNHS<C>`) binds the survivor of the ONE code whose `child_check` (the strict gate the census held the fold to, injected as `AltIdAliasRules::nhs_child_check`) admits its name — standing, or minted from the ledger in a rebuild — recording `GBNHS<code>` and `GBNHS<C>` on it once per run, never cached; no admitting arm, two, a withheld child triple or a shared target refuse, and the mention resolves as unarmed. S4 lists a child literal standing on another row as `identity` (its head or a mention name passes `child_check`: halt) or `identity-stray` (listed). **Rollback fence (S6):** after the first wet run, never deploy a binary older than U4 without unwinding first — it ignores the ledger and every merged spelling regrows |
| `match-org-identifiers` (`rule: nhs-body-unwind`, `code`, optional `cohort`) | YES (wet; dry by default) | issue 513 U4 (D7): the scoped undo of ONE body's `nhs-body` folds, named by its ODS code (`{"kind":"match-org-identifiers","rule":"nhs-body-unwind","code":"RTH"}`; `code` is required, three characters, and refused on any other rule). Issue 519 U4: `"cohort":"<verdict cohort>"` (refused on any other rule, and when empty) unwinds only that cohort's rows — the child cohort's identity losers back through E0 to a row carrying their own `GBNHS<C>`, its zero-only clusters back through 234 — keeping every other cohort's fold; it REFUSES, nothing written, while a later cohort's un-retired rows stand on the code or the survivor (last in, first out: unwind the later cohort first); it keeps `GBNHS<code>` while another cohort of the code stands or when the cohort did not record it (`code_kept`); and it drops the child literals it returns off the survivor whoever recorded them (the fold, or the triple arm's bind). A scoped unwind also REFUSES, nothing written, when the cohort has no standing row on the code — `it was unwound already` (a retry of a finished scoped unwind) or `it never folded <code>` (a typo), or `recorded but not folded yet` (its verdict on the code is unapplied: a capped run did not reach it) — unless the cohort is the code's own applied ONE-MEMBER fold (`recorded GBNHS<code>`) with no other cohort claiming the code, which alone takes the one-member path below (so a 513 one-member body's `GBNHS<code>` survives a retried or mistyped child unwind); and when an earlier cohort's standing row of the code folded the same loser literal or 234 key (two holders of one child triple folded in two cohorts): the moves pick mentions by literal and key, not by cohort, so only the code-wide unwind can dissolve both. An unwind named by a CHILD code that a standing row folded REFUSES, scoped or not, nothing written, naming the parent body and cohort (a child row carries its parent's `code`, so the child code has no row of its own and would otherwise take the one-member path, dropping the child's `GBNHS<C>` under the parent's standing row): unwind the parent's cohort. Without `cohort`, every cohort of the code, as below. Reads the body's ledger rows and their survivor, twice, and refuses with nothing written unless both readings name ONE live org: by the evidence's identity (a PPON target's exact triple, else its canonical key, else an altid move of its PPON; an anchor's 234 key) and by the keep's chain under the org era's floor (the merged-identifier backfill's time checks — after a from-archive rebuild the keep ids name other companies, so the unwind refuses; unwind any later fold of the survivor first). It then dissolves the survivor's former members back through the resolver's own arms: a mention with no identity whose 234 key is one of the losers' recorded `loser_name_norms` goes to the lowest-id identifier-NULL row on that key or a minted provisional; a mention whose identity is a loser's own literal (its PPON) goes to the row carrying that triple or a minted one. The survivor's own mentions stay. Wet, in one transaction: the mentions and their party / bid-party rows rebound, the moved notices' tenders stamped stale and the notices re-queued (the next fold re-derives their winner rows), the ledger rows moved to rule `nhs-body-retired` (the alias stops arming them), one `nhs-body-unwind` row per old loser to the org it dissolved into — a loser with its own literal (a PPON org) to the row carrying that literal, else the row its head key went to — so the old id follows the unwound state, `GBNHS<code>` and the losers' literals dropped off the survivor. A body with NO ledger row (a one-member body: its fold only recorded the code) unwinds to dropping `GBNHS<code>` (rule `nhs-body`) wherever it stands (`merged_identifiers_dropped`); a rebuild loses that record for good, an accepted gap. The verdict stays applied: folding the body again needs a new verdict. Stores `nhs-body-unwind` (`moved`, `by_name`, `by_identity`, `targets[]`, `minted`, `ledger_retired`, `redirects`, …) |
| `match-org-identifiers` (`rule: nhs-body-canary`, `notices`) | no (read-only; `dry_run: false` is refused) | issue 513 U4, stop rule S5's prod half: re-resolve the named notices' recorded mentions (`{"kind":"match-org-identifiers","rule":"nhs-body-canary","notices":[…]}`; at most 100 ids, `notices` refused on any other rule) through a FRESH resolver opened and armed exactly as the fold's (the altid and nhs-body aliases), inside one `BEGIN IMMEDIATE` on the writer that is ALWAYS rolled back, so nothing it would write lands. Each mention is rebuilt from its row (name, country, raw identifier, scheme, and the identity `project::gated_identifier` gives them) and taken down the resolver's new-mention path, never the idempotency early return. Stores `nhs-body-canary`: per mention `stored` (its `organization_id`), `resolved`, `minted`, `agree`; `agree`/`disagree`/`minted` counts, `notices_without_mentions`, and the alias's own counts (`alias.asked`, `bound`, …; since issue 519 U4 the exact-triple arm's too, `alias.triple_asked`, `triple_bound`, … — S5 for children needs `triple_bound` > 0, and the job line names them). The job line says `AGREES` only for 100% agreement. Opening the resolver preloads every identified org, as a fold does, and writes an `[issue 513] … ARMED` diag line (no closing line: the resolver is never finished) |
| `match-org-identifiers` (`rule: rekey`) | YES (wet; dry by default) | issue 453: acts on the issue-452 `wrong` identifier verdicts that name the right number (`correct_identifier`), and since issue 466 on `related` ones that name the org's own number (a parent's, subsidiary's or sister's number taken off for the org's own), `high` confidence only, for the org that carries the wrong triple now (none: `gone`; several: `several`, E0's family). GB company numbers only (`unkeyed` otherwise); a right number that keys where the wrong one does is `same_key`. The right number's owners are found through the canonical key over every identifier-bearing org, so any spelling counts. **merge**: exactly one owner, not itself under a `wrong` or unapplied `related` verdict (`withheld_target`), no consortium name on either side, the same GB legal form head against head, and one agreeing name pair under the altid name key (`denied_names`; issue 454: a HIGH `merge` verdict in `org_merge_verdicts` under (`GB`, `GB:rekey`, `<wrong literal>~<right number>`) whose members are exactly the two org ids admits past the name gate only, counted `admitted_verdict` and stamped applied by the merge; a `keep` verdict there denies, `verdict-keep`): the wrong-number org folds into the owner, `org_merge_log` rule `rekey` (evidence: `country`, `kind`, `wrong`, `right`, `keep_id`), change events, touched tenders changed. Several owners is `multi_target` (R2's family to fold first). The right owner must not carry a `wrong`/`related` verdict itself, nor may a move's destination triple (`destination_verdict`). **move**: no owner: the org's identifier becomes the right number's canonical body in the wrong literal's spelling (`GBCOH…` kept, bare stays bare; a reviewer's `GB-COH-…` never doubles the prefix) and one `organization changed` event goes out. Two owner-less candidates naming the same right number: only the one with the smallest key moves, the others list `pending-move` (after it moves they are merges into it, planned by the next dry run). Both stamp the verdict (`applied_at`, `applied_literal` = what the entity carries since, `job_id`). Stores `rekey-plan` (`keys` sorted, `plan` / `denied` listings with each target). A planned key pins what was reviewed: `<country>/<kind>/<wrong>>merge:<target literal>` or `…>move:<new literal>`, so a re-POSTed right number or a changed target is a different key and is deferred, never executed unreviewed. A wet run's re-recorded plan counts `plan_merge`/`plan_move` over the residual it records (`live_plan_merge`/`live_plan_move` are the live re-plan's). **Wet: dry first, always.** A wet run reads `rekey-plan`'s `keys` (refused without one, and while an org FK index is missing), re-plans under the writer, aborts before any write when the live and stored keys differ by more than max(2%, 5), executes only live ∩ stored in key order (50 per transaction, at most `max_groups`), defers a live key the plan lacks, and re-records the residual. **The alias** (`related` verdicts too, issue 466: on the 90 orgs the arm can re-key, 126 of 128 mentions of the related number name the org itself, so its repeats are the publisher's, and other spellings reach the related company only by name): every fold's resolver maps the stamped wrong triple to the org carrying `applied_literal`, following a chain of re-keys (at most 8 hops) and resolving at bind time, so an entity merged away since binds to its survivor, (the publisher's repeat reaches the entity, as it reached the wrong-number org before), and guards the wrong number's canonical key with that org among its owners, so another spelling of the wrong number reaches it only under a matching name (the number may be someone else's real one). Logged in the resolver's `[issue 452]` line as `re-keyed wrong number(s) aliased`. **Merged identifiers** (issue 460): a `rekey` merge never writes the loser's wrong number for the survivor, and both arms drop every merged identifier that spells it (canonical key or literal) from the entity — an earlier `r2` may have folded the bare number or another spelling into the re-keyed org, which the merge's carry would move onto the right company and the move would leave in place; identifiers that are not a spelling of it (a PPON) travel as in any merge. Counted `merged_identifiers_dropped_this_run` in the wet run's `rekey-plan` and its summary |
| `build-org-match-keys` | satellite only | wholesale rebuild, ~90 s; **weekly since issue 315** |
| `scan-org-match-keys` | edges only | the E3 candidate scan and tripwire 6's clock; weekly, dry then wet (issue 360) |
| `org-edge-census` | no | sizes the edge store into review cohorts (issue 314) |
| `xb-packet` | no | issues 311+314+355: the same-name cross-border review packet |
| `apply-case-reviews` / `unapply-case-reviews` | YES | the issue-311 verdict applier and its undo |
| `case-review-backlog` | no | the parked verdicts nobody consumes (issue 317) |
| `fusion-census` | no | which reviewed rows hold mentions naming somebody else (317 Unit A) |
| `rehoming-packet` | no | the reviewer's input: those mentions, addressed, with destinations |
| `apply-rehoming` | YES | moves a reviewed mention to the row it names; **refold after** |
| `satellite-orphans` | no | name variants a re-homing left behind (issue 321) |
| `drop-orphan-satellites` | **dry default** | drops the orphans a destination already carries by N2 KEY (321) |
| `restore-dropped-satellites` | **dry default** | puts them back from `org_name_drops` pre-images |
| `anchor-wall-census` | no | issue 318: where ingest binds and batch refuses |
| `fold-org-countries` | YES | backfills non-canonical country codes (issue 319) |
| `country-typo-census` / `repair-country-typos` | no / **dry default** | issue 326: one-letter-off country codes beside a validating sibling |
| `country-cluster-census` / `country-cluster-packet` | no | issue 357: one identifier under several codes, and the reviewer's packet (`max_groups`, default 600; clusters that already carry a country verdict are left out) |
| `apply-country-verdicts` | **dry default** | issue 355: executes the HIGH `move` rows of `org_country_verdicts` (dry stores `country-verdict-plan`; wet re-checks the reviewed tuples) |
| `duplicate-identity-census` | no | issue 328 follow-on: exact `(country, kind, identifier)` triples held by several rows, keyed and unkeyed |
| `provisional-echo-census` / `fold-provisional-echoes` | no / **dry default** | issue 351: NULL-country provisional rows sharing one name, and their fold (wall- and verdict-gated) |
| `repair-provisional-name-norm` | **dry default** | issue 432: re-derives `name_norm` for every identifier-less row under `store::org_name_norm` (lowercase, whitespace trimmed and collapsed, trailing `.`/`,`/`;` stripped — nothing more) and folds the same-country twins the corrected key reveals (`'procurement for housing '` beside `'procurement for housing'`) through the echo fold's loop, ledger rule `p1`, keep = lowest id. No wall: issue 234's reuse merges identical `(name_norm, country)` rows without one. Country-less rows are re-keyed but never folded here — run `fold-provisional-echoes` after, which folds them behind the wall. Also re-keys `org_name_verdicts` (a key already taken is left standing and listed under `verdict_conflicts`). Dry stores `provisional-name-norm-plan`; the wet arm reads its `groups` and `rows` and aborts outside max(2%, 50). `max_groups` caps the folds and defers the bulk re-key to an uncapped run |
| `generic-wall-census` / `generic-statistic-census` / `name-attribution-probe` / `name-pollution-census` | no | issues 331–334, 349–350: the genericness wall's carriers and the names behind them. `name-pollution-census` (issue 330) also measures the ADDRESS-SHAPED subset — names ending in a postal block `ingest::address::strip_trailing_address` recognises — under `address`: how many would gain a same-triple twin's agreement if the key builder stripped the block, and how many stripped keys another identifier already holds. The strip feeds no key yet; the census is the decision's input |
| `name-phrase-census` | no | issue 527 unit 1: which organizations a phrase reaches. Every served name is folded and matched as whole words (`{"kind":"name-phrase-census","phrases":["niet gegunde partij"]}`; 1–20 phrases, each three letters or more once folded). With `"mentions":true` every mention's published name is matched too (about six times the organization walk), so an organization whose served name is clean is reached through its mentions (`source: mention`, with the `mention_name` that reached it). For each organization it reports the mentions, the distinct tenders whose served version names it a winner (the buyer as its own winner aside, as `?winner=` serves) and the distinct tenders per served role (`bid:tenderer` from the bid parties, as `?bidder=` serves), with report-wide tallies (`by_phrase`, every asked phrase listed; `by_phrase_set`, the exact set each organization holds, which tells `gegunde partij` alone from `niet gegunde partij + gegunde partij`; `by_country`; `winner_orgs` and distinct `winner_tenders`; `roles`; `mention_orphans`, mentions whose organization row is gone). Folded twins among the phrases count once. The listing shows the first 500 by id; past 200,000 organizations `ceiling_hit` marks the tallies as a lower bound. Read-only; the report kind is `name-phrase-census` |
| `guid-census` | no (read-only; `dry_run: false` is refused) | issue 530 unit 1: sizes the v4-GUID identifier class (`idgate::uuid_v4`: 32 hex digits, dashed or not, any case, version nibble 4 and an RFC-4122 variant nibble; `nibble_only` counts the 32-hex values with the version nibble and another variant, which the issue's Verify regex counts and the class does not) by the distinct TENDERS each row's mentions span, never notices — a notice's TED and DÖE copies are one tender (`{"kind":"guid-census"}`; optional `stable_mentions`, default 100, 2 or under refused; optional `samples`, default 30, clamped to 1–200). Reports, for every GUID row: (1) `one_tender` (single-use), `multi_tender`, `no_tender` and `no_mentions`; (2) `twin`: notices where mentions of two or more DIFFERENT GUID rows share an `org_name_norm` of their published names (the Göttingen award: one buyer under two GUIDs in one notice), with the rows, single-use rows and stable rows reached under both readings of unit 2 (a): pair-scoped (`rows`, `one_tender_rows`, `stable_rows`: only the rows of the name the notice holds twice) and notice-scoped (`notice_rows`, `notice_one_tender_rows`, `notice_stable_rows`: every GUID row the twin notice mentions, named or not — the Göttingen winner Rommel too, whose name the notice holds once; `notice_stable_rows` is that rule's cost, stable keys it would refuse on those notices); (3) `cross`: rows whose `(org_name_norm(name), country)` already holds an EARLIER (lower id) GUID row with 2 or fewer mentions (Südbayern's per-notice rows have no twin but meet this), with the names, rows and single-use rows reached; every tally carries `one_tender_twin` (pair-scoped), `one_tender_twin_notice` (notice-scoped), `one_tender_cross`, `one_tender_either` and `one_tender_either_notice` (either signal with the twin pair- or notice-scoped; `one_tender` less one is what both rules miss under that reading); (4) `cross.names_with_stable` / `rows_in_stable_names` / `stable_rows`: the cross signal on a name that also holds a stable GUID row, and the stable rows it reaches (a key a rule on it would have refused at its mint); (5) `multi_notice` (issue 312's basis) beside `multi_tender` for `all`, `stable` (ids 254, 255, 8378 and every GUID row with `stable_mentions`+ mentions) and `not_stable`. Broken down `by_country` (`organizations.country`), `by_role` (the row's `tender_version_parties` roles: buyer (`Procedure-Buyer`, legacy buyer and purchasing body) / tenderer_winner (`Tenderer`, its main and subcontractors, the legacy winner, plus any bid-party or lot-result-winner row) / review_or_info (every `Lot-`/`Part-` role and the legacy review, mediation, information, tender-receipt, specifications and legislation bodies) / other (anything else: the eSender, signatory, payer, financer), several taking the first in that order, `role_mixed` counting them; `no_role` without any), `by_platform` (the row's notices' `notices.source` values joined by `+`, then the published BT-501 formatting of its mentions' `raw_identifier`: `dashed-lower`, `plain-upper`, …, `other`, `none` — the stored identifier is normalised and cannot tell them apart; the largest 60, the rest under `(rest)`), `by_profile` (likewise) and `by_first_year` (the earliest notice's UTC year). The eSender party and the submission URL are not read (`not_measured`). Samples (the first `samples` by id): `twin_groups` (by notice: the notice id, each member's org id, name and GUID, and the members' first three tender ids merged), then `one_tender`, `cross` (with `cross_with`, the earlier row), `cross_stable_names`, `multi_tender` (outside the stable population), `stable` (the largest by mentions) and `no_tender`, each of these with its org id, name, GUID, mentions, notices, tenders, first three notice and tender ids, role, platform, profile and its `twin` / `twin_notice` / `cross_with` marks; `canaries` reports 254, 255, 8378, 30530641, 22759261, 22759262, 22759265, 30971684, 30971734, 30971757, 30532145 and 30532147 (`canaries_absent` the ones not found as GUID rows). Walks `organizations` by rowid windows of 10,000 ids (the identifier's length a filter on the window, never a whole-table statement) up to the highest id at its start, and reads the GUID rows in batches of 512: mentions through `organization_mentions_org`, their notices by primary key, the Tenders through `tender_versions_notice`, roles through `tender_version_parties_org_role`, bid parties and winners through their `org` indexes (refused while any of the five is absent). Memory is one record per GUID row (about 450 bytes, about 100 MB at 175k rows; not capped, since a ceiling in id order would drop the newest per-notice mints first) and one 24-byte tuple per GUID mention (about 192 MB at the 8M cap, about 290 MB allocated for a moment while the buffer last grows; past it `mention_ceiling_hit` marks the twin counts a lower bound). Stoppable between windows, between batches and between the IN chunks inside a batch (the first batch always carries 254 and 255, about 97k mentions); a stopped run stores no report. Read-only; the report kind is `guid-census` |
| `dissolve-guid-twins` | YES (wet since issue 530 Build 2, `dry_run: false`; dry by default; the recurrence arm is read-only; the restore arm writes) | issue 530 unit 3, Build 1 (+ Build 1b, R2's fixes F1/F2; Build 2 below: the compiled KEEP, the wet run, the recurrence arm): plans rule 530-T's stock without writing (`{"kind":"dissolve-guid-twins"}`; optional `stable_mentions`, default 100, 2 or under refused; optional `samples`, default 30, clamped to 1–200; optional `extra_unsafe_names`, 1–50 distinct non-blank names without `|`, extra unsafe twin keys for an R1 re-run after an R2 finding — every other kind refuses that field). Rule 530-T (`ingest::project::guid_twin::guid_twin_decide`, a pure function of ONE notice copy — a TED and a DÖE copy are each decided alone): the ORG sections whose gated BT-501 (`gated_identifier` under the canonical country, the gate's own path) is a `uuid_v4` national key, grouped by (identifier country = `register_jurisdiction`, `org_name_norm(name)`); a group with 2 or more DISTINCT values refuses every section whose value is not in KEEP, unless the name is not safe (fewer than 2 content words once the legal forms gmbh, mbh, ggmbh, ag, kg, kgaa, se, ug, ev, aor, co, ohg, gbr, sa, sarl, sas are dropped; `generic_body_name`; `GUID_TWIN_UNSAFE_NAMES` — Vergabekammer, Vergabestelle, Zentrale Vergabestelle, Beschaffungsstelle, Tiefbauamt, Hochbauamt, Bauamt, Bauverwaltung, Gemeindeverwaltung, Stadtverwaltung, Finanzbehörde, Tribunal administratif, the polity umbrellas `<Freistaat|Land|Bundesland|Kanton|Canton> <one word>` — read on the name's whitespace tokens after a leading article, so `Land Nordrhein-Westfalen`, `Kanton Basel-Stadt`, `Canton de Bâle-Ville` and `Der Freistaat Bayern` are umbrellas, and the hyphenated Länder and two-word cantons are also listed so a space-written spelling binds — Bundesrepublik Deutschland, Freie und Hansestadt Hamburg, Freie Hansestadt Bremen, Republik Österreich, the person placeholders, and — Build 1b, R2 2026-10-19 — the umbrella chamber names Vergabekammer NRW and Geschäftsstelle der Vergabekammer (R2's two convicted keys, targets 3353580 and 6622773) plus, precautionary, Vergabekammer Nordrhein-Westfalen, Vergabekammer Bayern and Geschäftsstelle der Vergabekammern: exact whole names, never a `Vergabekammer <Land>` pattern, so single-chamber names such as Vergabekammer Rheinland, Westfalen or Niedersachsen still refuse; or `extra_unsafe_names`), the group has no country, or — Build 1b, A7's role guard (`role_mix`) — the group's roles by the notice copy's OWN OPT-300/OPT-301 references (never `tender_version_parties`) are role-incoherent: Lot-ReviewOrg or Part-ReviewOrg beside a buyer-side role (Procedure-Buyer, Procedure-SProvider, Lot-/Part-AddInfo, Lot-/Part-TenderReceipt, each read after its `Procedure-`/`Lot-`/`Part-` scope), or a Tenderer/Tenderer-MainCont/Tenderer-SubCont beside any other role. The role guard binds the whole group, so a buyer that printed itself into the review slot binds too: those right folds are traded for never giving a review body's role to the buyer's row (R2's W3, the AOK Niedersachsen copies 24253392/26483158). The roles come from one fold and one mapping (`store::guid_twin_fold_roles`, `ingest::project::guid_twin::guid_twin_role_ref` — `role_name` over OPT-300/OPT-301, DE-1.x aliases resolved and counted as references whatever their stored flag); the gate's `guid_twin_section_roles` reads the parsed copy with them, the planner the copy's stored `notice_ids` rows. The gate is NOT wired: production ingest is unchanged in Build 1 and 1b. KEEP is computed in memory from the walk (Build 2 commits it as `guid-keep-530.tsv`): every uuid_v4 value whose GUID rows span 2 or more distinct Tenders together (since 530's review round 3 counted per VALUE over all of its rows, as the several-Tenders tripwire, the compiled self-check and A1's recurrence arm count it — a value on two single-Tender rows in two Tenders, DE + AT or issue 62's duplicate triple, links two Tenders; `multi_row_values` lists every value on 2+ rows), the stable rows (254, 255, 8378 and every uuid_v4 row with `stable_mentions`+ mentions; only the default 100 is the decision's, and a run at any other value never passes R1) and every uuid_v4 value in `organization_merged_identifiers`, value-keyed (undashed upper case, country ignored); listed whole as `keep_candidates` (country, value, org id, mentions, tenders, reason). Plans per MENTION: a GUID row whose every mention is refused is planned whole, a row with a refused and a kept mention is partial (its refused mentions are rehomes: the TED/DÖE copies that disagree, `partial.by_sources`); each refused mention's target is the lowest-id identifier-NULL row of (`org_name_norm`, `register_jurisdiction(country)`) — the issue-234 path, never an identified row (300 §10) — and a key without one is a missing target (`missing_targets`). Refused mentions bound to a row that is not a GUID row are reported by bound kind (`off_guid`: identifier_null, merge_survivor, other_identified, guid_above_max_id — a v4 row minted above the walk's max id — absent; and how many already sit on their target), not planned; read in candidate notices only, so `off_guid` and the rule's tallies are LOWER BOUNDS (the stored report's `scope_basis` says so). Reports: guard exclusions by reason (`guard`: no_country, content_words, generic, unsafe_list, extra, role_mix, keep; groups and mentions, with samples, each sample with its notice-reference `ref_roles` as the planner read them; `roles.role_mix_by_clause` splits role_mix into buyer_review_org and tenderer_mixed), KEEP partners and what KEEP spares against an empty KEEP; planned rows by country, role and first year; the planned and partial row ids and the first 2,000 planned mentions (`planned_mention_keys`); the largest target; the city screen (`city`: each planned mention's BT-513 city through the `notice_texts` primary key against its target's cities over its newest 50 mentions — flag a, a city absent from a non-empty set; flag b, a target with 2 or more cities; flag c, an A6 extension, a target key found or missing whose planned mentions alone show 2 or more cities; every flagged key listed, found or missing); the PARTY-role composition of the refused groups (`roles`: by role set, buyer + Lot-ReviewOrg, tenderer/winner + another role, every incoherent group listed with its `ref_roles` — reported for R2; the guard reads the notice references, not these); the census twin rows before and after the plan (`post_state`, R3's expectations, stable and multi-Tender rows apart); distinct values over the GUID rows; R1's bars (`bars`: planned rows, whole and partial, with every twin partner on a GUID row at or under census 2655's max id 31,970,152 against 11,258 — `planned_rows_comparable_whole` is the dissolve count — rows above that id and rows with a partner off GUID rows apart; the tripwires — planned rows spanning 2+ Tenders, stable at R1's fixed 100 mentions or a stable id whatever `stable_mentions` says, or in KEEP — must be 0, and `stable_mentions` must be 100; planned rows spanning 0 Tenders are not an R1 bar (R1's bar line names the three), but the wet re-plan refuses on them — the Dissolve plan's exactly-1-Tender tripwire; the not-planned controls 254, 255, 8378, 30530641, 3866, 3868, 2529, 2530, 21983972, 21983973, 21987063, 5351, 8756, 10966, 1085, 22040636, 22040637, 23014305, 30971684, 30971734, 30971757, 30532145, 30532147 and (Build 1b, R2's W3) 22752175, 22752176, 22752177; 22759261 and 22759262 planned whole onto 2294259; target-country mismatches 0 — per planned mention, its target key's and found row's country against `register_jurisdiction` of the stored mention country (a mention whose country is no 2-letter code is counted `target_country_unchecked`, reported); the KEEP self-check; the role self-check `tripwire_role_mix` — refused groups whose notice-reference roles, as the planner itself read them from `notice_ids` (never the roles the rule returns), are role-incoherent, each listed in `role_mix_refused` — must be 0, and so must `role_ref_disagree` — twin groups whose roles as the rule returned them differ from that read (a rule that lost them), listed with their `rule_roles` in `role_ref_disagree_samples`; `pass` is all of them), the expected-not-barred controls (the Göttingen triple 22761155/56/59 → 2294259, Vergabekammer Niedersachsen 23014306/07), the Göttingen exhibit (GUID rows planned, partial, kept and the kept ids), and R2's reads: 60 random planned rows stratified buyer 25 / review_or_info 25 / other or tenderer 10 at salt 530003 (`((org_id XOR 530003) × 2654435761) mod 2³²`; Build 1b, the fresh re-read of decision A5 / R2 F3), excluding the 60 rows R2 read at 530002 (`GUID_TWIN_R2_READ`; `r2_read_excluded`, and `r2_random_excluded` planned rows skipped) so the next rows in salted order fill each stratum, the 30 target keys with the most planned mentions plus any of the 20 with the most planned rows, and the two-word office keys, flagged targets and incoherent groups (each whole up to 45, else the top 30 plus 15 salted) — every list reads keys with no identifier-NULL row too (a missing key mints one row for all its mentions; its `sample_id` is the smallest org id among its planned rows). Role composition reads "not a winner" by class: a result winner beside a bid tenderer is coherent. Walks `organizations` by rowid windows of 10,000 ids (the census's statement) and reads the GUID rows in batches of 512 (mentions through `organization_mentions_org`, new notices by primary key, versions through `tender_versions_notice`, role class through the party, bid-party and winner `org` indexes), walks `organization_merged_identifiers` by rowid windows, then reads EVERY ORG mention of each notice with a GUID-row mention (the `organization_mentions` primary-key prefix, `section_id` in the ORG range) in IN chunks of 512 — and, for the copies among them holding a twin group (the only copies where the role guard can bind, so the read is exact), their role references through the `notice_ids` primary-key prefix (one IN statement per chunk; `is_ref = 1` or a `DE1-` field; counted as `role_ref_notices`, `role_ref_rows_read`, `role_refs`, `role_ref_ms`) — the off-GUID rows by primary key, the targets through `organizations_name_country` (with one count and one 50-row reverse walk of `organization_mentions_org` each), the cities through the `notice_texts` primary key, and the refused groups' roles through `tender_version_parties_mention_key`, `tender_version_bid_parties_mention_key` and the winners' primary key — refused while any of `organization_mentions_org`, `tender_versions_notice`, `tender_version_parties_org_role`, `tender_version_bid_parties_org`, `tender_version_result_winners_org`, `organizations_name_country`, `tender_version_parties_mention_key`, `tender_version_bid_parties_mention_key`, `organization_merged_identifiers_org` is absent. Notices with no mention on a GUID row are not read (a twin whose every value already sits off GUID rows is not seen). Stoppable before every window, batch and chunk and at least every 64 target seeks; a stopped run stores no report. Read-only; the report kind is `guid-twin-plan`. **Build 2 (dry):** the rule reads the COMPILED KEEP and names the gate reads — `crates/ingest/sdk/guid-keep-530.tsv` (job 2660's 7,312 `keep_candidates`, added to only by a reviewed append that bumps the file's `# version:` and `GUID_KEEP_530_VERSION` together; `keep_source: compiled`, `keep_version` — `keep-530-job2660` until then — `keep_size`) and `GUID_TWIN_UNSAFE_NAMES` (`unsafe_names`) — while the walk's KEEP candidates are still computed and reported; `bars.keep_self_check_missing` is the walk's (census twin ∧ ≥ 2 Tenders) ∪ stable values ABSENT FROM THE FILE (A3 iii: it fails the plan, and `keep_add_commit` holds their file lines, the commit to make before re-planning; `keep_candidates_not_in_file` counts the other candidates the file lacks, advisory); two more tripwires fail it — `tripwire_merge_survivor` (a whole planned row holding a merged identifier, read through `organization_merged_identifiers_org`) and `tripwire_under_review` (a whole planned row a review table names, `org_ids_under_review`); it also reports `notice_max_id` (the newest notice at its start), `log_high_water` (the highest `guid_twin_dissolve_log` rowid at its start: a wet run on this plan adds back, and its manifest reports, exactly the rows logged above it), `wet_windows` (the wet run's windows over this plan: their count, each window's `rebound_estimate`, the largest window, the largest target's window and `heaviest_window` — the staged run's `hold_ms / rebound_target_mentions` extrapolates to `heaviest_window.rebound_estimate`), the Göttingen exhibit's `partial_ids` beside its `kept_ids`, and — a control absent from the walk read against the dissolve log (`guid_twin_dissolve_log_org`) — `dissolved_by_wet` on each control: a planned or expected control a wet run dissolved whole onto its expected target is met (so a fresh dry plan after a wet window passes R1 and a later wet run is not wedged), a not-planned one a wet run dissolved is a miss, and one absent with no log row is a miss as before; and `refused_buyer_copies` (copies whose refused section the copy references as Procedure-Buyer: the plan row's buyer key keeps the published identity, `NoticeState::published_mentions`). `extra_unsafe_names` still runs, as a measurement: a wet run refuses a plan read with them. **Wet** (`{"kind":"dissolve-guid-twins","dry_run":false,"gate_since_notice_id":N}`; REQUIRED `gate_since_notice_id`, A1's boundary — the highest notice id at or below which every parsed notice was projected before Build 2's deploy (the runbook's step 1) — refused at enqueue when missing and, for a queued Spec without it, at run time, refused above the stored plan's `notice_max_id` (a post-deploy plan's start: a larger value is a typo that would hide notices), and, once a window logged one, refused unless equal to it; recorded as passed on every logged row, never estimated; optional `max_windows`, the staged first run is 1, counting the windows a restarted job already committed — `0` is MANIFEST ONLY (review round 3: every check runs, no window is applied, the report stores the campaign manifest; outcome `manifest_only`); `extra_unsafe_names`, `samples` (the re-plan and drift read list a fixed number) and a `stable_mentions` other than 100 are refused): writes nothing until every check passed — the stored `guid-twin-plan` must exist, be read on THIS binary (`keep_source` compiled, `keep_version` and `keep_size` and `unsafe_names` the compiled ones, and a `log_high_water`; a Build 1b plan is refused: re-run the dry plan), with no extra names, and pass R1; the org FK indexes must stand; then a RE-PLAN (about 23 s) with the compiled statics must meet every bar and tripwire (`bars.pass`'s every term, named on failure: the census bar, several-Tenders, stable, KEEP, role-mix, role disagreements, target country, merge survivor, under review, the compiled self-check with its KEEP-add lines, the controls — a planned control a wet run already dissolved is met through the log — plus the no-Tender tripwire, which is no R1 bar), a TUPLE RECONCILE must hold: the dry plan's `{(row, whole)}` equals the re-plan's at or under the dry plan's `max_id` plus the rows the wet runs on that dry plan already logged at or under it (`guid_twin_dissolve_log` above the plan's `log_high_water`: a staged first run, a restarted job), with no row both re-planned and logged (`replanned_logged` fails it); rows above the dry plan's `max_id` were minted since — the re-plan's are left out of the comparison, listed as `minted_since` and applied by the windows with the rest, the logged ones listed as `logged_minted_since` — and A1's REVERSE DRIFT since the boundary must be 0 (the recurrence arm's read, below: a KEEP-add or unsafe-name commit deployed since the boundary left mentions the gate refused on identifier-NULL rows; the refusal names the restore to run first), and no KEEP-add candidate of that read may have its own GUID row in the re-plan (a value the gate already refused in a second Tender; the refusal lists its KEEP file line), nor may the read's candidate list be capped. Then the re-plan's mentions are applied in WINDOWS BY TARGET KEY (`store::guid_twin_windows`: a union-find over the keys `(org_name_norm, register country)` and the GUID rows, components never split, packed to 2,000 planned mentions or ~120,000 estimated re-read mentions (`reread_budget`, each window's `rebound_estimate`), a component over either alone), each one `BEGIN IMMEDIATE` (`Db::dissolve_guid_twin_window`, timed: `wait_ms` for the writer, `hold_ms` from BEGIN to COMMIT, and `rebound_target_mentions`, the mentions 542's re-derivation re-reads under the lock — the cost driver; one journal line per window): every planned mention must still sit on its planned row, every planned row must still exist and carry its planned identifier (its pre-image — country, identifier kind, value — is read here and logged as it is), a whole row's recorded mentions must be exactly its planned ones, and no whole row may be under review (the review keep-set re-read on the writer inside the window, as the 443 sweep reads it, so a verdict POSTed during the re-plan or between windows refuses the window); whole rows dissolve through 440's per-org body (`Db::dissolve_org_mentions`, the id-set selector) and partial rows (A2) rehome exactly their refused mentions — parties and bid parties follow the mention key, winners the 309 origin tier (a winner of a kept mention stays; a tier-3 chain whose candidates mix moved and kept mentions is unresolved, never a guess; an unresolved winner on a moved mention's Tender goes to tier 5) — onto the target the issue-234 resolver arm finds (`(org_name_norm, register_jurisdiction(country))`, identifier NULL, `ORDER BY id LIMIT 1`; a missing key MINTS one row per key, country the register's, named as the key's lowest planned (notice, section) publishes it, provisional); each landed target must be the planned one (A11); before a whole row's delete every `organization_id` index is sought (a residue aborts the window, as does a whole row's winner whose chosen mention is another org's — never a winner repointed onto id 0); tier 5's Tenders are stamped stale and their notices re-queued, the re-bound rows' name variants re-derived (542), every moved mention logged in `guid_twin_dissolve_log` (job, window, the org's id/country/kind/value/name, whole or rehome, (notice, section), target, minted; no FK, not on `/v1/sql`), and the changes announced: `organization removed` (each whole row; it answers 404 with no redirect, as in 440/510/522), `organization added` (each mint), `organization changed` (each reused target and each rehomed row), `tender changed` (every Tender a moved row touched). After each COMMIT a TRUNCATE checkpoint and the stop check; a window error rolls that window back and ends the run (the committed windows stand). Stores `guid-twin-dissolve` whatever happened (`outcome`: applied / capped / manifest_only / stopped / manifest_unread / window_error / refused; `manifest_only` is `max_windows: 0`: every check ran, no window by design): the re-plan's bars, counts, A9's expected post-state, controls and exhibits, the reconcile, `reverse_drift`, the windows with their `timings` (summed and largest `hold_ms`, the window holding it), the summed counts, the MANIFEST built from the log — the CAMPAIGN on the dry plan read (every row logged above its `log_high_water`, whichever wet job wrote it, each mention with its `job_id` and `window_no`): `dissolved` and `rehomed` orgs with their moved mentions and their own identifier kind, `minted` rows (named from the window, or read by id for an earlier stage), `reused_targets`; a manifest the job could not read (one bounded read, retried) is `{"unread": …}` and the outcome `manifest_unread` (the job fails: the rows stand in `guid_twin_dissolve_log`, which `/v1/sql` cannot read; queue `{"kind":"dissolve-guid-twins","dry_run":false,"max_windows":0,"gate_since_notice_id":<boundary>}` on the same dry plan, before any new dry plan, to store the campaign manifest — `max_windows: 0` is MANIFEST ONLY (outcome `manifest_only`): every check runs and no window is applied, whatever windows the unread job left; the runbook's step 3) — `refold_tenders_this_run` beside the manifest (this run's windows' tier-5 Tenders, kept when the manifest is unread), `verify` (A10: the Göttingen `goettingen_kept_ids`, `goettingen_partial_ids` and `goettingen_surviving_ids` = kept ∪ partial, the list A10 compares with the live GUID ids beside ids above `wet_max_org_id`, read after the windows; and every stable row's org id, value and mentions for A9's per-row check) and `r3` (`manifest_mentions_campaign`, `manifest_mentions_this_job`, `mentions_moved_this_run`, `windows_log_equal` — each window this run applied logged exactly the mentions it moved — `minted_keys_equal_missing_keys_this_run`, `skipped` with `skipped_basis` — structural since tier 5, A12's tripwire, must read 0 — and `refold_tenders` with `refold_check`, A12's second half below). A refusal, a stop, an unread manifest or a window error fails the job after the report is stored; a `max_windows` stop is a PARTIAL success. A restarted job re-plans and resumes from its log. Heavy writer; stoppable during the re-plan and between windows. **Recurrence** (A1, `{"kind":"dissolve-guid-twins","recurrence":true}`, optional `since_notice_id` (positive, 0 refused) — once a wet window logged A1's boundary it must equal it and defaults to it; before that it may not exceed the stored dry plan's `notice_max_id` or reach the newest notice, and with none passed it is refused while a `guid-twin-dissolve` report is stored (every wet run refused, or a from-archive rebuild emptied the log: pass it explicitly, review round 3); read-only, report `guid-twin-recurrence` with the `boundary` used; refused before any read while one of the planner's indexes or `organizations_identifier_id` is absent): the targets are every `guid_twin_dissolve_log` `target_id` plus, with `since_notice_id`, the identifier-NULL rows bound by the sections rule 530-T refuses in the twin copies of later notices (the `organization_mentions` primary key by 20,000-id windows, ORG sections; a refused section on an identified row is an anomaly, listed); each target mention the gate's candidate test admits (its raw value, scheme and country gated to a national `uuid_v4` key — since review round 3 the gate's own test, so a braced or punctuated raw counts — keyed by that gated value) is re-decided with its whole copy (compiled KEEP and names, the copy's own role references), and per refused value the distinct Tenders over its refused mentions plus the mentions of EVERY GUID row holding it (an `organizations_identifier_id` seek; `own_rows`) are counted: a value at ≥ 2 Tenders outside KEEP is a KEEP-add candidate, listed with its whole restore cohort `(notice, section, target)`. It also reads REVERSE DRIFT — mentions on an identifier-NULL row that the CURRENT rule binds (rule 530-T's constants are add-only: a KEEP-add or `GUID_TWIN_UNSAFE_NAMES` commit leaves every mention the gate or a wet run already moved where it is — a refold never moves it back, `RecordedMention::publishes`, while a rebuild binds it by identifier): `stock`, a logged mention still on its logged target whose gated identifier is still a national v4 key and that the rule no longer refuses (one the gate no longer admits as a v4 identity — a condemn, a refused scheme, a re-parsed raw — is counted `reverse_drift_stock_ungated`, no drift: a rebuild binds it by name, where it sits; a logged mention a later merge or verdict moved off its target is not read); `since`, a mention of a post-`since_notice_id` copy (every copy, twin or not) whose gated identifier is a national v4 key (`guid_twin_gated_v4`), bound to an identifier-NULL row, that the rule does not refuse and that no reviewed rehoming verdict placed there (`org_mention_rehoming`, counted `reverse_drift_verdict_skipped`: a restore never undoes a person's move); each `(value, register country)` pair listed whole with its restore target (`own_row`: the identity `(country, national, value)`'s row or a merged identifier's survivor; else `mint`) and the pre-image of the GUID row a wet run dissolved for it. It also reads FORWARD drift, the mirror: a refused mention still on a GUID row (a re-parse that turns a copy into a twin rebinds the changed sibling and keeps the unchanged one) — `forward_drift_stock`, a refused section of a decided copy on a GUID row (a lower bound), and `forward_drift_since`, the anomalies on GUID rows; a dry plan plans them and a wet run moves them. Pass: 0 candidates, 0 reverse drift, 0 forward drift. R3 runs it once; after that it is queued BY HAND monthly (no scheduler arm) and after every deployed KEEP-add or `GUID_TWIN_UNSAFE_NAMES` commit, at the recorded boundary. **Restore** (A1, `{"kind":"dissolve-guid-twins","restore":true,"since_notice_id":N}`, the boundary required and anchored as the recurrence arm's; refuses every other mode's field and `samples`, and refuses before any read without the org FK indexes or the recurrence read's; report `guid-twin-restore`): reads the reverse drift as the recurrence arm does, then per `(value, register country)` pair, in one `BEGIN IMMEDIATE` each (stoppable between values, a TRUNCATE checkpoint after each): every cohort mention must still sit where the read found it; the target is the row a rebuild binds the value to — the identity's row, a merged identifier's survivor, or ONE row minted as the resolver mints an identified one (country, `national`, value, the lowest drifted mention's published name, not provisional; `organization added`); each current row rehomes exactly its drifted mentions onto it through 440's body (parties and bid parties by the mention key, winners by the 309 tier, tier 5's Tenders stamped stale); a row the rehome EMPTIED is deleted as the 443 sweep deletes one (`removed`: its `org_sweep_log` pre-image, `organization removed`) unless a party, bid-party or winner row still names it or a review protects it (`emptied_kept`: run `sweep-orphan-orgs`); name variants re-derived, `organization changed` / `tender changed` announced; the change-feed doorbell after each COMMIT is best-effort, as the wet window's is. Re-run the recurrence arm after it (expect 0 reverse drift), then `project` for any stamped Tender |
| `org-head-drift` | no | issue 524 unit 1 (and issue 456 unit 1's stray buckets): whether each organization's served head (`organizations.name`, written at mint and never again) still names what its mentions say (`{"kind":"org-head-drift"}`; optional `min_mentions`, default 2, under 2 refused; optional `samples`, default 30, clamped to 1–200). A mention VOTES when its name is neither NULL/blank (`nameless`) nor a non-name: `partyname::not_a_name` classes `void_lot`, `placeholder` and `blank` hold no key (`non_name_mentions`, by class, every walked org's); `award_summary` still votes (`award_summary_mentions`). For every organization with at least `min_mentions` mention rows and a voting one, the voting names are grouped by `crosswalk::altid_name_key` (primary) and `store::org_name_norm` (secondary), and the head's key is `aligned` (the top key, or tied for it), a `minority` (held by some voting mention, by fewer than the top key) or `absent` (held by none); an org whose rows hold no voting name is `unnamed`. Flags beside the buckets: `cosmetic` (aligned, but no voting mention spells the head byte for byte); `not_a_name` (a non-name head, `award_summary` heads not flagged; `head_class` counts the census heads by class, `award_summary` included); `single_stray` (the head's key held by exactly one voting mention, the top by `stray_top_min` = 2 or more, and the head's key agrees with no other voting key under `crosswalk::altid_keys_agree`: the Sheffield shape); `below_half` (issue 524 (b): head-key count × 2 < voting mentions — every minority and absent head, plus the aligned heads that lead only a plurality); `form_only` (minority or absent, but the head's key agrees with a key at the top count: a GB legal form one side carries, `Carnall Farrar` over `Carnall Farrar Ltd`); `minority_norm` / `absent_norm` repeat the test on the norm key. Every bucket and flag carries its organizations and their mention rows, split by `identifier` NULL or not, `provisional` and merge survivor (`merged`: an `organization_merged_identifiers` row with `loser <> org_id`, i.e. an identifier-bearing merge whose loser's identifier was recorded — the alias rows that name the org as its own loser (`altid-alias` binds, `renormalise` pre-images, `nhs-body` codes, the nhs-body wet path's keep alias among them) mark no survivor; a merge carries the loser's own rows to the keep with their original loser, alias and pre-image rows included, and those DO mark the keep (issue 514's E0 survivors such as 31558903 read merged only through a carried `renormalise` pre-image); identifier-less (p0/p1) folds, rekeys and same-identifier merges whose loser carried no row are invisible; `org_merge_log` has no index on `keep` and is not read). `stray_mentions` is issue 456's mention-level count: a voting mention whose key no other voting mention holds and which agrees with no other voting key of its org, in an org whose top key holds `stray_top_min` (2) or more, split by identifier NULL or not and `by_scheme` (`organization_mentions.scheme`, NULL as `none`, which takes one of the places; the first 20 distinct schemes the walk meets, by org id and then by name within an org, the rest under `other`); roles are `not_measured`. Samples: the first `samples` by id of `minority`, `absent`, `single_stray`, `not_a_name`, `cosmetic`, `below_half` (its aligned plurality heads only; the minority and absent ones are listed under those) and `form_only`, each with its head and head class, mention rows, voting mentions, head-key and top-key counts, the top key's most frequent spelling (a tie prefers mixed case over ALL-CAPS) and its distinct keys; `canaries` reports 31558903, 22713056 and 30914328 whatever their bucket. Walks `organizations` by id 2,000 at a time and reads each page's mention names and schemes and merged identifiers through their `org` indexes in 512-id chunks (refused while `organization_mentions_org` or `organization_merged_identifiers_org` is absent); stoppable between pages, and a stopped run stores no report. Read-only; the report kind is `org-head-drift` |
| `refresh-org-names` | YES (wet, since issue 524 unit 2b; dry by default) | issue 524 unit 2a: re-elects each organization's served head from its mentions under `store::elect_head` (ADR-0013's 2026-10-15 amendment) and stores what a wet run WOULD do as `org-name-refresh-plan` (`{"kind":"refresh-org-names","dry_run":true}`; optional `min_mentions`, default 2, under 2 refused; optional `samples`, default 30, clamped to 1–200; optional `org_ids`, 1–100,000 distinct positive ids, plans only those). The voters are `org-head-drift`'s, clustered by `crosswalk::head_vote_key` (`altid_name_key` of the name with its Latin diacritics folded, `crosswalk::fold_diacritics`, and every legal-form token dropped, so casing, diacritics, `the`, Ltd/Limited/none and S.R.L./SRL/none are one name). In order: KEEP `E1` identifier NULL, `E2` a `wrong` or unapplied `related` identifier verdict on the triple (probed for the candidates only, so `E2` counts the orgs the rule would otherwise change or hold), `E3` no voter; `R0e`/`R0n` an empty or non-name head takes the top cluster; KEEP `D4` when the head's dominant script (Latin, Cyrillic, Greek, other) differs from the elected cluster's; KEEP `S1` when the head's cluster is the top or tied for it; `R1` a unique top with ≥ 55% of the votes, ≥ 3 and ≥ 2× the head's — and, since rule `524u2-v7` (the recency decision), when the head's cluster holds ≥ 2 votes, HOLD `recent` while the head owns the organization's newest procedures (an item is a distinct (vote key, tender) pair of the dated voting mentions, dated by its newest `tender_versions.published_at`, a notice's tender the smallest of its versions; k = the head's items; the window is the newest k items, ranked `at` descending, a non-head item before a head item on an equal `at`, then tender descending, then key; it fires when k ≥ 2 and 3·h_W ≥ 2·k; a third cluster counts against the head), unless the head is JUNK — unfit against itself for any reason but `lowercase` (control, mojibake, long, non_name: since rule `524u2-v8` a lower-case head is a brand, 3909) or its vote key carries a token of ASCII digits only that the elected key lacks (an address number, a postcode, a NIP; since v8 never a digit inside a word, 1846's `CA2BM`; since rule `524u2-v9` never a number the elected key holds glued to its neighbouring head word, 4295185's `SMD 3` → `Smd3`, and only where the elected key lacks at most ONE other head word of 3+ chars with no digit, so the elected name is the head minus junk — `SDIS 49` → `Service départemental …` releases, 9814's owner-and-street head does not) — (released and counted `recent_released`: `unfit` / `digits`; a `digits` release v9 stops keeps its hold and is counted `release_withheld`: `glue` / `junk_only`); with no dates HOLD `undated` (fails closed; a finished plan must read 0); `R2` a head cluster of ≤ 1 vote under a unique top of ≥ 3 and ≥ 25%; else KEEP `bar`. Since rule `524u2-v8` (the residue holdout, issue 524 `## Rule v8 — decided`), an `R1` or `R2` change whose organization's single NEWEST dated item is a head item (`newest_is_head`; on an equal `at` any other item ranks newer, so the head must be the only one of its names on the newest publication day) is HOLD `residue` with the spelling it would serve, unless the head is junk as above (released and counted `recent_released`); undated, it changes as before (`R1` with H ≥ 2 holds `undated` first); R0 is never held. Since rule `524u2-v9` (issue 524 `## Rule v9 — decided`, after plan 2526's dated re-judge), an `R1` or `R2` change that reaches its served spelling past the guards below is HOLD on five pure tests of the head and that spelling (`crosswalk::head_serve_hold`, asked after `head_change_hold` and `head_r2_hold`, never for R0; `recent`, `residue` and `undated` decide first; the first that holds names it; words are `crosswalk::fold_diacritics` split on non-alphanumerics): `review_body` (the new name holds a procurement review body's phrase — `vergabekammer`, `krajowa izba odwolawcza`, `tribunal administratif`, `tar`, … `crosswalk::HEAD_REVIEW_PHRASES` — and the head is not review-like: no such phrase, no stem of `crosswalk::HEAD_REVIEW_STEMS` anywhere, not led by `ta`/`tar`/`vk`; so a review body's self-rename, 791, 100, 1919, passes it — since rule v10 the canaries 791 and 1919 then hold `head_current`), `organ` (the new name is or starts with a contracting organ's phrase, `crosswalk::HEAD_ORGAN_PHRASES` — `junta de gobierno`, `alcaldia`, `pleno`, `presidencia`, … — and the head does not; the reverse, canary 2762, passes it — since rule v10 it then holds `head_current`), `code_added` (the new name adds a whole number the head's alphanumeric stream lacks, in a code position — inside `(…)`/`[…]`, joined by `_`, the last token, a leading list number before `.`/`)` — and its vote key less those numbers is the head's: `Commune de Chiconi (976)`), `address_added` (such a number in a `,`/`;` segment after the first, unless the folded name holds a Czech/Slovak school stem `skol`/`jidel`/`jedal`/`gymnazi`/`internat`/`druzin`) and `caps_variant` (an ALL-CAPS new name over a mixed-case head with the same words of 2+ chars, legal-form words aside — `Sp. j.` and `SPÓŁKA JAWNA` alike); since rule v11 two more, after those five: `doubled` (the new name's words are X then X again, X with a word of 3+ chars, and the head's are not: 1918631's `Instituto Foral de Bienestar Social - Instituto Foral de Bienestar Social`; separators are no words, so `X - X`, `X / X` and `X X` alike) and `seat_dropped` (the head's words are the new name's — 2 or more — plus a tail of 2–3 words with no digit whose first is `w`, `we`, `v`, `ve` or `vo`, standing in the head as a whole word before whitespace: 5350675's `Ośrodek Pomocy Społecznej w Kluczborku` → `Ośrodek Pomocy Społecznej`; `Sandefjord kommune v/VOIS` → `Sandefjord kommune`, an agent tail, changes). Since rule `524u2-v10` (issue 524 `## Rule v10 — decided`, from the dated re-judges of plans 2523, 2526 and 2527: 4 of 29 change:R1 items on a head still in print read wrong, 1 of 61 on a head not printed for 3+ years), an `R1` change that passes all of the above is HOLD `head_current` (with the spelling it would serve) when the head's newest dated item (`head_newest`) is at or after the plan's `as_of` less 1,095 days (`store::HEAD_CURRENT_DAYS`), unless the head is junk as above (released and counted `recent_released`; a `digits` release v9 stops reads `release_withheld`) or the window or the residue holdout already released it (canary 3042); R0 and R2 are never held by it, and an `R1` with no dated head item follows v7. The plan stores `as_of` (the planner's start, unix seconds) and `as_of_date`, and the summary prints `Rule v10: R1's head_current hold (a head printed within 1095 days of as_of YYYY-MM-DD): N`, at zero too. Since rule `524u2-v11` (issue 524 `## Rule v11 — decided`, panel wf_5d3b6fa8-186, after the capped run 2555's dated spot check), an `R1` change that passes all of the above, `head_current` included, is HOLD `top_stale` (with the spelling it would serve) when a RIVAL votes — a cluster other than the elected one and the head's, `clusters > 1 + (H > 0)`, so with H = 0 two clusters are a rival — and the elected cluster's newest dated item (`top_newest`) is before the plan's `as_of` less 365 days (`store::HEAD_TOP_STALE_DAYS`; no reading or no `top_newest` counts as stale: it fails closed), unless the window, the residue holdout or `head_current` already released the head (`recent_released` set: canary 3042); it never asks the junk exemption itself, so it mints no release, and R0 and R2 are never held by it. Every dated entry also carries `rival_newest` (the newest dated item of a rival cluster, `YYYY-MM-DD`), a diagnostic no verdict reads (issue 548's direct supersede test). The summary prints `Rule v11: R1's top_stale hold (a rival cluster, and the elected spelling's newest dated item more than 365 days before as_of YYYY-MM-DD, or undated): N; the placeholder_id bind guard: N`, at zero too. Since rule `524u2-v12` (issue 552 `## Unit 2 — decided`, after plan 2543's re-judge read all five wrong `unfit` releases on heads whose only defect is a line break), the junk exemption's `unfit` release of a CONTROL-ONLY head (`store::head_control_only`: `control` against itself, its whitespace-collapsed form carrying no control character or U+FFFD, and that form fit, or lower case with no non-name class; control plus `long` or mojibake is not control-only and releases as before) is asked of the LINES GUARD (`store::head_lines_guard`): the head split into lines on the characters that are both control and whitespace (`\t \n \x0b \x0c \r \x85`), each trimmed and keyed with `crosswalk::head_vote_key`, the release stands only when the elected key's tokens are a subset of one line's tokens or equal the union of a contiguous run of whole lines; otherwise it is withheld (`release_withheld: lines`, counted only on a verdict that stays a hold) and the `recent`, `residue` or `head_current` hold stands — unless the head has a break and EVERY break is a WRAP (`store::head_wraps`: the line before ends in no legal form, `crosswalk::head_tail_family` reading its last 1–5 words joined — `Sp. z o.o.`, `S.C.p.A.`, `GmbH & Co. KG` included —, and in no `,` `;` `:`, and its last word ends in an unspaced `-`/`–`, or is a connector with no `.` from `store::HEAD_WRAP_CONNECTORS` — `und and de del della dei degli di du des der den of for für fur i z w ze we e ed et y και του της των en van voor u na do od a o` —, or the next line starts lower case) and the junk exemption would not release the collapsed head (a bare number the elected key lacks would release it `digits` on the next plan, serving the withheld spelling after all: then the `lines` hold stands): then it CHANGES to the head with its whitespace collapsed, rule `R0w` (`by_rule.R0w`; the entry reads `release_withheld: lines`), asked only of the bind guards (`crosswalk::head_change_hold`: a reason holds it as `hold:<reason>` with rule R0w), never of the recency, residue, `head_current`, `top_stale` or serve holds. Its vote key is the raw head's, so the written head stays in its cluster and its holds keep standing. The summary prints `Rule v12: control-only unfit releases the lines guard withheld (release_withheld lines, held): N; R0w wrap collapses (the head's own words, its line breaks collapsed): N (M mentions)`, at zero too. The served spelling is the winning cluster's FIT spelling (`crosswalk::head_spelling_unfit`, read on the spelling as published and once collapsed: no control character or U+FFFD, no mojibake (`Ã`/`Â`/`Ä`/`Å` before a continuation byte's character or `€`/`™`, `È™`/`È›`, or `Ã`/`Â` after a lower-case letter at a word's end), ≤ 200 characters, not all lower case, no non-name class, no placeholder/void net hit or consortium mark the head lacks), with its whitespace collapsed (rule `v6`: trimmed, every run one space, and the fit spellings that collapse alike counted together, so no change serves a doubled space — often a dropped `&` —, and `crosswalk::head_vote_key` reads whitespace runs as one space, so the served head keys into its own cluster; a display equal to the head is a keep), mixed case before ALL-CAPS, then the most mentions, then byte order, and then, only if that winner has no non-ASCII letter, the most-mentioned of its TWINS of its case (`crosswalk::head_twin_key`: equal once lower-cased, folded and stripped of punctuation) that has one; none fit is HOLD `no_fit`, `crosswalk::head_change_hold` holds a legal-form family swap read on the folded names, and on them with dotted single letters joined (`family`, `gb_family`; `AG` → `Sàrl` or `S.à r.l.`; since rule `524u2-v13`, issue 548 U2, only when the head's own cluster holds H ≥ 2 votes, `store::HEAD_FAMILY_HOLD_MIN_VOTES`), a dropped consortium mark (`consortium_removed`; since v13 not when `crosswalk::head_consortium_subset` passes: the new name's words — `fold_diacritics`, split on non-alphanumerics, single-letter runs joined — less its legal forms, `family_token`'s keys and `R2_LEGAL_FORM_TOKENS`, are non-empty and all words of the head), a failed NHS ODS name check the old head passed (`nhs_check`) or (rule v11, every rule, R0 included, last of the four so each older guard keeps its reason) a placeholder identifier (`placeholder_id`: `KEINEANGABE`, `UNBEKANNT` or `NICHTVORHANDEN` followed by digits only, in any letter case, or all zeros — 22831984's `KEINEANGABE7`), and, for R2 only, `crosswalk::head_r2_hold` holds an `initialism` of the head (an upper-case token of 3–5 letters, no legal form or designation in `crosswalk::R2_LEGAL_FORM_TOKENS`, that is the initials of a run of the head's words, short particles skippable, the new name no longer spells out and that are not single letters — in a new name that is nothing but the token or is not ALL-CAPS: `CDC`, `CDCPC` ↔ `Communauté de communes du Pays de Caux`; unit 2a.1's head-cut-short arm is gone). The summary always names the initialism count. The plan: changes, holds and keeps by rule and reason (orgs and mention rows), the empty heads, the planned changes' mention sum (the joined tender reach's upper bound) and merge survivors, their legal-form family moves (`none->none`, `none->x`, `x->none`, `x->x`, `x->y`, each read on the folded name; an `x->y` read across the two family tables, `sarl` -> `ltd`, or (since rule v13) a same-table swap at H ≤ 1, is counted, not held), the D4 keeps' ENG satellites (`d4_eng`: `equal`/`agrees`/`differs`/`absent`), the `planned` and `held` entries by org id (old and new name and `name_norm`, V/T/H/second/clusters, merged, families, the top 5 voting spellings; each capped at 100,000 with `truncated` / `held_truncated`), the `samples` per `verdict:reason` with the smallest `((org_id XOR salt) × 2654435761) mod 2³²` (uniform, not the oldest ids; since rule v9 the salt is FNV-1a of the rule `epoch`, `store::ORG_NAME_REFRESH_SAMPLE_SALT`, so each epoch's samples are a fresh draw and a re-judge needs no skip list), the canaries with their expected verdicts (`canary_readings`, 60 rows since rule v12: keep:S1 for the 2516 renames 31558903, 22713056, 6341366, 760 and 61, for 10, (v11) for job 2554's six renames 4220276, 4823670, 8092631, 8152890, 9766149 and 4993534 (v10's change:R1 canaries, no expected spelling), and (v12) for job 2565's eight renames 8765882, 5393731, 9856301, 7822785, 11268197, 5011106, 2857103 and 2715115 and capped job 2567's renames 3042 (a `digits` release v10 and v11 kept changing) and 1800186 (v11's fresh top with a rival above the head) — 2567 renamed the first 500 ready targets by org id, its applied samples reaching org id 2,675,242; keep:D4 7912592; keep:bar 131 and 99; keep:E1 1165973; hold:recent 178 and the 2517 former-name reverts 2186, 3262, 3694, 6867, 10671, plus 1076, 10603 (plan 2523 dated it: its head form names the newest procedure), and v7's wrong releases 3909 and 1846; hold:residue 3740262 (an organ for its entity), 445 and 334 (each one's single head procedure is its newest item), and (v9) 4295185 (`glue`) and 9814 (`junk_only`), v8 releases withheld; (v10) hold:head_current, each head printed within 3 years of plan 2527 and each hold naming the spelling it would serve, the 2517 rights 1220, 2762, 3406, 6079, 11179, 1919 (a review body only) and (v9) 791 (a review body's self-rename); (v11) hold:top_stale 1556789, 1276213 and 1345526 (job 2555's revert and two wrongs), hold:doubled 1918631, hold:seat_dropped 5350675 and hold:placeholder_id 22831984 (`KEINEANGABE7`, a v10 `digits` release), each naming plan 2543's spelling; (v9) one per serve hold: hold:review_body 22028982, hold:organ 4065575, hold:code_added 17116440, hold:address_added 8069375, hold:caps_variant 16508165 (an R2); 30914328 reported only; an expected canary the walk does not find reads UNEXPECTED, and one a scoped plan did not ask for reads `outside the scope`), the rule `epoch` and the binary `rev`. Walks `org-head-drift`'s pages (2,000 orgs; mention names and notice ids through `organization_mentions_org` in 512-id chunks; refused while it, `organization_merged_identifiers_org` or `tender_versions_notice` is absent — the last a rebuild's fold drops), plus, per candidate, one verdict-probe seek, the merge-survivor read and, for a D4 keep, one `organization_names` ENG seek, and (rule v7) the candidates' notices' `tender_versions` rows through `tender_versions_notice` (512 per statement; a notice dated ≤ 0 or more than a day past the job's start is undated), after which every candidate is re-elected with its dates. The v7 plan adds the dated counts — `dated_orgs` and `undated_mentions` over the population's candidates, and the date reads' own `dated_notices` and `dated_version_rows` (per page, below-min canaries included, a notice shared across pages read once per page: they size the read, they are no denominator) — `recent_released` (the CHANGES the junk exemption released from a `recent` or, since v8, a `residue` hold; a release `serve` then held is not counted) with up to 500 ids by org id (`recent_released_ids`, since the v8 review: the re-judge reads every release; the job line names the first 30), the `residue` (planned changes whose org's newest item is the head's that still SHIP since v8 — R0's and the junk-released; sampled as `residue:head_newest`; the held residue is `by_hold.residue`, sampled as `hold:residue`), and per entry `head_procs`, `head_in_window`, `head_newest`, `top_newest`, `newest_is_head`, `undated_mentions` and `recent_released`; the summary prints `R1's recent hold: N; R1/R2's residue hold: N; undated N (MUST be 0)`. Rule v9 adds `release_withheld` (the HOLDS whose `digits` release v9 withheld, by `glue` / `junk_only`; since rule v12 also `lines`, a control-only head's `unfit` release the lines guard withheld) with up to 500 ids (`release_withheld_ids`; the job line names 30) and the entry field `release_withheld`; the serve holds land in `by_hold`, `held` and the samples (`hold:review_body` …); the summary prints `Rule v9: R1/R2 serve holds: review_body N, organ N, code_added N, address_added N, caps_variant N, doubled N, seat_dropped N; releases withheld (digits, rule v9: glue, junk_only; control-only unfit, rule v12: lines; …): … (ids […])` (rule v11's two serve holds in the same list), every reason named at zero too. Stoppable between pages; a stopped run stores no plan. The dry run is read-only: nothing but the report is written. **Wet** (unit 2b, `{"kind":"refresh-org-names","dry_run":false}`; optional `org_ids`, the same check, renames only those of the plan's planned ids; optional `max_groups` ≥ 1 caps the renames, the first ready ones by org id; since rule v9 optional `exclude_rules` (`R0e`, `R0n`, `R1`, `R2` and, since rule v12, `R0w`; not all of them) and `exclude_released` (`digits`, `unfit`) drop the plan's planned changes elected by those rules or released by the junk exemption that way (`recent_released`) BEFORE the pre-pass — never read, re-elected or renamed, and out of the drift limit's count — and the pre-pass also reads a target whose re-election now ships only through an excluded release (a head that named newer procedures since the plan) as `drifted`, so the filter holds at wet time too; an asked `org_ids` entry they drop is counted excluded, not `unplanned` — refused on a dry plan, empty, or with a value outside those lists, recorded on the params line (`… exclude_rules=R2 exclude_released=digits`), the job line (`Filters (rule v9): … — N planned change(s) in scope dropped before the pre-pass (… by rule, … by junk release)`) and the report (`exclude_rules`, `exclude_released`, `excluded_by_rule`, `excluded_by_release`, `excluded_ids`, the first 30); since rule v11 optional `exclude_stale_top: true` (the decision's pre-built fallback, off unless asked; refused on a dry plan, whatever its value) drops, after those two, every planned `R1` change with no `recent_released` whose stored `top_newest` is missing or falls on or before the day a year before the plan's `as_of` (`store::head_top_stale_date`: a stored day reads as its first instant, so the cutoff's own day is dropped), and the pre-pass drifts a target whose re-election is an `R1` change no release shipped on a top older than `as_of` less 365 days (or undated), so the filter holds at wet time too — recorded as ` exclude_stale_top=true` on the params line, `Filters (rule v9, v11): … (… by rule, … by junk release, N by stale top)` on the job line and `exclude_stale_top` / `excluded_by_stale_top` (and the ids in `excluded_ids`) in the report; `min_mentions`/`samples` are the plan's and not read): applies the STORED `org-name-refresh-plan`, refusing — with nothing written — when none is stored, when it was elected under another rule `epoch` (a v10 plan is refused by a v11 binary) or by another binary `rev` (re-run the dry plan after every deploy), when it carries no integer `as_of` (rule v10: `the stored plan carries no as_of`), when it holds anything `undated` (`by_hold.undated` must be 0), when it is stopped or `truncated`, or when it is scoped and the wet run is not; and while `tender_versions_notice` (or a mention index) is absent. A pre-pass re-elects every planned id as the planner did (same fns, verdict probe included, every target that passes the row checks dated as the planner dated it, the stop flag read between 512-target chunks; a target that now reads `hold:recent` — its head named newer procedures since the plan —, `hold:residue` — its head now names the newest one — or `hold:undated` is `drifted`; since rule v10 the pre-pass re-elects with the STORED plan's `as_of`, not the clock, so a target whose head was printed after the plan reads `hold:head_current` and is `drifted` too; since rule v11 so is one that now reads `hold:top_stale` — a rival cluster gained since the plan —, a v11 serve hold or `placeholder_id`): `ready` (still the plan's pre-image, identifier, rule and spelling), `already` (holds the plan's new pair: a replay), `drifted`, `gone`; drifted + gone above max(2%, 5) of the ids read ABORTS the run before any write (the job fails, the report is stored). Then windows of 2,000 renames, each one `BEGIN IMMEDIATE` holding the compare-and-set `UPDATE organizations SET name, name_norm = org_name_norm(name) WHERE id AND name AND name_norm IS <the plan's pre-image> AND identifier IS NOT NULL` (a row changed since the pre-pass is counted `raced` and left), an `org_name_refresh_log` row (the byte-exact pre-image, the new pair, the rule, `evidence` with the epoch, V/T/H, since rule v7 the window's `head_procs`, `head_in_window`, `head_newest` and `top_newest`, and since rule v10 the plan's `as_of` (`YYYY-MM-DD`), `job_id`) and one seq-less `organization changed` row, and the job's resume cursor in the same transaction; after each COMMIT the cursor doorbell, a TRUNCATE checkpoint and the stop check (a crash restart resumes after the last committed window, and its `max_groups` counts every rename the job id already made, cursor or not). A window that fails after one of this job id committed (in this run, or an earlier attempt of a recovered job) ends the run: the report (with `window_error`, and `left`, the ready targets never committed) is stored, then the job fails naming the renames that stand. Only a job with no committed window at all gets the bare error, nothing written and no report; a refusal or error before the report is stored (no plan, another `rev`, an index missing, a failed pre-pass) on a job that has renames names their count and the revert. No Tender, lot, `organization_names`, `org_name_verdicts` or `org_match_keys` row is written (`parties[].organization_name` is a read-time join; `org_match_keys` catches up at its next build). Stores `org-name-refresh-applied` (the pre-pass counts, `prior_applied`, `applied`, `raced`, `capped`, the first 30 `drift_samples`, 30 uniform `applied_samples` with old and new name, `window_error`, the asked `unplanned` ids, and `prepass_stopped_after`). The ledger is not on `/v1/sql`: spot-check renames through the report and `GET /v1/organizations/<id>`. Heavy writer; stoppable between the pre-pass's 512-target chunks (nothing written by the run; the report's `prepass_stopped_after` is how many targets the pre-pass read, and it carries no pre-pass count: `ready` … `left` read 0) and between windows (the committed windows stand). Runbook below |
| `revert-org-name-refresh` (`job`) | YES (no dry mode: `dry_run: true` is refused; no per-org or capped revert: `org_ids`, `max_groups`, `min_mentions`, `samples`, `exclude_rules`, `exclude_released`, `exclude_stale_top` are refused) | issue 524 unit 2b: undoes ONE wet `refresh-org-names` job (`{"kind":"revert-org-name-refresh","job":<id>}`; `job` required). Each of that job's `org_name_refresh_log` rows not yet reverted is restored to its pre-image under a compare-and-set on the pair the job wrote, stamped `reverted_at`/`reverted_job`, and announced as one `organization changed` row, in 2,000-row `BEGIN IMMEDIATE` windows with a TRUNCATE checkpoint after each. An organization renamed again since (a later job) is left as it is and counted `renamed again`; one that still holds the job's pair but whose identifier a case review stripped since counts `identifier stripped` (the restore is guarded `identifier IS NOT NULL`, as the rename is: its `name_norm` is a bind key now), one renamed again and stripped counts `renamed again`; a deleted one (merged away, swept) counts `gone`; none is stamped. A window that fails after one committed ends the run as the job's error, naming the restored count. Revert two jobs newest first. Heavy writer; stoppable between windows, and a re-run restores the rest. Stores no report: the job line carries the counts |
| `repair-label-prefixes` | **dry default** | issues 328/359: strips a publisher's field name glued to the identifier (`USTID…`, `NIP…`, `PIVA…`, `CIF…`), then re-validates |
| `repair-renormalised-identifiers` | **dry default** | issue 345: standing rows re-read under the live normaliser (Greek lookalikes, RO sub-unit suffix; since issue 514 unit 2 the GB register label spellings, `GBUKPRNUKPRN…` → `GBUKPRN…`). Since 514 the WET pass moves only the `(org, from, to)` rows of the stored dry plan (`held_unplanned` counts the rest) and refuses a `plan_truncated` plan; each move records its pre-image in `organization_merged_identifiers` under rule `renormalise` (`preimages`), so the old spelling keeps answering `?identifier=` |
| `repair-notice-instants` | **dry default** | issue 367: re-derives `notices.published_at` / `dispatched_at` from each parsed notice's own stored parse. Dry stores `notice-instant-repair`; the wet arm reads that plan's `rows` and aborts outside max(2%, 5). Read `resolver_silent` before running it wet — that class REMOVES a stored value (a dateless payload stamped 1970-01-01), where `epoch_published` and `null_published` add one. Writes `notices` only: the tender versions were never wrong, so nothing is re-projected |
| `sweep-orphan-orgs` | **dry default** | issue 443: deletes the organizations no recorded mention points at any more (issue 434's refresh re-binds a mention and leaves the row it left) — only when no party, bid-party or winner row names them either (`referenced`, kept until a fold moves those rows) and no review table does (`protected`: case reviews, re-homing case and target, country and merge verdicts, name drops — read inside every window, so a verdict POSTed mid-run protects its org). Provisional or not (step 4: the non-provisional orphans, tallied under `identified`, are identities no evidence carries any more). Each deleted row leaves its pre-image, identity and name variants included, in `org_sweep_log` (restorable from that row alone), takes its `organization_names` rows and publishes `organization removed`; foreign keys stay ON. Refuses to start while any of its five `(organization_id)`/`(org_id)` indexes is missing (run `reindex`). Every run counts first: dry stores `orphan-org-sweep-plan`, wet holds its count against that plan's `swept` and aborts outside max(2%, 50), then re-records the residual after every window that deleted, so a stopped, failed or restarted wet run resumes under the same parity. **A fold that re-bound or (issue 510) retired any mention queues it by itself** (job params `sweep-orphan-orgs auto (after a fold re-bound N and retired M)`, unless one is already queued or running): that run counts, sweeps at or under 10,000 orphans (`AUTO_SWEEP_CAP`) with no stored plan needed, and above it records the plan and writes nothing, leaving the wet run to a person |
| `backfill-merged-identifiers` | **dry default** | issue 460 unit 3, one-shot: gives the identifiers merges folded away BEFORE `organization_merged_identifiers` existed a place on today's survivor, so `?identifier=` finds it by them (every merge since writes its loser's identifier itself, in `repoint_org_references`; never a `rekey` loser's). Reads `org_merge_log` per rule: `e2-altid` through its partial index, then one rowid walk of the whole ledger (50,000 rows a window) for `r2`, `e0` and `r3`, whose evidence names the loser's literal as `$.loser_id` (`p0`/`p1` use that key for the org id and are never read). Each keep is followed through the merge ledger to its live survivor (`resolve_org`'s walk, 8 hops), with time checked three ways, any failure `out_of_time` and refused: the merge must be no older than the **era floor** (`created_at` of the lowest live org id — the oldest live organization; a from-archive rebuild re-mints every org id from 1, so a ledger row older than the floor names ids that now belong to other companies, and when such an id was minted again and merged in the new era the per-hop checks cannot see it), a hop must not be merged before the merge it continues, and the live row must not be minted after the merge that names it. The row takes the survivor's country and the kind the live normaliser gives the literal there (`kind_from_survivor` counts the literals it refused, which take the survivor's kind). A literal that spells (canonical key or literal) a number a reviewer found is not its carrier's — a `wrong` identifier verdict, applied or not, or a triple a `rekey` acted on — is written only when the survivor carries that number itself (its real owner); otherwise it is `wrong_number` and refused: the chain crossed a `rekey` merge onto the right company (`r2` and `e0` rows alike), or the survivor was re-keyed in place by the move arm. Per rule it counts `rows`, `written`, `present` (a row already stands for that identifier and loser, or the survivor already carries the identifier — a merge or an alias bind wrote it), `same_as_survivor` (nothing was lost — an `e0` row whose survivor still carries the number), `no_literal`, `unresolved` (no live survivor), `out_of_time`, `wrong_number` and `loser_live`; the report carries the `era_floor` it held rows against. Writes that lookup table and publishes `organization changed` for each survivor it wrote for (`organizations_changed`; its served `merged_identifiers` grew), one transaction per window; idempotent, stoppable between windows (a stopped run stores no report; its committed windows read as `present` next time). Both modes store `merged-identifier-backfill`. Expected on prod (2026-10-01): `e2-altid` 4,738 rows; `r2`/`r3` unknown until the dry run reads them. **Read the dry run's `out_of_time` and `era_floor` before the wet one**: a large `out_of_time` with an `era_floor` later than the first `e2-altid` jobs (1717) means a from-archive rebuild happened since, and those rows are refused by design. **After a from-archive rebuild** the table is emptied (it names re-minted ids) and this job cannot refill it (every older ledger row is below the floor): the fold's altid alias records each merged PPON it binds (above). Another spelling of an E1 register key binds to the org already holding that key (the resolver's canonical-key prevention, issue 300 Stage 2), so in a rebuilt fold it is never minted apart and never merged, and `?identifier=` by that spelling answers nothing: the documented exact-value scope of `identifier=`, not a lost merge |
| `analyze` | YES (`sqlite_stat1` only) | issue 429: `ANALYZE` of each table in `store::ANALYZE_TABLES` — the 24 read-path tables issue 428 measured a benefit on — one table per statement, so the writer is released between tables (the longest, `tender_version_parties`, took 242 s on prod data) and a stop costs at most the table in flight. Never touches a table outside the list; `organizations` is excluded (its resolver lookup went from 0.001 s to 6.19 s with statistics) and any `sqlite_stat1` row for it is deleted. Always ends, even when stopped, with a throwaway `CREATE`/`DROP TABLE` so pooled readers re-plan with the new statistics (they otherwise keep what they loaded at open, `tests/analyze_stats_pickup.rs`). The summary lists every table's seconds. Not scheduled yet: the schedule waits on 429's plan-capture diff |
| `repair-minted-countries` / `repair-placeholder-orgs` / `repair-nested-orgs` / `repair-swept-siblings` | **dry default** | the earlier repairs (issues 325, 300 Stage 1, 234, 259) |
| `ghost-census` / `disk-census` | no | issues 278 and 169: weekly stamps. `disk-census` also carries the live file's `db_allocated_bytes` beside `db_bytes` and a `db_overallocation_alarm` when allocation runs more than 5 % (and 1 GiB) past the size — the copy-on-write leftover class of issue 169 item 3, with the reclaim command in the alarm text; and `deleted_open_files` / `deleted_open_bytes` (+ a five-entry sample and `deleted_open_alarm` at 1 GiB) — files the server process holds open after unlinking them, issue 361's class, also on `/metrics` as `tender_db_deleted_open_{files,bytes}` |

Report kinds do NOT always match the job kind that writes them. `fusion-census`
stores under `fusion-candidates`, and `GET /admin/reports/<kind>` answers an
unknown kind with "no report of that kind has been computed" — which reads as
"the job never ran". The kinds are, exhaustively (read off `put_report` in the supervisor, 2026-09-06; `altid-merge-plan` since issue 448, `rekey-plan` since issue 453, `merged-identifier-backfill` since issue 460, `tender-link-backfill` since issue 481, `procedure-key-census` since issue 482, `buyer-role-census` since issue 483, `e0-merge-listing` since issue 514): `altid-merge-plan`, `anchor-wall-census`, `buyer-role-census`, `case-apply-plan`, `case-escalations`, `case-unapply-plan`, `country-cluster-census`, `country-cluster-packet`, `country-fold`, `country-typo-census`, `country-typo-repair`, `country-verdict-plan`, `data-quality`, `data-quality-headlines`, `data-quality-presence`, `disk-census`, `drop-orphan-satellites`, `duplicate-identity-census`, `e0-merge-listing`, `e0-merge-plan`, `fusion-candidates`, `generic-statistic-census`, `generic-wall-census`, `ghost-census`, `guid-census` (issue 530), `guid-twin-plan` (issue 530 unit 3), `guid-twin-dissolve`, `guid-twin-recurrence` and `guid-twin-restore` (issue 530 Build 2), `label-prefix-repair`, `member-twin-census`, `member-twin-repair`, `merged-identifier-backfill`, `minted-country-repair`, `name-attribution-probe`, `name-phrase-census` (issue 527), `name-pollution-census`, `notice-instant-repair`, `org-edge-census`, `org-edge-scan`, `org-edge-scan-alarm`, `org-edge-scan-plan`, `org-head-drift` (issue 524), `org-name-refresh-plan` (issue 524 unit 2a), `org-name-refresh-applied` (issue 524 unit 2b), `org-match-keys-build`, `org-match-keys-plan`, `org-merge-health`, `orphan-org-sweep-plan`, `procedure-key-census`, `provisional-echo-census`, `provisional-echo-plan`, `provisional-name-norm-plan`, `r2-census`, `r2-merge-plan`, `r3-census`, `r3-merge-plan`, `registry-contiguity`, `rehash-cursor`, `rehash-probe`, `rehoming-packet`, `rehoming-plan`, `rekey-plan`, `renormalise-repair`, `reveal-cursor`, `reveal-recheck`, `reveal-wrap`, `satellite-orphans`, `tender-link-backfill`, `void-names-refold` (issue 510), `placeholder-names-refold` (issue 511), `xb-packet`, `nhs-body-plan` (issue 513 U2), `nhs-body-unwind` and `nhs-body-canary` (issue 513 U4).

### The GUID twin dissolve (issue 530 Build 2)

Build 2 wires rule 530-T into the fold (`NoticeState::mentions_and_dropped`): a notice copy that
names one name-safe body under two or more v4 platform keys refuses the keys outside the frozen
KEEP (`crates/ingest/sdk/guid-keep-530.tsv`), and those mentions bind the body's name-keyed row.
**Identity is contextual from the deploy on**: a tool that rebuilds a mention's identity from its
stored row alone (513's nhs-body canary, 511's guard 3) sees a GUID the fold refused; a reader that
must agree with the fold re-derives the twin from the notice's stored mentions, as the planner and
the recurrence arm do. The plan row's buyer key keeps the published identity, so the deploy moves no
Tender. (The link guard's resolved-organization token `o:<id>` follows the binding — a refused buyer
gets the name-keyed row — which can only widen the guard's overlap; its parsed-side tokens keep the
published identity.) Between the deploy and the wet run only NEW twin notices bind the new way; the
stock moves with the wet run, once. Its order:

1. Before the deploy, read A1's boundary: the highest notice id at or below which every parsed notice
   was projected before the deploy. The gate runs at projection, not at ingest, so a notice parsed but
   not yet projected at deploy time is folded under the gate although its id is lower. Read it in a
   queue gap after a `project` that ended `ok`, with no `process`, fetch or reparse job since and none
   queued before the restart on the new binary: first the bounded probe
   `SELECT id FROM notices WHERE parse_state = 'parsed' AND projected = 0 ORDER BY id LIMIT 1` through
   `/v1/sql` (it seeks the `notices_unprojected` partial index). No row: the boundary is
   `SELECT id FROM notices ORDER BY id DESC LIMIT 1`. A row: run `project` and read both again — the probe
   must return no row; never take a lower id instead (a boundary below a notice folded before the gate is
   refused, below). Leave `pending` and quarantined ids out (they would drag the boundary back years and
   make the post-boundary walk read most of the corpus). Keep it: every wet run, the recurrence arm and
   the restore take it, the wet run refuses to start without it, and step 1 cannot be repeated after the
   deploy — the dry plan's `notice_max_id`, like any max id read then, lies above the copies the gate has
   refused since, so it would hide them: never pass a newly read max id. From the first wet
   window on, `guid_twin_dissolve_log` records it, and every wet run, recurrence arm and restore refuses
   any other value (the recurrence arm defaults to it). It is checked from both sides: above, it may not
   exceed the dry plan's `notice_max_id`; below (review round 3), while no window has recorded one, a
   post-boundary copy whose refused sections all sit on identified rows, one on a GUID row, was folded
   BEFORE the gate (the gate would have bound it name-keyed), so the wet run refuses before any window when
   such a row is one its re-plan holds, and the restore refuses on any such GUID row — "boundary N is below
   notice H, which was folded before the gate … Pass P", P the highest such copy on any GUID row. P is the
   recovery for a lost or mistyped boundary: pass it as `gate_since_notice_id` and `since_notice_id` (the
   anchor accepts it while no window has recorded a boundary, and it clears both checks). A refresh residue
   — a copy a re-parse turned into a twin after the gate, its changed sibling re-resolved onto the
   name-keyed row and its unchanged one kept on its GUID row — is no pre-gate fold: it is never named (the
   drift read counts it `forward_drift_since_residue`) and the wet run moves it. One residue shape cannot be
   told apart — the added sibling a KEEP value, so no refused section is name-keyed: if P's `ingested_at`
   (one primary-key read of `notices` through `/v1/sql`) is after the deploy, P was folded under the gate;
   keep the boundary and rehome P's refused GUID-row mention onto the name-keyed row by a reviewed verdict
   (`apply-rehoming`), which the check then skips. At `1`, the corpus start after a from-archive rebuild
   (below), the check does not run. A wrong boundary a window DID record cannot be changed by
   any job: recovering it is a reviewed commit that rewrites `guid_twin_dissolve_log.gate_since_notice_id`;
   until then the recurrence arm and the restore read more (or less) than they should and may flag
   pre-gate noise, so a restore driven by `since` drift with no KEEP or unsafe-name add since the deploy
   must not run.
2. Deploy. Run the dry plan (`{"kind":"dissolve-guid-twins"}`) on the new binary and read it:
   `keep_source: compiled`, `keep_version` the compiled `GUID_KEEP_530_VERSION` (`keep-530-job2660`
   until a KEEP-add commit bumps it), `bars.pass`, the compiled self-check empty (else commit
   `keep_add_commit`'s lines to the KEEP file in value order, bump its `# version:` header and
   `GUID_KEEP_530_VERSION` together, gate, deploy, re-plan — and then do step 6 for that commit before
   step 3; the frozen-file test holds the job-2660 rows as a floor, so an append passes it), the
   merge-survivor and under-review tripwires 0, `bars.tripwire_no_tender` 0 (no R1 bar, but the wet
   re-plan refuses on it), `multi_row_values` (values on 2+ GUID rows: "spans >= 2 Tenders" is counted
   per value over all of its rows, so such a value in two Tenders is a KEEP-add line, never a dissolve),
   `verdict_kept` (refused mentions a rehoming verdict placed: never planned), and `wet_windows`: windows
   close at 2,000 planned mentions OR ~120,000 estimated re-read mentions (`reread_budget`). The cost
   driver is not the planned count: 542's re-derivation re-reads every mention of every re-bound row
   inside the window's `BEGIN IMMEDIATE`, so each window's `rebound_estimate` is its targets' existing
   mentions plus what they receive plus its partial rows' kept mentions (`windows[]`), and
   `heaviest_window` names the worst by it, with its keys and biggest target. A key over the budget alone
   sits alone. Plan 2660's known heavy targets: 622 Vergabekammer des Bundes (111k existing mentions),
   2217 VK Bund beim BKartA (90k), the FR `tribunal administratif` run (164k over its keys); 7615 (29.6k)
   is not the heaviest.
3. The wet run, in a queue gap after 551's flip — not on 560's drain-free mornings, and serialised with
   473's drain on the single writer — staged:
   `{"kind":"dissolve-guid-twins","dry_run":false,"max_windows":1,"gate_since_notice_id":<step 1>}`,
   read its `guid-twin-dissolve` (the window's manifest, the change rows, `reverse_drift` 0, and
   `windows[0].hold_ms` — the writer hold every verdict POST waits behind — with
   `windows[0].rebound_target_mentions`, the mentions it re-read, beside its `rebound_estimate`) before
   queuing the rest: extrapolate `hold_ms / rebound_target_mentions` to step 2's
   `heaviest_window.rebound_estimate` (622 at 111k, 2217 at 90k, the FR tribunal run at 164k), and do not
   queue the rest unless that hold is acceptable. Any window holding the writer over 30 s passes issue
   256's `PERSIST_TIMEOUT`: enqueue NOTHING while the wet run runs (the scheduler's chain included — a
   job enqueued then is held in memory only and is lost on a restart), and expect verdict POSTs to wait
   the full hold. Then `{"kind":"dissolve-guid-twins","dry_run":false,"gate_since_notice_id":<step 1>}`
   for the rest — on the SAME dry plan: the reconcile adds back every row a wet run logged above that
   plan's `log_high_water`, and the latest report's manifest is the whole campaign's. About 11k
   `organization removed` (404, no redirect), about 52 `organization added`, about 10k `tender changed`.
   Read each wet job's `r3.skipped` (0: A12's structural tripwire) and keep each report's
   `refold_tenders_this_run`. **Save every report as it lands**: the full
   `/admin/reports/guid-twin-dissolve` body of each wet job (the staged one, the rest, any restart,
   re-queue or manifest-only job) to `.scratch/tender-db/530-triage/guid_twin_dissolve_<job>.json`, and the
   campaign's dry plan (`/admin/reports/guid-twin-plan`) to `guid_twin_plan_<job>.json`, before queuing
   anything else that stores that kind. Only the current report and `/previous` are served
   (`report_history` keeps 10 versions but serves none deeper), `guid_twin_dissolve_log` is not on
   `/v1/sql`, and once step 5's dry plan replaces the stored plan no wet job can rebuild the campaign
   manifest — a later wet report (a remedy run, a refused re-attempt: its `verify` is then empty and its
   `wet_max_org_id` null) replaces the "latest report". The run refuses while a KEEP-add candidate of the
   drift read (`reverse_drift.candidates`) sits on ANY GUID row the re-plan holds (every row of its value,
   `own_rows`): commit the lines the refusal names (step 2's commit path), then step 6, a fresh dry plan,
   and the wet run. Candidates none of whose rows the re-plan holds are listed in the report and handled
   at step 5. A wet job that ends `manifest_unread` committed its windows; only the read of
   `guid_twin_dissolve_log` failed (a `window_error` job whose log read failed too reads the same way).
   Before any new dry plan, queue
   `{"kind":"dissolve-guid-twins","dry_run":false,"max_windows":0,"gate_since_notice_id":<step 1>}` on the
   same dry plan: `max_windows: 0` is MANIFEST ONLY (outcome `manifest_only`) — the re-plan, the reconcile
   and the drift read run as checks, no window is applied whatever the unread job left, and the report
   stores the campaign's `manifest` (even if a check refuses). Never re-queue the unread job's own JSON
   instead: a re-queue is a new job id and `max_windows` counts only that id's windows, so the staged
   `max_windows: 1` JSON would apply the campaign's NEXT window. If the unread job was the staged, a
   capped or a stopped one, queue the rest separately afterwards (its report then carries the campaign
   manifest too). The manifest-only job's own `refold_tenders_this_run` and window equality are empty:
   take the unread job's `refold_tenders_this_run`, `windows[]` (`hold_ms`, `mentions_moved`) and
   `reverse_drift` from that job's saved report, feed every job's `refold_tenders_this_run` into step 4's
   union, and compare the latest report's `r3.manifest_mentions_campaign` with `windows[].mentions_moved`
   summed over every wet job on the dry plan.
4. `{"kind":"project"}` (A12): the tier-5 Tenders the windows stamped stale are re-projected. It must end
   `ok` — a stopped `project` is re-run, never read as done. Then the D7 completeness pair, before the
   census, over the DISTINCT union of `refold_tenders_this_run` across every wet job's report on this dry
   plan (the staged job, the rest, any restart, a `manifest_unread` job): `project`'s
   `compare: X verified, Y corrected` reads X + Y at least the union's size (never the summed per-window
   `N stamped stale`: a Tender stamped in two windows or two jobs is folded once, the 538 runbook's
   rule), and the bounded `/v1/sql` read `SELECT COUNT(*) FROM tenders WHERE id IN (<…>) AND
   projection_epoch = 0` over the union, in IN chunks of at most 500 ids (primary-key seeks; never retry
   a 408), sums to 0. `= 0` is the stamp's own value, so 551's or 561's epoch work cannot confuse it.
   Either check failing: re-run `project` and read both again. A job killed mid-run (re-queued under its
   own id: its report shows `windows_before` > 0 with no stored report covering the earlier windows)
   stamped Tenders no report lists; `project` re-projects them, but this read does not check them.
5. First read R3's inputs from the CAMPAIGN report — the last wet job on the campaign dry plan, the saved
   `530-triage/guid_twin_dissolve_<job>.json`, never the live one (it rotates): `replan.post_state`,
   `verify.goettingen_surviving_ids`, `verify.wet_max_org_id` and `r3.manifest_mentions_campaign`; record
   `wet_max_org_id` and the job id in issue 530's Verify. Then `{"kind":"guid-census"}`, the recurrence
   arm `{"kind":"dissolve-guid-twins","recurrence":true,"since_notice_id":<step 1>}` (expect 0 candidates,
   0 reverse drift and 0 forward drift; a mention a rehoming verdict placed is neither, counted
   `forward_drift_verdict_skipped` / `reverse_drift_verdict_skipped`), then a dry plan
   (`{"kind":"dissolve-guid-twins"}`: planned rows, whole and partial, 0 — any planned mention is FORWARD
   drift, a refused mention a re-parse left on its GUID row; one a rehoming verdict placed is never
   planned, `verdict_kept`; the remedy is a wet run on that plan with the same boundary — only AFTER R3's
   inputs are read and saved), then R3 against the saved campaign report (Göttingen by ids, A10: issue
   530's Verify line 1 prints the live Landkreis Göttingen GUID row ids; they must include every id in
   `verify.goettingen_surviving_ids`, and every other printed id must be above `verify.wet_max_org_id`). After that, queue the recurrence arm
   and the dry plan BY HAND monthly (no scheduler arm queues them; issue 530 owns the item) and after any
   refold or 434 refresh that re-parses eForms notices — the recurrence arm always at step 1's boundary
   (the recorded one is its default; with none recorded and none passed it refuses once a wet run was
   attempted, rather than read the dissolve-log targets alone; after a from-archive rebuild pass `1`, see
   below). A candidate is a reviewed KEEP-add commit, completed by step 6.
6. After ANY deployed KEEP-add commit (step 2's pre-wet one included) or `GUID_TWIN_UNSAFE_NAMES` commit:
   the mentions the gate or a wet run already moved for the added values stay on identifier-NULL rows (a
   refold never moves them back), while a rebuild would bind them by identifier. Run the recurrence arm
   with `since_notice_id` = step 1's boundary and read `reverse_drift`; restore it with
   `{"kind":"dissolve-guid-twins","restore":true,"since_notice_id":<step 1>}` (report `guid-twin-restore`:
   each (value, register country) pair onto its identity's row, or one minted as a rebuild mints it; a row
   the restore emptied is deleted as the 443 sweep deletes one, `removed`, and one something still names
   is listed `emptied_kept` for `{"kind":"sweep-orphan-orgs"}`), re-run the recurrence arm, and `project`
   for any Tender the restore stamped. The add is complete only when `reverse_drift_mentions` reads 0; a
   wet run refuses while it does not. A known limit: a `process` or reparse of the TED/DÖE eForms
   sources after the deploy that reclaims an old row in place, or changes a mention's published facts,
   re-decides that mention under the gate below the boundary, where neither half of the drift read sees
   it — hold such jobs until after R3, or note them.

A wrong fold found after the wet run (a target holding another body's mentions) is fixed the way R2
fixed its W findings: commit the key's exact folded name to `GUID_TWIN_UNSAFE_NAMES`
(`extra_unsafe_names` will not do: a wet run refuses it, and the restore reads only the compiled list),
gate, deploy, then run step 6. Every twin group under that name then binds whole: the recurrence arm
lists every value of those groups as `stock` reverse drift — the dissolved value and its twin partners
alike, in every notice — and the restore moves each onto its identity's row, or re-mints one (its first
drifted mention's name); the old org ids stay 404. Never add a wrong fold's value to the KEEP file: KEEP
takes only what the recurrence arm or a dry plan's `keep_add_commit` lists (the frozen-file test
rejects any other reason), and a KEEP value beside a fresh twin partner still refuses the partner, which
stays on the wrong target. The cost, as F1 accepted: right folds under that exact name bind again too.
Check the result against the saved campaign manifest (`530-triage/guid_twin_dissolve_<job>.json`
`manifest.dissolved[]`: id, country, identifier kind, value, name, every moved (notice, section); the live
report rotates) for every value under that name: the restore moves only a logged mention that still
sits on its logged target (plus any post-boundary copy the gate bound name-keyed). A 0 from the re-run
recurrence arm does not prove the cohort came back: a logged mention whose target a later fold merged
away (p1 `repair-provisional-name-norm`, an echo or provisional merge) or that a rehoming verdict
re-placed is neither `stock` nor `since` drift — compare the manifest's (notice, section) list with
`guid-twin-restore`'s `restored[].moved`, and rehome any missing mention onto the restored row
(`restored[].target_id`) by a reviewed verdict (`apply-rehoming`), leaving a verdict-placed one where the
verdict put it unless that verdict was the wrong fold. A wrong fold narrower than a name (two bodies in
one copy under one safe name) takes the decision's Risk 2 route instead: a reviewed `apply-rehoming`
from the saved manifest's value → (notice, section) cohort. A mention a rehoming verdict placed (rehome
or keep) stays where the person put it: neither half of the drift read counts it
(`reverse_drift_verdict_skipped`, `forward_drift_verdict_skipped`), a dry plan never plans it
(`verdict_kept`), so no wet run or restore moves it back, and a window refuses if one is recorded on its
mentions after its re-plan.

After a from-archive rebuild (`project` with `rebuild: true`), `guid_twin_dissolve_log` is emptied with
the org layer (`strip_organization_indexes`: the rebuild renumbers both the logged `org_id` and
`target_id`), and the rebuild ran rule 530-T over every notice, so no pre-boundary stock remains. A1's
boundary is then the corpus start until a wet window records a new one: every recurrence run, the monthly
monitor included, passes `"since_notice_id":1` (with none passed it refuses while a `guid-twin-dissolve`
report is stored, rather than read nothing), the restore passes `1` too, and so does a wet run — step 5's
forward-drift remedy after a rebuild is `{"kind":"dissolve-guid-twins","dry_run":false,"gate_since_notice_id":1}`
on a fresh dry plan, and its first window records `1` as the boundary from then on. Step 1's lower-bound
check does not run at `1` (the rebuild folded every notice under the gate, so no copy predates it; a
refused mention left on a GUID row is a refresh residue the wet run moves). That since-walk range-scans
every notice's ORG mentions by 20,000-id windows: run it in a queue gap. The campaign's record survives
in the saved `guid-twin-dissolve` and `guid-twin-recurrence` reports (not id-addressed). The 530 planner's
pinned control ids (254/255/8378, 7615, 2294259, 22759261/62 and the rest) are pre-rebuild production ids
and void until re-pinned, and a stored pre-rebuild dry plan (and its `log_high_water`) is void too: the
wet run's re-plan and tuple reconcile refuse it — run a fresh dry plan.

### The head refresh (issue 524 unit 2b)

`refresh-org-names` wet renames served heads in place, so it follows the T4 ladder: a fresh dry
plan on the running binary, a canary run, a capped run, the full run, each read before the next.
Run it in a queue gap: never during 534 U3's refold (~7.5 h), and never between an altid /
nhs-body / r3 dry run and its wet run — their stored plans read the heads, and their wet parity
fails closed on a changed one. Prefer the full run after 519 M6's GB-NHS identifier cohort, whose
`nhs_check` hold reads the frozen ODS table.

Rule `524u2-v7` (the recency decision, issue 524 `## Unit 2b — recency decision`) replaced the
first runbook after the capped run 2517 put five heads back to a former name (reverted by 2518).
Rule `524u2-v8` (issue 524 `## Rule v8 — decided`) followed plan 2523's dated re-judge, which failed
its residue bar (9 of 30 wrong) and its released bar (2 of 10 wrong): it holds the residue
(`hold:residue`) and narrows the junk exemption.
Rule `524u2-v9` (issue 524 `## Rule v9 — decided`, panel wf_5cca11b1-622) followed plan 2526's dated
re-judge, which failed change:R1 (2 of 30 wrong), change:R2 (2 of 30) and released (5 of 27): five
pure serve holds (`review_body`, `organ`, `code_added`, `address_added`, `caps_variant`), a
narrower `digits` release (`release_withheld`), samples salted with the epoch, and the per-class
wet-run filters (`exclude_rules`, `exclude_released`). It re-judges as step 3 lists, and each failed
class has a pre-committed filter.
Rule `524u2-v10` (issue 524 `## Rule v10 — decided`) followed plan 2527's dated re-judge, which failed
change:R1 (4 of 30 wrong) and change:R2 (4 of 30): pooled over the three re-judges, a change:R1 on a
head printed within 3 years of the plan read wrong 4 times in 29, on an older head once in 61. It holds
an R1 change on a head printed within 1,095 days of the plan's `as_of` (`hold:head_current`; the junk
exemption releases it as it releases `recent` and `residue`), and **every wet run passes
`"exclude_rules":["R2"]`** (v9's pre-committed R2 fallback; the planner still elects and lists R2 for
issue 548). The plan stores its `as_of` (the planner's start) and the wet pre-pass re-elects with THAT
value, not the clock: a head printed after the plan reads `hold:head_current` and drifts, and a plan
without `as_of` is refused. Expected on plan 2527's corpus: R1 changes 25,003 → about 18,480 (74%),
about 6,520 more holds.
Rule `524u2-v11` (issue 524 `## Rule v11 — decided`, panel wf_5d3b6fa8-186) followed the capped run
2555's dated spot check, which read 1 former-name revert and 3 wrongs on the lowest ids (reverted by
job 2556). It holds an R1 change when a RIVAL cluster votes (one other than the elected and the
head's) and the elected cluster's newest dated item (`top_newest`: any spelling of the winning name,
not only the one served) is more than 365 days before the plan's `as_of`
(`hold:top_stale`; undated counts as stale; a head the window, the residue holdout or `head_current`
already released ships). It adds the serve holds `doubled` and `seat_dropped` and the bind guard
`placeholder_id`, and emits `rival_newest` per entry, which no verdict reads. It pre-builds the plain-age
rule as a wet filter, `exclude_stale_top`, which every wet run passes if the re-judge's gate (a) fails.
**Every wet run passes `"exclude_rules":["R2"],"exclude_released":["unfit"]`** (v10's re-judge failed
`unfit`), plus `"exclude_stale_top":true` if gate (a) failed. Expected on plan 2543's corpus (the
judge's python port; the build's predicates reproduce it exactly on plan 2543's 29,255 entries): R1
changes 18,489 → about 15,936 (`by_rule.R1` about 16,104 with the junk releases), holds 9,076 →
about 11,630; wet scope about 16,003 targets, about 11,389 if the fallback fires.
Rule `524u2-v12` (issue 552 `## Unit 2 — decided`, revised after wf_0744b445-0ee) followed plan
2543's re-judge, which read all five wrong `unfit` releases on heads whose only defect is a line break
(control whitespace). A control-only head is released `unfit` only when the elected key sits inside
one of its lines or equals a contiguous run of whole lines (`release_withheld: lines` otherwise, the
hold standing), and a withheld head whose every break is a wrap changes to itself collapsed (`R0w`)
when the junk exemption would not release that collapsed head (so it stays held on later plans).
**Every wet run passes `"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]`** until the v12
re-judge (step 3) passes; then the wet runs drop `"R0w"` from `exclude_rules` and drop
`exclude_released` (v11's step 6, if still owed, runs with v12's filters on a v12 plan). Expected on
the v10-like population of plan 2543 (`decide_552_v2.py` in
`.scratch/tender-db/552-control-whitespace/`; the build's predicates agree with it on all 104
control-whitespace heads): about 79 control-only `unfit` releases, about 54 passing the guard, about
21 `release_withheld.lines` and about 4 `by_rule.R0w` (25 withheld in all), ±10% for drift.
Rule `524u2-v13` (issue 548 `## U1b + U2`, `.scratch/tender-db/548-triage/u2/decision.md`) followed
unit 1's read of v12 plan 2569's (2026-10-16) held listing. The bind guards are asked with H (the head
cluster's votes, the entry's `head`): `family` and `gb_family` hold only at H ≥ 2, and
`consortium_removed` is released when the new name's words, legal forms aside, are a non-empty subset
of the head's (`crosswalk::head_consortium_subset`). Nothing else moves: the serve holds,
`head_current`, `top_stale`, `nhs_check`, `placeholder_id` and every wet filter stand (**every wet run
still passes `"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]`**), and no canary expects a
`family`, `gb_family` or `consortium_removed` hold (22831984's `placeholder_id` hold is unchanged: its
`KEINEANGABE7` identifier holds it whatever the names). Expected on that plan's listing
(`u1b/u3_predict.py`): `by_hold.gb_family` −7 (10 → 3), `family` −36 (79 → 43), `consortium_removed` −179 (246 → 67), `head_current` +64, `top_stale` +23,
holds 11,652 → 11,517; R1 changes up to +106 (5 gb_family, 22 family, 79 consortium; an upper bound,
the serve holds unmodelled) and R2 changes +29 (excluded on wet runs). 15783942 leaves both `family`
and `consortium_removed` and holds `top_stale`.
Run each plan and its wet run back to back, outside the daily ingest, so a new procedure does not
drift the targets.

**Rule v12's re-judge FAILED (2026-10-17, plan 2569; issue 552 units 4–5).** Plan 2569 (rev `3475272`) passed
step 2's v12 bullet exactly (R0w 4, R1 77 — every one an `unfit` release —, `release_withheld.lines` 21), but step
3's re-judge read 2 wrong among the 54 guard-passed control-only releases (4040 a revert to a former partnership
name; 22678568 a brand short form), over the bar of 1. So **every rule-driven wet run keeps
`"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]`** — the weekly upkeep (unit 2c) included, when it is
built — until issue 552 lands a fix for those shapes and a fresh re-judge passes. Plan 2569's class was then read
item by item, and its judged-right ids shipped by explicit `org_ids` on that plan, one job per class with
`"exclude_rules":["R2"]` (issue 552 unit 4, a one-off, decided before the blind second reading): jobs 2570 (R0w 4),
2571 (control-only releases 49) and 2572 (other `unfit` releases 22), 0 drifted, dated spot check 55 right / 1
neutral / 0 wrong. Undo per class: `{"kind":"revert-org-name-refresh","job":2570|2571|2572}`. The next v12 plan
therefore reads about 0 `by_rule.R0w`, `recent_released.unfit` about 2 (4040 and 22678568, while their holds last:
22678568's `head_current` lapses about 2027-07-10, then it is a plain R1 change no v12 filter drops — issue 552
unit 5 must land first) plus new ones, `release_withheld.lines` about 21 less any head renamed since, and holds up
by about 4 (the collapsed R0w heads, now fit, held by `recent`/`residue`/`head_current`); `by_rule.R1` about the
unfit releases left plus drift and new elections. Read step 2's v12 bullet with those numbers, not its plan-2566
arithmetic.

1. **Build and gate**, then a 3-lens review; gate with `ops/check.sh` redirected to a file and read
   `GATE-EXIT=`. Deploy in a queue gap, never during a fold.
2. **A fresh dry plan (full walk) on that binary.** `{"kind":"refresh-org-names"}` (rule v9 salts
   the samples with the epoch, so the first 30 of each class are a fresh draw: no skip list, and the
   default 30 per class serves step 3). A wet run refuses
   any plan elected under another `epoch` or by another `rev`, so every deploy needs its own plan. Go
   requires:
   - `undated 0 (MUST be 0)` on the job line (`by_hold.undated` absent in the report);
   - 0 unexpected canaries (`(0 unexpected)`), except that a `change:R1` canary (since v11: 3042,
     2857103, 2715115 and 1800186; the six v10 canaries 4220276, 4823670, 8092631, 8152890, 9766149 and
     4993534 are `keep:S1` since job 2554 renamed them) may flip to
     `hold:recent`, `hold:residue` or (v10) `hold:head_current`, and (v10) a `hold:head_current`
     canary (1220, 2762, 3406, 6079, 11179, 1919 and 791, `change:R1` before v10) may flip to
     `hold:recent` or `hold:residue` (both decide before `head_current`, and like it they write
     nothing), as its head prints new procedures — the canaries are fixed ids, the corpus is not. A
     v10 canary reading `hold:head_current` means a notice printed its head within 3 years of the
     plan's `as_of`: its entry's `head_newest` says when. Judge a flip by the plan's OWN dates, its
     `canaries[]` entry: a `head_newest` later than plans 2526 and 2527 read it (1220 2026-09-29,
     11179 2026-10-05, 791 2026-09-04, 3042 2026-10-08, and 2026-07-17 for 1919, 2762, 3406 and 6079
     — their low ids were minted from the first package ingested, OJ 2026-07-17; v11's three
     `change:R1` canaries' heads were last printed 2012–2013 — plan 2543 read 2857103 2012-11-03,
     2715115 2013-09-25 and 1800186 2012-02-18 —, so any `head_newest` within 3 years of `as_of` is
     later) means a notice since then prints the head, and with `newest_is_head` true
     (`residue`), `head_in_window` ≥ ⅔ of `head_procs` (`recent`) or `head_newest` within 1,095 days
     of `as_of` (`head_current`) the rule is working: drop it from step 4 and go. A flip the dates do
     not explain (no newer head item, or the entry undated) is a no-go. Plan 2526 dated 1220, 11179,
     1919, 2762, 3406 and 6079 (`undated_mentions` 0);
   - `truncated` `false`.
   - rule v9: each serve hold's `by_hold` within ±10% (floor ±10) of plan 2526's deterministic counts
     (the panel's ports: `review_body` ~104, `organ` 460, `code_added` 81, `address_added` ~85,
     `caps_variant` ~290), `release_withheld` near 15 and `recent_released` near 12 (11 `digits` and v8's one `unfit`). These are pure
     functions of the entries, so a larger gap is a port bug: reconcile it before judging (rule v10
     lifts both, below). The new
     canaries: `hold:review_body` 22028982, `hold:organ` 4065575, `hold:code_added` 17116440,
     `hold:address_added` 8069375, `hold:caps_variant` 16508165, `change:R1` 791 and 3042,
     `hold:residue` 4295185 and 9814 (their v8 reason; the entry reads `release_withheld`). 791
     (`head_newest` 2026-09-04 in plan 2526) and 3042 (2026-10-08) are `change:R1` canaries for the
     flip rule above (superseded by v10 for 791, now `hold:head_current`; 3042 stays `change:R1`).
   - rule v10: the job line's `Rule v10: R1's head_current hold (… as_of YYYY-MM-DD): N` near 6,520
     and `by_rule.R1` near 18,480 (plan 2527's 25,003 R1 changes less the heads printed within 3
     years), the report's `as_of`/`as_of_date` the plan's start, the seven re-pointed canaries
     `hold:head_current` with their v9 spellings (1220, 2762, 3406, 6079, 11179, 1919, 791; or a
     `recent`/`residue` flip the rule above explains), 3042
     still `change:R1` (`recent_released: digits`), and the six v10 `change:R1` canaries planned (a
     canary whose head printed since plan 2527 may read `hold:head_current`: drop it from step 4).
     `recent_released` may grow (current heads the junk exemption releases); step 3 reads it in full.
     `release_withheld` may grow too (current heads whose `digits` release v9 withholds: it counts a
     withheld release whatever the hold, `recent`, `residue` or `head_current`), so the v9 bullet's
     `near 15` / `near 12` and its port-bug reading do not apply to a v10 plan.
   - rule v11 (`## Rule v11 — decided`, protocol step 2) — THE GO for a v11 plan: `undated 0`; 0
     unexpected canaries, the flips above aside (and a v11 `change:R1` canary may flip to
     `hold:top_stale` only if its entry explains it: 2857103 or 2715115 with a third cluster voting
     now — `clusters` above 1 + (`head` > 0) —, or 1800186 with a `top_newest` more than 365 days
     before `as_of`, which plan 2543's 2026-09-22 cannot be before 2027-09; a `hold:top_stale` canary
     may flip to `hold:recent`, `hold:residue` or `hold:head_current` if its head printed again, as the
     entry's dates say); and each of these within ±10% (floor ±10) of plan 2543's expected count:
     `by_hold.top_stale` ~2,430, `by_hold.doubled` ~91, `by_hold.seat_dropped` ~67,
     `by_hold.placeholder_id` ~10, `by_hold.head_current` ~6,302, and R1 changes ~15,936 — read
     `by_rule.R1` as ~16,104, since it counts the 66 `digits` and 102 `unfit` releases too
     (`recent_released`). A larger gap is a port bug: reconcile it before judging. The job line names
     `Rule v11: R1's top_stale hold (… as_of YYYY-MM-DD, or undated): N; the placeholder_id bind guard:
     N`, and the v9 list ends `…, doubled N, seat_dropped N`. The new canaries read `hold:top_stale`
     1556789, 1276213 and 1345526, `hold:doubled` 1918631, `hold:seat_dropped` 5350675,
     `hold:placeholder_id` 22831984, `change:R1` 2857103, 2715115 and 1800186, each with plan 2543's
     spelling, and the six job-2554 renames `keep:S1`; 3042 is still `change:R1` (`recent_released:
     digits`). Every dated entry carries `rival_newest` (or null).
     A RE-PLAN ON THE SAME BUILD (step 6's abort path) finds `change:R1` canaries this protocol's own
     wet runs already renamed, and each reads `keep:S1` `UNEXPECTED (want change:R1 "…")`: 2857103
     and 2715115 after step 4 if gate (a) passed, and 3042 and 1800186 after step 5 (both are in the
     capped 500 with or without `exclude_stale_top`; 2857103 and 2715115 only if (a) passed, since
     the fallback drops them). Each such reading is expected, not a no-go, when its `canaries[]`
     entry's `old_name` (the head now) is the spelling in its `want` and
     `curl -s https://tenders.zebreus.click/v1/organizations/<id> | jq -r .name` serves it (step 4's
     report lists its ids in `applied_samples`; step 5's 30 samples may miss them, and the ledger is
     not on `/v1/sql`, so the head is the check). Any other `keep:S1` on these ids is a no-go.
     The rule v9 and v10 bullets above are kept for the record: a v11 plan's go is this bullet, the
     canary flips above and `truncated` `false`; the v10 bullet's `by_rule.R1` near 18,480 and its six
     `change:R1` canaries planned do not apply to a v11 plan (they read ~16,104 and `keep:S1`).
   - rule v12 (issue 552 `## Unit 3 — build notes`) — THE GO for a v12 plan. v11's bullet above
     stands, read on the v12 plan: `undated 0`, `truncated` `false`, 0 unexpected canaries (its flip
     rules; no canary expects `change:R1` any more), and its hold counts within ±10% (floor ±10) —
     `by_hold.top_stale`, `doubled`, `seat_dropped`, `placeholder_id` and `head_current`, which a wet
     run does not move — with the holds grown by about the 21 `lines` withholdings (under `recent`,
     `residue` or `head_current`). Its R1 figure does NOT apply (R1 changes ~15,936, `by_rule.R1`
     ~16,104, and plan 2566's 16,095): every head a wet run renamed since reads `keep:S1`. Read
     `by_rule.R1` as plan 2566's 16,095, less the renames applied since (capped job 2567's 500, and
     step 6's, about 15,494, if it has run), less about the 25 `unfit` releases the lines guard
     withholds: about 15,570 while step 6 is still owed; after step 6, about 77 (the `unfit` releases
     every v11 wet run excluded that v12 still releases) plus drift and new elections since plan 2566.
     CHANGES moves with it. A larger gap is a port bug: reconcile it before judging. v11's dated gates
     (a)–(c) passed on plan 2562 and stand for a v12 plan, not re-run (v12 changes control-only heads
     alone); only step 3's v12 re-judge gates the new classes. A v12 plan resumes at step 3's v12
     re-judge, then step 5 — a capped 500 on the v12 plan, the lowest ids again, with its dated spot
     check — and step 6. While v11's step 6 is still owed, steps 5 and 6 may run before that re-judge
     with v12's filters (`"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]`), which leave out
     every class it gates. And:
     - the job line's `Rule v12: control-only unfit releases the lines guard withheld (release_withheld
       lines, held): N` near 21 and `R0w wrap collapses …: N` near 4 (25 withheld in all; ±10%, floor
       ±10); `recent_released.unfit` near 77 (plan 2566's 102 less those 25: about 54 control-only
       releases the guard passed, the 2 control-plus-`long` heads and about 21 non-control `unfit`
       releases), which step 6 does not move (its filter excludes them);
     - the canaries: 60 rows, all ten wet-run renames `keep:S1` — job 2565's eight (8765882, 5393731,
       9856301, 7822785, 11268197, 5011106, 2857103, 2715115) and capped job 2567's 3042 and 1800186
       (2567 renamed the first 500 ready targets by org id; its applied samples reach org id 2,675,242,
       so both are among them). A confirmation, not a gate: `curl -s
       https://tenders.zebreus.click/v1/organizations/<id> | jq -r .name` serves `Hôpitaux civils de
       Colmar` (3042) and `Terveystalo Julkiset Palvelut Oy` (1800186). Any other reading of them follows
       v11's flip rules;
     - every `R0w` entry in `planned` reads `release_withheld: lines`, its `new_name` the old head with
       its whitespace collapsed (`org_name_norm` collapses whitespace, so its `new_name_norm` is what the
       old head derives). Expect 22289866,
       22661109, 22981989 and 23474998 among them, and 22737299 held with `release_withheld: lines`.
   - rule v13 (issue 548 `## U1b + U2`) — THE GO for a v13 plan. v12's bullet above stands, read on
     the v13 plan, except the three bind guards v13 changes (`gb_family`, `family`,
     `consortium_removed`) and the two holds they feed: against the last v12 plan, 2569 (2026-10-16,
     holds 11,652), `by_hold.gb_family` ~3, `family` ~43, `consortium_removed` ~67,
     `head_current` +64 and `top_stale` +23 on that plan's counts, holds ~11,517 — each move within ±10%
     (floor ±10), after the renames and drift since that plan. R1 changes grow by at most ~106. A
     larger gap is a port bug: reconcile it (`u1b/u3_predict.py` reads that plan's listing) before
     judging. 10213161 and 11628199 (gb_family, H = 1) and 8217358 (Guintoli) read `change:R1`, or a
     later hold the entry's dates explain; 13448374 and 9087207 still read `hold:gb_family`, 3728305
     `hold:family`, 18287356 `hold:consortium_removed`; 15783942 reads `hold:top_stale`. v12's resume
     sentence above does not apply to a v13 plan: v12's re-judge stays FAILED (the note above step 1),
     so v12's filters stand on every v13 wet run. A v13 plan goes from here to step 3's v13 re-judge,
     then step 5 (a capped 500 on the v13 plan, the lowest ids again, with its dated spot check) and
     step 6, both with `"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]`. Step 4 has no v13
     canary run.
   Record the walk time and the dated read counts (`dated_orgs`, `dated_notices`,
   `dated_version_rows`). Stop and re-decide if `hold:recent` is above about 4,000 (a quarter of R1
   with H ≥ 2): the window would be reading noise. `hold:recent` and `hold:residue` should read
   plan 2526's (374 and 794) plus the withheld releases.
3. **The dated re-judge** (a classifier plus a skeptic on every `wrong` or `harmful`). Each judge GETs
   the newest 3 or more procedures on each side. Lists from `GET /admin/reports/org-name-refresh-plan`
   (rule v9; the samples are salted with the epoch, so they are a fresh draw — take the first 30):
   - rule v11 (`## Rule v11 — decided`, protocol step 3) — THE GATES for a v11 plan. Every sample is
     drawn in the plan's salted order (`((org_id XOR salt) × 2654435761) mod 2³²`, then the id; salt
     `store::ORG_NAME_REFRESH_SAMPLE_SALT`, 2,780,005,964 for `524u2-v11`); skip every org already
     labelled or read by an earlier panel (every `org_id` in
     `.scratch/tender-db/524-v11/skip-ids.tsv`: 565 ids from the 588 labels of `labels524.json`, the
     2517 spot check's 30 and the 9 unlabelled ids the v11 judge record names; the rest of that
     judge's by-name reads, 22 + 24 entries, named no ids and cannot be skipped); a skeptic reads
     every wrong.
     - **(a) stale no-rival, 30**: v11 `change:R1` entries with no `recent_released`, a `top_newest`
       365 or more days before `as_of`, and no rival (`clusters` ≤ 1 + (`head` > 0)), drawn from
       `planned`. At most 1 wrong and 0 reverts. A fail is NOT a stop: every wet run below then also
       passes `"exclude_stale_top":true`.
     - **(b) change:R1, uniform 30** (`samples["change:R1"]`, then `planned` in the same order): at
       most 1 wrong. A fail stops the run for a re-decision.
     - **(c) change:R1, low-id 30**, from the lowest 2,000 in-scope targets by org id (in scope: a
       `planned` entry that is not R2 and not `recent_released: unfit`; take the cut from the new plan
       — on plan 2543's entries re-classified under v11 (the judge's port) the 2,000th was org
       id 5,100,081; plan 2543 as stored, a v10 plan, reads 4,586,162): at most 1 wrong and 0 reverts. A fail stops the run.
     - For (b) and (c) only items the wet runs will apply count: if (a) failed, skip each item
       `exclude_stale_top` would drop (an R1 with no `recent_released` whose `top_newest` is missing or
       a year or more before `as_of`) and draw on in salted order to 30.
     - **(d) a COST REPORT of 30, not a gate**: `hold:top_stale` 20, `hold:seat_dropped` 5 and
       `hold:doubled` 5 (only those whose half differs from the head), in salted order. A reason that
       reads more right than wrong goes to issue 548; nothing is released inside v11.
     The v10 and v9 bullets below are kept for the record; a v11 plan is judged by (a)–(d) alone.
   - rule v12 (issue 552 `## Unit 3 — build notes`, Go) — THE RE-JUDGE before lifting the v12
     filters, with dates, a skeptic on every wrong: every guard-passed control-only release (the
     `planned` entries with `recent_released: unfit` whose `old_name` carries a `\n`, `\r` or `\t` and
     is control-only: about 54) and every `R0w` change (`by_rule.R0w`, about 4). An R0w change is judged
     against the RAW head it replaces (the elected spelling is withheld either way): better, neutral or
     worse. Go needs 0 R0w changes worse than the raw head and at most 1 wrong among the passed
     releases. Then every wet run drops `"R0w"` from `exclude_rules` and drops
     `"exclude_released":["unfit"]` (keeping `"exclude_rules":["R2"]`). A fail keeps both filters and
     goes back to issue 552. **Plan 2569's re-judge FAILED (2 wrong of 54; see the note above step 1):
     both filters stand.** The withheld heads (`release_withheld: lines`, about 21) write nothing and
     are not re-judged: their forgone fixes are the decision's accepted cost.
   - rule v13 (issue 548 `## U1b + U2`; U3 asks for "a dated re-judge" after the re-plan) — THE
     RE-JUDGE before a v13 wet run, with dates, a skeptic on every wrong. Its class is the released
     rows: the v13 plan's `planned` entries reading `change:R1` whose org id is in plan 2569's `held`
     under `gb_family`, `family` or `consortium_removed`
     (`.scratch/tender-db/548-triage/org-name-refresh-plan-2026-10-16.json.gz`; about 106, an upper
     bound). Issue 548 U1/U1c already read them on plan 2569 (gb_family 7/7 right, the H ≤ 1 release
     ships 5; family H ≤ 1 22/22; the consortium subset's salted 30 29 right / 1 neutral / 0 wrong, 34
     of its 79 recorded), so judge only those not recorded in `548-triage/u1/judgements.json` or
     `548-triage/u1c/verdicts.json` (about 45, nearly all consortium): every one when 60 or fewer, else
     a salted 30. Go needs 0 former-name reverts and at most 1 wrong. A fail runs no wet run on the
     v13 plan and goes back to issue 548 (U2 re-decides the failing release; a build without it is a
     new epoch). The v13 releases need no filter of their own: they are plain R1 changes, and v12's
     filters, which stand, do not drop them.
   - rule v10 (`## Rule v10 — decided`): the fresh salted `change:R1` 30 (at most 1 wrong; a failure →
     stop and re-decide), and every `recent_released_ids` entry again if the list grew (at most 1
     wrong; a failure → `"exclude_released":["digits"]`). change:R2 is not re-judged: R2 is excluded
     from every wet run (`"exclude_rules":["R2"]`, issue 548 measures it). `hold:head_current` writes
     nothing and is not re-judged; its forgone right fixes are the decision's accepted cost. The v9
     bullets below are kept for the record.
   - `change:R1` 30: at most 1 wrong. A failure → stop and re-decide.
   - `change:R2` 30: at most 1 wrong. A failure → every wet run below passes `"exclude_rules":["R2"]`
     (R2 goes to issue 548; no refit inside 524 unit 2).
   - Every `recent_released_ids` entry: at most 1 wrong. A failure → every wet run below passes
     `"exclude_released":["digits"]`.
   - The new serve holds, 30 stratified — `hold:organ` 12, `hold:review_body` 6, `hold:code_added` 4,
     `hold:address_added` 4, `hold:caps_variant` 4, the first of each sample list — as a COST REPORT,
     not a gate: a hold writes nothing, and a reason reading more right than wrong goes to 548 for
     narrowing, never released inside v9.
   - Not re-judged: `hold:recent` and `hold:residue` (unchanged from v8), and the `release_withheld`
     ids (plan 2526's 15 are labelled: 5 wrong, 2 neutral, 8 right — the 8 forgone fixes are the
     decision's accepted cost, recorded, not pooled into any bar).
   - A change is *wrong* if it serves a FORMER name (every procedure since some date uses the head),
     another entity or a worse name; a hold is *harmful* if the head it keeps is junk, a stray or
     another entity, or defective.
   Only a change:R1 failure stops the run; the other two fire their filter, which every wet run below
   then carries (the job line's `Filters (rule v9): …` clause shows it was passed).
4. **The canaries.** Rules v12 and v13: there is no new wet canary step (v13 changes no canary's
   reading) — R0w and the passed releases stay
   excluded until step 3's v12 re-judge, and then apply through the capped and full runs below. The v12
   build re-pointed every canary v11's steps 4 and 5 renamed (job 2565's eight, job 2567's 3042 and
   1800186) to `keep:S1`. Since rule v11:
   `{"kind":"refresh-org-names","dry_run":false,"org_ids":[8765882,5393731,9856301,7822785,11268197,5011106,2857103,2715115],"exclude_rules":["R2"],"exclude_released":["unfit"]}`
   — six fresh-top R1 renames judged right (8765882, 5393731, 9856301, 7822785, 11268197, 5011106)
   and the two stale no-rival tops 2857103 and 2715115 — when gate (a) PASSED. When it failed, drop
   2857103 and 2715115 from `org_ids` and add `"exclude_stale_top":true`. Less any step 2 dropped.
   The six fresh-top ids are NOT canaries, so step 2's readings never read them: before enqueuing,
   find each in the step-2 plan's `planned` list (`GET /admin/reports/org-name-refresh-plan`) as
   `verdict` `change`, `reason` `R1`, with plan 2543's spelling as `new_name` — 8765882 `Kiesel
   Norge AS`, 5393731 `Nodra AB`, 9856301 `SKY & FACILITY s.r.o.`, 7822785 `Parque Científico y
   Tecnológico Cartuja, S. A.`, 11268197 `Centar za pružanje usluga u zajednici Osijek`, 5011106
   `Thage i Skåne AB` — and drop from `org_ids` any that is in `held`, absent or another spelling.
   Expect every one `RENAMED` and 0 drifted (the checks below). At the first build after they are
   applied, add each fresh-top id this step renamed to `ORG_NAME_REFRESH_CANARIES` as `keep:S1`,
   re-point 2857103 and 2715115 (if renamed) to `keep:S1`, and 3042 and 1800186 likewise once step 5
   renamed them (until then a same-build re-plan reads them as step 2's v11 bullet says). (Since rule v10 the run was
   `{"kind":"refresh-org-names","dry_run":false,"org_ids":[4220276,4823670,8092631,8152890,9766149,4993534],"exclude_rules":["R2"]}`,
   applied as job 2554: those six are `keep:S1` canaries now. Before v10 it was
   `[791,1220,1919,2762,3042,3406,6079,11179]`; seven of those are `hold:head_current` now, so not
   planned, and 3042 is a `digits` release, dropped by `exclude_released: ["digits"]` if step 3 fires
   it.) Expect `RENAMED n head(s)`, n the number of ids
   asked less any the filters dropped (every one is a planned change),
   with `0 drifted`, `0 gone` and no `asked org id(s) the plan does not plan` clause (the report's
   `unplanned` 0);
   `GET /admin/reports/org-name-refresh-applied` (its `job` this job, `window_error` null) lists all
   n in `applied_samples`. GET each one:
   `curl -s https://tenders.zebreus.click/v1/organizations/8765882 | jq -r .name` — `Kiesel Norge AS` (plan
   2543's spelling; the plan's entry is the authority).
5. **Capped.** On a v12 or v13 build and plan, until the v12 re-judge (step 3) passes (on a v13 plan,
   only after step 3's v13 re-judge passed):
   `{"kind":"refresh-org-names","dry_run":false,"max_groups":500,"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]}`;
   only after it passes, drop `"R0w"` and `exclude_released`:
   `{"kind":"refresh-org-names","dry_run":false,"max_groups":500,"exclude_rules":["R2"]}`. On the v11 build
   (its enqueue refuses `R0w`) it was `{"kind":"refresh-org-names","dry_run":false,"max_groups":500,"exclude_rules":["R2"],"exclude_released":["unfit"]}`,
   job 2567. Rule v11: plus `"exclude_stale_top":true` if gate (a) failed. The lowest ids again, the population
   2517 and 2555 failed on (v11's next capped 500 reached about org id 2.95 M on plan 2543's entries
   re-classified under v11, about 3.36 M with `exclude_stale_top`; the new plan decides). Check the report's `window_error` is null, `stopped` false,
   `prepass_stopped_after` null and `left` 0 (a pre-pass stop reads `left` 0 too, as it counts
   nothing), then spot-check its 30 `applied_samples` WITH DATES (`GET /v1/organizations/<id>`
   and the newest procedures on each side; rule v11: read the head's and the elected spelling's dates).
   Go requires 0 former-name reverts and at most 1 other wrong. A fail reverts the job at once (as 2518 and
   2556) and stops.
6. **Full.** On a v12 or v13 build and plan, until the v12 re-judge passes (on a v13 plan, only after
   step 3's v13 re-judge passed):
   `{"kind":"refresh-org-names","dry_run":false,"exclude_rules":["R2","R0w"],"exclude_released":["unfit"]}`;
   only after it passes: `{"kind":"refresh-org-names","dry_run":false,"exclude_rules":["R2"]}`. On the v11 build
   (which refuses `R0w`): `{"kind":"refresh-org-names","dry_run":false,"exclude_rules":["R2"],"exclude_released":["unfit"]}`.
   Rule v11: plus `"exclude_stale_top":true` if gate (a) failed. Its 30 `applied_samples` get step 5's
   dated check; a fail reverts the job. An `ABORTED before any write by this
   run` job means more than max(2%, 5) of the plan drifted since it was elected (a fold or a merge
   moved heads, new mentions changed an election, or a head named newer procedures and now reads
   `hold:recent`, or — rule v8, the likelier — one new procedure printing the head made it the
   newest item and it reads `hold:residue`, or — rule v10 — a notice printed the head after the plan:
   the pre-pass re-elects with the plan's `as_of`, so it reads `hold:head_current`, or — rule v11 — a
   new rival cluster reads `hold:top_stale`, or with `exclude_stale_top` a re-election on a stale top
   drifts): re-run step 2 and go again (on the same build the `change:R1` canaries steps 4 and 5
   renamed read `keep:S1`: step 2's v11 bullet says when that is expected).
   - A job RESTARTED by a crash resumes after its last committed window: its resume cursor commits
     with each window, its skipped targets count as `Resumed after org id …`, and its `max_groups`
     counts the renames it made before (`… rename(s) by an earlier attempt of this job stand`). A
     restart onto a NEW deploy is refused instead (the plan's `rev` is the old binary's; the line
     says the earlier windows stand): re-run step 2, then re-enqueue.
   - A STOPPED or FAILED job does not resume. Re-run it as a new job: the committed windows read
     `already`, its `max_groups` starts again from zero, and its renames revert under its own job
     id (undo then means reverting both job ids, newest first). A job that failed in a window after
     one of its windows committed stores its report first (`window_error`, `left`), and its FAILED
     line names the window, the renames that STAND, `left` and `capped`. A job with no committed
     window fails with the bare error and stores no report. A job refused or failing before its
     report (another `rev`, a failed pre-pass) while it has renames names how many and the revert.
   - A job STOPPED during its pre-pass (`STOPPED by cancel during the pre-pass after N of M
     target(s); nothing written by this run`) wrote nothing: its report's `prepass_stopped_after` is
     N and its pre-pass counts, `left` included, read 0 — they were never counted. A recovered job's
     line still names the renames its earlier windows made and their revert. Re-run it as a new job.
   - `GET /admin/reports/org-name-refresh-applied` is the latest run's: check its `job` field.
   - E2 (a `wrong` or unapplied `related` identifier verdict) is read at the pre-pass only: post no
     identifier verdicts (`POST /admin/identifier-verdicts`) while a wet run is in flight.
7. **The 2b tail.** Re-run the dry plan: expect `by_rule.R2` about step 2's (rule v10: R2 is
   excluded from every wet run, and the planner still elects and lists it) and the other rules about
   0 planned, and `hold:recent` about the same as step 2. Re-run `org-head-drift`: `minority` and
   `single_stray` fall by about the applied count,
   and the negative canaries are unchanged. Run `build-org-match-keys` wet
   (`{"kind":"build-org-match-keys","dry_run":false}`) outside a fold window, or accept up to a week
   of stale wall counts until the weekly build. Re-plan any pending altid / nhs-body / r3 dry plans
   before their wet runs: their parity fails closed on the changed heads.

Unit 2c (the weekly auto-run) inherits the dated protocol; its 2,000 cap counts changes only, and
(rule v10) it carries `"exclude_rules":["R2"]` until issue 548 refits R2.

**Undo.** `{"kind":"revert-org-name-refresh","job":<the wet job's id>}` restores that job's renames
(read its job line: `RESTORED`, `renamed again`, `identifier stripped`, `gone`). There is no dry
revert: `dry_run: true` is refused; preview with the job's `org-name-refresh-applied` report
(`applied` plus `prior_applied`). Nor is there a per-org or capped one: `org_ids`, `max_groups`,
`min_mentions`, `samples`, `exclude_rules`, `exclude_released` and `exclude_stale_top` are refused. A head that still holds the job's pair but whose
identifier a case review stripped since the rename is skipped and left unreverted: its
`name_norm` is an issue-234 bind key now (restore the identifier, and a re-run takes it up). One
renamed again since counts `renamed again`, stripped or not.
Revert several jobs newest first: the restore's guard is the pair the job wrote, so it cannot tell
an A→B→A→B history apart, and reverting the older of two jobs that each wrote B restores A over
the newer one's B (the served name is still a pre-image; the newer job's ledger row is left
unreverted). A from-archive rebuild re-mints first-seen heads under new ids, after which the
ledger is history only and the heads need a fresh dry plan and full run.

### The nhs-body campaign (issue 513 U5)

The order is fixed by the decision's stop rules; each step reads before the next one writes.

1. **Dry.** `{"kind":"match-org-identifiers","rule":"nhs-body"}`. Read `GET
   /admin/reports/nhs-body-plan`: `admitted_codes` is the set the wet run will fold (156
   after U3); `verdict_status` names every body that is not admitted and why;
   `nhs_body_regrown` must be `[]` (S4) and `ledger_rows` 0 before the first wet run.
2. **Capped wet (S3's 20).** `{"kind":"match-org-identifiers","rule":"nhs-body",
   "dry_run":false,"max_groups":20}`. A parity abort (`nhs-body parity abort: … nothing was
   written`) means the census moved since the dry run: re-run step 1 and read it again.
3. **Read (S3).** The stored plan is now the residual, and its `codes[]` is the census from
   BEFORE the folds (each folded body's entry says `"folded": true`): read holders and
   `verdict.status` from a FRESH dry run (step 1's request), not from the stored wet plan.
   Per `folded[]` entry and its `nhs_body_merged` rows: the survivor is the reviewed one (the
   PPON org, or the anchor for a body without one); no non-member holder was touched (the
   fresh dry run's holders of the code are the survivor plus the non-members, each with its
   old mentions); the survivor's mentions equal its members' sum (plus arrivals since); each
   loser id answers 308 (`/v1/organizations/<loser>`); `?identifier=GBNHS<code>` answers the
   survivor, for all 20 codes. `held[]` bodies (`survivor-ambiguous`, `loser-ppon-unaliased`)
   stay in the residual: post a corrected verdict, never force them. The next fold's
   `[issue 513]` diag lines: refusals ≤ binds (S4). Then S5's canary: pick 3 bodies and per
   body 3 notices (a TED uncoded mention, an FTS coded party, a PPON-first party) and run
   `{"kind":"match-org-identifiers","rule":"nhs-body-canary","notices":[…9 ids]}`; the job
   line must say `AGREES` (`nhs-body-canary`: `disagree` 0), or halt.
4. **Full wet.** The same request without `max_groups`: it holds itself to the residual, so
   the 20 are not counted as drift. Then a dry run: `admitted_codes` `[]`, `nhs_body_regrown`
   with no `name`/`ppon`/`identity` entry (`[]` for 513; since issue 519 U4 an `identity-stray` is
   listed, never a halt), every folded code's `verdict.status` `applied`.
5. **Unwind (D7), only for a defect found by a read.** `{"kind":"match-org-identifiers",
   "rule":"nhs-body-unwind","code":"<CODE>"}` (dry: read `nhs-body-unwind`'s `survivor`,
   `moved`, `targets`; a refusal names the ledger row whose survivor it could not confirm),
   then the same with `"dry_run":false` (it retires the ledger rows in its own transaction,
   so the alias stops arming the body), then a `project` so the re-queued notices' tenders
   are re-derived. No `keep` verdict is needed, and none should be posted: a standing keep
   outranks every merge verdict for the code, so the body could not be folded again while
   it stands. Never roll the binary back past U4 without unwinding first (S6).

### Child codes in the nhs-body census (issue 519 unit 2)

A sub-ICB location (`13T`, `B2M3M`) or a measured trust site (`RBT20`) carries a digit, so its
GB-NHS code mints an identity org of its own beside its body. Since issue 519 unit 2 the same
dry request (`{"kind":"match-org-identifiers","rule":"nhs-body"}`) lists those orgs as holders
of their parent code, through the frozen dated child map `crates/ingest/sdk/nhs-ods-children.tsv`
(`ods_children_version`; `nhs-ods.tsv` and `ods_version` do not move with it). The job line ends
with a `children (issue 519, map …)` clause. Read in `nhs-body-plan`:

- per code, `children[]`: each child dated under it with a holder (its `periods_here`,
  `other_parents`, `holder_ids`, `coded_parties`). A child is listed under EVERY dated parent;
  the name chooses, never the date.
- per holder, `child_of`, `child` (it holds the code only through a child, so 519's gates
  apply), `child_shape` (`identity` / `zero-only`), `child_names[]` (every distinct mention name,
  `check_here` = the strict gate under the code, the dated counts, the answer under each other
  parent) and `child_head_check` (an identity holder's head gates only as one of its mention
  names; a zero-only cluster's head gates). `via` gains `child`.
- the child denials (`child-triple-shape`, `child-carries-ppon`, `child-withheld`,
  `child-impure`, `child-names-unread`, `child-name-refused`, `child-name-mixed`) in `denied`
  and `child_denied`; a verdict naming an identity child into a body with no PPON survivor reads
  `child-into-anchor`.
- report-wide: `child_codes_seen`, `child_holders`, `child_mentions`, `child_unanchored` (a
  child tail whose dated parents are all off the allowlist — U6), `child_no_parent`, and
  `keep_over_applied`.

S0 before a child cohort is posted (U3): `keep_over_applied` `[]` (hold a holder by leaving it
out of `members`, never by `keep`; a keep re-posted over the applied cohort itself counts, since a
re-record never clears the applied stamp); E3/E4 denials 0 (the measured value) or explained; the
zero-only holders are M5's list; `reviewed_holders` is the WIDENED live set, rehome targets
included (a rehomed stray's target becomes a child holder of the code it was published under).
513's applied verdicts read `applied` whatever the widening.

**Since issue 519 U4** the wet run folds a child cohort (decision §3.2-§3.7; U2's
`child-fold-unbuilt` hold is gone): an identity child as an identity loser (its `GBNHS<C>` a merged
identifier of the survivor; ledger `loser_kind: identity`, empty name keys), a zero-only cluster as
a name loser with `GBNHS<00x>` recorded; a body whose survivor would be a provisional anchor is
held `child-into-anchor`. The run order from decision §5 step 6:

6. **Capped wet run (5 bodies)** — `{"kind":"match-org-identifiers","rule":"nhs-body","dry_run":false,
   "max_groups":5}`. `max_groups` has no code filter: it folds the first five admitted codes in code
   order (on the predicted census QGH, QHL, QHM, QJK, QMJ), so read S3 on the job's `folded[]`, not on
   a chosen five — then S3's read (`s3check_children.py`): each loser answers 308;
   `?identifier=GBNHS<C>` answers exactly the survivor and `GBNHS<P>` still does; the 513-cohort ledger
   rows are unchanged; non-members untouched. Read the next fold's `[issue 519]` closing line: its
   `refused` counts (name, ambiguous, withheld, shared) — the line carries counts, not the names;
   each refused mention stands on its own `GBNHS<C>` row, which the next dry run's S4 lists.
7. **Canary, then the dry scoped unwind.** `nhs-body-canary` over 3 child-coded notices per body
   (oldest, middle, newest) plus a QWU regression trio: `AGREES` and `triple_bound` > 0. Then
   `{"kind":"match-org-identifiers","rule":"nhs-body-unwind","code":"<a code in step 6's folded[]>",
   "cohort":"<child cohort>"}` (dry; QHM if it folded — it has 513 rows and both identity and zero-only
   losers): only child moves, `code_kept: true`. A code the capped run has not reached is refused
   (`recorded but not folded yet`), and so is an unwind named by a CHILD code (`… is a child code folded
   into body <P>`: unwind the parent's cohort). No rollback past U4 without it (S6).
8. **Full wet run**, then a closing dry run: `admitted_codes` `[]`, `nhs_body_regrown` with no
   `name`, `ppon` or `identity` entry (`identity-stray` entries are listed, never a halt; the job line
   counts them apart from `keys regrown (S4)`).

### The admin surface beyond jobs (issues 230, 335, 348, 356)

Every route takes the operator secret (`x-admin-secret`); `tender-admin raw <METHOD> <path>`
sends it and reads a body from stdin.

| route | what |
|---|---|
| `GET /admin/jobs?limit=` | the queue, the running job's progress, recent runs (`limit` deepens the log, issue 313) |
| `POST /admin/jobs`, `DELETE /admin/jobs/{id}`, `POST /admin/jobs/{id}/cancel` | enqueue; cancel a queued or checkpointed job (issues 250, 252) |
| `GET /admin/reports/{kind}`, `…/previous` | the newest stored report of a kind, and the one before it (issue 335) |
| `GET /admin/name-key?name=` | the genericness wall, probed for one name (issue 348) |
| `GET /admin/unmapped-fields?profile=` | what one profile publishes at its own head that the projection does not read on that table's channel (issue 368); `window` ids back from its newest notice, `show` rows |
| `POST /admin/case-reviews`, `/admin/rehoming`, `/admin/name-verdicts`, `/admin/country-verdicts`, `/admin/merge-verdicts` | record one cohort's verdicts into the five review stores (`org_case_reviews`, `org_mention_rehoming`, `org_name_verdicts`, `org_country_verdicts`, `org_merge_verdicts`); recording only — the apply jobs (or the R2 arm, for merge verdicts) execute the HIGH subset. Issue 513 U2: a merge verdict under `GB:nhs-body` (key: the three-character ODS code, country `GB`) may name one member and carries `reviewed_holders`, required for a `merge` and refused on every other scheme; the `nhs-body` census reads it |
| `POST /admin/identifier-verdicts` | record one cohort's per-identifier verdicts into `org_identifier_verdicts` (issue 452): `{org_id, identifier, verdict: wrong\|related\|right, correct_identifier?, rationale, confidence}`. Recording is the action: from the next planner run or fold, a `wrong` number — and a `related` one until the re-key arm has acted on it (`applied_at`; issue 466: a related number is a real company's, and grouping by it folds the org into that company) — is left out of the R2/E0/R3 groups, the altid arm's owners, and the resolver's canonical bind: the number's key binds another spelling only to the owner whose names the mention's names match, a mint joins those owners instead of claiming the key, and the exact triple still binds, and `/v1/organizations` serves `identifier_status` (`register_mismatch` for `wrong`, `related_entity` for `related` until the re-key applies it). A verdict is keyed by the identity triple (identifier, kind, country) the named org carries now, so a rebuild's renumbering keeps it; one naming an org that no longer carries the number is skipped (`stale` in the answer). Recording never applies `correct_identifier`: the `rule: rekey` arm of `match-org-identifiers` does, for HIGH `wrong` verdicts and (issue 466) HIGH unapplied `related` ones, and stamps `applied_at` / `applied_literal` / `job_id` on the verdict (a re-POST keeps them). The readback (`/admin/case-reviews?table=identifier`) serves those three columns |
| `GET /admin/case-reviews?table=case\|rehoming\|name\|country\|merge\|identifier&cohort=&limit=` | read a verdict store back, newest first, bounded (issue 356; the stores are not on the `/v1/sql` allow-list) |

Two refusals an operator will meet, both deliberate:

- **`match-org-identifiers` r3, wet, refuses** when `org_match_keys` is empty
  or a build is mid-walk. R3's corroboration consults the generic-name wall
  (issue 316), and an unbuilt satellite makes every key look unique — the
  wall would silently not be there. The remedy is a **wet** build:
  `{"kind":"build-org-match-keys","dry_run":false}`. The default is dry, and
  a dry build stores nothing, so the flag is the whole remedy.
- **`scan-org-match-keys` refuses** on a missing covering index, a keys-epoch
  mismatch after a deploy, or a missing build report. Each refusal writes
  `org-edge-scan-alarm`; read it with
  `GET /admin/reports/org-edge-scan-alarm`.

### Restarts and recovery (no manual re-enqueue)

The job queue is **durable** (ADR-0007): every outstanding job is a row in
`job_queue`, written before it enters the in-memory queue and deleted only when
the job concludes or is cancelled. So a deploy, crash, or restart **does not lose
the queue** — the Supervisor rebuilds it from the durable rows at startup, before
the worker or scheduler run, and the job that was mid-run when the process died
comes back at the front and re-runs. **You never re-enqueue by hand after a
restart.** Re-runs are safe because ingestion is idempotent (notice identity is
`(source, publication_id, content_hash)`; the projection is a pure function of
the parsed layer), so a repeated walk inserts nothing new.

A long `process` job also carries a **resume cursor** (issue 32): the last
package it fully committed, advanced only after every member of that package
landed. On restart it resumes at the *next* package instead of re-walking years
of archive from the start — the interrupted (partial) package re-runs and dedups.
A *freshly enqueued* `process` never inherits a cursor, so a deliberate whole-
source reprocess still walks everything. The only operator-visible trace is a log
line: `job N resumes after <period> (K package(s) already done)`.

The daily scheduler is a plain in-process timer, so a tick that passed while the
process was **down** never fired — there is no queued job to recover. The next
start serves it instead: `catch_up_missed_tick` (issue 245) runs today's daily at
once when today's 09:35 Berlin tick passed with no successful `probe` after it and
none queued (see [Ingestion](#ingestion)). A day the box was down for entirely is
not re-run; re-drive that one via `/admin` if needed.

### Quarantine triage

A notice whose file matches no mapping profile (or fails a completeness check)
is **quarantined** rather than dropped (ADR-0004): the raw payload and a reason
are kept, and the rest of the package still ingests. Quarantine is the parser's
backlog, not data loss — the archive is intact, so a fixed parser recovers every
quarantined notice by reprocessing.

Triage loop:

1. **See it.** The dashboard (`/`, public) Data-quality panel shows the
   quarantine total, a breakdown by reason, and a recent sample (50). The same
   numbers ride each `process` job's `counts` line in `GET /admin/jobs` →
   `recent[]` (e.g. `… 3702 parsed, 15 quarantined …`).
2. **Diagnose.** Read the reason and the sampled payloads to find the unmapped
   element / customization ID / era the profile doesn't yet handle.
3. **Fix the parser** in `crates/ingest` (a new profile mapping, an ignore rule,
   or an inventory extension), with a fixture test, and **deploy** it
   (`./deploy.sh`).
4. **Reprocess.** Enqueue a `process` for the affected source (no `period` =
   the whole source) then a `project`. Processing re-parses from the archive and
   never re-downloads, so this is cheap and idempotent; quarantined notices that
   the new parser understands become canonical, and the quarantine count drops.

```sh
/root/aj.sh /admin/jobs '{"kind":"process","source":"ted"}'
/root/aj.sh /admin/jobs '{"kind":"project"}'
```

## Logs

```sh
journalctl -u tender-db -f              # follow
journalctl -u tender-db -n 200          # recent
journalctl -u tender-db --since '1 hour ago'
journalctl -u tender-db -p err          # errors only
```

nginx: `/var/log/nginx/access.log`, `/var/log/nginx/error.log`.

## Monitoring and alerting

The goal (issue 24) is that the operator learns when production breaks without
watching dashboards. Two health endpoints, and one external watcher that must
live **off the box** (so it still fires when the app — or the whole VPS — is
down). The watcher today is a Claude scheduled routine polling every ~4 h — see
[The off-box watcher](#the-off-box-watcher-an-external-claude-scheduled-routine).

### The two health endpoints

| Endpoint | Cost | Answers | Used by |
| --- | --- | --- | --- |
| `GET /health` | no DB access, always fast | process is up + serving HTTP — liveness only, does **not** query the DB (`{"ok":true,…}`) | `deploy.sh`'s post-deploy check |
| `GET /health/deep` | the newest job-log row + the ingest clock (also the DB-answer read) + one `statvfs` | liveness **plus** a real DB-answer check, ingest freshness, last-job outcome and disk usage | the external watcher routine |

(A third endpoint, [`GET /metrics`](#get-metrics--the-prometheus-scrape-issue-53),
serves the same signals as *time series* for trend-watching rather than as a
pass/fail verdict.)

`/health` is deliberately narrow: its `ok` reflects only liveness, so a deploy
is never failed by a stale-ingest or full-disk condition unrelated to the new
build. **Do not widen it** — `deploy.sh` greps its `ok:true`.

`/health/deep` (`crates/app/src/v1/health.rs`) folds four signals into one
verdict, so a single external check covers uptime, freshness, job failures and
disk:

- **database** — a real reader-pool read: the job-log reads below serve over WAL
  (issue 20), so their success is the "the database answered" signal and an error
  flips this check unhealthy — and leaves `ingest_freshness` and `last_job`
  unmeasured rather than judged on half a reading. `/health` itself is
  liveness-only and does not touch the DB (issue 61/213).
- **ingest_freshness** — unhealthy when no `probe` or `process` has succeeded in
  **26 h** (`INGEST_STALE_SECS`); maintenance kinds (`project`, `refold`, `reindex`,
  …) never reset the clock. The scheduler lands a successful ingest at least daily
  (TED Mon–Fri, DÖE + FTS every day), and 26 h carries a Friday success across the
  weekend. The clock is read from the whole `job_log`, not a newest-100 window
  (issue 461: a busy day of other kinds used to push the last ingest out of that
  window, and "not in the window" read as fresh). Only a box whose log is *empty*
  (fresh deploy, scheduler not yet fired) is reported healthy-but-unmeasured; a
  log that holds runs but no ok ingest is stale.
- **last_job** — unhealthy when the newest finished run in `job_log` has
  `outcome = "error"` (a Supervisor job ERRORED). Clears itself on the next
  success.
- **disk** — unhealthy once **90 %** (`DISK_FULL_FRACTION`) of the volume
  holding `TENDER_DB` (`/data`, `/dev/md3` on local NVMe — same filesystem as
  the archive) is in use; `total_bytes` is its size (1,780,595,036,160 on
  2026-10-01). The check also reports `wal_bytes`, the size of the
  `-wal` sidecar (issue 42): **informational only** — a large WAL is expected
  mid-backfill and never flips the verdict — but the alerting routine watches it
  for a runaway (see [Disk watch](#disk-watch)).

It answers **200** when every check passes and **503** when any fails, and the
JSON body names which check tripped:

```sh
curl -s https://tenders.zebreus.click/health/deep | jq
```

```json
{
  "ok": true,
  "rev": "…",
  "checks": {
    "database":         { "ok": true, "cursor": "12345" },
    "ingest_freshness": { "ok": true, "last_success_at": 1753000000, "age_secs": 3600, "threshold_secs": 93600 },
    "last_job":         { "ok": true, "kind": "project", "params": "rebuild=false", "outcome": "ok", "finished_at": 1753000000 },
    "disk":             { "ok": true, "used_fraction": 0.041, "free_bytes": 479000000000, "total_bytes": 500000000000, "wal_bytes": 3500000000, "threshold_fraction": 0.9 }
  }
}
```

### `GET /metrics` — the Prometheus scrape (issue 53)

A third endpoint, and the one for *trends* rather than verdicts:
`/metrics` (`crates/app/src/v1/metrics.rs`) serves the operational levels in
Prometheus text format. It exists because the numbers we hand-watch — WAL size,
RSS, per-job duration, quarantine counts — are all "watch this move over time"
questions, and eyeballing a dashboard answers them hours late. The 2026-07-22/23
WAL incident is the case in point.

```sh
curl -s https://tenders.zebreus.click/metrics | head -20
```

**It is a scrape, not a measurement.** Every gauge is either O(1) (change
cursor, RSS from `/proc/self/status`, live SSE streams, one `statvfs`, the
newest-100 job-log window) or a read of the **dashboard's
60-second cache** (canonical row counts, quarantine totals and per-reason
breakdown, import lag). No table-proportional scan runs on this path — that rule
is what keeps a 15-second scrape interval from becoming the load it exists to
observe, and it is the same discipline `coverage.rs` applies to its own heavy
sections. The one exception is the freshness clock, a `max()` over `job_log`
(issue 461): that table has only its primary key, but it holds a few thousand
rows and grows by tens a day, and the bounded window lost the clock on busy days.

**A gauge nobody has measured yet is ABSENT, never zero.** On a fresh box, or
while the dashboard's heavy sections are still gated behind a running write job,
the corresponding series simply do not appear. A scraper sees them start when
the first real measurement lands. This is deliberate: a fabricated `0` for
`quarantine_outstanding` reads exactly like a drained backlog.

Series exposed (all gauges — a restart cannot reset a counter mid-series
because nothing here accumulates in-process):

| Prefix | Source |
| --- | --- |
| `tender_db_change_cursor`, `tender_db_sse_streams`, `tender_db_rss_bytes` | in-process, O(1) |
| `tender_db_disk_{used_fraction,free_bytes,total_bytes}`, `tender_db_wal_bytes` | one `statvfs` + the `-wal` stat (same as `/health/deep`) |
| `tender_db_job_last_{duration_seconds,finished_timestamp_seconds,ok}{kind=…}` | newest run per kind in the bounded job-log window |
| `tender_db_sql_recent_requests{outcome=…}`, `tender_db_sql_recent_ok_seconds{quantile=…}`, `tender_db_sql_recent_window_seconds` | issue 494: an in-process window of the last 1,000 `/v1/sql` requests. It holds outcome counts (ok, bad_request, timeout, rate_limited, busy, error), nearest-rank 0.5/0.95/0.99/1 latency of its 200s, and the wall time it spans. It is a level of the window, not a counter: a restart empties it and the series are absent until a request finishes. Every request at 1 s or more, and every 408, also logs `[sql] slow: …` with the start of its SQL (`journalctl -u tender-db \| grep '\[sql\] slow'`). |
| `tender_db_ingest_last_success_timestamp_seconds`, `tender_db_ingest_{fetch,notice}_age_seconds` | the freshness clock (`probe`/`process` only, from the whole job log) + import lag |
| `tender_db_canonical_rows{table=…}`, `tender_db_quarantine_*` | dashboard cache (absent until measured) |
| `tender_db_legacy_adjacency_watermark` | one-row point read on the reader pool |

One gauge is deliberately exempt from the absence rule above:
`tender_db_legacy_adjacency_watermark` reports **0** rather than vanishing,
because 0 is a real state there — "legacy OJS adjacency coverage was never
established", in which every legacy delta takes the full-projection fallback
instead of the scoped closure walk (issue 58 v2). It is also the *only* external
view of that claim: the adjacency tables are projection machinery and are not in
`/v1/sql`'s public allow-list, so a value > 0 here is how an operator confirms
the incremental projection may scope a legacy fold at all. Read it after any
backfill of that layer, and after a full rebuild (which resets it).

**A Prometheus + Grafana server stays deliberately deferred** (team-lead
decision on issue 53): that is real operational weight — extra processes to run,
secure and resource-budget on the one box — against a single-process monolith
(ADR-0005). This endpoint is what makes standing one up a later, reversible
choice; until then the scrape is readable by hand or by any transient scraper.

Like the health probes, `/metrics` sits **outside the rate limiter** (a scraper
on a fixed cadence must not spend the public request budget) and, unlike them, is
**not** in the public CORS grant — it is an operator surface, so browser
JavaScript on another origin cannot read it.

### The off-box watcher (an external Claude scheduled routine)

The watcher must live **off the box** so it still fires when the app — or the
whole VPS — is down, and it must need no third-party account. The chosen shape is
an **external Claude scheduled routine** that polls `https://tenders.zebreus.click/health/deep`
**every 4 h** and alerts on any non-200 (or no response). Because `/health/deep`
folds uptime, ingest freshness, last-job outcome and disk into one 503-or-200
verdict, that single GET covers everything; the routine reads the JSON body to
name which check tripped, and can watch `disk.wal_bytes` for a mid-backfill WAL
runaway. This replaces the earlier plan of a hosted uptime monitor
(UptimeRobot/Better Stack) or a GitHub Actions cron — both were rejected together
with the storage box and a public GitHub repo (no external resources, 2026-07-21).

The 4 h cadence is a deliberate trade: cheap and account-free, at the cost of up
to ~4 h to notice an outage rather than the "within minutes" issue 24 first
aimed at. Acceptable for a single-operator, rebuildable dataset; tighten the
interval if that ever stops being true. The routine is configured outside this
repo, so nothing in the deploy ships it — it is an operator-owned schedule.

**As it stands (2026-09-29, issue 24).** Two routines run hourly at `:50` and
`:20` (User-Agents `tender-db-uptime-routine/1` and `…/1-twin`). A routine whose
cloud container never starts sends nothing, and that happened on 4 of 6 runs on
2026-09-29. So a **second, independent watcher** is set up in this repository,
`.github/workflows/uptime-check.yml` — the GitHub Actions option above. **Its predecessor
`uptime.yml` never fired on its schedule** (about 32 slots on 2026-09-29; the manual drill
worked), so on 20:50 UTC the check was re-created under the new name by the owner through the
API (issue 24). Until `event=schedule` runs appear it watches nothing. It became
possible once the repository was published to GitHub (public), and it needs no
account or secret. It curls `/health/deep` at :09/:24/:39/:54 with about a minute
of retries (User-Agent `tender-db-uptime-gha/1`). On DOWN it opens one
`uptime`-labelled issue that mentions the owner, and it closes that issue on
recovery. A manual run with `drill: true` exercises the issue path (a
"UPTIME DRILL" issue, no mention, closed by the same run). To remove it, delete
the file. Change its cron through GitHub, in the web editor or the API as the owner, and
not with a session push. GitHub attributes a schedule to whoever last changed the cron
line, and while the session's push identity held it, no scheduled run fired in 16 slots
(issue 24).

### Test procedure

- **Freshness / job / disk logic** is unit-tested against crafted signals
  (`crates/app/src/v1/health.rs` `#[cfg(test)]`) and end-to-end
  (`crates/app/tests/api.rs::the_deep_health_probe_reports_operational_health`:
  a fresh box is 200, a recorded `error` run flips it to 503;
  `a_window_full_of_other_runs_does_not_hide_a_stale_ingest`: a 27 h old `probe`
  under more newer ok runs of another kind than `/metrics`' window holds is still
  the clock, and still 503; a log with runs but no ok ingest is 503 too).
- **The live alert path** (the acceptance drill, run *after* a deploy, in a
  quiet window with no backfill in flight): `systemctl stop tender-db` on the
  box, confirm the watcher routine alerts on its next poll (within ~4 h), then
  `systemctl start tender-db`. To exercise the freshness signal without waiting
  26 h, temporarily lower `INGEST_STALE_SECS`, deploy, and confirm
  `/health/deep` reports `ingest_freshness.ok = false` — then revert.

## TLS and nginx

- vhost: `/etc/nginx/sites-available/tenders.zebreus.click` (symlinked into
  `sites-enabled`; the stock `default` site is removed).
- cert: `/etc/letsencrypt/live/tenders.zebreus.click/{fullchain,privkey}.pem`,
  issued by `certbot --nginx`. Renewal is automatic via certbot's systemd timer
  (`systemctl list-timers 'certbot*'`); dry-run it with
  `certbot renew --dry-run`.
- The vhost sets `proxy_buffering off`, `proxy_cache off` and a 24 h read
  timeout so SSE streams flow unbuffered, and enables HTTP/2. Certbot manages
  the `listen 443 ssl` / cert lines; the `http2 on;` line is ours — re-check it
  after any certbot config rewrite.

```sh
nginx -t && systemctl reload nginx
```

## Disk watch

Two filesystems, watched separately:

- **`/data` — `/dev/md3`, software RAID on the box's local NVMe** (1.7 T on
  2026-10-01; `df -h /data`), the one that grows with ingestion. It carries both
  the raw archive and the database, because the parsed DB alone will not fit the
  root disk (text satellites dominate — pilot-sizing.md).
  - `/data/archive/<source>/…` — raw fetched packages, immutable, append-only.
    TED under `ted/{daily,monthly}/`, DÖE under `doe/{daily,monthly}/`, FTS
    under `fts/{daily,monthly}/`. The FTS zips are assembled by the fetcher,
    one member per release. A `<period>.pages/` directory beside one is the
    staging of an unfinished walk and is removed once its zip lands.
  - `/data/db/tender-db.db` (+ `-wal`) — the Turso database. The raw archive is
    the large static tenant (~178 GB and barely moving between backfills); the DB
    plus its WAL is what grows during a load.
- **`/` — `/dev/md2`, the root disk** (120 G, 75 G used on 2026-10-01; `df -h /`).
  Pressure here is almost always the nix store
  (build artifacts + old bundles), not application data.

```sh
df -h /data /
du -sh /data/archive/* /data/db/*        # where the volume budget is going
ls -lh /data/db/tender-db.db-wal         # WAL size during a bulk load (issue 42)
```

A full backfill is the thing to plan for: the TED archive is the big one, and a
complete DÖE backfill is ~3 GB of ZIPs plus the projected rows. Watch `/data`
against a **~70 % operational guard** while a backfill runs — a self-imposed
ceiling well under the `/health/deep` 90 % alarm, leaving room to grow the volume
or pause the load before a write can fail mid-ingest. (The guard is a run-driver
convention, not a code constant; the only threshold in code is the 90 % deep-
health disk check.)

**WAL growth during bulk loads (issue 42).** Turso never auto-checkpoints fresh
frames, so a long `process`/`project` run would otherwise pile the whole run's
writes into `tender-db.db-wal` unbounded — it reached **13 GB and climbing**
during the first backfill. The processor now folds the WAL back at each package
boundary (a `wal_checkpoint(TRUNCATE)` at the writer-idle moment right after a
package commits), so the `-wal` stays bounded (single-digit GB) instead of
tracking the whole run. Idle pooled readers do not pin it; a reader mid-scan only
delays reclaim to the next package. The size is surfaced as `disk.wal_bytes` on
`/health/deep` — a large WAL mid-backfill is expected and never alarms, but a
*monotonically climbing* one across many packages is the signal that the
checkpoint is not reclaiming (investigate before `/data` fills).

Root-disk reclaim: `nix store gc` deletes unreferenced store paths — **including
old bundles you might want for a rollback**, so switch the symlink to the bundle
you want to keep before running it, or verify the current one is safe.

Note that nothing outside the service writes to `/data/db`: turso is
single-process, so the running server holds the database open exclusively (see
the Ingestion rule above). A scratch `*.db` may appear here from earlier
dev/verification work — harmless, but never point a CLI at the production file.

## Disaster recovery (no off-box backup)

**The snapshot feature and its local ring were removed on 2026-08-06** (owner
decision under storage pressure; commits 39c0e08/aa9f2a1/1faf9d9). There is still
**no off-box copy**. On the box there is again a weekly reflink snapshot — one kept since issue 488,
two before (issue 269, `tender-db-snapshot.timer`, Sun 05:23 Berlin; `ls -l
/data/db/snapshots`; mechanics and knobs under [Live DB file allocation vs
size](#live-db-file-allocation-vs-size-issue-169-2026-09-06)), but
it sits on the same volume: a forensics and verification artifact that dies with
`/data`, not disaster recovery. DR = re-ingest from sources. The full scenario
analysis, measured stage rates, and the recommendation menu for re-introducing a
minimal backup live in `docs/research/dr-premise-2026-08.md` (2026-08-09); the
honest numbers:

- **Canonical layer damaged** (bad projection, layer wipe; DB file healthy):
  `project rebuild=true` + verify ≈ **1–1.5 days**. The public instance serves
  an empty/partial tender layer the whole time (ADR-0009).
- **DB file lost** (archive intact): ≈ **4–6 days** — and today that includes a
  forced full ~180 GB re-download, because the `fetches` registry (which
  `process` walks) is itself DB-resident; there is no register-existing-files
  path yet (dr-premise §2 — a cheap planned fix).
- **Box lost, volume survives**: ≈ **0.5–1 day** (re-provision per this doc;
  everything on the root disk is regenerable).
- **NOT recoverable at any cost**: `users`, `api_tokens`, `webhook_endpoints`,
  and change-cursor/epoch continuity (< 1 MB today). No email exists on
  accounts by design, so account loss is permanent per user. An off-box copy
  of exactly this state is the standing pre-launch recommendation
  (dr-premise §7(b)), pending the owner's re-decision.

Mechanism knowledge worth keeping (if a backup path returns): turso has no
online-backup API and `VACUUM INTO` OOMs at scale (turso-scale.md §1); the
workable mechanism is writer-held `wal_checkpoint(TRUNCATE)` + file copy +
offline verify by `integrity_check` **plus row-count comparison** (a killed
copy can pass integrity_check alone). Measured at 455 GB on this box:
**941 s writer freeze + 606 s verify** (job 570, 2026-08-06 — the last
snapshot ever taken). Restore was a plain file copy: stop service, swap
`.db`, delete `-wal`/`-shm`, chown, start, `/health`.

### Disk headroom

`/data` (1.7T in `df -h /data` on 2026-10-01, i.e. 1.78 TB) holds archive (~180
GB) + DB (685 GB on 2026-10-01, `stat -c %s /data/db/tender-db.db`, and growing; it
can never shrink — VACUUM is impossible) + the snapshot (up to a full DB's worth once
un-shared, issue 488). 443 GiB (475.7 GB,
`/health/deep` `free_bytes`) free on 2026-10-01; a plain on-box DB copy no longer
fits (XFS reflink copies do).
Growth model and volume-full forecast: `docs/research/` storage-lifecycle
study (issue 169).


## Verification

Two black-box binaries check a *running* instance from the outside — they talk
only to its public API (default `https://tenders.zebreus.click`), never to the DB
or the box, so they run from the dev machine. Both use the account-gated
read-only `/v1/sql` endpoint for the counting queries and need an API token
(`--token`, or `TENDER_API_TOKEN`); without one the token-gated checks are
reported *skipped*, never silently passed.

- **`verify`** (`crates/ingest/src/bin/verify.rs`) — the standing **acceptance
  harness**: pass/fail against *external* ground truth. It checks per-year TED
  coverage against the vendored counts, cross-checks a few eForms days against the
  live TED Search API (set membership), and walks one real notice per format era
  through the API (Notice → Tender → award → winner). Exits non-zero on any
  executed check that fails or could not run. **Run it after a deploy and after a
  backfill** to confirm the instance still meets ground truth. A partially
  backfilled instance honestly reports failure for the years it does not yet hold.
- **`data-quality`** (`crates/ingest/src/bin/data-quality.rs`) — the descriptive
  sibling: **no pass/fail**, it *measures* how complete the imported data is
  (per-era field completeness, award linkage, results materialisation, TED↔DÖE
  merge) with bounded `GROUP BY` aggregates, safe against the live rate-limited
  SQL endpoint. **Run it to read the numbers** after a parser change or backfill,
  when you want more depth than the dashboard's data-quality panel.

```sh
# From a dev checkout (nix provides the toolchain); --json for machine output.
TENDER_API_TOKEN=<token> cargo run -p ingest --bin verify
TENDER_API_TOKEN=<token> cargo run -p ingest --bin data-quality
```

### Running the tests

CLAUDE.md's **Testing** section owns this recipe, and this section only points at it so
the two cannot drift: run the suites through `ops/check.sh` (the gate `deploy.sh`
reads), and run a single focused test only with the gate's flags AND the gate's package
set, as CLAUDE.md spells out. A plain `cargo test` (debuginfo on) and a `-p <crate>`
alone (features resolved for that crate only, issue 260) each build a second artifact
family of every crate, and both have filled this container's disk.

The trap that used to be the whole of this section still holds: `crates/app`'s server
modules (`v1`, the supervisor, health, metrics) sit behind `#[cfg(feature = "server")]`,
so `cargo test -p tender-db` without that feature compiles none of them and prints `test
result: ok. 0 passed; 0 failed` — a pass that checked nothing. The gate's command
carries `--features tender-db/server`.

## Open items

Known gaps in the production setup, tracked here so they aren't rediscovered:

- **Reboot survival is unexercised.** The unit is `enabled` (survives reboot by
  configuration), but no actual reboot has been done to confirm the service, the
  `/data` mount, and nginx all come back clean. Needs a deliberate quiet window —
  do it when no ingestion/backfill is in flight, then verify `systemctl status
  tender-db` and `curl https://tenders.zebreus.click/health`.
- **The off-box watcher is a per-4h Claude routine, not sub-minute.** `/health/deep`
  ships and covers uptime, freshness, disk and job failures, and the external
  Claude scheduled routine polls it every ~4 h (see
  [Monitoring and alerting](#monitoring-and-alerting)). That means an outage can
  go unnoticed for up to ~4 h — accepted for now given the rebuildable dataset
  and single operator; tighten the interval if that changes.

### Live DB file allocation vs size (issue 169, 2026-09-06)

`/data` is XFS with reflink, and the weekly snapshot ring makes the live DB a
reflink-shared file: every page rewrite is a copy-on-write allocation. With XFS's
default COW extent-size hint (128 KiB) each 4 KiB page rewrite reserved a 128 KiB
window and left the unused remainder as speculative preallocation that the periodic
GC never reclaimed on the always-open DB file — 220 GiB of `du`/`stat.blocks` over
the file's size by 2026-09-06, counted by `df` as used. Two standing measures:

- the live file carries `cowextsize 4096` (`xfs_io -r -c "stat -v" /data/db/tender-db.db`
  shows `fsxattr.cowextsize = 4096`), so a COW write allocates exactly the page. If
  the file is ever replaced (a clone swap, a restore), set it again on the new inode.
- if the disk census shows the live file's allocation well above its size again:
  `xfs_spaceman -c "prealloc -s -m 100g" /data` frees the speculative reservations of
  every file ≥ 100 GiB, online, without touching data — 2.5 minutes and 224 GB back
  the first time. Bounded and free under the prod-box-reads rule (metadata only).

The reads that diagnose it are all bounded: `stat -c '%s %b'` on the file (size vs
blocks × 512), `xfs_io -r -c "stat -v"` for the hints and extent count. `filefrag` and
`xfs_bmap` on this file are NOT (millions of extents).

**Superseded in part by issue 488 (2026-10-06): the 4 KiB hint fragments the file.**
The page-sized COW allocation above fixed the preallocation leak and caused something
worse. Each weekly reflink snapshot shares every extent with the live DB, and every
page the app rewrites afterwards is copied out of sharing *in units of `cowextsize`*.
At 4 KiB that was one new extent per rewritten page: by 2026-10-05 the 694 GB file
was 90.2M extents (~7.7 KB each), and a `perf` sample of a fold showed 76 % of its
CPU in the kernel walking the extent tree (`xfs_iext_lookup_extent` and friends).
The same un-sharing is a disk cost: a snapshot costs nothing on the day it is taken
but grows toward a full second copy as the DB is rewritten — a corpus-wide refold
un-shared ~40 GB/h of `/data` (319 → 280 GB free, issue 479). On 2026-10-06 the
file was defragmented (`ops/defrag-db.sh`, 90.9M → 1.95M extents, ~50 min downtime),
both snapshots were deleted, and the hint was raised to `cowextsize 16m` by hand (inert while
no snapshot shares the file; the script's default stays 4096, below).

`tender-db-snapshot.sh` now holds that line itself, with three knobs (environment
variables on the unit):

- **Decided 2026-10-06 (issue 488): the weekly timer is disabled** — `install.sh` no longer enables it and disables it if present; the script and unit remain for a deliberate manual run. Without a reflink snapshot nothing is copy-on-write, so neither the fragmentation nor the leak can occur.
- **`TENDER_SNAP_COWEXT`** (default `4096`, issue 169's leak-safe value; issue 488
  decides whether to drop reflink snapshots or pair a larger hint with a periodic
  `xfs_spaceman prealloc -s` reclaim): before each snapshot it runs `xfs_io -c
  "cowextsize <value>"` on the live DB, so a replaced inode (restore, defrag swap) gets
  the hint back within a week. A failure is a `WARN snapshot: could not set
  cowextsize …` line, not fatal. The trade-off is the one this section opened with:
  a large hint can leave speculative COW preallocation on the always-open file. If
  `stat -c '%s %b'` shows allocation well above size again, the `xfs_spaceman …
  prealloc` command above reclaims it online.
- **`TENDER_SNAP_KEEP`** (default `1`, was 2): every snapshot alive is another
  potential full copy in un-sharing. Pruning still happens AFTER the new copy, so
  with `KEEP=1` a snapshot exists at every moment. The cost is that the old one's
  exclusive blocks are still held while the new one is taken. The prune never
  removes the last snapshot or the one just written, and `KEEP=0` prunes nothing.
- **`TENDER_SNAP_FREE_FACTOR`** (default `1.2`): the disk guard. Since one snapshot
  can cost up to the whole live file in un-sharing before the next run prunes it, the
  script refuses unless `df -B1 --output=avail` on the DB's filesystem reports at least
  live DB size × factor. It then prints `SKIP snapshot: only N bytes free …, need M`
  and exits 0: nothing is written, nothing pruned, and the old snapshot stays. If
  space is short, delete the old snapshot by hand and re-run. Free space that cannot
  be parsed fails closed (`ERROR snapshot: cannot read free space …`, exit 1).
