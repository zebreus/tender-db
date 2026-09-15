# 398 — `/v1/notices/{id}` serves the reclaimed quarantine record on a parsed, projected notice: the read has no stamp filter, so `quarantine != null` means "ever held", not the documented "held"

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: docs (the served contract — `NoticeDetail` in `crates/app/data/openapi.json:989`, mirrored in the two code docstrings at `crates/store/src/read.rs:3079` and `crates/app/src/v1/json.rs:124-128`; the alternative fix is a one-line predicate in `read::notice_quarantine`, so triage's job is to pick which half is the stale one)
Relates to: 218 (RESOLVED — Part A, rev `d127ccb`, is what put this field on the detail; its acceptance reads "held notice 20 → zero sections (its `quarantine` field on the detail says why)", i.e. held vs never-held only, and it is silent on what a RECLAIMED notice should serve), 137 (the measurement that makes this the majority shape, not the exception: 1,734,594 quarantine rows — **71.7%** — were already reclaimed on 2026-08-05, and the retained row is the ledger's design, not a leak), 288 (CLOSED 2026-08-26 — the sibling question about these same three outcomes one layer down; it settled "reclaimed WINS" for `quarantine_resolution`'s counters, and nobody settled the same question for this endpoint), 87 (RESOLVED-VERIFIED — the neighbouring "a served quarantine field is stale" class, there the reason on a still-held row), 40 / 76 (the resolution ledger and the reprocess mechanism the historical row exists to serve), 370 (DONE units 1,2,3,5 — the class this belongs to: served claims that are hand-written prose with no gate re-deriving them from behaviour), 391 (filed by this fan-out — the same drift cluster in the same three files; its unit 5 is `components.schemas` BREADTH, not `NoticeDetail.quarantine`'s meaning, so this is deliberately filed apart)

## Observed (verified 2026-09-14 on prod)

```
curl -sS 'https://tenders.zebreus.click/v1/notices/28783598'
```

→ `"parse_state":"parsed"`, `"published_at":"2026-08-18T22:00:00Z"`, and a non-null quarantine object:

```json
"quarantine":{"reason":"unrepresentable-value",
              "detail":"BT-161-NoticeResult: amount has more than two fraction digits: 555.242 at ...",
              "first_seen":"2026-08-21T07:36:16Z",
              "reprocessed_at":"2026-08-22T01:04:15Z",
              "skipped_at":null,...}
```

The notice is not held by any measure other than that field. Bounded, id-scoped reads on the box:

```
ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@zebreus.click 'echo "SELECT count(*) AS sections, sum(parent_section_id IS NULL) AS roots FROM notice_sections WHERE notice_id = 28783598" | /root/sq.sh'
```

→ `[26, 1]` — 26 sections, one root. `/v1/notices/28783598/content` serves the same 26.

| notice | `parse_state` | `published_at` | sections | `quarantine` | `reprocessed_at` |
| --- | --- | --- | --- | --- | --- |
| 28783598 | `parsed` | 2026-08-18T22:00:00Z | 26 | **present** | 2026-08-22T01:04:15Z |
| 28758103 | `parsed` | set | parsed tree | **present** | 2026-08-22T01:04:15Z |
| 28756078 | `parsed` | set | parsed tree | **present** | 2026-08-22T01:04:15Z |
| 28783590 (control, no quarantine row) | `parsed` | set | parsed tree | `null` | — |
| 31276597 (control, genuinely held) | `quarantined` | `null` | 0 | present | `null` |

So it is the row's EXISTENCE, not its outcome, that drives the field. How common that is:

```
ssh ... 'echo "SELECT count(*) AS n, sum(reprocessed_at IS NOT NULL) AS reclaimed, sum(skipped_at IS NOT NULL) AS skipped, sum(reprocessed_at IS NULL AND skipped_at IS NULL) AS outstanding FROM quarantine WHERE id BETWEEN 4300000 AND 4313674" | /root/sq.sh'
```

| quarantine id band | rows | reclaimed | skipped | outstanding |
| --- | --- | --- | --- | --- |
| 4300000–4313674 | 11,766 | **11,737** | 0 | 29 |
| 4200000–4213674 (adjacent) | 13,675 | **13,675** | 0 | 0 |

All 11,737 reclaimed rows in the first band join to notices with `parse_state = 'parsed'`. Corpus-wide, issue 137 measured the same shape at 71.7%.

Source. `crates/store/src/read.rs:3080`:

```rust
"SELECT reason, detail, profile, first_seen, attempts, last_attempt_at,
        reprocessed_at, skipped_at, skipped_reason, first_reason, first_detail
   FROM quarantine WHERE notice_id = ? ORDER BY first_seen DESC LIMIT 1",
```

No filter on `reprocessed_at`/`skipped_at`, under a docstring that says "`None` means the notice is not held" (`read.rs:3079`). The handler (`crates/app/src/v1/mod.rs:1216-1228`) attaches whatever comes back, and `crates/app/src/v1/json.rs:124-128` repeats the claim: "`null` when the notice parsed".

The spec contradicts itself inside one document:

| `openapi.json` | says |
| --- | --- |
| `NoticeDetail.quarantine` (`:989`) | "Why the notice is held out of the canonical layer, or null when it parsed." |
| `NoticeDetail.description` | "A held notice has no parsed satellites and no canonical tender, so its quarantine row is the only content it carries." |
| `Quarantine.description` | "The terminal stamps say which outcome the member reached: outstanding (both null), reclaimed (`reprocessed_at` set), or skipped-by-policy (`skipped_at` set)." |
| `Quarantine.reprocessed_at` | "set when the member was reclaimed into the corpus" |

The last two are only observable if reclaimed records are served, which is exactly what the first two deny.

## Why it matters

The field's documented meaning is a predicate: `quarantine != null` ⇒ this notice is held out of the canonical layer, has no parsed satellites and no tender. A client that codes that predicate — the only one the spec offers — classifies 28783598 as held while it carries 26 sections and backs a live tender version. In the 11,766-row band above it gets 11,737 of them wrong and 29 right, so the documented reading is wrong for **99.75%** of the notices it applies to, and issue 137's corpus figure says that is not an artefact of the band.

The failure is silent and it is inverted in the worst direction: a "held" classification is a reason to go looking for the notice's content elsewhere, or to exclude it from a count, when the content is right there. A coverage or completeness dashboard built on the served contract under-reports by the size of the reclaim campaign — which is most of the quarantine.

`parse_state` disambiguates today, but nothing in the contract tells a client to prefer it, and the one API test on the field (`notice_detail_carries_the_quarantine_field`, `crates/app/tests/api.rs:1382`) asserts "a parsed notice is not held → quarantine null" against a fixture that was never quarantined. The reclaimed-then-parsed shape — the common one — is untested and undocumented.

## Why this is ours, not the publisher's

No publisher fact is anywhere near this. The quarantine ledger, the three terminal stamps and this endpoint are all tender-db's own design, and the served shape is deliberate: issues 84/137 record that a quarantine row is retained as a historical record after reclaim, which is what makes the reclaim ledger (40) and the reprocess mechanism (76) auditable. So the behaviour is right and the prose is the stale half — issue 218 introduced the field for held-vs-never-held and never revisited it when reclaim became the dominant outcome. That makes this issue 370's class one layer down: the contract is a hand-written string constant, the two spec guards compare NAMES only, and nothing re-derives "null when it parsed" from a notice that actually parsed.

## Repro

Under two minutes, no token; the SQL is id-bounded, so it is a free read:

1. `curl -sS 'https://tenders.zebreus.click/v1/notices/28783598'` → `parse_state "parsed"`, `published_at "2026-08-18T22:00:00Z"`, `quarantine.reason "unrepresentable-value"`, `quarantine.reprocessed_at "2026-08-22T01:04:15Z"`, `skipped_at null`.
2. `curl -sS 'https://tenders.zebreus.click/v1/notices/28783598/content' | head -c 300` → a section tree; SQL `notice_sections` for the same id → `[26, 1]`. Not held, by the detail's own definition of held.
3. `curl -sS 'https://tenders.zebreus.click/v1/notices/28783590'` (control, no quarantine row) → `quarantine: null`. The difference between 2 and 3 is the row's existence, not its outcome.
4. `curl -sS 'https://tenders.zebreus.click/v1/notices/31276597'` (control, genuinely held) → `parse_state "quarantined"`, `published_at null`, `reprocessed_at null`, `/content` → 0 sections.
5. The band aggregate: `SELECT count(*), sum(reprocessed_at IS NOT NULL), sum(skipped_at IS NOT NULL), sum(reprocessed_at IS NULL AND skipped_at IS NULL) FROM quarantine WHERE id BETWEEN 4300000 AND 4313674` → `[11766, 11737, 0, 29]`. Ids 28758103 and 28756078 show the identical shape to step 1.

## Done when

- Triage has picked ONE of the two answers, and the same sentence is true in all four places that state it: `openapi.json:989`, the `NoticeDetail` description, `crates/store/src/read.rs:3079`, `crates/app/src/v1/json.rs:124-128`. The two are not equally good — see the note below.
- **If the contract is amended (recommended):** `NoticeDetail.quarantine` says the field carries the notice's hold HISTORY with its terminal stamps, that `parse_state` — not the presence of `quarantine` — says whether the notice is held today, and that `reprocessed_at != null` means the member was reclaimed and its content is served. The `NoticeDetail` description stops asserting "a held notice has no parsed satellites" as a property of every notice carrying the field, and stops contradicting the `Quarantine` schema three paragraphs away.
- **If the read is filtered instead:** `read::notice_quarantine` gains `AND reprocessed_at IS NULL` (skipped rows STAY — a skipped member is still out of the corpus), 28783598 serves `quarantine: null`, and `/docs` names `/v1/sql` as where the hold history lives. This throws away the only REST path to a reclaim record, which is why it is the weaker half.
- The reclaimed shape is pinned by a test rather than left to prod: `notice_detail_carries_the_quarantine_field` (`crates/app/tests/api.rs:1382`) grows a fixture that is quarantined, then reclaimed, then parsed, and asserts whichever answer triage picked. Its comment stops implying that "parsed" and "no quarantine row" are the same condition. The store arm lands beside the held case in `crates/store/tests/notice_quarantine.rs`, where a held row can actually be minted.
- Controls unchanged after the fix: 31276597 → `parse_state "quarantined"`, non-null quarantine with `reprocessed_at null`, `/content` 0 sections; 28783590 → `quarantine: null`.
- One sentence somewhere a client reads answers "how do I ask whether this notice is held?" — and it names `parse_state`.
