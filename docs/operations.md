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
#                             `audit-fts-ids` (issue 477 unit 3).
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
`residual_slip`). It catches a lot slip whose true value appears nowhere in the chain: 8784848's
£60 bn lot 1 is 100× the £600 m its £1 bn procedure leaves after lots of £150 m and £250 m. A ratio
alone was no signal (23 heads at ≥ 10× adjudicated: 18 slips, 5 genuine, no ratio band separates
them); this shape had no false positive, and over every version holding a stored lot value of
€1 bn or more it fires on 5 Tenders, all slips (`.scratch/tender-db/505-lot-over-procedure/`).
Drain as for the ×100 extension (`refold-value-band` dry, wet, `project`). Expected: 8784848's head
falls from €72 bn to £1 bn, 553044's to €26.958 m, 5748163's and 1120720's to their procedure
figures; 915781's lot 5 loses its stored value (its head is already the procedure figure). Nothing
else moves; a sixth Tender is a finding.

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
<n> notices → <t> tenders (<i> islands), <v> versions; <w> tenders written, <u> verified unchanged
; <the issue-318 genericness-wall suffix, when the wall was consulted>
; issue-448 alias asked A bound B refused R (poisoned …, no owner …, veto …, names …, generic …)   ← only when the altid alias was asked
; issue-364 previous-publication citations: N admitted, M refused (prior-information …, buyer-profile …, periodic-indicative …, qualification-system …, DPS …, undeclared …, unknown kind …)
; issue-364 edges refused by the cited notice's own type: K (prior-information …, buyer-profile …, periodic-indicative …, qualification-system …, DPS …, unknown kind …)
; issue-481 tender links joined: J (previous-notice … of which cross-source …, logical-notice …, matched …); refused: R (not-earlier …, fan-in …, not-one-to-one …, keyed-weld …, oversized …); deferred: D; largest component: L key(s)
; issue-385 F14 corrigendum dates: …
; issue-479 procedure type: eforms F folded / U unmapped / N none, r209 …, text …   ← one entry per profile family the run planned
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
"the job never ran". The kinds are, exhaustively (read off `put_report` in the supervisor, 2026-09-06; `altid-merge-plan` since issue 448, `rekey-plan` since issue 453, `merged-identifier-backfill` since issue 460, `tender-link-backfill` since issue 481, `procedure-key-census` since issue 482, `buyer-role-census` since issue 483): `altid-merge-plan`, `anchor-wall-census`, `buyer-role-census`, `case-apply-plan`, `case-escalations`, `case-unapply-plan`, `country-cluster-census`, `country-cluster-packet`, `country-fold`, `country-typo-census`, `country-typo-repair`, `country-verdict-plan`, `data-quality`, `data-quality-headlines`, `data-quality-presence`, `disk-census`, `drop-orphan-satellites`, `duplicate-identity-census`, `e0-merge-plan`, `fusion-candidates`, `generic-statistic-census`, `generic-wall-census`, `ghost-census`, `label-prefix-repair`, `member-twin-census`, `member-twin-repair`, `merged-identifier-backfill`, `minted-country-repair`, `name-attribution-probe`, `name-pollution-census`, `notice-instant-repair`, `org-edge-census`, `org-edge-scan`, `org-edge-scan-alarm`, `org-edge-scan-plan`, `org-match-keys-build`, `org-match-keys-plan`, `org-merge-health`, `orphan-org-sweep-plan`, `procedure-key-census`, `provisional-echo-census`, `provisional-echo-plan`, `provisional-name-norm-plan`, `r2-census`, `r2-merge-plan`, `r3-census`, `r3-merge-plan`, `registry-contiguity`, `rehash-cursor`, `rehash-probe`, `rehoming-packet`, `rehoming-plan`, `rekey-plan`, `renormalise-repair`, `reveal-cursor`, `reveal-recheck`, `reveal-wrap`, `satellite-orphans`, `tender-link-backfill`, `xb-packet`.

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
