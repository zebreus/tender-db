# 479 — a Tender carries no procedure type (BT-105), in any source: the filter procurement analysis reaches for first is missing

Status: ready-for-agent — filed 2026-10-01 15:5x UTC from closing 465, which called it "a separate model question …
out of scope". Nothing owns it. The first unit is the decision and the census: which code list, which scope
(procedure, and whether per lot), how every era maps onto it, and what the backfill costs. Measure before building.
Kind: data model / API (a core attribute that is parsed and then dropped)
Relates to: 465 (FTS now parses BT-105 into the notice layer), 397 (contract nature, folded from every era: the
template), `.scratch/tender-db/api-dq-review-2026-09-15.md` (row dq-eforms: "BT-105, BT-23, BT-01, BT-36, BT-765/766,
BT-60 … have no place on the Tender", marked "not recorded")

## What is wrong

Open, restricted, negotiated with or without a call, competitive dialogue, innovation partnership, direct award:
the procedure type decides who could bid. It is the first filter a market analyst or a transparency reader applies
("show me the direct awards above €1 M"). Every source publishes it:
- eForms: BT-105-Procedure, `procurement-procedure-type`.
- DÖE: eForms-DE and sdk-0.1.
- FTS: `procurementMethodDetails`, mapped onto BT-105 since 465 (`b629d0b`).
- TED's legacy eras: the r209 standard forms' section IV.1.1 / `PROCEDURE` element, and r208's equivalents.
- The text era: a header field (to check).

tender-db parses it into the notice layer (`/v1/notices/{id}/content`) and drops it at the fold. Read 2026-10-01:
`/v1/tenders?limit=1` items carry `country, cpv, dispatched_at, id, kind, lots, notice_subtype, original_lang,
procedure_key, publication_id, published_at, source, submission_deadline, submission_deadline_scope, title, value,
version`. The detail adds satellites (`classifications` with `cpv | nuts | nature`, `amounts`, `dates`, `parties`, …),
and none of them is a procedure type. `crates/ingest/src/project.rs` has no reference to BT-105. A reader can only get
the procedure type by fetching and walking every notice's content.

## First unit (decide, then measure)

1. **Representation.** Prefer a classification scheme `procedure` (BT-105 codes, `field = 'procedure'`) beside 397's
   `nature`. It needs no new table: `tender_version_classifications` already versions per tender, it is already
   filterable, and it already folds through the supersession rule. Decide the list filter's name
   (`?procedure_type=open`), whether lots can differ (eForms has no lot-level BT-105; FTS has one procedure), and how
   the legacy codes map onto eForms' list (r209's `PT_OPEN`, `PT_RESTRICTED`, `PT_NEGOTIATED_WITH_PRIOR_CALL`,
   `PT_AWARD_CONTRACT_WITHOUT_CALL`, …). Unmappable legacy values go in an explicit "kept as published" column or are
   left unmapped and counted, never guessed.
2. **Census.** Count per profile how many notices publish a procedure type, with a bounded read per profile through
   the notice layer's field index, or a dry job. Record the per-era coverage this would give.
3. **Cost.** Adding a classification row per tender version needs the fold to rewrite every tender: a
   whole-corpus `project rebuild` (job 1616 took 442 min) or a scoped `refold-fields` per profile. Pick by measurement,
   off the daily tick.

Then build: fold, API filter, docs and OpenAPI, tests per era (one fixture each), and the refold.

## Verify

    curl -s 'https://tenders.zebreus.click/v1/tenders/8576017' | jq -c '[(.classifications // [])[] | select(.scheme == "procedure") | .code]'

- **open** (2026-10-01): `[]`. The notice says `BT-105-Procedure=open`, and the tender carries nothing.
- **done:** `["open"]`, and `/v1/tenders?procedure_type=open&limit=1` answers with that filter applied, not ignored.
