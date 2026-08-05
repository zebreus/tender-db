# Mapping the standing checks to incremental assertions (task #28, post-pivot)

Team-lead's decision: verification moves from a batch pass over a snapshot to **assertion at projection
time**. This maps every check in `standing_gate.sh` to its incremental form, and raises the one that has
none.

**The headline is that most of them should not be assertions at all.** Two-thirds are single-row
predicates or uniqueness constraints, and the engine can make those **unrepresentable** rather than
detected — which is the ledger's own conclusion from entry #9: *prefer structure over a guard wherever
the structure exists*. An assertion is a guard that must be remembered, run, and read; a constraint is
none of those.

## 1. Single-row predicates → `CHECK` constraints (12 checks)

| check | constraint |
|---|---|
| `kind_bad` | `kind IN ('procedure','registration')` |
| `source_bad` | `source IN ('ted','doe')` |
| `identifier_kind_bad` | `identifier_kind IS NULL OR identifier_kind IN ('vat','national')` |
| `changes_op_bad` | `op IN ('added','changed','removed')` |
| `changes_kind_bad` | `entity_kind IN (…)` |
| `absurd_pubdate` | `published_at BETWEEN 631152000 AND 1800000000` |
| `identity_overlap` | `(procedure_key IS NULL) <> (island_notice_id IS NULL)` |
| `provisional_identifier` | `(provisional = 1) = (identifier IS NULL)` |
| `islands_multi` | `island_notice_id IS NULL OR current_seq = 1` |
| `negative_amount` | `cents >= 0 OR field = 'result_value'` |
| `negative_awarded` / `negative_bid` | pending #37's re-spec — the *form* is a CHECK either way |

**Two of these were classified as cross-row and are not.** `identity_overlap` compares two columns of the
same row; `org_identity_dupe` (below) is a uniqueness property. Neither needs an aggregate, and treating
them as if they did would have bought a running counter for something a constraint enforces for free.

## 2. Uniqueness → a partial `UNIQUE INDEX` (1 check)

`org_identity_dupe` — *org identity unique among identified profiles* — is
`UNIQUE(country, identifier_kind, identifier) WHERE identifier IS NOT NULL`. Structure, not assertion.

## 3. Per-tender invariants → end-of-batch assertion, the #27 pattern (4 checks)

`no_head`, `head_not_max` (**already built**, #27), `head_pub_mismatch`, `min_seq_is_1` (per-tender form:
each chain starts at seq 1). Scoped to the Tenders the batch rewrote, inside the write transaction, so a
violation rolls back rather than lands — exactly `assert_heads_match`.

`no_head` specifically **cannot** be a CHECK: `current_seq` is legitimately NULL between the tender INSERT
and the head UPDATE, and a CHECK fires per statement. It has to be a transaction-scoped assertion.

`versions_ge_tenders` is subsumed: if every tender has a head and the head names an existing version, the
count relation follows. Not a separate check.

## 4. Referential integrity → end-of-batch assertion, NOT foreign keys (9 checks)

All the `orphan_*` checks are FK-shaped, and **FKs are unavailable where it matters: the projection runs
with `set_foreign_keys(false)`** for fold throughput. So the structural answer is closed off by an
existing, deliberate decision, and these become batch-scoped assertions over the rows just written —
bounded by the batch, seeking by the same keys the writes used.

Worth stating plainly because the obvious move is to "just turn FKs back on", and that is a throughput
decision made elsewhere for reasons that still hold.

## 5. `junk_hub` → per-write lookup, needs an index

*One notice causes a version in exactly one tender.* Incremental form: when writing a version caused by
notice N, assert no **other** tender already has one. A seek, not an aggregate — but it needs an index on
`tender_versions(caused_by_notice_id)` to be O(1) rather than a scan per row. **Check the plan before
building it**, at corpus scale: this is precisely the issue-80 shape (a per-row probe inside a
full-corpus loop that must seek).

## 6. RAISED: `present_*` has no incremental form (13 checks)

**"Table non-empty" cannot be asserted incrementally.** An incremental run touches only its own rows and
cannot observe that a table is empty — and a *rebuild* legitimately empties the layer at its start, so
even a batch-scoped assertion would fire on correct behaviour (the relative-not-absolute trap, issue 133).

This is the check that matters most: it is the **vacuity guard**, the thing that catches a nuked or
emptied layer, and the entire reason the presence set exists. Every other check returns 0 on an empty
table and reports green.

**It is not lost — it belongs to issue 133**, the live-layer emptied detector: an O(1) `EXISTS` per table
against the **live** DB, in-process, no snapshot, no cache cost, surfaced as state and alarming on
transition-to-empty. That was already designed and is unaffected by the pivot. But it must be built, or
the incremental scheme has no vacuity guard at all — every assertion above is conditional on rows
existing to assert about.

**So: #133 is now load-bearing for #28 rather than a follow-up.**

## Summary

| disposition | checks |
|---|---|
| `CHECK` constraint | 12 |
| partial `UNIQUE INDEX` | 1 |
| end-of-batch assertion (#27 pattern) | 13 (4 per-tender + 9 referential) |
| per-write seek + index | 1 |
| **no incremental form → issue 133** | **13 presence checks** |

The `blind` concept survives unchanged: an assertion that cannot run is not an assertion that passed.
