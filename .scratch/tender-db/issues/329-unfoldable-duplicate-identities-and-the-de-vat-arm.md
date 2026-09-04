# 329 — Unfoldable duplicate identities, and whether `canonical_key` should get a DE:vat arm

Status: MEASURED AND DECIDED 2026-09-01 (job 568, `b7f1a8f`, 2 s).
**The answer is NO: `canonical_key` must NOT get a blanket DE:vat arm.** The
residual opportunity is a corroborated arm, filed as its own proposal below.
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

## First two prod runs: both cancelled, and the second one taught the lesson

**Run 566** (`0da3b2c`) was still going after 25 minutes and was cancelled
(honest cancel, no report stored). I diagnosed the per-member correlated
`COUNT(*) FROM organization_mentions` in pass 2 and moved mention counts to a
fourth pass over the listed rows only (`916e8bd`). That change is a real
improvement — the count feeds nothing but the capped listing — **but it was not
the cause, and I should not have inferred a cause from a runtime.**

**Run 567** (`916e8bd`) carried the progress feed added in the same commit, and
it answered the sizing question in seconds:

> `reading names for 7289 unkeyed member rows`

**7,289 rows.** Fifteen batched primary-key lookups. Nothing in pass 2 could
take 25 minutes, so the mention subquery was never the bottleneck. The stall is
in the classification loop, whose ONLY database call is the genericness probe —
and that probe was the thing I had just changed:

```sql
-- the stalling form: an IN on the LEADING column of
-- org_match_keys_kk(key_kind, key, org_id)
WHERE key_kind IN ('n2','n3') AND key = ?
```

SQLite turns that into two index seeks. turso, a reimplementation, evidently
does not — so each new key scanned a multi-million-row table. Replaced with two
equality probes ('n3', then 'n2' if absent), which is index-safe under any
planner and is the shape `name_key_is_generic` was already proven on.

Two process notes worth keeping:

* **The feed found the bug the runtime could not.** Run 566 gave a number
  (1,500 s) and I attached a cause to it. Run 567 gave a *measurement* (7,289)
  and the cause fell out immediately. The observability fix paid for itself on
  its first firing.
* **The first feed still lied**, because it ticked every 5,000 groups and the
  class is a few thousand — so the phase line sat on the previous pass's message
  for the whole classification loop. Now every 500, and per chunk in pass 2.

## The measurement (job 568, `b7f1a8f`, 2 seconds)

| | |
| --- | --- |
| organizations walked | 1,118,068 |
| distinct `(country, kind, identifier)` triples | 1,114,221 |
| triples held by >1 org row | **3,458** (covering 7,305 rows) |
| …keyed by `canonical_key` (R2 already sees them) | 8 |
| …**unkeyed — no arm can reach them** | **3,450** |
| name keys absent from `org_match_keys` | **0** ⇒ the agree/generic split is trustworthy |

`DE:vat` is 3,215 of the 3,450 — **93% of the whole unreachable class**. The
next-largest scopes are `LT:national` 101 and `DE:national` 77; the remaining
thirty-odd scopes are 1–9 groups each and not worth an arm.

### DE:vat name verdicts (3,215 groups)

| verdict | groups | share |
| --- | --- | --- |
| `agree-distinctive` | 1,780 | 55.4% |
| `agree-generic` | 329 | 10.2% |
| `contained` | 547 | 17.0% |
| **`disagree`** | **559** | **17.4%** |
| `unnamed` | 0 | — |

## THE DECISION: no blanket DE:vat arm — and the reason is not the one expected

17.4% disagreement would be disqualifying on its own. But reading the actual
values (rather than the counts) changes the *reason*, and the reason matters for
what to do next. **The `disagree` bucket is not mostly Organschaft. It is German
public bodies sharing a Land-level VAT registration.**

```
DE811335517  8 rows, 25,018 mentions
  Regierung von Oberbayern
  Regierung von Oberbayern, Vergabekammer Südbayern
  Vergabekammer Nordbayern bei der Regierung von Mittelfranken
  Regierung von Mittelfranken - Vergabekammer Nordbayern

DE812056745  5 rows, 19,276 mentions
  Vergabekammer des Landes Hessen beim Regierungspräsidium Darmstadt
  Regierungspräsidium Darmstadt

DE143845578  4 rows, 60 mentions
  Verkehrsverbund Rhein-Neckar GmbH (VRN)
  Friedrich Müller Omnibusunternehmen GmbH        ← unrelated companies
```

Two things make this decisive:

1. **These are distinct authorities**, not one entity fragmented. Oberbayern and
   Mittelfranken are different Bavarian governments; a Vergabekammer is a
   different body from the Regierung that hosts it. Folding them destroys real
   distinctions in exactly the layer the org matcher exists to sharpen.
2. **The blast radius is concentrated in the wrong bucket.** The disagreeing
   groups carry the largest mention counts in the class — 25,018 / 19,276 /
   18,295 — while the clean `agree-distinctive` specimens carry 25–428. A
   blanket arm would be most wrong exactly where it moved the most corpus.

So the pinned negative in `crosswalk::canonical_key` is **right for VAT too**,
but for a different reason than the one it states. Its comment says *court-scoped
registers*, which is an argument about `HRB`. The argument for VAT is **shared
public-sector VAT registration**. Worth adding to that comment so the next reader
does not re-open this on the grounds that the stated reason does not apply.

## `contained` stays undecided, and there is now a specimen proving it

```
DE129274202   Siemens AG / Siemens AG Smart Infrastructure          ← a division; foldable
DE266749428   Karlsruher Institut für Technologie (KIT) / …ohne (KIT) ← foldable
DE158840613   Prospitalia GmbH / Vertragseinrichtungen der Prospitalia GmbH, Ulm
```

The last one is the live counter-example: *Vertragseinrichtungen der Prospitalia*
is the set of **client institutions contracted to** Prospitalia, not Prospitalia.
Containment cannot separate that from a branch office, which is what the bucket's
existence asserts. Confirmed by data rather than by argument.

## A caveat on `agree-generic`, recorded so nobody over-reads it

`STOPLIST_CAP` is 20, and the carrier count is taken over `org_match_keys` —
which contains a row per **org row**, duplicates included. So the very
fragmentation this census measures inflates carrier counts and pushes some real,
distinctive names into `agree-generic` (`DE115302781 — H. Hüther GmbH` is one:
not a generic name by any reading). **`agree-generic` is therefore an
over-count**, and `agree-distinctive` a slight under-count. It does not change
the decision — the decision rests on `disagree` — but a later corroborated arm
must not treat `agree-generic` as a refusal without re-deriving it.

## Next: a corroborated arm, not a `canonical_key` arm

The 1,780 `agree-distinctive` DE:vat groups are a real opportunity and the
specimens read clean (`Badegärten Eibenstock GmbH`, `Perfekt Bodenbau GmbH`,
`HEUSSEN Rechtsanwaltsgesellschaft mbH`, `grbv Ingenieure im Bauwesen GmbH & Co.
KG` under two capitalisations). But they cannot be reached through
`canonical_key`, which is identifier-only by design — and that design is what
keeps R2 honest.

The shape that fits is **R3's**: unique anchor + standing target + exact name
corroboration + generic-name denial. That is a separate proposal with its own
dry-plan/review/parity ladder, and it must carry a public-body veto, because the
`disagree` reading shows the failure mode is governmental rather than corporate.
Not started; filed here rather than begun, so the board stays the record.

## 2026-09-04 — a new unkeyed same-triple pair from issue 345's repair, as a specimen

Orgs 311 and 1079 (the Greek Single Public Procurement Authority, 8,029 and 3,601
mentions) now share `GR national 1000E009610001` after the v2.1 lookalike fold
and the 345 repair aligned their identifiers. The GR arm keys 9-digit AFMs only,
so R2 (job 638) left the pair standing — the largest group this class has by
mention count. Names agree modulo case and accents (`Ενιαία Αρχή Δημοσίων
Συμβάσεων (Ε.Α.ΔΗ.ΣΥ)` / `ΕΝΙΑΙΑ ΑΡΧΗ ΔΗΜΟΣΙΩΝ ΣΥΜΒΑΣΕΩΝ`), i.e. `agree-distinctive`
under an accent-insensitive N2. Two routes, both bounded: a GR authority-code arm
in `canonical_key` (14-char, letter-bearing, one group so far — an arm for one
group is the "not worth an arm" call this issue already made), or a case-review
merge verdict through the 311 machinery. Left for the per-scope decision; the
repair's job was to make it visible, and it is.

## Re-census after the 345 repair (job 639, 2026-09-04 02:51 UTC, 2 s)

| | 2026-09-01 (job 568) | 2026-09-04 (job 639) |
| --- | --- | --- |
| rows walked | 1,118,068 | 1,117,963 |
| triples held by >1 org row | 3,458 (7,305 rows) | **3,684 (7,781 rows)** |
| …keyed by `canonical_key` | 8 | 18 |
| …unkeyed | 3,450 | **3,666** |
| name verdicts (unkeyed) | DE:vat only was tabled | agree-distinctive **2,030** / agree-generic 357 / contained 600 / disagree 679 (18%) |

The class grew by exactly the repair's residue: issue 345 moved 2,409 rows onto
their live-normaliser reading, R2 folded the 1,846 groups its arms key, and the
rest became visible same-triple duplicates — above all **`GR:national`: 210
groups** (the 14-character Greek authority codes the GR arm does not key), with
verdicts agree-distinctive 134 / agree-generic 4 / contained 15 / **disagree
57**. LT:national 101 (43 / 9 / 16 / 33), DE:national 77, DE:vat 3,215 as before.

**So the GR question answers itself the way DE:vat did:** 27% of the Greek
groups carry different names under one code — the shared public-sector
registration shape — and a blanket GR arm would fold distinct authorities
exactly where it moved the most corpus. No arm.

**The rule that IS safe, corpus-wide: E0 + agree-distinctive.** Two org rows
with the same `(country, kind, identifier)` triple whose name keys agree and
are not generic — 2,030 groups, 55% of the class, 311/1079 among them — are
one entity by both kinds of evidence at once, stronger than any E1 key alone.
`agree-generic`, `contained` and `disagree` stay out (the Prospitalia
counter-example above is why `contained` cannot ride along). Next unit: a dry
job that lists the E0 agree-distinctive groups with the keep chosen by mention
count, a 100-sample precision review at the §8 Stage-2 bar (100%), then a wet
run through the merge arms with their denial stack. Filed on the task board.
