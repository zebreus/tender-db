# 576 — a party role whose eForms reference names a touchpoint (TPO-…) is dropped from the tender's parties

Status: needs-triage — filed 2026-10-19 from the hourly audit (workflow wf_90e506fc-209, auditor + skeptic, confirmed;
`.scratch/tender-db/audits/2026-10-19-recent-tenders/verify_award.md`). Next: unit 1, size how many role references target a
touchpoint (by role and platform) and decide what a touchpoint-targeted role should serve.
Kind: data quality (parties: omission; no wrong organization is served)
Relates to:
- 530 (A7 / Build 1b): its role guard reads OPT-300/OPT-301 references; a touchpoint target is never an ORG section there either.
- 368 (unmapped source vocabulary dropped silently), 456 (each party serves its own published name).

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8851944 | jq -c '[.parties[] | [.role, .organization_id]]'

- **done**: unit 1's decision is applied: either line 1 serves a review-body party for LOT-0000 (as decided), or the docs say
  plainly that a role whose reference names a touchpoint is not served, and why.
- **open (2026-10-19)**: `[["Procedure-Buyer",6614],["Tenderer",12032625]]` — no Lot-ReviewOrg.

## What was found

- Tender 8851944 (TED 00722535-2026, notice 47411306, eForms subtype 29, ES): LOT-0000 carries `OPT-301-Lot-ReviewOrg = TPO-0001`
  (an id-ref, scheme `touchpoint`). TPO-0001 is a Touchpoint section under ORG-0001 with BT-500 "Tribunal Administrativo Central
  de Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ).", 28000 Madrid, ES300. The served parties hold only the
  buyer and the winner; the review body is missing. Re-read byte-identical (version 1, one notice).
- Systemic: notice 47413157 / tender 8851996 (ES) loses Lot-ReviewOrg and Lot-AddInfo on 4 lots the same way; every
  ORG-targeted role on the audited tenders (8851923, 8852005, 8852009) is served.
- Cause: `crates/ingest/src/project.rs` (bind_organizations, ~6111): "A reference to something that is not an Organization
  section (a touchpoint, a lot, a result) is notice-layer detail, not a party" — the role is skipped when its target is not a
  minted ORG section. No public doc says so (/docs and OpenAPI list Lot-ReviewOrg with no touchpoint caveat).
- Design question for unit 1: a touchpoint is a contact point OF its parent organization, but here the publisher used the
  buyer's touchpoint to print a different body (the review tribunal). Binding the role to the parent ORG would serve the buyer
  as its own review body (530's AOK shape); minting a party from the touchpoint's own name would serve the tribunal.

## Proposed units

1. **Size and decide (S).** A bounded census of `notice_ids` role references whose target section kind is a touchpoint (by role,
   source, platform/country), plus a read of 20 salted cases: is the touchpoint's printed name the parent org (same body) or a
   different body? Decide: (a) bind to the parent ORG when the names agree, (b) mint a party from the touchpoint's own name and
   address, (c) leave out and document. Record the decision.
2. **Build + drain (M)** per the decision (a refold of the affected notices), or the docs sentence.
