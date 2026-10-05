# 486 — an eForms notice citing a qualification-system notice or PIN (OPP-090) welds unrelated procedures into one Tender

Status: ready-for-agent — UNIT 1 DONE 2026-10-05 (qualification systems refuse previous-notice links; deployed `9f9db13`; wet 1994 + project 1995; Verify 202112: 234 → 6 versions). NEXT: unit 1b (a PIN / periodic / buyer-profile notice refuses only when ≥ 2 keyed components cite it), and the 200-key component at notice 24090776. Was: ready-for-agent — NEXT: gate (`ops/check.sh`) with 487's budget change, then commit and deploy unit 1. On
prod, run the dry `backfill-tender-links` and apply the go/no-go rule under "Review fixes" to `would_split_shared` and
`would_split_shared_kinds`. Then the wet run, the next daily, and the Verify below.
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

## Unit 1 — landed (not deployed)

**What changed.**
- `crates/ingest/src/project.rs`: `Ident::read` stamps `shared_kind` for every non-legacy row through
  `eforms_shared_kind(subtype, notice_type)` — `SHARED_EFORMS_SUBTYPES` on `OPP-070-notice` (DE-1.x folded
  onto it), falling back to `SHARED_EFORMS_TYPES` on `BT-02-notice` / `SDK01-NoticeTypeCode` when a notice
  carries no subtype. Same vocabulary as `SHARED_DOC_TYPES`. The mapping, each family pinned by the fields only
  it may carry in the vendored `sdk/fields-1.15.0.json` (read back by the unit test
  `shared_subtypes_follow_the_sdk_field_constraints`):
  - `1`-`3` `NOTICE_BUYER_PROFILE` (`pin-buyer`): BT-508 buyer-profile URL mandatory there and only there;
  - `4`-`6` PIN only: the only subtypes allowed the PIN's `…-Part` fields;
  - `7`-`9` PIN to shorten time limits: with 4-6 the only ones allowed BT-127 future notice;
  - `15` `NOTICE_QUALIFICATION_SYSTEM` (`qu-sy`; 24716938 itself);
  - in 1-9 the legal bases run 2014/24, 2014/25, 2009/81 inside each triplet (the defence subcontracting
    fields BT-64/65/651/729 are allowed in `9` and `18` only). So `2` is the utilities buyer-profile notice
    (`NOTICE_BUYER_PROFILE`), only `5` and `8` read `PERIODIC_INDICATIVE_NOTICE` (legacy `M`/`P`), and `4`, `6`,
    `7`, `9` read `PRIOR_INFORMATION_NOTICE`.
  - NOT flagged (review fix 1): `10`-`14`, the PIN used as a call for competition (the only subtypes allowed
    BT-631; `14` is the 2014/23 one, since BT-740 is mandatory on it with 19, 32, 35). It opens one procedure,
    exactly the CN-to-CAN shape 481 joins. Its legacy twin `A` was never measured. Also not flagged: the transport
    PIN `T01`/`pin-tran`, the national `E1`/`E2` planning notices (unmeasured), 16-40, and the rest of the
    national tail. eForms has no DPS-call subtype.
- `crates/store/src/canonical.rs`, link step: `LINK_EDGE_JOIN_SQL` reads both ends' `shared_kind`; a
  previous-notice edge (same- or cross-Source) with either end stamped is refused after `not-earlier`,
  before `buyer-disjoint` and the fan-in count, and counted in the new `LinkTally::shared_kind` (in
  `refused()`, `add()`, the stderr line). **One place**: the 364 gate keeps `WHERE legacy = 1` — it governs
  `plan_ojs_edge` (legacy OJS chains; eForms rows have no `ojs_self`), the link step governs ledger edges,
  whose legacy ends were already stamped. No edge passes both gates. **Not "legacy unchanged" for ledger
  edges:** an eForms OPP-090 citing a legacy `0`/`A`/`P`/`M`/`B`/`O`/`Q`/`Y` notice is refused now, where before
  486 the link step read no plan row's kind and joined it (review fix 4; pinned by a test). The verdict reads the two endpoints' plan
  rows only, so it never defers and is the same on a full and an incremental plan.
- Plan semantics changed, so a plan from before 486 is not resumable: `clear_plan_on` creates the empty
  marker `plan_eforms_shared_kind`, `plan_is_complete` refuses a plan without it (483u2's pattern).
- `crates/app/src/supervisor.rs`: `link_suffix` prints `shared-kind N` among the refusals (test updated);
  the `tender-link-backfill` body, summary and progress line carry `shared_kind` / `would_split_shared`.
- **Re-queue (reused, no new job):** the ledger backfill's census (`LinkEndpoint::shared_kind`, from the
  same `Ident::read`) treats a shared-kind end as the fold does: of `would_merge` it is `shared_kind`
  (nothing re-queued), and a pair one Tender today under different keys is `would_split` +
  `would_split_shared` — both ends re-queued by the wet run, split by the next daily.
- `docs/operations.md`: "The shared-publication refusal (issue 486)", the prod re-queue steps and the go/no-go rule.
- Stack budget (issue 467): the census's grown future is built and boxed in its own fn
  (`declared_window_boxed`), so the walk's poll frame holds a pointer. No `run_spec` arm added.
  **Pre-existing, not 486:** the whole `--lib` run aborted on the link-backfill gauge, on HEAD `3f66efe` as
  well. Filed and fixed as issue 487 (review fix 5).

**Tests.** `a_qualification_system_notice_does_not_weld_the_awards_citing_it` (project_incremental; since the review it also asserts the per-kind census `[(NOTICE_QUALIFICATION_SYSTEM, 4, 4 samples)]`) and its sibling `the_shared_kind_refusal_reads_the_citing_end_and_legacy_targets_but_not_a_pin_used_as_a_call` (review fix 6): QS
notice + three TED awards (distinct BT-04) + a DÖE award citing it by OPP-090, and a CN/award control.
Folded with the QS untyped → one welded Tender; re-parsed with OPP-070 `15` / BT-02 `qu-sy` → dry census
`would_split 4 (shared 4)`, 5 to re-queue; wet; the daily splits into the QS Tender + four, identical to the
full fold (`absorb_and_compare`), `shared_kind 4`, control `previous_notice 1`, nothing retired; a later
citer refused on both paths. `shared_subtypes_follow_the_sdk_field_constraints` (the table vs the SDK);
the resume test gains `pre-486`; `the_link_suffix_…` and the backfill-job test updated. The legacy 364 tests
and every projection test binary pass unchanged.

**Cost to accept.** An award that cites its own PIN (PIN only, or a PIN to shorten time limits) under ANOTHER BT-04
no longer joins it. A shared BT-04 still joins them, and a PIN used as a call for competition is not flagged (review
fix 1). A QS amendment under a new BT-04 is apart from its QS. The dry census's `would_split_shared_kinds` shows per
kind how many Tenders that splits, with samples. Read it before the wet run (the rule is under "Review fixes").

**Prod (after deploy).** Run `backfill-tender-links` dry. Read `would_split_shared` and `would_split_shared_kinds`, and
apply the go/no-go rule. Then the wet run, the next daily, and the Verify.

## Review fixes (2026-10-05)

1. **Subtypes 10-14 (PIN as call for competition): fixed.** They were dropped from `SHARED_EFORMS_SUBTYPES`, and
   `pin-cfc-standard`/`pin-cfc-social` from `SHARED_EFORMS_TYPES`. A pin-cfc opens one procedure (BT-631 is allowed
   there only), and its award's OPP-090 is the CN-to-CAN link 481 exists for. Legacy `A`, its twin, was never
   measured by 364. 7-9 (pin-rtl) stay flagged: they are planning notices like 4-6 and legacy `0` (F01, measured at
   2,161 in 2011), and the call comes later as a contract notice. DPS: a DPS opened by a pin-cfc now stays joined,
   as one opened by a CN (16/17) does. The test pins it: `10`-`14` map to none, and the award citing a pin-cfc joins
   it on both the full fold and the daily.
   **Measuring per kind (also fixed):** the census now reports `shared_split_kinds` (body
   `would_split_shared_kinds`): per kind, the pair count and its own reservoir of up to 30 samples (`LINK_BACKFILL_SAMPLES`), so PIN splits
   read apart from the ~234 qualification-system ones.
2. **The legal-basis order in the comment: fixed.** `2` is the utilities buyer-profile notice. The PIN triplets
   (1-9) run 2014/24, 2014/25, 2009/81. The pin-cfc line now names 10 (2014/24), 11 (2014/25), 12-14 (2014/24,
   2014/25, 2014/23), with BT-740's mandatory list (14, 19, 32, 35) cited as the evidence for 14 and asserted by
   the SDK test.
3. **`pin-tran`/T01: fixed.** `pin-tran` was removed from the type table, so both tables agree that the transport
   PIN is not flagged (a Reg. 1370/2007 PIN announces one planned award). `E1`/`E2` are named in the comment as
   national planning notices left unflagged on purpose. The DE1 notice-type field is not folded onto BT-02: it
   cannot matter while a DE-1.x notice always carries its subtype, which decides. The test asserts `T01`, `T02`,
   `E1`-`E6`, `X01`, `X02`, `CEI` and the `pin-cfc-*`/`pin-tran` types all map to none.
4. **Legacy-cited ledger edges: fixed (docs + test).** The issue and `docs/operations.md` now state that an eForms
   OPP-090 citing a legacy shared-kind notice is refused. The new test pins it on the full fold, the daily and the
   census.
5. **The gate was red: fixed, filed as issue 487.** This is not caused by 486: `3f66efe` aborted identically in a
   detached worktree. A paint gauge measured the four window walks' deep first-poll path at 422-429 KiB (UUID-hub
   requeue 237, FTS audit 336). The 192 KiB budgets had come from the shallow path. The budgets are now the deep path
   plus ~24 KiB (456/264/360 KiB), in line with the seeded buyer-role gauge's 456. Two whole-`--lib` runs were green.
6. **No test for a stamped citing end or a legacy cited end: fixed.** Added
   `the_shared_kind_refusal_reads_the_citing_end_and_legacy_targets_but_not_a_pin_used_as_a_call`. An eForms PIN
   (OPP-070 `4`) citing an earlier CN is refused. An award citing a legacy `TD` `0` notice is refused. An award
   citing a pin-cfc (`10`) joins it. The test covers the full fold and the daily (`shared_kind 2, previous_notice 1`
   on both) and the census (`would_merge 2, shared_kind 2, requeued 0`). Both of the reviewer's mutations were run
   and both fail the test: dropping the fold's citing-end check, and dropping the census's citing-end fallback.
7. **The subtype-2 text: fixed** (same as 2), in both the issue and the code comment.
8. **The pin-tran disagreement: fixed** (same as 3).
9. **No go/no-go rule for the wet run: fixed** in `docs/operations.md`.
   - **Expected:** the qualification-system kind in the hundreds to low thousands.
   - **Check:** open about 10 samples per kind on TED.
   - **Go:** total `would_split_shared` ≤ 5,000 and at most about 1 in 10 samples per kind a correct merge.
   - **Stop:** otherwise. Report the counts here and decide per kind. A kind that must not split has to leave the
     table in code, because the fold refuses it on every re-planning daily whether or not the wet run happens.
   - The docs also say that the wet run carries the pending 481 `would_merge` and `stale` re-queues along.

## 2026-10-05 05:4x–06:1x UTC — deployed `bb87cdc`; dry 1982 → STOP for PINs; unit 1a narrows the refusal

Dry `backfill-tender-links` 1982 (report `.scratch/tender-db/481-dedup/backfill-1982.json`): would_split 12,115, of
which **would_split_shared 12,112** — PRIOR_INFORMATION_NOTICE 10,277, PERIODIC_INDICATIVE_NOTICE 951,
NOTICE_QUALIFICATION_SYSTEM 621, NOTICE_BUYER_PROFILE 263; shared_kind (not yet joined) 977. Over the go/stop rule's
5,000 → **STOP, no wet run.** A PIN cited by its own procedure's CN/CAN is mostly that one procedure announced early;
splitting 10k such pairs would break correct merges. And the fold refuses every re-planned edge, so the daily would
have started splitting them: **unit 1a (this commit)** limits the REFUSAL to the two kinds that by definition publish
many procurements — `store::link_refuses_shared_kind`: NOTICE_QUALIFICATION_SYSTEM and NOTICE_BUYER_PROFILE (884
pairs in 1982's census). PIN and periodic-indicative stay stamped (`shared_kind` on the plan row), are no longer
refused, in the fold and the census alike. NEXT: deploy 1a, dry again (expect would_split_shared ≈ 884), read 10
samples per kind, wet, daily, Verify 202112. **Unit 1b:** a PIN / periodic notice refuses only when cited by ≥ 2
keyed components (the same-Source analogue of 481's fan-in guard).

## 2026-10-05 06:3x–07:0x UTC — 1a deployed (`f6f8e5d`); dry 1983; unit 1c: qualification systems only

Dry 1983 (`481-dedup/backfill-1983.json`): would_split 892, would_split_shared 884 (QUALIFICATION_SYSTEM 621,
BUYER_PROFILE 263). The buyer-profile samples hold correct merges: 00261430-2024 (subtype 1, "Construcción de un
edificio de 178 viviendas VPPA…", Agencia de Vivienda Social de Madrid) → 00364806-2024 (CN "178 VPPA PARC FR-63
MOSTOLES"): one procedure announced on the buyer profile. A buyer-profile notice is a PIN published on the profile:
same rule as a PIN (unit 1b's fan-in). **Unit 1c:** `link_refuses_shared_kind` = qualification system only. NEXT:
gate, deploy after the 07:35 daily, dry (expect ≈ 621 shared splits), read QS samples, wet, next fold, Verify 202112.

## 2026-10-05 07:2x–08:0x UTC — 1c deployed (`9f9db13`); dry 1993; WET 1994

Daily 1991 under 1c: `shared-kind 0` refused (no re-planned QS edge that day), largest component 200 at notice
24090776 (to inspect after the split). Dry 1993 (`481-dedup/backfill-1993.json`): would_split 632, **would_split_shared
624, all NOTICE_QUALIFICATION_SYSTEM**; samples cite a few QS notices many times each (206469-2025 ENEL ×6 in 30,
247665-2026, 272953-2024, 282697-2026 ×2) — the many-procurements shape. Under the rule's 5,000 → **wet 1994**, then
an incremental project. NEXT: Verify 202112; read the 200-key component.

## 2026-10-05 ~09:00 UTC — UNIT 1 DONE on prod (qualification systems)

- Wet 1994: 745 notices re-queued. Project 1995: 745 notices → 741 Tenders; `refused: 673 (shared-kind 665,
  buyer-disjoint 8)`; largest component of that fold 26 keys.
- **Verify: done** — `/v1/tenders/202112` now holds **6 versions** (was 234): `Sistema de Clasificación de
  Proveedores del Grupo ENEL - Obras` and its own amendments; each award sits on its own procedure's Tender.
- OPEN, ready-for-agent: **unit 1b** — a PIN / periodic-indicative / buyer-profile notice refuses only when cited by
  ≥ 2 keyed components (the many-procedures shape; dry 1982 showed 10,277 PIN pairs, mostly 1:1 and correct). Also
  read daily 1991's `largest component: 200 key(s) at notice 24090776` for its shape.

## 2026-10-05 09:0x UTC — the 200-key component is unit 1b's shape

Daily 1991's `largest component: 200 key(s) at notice 24090776` = Tender **8813166**: **405 versions**, one buyer
(Polska Grupa Górnicza S.A.), subtypes 17 ×243 / 30 ×160 / **8 ×2** (utilities periodic indicative). Its notices
carry distinct BT-04 keys (7fd2df6b…, ec0269cd…, 55528cfe…, c69ad6f6…) and cite different OPP-090 targets
(643900-2024, 158369-2023, 396975-2026, 355936-2025 — the last is a PERIODIC_INDICATIVE sample of dry 1982): PGG's
periodic indicative notices are each cited by many procedures, and procedures cite each other in chains. Unit 1b
(fan-in on PIN-kind targets) is the fix; launched 2026-10-05.
