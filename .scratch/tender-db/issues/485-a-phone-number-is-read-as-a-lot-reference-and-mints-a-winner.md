# 485 — a phone number is read as a lot reference and mints a junk winner

Status: done — 2026-10-05: landed with 484 unit 2 (`064870d`, deployed `14df8b4`); text era re-parsed and re-projected; 2808875 serves 19 winners, 0 `Fax …` (Verify). Was: ready-for-agent — NEXT: ships with 484 unit 2 (landed, not deployed: the fix is in
`crates/ingest/src/text/parse.rs`, same change). After the deploy it rides 484's text-era re-parse and the one
`project` after it; then run the Verify below. No separate re-parse.
Kind: data correctness (awards, organizations)
Relates to: 484 (found as its by-catch; same function, same re-parse), 234 (identity-less provisional orgs), 244
(the text-era winner derivation)

## What is wrong

The text-era winner derivation (`awarded_names`) mints organizations named after the tail of a contact block and
serves them as winners:

| notice | served winner(s) besides the real one |
| --- | --- |
| 3009398 (Autoridad Portuaria de Málaga) | `URL: www.puertomalaga.com. Fax 952 12 50 02` |
| 2808875 (Cardiff, 19 providers) | 19 × `Fax 0044 2920 …` (`Fax 0044 2920 644615`, `Fax 0044 1792 457041`, …), one per V.3 block |
| 3016761, 3205106 | same shape (484's sample read) |

Read 2026-10-04 from `GET https://tenders.zebreus.click/v1/notices/{id}/content` (the stored parse: `ORG-n`
`TED-OFFICIALNAME`).

## Cause

`winner_segments` splits a value at a `.` that a lot reference follows. `lot_prefix_len` accepts any head made of
digit groups, separators and single letters before a `:` or `.` — and the separators include a bare space. So
in

    AWARDED: AWETU, Att: Suzanne Smith. 41a, Lower Cathedral Road, UK-Cardiff CF11 6LW.
    Tel. 0044 2920 394141. Fax 0044 2920 644615.  V.4) …

`0044 2920 394141.` passes as a lot reference, the `.` after `Tel` splits the value, the lot "prefix" is stripped
from the next segment, and what is left — `Fax 0044 2920 644615`, no comma, bounded by `V.4)` — is a name.

## Fix (landed with 484 unit 2, not deployed)

Both in `crates/ingest/src/text/parse.rs`:
1. **`lot_prefix_len` refuses two digit groups separated only by whitespace** (`digit_groups_space_separated`).
   That is phone notation; every measured lot list separates references with `,` `/` `-` or `and`
   (`1, 2, 3 and 4:`, `1/2:`, `1:`), and those still pass.
2. **`plausible_name` refuses a contact line** (`opens_with_contact_line`): `Fax`/`Tel`/`Telefax` followed (past
   `.`, `:`, spaces) by a digit, `+` or `(`; or `URL:` / `E-mail:` / `Email:`. Narrow on purpose:
   `Tel Aviv Holdings` and `Faxon Ltd` are still names.

Test: `a_phone_number_is_not_a_lot_reference` — the stored V.3 values of 3009398 (→ `["Autoridad Portuaria de
Málaga"]`, was + the URL/Fax string) and two of 2808875's (→ `["AWETU", "BAWSO"]`, was + two `Fax …`).

Effect on the real winners: none in these exhibits — the segment before the bad split already ended at its first
comma, so `AWETU` etc. were read correctly; only the phantom second segment goes. 3009398's remaining winner is the
authority itself, which is what V.3 publishes (484: source-says-so, unit 3's `is_buyer` flag).

Re-parse expectation (484 review): the same text-era re-parse also carries 484's wider numbered bound, which ADDS
winners era-wide (1993 daily: 117 → 150) and drops price-list junk (1200611). So a falling count of `Fax …`/`URL:
…` winners will sit beside a RISING total of text-era winners and organizations — read 484's step 5 per direction.

## Verify

    curl -s https://tenders.zebreus.click/v1/notices/2808875/content | jq '[.sections[].values[] | select(.field_id=="TED-OFFICIALNAME") | .value | select(startswith("Fax"))] | length'

- **open** (2026-10-04): `19`
- **done:** `0` (after 484's text-era re-parse; the served parties follow after the `project`).
