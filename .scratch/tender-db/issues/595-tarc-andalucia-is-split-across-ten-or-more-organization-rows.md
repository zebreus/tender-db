# 595 — TARC Andalucía (the Junta's contracts review tribunal) is split across ten or more organization rows

Status: needs-triage — filed 2026-10-24 from 594 unit 2's decision (F1, `594-u1/decision.md` "Follow-ups"). Next: unit 1,
decide a reviewed 513/519-style consolidation onto 22235590 (keyed by TARC's own DIR3 code A01034437), scoped to the rows keyed
by TARC's own codes and its identifier-less rows — never the NIF-keyed mentions on 2660 (594 keeps those on the Junta row).
Kind: data quality / organizations (N: a split of one body)
Relates to: 594 (the Junta NIF row 2660 and its pin), 513 / 519 (reviewed consolidation), 586 (touchpoint aliasing of review bodies).

## What was found (594 unit 2 reader)

Rows: 3601 (17,461 mentions, identifier-less), 3947 (4,669), 2720543 (1,076), 8738172 (630), 22235590 (300, DIR3 A01034437),
14748 `TARCJA001` (208), 22635797 (122), 656 `S411101F` (125, a typo of the Junta NIF), typo rows, 22525033 `ES4111001F` (4,
left unpinned by 594's A3).

## Verify

    for o in 3601 3947 22235590; do curl -s https://tenders.zebreus.click/v1/organizations/$o | jq -c '[.id,.name,.mentions]'; sleep 2; done

- **open (2026-10-24)**: three separate rows serve TARC.
- **done**: TARC's own rows answer one organization (the others 308 to it), NIF-keyed mentions untouched.
