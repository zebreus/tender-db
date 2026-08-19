# 237 — a multi-lot bid names a LotsGroup, and we never record which lots that group contains

Status: DONE 2026-08-19 (`d4fee5f`, `2c31ac3`, `ff87753`; verified on prod) — mapping is COMPLETE
relative to what the corpus publishes; the remaining gap is a source limitation, not a projection one.
Open follow-up, split out: the SDK-native disposition gate (second finding below) is unbuilt.
Kind: projection mapping gap (parse layer HAS the data) + a coverage-gate blind spot
Blocked by: —
Relates to: 13 (results layer), 116 (tender detail reported more lots than it shipped), 88/85 (the
"parser captures it, fold drops it" class), 235 (same class, one metric over)

## The model is right; one edge is missing

A Bid is never many-to-many with Lots, and tender-db is faithful to eForms here. `efac:LotTender`
(TEN-nnnn) carries exactly ONE lot reference via `BT-13714-Tender`, pointing at either a Lot or a
**LotsGroup** (`docs/research/eforms-data-model.md:173` — "**Tender (tender-db: Bid)** N—1
Lot/LotsGroup"). A bid covering five lots is expressed by the buyer declaring a LotsGroup (GLO-nnnn)
for combined award, and the bid referencing the group.

tender-db models that correctly: `LOT_KINDS = ["Lot", "LotsGroup", "Part"]` (project.rs:297), a
LotsGroup is a row in `lots` distinguished by `tender_version_lots.kind`, and
`tender_version_bids.lot_id` can therefore name a group. Its `PRIMARY KEY (tender_id, seq, bid_id)`
with a single `lot_id` is the CORRECT cardinality, not a limitation — worth stating plainly, because
the table comment "one offer on one Lot" reads like a simplification and is not one.

## What is missing: the group's composition

**`BT-1375-Procedure` ("Group Lot Identifier") — the LotsGroup's member-lot references — is captured
by the parser and dropped by the fold.**

- The SDK declares it (`sdk/fields-1.13.0.json`:
  `…LotsGroup/cac:ProcurementProjectLotReference/cbc:ID[@schemeName='Lot']`), and the eForms parser is
  inventory-driven, so it is claimed into the parse layer. ADR-0004's exhaustive-consumption rule makes
  this near-certain rather than hopeful: a field the parser did NOT claim would quarantine every notice
  carrying a lots-group composition, and those notices are in the corpus, parsed.
- It appears **nowhere** in `project.rs` — not in a fact table, not in the explicit-ignore list. There is
  no `lot_group_members` table and no lot→lot linkage of any kind.

So we can say "this bid covers GLO-0002" and cannot say which lots GLO-0002 is.

## Why it matters

- Any per-lot rollup of bids or awards **silently omits combined-award bids**: they attach to the group
  row, not to its member lots. A bidder's per-lot win rate, a lot-level competition count, "who bid on
  lot 3" — all quietly wrong wherever a group was used, and right everywhere else, which is the hardest
  kind of wrong to notice.
- It is **recoverable without a re-parse**: the facts are already in the parse layer, so this is a
  projection mapping plus a re-fold — far cheaper than the text-era (issue 232) and DE-1.x (issue 100)
  cohorts, which need the parse layer rewritten.

## Second finding: the disposition gate does not cover SDK-native ids

`ubl_grafts_are_all_mapped_or_ignored` (project.rs:3856) enforces "every field is MAPPED or EXPLICITLY
IGNORED with a reason" — but only for tender-db's own `UBL-*` grafts, by scanning `index.rs` for the
`UBL-` prefix. SDK-native `BT-*` ids have no equivalent gate, which is exactly how BT-1375 slipped
through silently. That is the same class the gate was built to stop (issue 85's 218K factless tenders).

Extending it to `BT-*` means dispositioning every SDK field, which is a much larger set — so this
probably wants to start as a report (how many SDK fields the fold reads vs. how many the parser claims)
rather than a hard gate that fails on day one. Sized honestly, it is its own piece of work.

## The published shape, read from the bytes (2026-08-18, test in `adbcf82`)

Verified against the committed `eforms/can-maximal-sdk17.xml` before writing any mapping, and it is
NOT what this issue assumed. Membership does not live on the LotsGroup section:

    ND-GroupComposition#0   kind=GroupComposition   parent=PROCEDURE
      BT-330-Procedure   -> GLO-0001              (the group being composed)
      BT-1375-Procedure  -> LOT-0001, LOT-0002    (one repeat per member lot)

    GLO-0001               kind=LotsGroup          parent=PROCEDURE
      BT-137/157/21/24/27/300-LotsGroup  (its own title, value, description…)

The composition is a SIBLING of the group, hanging off the notice root — so a mapping that looked for
membership under `GLO-nnnn` would find nothing and conclude the data was absent. That is plausibly how
this stayed unmapped: the field id points at the group, but the section it lives in does not.

Both ends resolve to `lots` rows once projected (the group is a `LotsGroup`-kind section, and members
are ordinary `Lot` sections), so the link table needs no new identity — exactly the property the design
decision above depends on.

**Also unmapped: `BT-330-Procedure`.** It is the other half of the pair and shares BT-1375's fate, so
whatever reads one must read both.

### IMPLEMENTED and deployed (2026-08-18, rev `d4fee5f`)

`tender_version_lot_group_members(tender_id, seq, group_lot_id, member_lot_id)` — version-keyed like
every satellite, both ends `lots` ids, no new identity. The projection reads each `GroupComposition`
section (BT-330 → group, repeated BT-1375 → members), carries the pairs on `NoticeState` and through the
plan's `BucketRow`, and the fold writes them AFTER the lots loop so both ends resolve as lookups. The
table is allow-listed on `/v1/sql` with a note explaining the join.

**Four lifecycle lists needed the table, and a search found them where a guess would not have:**
`clear_canonical`, `reset_tender_layer`, `delete_version` and `RETIRE_TABLES`. `delete_version` is the
load-bearing one — without it a re-fold would double the rows — and `RETIRE_TABLES` matters because
membership references `lots`, so retirement would otherwise fail on the FK when deleting lots.

Two mistakes caught before shipping, both worth recording:

- I put `#[serde(default)]` on the new `BucketRow` field with a comment about old plans still
  deserialising. `BucketRow` is framed with **postcard**, which is not self-describing, so that is
  simply false — old bytes misparse rather than defaulting. What actually makes the format change safe
  is the lifecycle: the bucket directory is `remove_dir_all`'d at the start of every sharded run AND at
  the end, so no run ever reads bytes another binary wrote. The attribute is gone and the comment says
  the real reason.
- A pattern-replace inserted the table name TWICE into three of the four lists, because the 8-space
  indent pattern is a substring of the 12-space one. Caught by counting occurrences after the edit
  (8 expected, 11 found) rather than by the compiler, which would have accepted the duplicates happily.

Not yet done: **nothing populates history.** The fold maintains membership for Tenders it touches, so
pre-`d4fee5f` groups read as empty until a re-fold. Same choice as `current_title` (issue 239) — a
bounded backfill job reading `notice_ids` for BT-330/BT-1375 would fill it without a corpus rebuild, and
is the natural next unit.

### Implementation, now that the shape is known

Read each `GroupComposition` section in `project.rs`'s `read()`: the group is its single BT-330 id-ref,
the members are all its BT-1375 id-refs. Carry the pairs on `NoticeState`, and have the fold resolve
both lot keys through the existing `lots` lookup and write
`lot_group_members(tender_id, group_lot_id, member_lot_id)`.

Sequencing note: populating it needs a re-fold, and the `current_title` work (issue 239) showed a
projection-side column can be filled by a bounded backfill job rather than a corpus rebuild — the same
option applies here, since membership derives from the parse layer that is already stored.

## Acceptance

- A `lot_group_members` (or equivalent) canonical link, populated from `BT-1375-Procedure`, with the
  group and member both resolving to `lots` rows of the right `kind`.
- A test built on the committed fixture `eforms/can-maximal-sdk17.xml`, which carries BT-1375 twice, so
  no new fixture is needed.
- A re-fold, then a check that bids on groups can be attributed to member lots.
- Separately: a recorded decision on whether the SDK-native disposition gap becomes a gate or a report.

## Design decision (2026-08-18): make MEMBERSHIP first class; do NOT synthesise singleton groups

Asked directly: should LotsGroup become first class, with a synthetic group for every lot even when it
contains only that one lot, so a bid always references a group? **Recommendation: no** — populate
membership from published data instead. The uniformity is genuine and the cost is too high on four
counts.

**1. It invents data the source never published.** A singleton group has no `notice_sections` row behind
it. This layer's discipline is that a canonical row traces to a published fact — `organization_mentions`
is "the immutable evidence", ADR-0004 demands exhaustive consumption of what IS there. Synthesising
entities is the same class of problem as issue 234's provisional organizations, except self-inflicted
rather than forced by a source that publishes no identifier.

**2. Identity breaks, and this schema is unusually exposed to that.** A derived key
(`GLO-synth-LOT-0003`) is stable but fictional, and will eventually collide with a real `GLO-` id a buyer
publishes. A rowid key is unstable across `project --rebuild`, which drops and re-derives the canonical
layer — so every synthetic id changes and any external reference rots. The current model avoids this
completely because `lots.lot_key` IS the published id.

**3. Row cost, measured rather than guessed** (dashboard counts, 2026-08-18): `lots` = **13,304,590**,
`bids` = 6,898,781. Synthesising a group per ungrouped lot is roughly **+13M group rows and +13M
membership rows**, near-doubling the lot layer to serve the minority shape. In a corpus where a full scan
of a large table costs seconds and the `v_*` views cannot be filtered at all (issue 239), growing a table
for a rare case is the wrong direction.

**4. It relocates the polymorphism rather than removing it.** eForms still publishes `BT-13714-Tender`
pointing at either `LOT-nnnn` or `GLO-nnnn`, so the writer must still branch — and now also mint or look
up a synthetic group, in the fold, the re-parse and the cross-source merge, which are the hard paths. The
reader's uniformity is paid for by the three writers.

### What to build instead

`lot_group_members(tender_id, group_lot_id, member_lot_id)` populated from `BT-1375-Procedure`. Coverage
is then one branch: if the bid's lot row has `kind = 'LotsGroup'`, join membership; otherwise the bid
covers that lot. Published facts only, no synthesis, no new identity.

If consumers later want the flat shape, build it as a DERIVED `bid_lot_coverage` on top of published
membership, when a read pattern actually demands it. (A view would be the obvious vehicle and is not
available: per issue 239 a filtered query on a view here reads the whole corpus.)

**Kept as a live option, not dismissed:** if a future consumer needs per-lot award attribution
everywhere and the branch proves genuinely painful in practice, the flat derived table is the answer —
and it can be built without touching identity, which is exactly why the identity layer should stay
free of synthetic rows now.

## Deployed, and covering only ~1% of carriers — measured, with the fix designed (2026-08-18)

`refold-sections GroupComposition` (job 740) re-queued **9,694 carrier notices** and stamped 5,890
tenders; the projection (741) rewrote all 5,890. The membership table then held:

    member_rows 233 · groups 59 · member_lots 191 · tenders 54

**233 rows across 54 tenders, from 9,694 carrier notices.** The mapping works on the committed fixture
and misses ~99% of the corpus — the fixture-is-not-the-corpus trap, caught by checking the number rather
than trusting the green test.

### Why

`group_members()` requires `BT-330-Procedure` (the group reference) and skips a composition without one.
Across 10 sampled carriers' composition sections:

    BT-1375-Procedure   59 values   (the members)
    BT-330-Procedure    13 values   (the group)

So most compositions list their members and never name the group. The maximal fixture happens to carry
BT-330; real notices mostly do not.

### The group is still recoverable, for almost all of them

Sampled carriers, sections per notice:

    9 of 10 notices:  1 LotsGroup, 1 GroupComposition   → unambiguous without BT-330
    1 of 10 (24189305): 4 LotsGroups, 4 GroupCompositions → ambiguous without it

So the fix is a fallback with a hard edge, not a guess:
- BT-330 present → use it (unchanged).
- Absent AND the notice publishes exactly ONE LotsGroup → that group. Unambiguous, and it covers the
  overwhelming majority.
- Absent AND several LotsGroups → **skip, and count the skips**. Pairing `ND-GroupComposition#0` with the
  first group in document order is the obvious guess and is exactly that; the composition order and the
  group order are not documented to correspond, and a wrong membership row is worse than a missing one
  because it silently reassigns a bid's coverage.

The skip counter matters: it turns the residual into a number someone can watch rather than a silence.

### What is deployed meanwhile is under-coverage, not wrongness

Every row written so far came from an explicit BT-330 reference, so the 233 rows are correct — the table
is simply missing most of what it should hold. That is the safe direction to be wrong in, and it means
the fallback can land later without correcting any existing row.

Next: implement the fallback, re-run `refold-sections GroupComposition`, and expect the tender count to
move from 54 toward the 5,890 the refold touched. If it does not, the remaining carriers differ in some
further way and want the same treatment — measure before assuming.

## Fallback landed (2026-08-18, rev `2c31ac3`) — re-fold in flight

`group_members()` now takes the notice id and resolves the group three ways: BT-330 present → use it;
absent with exactly ONE `LotsGroup` in the notice → infer it; absent with several → skip, logging the
notice and the group count so the residual stays countable. A unit test covers all three plus the
no-group-at-all case, and the whole-corpus distribution sized the ambiguous arm before it was written:

    1 LotsGroup:  9,272 notices (95.6 %)
    2+:             ~406 notices

Deployed and `refold-sections GroupComposition` re-queued (jobs 1 + 2 after the restart). Expected: the
tender count moves from 54 toward the 5,890 the cohort re-folds. If it lands well short, the remaining
carriers differ in some further way and want the same measure-first treatment.

### How the residual gets counted, and why no code counts it

The skip is an `eprintln` per notice, not a `Report` counter. Threading a counter from `group_members()`
(a free function inside `NoticeState::read`) up to the run report means plumbing it through BOTH the
incremental path and the sharded bucket path, where it would also have to aggregate across shards — and
it would measure the same thing a bounded query measures exactly:

    SELECT COUNT(DISTINCT tv.caused_by_notice_id)
      FROM tender_version_lot_group_members m
      JOIN tender_versions tv ON tv.tender_id = m.tender_id AND tv.seq = m.seq

against the 9,694 carriers `notice_ids_with_section_kind(['GroupComposition'])` returns. The query is the
better instrument because it counts what actually landed rather than what the projection believed; the
log line stays for naming WHICH notices, which the query cannot say. Recorded rather than built.

### Two checks the fallback makes worth running, which BT-330 alone did not

`lot_identity` MINTS a `lots` row for a key the version does not publish (canonical.rs, deliberately —
better than dropping a published composition). With BT-330 that path almost never fired; at ~100× the
volume it may. So the verification also counts membership rows whose member — or group — is absent from
the same version's `tender_version_lots`. Non-zero is not wrong, but it is a shape the corpus was not
known to have, and it should be a number on this issue rather than a surprise later.

## Corrected by a whole-corpus count (2026-08-18): BT-330 is never missing, BT-1375 almost always is

The "covers only ~1 %" section above, and the fallback built from it, rest on a **mis-measured sample**.
Counted across every `GroupComposition` section in the corpus rather than ten sampled notices:

    GroupComposition sections                          11,416   (across 9,694 notices)
    ...carrying BT-330-Procedure  (the group)          11,416   ← every single one, one value each
    ...carrying BT-1375-Procedure (the members)            68   (62 notices, 235 values)
    ...carrying any OTHER field                             0   ← nothing else is ever in one

So the earlier reading — "most compositions list their members and never name the group" — is **exactly
backwards**. Every composition names its group. Almost none lists its members.

That re-reads the coverage number completely. Membership held 233 rows / 59 groups / 54 tenders across
**62 carrier notices**, and 62 is precisely the number of notices that publish BT-1375 at all. The
mapping was never at 1 % — it was, and is, **complete with respect to what is published**. The
denominator in the earlier section (9,694 carriers) counts notices that publish a group's IDENTITY; only
62 of them publish its COMPOSITION.

### What that means for the fallback shipped in `2c31ac3`

It cannot fire on today's corpus: it triggers only where a composition carries no BT-330, and no such
composition exists (11,416 of 11,416 carry one). It is dead code — kept, because it is three lines, it
fails safe (skip and log, never guess), and a future SDK version omitting BT-330 is exactly the shape it
handles. But it bought no coverage, and the commit message claiming it would is wrong. The lesson is the
plain one: **the sample was 10 notices and the population was 9,694, and the cheap whole-corpus count
that inverted the conclusion was available the whole time.** Count the population when the population is
one indexed query away.

### The real defect the re-check found: membership did not survive the chain

`fold()` carried `facts`, `lots` and `rounds` forward from the previous version and took
`group_members` from the causing notice ALONE. So membership sat on the version published by the notice
that defines the groups — a contract notice — while the bids that reference a group arrive with the award
notice, versions later. A consumer joining a bid to membership at the same `(tender_id, seq)` would have
found nothing.

Sizing, from the `notice_ids_target` index (every reference that resolves to a `LotsGroup` section):

    BT-330-Procedure     11,416 refs   9,694 notices   (the composition naming its group)
    BT-13714-Tender       1,408 refs     505 notices   ← BIDS on a group: what this table is for
    BT-13716-notice         173 refs      72 notices
    BT-556-NoticeResult     117 refs     107 notices   (per-group result statistics)
    BT-786-Review             1 ref        1 notice

Fixed in `ff87753`: membership carries forward like the rest, superseded **per group** — a notice
republishing a group's composition replaces that group's list entirely, a group it is silent about keeps
the list it had. Unit-tested over a three-notice chain (compose → silent → recompose), because the
silent middle notice is the case that matters: the award notice is usually the silent one.

## What is left, and it is not ours to fix

For **9,632 of 9,694** carriers the composition is simply not published: the notice names the group, and
nothing in it — no field, no other section — says which lots the group contains. Nothing else in the
notice references the group either (checked on notice 379: the only reference to `GLO-0005` anywhere in
it is the composition's own BT-330).

So for those groups a bid's per-lot coverage is **not recoverable from the notice**, at any effort. That
is a statement about TED's data, not about this projection, and it changes what any consumer should be
told: "which lots did this bid cover" is answerable for the 62 notices that publish it and unanswerable
for the rest. A per-lot rollup must therefore report combined-award bids as an explicit unattributable
residue rather than silently omitting them — which is the one part of this issue's original complaint
that still stands, now with a number on it (1,408 group-referencing bids, of which the ~233-pair
population is attributable).

Possible future recovery, in descending order of honesty: the group's own title (`BT-22-LotsGroup`
carries a buyer's internal id like `NN.270.4.2025`) sometimes encodes the lots; `BT-556-NoticeResult`
statistics per group hint at member count; neither is a published composition and both would be
inference. Not worth doing unless a consumer asks for it, and if it is ever done it belongs in a
DERIVED table with its own provenance, never in `tender_version_lot_group_members`.

## Verified on prod after the carry-forward re-fold (2026-08-19, rev `ff87753`)

`refold-sections GroupComposition` (job 744) + projection (745), then bounded checks:

    member_rows                                       277   (233 before the carry-forward)
    groups / member_lots / tenders              59 / 191 / 54
    versions carrying membership                       71   (62 before)
    rows whose GROUP end is not a LotsGroup-kind lot     0
    rows whose MEMBER end the version does not publish   0   (lot_identity never had to mint one)

The +44 rows are the carry-forward doing its job: the same 59 groups, now present on the later versions
of their tenders instead of only the version that published the composition.

**The acceptance criterion — "a bid on a group can be attributed to member lots" — now holds:**

    bids sitting on a group whose membership is recorded at the SAME version    2
    the same count computed at TENDER level (any version)                       2

Both being 2 is the interesting part: within these 54 tenders the carry-forward leaves nothing behind, so
version-scoped and tender-scoped attribution agree. The number is 2 rather than hundreds because
composition-publishing and group-bidding rarely co-occur in one tender — of the 1,408 bids that reference
a group corpus-wide, the attributable set is bounded by the 62 notices that publish any composition.

That is the whole of what this issue can deliver. The residue (1,406 group bids with no published
composition anywhere in their tender) is the source limitation described above, and the right response is
to report it as an explicit unattributable count, not to infer it.
