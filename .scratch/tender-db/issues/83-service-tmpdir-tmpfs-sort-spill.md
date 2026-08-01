# 83 — service TMPDIR is tmpfs (RAM) → large external sorts spill into RAM and fail with ENOSPC

Status: open — ROOT-CAUSED 2026-08-01 (run-driver). PROD PATCHED via machine-local drop-in (`/etc/systemd/system/tender-db.service.d/tmpdir.conf` → `TMPDIR=/data/tmp`, `ReadWritePaths=/data/tmp`). REPO FIX DEFERRED: the source is `nix/module.nix:135 PrivateTmp = true` (puts /tmp on tmpfs); the canonical fix must set `TMPDIR` (+ `ReadWritePaths`) to a disk-backed path matching where the DB lives on the deployment (`/data`) — confirm the `stateDir`↔`/data` mapping first so the nix change doesn't mis-map the spill or break the sandbox. Land with the issue-82 deploy after the recovery verifies.
Kind: reliability / ops-correctness
Blocked by: —
Relates to: 82 (the index build that exposed it), 62/63 (large sorts in the projection/index builds), coverage refresher scans

## Root cause

The systemd unit runs with `PrivateTmp` (or a default) that puts `/tmp` on **tmpfs = RAM**, ~3.8 GB total.
turso's external sort (used by large `CREATE INDEX` builds and by big scan/aggregate paths) spills to a
`tursodb_temp_file` under `TMPDIR`. On a 254–441 GB database, the spill is many GB → it fills the 3.8 GB
tmpfs → `error=I/O error (pwrite): no storage space` → the sort/index build FAILS.

This — not `/data` (which had 180 GB free throughout) — is the real cause of the
`02:28:43 ERROR turso_core::io::unix: error=I/O error (pwrite): no storage space` seen during the recovery
rebuild's index-build window, and of the missing `tenders_current_published` index never getting built at
boot ([[82-rebuild-drops-tenders-current-published-index]]).

## Evidence (2026-08-01, run-driver)

Boot index build: PrivateTmp usage climbed +180 MB/min toward the 3.8 GB tmpfs cap (~10 min to full) while
`/data` stayed at 180 GB free; the index build (6.3 GB read, external-sort spill) could not fit.

## Fix

Point the service `TMPDIR` at disk-backed storage with real headroom (`/data/tmp`, 180 GB free) via a
persistent systemd drop-in, e.g.:

```
[Service]
Environment=TMPDIR=/data/tmp
```

(applied as the workaround). Make it canonical in the deploy config, ensure `/data/tmp` exists + is owned by
the service user, and confirm `PrivateTmp` isn't silently redirecting it. Any full-corpus sort (index builds,
coverage refresher's multi-million-row scans, ad-hoc analytics) then spills to disk, not RAM.

## Validation

A large `CREATE INDEX` over the full tenders/notices layer completes without ENOSPC; `du` on `/data/tmp`
shows the spill landing there during the build; tmpfs usage stays flat. Regression: assert TMPDIR is set +
disk-backed at service start.

## Note

This is a latent trap for ANY large sort on this box, not just the rebuild — the coverage refresher's
scans and future analytics would hit it too. Worth folding into the standing ops/deploy config, not just a
one-off drop-in.
