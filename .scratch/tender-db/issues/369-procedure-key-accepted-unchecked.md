# 369 — a published BT-04 becomes the Tender group key verbatim: an all-zero v4 UUID glues seven notices from three buyers into one served record

Status: ready-for-agent (filed 2026-09-07; unit 1 census done 2026-09-08 — it revised unit 2's rule)
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

## Done when

- the census is on this issue; ✅ 2026-09-08
- a test pins that `00000000-0000-4000-8000-000000000000` does not key a Tender **when its notices
  name three different buyers**, and that `11111111-2222-4aaa-8333-444444444444` still does key one
  (same shape, one buyer) — the pair, not the first alone, is what pins the rule the census settled;
- tender 1's three procurements are three records, each serving its own dates and its own bid;
- the gauge lists no BT-04-keyed tender with three buyers, and Rostock's ten stay whole.

*One issue because:* the served-record damage on tender 1 and the reviewer's "the nil UUID should never be accepted" both reduce to one filter — `!k.trim().is_empty()` — standing where the organization layer has a whole module.
