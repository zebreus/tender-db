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
  request) + `process fts daily (all)`; then one `project` that folds whatever landed.
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
#                             `fetch` (issue 450: `stoppable` in supervisor.rs).
#                             A TED/DÖE fetch is one download and answers 409.
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

### Reading a `project` job's `counts` line (issues 318, 364, 448)

A finished `project` writes one summary line to its job row (`GET /admin/jobs` →
`recent[].counts`), and the run's gate tallies ride it. The service's stderr does
reach the journal (the `[project] …` heartbeats are `eprintln!`; issues 61/63 once
said otherwise), but the journal is size-capped and rotates, while the job row is
durable and is what `/admin/jobs`, `jobwatch` and the issues' Verify lines read:

```
<n> notices → <t> tenders (<i> islands), <v> versions; <w> tenders written, <u> verified unchanged
; <the issue-318 genericness-wall suffix, when the wall was consulted>
; issue-448 alias asked A bound B refused R (poisoned …, no owner …, veto …, names …, generic …)   ← only when the altid alias was asked
; issue-364 previous-publication citations: N admitted, M refused (prior-information …, buyer-profile …, periodic-indicative …, qualification-system …, DPS …, undeclared …, unknown kind …)
; issue-364 edges refused by the cited notice's own type: K (prior-information …, buyer-profile …, periodic-indicative …, qualification-system …, DPS …, unknown kind …)
; issue-481 tender links joined: J (previous-notice … of which cross-source …, logical-notice …, matched …); refused: R (not-earlier …, fan-in …, not-one-to-one …, keyed-weld …, oversized …); deferred: D; largest component: L key(s)
```

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
fold, zeros included, with how many aliases the fold was armed with.

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
`buyer_disjoint` ones below do not), `cross_source` (of `would_merge`,
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
  buying for two schools does not meet itself);
- a name that is a **whole-word prefix** of the other's name or equals its **head**, the text before
  the first separator, of three content words or more (`ARPAS` / `ARPAS - Agenzia…`, two
  `Servicio Andaluz de Salud. …` units), but never two names that only start alike;
- an **acronym** and its spelled-out name (`ICS` / `Institut Català de la Salut`).

Still disjoint, and so refused: one buyer that changed or respelled its name beyond that, with no
identifier in common, written in another co-official language (`Servizo Galego de Saúde` against a
`Servicio Gallego de Salud` area), or abbreviated mid-name (`im. M.Nenckiego PAN`); a CAN filed by
another body than the CN's buyer; and a group PIN or qualification system (the DB group's
deadline-shortening PIN, ÖBB-Holding's), which is cited by many procedures and is not one of them.

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

### The organization-layer jobs (issues 300, 311-317)

These are their own family: censuses that measure, merge arms that write, and
review machinery that records verdicts. **Every writing one defaults to
`dry_run: true`** — a forgotten flag must mean the harmless thing — and the
merge arms run the T4 ladder (dry census → recorded plan → capped wet →
uncapped wet), so a wet run refuses unless its dry plan is on file.

| kind | writes? | notes |
|---|---|---|
| `org-merge-health`, `r2-census`, `r3-census` | no | the standing measurements; `org-merge-health` is weekly and carries the Stage-5 `null_country` bucket (issue 300) plus three tripwires read against its previous report: `parser_vs_stock.alarms` (issue 325), `gln_9110.shared_one_country` (327) and `name_growth.alarms` (300 tripwire 2: a top-100 org gaining ≥20 distinct names, a new entrant at ≥50, or `ge6` growing >5 %). Any alarm shows in the job's summary line as `… ALARM(S): …`; the first run after a deploy that adds a block has no baseline for it and stays quiet |
| `match-org-identifiers` (`rule: r2` / `e0` / `r3`) | YES | the merge arms; `max_groups` caps a run. Since issue 359 the R2 arm denies a group whose named members share no core token (`denied_names`, listed for review in `r2-merge-plan`); since issue 362 a reviewer's verdict in `org_merge_verdicts` overrides that (`denied_verdict`, `admitted_verdict`, `verdict_stale`) |
| `match-org-identifiers` (`rule: altid`) | YES (wet; dry by default) | issue 448: the Companies House ↔ PPON pairs FTS parties publish in `additionalIdentifiers` (the fold keeps only a party's first identifier, so a supplier published PPON-first somewhere stands as a second org). Re-harvested from the FTS notices' `BT-501` rows every run and gated as E2 evidence: v2 gate, consortium veto, GB legal form head against head, the mention-evidence wall and a PPON org naming another GB key (no verdict overrides those), then a `keep`/HIGH `merge` verdict under (`GB`, `GB:altid`, `<coh>~<ppon>`), then the conflict flags, name-key corroboration and the generic wall. Corroboration reads each org's names from its mentions OUTSIDE the pair's witness notices (a side made only of witness mentions uses those), because a witness's own party name is recorded on the org its first identifier binds and would otherwise vouch for itself; a pair only the designated names join is listed `witness-only`, and names whose GB legal forms conflict across the two sides (plc against Ltd, whichever names the heads carry) list as `form-conflict` (issue 448 unit 1b). Stores `altid-merge-plan`: `both_distinct` is the issue's split count, `pairs` the sorted planned keys, `plan` / `denied` / `conflict_listing` list pairs in the merge-verdict shape and `no_target_sample` samples the pairs with no standing org. Refuses like r3 while a key build is in flight or the keys epoch drifted, and a wet run also while `org_match_keys` is empty. **Wet (issue 448 unit 2): dry first, always.** A wet run reads the latest `altid-merge-plan`'s `pairs` as the reviewed set (refused with no stored plan, and while an org FK index is missing, like r2), re-plans live under the writer with the same gates, and aborts before any write when the live plan and the stored set differ by more than max(2% of the stored set, 5) pairs (symmetric difference; the error names both counts). It merges only live ∩ stored pairs, in sorted key order, 50 per transaction, at most `max_groups` of them: the PPON org folds into the company-number org (always the survivor), `org_merge_log` rule `e2-altid` with flat evidence (`coh`, `ppon`, both literals, `keep_id`/`loser_id`, `name_key`, `witnesses`, up to three `witness_notices`, `verdict`), change events, and an admitting HIGH verdict stamped applied. A live pair the stored plan lacks is **deferred** (`deferred_unreviewed`/`deferred_pairs`), never merged: the next dry plan lists it for review. The two-org pairs a gate denied become open `e2-altid`/`E2` rows in `org_candidate_edges` (evidence `status` = the gate; never deleted; `state='merged'` once merged); `org-edge-census` counts them apart (`e2_altid`, `e2_altid_merged`) and neither it nor `xb-packet` joins them into components. The run re-records `altid-merge-plan` as the RESIDUAL (`residual_of_wet_run: true`, `pairs` = the reviewed pairs still unmerged), so a capped or stopped run continues under parity. The `plan` listing carries EVERY planned pair (issue 448 unit 3, the campaign's prerequisite: `ALTID_PLAN_LISTING_CAP` = 20,000, far above any realistic plan, `plan_listing_truncated` if it ever binds; the `denied` and `conflict_listing` listings keep R2's 500), each with up to three witness publication ids, so a reviewer reads every pair a wet run would merge. Every two-org listing also carries what the pair is judged by (unit 3b): `coh_names` / `ppon_names`, the names each side's corroboration read (the first 8 in key order, `coh_name_keys` / `ppon_name_keys` the totals; a head name is a first-seen election that one stray mention can set), `corroborated_by`, the name pair that cleared the wall (planned pairs only), and `cooccurring` / `cooccur_publications`, the notices other than the pair's own witnesses where both orgs are distinct parties (a wrong pair shows it, and so does a true pair beside a publisher that gave one supplier the other's identifier). `plan_cooccurring` counts the planned pairs that co-occur; it gates nothing. **The resolver alias (issue 448 unit 3).** Every `project` fold (incremental, full and rebuild) arms its mention resolver from the `e2-altid` ledger: each merged PPON key maps to the company-number KEY it was merged with (identity to identity, never org ids, so it survives a rebuild's renumbering); a pair under a `keep` verdict (`GB`, `GB:altid`, `<coh>~<ppon>`) is dropped, and a PPON the ledger merged into two company numbers is poisoned and never aliased. A mention whose raw identifier keys to that PPON, and which misses both the exact triple and the canonical key (its org was folded away), binds to the org that now owns the company number, at the arm's own bar and never looser: neither key poisoned, a standing owner, no consortium name on either side, the same GB legal form head against head, the planner's shared name predicate (`altid_corroborates`) between the mention's names and the owner's head and satellites, and the generic wall through the resolver's memo (lenient when the wall is disabled for the run, counted as bound with the wall unable to answer). A bind is never cached, so each repeat re-earns the bar with its own name. A refused mention mints or binds exactly as it would without the alias; as before 448, the PPON org it mints then takes the later mentions of that PPON, and the next dry plan lists the pair again. In a rebuild, a PPON-first notice older than the supplier's first company-number-first notice finds no owner yet and mints (`no owner`). The alias logs `[issue 448]` to the diag log on every fold (armed with how many aliases, then asked / bound / refused by cause, zeros included) and adds `; issue-448 alias asked … bound … refused …` to the `project` job's counts line when it was asked anything. Since issue 460 a bind also records the PPON as a merged identifier (`organization_merged_identifiers`, rule `altid-alias`) of the org it bound to when that org carries none, once per (PPON, org) a fold, each an `organization changed` (the counts line then says `N PPON(s) recorded as merged identifiers`): after a from-archive rebuild, which empties that table, the alias binds a merged PPON straight to the company-number org, no PPON org is minted and no merge ever writes it back, so this is where `?identifier=<PPON>` comes back. **No wet run until unit 3 is deployed AND the full `plan` listing of the dry plan the wet run will hold to has been reviewed** (issue 448 unit 4's campaign). The deploy alone lifts nothing: without the alias the next FTS fold re-mints every PPON-first supplier a merge folded, and without the review nobody has read what the run merges |
| `match-org-identifiers` (`rule: rekey`) | YES (wet; dry by default) | issue 453: acts on the issue-452 `wrong` identifier verdicts that name the right number (`correct_identifier`), `high` confidence only, for the org that carries the wrong triple now (none: `gone`; several: `several`, E0's family). GB company numbers only (`unkeyed` otherwise); a right number that keys where the wrong one does is `same_key`. The right number's owners are found through the canonical key over every identifier-bearing org, so any spelling counts. **merge**: exactly one owner, not itself under a `wrong` verdict (`withheld_target`), no consortium name on either side, the same GB legal form head against head, and one agreeing name pair under the altid name key (`denied_names`; issue 454: a HIGH `merge` verdict in `org_merge_verdicts` under (`GB`, `GB:rekey`, `<wrong literal>~<right number>`) whose members are exactly the two org ids admits past the name gate only, counted `admitted_verdict` and stamped applied by the merge; a `keep` verdict there denies, `verdict-keep`): the wrong-number org folds into the owner, `org_merge_log` rule `rekey` (evidence: `country`, `kind`, `wrong`, `right`, `keep_id`), change events, touched tenders changed. Several owners is `multi_target` (R2's family to fold first). The right owner must not carry a `wrong`/`related` verdict itself, nor may a move's destination triple (`destination_verdict`). **move**: no owner: the org's identifier becomes the right number's canonical body in the wrong literal's spelling (`GBCOH…` kept, bare stays bare; a reviewer's `GB-COH-…` never doubles the prefix) and one `organization changed` event goes out. Two owner-less candidates naming the same right number: only the one with the smallest key moves, the others list `pending-move` (after it moves they are merges into it, planned by the next dry run). Both stamp the verdict (`applied_at`, `applied_literal` = what the entity carries since, `job_id`). Stores `rekey-plan` (`keys` sorted, `plan` / `denied` listings with each target). A planned key pins what was reviewed: `<country>/<kind>/<wrong>>merge:<target literal>` or `…>move:<new literal>`, so a re-POSTed right number or a changed target is a different key and is deferred, never executed unreviewed. A wet run's re-recorded plan counts `plan_merge`/`plan_move` over the residual it records (`live_plan_merge`/`live_plan_move` are the live re-plan's). **Wet: dry first, always.** A wet run reads `rekey-plan`'s `keys` (refused without one, and while an org FK index is missing), re-plans under the writer, aborts before any write when the live and stored keys differ by more than max(2%, 5), executes only live ∩ stored in key order (50 per transaction, at most `max_groups`), defers a live key the plan lacks, and re-records the residual. **The alias**: every fold's resolver maps the stamped wrong triple to the org carrying `applied_literal`, following a chain of re-keys (at most 8 hops) and resolving at bind time, so an entity merged away since binds to its survivor, (the publisher's repeat reaches the entity, as it reached the wrong-number org before), and guards the wrong number's canonical key with that org among its owners, so another spelling of the wrong number reaches it only under a matching name (the number may be someone else's real one). Logged in the resolver's `[issue 452]` line as `re-keyed wrong number(s) aliased`. **Merged identifiers** (issue 460): a `rekey` merge never writes the loser's wrong number for the survivor, and both arms drop every merged identifier that spells it (canonical key or literal) from the entity — an earlier `r2` may have folded the bare number or another spelling into the re-keyed org, which the merge's carry would move onto the right company and the move would leave in place; identifiers that are not a spelling of it (a PPON) travel as in any merge. Counted `merged_identifiers_dropped_this_run` in the wet run's `rekey-plan` and its summary |
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
| `repair-label-prefixes` | **dry default** | issues 328/359: strips a publisher's field name glued to the identifier (`USTID…`, `NIP…`, `PIVA…`, `CIF…`), then re-validates |
| `repair-renormalised-identifiers` | **dry default** | issue 345: standing rows re-read under the live normaliser (Greek lookalikes, RO sub-unit suffix) |
| `repair-notice-instants` | **dry default** | issue 367: re-derives `notices.published_at` / `dispatched_at` from each parsed notice's own stored parse. Dry stores `notice-instant-repair`; the wet arm reads that plan's `rows` and aborts outside max(2%, 5). Read `resolver_silent` before running it wet — that class REMOVES a stored value (a dateless payload stamped 1970-01-01), where `epoch_published` and `null_published` add one. Writes `notices` only: the tender versions were never wrong, so nothing is re-projected |
| `sweep-orphan-orgs` | **dry default** | issue 443: deletes the organizations no recorded mention points at any more (issue 434's refresh re-binds a mention and leaves the row it left) — only when no party, bid-party or winner row names them either (`referenced`, kept until a fold moves those rows) and no review table does (`protected`: case reviews, re-homing case and target, country and merge verdicts, name drops — read inside every window, so a verdict POSTed mid-run protects its org). Provisional or not (step 4: the non-provisional orphans, tallied under `identified`, are identities no evidence carries any more). Each deleted row leaves its pre-image, identity and name variants included, in `org_sweep_log` (restorable from that row alone), takes its `organization_names` rows and publishes `organization removed`; foreign keys stay ON. Refuses to start while any of its five `(organization_id)`/`(org_id)` indexes is missing (run `reindex`). Every run counts first: dry stores `orphan-org-sweep-plan`, wet holds its count against that plan's `swept` and aborts outside max(2%, 50), then re-records the residual after every window that deleted, so a stopped, failed or restarted wet run resumes under the same parity. **A fold that re-bound any mention queues it by itself** (job params `sweep-orphan-orgs auto (after a fold re-bound N)`, unless one is already queued or running): that run counts, sweeps at or under 10,000 orphans (`AUTO_SWEEP_CAP`) with no stored plan needed, and above it records the plan and writes nothing, leaving the wet run to a person |
| `backfill-merged-identifiers` | **dry default** | issue 460 unit 3, one-shot: gives the identifiers merges folded away BEFORE `organization_merged_identifiers` existed a place on today's survivor, so `?identifier=` finds it by them (every merge since writes its loser's identifier itself, in `repoint_org_references`; never a `rekey` loser's). Reads `org_merge_log` per rule: `e2-altid` through its partial index, then one rowid walk of the whole ledger (50,000 rows a window) for `r2`, `e0` and `r3`, whose evidence names the loser's literal as `$.loser_id` (`p0`/`p1` use that key for the org id and are never read). Each keep is followed through the merge ledger to its live survivor (`resolve_org`'s walk, 8 hops), with time checked three ways, any failure `out_of_time` and refused: the merge must be no older than the **era floor** (`created_at` of the lowest live org id — the oldest live organization; a from-archive rebuild re-mints every org id from 1, so a ledger row older than the floor names ids that now belong to other companies, and when such an id was minted again and merged in the new era the per-hop checks cannot see it), a hop must not be merged before the merge it continues, and the live row must not be minted after the merge that names it. The row takes the survivor's country and the kind the live normaliser gives the literal there (`kind_from_survivor` counts the literals it refused, which take the survivor's kind). A literal that spells (canonical key or literal) a number a reviewer found is not its carrier's — a `wrong` identifier verdict, applied or not, or a triple a `rekey` acted on — is written only when the survivor carries that number itself (its real owner); otherwise it is `wrong_number` and refused: the chain crossed a `rekey` merge onto the right company (`r2` and `e0` rows alike), or the survivor was re-keyed in place by the move arm. Per rule it counts `rows`, `written`, `present` (a row already stands for that identifier and loser, or the survivor already carries the identifier — a merge or an alias bind wrote it), `same_as_survivor` (nothing was lost — an `e0` row whose survivor still carries the number), `no_literal`, `unresolved` (no live survivor), `out_of_time`, `wrong_number` and `loser_live`; the report carries the `era_floor` it held rows against. Writes that lookup table and publishes `organization changed` for each survivor it wrote for (`organizations_changed`; its served `merged_identifiers` grew), one transaction per window; idempotent, stoppable between windows (a stopped run stores no report; its committed windows read as `present` next time). Both modes store `merged-identifier-backfill`. Expected on prod (2026-10-01): `e2-altid` 4,738 rows; `r2`/`r3` unknown until the dry run reads them. **Read the dry run's `out_of_time` and `era_floor` before the wet one**: a large `out_of_time` with an `era_floor` later than the first `e2-altid` jobs (1717) means a from-archive rebuild happened since, and those rows are refused by design. **After a from-archive rebuild** the table is emptied (it names re-minted ids) and this job cannot refill it (every older ledger row is below the floor): the fold's altid alias records each merged PPON it binds (above). Another spelling of an E1 register key binds to the org already holding that key (the resolver's canonical-key prevention, issue 300 Stage 2), so in a rebuilt fold it is never minted apart and never merged, and `?identifier=` by that spelling answers nothing: the documented exact-value scope of `identifier=`, not a lost merge |
| `analyze` | YES (`sqlite_stat1` only) | issue 429: `ANALYZE` of each table in `store::ANALYZE_TABLES` — the 24 read-path tables issue 428 measured a benefit on — one table per statement, so the writer is released between tables (the longest, `tender_version_parties`, took 242 s on prod data) and a stop costs at most the table in flight. Never touches a table outside the list; `organizations` is excluded (its resolver lookup went from 0.001 s to 6.19 s with statistics) and any `sqlite_stat1` row for it is deleted. Always ends, even when stopped, with a throwaway `CREATE`/`DROP TABLE` so pooled readers re-plan with the new statistics (they otherwise keep what they loaded at open, `tests/analyze_stats_pickup.rs`). The summary lists every table's seconds. Not scheduled yet: the schedule waits on 429's plan-capture diff |
| `repair-minted-countries` / `repair-placeholder-orgs` / `repair-nested-orgs` / `repair-swept-siblings` | **dry default** | the earlier repairs (issues 325, 300 Stage 1, 234, 259) |
| `ghost-census` / `disk-census` | no | issues 278 and 169: weekly stamps. `disk-census` also carries the live file's `db_allocated_bytes` beside `db_bytes` and a `db_overallocation_alarm` when allocation runs more than 5 % (and 1 GiB) past the size — the copy-on-write leftover class of issue 169 item 3, with the reclaim command in the alarm text; and `deleted_open_files` / `deleted_open_bytes` (+ a five-entry sample and `deleted_open_alarm` at 1 GiB) — files the server process holds open after unlinking them, issue 361's class, also on `/metrics` as `tender_db_deleted_open_{files,bytes}` |

Report kinds do NOT always match the job kind that writes them. `fusion-census`
stores under `fusion-candidates`, and `GET /admin/reports/<kind>` answers an
unknown kind with "no report of that kind has been computed" — which reads as
"the job never ran". The kinds are, exhaustively (read off `put_report` in the supervisor, 2026-09-06; `altid-merge-plan` since issue 448, `rekey-plan` since issue 453, `merged-identifier-backfill` since issue 460, `tender-link-backfill` since issue 481): `altid-merge-plan`, `anchor-wall-census`, `case-apply-plan`, `case-escalations`, `case-unapply-plan`, `country-cluster-census`, `country-cluster-packet`, `country-fold`, `country-typo-census`, `country-typo-repair`, `country-verdict-plan`, `data-quality`, `data-quality-headlines`, `data-quality-presence`, `disk-census`, `drop-orphan-satellites`, `duplicate-identity-census`, `e0-merge-plan`, `fusion-candidates`, `generic-statistic-census`, `generic-wall-census`, `ghost-census`, `label-prefix-repair`, `member-twin-census`, `member-twin-repair`, `merged-identifier-backfill`, `minted-country-repair`, `name-attribution-probe`, `name-pollution-census`, `notice-instant-repair`, `org-edge-census`, `org-edge-scan`, `org-edge-scan-alarm`, `org-edge-scan-plan`, `org-match-keys-build`, `org-match-keys-plan`, `org-merge-health`, `orphan-org-sweep-plan`, `provisional-echo-census`, `provisional-echo-plan`, `provisional-name-norm-plan`, `r2-census`, `r2-merge-plan`, `r3-census`, `r3-merge-plan`, `registry-contiguity`, `rehash-cursor`, `rehash-probe`, `rehoming-packet`, `rehoming-plan`, `rekey-plan`, `renormalise-repair`, `reveal-cursor`, `reveal-recheck`, `reveal-wrap`, `satellite-orphans`, `tender-link-backfill`, `xb-packet`.

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
| `POST /admin/case-reviews`, `/admin/rehoming`, `/admin/name-verdicts`, `/admin/country-verdicts`, `/admin/merge-verdicts` | record one cohort's verdicts into the five review stores (`org_case_reviews`, `org_mention_rehoming`, `org_name_verdicts`, `org_country_verdicts`, `org_merge_verdicts`); recording only — the apply jobs (or the R2 arm, for merge verdicts) execute the HIGH subset |
| `POST /admin/identifier-verdicts` | record one cohort's per-identifier verdicts into `org_identifier_verdicts` (issue 452): `{org_id, identifier, verdict: wrong\|related\|right, correct_identifier?, rationale, confidence}`. Recording is the action: from the next planner run or fold, a `wrong` number is left out of the R2/E0/R3 groups, the altid arm's owners, and the resolver's canonical bind: the number's key binds another spelling only to the owner whose names the mention's names match, a mint joins those owners instead of claiming the key, and the exact triple still binds, and `/v1/organizations` serves `identifier_status` (`register_mismatch` for `wrong`, `related_entity` for `related`). A verdict is keyed by the identity triple (identifier, kind, country) the named org carries now, so a rebuild's renumbering keeps it; one naming an org that no longer carries the number is skipped (`stale` in the answer). Recording never applies `correct_identifier`: the `rule: rekey` arm of `match-org-identifiers` does, for HIGH `wrong` verdicts, and stamps `applied_at` / `applied_literal` / `job_id` on the verdict (a re-POST keeps them). The readback (`/admin/case-reviews?table=identifier`) serves those three columns |
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
**no off-box copy**. On the box there is again a weekly reflink ring of two (issue
269, `tender-db-snapshot.timer`, Sun 05:23 Berlin; `ls -l /data/db/snapshots`), but
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
can never shrink — VACUUM is impossible) + the snapshot ring. 443 GiB (475.7 GB,
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
`xfs_bmap` on this file are NOT (30 M+ extents).
