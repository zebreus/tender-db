# 135 — a snapshot that crashes mid-copy leaves a partial file in the ring, permanently

Status: closed-obsolete (2026-08-09, orchestrator) — voided by `39c0e08` (owner decision, 2026-08-06):
the snapshot feature is removed, ring deleted, `backup.rs` gone. The copy-then-rename mechanism this
asked for has no code to land in. If snapshots ever return, the fix direction below stands.
Originally: latent, found while checking the pointer against sdk-vendor's half-written-snapshot
finding (`1b3564c`). Not observed in the wild; the mechanism was in the code.
Kind: data integrity / ops
Owner: proj-fix (store code)
Relates to: 23 (snapshots), 28 (the gate that would read it), 107, 119

## The mechanism

`Db::snapshot` (`store/src/backup.rs:83`) copies to `dest` **directly — no temp name** — and only
removes the file when a check *fails*:

- copy short → `remove_file(dest)`, error;
- `integrity_check` not ok → `remove_file(dest)`, error.

Both are the *verify* path. **A crash during the copy — OOM, `kill`, power loss, a killed job — runs
neither**, and leaves a partial file at a perfectly valid snapshot name (`tender-db-<unix>.db`).

Nothing ever removes it:

- **`prune` counts it as a ring member.** It matches `PREFIX`/`SUFFIX`, so on a `keep=2` ring one crash
  can leave the ring holding one real snapshot and one corpse — halving the retention nobody knows has
  halved.
- **`newest` selects it forever after**, since its name sorts newest.
- The **pointer skips it** (no pointer is written for a run that never returned `Ok`), so *pinned*
  consumers are unaffected — which is why this was invisible while looking at the pinned path.

## Why it is worth fixing rather than detecting

sdk-vendor's size check (`1b3564c`) catches it **on read**, which is the right defence for a consumer
that resolves by glob. But the corpse persists, keeps occupying a ring slot, and every future reader pays
the check to reject the same file.

The structural fix is the one `publish_latest` already uses one layer up: **copy to a temp name in the
same directory, then `rename`.** The final name then only ever exists complete — a crash leaves
`.tender-db-<unix>.db.tmp`, which `prune`'s prefix match ignores, and which a later run can clear.

That also removes the *original* hole at its source: with a temp name there is no window in which a
mid-write file is visible under a valid snapshot name, so `ls | sort | tail -1` cannot select one and the
size check becomes a belt-and-braces second line rather than the only line.

## Caveat on the fix

`rename` must be within the same filesystem — the temp file goes in the snapshot dir, not `/tmp` (that is
exactly the issue-83 trap: `/tmp` is a tmpfs and the DB is on `/data`).

And the copy is ~455 GB, so the temp file needs the same headroom as the final one — no change to the
disk budget, since the file was already being written at full size under its final name.

## Not yet established

Whether a corpse exists on prod right now. `keep=2` with a daily cadence means one would have to have
been created since the ring last cycled past it; the two current snapshots are both plausible sizes, so
probably not — but "probably" is doing work there and the size check would settle it in seconds.
