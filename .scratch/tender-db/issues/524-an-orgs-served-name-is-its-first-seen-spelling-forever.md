# 524 — an org's served name is its first-seen spelling forever, so merges and drops leave a wrong or junk head

Status: needs-triage — filed 2026-10-12 from the hourly audit after 514's rollout and 511's drain.
Kind: data quality (served organization names)
Relates to:
- 514 (A6);
- 511 (unit 6 item 7);
- 522 (the s.94 PPON org);
- 513 (nhs-body survivors keep their head name);
- 321 (rehoming leaves the name satellite behind);
- 234 (`name_norm` is the resolver's key).

## Verify

    ssh root@zebreus.click "curl -s http://127.0.0.1:8080/v1/organizations/31558903" | head -c 200

- **done**: the name reads 'The University of Sheffield' (or its register name);
- **open**: 'Sheffield Hallam University', as served on 2026-10-12.

## What was found

`organizations.name` is written at mint and never again. No `UPDATE organizations SET name` exists; only `name_norm`
is ever rewritten (canonical.rs ~32951, ~33481, lib.rs ~4063). A merge keeps the survivor's own head, and a dropped
mention leaves the head it named. Served examples on 2026-10-12:
- **31558903**, UKPRN 10007157, which is the University of Sheffield. It is served as **'Sheffield Hallam
  University'**, from one stray mention. 514's review measured 15 of its 16 mentions as Sheffield (A6; the survivor
  of G24 by lower id).
- **22713056** is served as **'s.o.'** ('siehe oben', a placeholder). 511's drain retired the placeholder mentions,
  and its 28 remaining ones read 'Vergabekammer bei der Bezirksregierung'. They bind by the chamber's phone number
  0251411691.
- **30914328** is served as 'Withheld Section94 Supplier' while holding HISCOX and DOC-Labor mentions (522).

## Why it matters

The API, search and every statistic show the head name. Merges (R2/E0/altid/nhs-body/514) and drops (510/511) are
now routine, so the head drifts away from what the org's mentions say.

## Proposed units

1. **Measure.** Orgs whose head name no longer appears among their mention names, or is a minority of them weighted
   by count, or is a placeholder under `partyname::not_a_name`. Run it as an in-app dry census, never a corpus scan
   from the agent.
2. **Decide.** The display name rule:
   - the weighted-majority mention name;
   - the register name for a register identity (UKRLP/CC/FCA/ODS), when one is cached;
   - otherwise keep, with stickiness so a single new mention cannot flip it.
   Decide also whether `name_norm` (the 234 key) follows. Probably not, so binding keys are left alone and only the
   display name is refreshed. ADR needed.
3. **Build** a dry-first `refresh-org-names` job with a ledger (old/new), then the D5 correction rows.
