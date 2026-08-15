# Reading the production box

When may an agent read something on the prod box without asking first? This is the team rule, adopted
2026-08-04. It replaces an earlier boundary that two agents were reading differently in practice.

## The rule

**Safe by construction = metadata AND bounded.** Both, not either.

|                | bounded                                                                        | unbounded                                                                                         |
| -------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| **metadata**   | `df`, `stat`, `systemctl`, `xfs_info`, `sqlite_master`, a one-row `sqlite_stat1` read — **free, no permission needed** | `filefrag -v`, `du` over a large tree, unlimited `journalctl`, `ls` on a huge directory — **bound it explicitly, or treat it as a data read** |
| **data pages** | indexed seeks / single-table aggregates via `/v1/sql` — **gate on the team lead's word, per read** | table scans of the corpus, `integrity_check`, `dbstat`, any timing or characterisation run — **no on-box path exists; see below** |

**Updated 2026-08-06 — the snapshot ring no longer exists** (owner decision: the feature is removed,
storage). The old prescription "data pages run against a checkpointed snapshot" is unfollowable, and an
unfollowable rule gets reasoned past under pressure. The replacement, decided rather than left as a gap:

- **Bounded data-page reads** (an indexed seek, a single-table aggregate — the shape the 84-remainder
  attribution ran) go **through `/v1/sql` against the serving DB**, on the team lead's word per read,
  in a low-traffic window. The endpoint's own bounds are load-bearing: one bare SELECT, 10 s cap,
  allow-listed tables, the Class B shed. Never retry a 408 — the cap bounds your wait, not the work;
  turso cannot interrupt a statement, so each retry stacks another uninterruptible scan.
- **Heavy/unbounded data-page reads** (corpus-scale scans, characterisation runs) have **no compliant
  on-box path at all** now. Do not construct one ad hoc. If one is genuinely needed, that is an owner
  conversation (a purpose-built copy, off-box hardware) — bring the need to the team lead, who brings
  it to Lennart.
- The **archives** (immutable tars) are the durable artifact for member-level questions — reading them
  is a bounded I/O job, gated like any data read.

**Scope: this rule governs an AGENT reading the prod box.** The app reading its own database in-process
is not covered by it at all — that is the service doing its job, not an operator taking a look. So an
assertion inside the projection, or behind `/health`, raises no question here; the question there is a
design one (where does the check belong) rather than a permissions one. Stated because the confusion is
live: a live-DB carve-out was proposed, granted and withdrawn on the assumption that an in-app check
needed one. It never did.

## Why category, and not size

The tempting rule is "small reads are fine". It is wrong, and specifically it is wrong in the way that
already cost us an incident: the 2026-08-03 failure was a **misjudgement of magnitude** — someone
reasoned that a load run was small enough and was not. A rule that asks anyone to estimate magnitude
reintroduces the exact failure mode it exists to prevent, because *"it's obviously fine"* is the
reasoning the rule is there to distrust.

Category and boundedness are checkable without estimating anything. `df` is constant-time regardless of
database size; a scan of `quarantine` is proportional to a table with millions of rows whatever anyone
believes about it.

The unbounded-metadata cell is the one that is easy to miss, and it is not a footnote: `du -sh
/data/archive` walks 178 GB of archive inodes, and an unlimited `journalctl` is the same shape. Both look
like harmless metadata. Bound them (`head`, `--since`, a narrow path) and they return to the free cell.

## This is not a new convention

It promotes a distinction the codebase already relies on. From
`.scratch/tender-db/canonical-verify/hot_read_plans.sh`:

> READ-ONLY and METADATA-ONLY. Reads `sqlite_master` and compiles query plans. It never executes a data
> query, so it touches no data pages — unlike every other gate in this directory it is safe to run
> against the serving DB.

That gate is trusted and its safety argument is exactly this line.

## Impact and success are independent axes

This rule governs **prod impact**. It says nothing about whether a read will *succeed*.

A metadata read against the live database can still fail — stock `sqlite3` cannot read a turso-written
WAL, and returns `database is locked` regardless of `.timeout`. That is a *reliability* property, not a
safety one, and the two must not be conflated in either direction:

- reading a lock failure as *"that read was dangerous"* wrongly tightens the rule;
- reading a successful heavy scan as *"that read was fine"* wrongly loosens it — and that is the
  2026-08-03 failure exactly. Getting away with something once is evidence-shaped nothing.

## In practice

- Free-cell reads: just run them. Inspecting the box must not require a negotiation, or nobody inspects
  the box.
- Unbounded metadata: bound it, and say how in the write-up.
- **`journalctl -b` is not "since this server started" — it is since the machine booted**, and the
  service restarts on every deploy without the machine rebooting. So `journalctl -b | grep '[store]
  reclaim stamped NO ledger'` returns the *accumulated* count across every instance since the last
  machine boot — 516,144 lines on 2026-08-15, all from the Aug 12–14 reclaim-all campaign (issue 139's
  log signature), none from the running server. A raw count there both false-alarms and can bury a
  genuinely new line. For the OPERATE reclaim check, scope to the current instance:
  `journalctl -u tender-db _PID=$(systemctl show tender-db -p ExecMainPID --value)` or
  `--since "$(systemctl show tender-db -p ActiveEnterTimestamp --value)"`. A non-zero count *there* is
  the real finding.
- Data pages: ask the team lead, state the query and its bound, and run it through `/v1/sql` in a
  low-traffic window (no snapshots exist to name any more). Someone with box access runs it; the
  requester does not need to be that person.

Both failure directions are real. Being too loose does not announce itself — nobody tells you about the
read that quietly hurt. Being too strict produces beliefs that survive because nobody was permitted to
check them; that has cost us a finding left unverified while two people re-derived it. The rule exists so
neither is a judgement call.

---

*Adopted from a joint proposal: the category framing and the impact-vs-success axis are proj-fix's, the
metadata-**and**-bounded correction is sdk-vendor's, and it was run-driver's one-row `sqlite_stat1` read —
which caught a false premise in a design — that the rule had to keep permitted.*
