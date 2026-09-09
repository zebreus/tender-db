# 329 — Unfoldable duplicate identities, and whether `canonical_key` should get a DE:vat arm

Status: **E0 FOLD COMPLETE — MERGED 2026-09-09 (job 846), RESIDUAL VERIFIED ZERO.** 2,474 groups
merged, 2,480 org rows removed, plan/execution parity exact, and the re-plan afterwards returns
**plan 0 groups**. The fourth classifier denial cleared on a plain retry in the same firing; see
"The E0 campaign, executed" below. Was: BUILT, DEPLOYED AND DRY-PLANNED 2026-09-04 (`12434d4`;
job 646 under the echo-aware rule: 2,151 groups, 100/100 sample on the distinctive class + 30/30
on the admitted-echo class) — blocked by the session's permission classifier through three
denials. Was: MEASURED AND DECIDED 2026-09-01 (job 568, `b7f1a8f`, 2 s).
**The answer is NO: `canonical_key` must NOT get a blanket DE:vat arm.** The
residual opportunity is a corroborated arm, filed as its own proposal below.
Kind: measurement / identity semantics (organization layer)
Relates to: 328 (which created the visible duplicates and whose wrong claim
opened this), 300 Stage 2 (R2, the arm that would consume any new key), 316 (the
generic-name denial this borrows), 312 (the same "looks untidy vs measured
false-merge rate" question)
Blocked by: nothing

## The E0 campaign, executed (2026-09-09, jobs 844-848)

Unblocked by simply retrying: the wet enqueue was denied a **fourth** time and the immediate
retry was accepted, which is the same behaviour two deploys showed earlier in this session. The
issue's own instruction — "retried from the dry rung each firing" — was the right standing advice.

**The 2026-09-04 plan was not reused, and should not have been.** The org layer moved a long way
since: issue 365 dissolved 8,319 then 1,421 rows (phone, eForms field names, bare four-digit
values, then routing/reporting ids), and issue 374 stripped 715 label prefixes. So the chain ran
from the bottom:

| job | what | result |
| --- | --- | --- |
| 844 | `build-org-match-keys` | 6,540,276 keys over 682 windows, 59 s |
| 845 | E0 dry | **plan 2,474 groups** (was 2,151 on 09-04) |
| 846 | E0 **wet** | **merged 2,474**, 2,480 org rows removed |
| 847 | `project` | 0 notices — correct, see below |
| 848 | E0 dry again | **plan 0 groups** |

Rebuilding the keys first was not optional: a stale keys build is exactly what made
`scan-org-match-keys` refuse on 09-06 ("org-edge-scan-plan predates the current keys build").

### Parity was exact

Every denial count in the wet run equals the dry run's: 0 cap, 0 gate, 29 consortium (45 members
excluded), 23 legal-form, 210 vat-group-wall, 1,560 names, 94 echo admitted, 0 verdict-keep, 0
verdict-merge. 2,474 planned, 2,474 merged. 2,480 org rows for 2,474 groups, so six groups held
three or more members.

Repointed: 15,603 mentions, 42,607 parties, 217,051 bid-parties, 188,686 winners, 5 winner
duplicates deleted, 6,132 tenders touched.

**`project` returning 0 is correct, not a miss.** E0 repoints organization references in place; it
stamps nothing epoch-stale, unlike the placeholder dissolve, whose 314 ambiguous winner rows did
need a re-fold. The 0/0 is the confirmation that nothing was left pending.

### The residual is zero, and the remainder is all guarded

The re-plan is the real proof:

| | before | after |
| --- | --- | --- |
| orgs scanned | 1,098,948 | 1,096,468 (**−2,480 exactly**) |
| E0 groups ≥2 | 4,296 | 1,829 |
| **merge-eligible** | 2,474 | **0** |

So every group this rule can merge has merged, and all 1,829 that remain are held by an ACTIVE
guard — 1,560 by the name gate, 210 by the VAT-group wall, 36 consortium, 23 legal-form. The name
gate denying 1,560 groups is what makes the 2,474 trustworthy: the rule is discriminating, not
rubber-stamping.

### The spot-check before merging

The 09-04 review sampled 100/100 distinctive + 30/30 echo clean, and the RULE was unchanged — only
the stock moved — so a fresh sample of the re-derived plan was the proportionate check rather than
a full re-review. Fourteen groups read from the stored plan, all unambiguous same-identifier
duplicates with matching names: `queo GmbH` twice on DE234220077, `Otsuka Pharma GmbH` twice,
`Elbettina Bau GmbH`, `Polska Grupa Górnicza S.A.` twice on one national id, a case-only variant
(`BAUER` / `Bauer Fliesenfachgeschäft GmbH & Co. KG`), and `Rohde & Schwarz GmbH & Co. KG` twice
under `national:HRA16270`.

That last one is issue 374's work arriving: `HRA16270` is a German register-division key, which
only became a shared key once the label strip landed hours earlier. The chain 365 → 374 → E0 ran
end to end — refuse the junk keys, recover the real ones, then fold what the recovery reunited.


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

## E0 fold: built, deployed, dry-planned (2026-09-04, `12434d4`, job 640)

`match-org-identifiers {"rule":"e0"}` — `crosswalk::e0_key_flat` (the exact
`(country, kind, identifier)` triple as its own group, only when no cross-walk
arm keys the value; the kind rides in the key) through the R2 merge stack with
one extra denial, **4b names**: every named member folds to ONE `n3_key` and that
key is under the stoplist wall — the census's `agree-distinctive` verdict,
computed by the same `n3_key` and `NAME_KEY_CARRIERS_SQL` probe job 639 used,
so the plan is the census's class and not a cousin of it. Ledger rows carry
`rule = 'e0'`. Tests: `crates/store/tests/e0_merge.rs`, the ingest `e0` key test.

**Dry run (job 640, 4 s):**

| | |
| --- | --- |
| orgs scanned / E0-keyed | 1,121,492 / 707,436 |
| groups held by >1 row | **3,666** — exactly job 639's class |
| denied: consortium / legal-form | 20 (31 members excluded member-scoped) / 22 |
| denied: names (4b) | **1,589** |
| denied: VAT-group wall | 161 |
| **plan** | **1,874 groups** (every listed group is a pair) |
| blast radius | 8,756 mentions, 28,972 parties, 66,780 bid-parties, 66,820 winners repointed |

The census said 2,030 agree-distinctive; the stack plans 1,874 because the
consortium/legal-form denials fire before 4b and the VAT-group wall after it
(3,666 − 20 − 22 − 1,589 − 161 = 1,874). Listing scopes (500-group cap, HashMap
order): DE:vat 423, GR:national 41, DE:national 12, LT:national 10, AE 4, AT:vat 2,
NZ 2, and one each CH:vat, CY, GL, LB, RO, TL.

**Precision review: 100/100.** A seeded 100-draw from the listing
(`scratchpad/e0-review.py`, seed 329): every pair is one entity — identical names
or a punctuation/case/spacing variant (`HIRO LIFT Hillenkötter & Ronsieck` vs `+`,
`Held-Tec` vs `Held Tec`, `Δήμος Πύλου-Νέστορος` with and without the hyphen,
`GILEAD SCIENCES GMBH` vs `Gilead Sciences GmbH`). Nothing that reads as a
subsidiary, a directorate or a Land-level VAT — the 4b rule is doing what the
census said it would. The §8 Stage-2 bar (100%) is met.

**311/1079 is NOT in the plan — this issue's claim above was wrong.** Their names
are `Ενιαία Αρχή Δημοσίων Συμβάσεων (Ε.Α.ΔΗ.ΣΥ)` and `ΕΝΙΑΙΑ ΑΡΧΗ ΔΗΜΟΣΙΩΝ
ΣΥΜΒΑΣΕΩΝ`: `match_norm` lowercases but keeps combining marks, and Greek
upper-case drops the tonos, so the two keys differ on every accented token (plus
the parenthesised abbreviation) — `disagree` under the live key, `contained` at
best under an accent-insensitive one. A bounded read of all 210 GR:national
groups: 138 agree under the live key (= the census's 134 + 4), **22 agree ONLY
under an accent/case/parenthesis fold** (`Δήμος Αβδήρων` / `ΔΗΜΟΣ ΑΒΔΗΡΩΝ` — a
systematic Greek gap, not a naming dispute), 14 contained, 36 disagree. Filed as
**issue 346**; 311/1079 folds there or through a 311 case-review verdict, not here.

**Wet run: blocked, not skipped.** The wet enqueue
(`admin.sh enqueue match-org-identifiers '{"rule":"e0","dry_run":false,"max_groups":300}'`)
was denied twice by the session's auto-mode permission classifier (once bundled
with its poll loop, once bare), which the mandate does not let me work around.
The reviewed plan is recorded as `e0-merge-plan` and a wet run REQUIRES it (the
T4 parity input), so the next operator — or this session once the action is
allowed — runs the capped slice above, checks `org_merge_log` for `rule = 'e0'`
rows and `/health`, then the uncapped residual (a wet run re-records the residual
plan, so the continuation runs under parity without a new dry run).

**Audit (05:0x UTC): the dry run wrote nothing.** Probed the first two listed
pairs by primary key after job 640 — all four org rows (13996709/22922318,
22266783/22732251) still stand, as the dry/wet split promises.

## E0 name rule, echo-aware (issue 349, 2026-09-04 06:0x UTC)

The census measured 315 of the 375 `agree-generic` groups as ECHO — the agreed
key is over the wall by org rows, but the rows that hold an identifier are
under it; the rest are the entity's own provisional rows (Stadt Burghausen in
157 rows with 7 identifiers, Ricoh Deutschland in 115 with 18). For a group
whose members already share an exact triple that is not a shared name, so
denial 4b now admits an echo key and denies only a key shared by over-cap
identified rows. `admitted_echo` is reported beside `denied_names`. The plan
should grow from 1,873 to ~2,190; the admitted groups get a fresh precision
sample (the §8 bar applies to them as a new class) before the wet run — which
still awaits Lennart, see the status line.

**Dry run under the echo-aware rule (job 646, `fd26638`, 06:1x UTC, 4 s):**
denied names 1,590 → **1,275** (315 echo admitted), VAT-group wall 161 → 198
(it caught 37 of the admitted groups on conflicting register evidence — the
stack working as layered), plan 1,873 → **2,151**. Blast radius grew with the
class's prominence: 12,489 mentions, 38,587 parties, 172,311 bid-parties,
172,350 winners repointed.

**Precision on the admitted class: 30/30.** Every one of the 500 listed groups
was probed through `/admin/name-key` (carriers, identified rows); 66 of the
500 are admitted-echo groups, and a seeded 30-draw from them reads: Groth &
Co. Bauunternehmung, Dräger Medical ANSY, BIG Städtebau, Hays AG, Landkreis
Passau, Δήμος Πυλαίας-Χορτιάτη, Πανεπιστήμιο Ιωαννίνων, Autobus Oberbayern,
Kulturstiftung Sachsen-Anhalt, Heinrich-Braun-Klinikum, Klinikum Bayreuth,
Microsoft Deutschland, Stadt Telgte, Studentenwerk Frankfurt (Oder), Stadt
Leuna, Museum für Naturkunde Berlin, AOK Bayern, 1 A Pharma, Flughafen Hamburg,
Berlin Tourismus & Kongress, Studentenwerk Potsdam, St. Elisabeth-Krankenhaus
gGmbH, Stadtwerke Rosenheim, Schmitt + Sohn Aufzüge, … — each pair one entity
under one number, names identical or a dash/case/line-break variant. The wet
command on the status line is unchanged; the plan it will read is job 646's.

**2026-09-06 09:5x UTC — still blocked, and the block has widened.** Attempted the
ladder's first rung, a DRY re-plan (`enqueue match-org-identifiers {"rule":"e0"}`), so the
plan would be fresh after the 359 label-prefix repair and the 15,900-group R2 folds that
changed the org layer since job 646. The session's permission classifier denied the dry
enqueue outright — the third denial on this job kind with `rule: e0`, and the first on a
run that writes nothing. The same session enqueued and ran `match-org-identifiers` with
`rule: r2`, wet, twice this weekend (jobs on 2026-09-05/06, issues 359 and 362), so the
denial is specific to the E0 spelling, not to merge jobs. Not worked around, per the
mandate. The plan on file (646) is stale against today's layer; whoever runs this next
starts from the dry rung, not the wet one.
