# 566 — a superseded version's served `value` is picked by the HEAD's EUR value, not that version's own elected figure

Status: blocked — filed 2026-10-17 as unit 7 of issue 561's decision (workflow wf_3176baf8-d14). Blocked by: 561 unit 4
(the stored per-version elected value `tender_versions.value_eur_cents`). Next, once that lands: pick a non-head
version's served value by `v.value_eur_cents`, and decide whether 551's derivation marker also goes per version.
Kind: correctness (served `value` of non-head versions: SSE catch-up payloads with `include_data`, version reads)
Relates to:
- 561 (stores the elected value per version; its decision names this follow-up);
- 551 (derived lot-sum heads carry their money and marker on `tenders` only);
- 547 (`Scope::At` serves non-head versions).

## Verify

    grep -c 's.eur_cents = t.current_value_eur_cents' crates/store/src/read.rs

- **done**: a non-head version's served `value` is that version's own elected figure — an api test of a two-version
  chain whose v1 elects a different figure than v2 serves v1's figure at v1 — and a derived superseded version either
  serves its lot sum with a per-version marker or is documented as serving `value: null`.
- **open (2026-10-17)**: read.rs ~2584–2592 picks the amount row whose `eur_cents` equals the HEAD's value
  (`s.eur_cents = t.current_value_eur_cents`); at a superseded version that elected something else the pick is null or
  another figure.

## What was found

A superseded version's served `value` (for example in an SSE catch-up payload with `include_data`) is chosen as the
amount row of that version whose `eur_cents` equals the Tender's CURRENT head value. When the version elected a
different figure, the pick finds nothing (null) or a row that merely shares the head's cents. After 561 unit 4, the pick
can match `v.value_eur_cents` instead.

The 561 decision's adversarial check adds: a derived (551) superseded version has no amount row carrying its sum S, so
matching `v.value_eur_cents` alone still serves null with no derivation, while the head serves
`current_value_cents`/`current_value_currency`. Either add the 551 marker as quiet per-version columns in 561's epoch
bump (value_derivation, value_cents, value_currency; refreshed by the same quiet rule, excluded from equality), or state
that a derived superseded version serves `value: null`.
