# 369 — a published BT-04 becomes the Tender group key verbatim: an all-zero v4 UUID glues seven notices from three buyers into one served record

Status: ready-for-agent — **units 1, 2a, 2b, 2c and 3 DONE and verified on prod 2026-09-08 (`a83495b`,
jobs 819+820): the gate refused exactly the census's 3 keys, the welded tenders are retired, the 4
correct shaped tenders are untouched, and a split-out notice now serves its own title. REMAINING: unit
4 (detector, cross-referenced to the OJS-closure issue) and NEW unit 5 — the island fallback
over-splits (93 notices → 91 islands), so group a refused key by BUYER instead.** Was: filed 2026-09-07;
unit 1 census done 2026-09-08 — it revised unit 2's rule
Kind: defect (identity / grouping) — the placeholder-gate asymmetry with the org layer
Relates to: 34 (built `is_uuid` for exactly this failure mode), 300 / idgate (the
placeholder machinery organizations have and procedure keys do not),
`legacy-ojs-closure-welds-unrelated-procurements` (the same invariant, the other
mechanism — its unit 3 gauge is this issue's detector), ADR-0003

## Observed

`SELECT id, procedure_key, current_title, current_seq FROM tenders WHERE id=1` → procedure_key **`00000000-0000-4000-8000-000000000000`**, title "Ladekabel eAutos", 7 versions. All seven caused-by notices publish that same string: six as `BT-04-notice`, one (26244735, DE-1.x) as `DE1-ContractFolderID`.

**Three buyers** via `tender_version_parties JOIN organizations`: seq 1–2 "Klinikum Neumarkt" (org 2548), seq 3–4 "Land Baden-Württemberg … Regierungspräsidium Karlsruhe" (org 22265484), seq 5–7 "Berufsgenossenschaft Holz und Metall" (org 22040540). **Three titles**: "HPS - Heizung" (seq 1–2), "Ertüchtigung des Rheinhochwasserdammes (RHWD) XXXIX - Grundwassermodellierung" (seq 3–4), "Ladekabel eAutos" (seq 5–7).

**Field mixing, proven**: `SELECT seq, lot_id, field, utc_seconds FROM tender_version_dates WHERE tender_id=1` → `duration_start` 1716156000, `duration_end` 1761865200 and `opening_date` 1707822000 are set at **seq 1** (the Klinikum Neumarkt heating job) and carried unchanged through seq 7 — so the served "Ladekabel eAutos" tender publishes the heating job's dates. `curl /v1/tenders/1` serves both Ing. Kobus's €103,825.80 bid (the Rhine flood-dam groundwater job) and Locio GmbH's €505,800 bid (the EV-cable job) on the same `LOT-0001`.

**It is not the nil UUID** (a filter built on that would miss): `00000000-0000-4000-8000-000000000000` has version nibble 4 and variant nibble 8 — a structurally valid v4 whose payload is all zeros. `grep -rn "00000000-0000"` over `.scratch/tender-db/issues/`, `docs/`, `CONTEXT.md` and `crates/` returns nothing.

## Why, exactly

- `procedure_key` returns `first_id(parsed, PROCEDURE_KEY_FIELD).filter(|k| !k.trim().is_empty())` — a whitespace test and nothing more — `crates/ingest/src/project.rs:4486-4489`; and `build_plan_groups` makes that string the group key verbatim: `CASE WHEN procedure_key IS NOT NULL THEN procedure_key`, `crates/store/src/canonical.rs:7126-7128`.
- The reason is stated out loud in the doc comment: "`procedure_key` accepts any non-empty BT-04 unchecked, because on TED BT-04 is a spec-guaranteed uuid" — `crates/ingest/src/project.rs:653-657`. The key is trusted because the spec promises it, and nothing verifies the promise was kept. The same paragraph then describes this exact failure for the dialects: "a portal-local reference number here would key a Tender on a string that is only notice-local — and every notice sharing it would collapse into one Tender. That is issue 34's failure exactly."
- The one gate that exists is shape-only and applied elsewhere: `is_uuid` (`crates/ingest/src/project.rs:4505-4515`) tests 36 chars, hex, dashes at 8/13/18/23, and is reached only by the national dialects' folder ids (project.rs:4498-4500). The zeroed UUID passes it — which is why the DE-1.x notice 26244735 joined too, through the gate issue 34 built to prevent precisely this.
- **The asymmetry is the finding.** `crates/ingest/src/idgate.rs` is an entire module of placeholder detection for ORGANIZATION identifiers — `lexicon_hit` (`crates/ingest/src/idgate.rs:251-281`) rejects `NIMAT…`, `ORG-…`, `BT501`, zero-padded stubs and the `1234…` family; plus digit-run, letter-run and per-scheme checksum rules. There is no equivalent module, lexicon or call site for procedure keys, and the correct gate here is the same *kind* of rule (a plausibility/entropy test by shape) rather than an equality against a known bad value.
- **Amplifier — why the weld looks plausible instead of broken:** `fold` supersedes facts per field and merges lots by lot KEY (`crates/ingest/src/project.rs:3395-3411`). All three procurements publish `LOT-0001`, so their dates, CPVs and values overwrite one another into one internally coherent, entirely fabricated record.

## Units

1. **Census first** (cheap, bounded): how many tenders carry a BT-04-derived key that a placeholder gate would refuse, and how many versions / distinct buyers each has. Record here before flipping anything.
2. **The gate**, by shape and not by equality (the idgate way): an all-zero or single-repeated-nibble payload, nil/max UUIDs, and a small lexicon of published placeholders — applied at `crates/ingest/src/project.rs:4486-4489` to BT-04 as well as to the dialect folder ids. A refused key falls back to `island:{notice_id}` (`crates/store/src/canonical.rs:7126`), which splits — the safe direction under CONTEXT.md:112-113.
3. **Repair**: re-project the affected keys so the welded tenders split; tender 1 becomes three tenders (or three islands).
4. **Detector**: the component-plausibility gauge in `legacy-ojs-closure-welds-unrelated-procurements` unit 3 (distinct buyers / titles / year span per tender) catches both mechanisms — cross-reference, do not build it twice.

## Unit 1 — the census (run 2026-09-08 against prod, bounded index seeks via `/v1/sql`)

**Method.** `tenders_procedure_key` is a plain index on `tenders(procedure_key)`, so a
`>= 'nnnnnnnn-' AND < 'nnnnnnnn.'` range is an indexed seek (20–46 ms each, measured). 30 such
ranges were probed one at a time — all 16 single-nibble first blocks in both cases, plus
`12345678`, `01234567`, `87654321`, `11223344`, `deadbeef`, `abcdefab`. A single 31-arm `OR`
was tried first and fell back to a scan (408 at the 10 s cap, not retried — policy).

**Hits: 4 first blocks, 13 tenders.** Every other probed block returned 0, including the
`1234…` family the org layer's `idgate` lexicon rejects and both cases of `deadbeef`.

| tender | procedure_key | versions | **buyers** | orgs | titles | first → last publication |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `00000000-0000-4000-8000-000000000000` | 7 | **3** | 10 | 3 | 2024-01-09 → 2026-07-02 |
| 82802 | `11111111-1111-4111-9111-111111111111` | 42 | **7** | 34 | 15 | 2024-01-08 → 2025-01-06 |
| 82804 | `11111111-2222-4000-8111-123412341235` | 44 | **11** | 42 | 31 | 2024-02-25 → 2025-05-14 |
| 82803 | `11111111-2222-4000-8111-123412341234` | 4 | 2 | 6 | 7 | 2024-02-07 → 2024-04-16 |
| 82805 | `11111111-2222-4aaa-8333-444444444444` | 2 | 1 | 3 | 1 | 2024-08-07 → 2024-08-11 |
| 82806 | `…444444444445` | 4 | 2 | 8 | 2 | 2024-08-15 → 2024-10-24 |
| 82807 | `…444444444446` | 4 | 2 | 9 | 2 | 2024-11-14 → 2024-11-19 |
| 82808 | `…444444444447` | 4 | 1 | 3 | 2 | 2024-12-18 → 2025-02-13 |
| 82809 | `…444444444448` | 4 | 2 | 5 | 2 | 2025-04-16 → 2025-08-20 |
| 82810 | `…444444444449` | 4 | 2 | 6 | 2 | 2025-12-17 → 2026-01-27 |
| 82811 | `…444444444450` | 4 | 2 | 5 | 2 | 2026-05-21 → 2026-06-18 |
| 160170 | `22222222-2222-4222-8222-222222222222` | 2 | 1 | 3 | 1 | 2024-01-15 → 2024-01-17 |
| 778091 | `aaaaaaaa-aaaa-4aaa-8abc-aaaaaaaaaaaa` | 2 | 1 | 3 | 1 | 2025-07-09 → 2025-07-10 |

`buyers` = `count(DISTINCT organization_id)` over `tender_version_parties` at `role='Procedure-Buyer'`
(the role vocabulary is `Procedure-Buyer` / `Tenderer` / `Lot-*` / `Procedure-SProvider`; `orgs` counts
every role, so it is inflated by bidders and review bodies and is NOT the weld signal).

### The two-buyer rows are org duplicates, not welds — checked by name

- 82806's two: org 22771062 and org 30900729, both "Rostocker Gesellschaft für Stadterneuerung,
  Stadtentwic…", both DE.
- **82803's two: org 20040451 "Stadt Osnabrück - FD Öffentliche Aufträge" and org 22494154
  "Stadt Osnabrück - Fachdienst Öffentliche Aufträge"** — one authority, one abbreviated. So 82803 is
  NOT welded, despite 7 distinct title strings (4 versions × DE/EN plus lot titles).

**So `buyers = 2` is the org layer's duplication noise floor, measured twice — not evidence of a weld.**

### What the census actually shows

**Three tenders are welded** — 1, 82802, 82804: 3, 7 and 11 distinct buyers over 93 versions.
**Ten are correctly grouped** on placeholder-*shaped* keys that are nonetheless doing real work:
Rostock's procurement office incremented the last block per procurement
(`…444444444444` … `…444444444450`, one tender each, 2–4 versions, one buyer, one title in DE+EN),
and 160170 / 778091 are single-buyer two-version records.

## Unit 2 — the decision the census forces: shape is necessary and NOT sufficient

The issue proposed "a plausibility/entropy test by shape … a refused key falls back to
`island:{notice_id}`". **The census refutes shape-only, and the counter-example is inside the same
first block.** Compare:

- `11111111-2222-4000-8111-123412341234` (82803, correct) — blocks are constant runs and a repeated
  `1234` cycle,
- `11111111-2222-4aaa-8333-444444444444` (82805, correct) — same,
- `11111111-2222-4000-8111-123412341235` (82804, **welded across 11 buyers**) — same, and it differs
  from 82803's key by one final character.

No entropy or lexicon rule separates them, because the failure is not that the string looks like a
placeholder: **it is that two different buyers typed the same one.** A shape-only gate would refuse
10 correct tenders (28 versions → islands) to fix 3.

**Decision: the gate is `placeholder-shaped ∧ the key's notices disagree on their buyer`,
with the disagreement threshold at ≥3 distinct buyers** (2 is the measured org-duplicate floor, and
raising the floor with an org-identity fix is issue 329/362's job, not this gate's). Refused ⇒ fall
back per `crates/store/src/canonical.rs:7126` (legacy OJS closure, else `island:{notice_id}`).

This is a group-level predicate, and `build_plan_groups` is exactly where it can be evaluated: it
already has every notice carrying a candidate key, so it can count their buyers before electing the
key. **That is also unit 4's plausibility gauge** — the same predicate, read rather than enforced —
so build it once, in the planner, and let the gauge report it (cross-reference
`legacy-ojs-closure-welds-unrelated-procurements` unit 3, which needs the identical count for the
other mechanism).

Blast radius at the ≥3 threshold, from the table above: **3 tenders split, 10 untouched.**

### Correction, same day: the prefix probe under-counted, and the miss argues FOR the rule

The census above probed 30 fixed first blocks. That is a **lower bound, not a complete list** — it can
only find keys whose first block is one of the 30. Sampling 796 distinct real `procedure_key`s off prod
(six id slices, `LIMIT 400` each) and running a real shape predicate over them turned up one the probe
had missed: **`00000001-2023-4000-a000-000000000001`** — tender 2, "Landkreis Göttingen - Beschaffung
Fachverfahren Kf…", 2 versions, **1 buyer**. Hand-typed, obviously placeholder-shaped, and **correctly
grouped**. Shape-only would have split it; the buyer test admits it. So the miss costs the census
completeness and *earns* the two-part rule another witness: 14 shaped tenders known, still 3 welded.

The complete corpus-wide list is not reachable from here — the predicate cannot be expressed in SQL
cheaply and a scan of `tenders` is the class of read with no compliant on-box path
(`docs/agents/prod-box-reads.md`). It arrives for free with unit 2: once the planner computes
`key_shaped` per notice, the count is a grouped read, and unit 4's gauge reports it.

### The shape predicate, chosen against real keys

`distinct(payload) <= 6`, where *payload* is the 32 hex characters with the version nibble (block 3
char 1) and variant nibble (block 4 char 1) dropped — those two are structurally fixed and would
otherwise inflate the count on a genuine placeholder.

Measured on the 796-key sample: it flags **exactly 2** keys, both genuine placeholders, **0 false
positives**. That is not luck — a v4 UUID draws 30 random hex characters, so P(≤6 distinct) is
vanishingly small; the predicate is safe by construction rather than by tuning. Alternatives measured
on the same sample: `const_blocks>=3` and `periodic` flag only 1 (they miss
`00000001-2023-…`, whose blocks are neither constant nor periodic), `maxrun>=8` flags the same 2 and
is a reasonable belt-and-braces disjunct.

Against the known keys: `00000000-…` → 1 distinct, `11111111-1111-…` → 1, `11111111-2222-4000-8111-123412341235`
(welded, 11 buyers) → 6, `22222222-…` → 1, `aaaaaaaa-…` → 3, `00000001-2023-…` → 4. All six flagged.
Rostock's `…444444444444` → 5 is flagged too and `…444444444450` → 7 is not — which does not matter,
because both have ≤2 buyers and the buyer test admits them either way. **Shape being ragged at the
edges is harmless precisely because it is only a pre-filter.**

### Why the shape pre-filter stays even though it cannot discriminate

Buyer-disagreement alone is not safe corpus-wide: a **joint procurement** legitimately names several
buyers under one BT-04 (a central purchasing body plus its participating authorities), and `≥3
distinct buyers` would refuse it. Keeping *placeholder-shaped* as a pre-condition bounds the rule's
reach to the 13 tenders measured above, where `≥3` is exactly right — and inside that set the welds
are not merely multi-buyer, their buyers are **disjoint across versions** (tender 1: seq 1–2 Klinikum
Neumarkt, 3–4 Land BW, 5–7 BG Holz und Metall), which is what a joint procurement never looks like.
So: shape bounds the blast radius, buyer-count discriminates inside it. Neither alone.

If unit 2 ever wants to drop the shape pre-filter and gate every BT-04, the predicate must become
*pairwise-disjoint buyer sets across the key's notices*, not a count — record that before widening.

### Refuted, measured: "refuse the key and let the previous-publication edges re-join it"

The tempting simplification is to drop the buyer machinery entirely — refuse a placeholder-shaped key
in `procedure_key()` (the existing extension point, right where `is_uuid` already gates the dialect
folder ids), let the notices fall to `island:{notice_id}`, and trust ADR-0011's `plan_prev_edge`
closure to put the genuinely-related ones back together. Ten lines instead of two schema columns.
**It does not work, and the measurement says why twice over.**

*First:* the citations are not there. `OPP-090-Procedure` counts over every version of all 13 shaped
tenders (via `notice_ids`) are **0 on 24 of the 28 versions measured**, 1 on four (82809 seq 3–4,
82810 seq 1–2). Refusing the key would not re-join these chains — it would shatter them into one
island per notice, permanently.

*Second, and worse:* the shaped key is carrying the **cross-source merge**. Every correct shaped
tender alternates two publication-id shapes, and the notice rows say what they are — tender 82806's
four versions are notices 26430096 and 26487992 (`source=doe`, `eforms:eforms-de-1.1`,
`27aa15a5-…-01`) paired with 24171307 and 24278203 (`source=ted`, `eforms:eforms-sdk-1.7`,
`00495356-2024`). The hand-typed key is the ONLY thing joining each German portal notice to its TED
twin — which is precisely the merge issue 34's `is_uuid` gate was built to permit. A blanket refusal
would undo issue 34's win to fix issue 369's, on the same notices.

So the buyer test is not an optional refinement over a simpler design; it is what makes the fix
possible at all. Recorded here because "just refuse it and let the closure sort it out" will look
obvious again to the next reader.

### The planner change unit 2 needs (design, 2026-09-08)

The election is one set-blind statement — `UPDATE plan_notice SET group_key = CASE WHEN procedure_key
IS NOT NULL THEN procedure_key …`, `crates/store/src/canonical.rs:7236-7240` — and `plan_notice`
(`insert_plan_tx`, `canonical.rs:7117-7131`) carries no buyer and no shape. Two additive columns fix
that, both written at plan time where the parsed notice is still in hand:

- **`buyer_key TEXT`** on `PlanRow` / `plan_notice`, from the PARSED notice, not from resolved
  `organizations` — that is the settled answer to the question below. `into_plan_row`
  (`crates/ingest/src/project.rs:3551`) builds the row from `Parsed`, so the buyer's published
  identifier (country + normalised value, through the resolver's own normaliser) is available with no
  dependency on the org layer, and the ≥3 threshold already absorbs the duplication that layer would
  add. Fall back to the N2 name key when the buyer publishes no identifier.
- **`key_shaped INTEGER NOT NULL DEFAULT 0`** — the placeholder-shape verdict computed in Rust
  (constant-run blocks, repeated short cycles, all-zero payload), because SQL cannot express it and
  because storing it keeps the refusal query a cheap grouped read over `WHERE key_shaped = 1` instead
  of a scan of every key in the plan.

Then, before the batched UPDATE, materialise the refused set —
`SELECT procedure_key FROM plan_notice WHERE key_shaped = 1 GROUP BY procedure_key
HAVING count(DISTINCT buyer_key) >= 3` — into a small `plan_refused_key` table, and add
`AND procedure_key NOT IN (SELECT procedure_key FROM plan_refused_key)` to the first CASE arm. A
refused notice falls through the existing arms exactly as designed: legacy closure if it has one,
else `island:{notice_id}`.

Repair (unit 3) is then a scoped re-plan + re-project of the three welded keys' notices;
`retire_regrouped_nonlegacy_tenders` (`canonical.rs:19525`) already retires the tenders whose key the
plan stops producing, which is precisely what happens to 1, 82802 and 82804.

### Settled, 2026-09-08

Parsed-side, not resolved: see `buyer_key` above. The census used resolved rows because that is all a
read-only probe can reach; the planner has the parsed notice and should not take a dependency on the
org layer to decide identity.

## Units 2c + 3 DONE and verified on prod (2026-09-08, rev `a83495b`, jobs 819+820)

**The gate refused exactly the census's three keys.** Log line, unconditional by design:

```
[project] group step refused-keys: 3 placeholder-shaped key(s) with >= 3 distinct buyer sets, 0.0s
```

**Repair** (unit 3) needed no new code: `refold-notices` already takes explicit ids, re-queues them
(`projected = 0`) and pairs an incremental projection. The 93 notices of tenders 1, 82802 and 82804
went through it — cap is 1,000, and the ids land in the job log so the run is attributable.

- job 819 `refold-notices`: `93 notice(s) named: re-queued 93, stamped 3 tender(s)`
- job 820 `project`: `93 notices → 91 tenders (91 islands), 93 versions; 91 tenders written`, **6 s**

**Verified against the pre-state recorded before the deploy:**

| check | result |
| --- | --- |
| tenders 1 / 82802 / 82804 | **gone** (`row_count 0`) — retired by `retire_regrouped_nonlegacy_tenders` |
| 82803, 82805, 160170, 778091 | **untouched**, same keys AND same `current_seq` (4/2/2/2) |
| notice 26244735 (old tender 1's DE-1.x member) | now tender **7962825**, `procedure_key` NULL, `island_notice_id` 26244735, title **"HPS - Heizung"** — its OWN title, not the "Ladekabel eAutos" it used to be served as |

That is the "Done when" pair satisfied on production rather than in a fixture: the shaped-but-correct
keys still key their Tenders, and the welded ones no longer key anything.

### The cost, stated plainly: this OVER-splits, and the fix for that is unit 5

93 notices became **91 islands** — one tender per notice, bar two rejoined by previous-publication
edges. For tender 1 (7 notices, 3 buyers) the issue's stated goal was "tender 1 becomes three tenders
(or three islands)"; what landed is the islands, not the three tenders. Worse at scale: 82802's 42
versions were ~7 real procurements by buyer and are now 42 separate Tenders; 82804's 44 were ~11 and
are now 44.

So three fabricated records were replaced by 91 truthful but FRAGMENTED ones. That is the safe
direction (CONTEXT.md:112-113) and it is what "falls back to `island:{notice_id}`" was always going to
do — but it is a real loss of genuine structure, not a clean win, and the cross-source doe↔ted merges
that the hand-typed key was carrying inside those groups are gone with it.

## Unit 5 (new) — group a refused key's notices BY BUYER instead of per notice

The fallback should be `refused:{procedure_key}:{buyer_key}` rather than `island:{notice_id}`. Then a
refused key splits into one Tender **per buyer set** — which is exactly the issue's original goal
("tender 1 becomes three tenders"), and it recovers most of the structure the island fallback discards:
tender 1 → 3, 82802 → ~7, 82804 → ~11, instead of 7/42/44.

Cheap, because the input already exists: `buyer_key` is stored on `plan_notice` (unit 2b) and the
refusal already reads it. This is a change to ONE `CASE` arm plus a re-run of the same
`refold-notices` repair over the same 93 ids — the current islands are re-planned, not unwound, so
there is nothing to migrate.

Two things to settle when doing it:

1. **A notice with no buyer at all** (`buyer_key IS NULL`) still needs `island:{notice_id}` — grouping
   every buyer-less notice of a refused key together would be a new weld, smaller but the same kind.
2. **Verify the doe↔ted merge survives** where both twins name the same buyer, since that merge is the
   thing issue 34's gate exists to permit and the island fallback currently breaks it.

## Done when

- the census is on this issue; ✅ 2026-09-08
- a test pins that `00000000-0000-4000-8000-000000000000` does not key a Tender **when its notices
  name three different buyers**, and that `11111111-2222-4aaa-8333-444444444444` still does key one
  (same shape, one buyer) — the pair, not the first alone, is what pins the rule the census settled;
- tender 1's three procurements are three records, each serving its own dates and its own bid;
- the gauge lists no BT-04-keyed tender with three buyers, and Rostock's ten stay whole.

*One issue because:* the served-record damage on tender 1 and the reviewer's "the nil UUID should never be accepted" both reduce to one filter — `!k.trim().is_empty()` — standing where the organization layer has a whole module.

### Units 2a + 2b landed (`7be5994`, `8f48072`) — and coupling 1's MECHANISM is the reverse of what was written

**2a — `key_shaped`** on `PlanRow` / `plan_notice`, computed in Rust at plan time from the key the row
already carries, activating the shape predicate that landed inert. Stored rather than recomputed so
2c's refusal is a grouped read over `WHERE key_shaped = 1` instead of a scan of every key in the plan.
A test pins that it is stored PER NOTICE rather than defaulted — a column silently holding 0 for every
row would leave the gate a no-op that still read as fully wired.

**2b — `buyer_key`**, and it is a **SET, sorted and joined**, which is a deliberate change from the
design recorded above. The design said one buyer per notice with the gate counting distinct buyers. But
a notice can legitimately name several, and a **joint procurement** — a central purchasing body beside
its participating authorities — is precisely the case a buyer COUNT refuses. Keying on the set makes
`count(DISTINCT buyer_key) >= 3` read as "three distinct buyer SETS": the joint procurement repeats one
set across its notices and is admitted; the welds carry buyers disjoint across versions and present
three sets. Same single column, no second one needed.

Bounded gap, documented rather than solved: notices naming overlapping but UNEQUAL subsets (`{X,Y}`,
`{X,Y,Z}`, `{X,Z}`) present three sets and would be refused despite sharing buyers. Inside the shape
pre-filter's reach — the 14 measured tenders, whose welded members are disjoint — that cannot arise.
The "pairwise-disjoint sets" note below therefore still stands as the rule for ever dropping the
pre-filter.

Both units are **inert**: nothing consumes either column yet, the election still computes `group_key`
without them, and `ops/check.sh` is green at 113 suites for both.

#### Correction: coupling 1 does not announce itself, and it fires on `OPT-300-Procedure-Buyer`

Coupling 1 below says the meta-gate "will go red, and it should". **It will not go red on its own**, and
waiting for that signal would waste a firing. `no_de1_alias_reaches_the_grouping_or_the_fold_order`
(`crates/ingest/src/project.rs`) does not detect what the code reads: `decides_the_fold` is a
**hand-maintained list**, and the assertion is only that no entry in `DE1_FIELD_ALIASES` targets
something on that list unless it is declared in the test's `IDENTITY` allowlist. Unit 2b landed with
org fields feeding a plan column and the gate stayed green — correctly, by its own construction.

So the widening must be **declared INTO the list** by unit 2c, not waited for. And the consequence is
now known rather than guessed: **`OPT-300-Procedure-Buyer` IS a `DE1_FIELD_ALIASES` target** (checked
2026-09-08 over the whole alias table). The moment 2c adds the buyer role fields to `decides_the_fold`,
the gate fires on that alias — which is exactly the review it exists to force: a DE-1.x notice's buyer
reference becoming an input to TENDER IDENTITY is a fold-impact question, not a mapping question, and it
lands on the same DE-1.x line that issue 34's `is_uuid` gate serves.

**Unit 2c therefore owes a decision, not just an edit:** either declare
`("DE1-…-Buyer", "OPT-300-Procedure-Buyer")` in `IDENTITY` with the fold-impact reasoning written down,
or scope `buyer_key` so the DE-1.x alias is not one of its inputs. Recording it here because a gate that
must be *fed* before it can protect you is the same hazard as a gate that stopped running: silence reads
as safety either way.

#### Unit 2c's coupling decision, settled 2026-09-08: DECLARE it, and here is the argument that holds

The alias is `("DE1-ContractingParty-Party-PartyIdentification-ID", "OPT-300-Procedure-Buyer")`. Unit 2c
must add the buyer role fields to `decides_the_fold`, the gate will then fire on this alias, and the
choice is: declare it in `IDENTITY` with fold-impact reasoning, or scope `buyer_key` so DE-1.x buyer
refs are not inputs.

**Decision: declare it.** And the reasoning matters, because the obvious argument for it is FALSE and
was checked before being written down. The tempting claim is "the DE-1.x alias is necessary to refuse
tender 1, whose seventh notice is DE-1.x (26244735)". **It is not.** Tender 1's three buyers are each
carried by at least two notices (seq 1–2 Klinikum Neumarkt, 3–4 Land BW, 5–7 BG Holz und Metall), so
dropping the single DE-1.x notice still leaves three distinct buyer sets and the key is still refused.
The fix works on the motivating tender either way.

The argument that does hold is about the failure DIRECTION. The gate counts buyer disagreement, and
excluding one dialect's buyers makes that count **dialect-dependent**: a key whose weld is visible only
through its DE-1.x notices would be UNDER-counted and silently admitted. Under-refusing is the silent
direction — a weld that keeps serving one fabricated record — whereas over-refusing splits, which
CONTEXT.md:112-113 names as the safe direction. A buyer is a buyer; which vocabulary published it is
not a property of the procurement.

The fold-impact answer the gate demands: **the blast radius is unchanged.** DE-1.x buyers only ever
reach the election through the refusal, and the refusal is gated on `key_shaped = 1` — the 14 measured
tenders. No other tender's grouping can move, whatever the alias contributes. That is what makes this a
declaration rather than a re-grouping.

### Two couplings unit 2 must pay for, found while reading (2026-09-08)

1. **`no_de1_alias_reaches_the_grouping_or_the_fold_order`** (`crates/ingest/src/project.rs`) is a
   standing meta-gate holding the allowlist of fields that "decide the fold". A buyer-aware key
   election makes ORGANIZATION fields (`BT-500`/`BT-501`/`BT-514`, `TED-OFFICIALNAME`,
   `TED-NATIONALID`, the DE-1.x aliases) inputs to TENDER IDENTITY for the first time. That test will
   go red, and it should — the widening is the decision, and it must be declared there rather than
   worked around.
2. **`PROJECTION_EPOCH`** (`crates/store/src/canonical.rs`) must be bumped so standing tenders
   re-fold, and `crates/ingest/tests/fixtures/golden/project_apply.snapshot` will move because its
   `tenders` digest covers `procedure_key` and `island_notice_id`. Per that module's header the golden
   is regenerated only for a deliberate, reviewed derived-layer change — which this is, so say so in
   the commit rather than quietly re-recording it.

#### Correction to coupling 2, 2026-09-08: unit 2c does NOT need the epoch bump

Coupling 2 says `PROJECTION_EPOCH` "must be bumped so standing tenders re-fold". **It must not, and
bumping it would buy a 6-hour corpus rewrite for nothing.**

`PROJECTION_EPOCH` gates one thing (`canonical.rs:8748`): whether a tender whose stored version chain is
UNCHANGED may early-return. Its own doc gives the condition for a bump — "the same notices now fold to
different content", i.e. changed fold logic over an unchanged grouping. Unit 2c changes the GROUPING:

- the three welded tenders lose their key, so their notices form NEW groups under new ids and fold
  against an empty stored chain; the old tenders are retired by `retire_regrouped_nonlegacy_tenders`;
- the correctly-grouped shaped tenders keep key AND content, so early-returning them is correct;
- a refused notice joining an existing legacy component changes that component's stored chain, which
  the `keep < stored.len()` comparison already catches.

None of those paths reads the epoch. The cost avoided is real: the constant's doc measures a bump at
**6 h 02 m / 14.2 M version writes for a 2.69 M-notice cohort** (issue 179).

**Knock-on:** issue 366 had recorded that its own standing-row re-fold could ride this bump. That is
retracted there — 366's epoch-vs-repair decision is live again.

**The golden claim in coupling 2 is conditional, not automatic.** The snapshot moves only if a FIXTURE
carries a placeholder-shaped key whose notices name ≥3 distinct buyer sets. Units 2a and 2b both landed
with the golden unmoved, which is evidence the fixture corpus holds no such key. Do not pre-authorise a
regeneration: gate 2c, and if the snapshot does not move, that is the informative result (the change
reaches only the welded class) rather than a step skipped.

Buyer-key details settled by the same reading: the buyer role is `Procedure-Buyer` (eForms and
eForms-DE 1.x, via the `DE1-ContractingParty-Party-PartyIdentification-ID` alias) or `buyer` (legacy
TED `ADDRESS_CONTRACTING_BODY` / `…_ADDITIONAL` / `CA_CE_CONCESSIONAIRE_PROFILE`, and the DÖE sdk-0.1
synthesis) — the corpus-wide `role LIKE '%uyer%'` predicate every consumer already uses. The mention's
country and identifier are ALREADY normalised at plan time: `NoticeState::mentions` runs
`canonical_country` then `normalise_identifier` before `into_plan_row` is called in the same loop, so
`buyer_key` costs a join from role-target section id to mention section id through
`nested_org_aliases`, and no new normalisation. For the identifier-less fallback prefer `match_norm`
(the N2 key) over `organizations.name_norm`: it folds harder, so it errs towards saying two notices
AGREE about their buyer, and every error in that direction is a weld left standing rather than a
correct tender split.
