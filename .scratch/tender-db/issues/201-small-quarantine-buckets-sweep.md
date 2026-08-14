# 201 — small outstanding quarantine buckets: name and decide the tail

Status: needs-triage
Kind: quarantine composition sweep (the issue-30 discipline applied to the residue)

## What (outstanding-by-reason readout, 2026-08-14, post-200)

- translation-structure-mismatch 94
- duplicate-section-id 23
- ambiguous-field 14
- unknown-field-code 9 (the OC/ON ledger key shows what drained; these 9 are the residue)
- "unreadable zip bundle: … Could not find EOCD" 8 (corrupt-at-source candidates — the
  1996 truncated-bundle family? verify against issue 30's corrupt class)
- unknown-customization 3 (notices declaring an unvendored minor — name WHICH; if a real
  new minor is publishing, it needs vendoring per the ACCEPTED gate)
- (unrepresentable-value 5,169 and unclaimed-content 51 are owned: sub-cent kept class,
  issue 199 + the parked r208 pair)

Each bucket: one grouping query, member extraction where needed, then fix / documented
keep / ledger naming per the 143/144 pattern. The unknown-customization 3 first — that
gate should be empty.

**2026-08-14 ~10:3x CEST box time (orchestrator) — unknown-customization bucket: SDK 1.2
vendored.** The 3 rows are the only eforms-sdk-1.2 notices TED ever carried (one eSender,
the 2022-11-11 daily) — which is exactly why 1.2 was never vendored: it looked unpublished.
Vendored verbatim from the OP-TED 1.2.0 tag (730 fields; upstream ships the tag with a
stale 1.1.0 sdkVersion stamp, normalized with a $comment), completeness harnesses green
over the new inventory, negative test moved to the genuinely-unpublished 1.4. Corrects my
answer to Lennart's coverage question ("1.1/1.2/1.4 never published" — 1.2 was, thrice).
Deploy + 3-row reclaim next firing (daily occupying the queue). Remaining buckets:
translation-structure-mismatch 94, duplicate-section-id 23, ambiguous-field 14,
unknown-field-code 9, unreadable-zip 8.

**2026-08-14 ~11:0x CEST box time (orchestrator) — translation-structure-mismatch read
(code side).** The r209 parser overlays translation copies onto the original-language
structure; a translation OPENING a section the original never had (r209/parse.rs
open_section, `translating` guard) rejects the member whole. The 94 rows are ~15 shapes:
F14 CHG-N blocks (translations enumerating changes differently, ~38), extra review-body/
authority ORG-N sections (~26), LOT-2 on F03, r208 twins. Next: extract 2-3 members (after
the fold frees the disk) and decide per family — tolerate (open the translation-only
section; it is published content) vs documented keep. Deploy stack (ledger entry, sibling
guard, SDK 1.2) + the 3-row 1.2 reclaim also pending queue-idle.
