# 486 — an eForms notice citing a qualification-system notice or PIN (OPP-090) welds unrelated procedures into one Tender

Status: ready-for-agent — NEXT: unit 1: stamp the cited notice's OWN shared-publication kind for eForms too (BT-02
notice type / OPP-070 subtype: qualification system, prior information, buyer profile, periodic indicative, DPS) and
refuse a previous-notice link (481's ledger edge, same- and cross-Source) when either end is such a notice; count the
refusals per kind on the job row; then re-queue the welded Tenders and let a fold split them.
Kind: data correctness (a false merge)
Relates to: 364 (the cited-type gate, legacy-only today: `shared_kind` is stamped only for legacy rows), 481 (the
OPP-090 previous-notice producer and its guards: buyer-disjoint, fan-in, not-earlier — none reads the cited type),
482 (BT-04 hubs; a different weld), the served Tenders

## What is wrong

Found 2026-10-05 by the 481 job-row field `largest component … at notice` (project 1974/1978: `largest component:
226 key(s) at notice 24682900`). Tender **202112** holds **234 versions** of unrelated Endesa/ENEL procurements, each
with its OWN BT-04 procedure key, e.g.:
- 24682900 `S - ICT - ES - Licitación Mto Nice` (BT-04 db9baf44-…)
- 24704700 `SERVICIO_MURO DE ALCOY (ALICANTE)_INST. MANT ALUMBRADO ESE` (7bace04e-…)
- 47155819 `SUMINISTRO DE GRUPOS ELECTROGENOS UPH NOROESTE, UPH EBRO PIRINEOS, UPH SUR` (b349a916-…)
- 46326776 `CH Jabarrella REPOWERING`
All of them cite OPP-090 `206469-2025` = notice 24716938, `Sistema de Clasificación de Proveedores del Grupo ENEL`
(BT-02 `qu-sy`, OPP-070 `15`): a qualification-system notice, which by definition is the call for MANY procurements.
481's previous-notice edge joins every citer to it, and through it to each other (same buyer, so the buyer-disjoint
guard passes; fan-in is cross-Source only).

364 unit 6 already refuses exactly this shape for the legacy era ("one PIN welded hundreds of unrelated
procurements"), but its `shared_kind` is stamped only on legacy plan rows (`project.rs` `shared_kind: legacy.then(…)`),
and 481's link step does not consult it.

## Fix (unit 1)

- Stamp `shared_kind` for eForms rows from the notice's own type (BT-02: `qu-sy`, `pin-*`, `pin-buyer`, …; or the
  OPP-070 subtype table) with the same kind vocabulary as `shared_publication_kind`.
- In the 481 link step, refuse a previous-notice edge whose cited OR citing notice carries a `shared_kind` (count as
  `shared-kind` per kind on the `issue-481` line). A citing qualification-system notice that cites last year's is the
  same shape from the other side (364's reasoning).
- Re-queue: the Tenders whose components contain such an edge (202112 and its kin) — a dry/wet requeue like
  `requeue-uuid-hubs`, or projection-epoch stamping of the affected notices.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/202112 | jq '.versions | length'

- **open** (2026-10-05): `234`.
- **done:** a handful (the qualification-system notice and its own amendments), each award on its own procedure's
  Tender; the `issue-481` line's largest component well below 226.
