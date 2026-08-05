# Mapping the 41 count-free checks to incremental assertions — and what does not survive

Working doc for proj-fix's in-app verification. Owner: sdk-vendor (check design), proj-fix (build).
Supersedes the batch-snapshot gate, killed by two independent constraints: turso's hash cliff on
high-cardinality GROUP BY/DISTINCT, and `Db::open()` mutating a snapshot (schema-migrates, no
read-only mode, a bare connect writes a WAL).

## The structural gap, stated first because it changes what the layer can claim

**An assertion at projection time verifies what projection WRITES. It is blind to anything that
happens to the layer while projection is not running.**

That is not a quibble — it is the exact catastrophe #28 was created for. From issue 121:

> The prenuke has residual value ONLY because we do not continuously verify the tender layer: a
> latent salvage re-nuke could corrupt the live layer, and the 2-deep snapshot ring would faithfully
> copy the corruption before anyone noticed.

A re-nuke is `DROP TABLE` + `CREATE TABLE` — **DDL, committed immediately, with no projection write
to assert against.** The same applies to a killed rebuild, an external mutation, or disk corruption.
An incremental assertion cannot fire for any of them, because nothing calls it.

Concrete example of an invariant that can be violated with no write to hook: **`orphan_versions`**
("every version belongs to a tender"). Asserting it when a version is written is easy and correct.
But the invariant breaks later if a *tender* is deleted — no version is written at that moment, so
no assertion runs, and the layer silently acquires orphans.

**So the honest claim is narrower than the old gate's:** incremental assertion catches *projection
writing something invalid*; it does not catch *something else breaking what projection wrote*. The
second needs a live periodic check — which is cheap for the presence tier and is the part worth
keeping (proj-fix's issue 133 is already half of it).

## Three destinations, not one

| destination | what it suits | cost |
|---|---|---|
| **(I) incremental assert** at write time | invariants over a row/tuple being written, checkable from data already in hand | ~free |
| **(L) live periodic** check on the serving DB | `EXISTS`-shaped and single-table predicates; **the only thing that catches an emptied or externally-damaged layer** | O(1)–one scan |
| **(X) lost** | needs a corpus-scale join, no write to hook, cliffs on turso | — |

## The mapping

**Presence tier — 13 checks → (L), and they are the load-bearing ones now**

`present_tenders`, `present_versions`, `present_orgs`, `present_mentions`, `present_texts`,
`present_amounts`, `present_parties`, `present_winners`, `present_lots`, `present_lot_results`,
`present_lot_result_rows`, `present_bid_rows`, `present_changes`

Each is `SELECT EXISTS(SELECT 1 FROM t)` — **O(1), no cardinality, no cliff, no snapshot.** They run
against the live DB in milliseconds. This tier detects the nuked/emptied layer, which is precisely
what incremental assertion cannot see. **If only one tier survives, it is this one.**

**Tier A single-table — 15 checks → mostly (I), some also (L)**

| check | form |
|---|---|
| `identity_overlap`, `no_head`, `kind_bad`, `source_bad`, `islands_multi`, `min_seq_is_1` | **(I)** — all decidable from the tender row being written |
| `provisional_identifier`, `identifier_kind_bad` | **(I)** — decidable from the organization row |
| `changes_op_bad`, `changes_kind_bad` | **(I)** — decidable from the change row |
| `absurd_pubdate` | **(I)** — decidable from the version row |
| `negative_amount`, `negative_awarded`, `negative_bid` | **(I)** — decidable from the amount/bid/result row. **Note:** these are the #36/#37 surfaces, and per issue 136 the values are source-published, so the assertion must *flag*, not reject — a hard assert here would refuse legitimate (if implausible) source data and stall projection |
| `versions_ge_tenders` | **(L)** — a cross-table count comparison; cheap as two counts, no join |

**Tier B joins/anti-joins — 11 checks → the hard cases**

| check | destination | why |
|---|---|---|
| `head_pub_mismatch`, `head_not_max` | **(I)** | both sides are in hand when the head pointer is written — `head_not_max` is literally #27, already proven as a projection-time assert |
| `orphan_texts`, `orphan_amounts`, `orphan_parties` | **(I)** partial | assertable when the satellite row is written (its parent version must exist). **Blind to later parent deletion.** |
| `orphan_versions`, `orphan_lots`, `orphan_lot_results` | **(I)** partial | same shape, same blindness — see the example above |
| `orphan_party_orgs`, `orphan_winner_orgs` | **(I)** partial | assertable at write; blind to later org deletion |
| `org_identity_dupe` | **(I)** | a uniqueness invariant — better still, a **UNIQUE index** makes it unrepresentable rather than asserted. Prefer structure over a check (this file's own rule) |
| `junk_hub` | **(X)** likely | "one notice causes a version in exactly one tender" is a GROUP BY over notices with a HAVING — the exact high-cardinality shape documented as cliffing. No natural write to hook: the violation emerges across *two* tenders written at different times |

**Tier C — 1 check → (X)**

`orphan_mention_orgs` — a ~30M-row `organization_mentions` anti-join. No write-time hook that covers
it (a mention's org can be deleted later), and the largest cliff risk. **Lost unless a FK makes it
unrepresentable**, which is the better fix if the schema allows it.

## Summary of what changes

* **13 presence + `versions_ge_tenders` → live periodic.** Cheap, and now the *only* detector of an
  externally damaged layer. This tier gets *more* important, not less.
* **~24 checks → projection-time assertions.** Genuinely better than a daily batch: they fire at the
  moment of the bad write, with the offending row in hand, instead of up to 24 h later.
* **~10 orphan checks → partial.** They catch bad writes and miss later deletions. **That gap should
  be written down wherever the layer's coverage is claimed**, not discovered during an incident.
* **2 checks (`junk_hub`, `orphan_mention_orgs`) → lost**, unless expressed as constraints.

**The recurring better answer is a constraint, not an assertion.** `org_identity_dupe` as a UNIQUE
index and the orphan family as foreign keys make the violations *unrepresentable* — no check to run,
no cliff, no blind window, nothing to keep honest. Where the schema permits it, that beats every
option above, and it is the same move as the fail-closed argument contract: prefer structure over a
guard wherever the structure exists.

## What carries unchanged

`blind` ≠ `red` (now: could-not-assert → a failed job, #32), state-not-event verdicts, the presence
set that closes the vacuity hole, and both-ways self-tests that exercise the **path** and not only
the predicate.
