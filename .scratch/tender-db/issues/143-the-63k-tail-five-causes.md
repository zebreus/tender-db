# Diagnosis 3: the 63,037-row eForms residue after the three parser fixes (rev c6a7382)

Status: closed-diagnosis (historical record) — wave 3 (the 63K tail's five constructs); acted on,
counted in the ledger's "eForms SDK 1.0-1.11" entry

Method: same as issues 141/142. Twelve outstanding members sampled stratified
from prod quarantine (`reason='unknown-customization'`, `reprocessed_at IS NULL
AND skipped_at IS NULL`) across four fetches, extracted from the archive tars
(one sequential pass per tar), and fed through `ingest::profile::dispatch` →
`ingest::process::parse_payload` via `crates/ingest/examples/diag39.rs` in this
clone (/tmp/diag-39c, HEAD = c6a7382 — all three prior fixes are in).

## Per-SDK split of the 63,037 (prod, grouped query, sums exactly)

| profile | outstanding |
|---|---|
| eforms-sdk-1.7 | 41,027 |
| eforms-sdk-1.8 | 17,439 |
| eforms-sdk-1.9 | 1,693 |
| eforms-sdk-1.6 | 1,130 |
| eforms-sdk-1.10 | 896 |
| eforms-sdk-1.11 | 573 |
| eforms-de-1.1 | 142 |
| eforms-de-1.2 | 99 |
| eforms-sdk-1.0 | 29 |
| eforms-sdk-1.5 | 6 |
| eforms-sdk-1.2 | 3 |

(The 10s `/v1/sql` cap forces id-range chunking; nine chunked grouped queries,
totals add to 63,037 exactly. The 1.7 tail spreads evenly, ~2-3K per monthly
fetch, across fetches 18-33 and 446-450 — no single-period cluster.)

## Per-sample verdicts (harness, real member bytes, at c6a7382)

| member (fetch, period) | SDK | lang/country | outcome | cause |
|---|---|---|---|---|
| 20240723_142/00440551_2024.xml (23, 2024-07) | 1.7 | SPA/ESP | QUARANTINED ambiguous-field: CER/ExecutionRequirementCode ∈ [BT-736,743,744,764,OPT-060] | A |
| 20240723_142/00440573_2024.xml (23) | 1.7 | SPA/ESP | same | A |
| 20240723_142/00442578_2024.xml (23) | 1.7 | SPA/ESP | same | A |
| 20250103_002/00002301_2025.xml (446, 2025-01) | 1.7 | SPA/ESP | same | A |
| 20250103_002/00003343_2025.xml (446) | 1.7 | DEU/DEU | QUARANTINED unclaimed-content: Lot CER/cbc:Description | E |
| 20240723_142/00441127_2024.xml (23) | 1.8 | FRA/FRA | QUARANTINED unclaimed-content: Lot TenderValidityPeriod/cbc:EndDate | B |
| 20240723_142/00442074_2024.xml (23) | 1.8 | BUL/BGR | QUARANTINED unclaimed-content: NoticeResult/LotTender/SubcontractingTerm/efbc:TermCode | D |
| 20241209_239/00749507_2024.xml (18, 2024-12) | 1.8 | BUL/BGR | same | D |
| 20241209_239/00749909_2024.xml (18) | 1.8 | BUL/BGR | same | D |
| 20240723_142/00441278_2024.xml (23) | 1.9 | FRA/FRA | QUARANTINED unclaimed-content: Lot TenderingProcess/ProcessJustification/cbc:Description | C |
| 20240723_142/00441436_2024.xml (23) | 1.9 | FRA/FRA | same | C |
| 20250325_059/00192492_2025.xml (448, 2025-03) | 1.9 | ITA/ITA | same | C |

No sample PARSED — the residue is genuinely outstanding at c6a7382, and every
verdict line is from the harness on the real bytes.

## Causes, with minimization proofs (each flips ONE member to PARSED)

### A (4/12; 4/5 of the sdk-1.7 sample): the `permission` publisher quirk — issue 141's known cause, still the 1.7 bulk

`<cbc:ExecutionRequirementCode listName="permission">` under
`cac:ContractExecutionRequirement`. Defined in NO SDK minor 1.0-1.15
(`permission` is BT-63/BT-769's codelist); Spanish publisher (all four samples
SPA/ESP, two fetches six months apart — a persistent platform emitter, not a
one-off batch). Walker: no exact leaf predicate matches → relaxed by name →
five candidates → `ambiguous-field`.
Minimization: deleting only the `permission` CER blocks flips 00440551 to
PARSED (22/116). Matches issue 141's minimization of three different members.
NOT claimable under any inventory — needs the policy decision issue 141 already
scoped (claim-and-store as a recognised publisher-error construct, or a
documented skip).

### B (1/12): `cac:TenderValidityPeriod/cbc:EndDate` — tender validity as a date, in NO SDK

BT-98 (tender validity) is `cbc:DurationMeasure` in every minor 1.7-1.15;
no minor declares an EndDate leaf there. A French publisher writes the validity
deadline as a date instead of a duration. Walker: the TVP branch exists (BT-98)
but has no EndDate leaf and no by-name candidate → `unclaimed-content`.
Minimization: deleting the TVP/EndDate block flips 00441127 to PARSED (31/156).
Fix direction: a `UBL-` gap-fill leaf (e.g. `UBL-TenderValidityDeadline`,
kind `date`) on the TVP branch — same pattern as `UBL-InvitationSubmissionDeadline`,
which exists for exactly this "SDK enumerates one temporal shape, publishers
send the other" class. Mechanical and low-risk.

### C (3/12; 3/3 of the sdk-1.9 sample): bare Lot-level `ProcessJustification/cbc:Description` — the 1.12+ shape on older minors, AND a scope miss in the c6a7382 fix

Lot-scoped `cac:TenderingProcess/cac:ProcessJustification` holding only a
`cbc:Description` (real free text — French and Italian buyers; e.g. provisional-
guarantee terms), no `cbc:ProcessReasonCode`. Two independent reasons this
should already parse but doesn't:
1. **Later-SDK shape on an older minor (the BT-803/OPT-060 mechanical class):**
   at 1.12.0 BT-745-Lot's xpath drops its predicate and becomes exactly
   `.../cac:ProcessJustification/cbc:Description`; ≤1.11 requires
   `[cbc:ProcessReasonCode/@listName='no-esubmission-justification']`.
2. **The c6a7382 gap-fill never reaches the Lot branch:** the
   `UBL-ProcessJustificationDescription` fix is a direct `insert_extra` call in
   `index.rs::build`, not an `EXTRA`-table entry — and the alias loop that
   replicates `/*/cac:TenderingProcess` onto
   `/*/cac:ProcurementProjectLot[Lot]/cac:TenderingProcess` replicates only
   `sdk.fields` and the `EXTRA` const. The procedure-level bare PJ is fixed;
   the Lot-level one still exact-matches the (alias-replicated) predicate-free
   PJ graft branch and dies unclaimed.
Minimization: deleting the bare PJ block flips 00441278 to PARSED (19/102).
Fix direction: claim the Lot-level bare `PJ/cbc:Description` — either as
BT-745-Lot under its 1.12+ shape (gap-fill, like the OPT-060 fix), or by moving
the existing `UBL-ProcessJustificationDescription` insert into the `EXTRA`
table so the alias loop replicates it. The 1.12+ inventory precedent argues for
BT-745-Lot: this content IS the field later SDKs declare there.

### D (3/12; 3/4 of the sdk-1.8 sample): `efac:SubcontractingTerm/efbc:TermCode` without `@listName` — BT-773 with its discriminator dropped

`<efac:SubcontractingTerm><efbc:TermCode>no</efbc:TermCode></efac:SubcontractingTerm>`
under NoticeResult/LotTender. Every minor 1.7-1.15 declares BT-773-Tender as
`SubcontractingTerm[efbc:TermCode/@listName='applicability']/efbc:TermCode`;
the attr-less form exists in no minor. All three samples Bulgarian (BUL/BGR),
two fetches five months apart — one national eSender. The value (`no`) is a
legitimate BT-773 codelist value; only the listName attribute is missing.
Walker: the SDK also has ND-SubcontractedActivity at the predicate-FREE
`efac:SubcontractingTerm` path, so the bare element exact-matches that branch
(relaxed=false), whose children (BT-64/65, FieldsPrivacy) don't include a bare
TermCode → `unclaimed-content`, relaxed fallback suppressed — the issue-142
cause-A mechanism exactly.
Minimization: deleting the SubcontractingTerm block flips 00442074 to PARSED
(25/120).
Fix direction: claim the attr-less `SubcontractingTerm/efbc:TermCode` as
BT-773-Tender (gap-fill leaf on the predicate-free branch, `code`). Like
OPT-060: predicated branches keep sorting first, declared inventories keep
their own.

### E (1/12): bare Lot `ContractExecutionRequirement/cbc:Description` — BT-70 text with its `conditions` code dropped

CER blocks holding ONLY a `cbc:Description` (German notice, genuine
contract-execution terms text), no `cbc:ExecutionRequirementCode` sibling.
BT-70-Lot requires `CER[cbc:ExecutionRequirementCode/@listName='conditions']`
in every minor. Walker: the predicate-free CER branch (home of the
BT-736/743/744/764 code leaves) exact-matches the bare element; it has no
Description leaf → `unclaimed-content`. The existing
`UBL-ContractExecutionDescription` gap-fills are per-listName predicated
branches and deliberately NOT a predicate-free step (the ~1.4k-notice
regression note on `EXTRA`), so they don't cover a code-less CER.
Minimization: deleting the three description-only CER blocks flips 00003343 to
PARSED (48/259).
Fix direction: add a Description LEAF to the already-existing predicate-free
CER branch (`UBL-ContractExecutionDescription`, text). Distinct from the
regression case: that warned against creating a predicate-free CER *branch*
that would swallow whole blocks; the branch already exists (BT-736 et al.),
adding one leaf only claims currently-unclaimed descriptions. Verify against
the dress-rehearsal corpus anyway.

## Sizing (honest: 12 members bound, they do not size)

- **sdk-1.7 (41,027):** 4/5 sampled = A (permission, Spanish), 1/5 = E. Point
  estimate ~80% A ≈ 33K, but a 5-member sample across 2 fetches only supports
  "A is the dominant 1.7 cause, with a nonzero non-A tail". Issue 141's 3/3 (A)
  from fetch 23 raises confidence that A dominates within that fetch; the even
  ~2-3K/fetch spread across 22 fetches fits a steady platform emitter.
- **sdk-1.8 (17,439):** 3/4 = D (Bulgarian eSender, both sampled fetches),
  1/4 = B. D likely the 1.8 majority; ~13K point estimate, wide bounds.
- **sdk-1.9 (1,693):** 3/3 = C, in two countries (FRA, ITA) and two fetches —
  consistent with a construct-level (not publisher-level) cause; plausibly
  most of the 1.9 tail, cannot be sized further from 3 members.
- **Not sampled at all:** sdk-1.6 (1,130), sdk-1.10 (896), sdk-1.11 (573),
  de-1.1/1.2 (241), sdk-1.0/1.5/1.2 (38). Their causes are undetermined; C's
  construct-level nature means it may extend into 1.10/1.11 (both <1.12), but
  that is conjecture, not evidence.
- Corpus-wide cause shares cannot be read from the DB (re-parse failures are
  not recorded; stored `detail` is quarantine-time stale — all 63K say
  "no vendored SDK metadata", describing 2024, not now). Exact sizing needs a
  bulk re-parse over the outstanding members' archive bytes, as in 141/142.

## Cluster observations

- A = Spanish platform (4/4 ESP, fetches 6 months apart) — the issue-141
  publisher, persistent through at least 2025-01.
- D = Bulgarian eSender (3/3 BGR, fetches 5 months apart).
- B = French (1 sample — cluster unconfirmed).
- C = cross-country (FRA+ITA) — construct-level, mirrors how 1.12 itself
  relaxed the predicate because publishers were already emitting the bare form.
- E = German (1 sample — cluster unconfirmed).
- The Estonian eSender of issue 142 does not appear in this sample.

## Fix classes (direction only, none implemented)

| cause | class | fix shape | est. impact |
|---|---|---|---|
| C | later-SDK shape on older minor + fix-scope miss | claim Lot `PJ/Description` as BT-745-Lot (or move the UBL- insert into `EXTRA`) | most of 1.9's 1,693; maybe parts of 1.10/1.11 |
| B | publisher temporal-shape variant | `UBL-TenderValidityDeadline` gap-fill leaf | ≥1 sample; share of 1.8 unknown |
| D | dropped discriminator attr on a real field | claim attr-less `TermCode` as BT-773-Tender | plausibly majority of 1.8's 17,439 |
| E | dropped discriminator code on a real field | Description leaf on the predicate-free CER branch | nonzero share of 1.7 |
| A | invalid codelist-home copy (no SDK ever) | policy decision from issue 141 (claim-and-store or documented skip) | plausibly majority of 1.7's 41,027 |

## Provenance

- Prod reads: bounded `/v1/sql` SELECTs (chunked grouped counts + sample rows +
  v_fetches); two initial un-chunked aggregates hit the 10s cap (408, not
  retried) before switching to id-range chunks. 4 monthly tars opened
  (2024-07, 2024-12, 2025-01, 2025-03), ONE sequential extraction pass per tar
  (inner daily tar.gz then members in a single invocation each);
  /root/diag3 cleaned up. No DB writes, no service changes.
- Harness: /tmp/diag-39c (fresh clone, HEAD c6a7382) +
  crates/ingest/examples/diag39.rs copied unchanged from the issue-141/142
  clones; built and run in the repo dev shell. Sample bytes:
  /tmp/diag-39c-samples/, minimized variants /tmp/diag-39c-samples/min/.
- Quarantine row ids: 3535881 (00440551), 3535512 (00440573), 3535064
  (00442578), 3889728 (00002301), 3890471 (00003343), 3534860 (00441127),
  3536485 (00442074), 3836081 (00749909), 3837575 (00749507), 3535013
  (00441278), 3536608 (00441436), 3985695 (00192492); fetches 23 (ted/monthly
  2024-07), 18 (2024-12), 446 (2025-01), 448 (2025-03).
