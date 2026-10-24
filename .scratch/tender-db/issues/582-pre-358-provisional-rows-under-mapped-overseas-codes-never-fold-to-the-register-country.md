# 582 — 5,076 pre-358 provisional rows under mapped overseas codes never fold to the register country

Status: ready-for-agent — filed 2026-10-24 as issue 578's U4 (578 "Decision D578-1" and "Units"; the evidence is 578's R3, k1
and k2). Next: unit 1, one bounded read of those rows' mentions.
Kind: data quality (organization identity: frozen splits under a raw overseas country code)
Relates to:
- 358 (register country: RE/GP/MQ/GF/YT/PM/BL/MF/WF/AX/SJ/GL fold to FR/FI/NO/DK at mint);
- 434 (`publishes`: a recorded mention never moves);
- 473 (dissolves through `repair-placeholder-orgs`; this lands first);
- 578 (Legacy440 → resolver parity; 30920766 is this issue's first member).

## Verify

    python3 -c "import re;s=open('.scratch/tender-db/issues/582-pre-358-provisional-rows-under-mapped-overseas-codes-never-fold-to-the-register-country.md').read();print(len(re.findall(r'^## Unit 2 — (?:applied|done)', s, re.M)))"

The Verify reads this file. The rows' state needs a /v1/sql read (unit 1), which is not a free check.
- **done**: `1`. A `## Unit 2 — applied` section records the rehomes and country moves, and R3's row half re-taken reads
  0 identifier-NULL rows under the 12 mapped codes, apart from rows whose mentions themselves publish no register-country
  key.
- **open (2026-10-24)**: `0`.

## What was found (578 triage, 2026-10-21)

- **Rows.** R3, `probe4.sql` r_combined, `truncated:false`: AX 37, GF 348, GL 14, GP 941, MF 3, MQ 1,108, PM 6, RE 2,433,
  SJ 1, WF 10, YT 176, BL 0, which is 5,077 identifier-NULL provisional rows under the 12 codes 358 maps to a register
  country.
  - All but one were created by the 2026-08-15 rebuild (created_at 1786785861), before 358 landed.
  - The one exception is 30920766 (MQ, `rectorat de martinique`, 4 mentions), minted 2026-09-09 by 365's Legacy440 dissolve
    835. That shape is fixed by 578 U2.
- **358's premise does not hold.** 358:120-122 expected these rows to fold "naturally as mentions re-project". They do not:
  - `resolve_one_mention` probes only the register key (canonical.rs ~23647-23695), so it never revisits a mapped-code
    row;
  - 434's `publishes` (~15644) compares name, country, raw identifier and scheme, so a recorded mention keeps its row.
- **Served effect.** One body can be served as two organizations: one under FR/FI/NO/DK, which new mentions bind, and one
  under the raw overseas code, which holds the pre-358 mentions. `?country=` and per-organization lists split between them.

## Proposed units

1. **Read (S).** One bounded read of these rows' mentions: a per-code count through `organization_mentions_org`,
   plan-probed. For each row, also whether a register-key row stands on (name_norm, folded country), as in 578's k2
   (2660729 for `rectorat de martinique`).
2. **Apply (M).** For each row:
   - where a register-key row stands, rehome its mentions onto it and sweep the empty row (578 U3's shape: 30920766 →
     2660729; NOT onto an identifier row such as the SIRET 14624875);
   - otherwise, a country move to the register country (`apply-country-verdicts`).

   The bar is 0 rows under mapped codes created after 2026-09-07 (R3's row half), and the re-take shows the 5,076 gone.
   Org merges feed path W (R1/R6), so under issue 551 this is an in-place W-input writer: it runs after 11-04 under C9',
   or as a reviewed segment with its pre-step.
3. **Verify** the re-take and a salted read of 30 rehomed mentions.
