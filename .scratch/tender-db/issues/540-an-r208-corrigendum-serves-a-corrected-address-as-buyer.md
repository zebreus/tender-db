# 540 — an R2.0.8 corrigendum (F14) serves a corrected address as `buyer`, whichever address it corrects

Status: needs-triage — filed 2026-10-14 from issue 534's review round (a schema read, unmeasured). Next: read 3–5 F14
payloads that correct an address and see which role their new block is served under.
Kind: data quality (party roles; buyer grouping)
Relates to:
- 534 (legacy role folds), where this was read;
- 393 (the legacy role fold that maps `CA_CE_CONCESSIONAIRE_PROFILE` → `buyer`);
- 483 (buyer-role census).

## Verify

    curl -s "https://tenders.zebreus.click/v1/notices?publication_id=<an F14 correcting an address>" | jq -c '[.items[].id]'

- **done**: the corrected block of a non-buyer address (a review body's, say) is not served as a `buyer` party.
- **open (2026-10-14)**: unmeasured. Pick the exhibit in unit 1 and replace the placeholder above with its number.

## What was found

This comes from the R2.0.8.S02 eSender schema; no payload has been read yet.
- **Where the new block sits.** A corrigendum (F14) that corrects an address puts it under
  `ADDR_CORR` → `MODIFIED_ADDRESS` → `CA_CE_CONCESSIONAIRE_PROFILE`, next to a `WHERE` saying which address it corrects.
- **How it folds.** `legacy_role` folds `CA_CE_CONCESSIONAIRE_PROFILE` to `buyer`, so the corrected block is served as
  a buyer whichever address `WHERE` names.
- **Why it matters.** Buyers drive how notices group into tenders (481, 483) and the `?buyer=` filter. A corrected
  review-body or contact address would then add a second "buyer" to the tender.

## Proposed units

1. **Measure (S).** Find F14 notices with an `ADDR_CORR` block, by public GETs of `/v1/notices/{id}/content` on a
   bounded sample of R2.0.8 corrigenda. Read what `WHERE` names and which role the block is served under. If every
   `WHERE` names the buyer's own address, close this as harmless.
2. **Decide (S).** Fold by `WHERE`, or drop a non-buyer corrected block, or keep it with a distinct role. Then refold
   the cohort through `refold-fields` (size first).
