# Diagnosis 2: the 1,456-row sdk-1.3 residue after the BT-803 fix

Status: closed-diagnosis (historical record) — wave 2 (OPT-060 pre-1.7, bare ProcessJustification);
acted on, counted in the ledger's "eForms SDK 1.0-1.11" entry

Method: same as issue 141. Six outstanding residue members sampled from prod
quarantine (`reason='unknown-customization'`, `detail LIKE '%eforms-sdk-1.3%'`,
`reprocessed_at IS NULL AND skipped_at IS NULL`) across two fetches (36 =
2023-06, 31 = 2023-11), extracted from the archive tars, and fed through
`ingest::profile::dispatch` → `ingest::process::parse_payload` via
`crates/ingest/examples/diag39.rs` in this clone (/tmp/diag-39b, HEAD =
d7ba5ec, BT-803 fix confirmed as ancestor). All six samples DO carry the
`efbc:TransmissionDate`/`efbc:TransmissionTime` stamp — the BT-803 fix holds;
each fails on a second, different defect.

## Residue distribution (prod, cheap signal)

No fetch/period cluster: the 1,456 rows spread over all nine periods roughly
in proportion to volume — 2023-05:104, 2023-06:209, 2023-07:165, 2023-08:163,
2023-09:173, 2023-10:191, 2023-11:201, 2023-12:156, 2024-01:94 (ted/monthly,
fetches 29-37). The clustering that DOES show is publisher-level: all six
sampled members are Estonian-language notices (`NoticeLanguageCode EST`,
Estonian buyer text) — consistent with one national eSender emitting
sdk-1.3-declared notices long after later SDKs shipped. Corpus-wide share of
EST in the residue: cannot determine without reading member bytes.

## Per-sample verdicts (harness, real member bytes)

| member (fetch, period) | outcome at d7ba5ec | exact error |
|---|---|---|
| 20230602_105/00331135_2023.xml (36, 2023-06, q-id 3107908) | QUARANTINED | `unclaimed-content`: unclaimed element at `cac:TenderingProcess/cac:ProcessJustification/cbc:Description` |
| 20230602_105/00331144_2023.xml (36, q-id 3107912) | QUARANTINED | same (cause A) |
| 20231107_214/00675602_2023.xml (31, 2023-11, q-id 3119032) | QUARANTINED | same (cause A) |
| 20230602_105/00331145_2023.xml (36, q-id 3107910) | QUARANTINED | `ambiguous-field`: `.../cac:ContractExecutionRequirement/cbc:ExecutionRequirementCode` could be any of [BT-736-Lot, BT-743-Lot, BT-744-Lot, BT-764-Lot, BT-801-Lot] |
| 20231107_214/00675861_2023.xml (31, q-id 3118951) | QUARANTINED | same (cause B) |
| 20231107_214/00675914_2023.xml (31, q-id 3119009) | QUARANTINED | same (cause B) |

The two causes do not co-occur in any sample (A-members carry no `conditions`
code; B-members carry no bare ProcessJustification), and both causes appear in
both sampled fetches.

## Cause A (3/6): bare `ProcessJustification/Description` — publisher-invalid in EVERY SDK minor

- Construct: `<cac:ProcessJustification><cbc:Description>UUID</cbc:Description></cac:ProcessJustification>`
  at procedure level (`/*/cac:TenderingProcess/...`), with NO
  `cbc:ProcessReasonCode` sibling anywhere. The UUID is the notice's own
  `cbc:ContractFolderID` copied verbatim — publisher junk (samples:
  restricted-procedure CANs, so a direct-award justification would not even
  apply).
- No SDK minor 1.0-1.15 declares a predicate-less procedure-level
  `ProcessJustification/cbc:Description`: BT-1252-Procedure requires
  `[cbc:ProcessReasonCode/@listName='direct-award-justification']` in every
  minor; the 1.12+ predicate-free variant (BT-745-Lot) is Lot-scoped only. So
  unlike BT-803 this is NOT claimable under any later inventory — same class
  as the 1.7 `permission` quirk.
- Walker mechanism: the privacy graft in `crates/ingest/src/eforms/index.rs`
  (the `ProcessJustification/ext:UBLExtensions` GRAFT targets) creates a
  predicate-free `cac:TenderingProcess/cac:ProcessJustification` branch. The
  bare PJ element exact-matches that branch (`relaxed=false`), whose children
  are only the grafted `ext:UBLExtensions` subtrees — so `cbc:Description`
  finds no branch even by name → `unclaimed-content`. (Without that graft
  branch the walker's relaxed fallback would have claimed it as BT-1252.)
- Minimization: deleting only the PJ block flips all three samples to PARSED
  (25/115, 30/118, 89/329 sections/values). Confirmed single blocking
  construct per member.

**Fix needed (not implemented):** decide policy, then either (i) a gap-fill
claim for the bare procedure-level `ProcessJustification/cbc:Description`
(GAP_FILL entry in index.rs, e.g. `UBL-ProcessJustificationDescription`,
mirroring the existing `UBL-ContractExecutionDescription` pattern for
unlisted-listName CER descriptions), or (ii) graft the `PJ[direct-award]`
Description leaf onto the predicate-free PJ branch and store it as BT-1252
under a relaxed claim. (i) is honest (the content is a duplicated folder id,
not a real justification) and matches the established precedent.

## Cause B (3/6): `ExecutionRequirementCode[@listName='conditions']` = OPT-060, an SDK-1.7+ field on 1.3 notices

- Construct: a legitimate BT-70 block —
  `<cac:ContractExecutionRequirement><cbc:ExecutionRequirementCode listName="conditions">performance</cbc:ExecutionRequirementCode><cbc:Description>…</cbc:Description></cac:ContractExecutionRequirement>` —
  plus three other CER code elements (reserved-execution / ecatalog-submission
  / einvoicing) that 1.3 claims fine.
- The `conditions` code element itself is **OPT-060-Lot**, which enters the
  vendored line at **1.7.0** (leaf-predicate shape in 1.7/1.8, parent-predicate
  shape + `-List` attr field in 1.9-1.15) and exists in NO minor ≤1.6. The
  member's BT-70 `cbc:Description` IS declared in 1.3 — only the code element
  isn't. Same "construct from a later SDK on an older-minor notice" class as
  BT-803, except emitted by the buyer/eSender side rather than the TED
  publication pipeline.
- Walker mechanism: under 1.3 the CER element matches both the predicate-free
  CER branch (BT-736/743/744/764/801 code leaves) and `CER[conditions]`
  (BT-70 Description leaf); the `conditions` code matches no exact leaf
  predicate → relaxed by name → five differing field candidates →
  `ambiguous-field` (parse.rs relaxed-ambiguity guard).
- Minimization: deleting the whole CER[conditions] block flips 00331145 to
  PARSED (23/124). Deleting only the code element does NOT parse — the
  Description then loses its discriminator and dies `unclaimed-content` —
  proving the code element is the exact blocker and that a fix must claim it,
  not strip it.

**Fix needed (not implemented):** make OPT-060 claimable on pre-1.7 minors —
same shape as the BT-803 fix: either claim
`ExecutionRequirementCode[@listName='conditions']` as a known later-SDK field
in the parser/gap-fill layer, or a per-minor inventory supplement (ADR-0002:
supplement, not an edit of the vendored files). Expected to also require
nothing for the Description half (already claimed by 1.3's BT-70).

## Sizing

- Sample split: 3 A / 3 B out of 6, both causes in both sampled fetches, no
  co-occurrence. The residue is at least two-cause; a ~50/50 split is the
  point estimate but rests on 6 members.
- The corpus-wide split cannot be read from the DB (re-parse failures are not
  recorded; stored `detail` is quarantine-time stale). Exact sizing needs a
  bulk re-parse over the outstanding members' archive bytes (9 monthly tars) —
  same caveat as issue 141. Other, un-sampled causes in the 1,456 cannot be
  ruled out.
- Fixing BOTH causes is expected to reclaim the sampled members fully; whether
  that closes all 1,456 depends on the unruled-out tail.

## Provenance

- Prod reads: bounded SELECTs via /v1/sql (grouped counts + 6 sample rows);
  6 tar member extractions from /data/archive/ted/monthly/{2023-06,2023-11}.tar;
  /root/diag-members2 cleaned up. No DB writes, no service changes.
- Note: extraction ran one `tar -xOf` per member (3 passes per tar), not the
  intended single pass per tar — read-only but more IO than planned.
- Harness: /tmp/diag-39b (fresh clone, HEAD d7ba5ec) +
  crates/ingest/examples/diag39.rs copied unchanged from the issue-141 clone;
  built and run in the repo dev shell. Sample bytes: /tmp/diag-39b-samples/,
  minimized variants under /tmp/diag-39b-samples/min/.
