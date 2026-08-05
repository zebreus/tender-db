# Runbook — deploying `8c8182a` (issue 84 backfill)

Written 2026-08-05 by run-driver, who gated it and then **declined to execute it**
rather than start a prod migration he could not supervise through post-checks. This
exists so the next executor runs a procedure instead of reconstructing one under
pressure. Everything below is verified, not remembered.

## State at handoff

| | |
|---|---|
| deploy SHA | **`8c8182a`** (frozen; `af68dff` is docs-only on top, code identical) |
| code commits | `907d830`, `ba95b60`, `80e2365`, `a9265fb`, `8c8182a` |
| build subject | **`/tmp/rd-deploy-final`** — dedicated checkout, detached at `8c8182a`, verified clean |
| canonical gate | **GREEN** — 42 suites, **320 passed / 0 failed**, 0 compile errors, run completed |
| gate command | `cargo test --workspace --features tender-db/server --no-fail-fast` |
| scope review | **CLEAR** at `8c8182a` |
| staged? | **NO.** Nothing is staged. Start from the build. |

**Do not build from the shared worktree.** It was dirty on three separate looks —
teammates edit it continuously. Build from `/tmp/rd-deploy-final`, or a fresh
checkout of `8c8182a`, and verify with a **fail-closed** check:

```sh
git status --short crates/ | grep -q . && { echo REFUSING: dirty; exit 1; }
```

*(Not a `git status` followed by a hopeful `echo "clean"` — that is a label, not a
check, and it let a gate start on a dirty tree earlier today.)*

## `deploy.sh` is NOT two-phase — this is the decomposition

`./deploy.sh <ref>` pushes, builds, **switches the symlink and restarts in one run**.
There is no stage/confirm flag. To get the two-phase treatment the commands below
split it at the natural seam: the build writes only `app-result`, so **nothing is
live until the `mv -T`**. A failed build cannot touch the symlink.

### Phase 1 — stage (safe, reversible, no service impact)

```sh
cd /tmp/rd-deploy-final
git status --short crates/ | grep -q . && { echo REFUSING: dirty; exit 1; }
REV=$(git rev-parse HEAD)            # expect 8c8182a…
git push root@zebreus.click:/opt/tender-db/repo.git HEAD:refs/heads/main -f

ssh root@zebreus.click '
  set -e
  cd /opt/tender-db/src
  git fetch origin main && git checkout -f main && git reset --hard '"$REV"'
  git rev-parse HEAD
  nix build /opt/tender-db/src#tender-db -o /opt/tender-db/app-result --print-build-logs
  readlink -f /opt/tender-db/app-result
'
```

**STOP. Confirm here.** The build is done and production is untouched — the running
service still points at the old store path. This is the whole value of the split.

### Phase 2 — swap (the irreversible-ish step; ~seconds)

```sh
ssh root@zebreus.click '
  set -e
  STORE_PATH=$(readlink -f /opt/tender-db/app-result)
  ln -sfnT "$STORE_PATH" /opt/tender-db/app.new
  mv -T /opt/tender-db/app.new /opt/tender-db/app        # atomic
  echo '"$REV"' > /opt/tender-db/deployed-rev
  install -d /etc/systemd/system/tender-db.service.d
  printf "[Service]\nEnvironment=COMMIT_SHA=%s\n" '"$REV"' \
    > /etc/systemd/system/tender-db.service.d/rev.conf
  systemctl daemon-reload
  systemctl restart tender-db
'
```

## Post-checks — all must pass

```sh
ssh root@zebreus.click '
  systemctl show tender-db -p ActiveState -p MainPID -p NRestarts --value | tr "\n" " "; echo
  curl -s -o /dev/null -w "health %{http_code} %{time_total}s\n" -m 10 http://127.0.0.1:8080/health
  cat /opt/tender-db/deployed-rev
  journalctl -u tender-db --since "3 min ago" --no-pager -o cat | grep -iE "column|migrat|error|panic" | head
'
```

* `ActiveState=active`, `/health` **200**
* **`MainPID` changed** — this is the restart proof, and the ONLY one. Record the old
  and new values.
* **`NRestarts` stays 0.** It counts *failure* auto-restarts, not a clean
  `systemctl restart`. An earlier draft of this file said "0 → 1 expected", which is
  wrong in the dangerous direction: seeing 0 and believing the restart had not
  happened would prompt a second, unnecessary restart. **0 is correct.** Confirmed on
  the 2026-08-05 deploy — `MainPID` 3646386 → 4051862, `NRestarts` 0 throughout.
  Identity, not state — the same rule the abort firing-test uses.
* `deployed-rev` = `8c8182a…`
* **The two nullable `ALTER TABLE quarantine ADD COLUMN`** (`skipped_at`,
  `skipped_reason`): must be a **silent no-op-or-add**. O(1) — SQLite/turso
  `ADD COLUMN` with no default rewrites no rows, so it does not touch the 455 GB.
  **Anything else in the boot log — a long pause, an error, a row-rewrite — STOP and
  roll back.** This is the first schema change to `quarantine` in a while and it has
  only been exercised on scratch DBs.

**Expected and NOT a regression:** the negative-amount check now reports **51**, not
17,738 (proj-fix's `run_light` 3.7 re-spec; issue 132 closed — the 51 satisfy P4
individually and are upstream data).

## Rollback

Additive nullable columns are ignored by the old binary and the new op is dry-run by
default, so rollback is a **binary revert** — no data undo:

```sh
ssh root@zebreus.click '
  ln -sfnT <PREVIOUS_STORE_PATH> /opt/tender-db/app.new
  mv -T /opt/tender-db/app.new /opt/tender-db/app
  systemctl restart tender-db
'
```

Capture `readlink -f /opt/tender-db/app` **before Phase 2** — that is
`<PREVIOUS_STORE_PATH>` and there is no other record of it once the symlink moves.

## Sequencing

* **Not under an active Tier B/C during-window.** The restart is the perturbation that
  confounds a confinement measurement. Check first — and include `activating`, since a
  `--service-type=oneshot` unit sits in that state for its whole run:

```sh
ssh root@zebreus.click "systemctl list-units 'tdb-*' --state=active,activating,deactivating --no-legend"
```

* Coordinate with sdk-vendor; tell them when the restart lands, because it **empties
  the live page cache** and their Tier C numbers won't compare to B's warm baseline.

## After the deploy

proj-fix fires the dry-run (`{"kind":"mark-skipped-siblings"}` — dry-run is the
default; an omitted flag cannot write). **Both numbers go to team-lead:**

* **593,010** would be marked
* **0** rejected by the sibling guard, now reported as two populations — *no English
  original at all* (a fetch/ingest gap) vs *original held but unparsed* (a parse failure)

**A shortfall is a data-loss finding, never a reason to widen the predicate** — those
are exactly the rows that must not be marked, and execute now refuses on its own when
gaps > 0, with no override.
