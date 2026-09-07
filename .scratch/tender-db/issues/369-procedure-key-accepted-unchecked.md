# 369 — a published BT-04 becomes the Tender group key verbatim: an all-zero v4 UUID glues seven notices from three buyers into one served record

Status: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
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

## Done when

- the census is on this issue;
- a test pins that `00000000-0000-4000-8000-000000000000` does not key a Tender;
- tender 1's three procurements are three records, each serving its own dates and its own bid;
- the gauge lists no BT-04-keyed tender with three buyers.

*One issue because:* the served-record damage on tender 1 and the reviewer's "the nil UUID should never be accepted" both reduce to one filter — `!k.trim().is_empty()` — standing where the organization layer has a whole module.
