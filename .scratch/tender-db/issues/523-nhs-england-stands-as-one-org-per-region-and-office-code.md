# 523 — NHS England stands as one org per region and office code

Status: needs-triage — filed 2026-10-12 from issue 519's unit-1 class C and unit-2 ruling 11.
Kind: data quality (organization identity)
Relates to: 519 (class C), 513 (nhs-body verdicts), 300 (identity ladder)

## Verify

    ssh root@zebreus.click "curl -s 'http://127.0.0.1:8080/v1/organizations?identifier=GBNHSX24'" | head -c 300

- **done**: one NHS England body, with the region and office codes as its merged identifiers (or a recorded decision to
  keep them apart);
- **open**: X24 answers its own org, and the region and office codes answer others.

## What was found

519 unit 1 (`../519-nhs-digit-codes/unit1-results.md`, class C: 24 orgs / 383 mentions) found NHS England as many
canonical orgs, each minted by a digit GB-NHS code:
- X24 "NHS England" (95 mentions), ODS RO162 (an other statutory authority, the legal body);
- Y56 (86), ODS "London Commissioning Region", RO209;
- the Y5x/Y6x commissioning regions;
- the Q4x/Q7x local offices (RO210, e.g. Q48 "Merseyside Area Team", Q71 "NHS England London");
- X26 (RO116), X2407 and XDH04 (sites);
- the typed names `NHSENGLANDNORTHEASTANDYORKSHIRE…Q72/Q74/Y63`.

Some of these codes are mis-coded on trusts and surgeries: Y54 is on York & Scarborough, Y61 on Swan Surgery, Y62 on
East Lancashire Hospitals.

## Decide

- (a) One body: fold the region and office codes into X24's org by reviewed verdict, 513/519-style. The regions are
  offices of one legal body, NHS England, which itself merges into DHSC from 2026–27.
- (b) Keep the regions apart as buying points, but fold the typed-name and duplicate orgs into their region.
- In either case, the mis-coded Y5x/Y6x on trusts go to 452 `wrong` (519 ruling 8).

Measure first:
- the mention names per org (bounded reads);
- ODS Rels (region → X24?);
- how publishers name themselves ("NHS England – North West" vs "NHS England").
