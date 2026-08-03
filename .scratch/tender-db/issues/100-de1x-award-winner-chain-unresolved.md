# 100 — eForms-DE 1.x award-winner chain resolves to nothing (synthetic result-section ids vs published-id references)

Status: open — DISCOVERED 2026-08-02 (sdk-vendor, measured against snapshot `tender-db-1785661162.db`)
Kind: correctness / completeness (parse-layer identity)
Blocked by: —
Relates to: 75 (the section-id synthesis decision this comes from), 78 (the DÖE grafting that forced it),
98 (the organization-reference class — a *different* defect that does NOT fix this), 85, 13 (results layer)

## Symptom

DE-1.x award notices project their results **structure** but almost never a **winner**:

- of 247 award-bearing DE-1.x versions in a de-1.2 window, **16 (2%)** carry a winner — and those 16 are
  parties carried forward from a merged TED twin, not DE data (same carry-forward that made the buyer
  rate read 35% when the true DE contribution was 0%, issue 98);
- `lot_results` 247, `bids` 216, `contracts` 217 in the same window — the graph's *nodes* land fine.

So the results layer exists and is empty of the one fact that matters commercially: who won.

## Root cause — the references and the sections speak different id vocabularies

Measured over 50 DE-1.2 notices. `resolves` = the reference value matches a `notice_sections.section_id`
in the same notice:

| reference field | values | resolves | example value |
|---|---:|---:|---|
| `DE1-NoticeResult-LotResult-TenderLot-ID` (→ lot) | 20 | **20** | `LOT-0000` |
| `DE1-NoticeResult-LotTender-TenderLot-ID` (→ lot) | 18 | **18** | `LOT-0000` |
| `DE1-NoticeResult-TenderingParty-Tenderer-ID` (→ org) | 14 | **14** | `ORG-0001` |
| `DE1-NoticeResult-LotResult-LotTender-ID` (OPT-320) | 18 | **0** | `TEN-0000` |
| `DE1-NoticeResult-LotTender-TenderingParty-ID` (OPT-310) | 18 | **0** | `TPA-0000` |
| `DE1-NoticeResult-LotResult-SettledContract-ID` (OPT-315) | 18 | **0** | `CON-0000` |
| `DE1-NoticeResult-SettledContract-LotTender-ID` (BT-3202) | 18 | **0** | `TEN-0000` |
| `DE1-NoticeResult-LotResult-ID` (own id) | 20 | 0 | `RES-0000` |
| `DE1-NoticeResult-LotTender-ID` (own id) | 18 | 0 | `TEN-0000` |

**References to lots and organizations resolve; every result-to-result reference resolves to nothing.**

Issue 75 gave the result nodes (`LotResult`, `LotTender`, `SettledContract`, `TenderingParty`)
**synthetic** section ids, because their published `RES-`/`TEN-`/`CON-`/`TPA-` ids repeat across the DÖE
serializer's grafted positions (issue 78) and would collide; lots and organizations kept their published
ids, which is why those two resolve. But the *references* still carry the published ids. So in
`read_results` (project.rs):

```rust
("OPT-320", NoticeValue::Id { value, .. }) => r.bid_refs.push(value.clone()),   // "TEN-0000"
...
let Some(b) = raw.bids.iter_mut().find(|b| b.key == owner)                      // key is "LotTender#0"
```

`r.bid_refs` never matches a bid key, `b.party_ref` never matches a tendering party, and the winner
chain **LotResult →(OPT-320)→ LotTender →(OPT-310)→ TenderingParty → Tenderer** breaks at every hop.

## Why issue 98 does not fix this

98 flags organization references so the role arm sees them. `read_results` matches
`NoticeValue::Id { value, .. }` and **never tests `is_ref`**, so the results graph was never gated on
that flag — it is gated on section-id identity, which 98 does not touch. After 98 the cohort gains
buyers and every lot role; **winners stay at 0% from DE**. Confirmed before folding rather than after,
which is the point.

## Fix direction (not costed yet)

Result sections need identity that matches what the references name: **the published id where it is
unique within the notice, disambiguated only where it genuinely collides.** The naive "just key on the
published id" is wrong — it reintroduces exactly the collisions issue 75 measured, which is why the ids
were synthesised in the first place.

This is a **parse-layer** change: `notice_sections.section_id` is written at parse time. So unlike 98 it
cannot ride a re-fold — it needs the cohort **re-parsed from the archive** (the issue-76 reprocess
mechanism), then re-folded. Materially more expensive than 98/99, hence its own issue and its own batch.

**Open question to settle first, and it decides the whole design:** are the published result ids unique
*within a single notice*, or do they collide there too? Issue 75 recorded them repeating "across grafted
positions"; whether that means *within one notice* or *across notices* is the difference between a
one-line fix and a real disambiguation scheme. **I attempted this measurement and it did not complete**
(the query was starved by contention on the box) — so it is recorded here as unmeasured, not as known.
Measure it before designing:

```sql
SELECT ni.field_id, COUNT(*) AS vals,
       COUNT(*) - COUNT(DISTINCT ni.notice_id || '/' || ni.value) AS duplicates_within_a_notice
  FROM notice_ids ni
 WHERE ni.notice_id BETWEEN <a> AND <b>
   AND ni.field_id IN ('DE1-NoticeResult-LotResult-ID','DE1-NoticeResult-LotTender-ID',
                       'DE1-NoticeResult-SettledContract-ID','DE1-NoticeResult-TenderingParty-ID')
 GROUP BY ni.field_id;   -- duplicates = 0 → published ids are safe as section ids
```

## Verification this needs

The suite gained party provenance for 98 (`C11`/`C12`/`H7`: `mention_notice_id` = the DE notice, not a
merged twin). Winners need the same shape, because a DE version can show a winner inherited from its TED
twin — that is exactly what the 16 of 247 are:

```sql
-- winners EVIDENCED BY the DE notice itself; pre-fix this is 0
SELECT COUNT(*) FROM notices n
  JOIN tender_versions v ON v.caused_by_notice_id = n.id
  JOIN tender_version_result_winners w ON w.tender_id = v.tender_id AND w.seq = v.seq
  JOIN organization_mentions om ON om.organization_id = w.organization_id AND om.notice_id = n.id
 WHERE n.profile IN ('eforms:eforms-de-1.0','eforms:eforms-de-1.1','eforms:eforms-de-1.2');
```

## Scope decision (team-lead, 2026-08-02)

Winners are **out of scope for the 98+99 fold**. That fold lands buyers, every lot role and the 2,185
skipped shells; the DE cohort goes live materially better than today but **without award winners**. The
ledger/dashboard wording for that gap is the user's call and is deliberately not touched here.

## Note

Third defect traceable to the issue-75 empirical inventory, after the `id` vs `id-ref` typing and the
missing role aliases (both issue 98). The common root is that an empirical inventory derived from
observed XML can recover *structure* but not *semantics* — which ids are references, and which ids are
identities the references will name. Both need the SDK's own model, or an explicit check against it. That
lesson belongs in the DE-2.x work and any future dialect vendoring, not just here.
