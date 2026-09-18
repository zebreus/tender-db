# 413 — two sdk-1.13 notices hold as `unclaimed-content` on a `StrategicProcurement` block the parser already knows, and the quarantine terminal ledger has been lit by them since the June-2025 backfill

Status: ready-for-agent — found 2026-09-18 02:5xZ by the hourly audit (step 3): `/metrics` reads `tender_db_quarantine_terminal_exceeded 1`, and the one reason over its policy is `unclaimed-content` at 2 rows against `Fixed(0)`.
Kind: defect (ingest — the eForms element mounts in `crates/ingest/src/eforms/index.rs`; plus two held rows to reprocess) — and an operational one: the terminal ledger's alarm is now permanently on for two rows, which is how an alarm stops meaning anything.
Relates to: 195 (RESOLVED 2026-08-14 — drained `unclaimed-content` to ZERO and installed the mount for exactly this block under a `Lot`-scheme lot; these two are the first rows of the reason since), 303 (the terminal ledger: `quarantine_terminal_policy` defaults an unnamed reason to `Fixed(0)` DELIBERATELY, so a new hold of a drained class is an alarm, not noise), 402 (the OJ S daily backfill of 2025-06 that brought both members in on 2026-09-15 16:54Z), 268 (`unrepresentable-value` is `AcceptedInflow` — the 326 held under it today are the accepted class and are NOT this issue)
Blocked by: nothing

## Observed (2026-09-18, live)

`GET /metrics`:

    tender_db_quarantine_reason_members{reason="unclaimed-content"} 2
    tender_db_quarantine_terminal_exceeded 1

`GET /api/dashboard` `quarantine.recent`, the two rows, both `eforms:eforms-sdk-1.13`, both
`first_seen` 2026-09-15 16:54Z (issue 402's backfill of OJ S 2025-06):

| member | detail (namespaces abbreviated) |
| --- | --- |
| `06/20250613_2025112.tar.gz/20250613_112/00381774_2025.xml` | `unclaimed element at /cac:ProcurementProjectLot/cac:TenderingTerms/ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent/efext:EformsExtension/efac:StrategicProcurement/efbc:ApplicableLegalBasis` |
| `06/20250626_2025120.tar.gz/20250626_120/00412845_2025.xml` | the same path |

The dashboard's quarantine panel shows the reason with the ledger's "exceeded" verdict beside it
(`ui.rs` `quarantine_terminal_exceeded`, computed from the same counts as `/metrics`, so the two
agree by construction — issue 303).

## Mechanism, from the source

`efbc:ApplicableLegalBasis` (BT-717) inside `efac:StrategicProcurement` is a block the eForms
walker knows: `crates/ingest/src/eforms/index.rs` mounts it in three shapes (the canonical lot
`TenderingTerms` extension, the `LotResult` mirror, and issue 195's AwardingCriterion variant).
Every one of those mounts is anchored at

    /*/cac:ProcurementProjectLot[cbc:ID/@schemeName='Lot']/cac:TenderingTerms/…/efac:StrategicProcurement

and the held path is that mount **minus its predicate**. The quarantine detail does not print
predicates, so the most likely reading — a hypothesis, not a finding, until the member bytes are
read — is that these two notices carry the block under a `ProcurementProjectLot` whose
`cbc:ID/@schemeName` is `Part`, not `Lot`: the SDK gives a Part no StrategicProcurement block, an
eSender published one anyway, and the walker's exhaustive-consumption rule held the whole notice
(ADR-0004), as designed.

## Why it matters

Two notices unserved is small. **The lit alarm is not**: `Fixed(0)` is the ledger's whole
mechanism for noticing that a drained class has come back, and `quarantine_terminal_exceeded` has
read `1` since 2026-09-15 with nothing on the board naming it. An alarm that stays on for a known
two rows is one nobody will read the day it means something else — the same shape issue 303
closed for the corrupt-zip reason with `Fixed(8)`, and 412's "a status nobody re-reads is a
photograph", one level down.

## Repro

    curl -s https://tenders.zebreus.click/metrics | grep -E 'unclaimed-content|terminal_exceeded'
    curl -s https://tenders.zebreus.click/api/dashboard | python3 -c "import sys,json; print([r['member_path'] for r in json.load(sys.stdin)['quarantine']['recent'] if r['reason']=='unclaimed-content'])"

## Verify

    curl -s https://tenders.zebreus.click/metrics | grep -E '^tender_db_quarantine_(reason_members\{reason="unclaimed-content"\}|terminal_exceeded) '

- **done**: only `tender_db_quarantine_terminal_exceeded 0` — the reason line is gone (0 rows are not listed) and no reason is over its policy
- **open**: `tender_db_quarantine_reason_members{reason="unclaimed-content"} 2` then `tender_db_quarantine_terminal_exceeded 1` (read 2026-09-18)

## Done when

- The shape is read off one of the two members (an archive read of one member is bounded) and
  recorded here: `Part`-scheme lot, or whatever it actually is.
- Either the mount admits the shape (a `[cbc:ID/@schemeName='Part']` twin of the canonical
  anchor, with a fixture cut from the member and a test that the block is claimed), or the
  element is entered in issue 195's ledger as an explicit ignore with its reason — never a silent
  widening of the walker.
- The two rows are reprocessed (`reprocess`, reason `unclaimed-content`, profile
  `eforms:eforms-sdk-1.13` — a production write, so it waits with the other gated jobs if the
  classifier refuses it) and serve.
- `tender_db_quarantine_terminal_exceeded` reads `0`, and stays a signal.
