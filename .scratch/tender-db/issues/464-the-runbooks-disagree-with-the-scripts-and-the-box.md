# 464 — the runbooks disagree with the scripts and the box

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the DST wording, due before the switch to CET at 2026-10-25 01:00 UTC: write the daily tick and the journal clock as Berlin wall-clock in the handover and in the `tender-db-tmpsweep.timer` comment.
Kind: docs drift (runbooks, agent rules, box tooling)
Relates to: 224 (box-only scripts vanished; why box tooling lives in git), 459 (uncommitted edits in
`docs/operations.md`, `deploy.sh`, `ops/watchdogs/install.sh` and `test-watchdogs.sh` as this was filed), 245 (the
startup catch-up), 269 (the snapshot ring), 339 (stderr read from the journal), 425/438 (the interrupt), 404/432 (the
stack lesson), 260 (test artifact families), 24 (the watcher prose, open)

## What is wrong

Four groups. Every line below was re-read on 2026-10-01 at repo `3994975` and on the box (deployed `9b44528`).
Line numbers are `3994975`'s. 459's uncommitted edits add lines to `docs/operations.md` (after 60, 113 and 476) and
`deploy.sh`, so once they land, find a line by its quoted text.

### 1. DST: the UTC wording goes stale on 2026-10-25 (the first unit)

The scheduler runs on Berlin wall-clock and is correct. `spawn_scheduler` sleeps until `next_berlin_tick(now, 9, 35)`
(`crates/app/src/supervisor.rs:11647`), `berlin_offset` (`:12416`) applies the EU rule, and
`berlin_offset_follows_the_eu_dst_rule` (`:13337`) asserts the switch at 2026-10-25 01:00 UTC. `docs/operations.md:193`
says "09:35 Europe/Berlin", which is also correct. The box clock is `Europe/Berlin (CEST, +0200)` (`timedatectl`), and
the journal and nginx stamp in it: `2026-10-01T09:39:34+02:00` and `[01/Oct/2026:14:02:47 +0200]`.

From 2026-10-25 the tick fires at 08:35 UTC and the stamps read +01:00. These lines will then be wrong:

| where | text |
|---|---|
| `.scratch/tender-db/HANDOVER-2026-10-01.md:90` | deploy "not right before the 07:35 UTC daily tick" |
| `HANDOVER-2026-10-01.md:102` | "Journal and nginx timestamps are CEST (UTC+2)." |
| `HANDOVER-2026-10-01.md:133` | "The daily tick runs at **07:35 UTC**" |
| `HANDOVER-2026-10-01.md:189` | start each backfill chunk "≥4 h before a 07:35 UTC tick" |
| `ops/watchdogs/tender-db-tmpsweep.timer:5` | "After the daily tick (07:35 UTC)". Its `OnCalendar` (23:41) is box-local, so the timer itself stays right. |

The box timers (snapshot Sun 05:23, driftwatch 06:41, tmpsweep 23:41) and the in-process 03:10 report tick
(`REPORT_TICK`, `:11770`) are all Berlin-local, so they shift with the daily tick and only the prose drifts. The likely
slips are reading a +01:00 journal stamp as UTC+2, or deploying at about 08:20 UTC straight into the tick. The 245
catch-up and deploy.sh's busy-queue refusal soften the second.

### 2. `docs/operations.md` against the scripts and the box

| line | the doc says | what is true |
|---|---|---|
| 259 | `SECRET=$(cat /root/tender-admin-secret)`, and every "Driving it" curl sends `$SECRET` | The file is an EnvironmentFile with one `TENDER_ADMIN_SECRET=<hex>` line (the same doc, 224–228; on the box `grep -c '^TENDER_ADMIN_SECRET='` prints `1` and the file has 1 line). The header therefore carries the whole `KEY=VALUE`. On the box on 2026-10-01, `GET /admin/jobs` answered **403** with the documented form and **200** with the value stripped the way `deploy.sh:85`, `ops/admin.sh:18` and `tender-db-jobwatch.sh:39` strip it. |
| 29 | `./deploy.sh # deploys main` | `deploy.sh:22` is `REF="${1:-HEAD}"`, and its comment explains why HEAD replaced main (2026-09-02). `deploy.sh:4`, its own header, still says "Pushes the current main". |
| 17, 700, 859, 943 | `/data` is "the 500 GB (Hetzner) volume", or "1 TB since 2026-07-22" | `/data` is `/dev/md3`, 1.7T, 74% used (`df -h`). The public `/health/deep` reports `total_bytes` 1,780,595,036,160. The only block devices are two 1.7T Micron 7450 NVMe drives (`lsblk`), so there is no Hetzner volume. `CONTEXT.md:195` also says "a 1 TB Hetzner volume". |
| 861, 870 | "the 75 GB root disk" | `/` is `/dev/md2`, 120G (75G is the current usage) |
| 75, 778 | "the box's 4 cores", "an 8 GB box" | `nproc` prints 32 and `free -g` shows 62 total |
| 204–206, 614–616 | a box that was down at 09:35 "simply misses that tick; re-drive it by hand" | `catch_up_missed_tick` (`supervisor.rs:11601`, issue 245) runs when the scheduler starts and serves a tick that passed unserved |
| 910–912 | "there is **no copy of the DB anywhere**, on-box or off" | The weekly reflink ring (issue 269, `ops/watchdogs/tender-db-snapshot.sh`) is live. `/data/db/snapshots` holds two files, and the timer last ran Sun 2026-09-27 05:23 CEST with `Result=success`. The DR premise (no off-box backup, 23/170) still holds. Only "no copy anywhere" is wrong, and the heading "there are no backups" needs that qualification. |
| 385 | "this runtime's stderr does not reach journald (issues 61/63)" | It does. Since 2026-10-01 00:00 the `tender-db` journal holds 6 `[project] … phase 2` lines, for example `[project] incremental phase 2: ParsedFold over 19541 planned notices`, which is an `eprintln!` (`crates/ingest/src/project.rs:2394`). Issue 339 was verified from the same kind of line. The doc comments at `supervisor.rs:2338` and `:2412` repeat the claim. |
| 980–1001 | "Running the tests": `cargo test-app`, "`cargo test -p store` and friends already run everything", debuginfo off only "on a tight disk", "83 unit tests" | CLAUDE.md's Testing rules say to run the suites through `ops/check.sh`, and to run a focused test only with the gate's flags AND the gate's package set. A plain `cargo test` builds a second (debuginfo) artifact family, and `-p <crate>` alone resolves features differently (issue 260); both have filled the disk. The 83 is old: 432 records `--lib` at 147/147 on 2026-09-27, and `.cargo/config.toml:13` repeats the 83. |

Also stale, lesser:
- The watcher prose (665, 790–801, 833, 1012–1016) describes one 4-hourly routine, while line 806 of the same doc says
  two hourly routines plus a GitHub Actions check. That prose belongs to 24, which is open and still changing it.
- Lines 1018–1021 say "A public GitHub repo was declined for now", but `zebreus/tender-db` is public (GitHub API
  `"visibility": "public"`, read 2026-10-01).

### 3. The agent rules

- **`docs/agents/prod-box-reads.md:22–23`** says: "Never retry a 408 — … turso cannot interrupt a statement, so each
  retry stacks another uninterruptible scan." **`:108`** (the traps table) says "a capped query is NOT cancelled: it
  pins its worker to completion". Both have been false since 425/438. The vendored SDK exposes `interrupt()`
  (`crates/vendor/turso/src/connection.rs:268`, wired in by the root `Cargo.toml:147–148` `[patch.crates-io]`), and
  `crates/app/src/v1/stop.rs` interrupts a read at its limit and again every 50 ms (`TICK`, `:55`). 425 measured this
  live: a capped `COUNT(*)` answered 408 at 10.02 s, and two seconds later `tender_db_sql_pinned_computations` read 0.
  The rule can stay, but its reason is wrong. The true reason is `v1/sql.rs:66–69`: the interrupt does not reach time
  spent waiting for a reader or a single long instruction, so a retry still spends another full cap. The file's last
  commit, `37214f5` (2026-09-30), changed the cap on line 21 and left line 23 as it was. The same stale reason appears
  in `v1/sql.rs:2262` (the doc of `in_flight_tracks_the_computation_not_the_request`) and in
  `crates/ingest/src/bin/plan-probe.rs:21–22`.
- **`CLAUDE.md:44–50`** says `run_spec` "is a 62-arm async match". Today it has 84 arms (`supervisor.rs:3893–10636`,
  counted as arm-opening `Spec::` lines at the match's indent). The note's only remedy is "Wrap a big arm's body in
  `Box::pin`". 404's section "The stack lesson, because it cost four wrong guesses" (404:815–829) found that this is
  not enough, for three reasons:
  - `Box::pin` constructs the frame on the stack before it moves it, so a boxed arm must also be small.
  - An awaited future is part of the outer future, so the deep awaits need boxing too.
  - The fast way to find the culprit is to replace the arm body with `Err(...)`.

  432 hit the same trap again on 2026-09-27 (432:113–114: "with only the arm's outer `Box::pin`,
  `an_execute_without_an_expected_count_is_refused` overflowed its stack"). Outside 404, the lesson exists only in
  scattered helper comments (`supervisor.rs:3782`, `:3874`, `:12531`). `run_spec` itself has no doc comment, and the
  handover (`:301`) repeats "wrap big arms in `Box::pin`".
- **`CLAUDE.md:92–93`** says data-page reads run "against a snapshot, never the serving DB". `prod-box-reads.md:15–23`
  replaced that rule on 2026-08-06: bounded data-page reads go through `/v1/sql` against the serving DB.

### 4. `/root/aj.sh` exists only on the box

`/root/aj.sh` (505 bytes, mtime Sep 8) is the handover's admin recipe (`HANDOVER-2026-10-01.md:91–95`: `<path>` GETs,
`<path> '<json>'|@file` POSTs, no method word). No file in the repo contains it. 24 issue files call it, including 5
of the 18 open Verify lines that `ops/board-verify.sh` runs: 429, 443, 448, 60 and 63.

The versioned CLI is `ops/admin.sh`. `ops/watchdogs/install.sh:48` installs it as `/usr/local/bin/tender-admin`, and
the box copy matches `/opt/tender-db/src/ops/admin.sh` today. But its call shape is different (`raw <METHOD> <path>`,
with the body on stdin), the handover names it only as "the on-box admin CLI" (`:304`), and no open Verify line uses it.

This is issue 224's shape. Box-only scripts vanished on 2026-08-09, and the watchers were blind for a week.
`ops/watchdogs/README.md` "Why they live in git" is the rule this breaks. If `/root/aj.sh` is lost, those 5 Verify
lines print errors and the handover's recipe stops working.

## Proposed fix

The root cause is that the runbooks hold photographs of facts that the code and the box own: a UTC time, a core count,
a disk size, "no snapshots". They also hold second copies of recipes that already live in a script or in CLAUDE.md:
the secret reader and the test commands. So write each fact in a form that does not rot, or point to its owner, as
`issue-tracker.md` says ("record how to re-take it"). One docs pass, in this order:

1. **DST, before 2026-10-25.**
   - Handover lines 90, 102, 133 and 189, and `tmpsweep.timer:5`: write the tick as "09:35 Europe/Berlin (07:35 UTC
     until 2026-10-25, 08:35 UTC after)".
   - Describe the journal and nginx as "box-local Berlin time: +02:00 now, +01:00 from 2026-10-25". Job rows are
     unix seconds.
   - The code needs no change, and `berlin_offset_follows_the_eu_dst_rule` already pins it.
2. **`docs/operations.md`.**
   - "Driving it" calls `tender-admin` (or `aj.sh`, see 4) instead of carrying its own secret reader. Then the one
     reader that the offline tests pin is the only reader. Where a raw curl stays, read the secret with
     `sed -n 's/^TENDER_ADMIN_SECRET=//p'`.
   - Line 29 and `deploy.sh:4`: the default is HEAD.
   - Hardware: name `/dev/md3` and `/dev/md2` on local NVMe, and put the commands that re-read them (`nproc`,
     `free -g`, `df -h /data /`, `/health/deep` `.checks.disk`) beside any dated number. Fix `CONTEXT.md:195` too.
   - Missed tick: describe the 245 catch-up.
   - DR: "no off-box copy; a weekly on-box reflink ring of two (issue 269)".
   - stderr: it reaches the journal. Keep the job row as the durable surface only if a true reason still stands, for
     example journal rotation. Fix the two `supervisor.rs` doc comments to match.
   - "Running the tests": replace the section with a pointer to CLAUDE.md's Testing section. Drop the 83 here and in
     `.cargo/config.toml:13`.
   - AGPL: the repo is public.
   - Leave the watcher prose to 24.
3. **Agent rules.**
   - `prod-box-reads.md:23` and `:108`: keep "never retry a 408", with the true reason. The interrupt stops the
     statement, but a retry still spends another cap, and the interrupt does not reach reader waits or one long
     instruction. Fix `sql.rs:2262` and `plan-probe.rs:21–22` the same way.
   - CLAUDE.md:
     - drop the arm count ("one async match over every `Spec` arm");
     - add 404's three points;
     - fix lines 92–93 to match prod-box-reads.

     CLAUDE.md is the user's checked-in instruction file, so its edit goes through the user.
   - In any case, put the full lesson as a doc comment on `run_spec`, where the next arm's author will read it.
4. **Put `aj.sh` in git.**
   - Version it as `ops/aj.sh`, honouring `TENDER_ADMIN_URL` and `TENDER_ADMIN_SECRET_FILE` as `admin.sh` does.
   - Have `install.sh` install it to `/root/aj.sh` with mode 0700, beside `tender-admin`. The 24 issue files, the 5
     open Verify lines and the handover then keep working unchanged.
   - Pin it with a `test-watchdogs.sh` case against the existing fixture: "aj.sh GETs /admin/jobs with the stripped
     secret and gets the jobs object", plus the wrong-secret 403 case.
   - 459 is mid-edit in `install.sh`, `test-watchdogs.sh`, `deploy.sh` and `docs/operations.md`, so `git diff` each
     file before staging.

## Verify

    grep -c -E 'SECRET=\$\(cat /root|# deploys main|Pushes the current main|misses that tick|no copy of the DB anywhere|stderr does not reach journald|500 GB (Hetzner )?volume|4 cores|cannot interrupt a statement|capped query is NOT cancelled|62-arm|against a snapshot, never|07:35 UTC daily tick|\*\*07:35 UTC\*\*|before a 07:35 UTC tick|\(07:35 UTC\)|are CEST \(UTC\+2\)' docs/operations.md docs/agents/prod-box-reads.md CLAUDE.md deploy.sh .scratch/tender-db/HANDOVER-2026-10-01.md ops/watchdogs/tender-db-tmpsweep.timer ops/aj.sh 2>&1

The command covers every unit at once, and each file's line shows which unit is still open. The DST unit is the
handover's line and `tmpsweep.timer`'s line.

- **open** (2026-10-01 12:0x UTC): `docs/operations.md:10`, `docs/agents/prod-box-reads.md:2`, `CLAUDE.md:2`,
  `deploy.sh:1`, `.scratch/tender-db/HANDOVER-2026-10-01.md:4`, `ops/watchdogs/tender-db-tmpsweep.timer:1`, and
  `grep: ops/aj.sh: No such file or directory`.
- **done**: every file reads `:0`, and `ops/aj.sh:0` replaces the "No such file" line. A correct rewrite matches no
  pattern; for example, "09:35 Europe/Berlin (07:35 UTC until 2026-10-25, 08:35 UTC after)" passes. If a newer
  `HANDOVER-*.md` replaces the 2026-10-01 one, point the command at the new file.
