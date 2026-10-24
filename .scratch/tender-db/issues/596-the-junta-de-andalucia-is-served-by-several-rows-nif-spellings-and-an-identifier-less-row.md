# 596 — the Junta de Andalucía is served by several rows: four sibling NIF spellings beside 2660 and the identifier-less 3231

Status: needs-triage — filed 2026-10-24 from 594 unit 2's decision (F2). Next: unit 1, decide (a) a reviewed merge of the 4
sibling NIF rows (22341455 `ESS4111001F`, 22419088 `S4111001`, 22525033 `ES4111001F`, 11044446 `4111001F`) into 2660 — the weak E2
VAT key never canon-binds (`crosswalk.rs:224-228`) — and (b) whether the identifier-less 3231 "JUNTA DE ANDALUCIA" (2,807 mentions,
buyer on 2,644 Tenders) should be captured by the NIF row (an E3 capture question). N today (a split of the right body); after
594's pin the NIF rows all serve "Junta de Andalucía", so this is cosmetic-to-moderate.
Kind: data quality / organizations (N)
Relates to: 594 (pin), 595 (TARC split), 469 / 514 (identifier-keyed merges), 351 (name-keyed reuse).

## Verify

    for o in 2660 3231 11044446; do curl -s https://tenders.zebreus.click/v1/organizations/$o | jq -c '[.id,.name,.mentions]'; sleep 2; done

- **open (2026-10-24)**: three rows (2660 serving TARC's name until 594 ships).
- **done**: one Junta row (the others 308 to it), or a recorded reason to keep 3231 apart.
