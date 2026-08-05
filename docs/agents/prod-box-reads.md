# Reading the production box

When may an agent read something on the prod box without asking first? This is the team rule, adopted
2026-08-04. It replaces an earlier boundary that two agents were reading differently in practice.

## The rule

**Safe by construction = metadata AND bounded.** Both, not either.

|                | bounded                                                                        | unbounded                                                                                         |
| -------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| **metadata**   | `df`, `stat`, `systemctl`, `xfs_info`, `sqlite_master`, a one-row `sqlite_stat1` read — **free, no permission needed** | `filefrag -v`, `du` over a large tree, unlimited `journalctl`, `ls` on a huge directory — **bound it explicitly, or treat it as a data read** |
| **data pages** | —                                                                              | table scans, `integrity_check`, `dbstat`, any timing or characterisation run — **gate on the team lead's word; snapshot only, never the serving DB** |

A data-page read runs against a **checkpointed snapshot**, never the live database, because it competes
for the one disk with the writer.

## The one live-database exception: provably O(1) page count

A read of the **live** database is permitted — despite touching data pages — if and only if its page
count is **provably O(1) in corpus size**: a b-tree point or first-row probe (`EXISTS(SELECT 1 …)`, an
indexed equality, `SELECT 1 … LIMIT 1`) whose `EXPLAIN QUERY PLAN` shows a **seek or single-row access,
never a SCAN**. Such a probe touches roughly tree-depth pages whether the table holds one row or a
hundred million, so it cannot evict the working set or contend for the disk.

Three conditions, all of them load-bearing:

- **The test is asymptotic, not numeric.** "It only took 34 ms" is the magnitude reasoning this whole
  document exists to replace. The claim being made is about *growth*, and it has to be verified per probe
  rather than assumed from a timing.
- **Plan-verify every probe.** Not the pattern, each probe. An `EXISTS` on an unindexed column is a
  full scan wearing an `EXISTS`'s clothes.
- **A SCAN is never admitted, however small its result.** The dangerous case is *matches nothing*: an
  unindexed `EXISTS` that finds no row has walked the entire table to establish it. Small output is not
  evidence of small work — that is the same inversion as a fast first page hiding an expensive later one.

Any probe that cannot be shown to seek stays snapshot-side.

*(Ruled by team-lead, 2026-08-05, for issue 28's liveness tier. Argued once, written here, narrow.)*

**Note what this exception does NOT cover, because it is easy to over-apply.** It governs an **agent**
reading the live database. **The app reading its own database in-process was never governed by this rule
at all** — that is the service doing its job, not an operator taking a look. So an assertion inside the
projection or behind `/health` needs no exception; the right question there is a design one (where does
the check belong) rather than a permissions one. The exception is worth having on its own merits, for the
agent case; it is not what makes an in-app check legitimate.

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
- Data pages: ask the team lead, name the snapshot, state its age, and run it in a low-traffic window.
  Someone with box access runs it; the requester does not need to be that person.

Both failure directions are real. Being too loose does not announce itself — nobody tells you about the
read that quietly hurt. Being too strict produces beliefs that survive because nobody was permitted to
check them; that has cost us a finding left unverified while two people re-derived it. The rule exists so
neither is a judgement call.

---

*Adopted from a joint proposal: the category framing and the impact-vs-success axis are proj-fix's, the
metadata-**and**-bounded correction is sdk-vendor's, and it was run-driver's one-row `sqlite_stat1` read —
which caught a false premise in a design — that the rule had to keep permitted.*
