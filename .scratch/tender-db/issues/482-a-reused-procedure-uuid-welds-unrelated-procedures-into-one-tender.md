# 482 — a reused procedure UUID (BT-04) welds unrelated procedures into one Tender, and nothing checks the buyers

Status: ready-for-agent — filed 2026-10-02 from 481's calibration (the skeptic lens, false-merge shape 14). The first unit
is a census: count UUID-keyed Tenders whose versions name disjoint buyer sets, and split them into legitimate (joint
procurement, a central purchasing body acting for others) and collisions.
Kind: data correctness (a false merge under the declared rule)
Relates to: 369 (procedure-key placeholder gate: `key_shaped=1` keys with ≥ 3 distinct buyer sets are refused; UUID keys
are never checked), 481 (cross-source dedup; its weld guards), ADR-0003 (a declared link merges), 12/34 (the BT-04 key)

## What is wrong

The fold groups notices by procedure key. For a UUID key, nothing else is checked: issue 369's gate applies only to
shaped (non-UUID) keys, and only at ≥ 3 distinct buyer sets. So when a publisher reuses a procedure UUID (a copied
template, a platform that reuses one BT-04, a typo-identical key), unrelated procedures fold into one Tender.
Supersession then lets the newest notice's buyer and texts win.

**Exhibit 1, tender 1110706** (read 2026-10-02):

| seq | notice | publication | buyer org | published |
|---|---|---|---|---|
| 1 | 26265518 | DÖE `0a746ecc-…-01` | 22170532 | 2024-02-05 |
| 2 | 23783071 | TED `00081139-2024` | 22170532 | 2024-02-07 |
| 3 | 24701149 | TED `00274114-2025` | **3869685 (Община Белоградчик, Bulgaria)** | 2025-04-29 |

All three carry BT-04 `f3943baf-54ae-441a-aee9-3e802998024a`. Seq 1–2 are a correct DÖE↔TED pair for a German
buyer (org 22170532, Wesel per 481's sampler). Seq 3 is a Bulgarian award notice that reused the same UUID 15 months
later. The Tender now presents `Община Белоградчик` as its buyer (`/v1/tenders/1110706` → `buyers:["Община
Белоградчик"]`), and the German procedure has lost its own identity.

**Exhibit 2, tender 42726:** 36 versions, 36 distinct TED publications (2024-09 → 2025), contract notices and award
notices, under BT-04 `083519c7-e0ad-4aab-9d71-05b39f0e55e3`. Several buyers are named (Gemeinde Blekendorf über das Amt
Lütjenburg, Stadt Bad Segeberg, …; 481's sampler counted 8). This is either a platform reusing one UUID for many
procedures, or an authority running every procedure under one key. The census decides which.

Evidence rows: `.scratch/tender-db/481-dedup/data/skeptic-collision.json`.

## First unit: census (dry job)

Over every UUID-keyed Tender with ≥ 2 notices, compare the buyer sets the NOTICES name. Use each notice's own
buyer mentions (normalised identifier, or `buyer_key()` from project.rs), not the Tender's accumulated parties. Report:
- Tenders whose notices name pairwise-disjoint buyer sets, split by: same country vs different countries; one source
  vs both; the time span between the first and the last notice;
- 30 samples per bucket for a reader;
- the class sizes. That says whether a guard is a gate (refuse the key, as 369 does) or a split (cut the component at
  a buyer-disjoint edge).

## Design direction (decide after the census)

- A notice whose buyer set is disjoint from every other notice under the key, from another country, and more than N
  months away is not the same procedure. Split it out as an island (or onto its own key), and tally it as
  `key_collisions` beside 369's `plan_refused_key`.
- Joint procurement and central purchasing bodies legitimately name several buyers. The guard must read set
  *overlap*: a notice that shares ≥ 1 buyer with the rest stays.
- Pin both exhibits as fixtures: 1110706's Bulgarian notice splits; a joint-procurement fixture stays merged.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/1110706 | jq -c '[(.versions | length), ([.parties[]? | select(.role|test("uyer")) | .organization_name] | unique)]'

- **open** (2026-10-02): `[3,["Община Белоградчик"]]`, a German procedure presented under a Bulgarian buyer.
- **done:** `[2,[<the German buyer>]]`, with the Bulgarian notice on a Tender of its own.
