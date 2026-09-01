# 329 — Unfoldable duplicate identities, and whether `canonical_key` should get a DE:vat arm

Status: NEEDS-TRIAGE 2026-09-01 — census BUILT, not yet run on prod.
Kind: measurement / identity semantics (organization layer)
Relates to: 328 (which created the visible duplicates and whose wrong claim
opened this), 300 Stage 2 (R2, the arm that would consume any new key), 316 (the
generic-name denial this borrows), 312 (the same "looks untidy vs measured
false-merge rate" question)
Blocked by: nothing

## Where it came from

Issue 328's label-prefix repair moved 5,309 rows onto the identifier the
publisher actually meant. **3,215 exact duplicate `(DE, vat, DEnnnnnnnnn)`
triples now stand** — `DE329214156` is held by three "Die Autobahn GmbH des
Bundes" rows. I claimed R2 would fold them. It will not:
`crosswalk::canonical_key` has no German arm at all, pinned by its own
must-NOT panel:

```rust
// DE has no cross-walk at all — court-scoped registers.
assert_eq!(key(Some("DE"), "vat", "DE136695976"), None);
```

So German rows were never E1-keyed, before the repair or after. The duplicates
are not a backlog R2 will get to — they are structurally invisible to every
merge arm.

## The decision, and why it is not an armchair call

**For a DE:vat arm**: `DE:vat` is a HARD scheme, the checksum is strong, and
exact triples with matching names are as clean a merge signal as this corpus
offers.

**Against**: a German VAT number can be shared across an **Organschaft** — a
fiscal unity of legally distinct companies — which is exactly the false-merge
shape the CZ699 group-VAT negative already guards against. And the pinned
negative's stated reason (*court-scoped registers*) is about `HRB`, not VAT, so
the two halves of that assertion may deserve different answers.

Disagreeing names are the Organschaft signature. So: count them.

## What was built

`duplicate_identity_census` (store) + the `duplicate-identity-census` job
(read-only, stoppable, stores a report). It walks standing
`(country, kind, identifier)` triples, keeps those held by more than one org
row, and splits them:

* **keyed** — `canonical_key` returns a key. R2 already sees these; whatever
  stands is its denial stack's business, not this census's.
* **unkeyed** — no key, no candidate, no plan. These are the class.

Every unkeyed group then gets a name verdict over its distinct **N3 keys**
(legal-form family abstracted, so `AG` / `Aktiengesellschaft` is one key):

| verdict | meaning |
| --- | --- |
| `agree-distinctive` | one name, and it is not generic ⇒ the clean fold signal |
| `agree-generic` | one name, but over the stoplist cap ⇒ agreement nobody chose to make (issue 316) |
| `contained` | one name's tokens inside the other's ⇒ **UNDECIDED** |
| `disagree` | distinct cores ⇒ the Organschaft signature |
| `unnamed` | every member nameless ⇒ abstain (issue 257's class) |

Deliberately **not** DE-only: `unkeyed_by_scope` names every `(country, kind)`
pair holding unfoldable duplicates, so the next arm is chosen from the corpus
rather than from whichever country came up in conversation — the discipline
`VAT_SUFFIXES` had to be chosen under.

### Three things the build got wrong first, recorded because each is a premise

1. **`contained` is not a safe bucket.** The first draft's comment said "a
   branch or department of one entity, not two entities". That is wrong: an
   *Organschaft subsidiary* is named exactly this way, because a fiscal unity is
   a parent plus companies named after the parent with a qualifier
   (`Muster Holding GmbH` / `Muster Holding Immobilien GmbH`). Nothing in a name
   separates a branch from a subsidiary. The bucket exists so that **neither**
   answer is asserted — counting it as agreement overstates the signal, counting
   it as conflict overstates the risk. Pinned by
   `containment_is_a_third_bucket_and_not_a_licence_to_fold`.
2. **A contiguous-token-window containment test read half the class as
   conflict.** German publishers put the qualifier on either end
   (`x GmbH Niederlassung Nord` *and* `Niederlassung Nord x GmbH`), and a
   rotation is never a contiguous run of its counterpart. Its own test caught
   it; now a token subset.
3. **The genericness probe reads `org_match_keys`**, so on a stale or unrun
   key-build every agreeing group would read `distinctive` and the census would
   silently overstate the fold signal. `name_keys_absent` counts keys the table
   does not hold at all. **If it is non-zero, `agree-distinctive` cannot be
   trusted.**
4. **The probe must span `n2` AND `n3`.** Caught by reading the key build
   before running the census, not by running it: `build-org-match-keys` writes
   an `n3` row only `if k3 != k2`, so a name with no legal-form token
   (`Kreisverwaltung Ahrweiler`) has its N3 key stored under kind `n2` and
   nothing under `n3`. A `key_kind = 'n3'` probe would have reported most of
   the corpus absent — silently, and in the direction that looks safe. The two
   kinds cannot be conflated: `n3_key` emits a `§family` marker for every
   legal-form token, so a key containing `§` is only ever an N3 key and a key
   without one is an N2 key identical to its own N3 key. Pinned by
   `a_key_the_build_stored_under_n2_is_still_found`.

`cap` bounds the LISTING and never the TALLY (issue 326's inverted conclusions).

## Next

1. Run `duplicate-identity-census` on prod; read `name_keys_absent` FIRST — if
   it is non-zero, run `build-org-match-keys` and re-run before reading
   anything else.
2. Read the DE:vat cut. A high `disagree` share argues the pinned negative is
   right for VAT too, and the 3,215 stay as they are — a NEGATIVE conclusion is
   a real outcome here, as it was for issue 327.
3. Only if `disagree` is small: propose the arm, and it goes through the same
   ladder every write path here does — dry plan stored as a report, human review
   of actual values, wet run gated on `expect_rows` parity.
4. `unkeyed_by_scope` will name other scopes. File them separately; do not widen
   this issue into all of them.
