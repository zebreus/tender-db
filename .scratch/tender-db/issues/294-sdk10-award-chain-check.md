# 294 — sdk-1.0 award-chain check (the sibling 188 left for its own pass)

Status: DIAGNOSED-HONEST 2026-09-19 — publication reality, the same root as 188's sdk-0.1 verdict in its other shape: an sdk-1.0 (DÖE) award notice carries a freshly minted BT-04 that differs from its own contract notice's and publishes no prior-notice reference (BT-125 / OPP-090 absent, BT-01/BT-02 empty), so nothing in the payload links the pair. Two same-buyer pairs read through the API (see the foot); the panel and the report's section 2 name sdk-1.0 beside sdk-0.1 — gated 128/128 and DEPLOYED 2026-09-19 09:07 UTC at `e25e219` (the report sentence prints on the next data-quality run, Sunday 09-20). No mapping fix is possible without a title-text heuristic, which ADR-0011 refuses. Was: BACKLOG (filed 2026-08-26; the deferral lives in 188's status line)
Kind: diagnosis (eForms sdk-1.0 era)
Relates to: 188 (sdk-0.1 verdicted publication reality), 264 (age-confound method).

Issue 188 verdicted sdk-0.1's 98% unchained awards as publication reality (the
source publishes no folder key) and explicitly "left sdk-1.0 for its own check" —
sdk-1.0 awards measure ~99.9% unchained and nobody has yet determined whether
that is the same publication reality or a mapping gap on our side.

Method exists: 264's within-era sampling (compare linkage against what the raw
XML actually publishes on a bounded sample). Outcome: either a panel explanation
row (like 188's) or a mapping fix + refold.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh /admin/reports/data-quality" | python3 -c 'import sys,json; b=json.load(sys.stdin)["body"]; s=b[b.find("== 2."):b.find("== 3.")]; print([l.strip() for l in s.split("\n") if "sdk-1.0" in l])'

- **done**: the row is EXPLAINED — the verdict at the foot of this record (publication reality, two same-buyer pairs) and the panel/report sentence naming sdk-1.0 beside sdk-0.1 (deployed 2026-09-19 09:07 UTC at `e25e219`); `linked` stays ~0.3 % by construction
- **open**: `['eforms-sdk-1.0  713  711  0.3%']` unexplained (read 2026-09-19: 713 awards, 711 unchained)

## DIAGNOSED 2026-09-19 — two same-buyer pairs, read through the public API, name the mechanism

`eforms-sdk-1.0` is a DÖE profile (the dashboard's coverage rows: 2022–2026, ~3,550 held; 188's
"TED" was wrong on that point). The award-linkage metric counts a Tender whose first version
carries `lot_results` and has no second version. Sampling DÖE windows for award subtypes (29–37)
and checking each first notice's `profile` found four sdk-1.0 awards in 2023-06-01..03, all
single-version; the buyer filter then found their contract notices:

| award (subtype 29) | its BT-04 | contract notice (subtype 16) | its BT-04 | title |
|---|---|---|---|---|
| 856887 (notice 26120349, 2023-06-02) | `bb801623-…` | 514054 (notice 26065018, 2023-03-04) | `7065e9cf-…` | KG 410+KG 470 Sanitäranlagen …, **V0171/2023**, Klinikum Bremen-Mitte |
| 556576 (notice 26120345, 2023-06-02) | `79d073a1-…` | 137693 (notices 26056514/26074395/26058844, 2023-02-19) | `1d35d9c3-…` | KG 420 Heizungs- und Kälteanlagen, **V0135/2023**, Klinikum Bremen-Mitte |

Both sides are `eforms:eforms-sdk-1.0` from the same publisher (Gesundheit Nord gGmbH, buyer org
2699207). The contract notice keeps ONE BT-04 across its three republications (137693 has three
subtype-16 versions grouped on it — the platform can hold a key), yet the award gets a fresh UUID.
The award's parse layer (114 values) has `BT-01-notice` and `BT-02-notice` empty and no `BT-125`,
`OPP-090` or any reference field: there is nothing to map. The only link is the procedure number
in the title text, and ADR-0011 keeps the identity fold on strong references.

So 188's foot — "if the field is there and unextracted it is ours; if absent, honest" — resolves to
a third case: present, extracted, and DIFFERENT. Not ours. The deliverable is the same as 188's:
the panel and section 2 say so for sdk-1.0. Side reads made on the way, none a defect: the two
organisations are provisional with no identifiers because the notice publishes `123 456 789` (a
placeholder the identifier floor refuses) beside a UUID (the GUID floor refuses); the eForms role
vocabulary (`Procedure-Buyer`, `Tenderer`, …) is the served contract for every eForms era.
