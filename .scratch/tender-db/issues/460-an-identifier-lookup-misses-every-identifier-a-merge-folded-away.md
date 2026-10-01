# 460 — an identifier lookup misses every identifier a merge folded away, and the survivor serves only its own

Status: ready-for-agent — DEPLOYED 2026-10-01 at `b26cf3a` (built `24ed6f2`, review fixes `f22bcfb`). Dry backfill 1831 counts 40,294 merged identifiers to write (e2-altid 4,736, r2 35,540, r3 18, e0 0; 0 unresolved, 1 wrong number withheld). The WET backfill is ENQUEUED as job 1846, behind the 477 chunk 2022. NEXT: read 1846, then the Verify (`GBPPONPWYP8439MZWY` → 5718658 with `resolved_filters`).
Was status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the store half: a merged-identifier table that `repoint_org_references` fills at every merge, backfilled from the 4,738 `e2-altid` ledger rows, and read by both `identifier=` builders.
Kind: API correctness (stable identifiers)
Relates to: 455 (the org-id half of this defect, done), 448 (e2-altid; its design doc left this open at
`.scratch/tender-db/448-altid-design.md:246`, "Decide whether to file a follow-up", and nothing was filed),
453/452 (rekey folds a wrong number, which must not become a lookup key), 456 (an identifier bound to the wrong org),
329 (one identifier can answer several orgs), 284 (both org builders apply `identifier`)

## What is wrong

An official identifier is the other handle a reader keeps. `/docs` (`crates/app/src/v1/docs.rs:263`, served live)
calls `identifier=` "the front door to participation history": resolve the identifier to an org id, then ask
`?buyer=`, `?winner=` or `?bidder=`. Both org builders match it exactly against the org's single identifier column,
`AND o.identifier = ?` (`crates/store/src/read.rs:3815` id-ordered, `:3880` name-ordered). Every merge path calls
`repoint_org_references` (`crates/store/src/canonical.rs:2038`, seven call sites) and then deletes the loser's row,
so the loser's identifier leaves the index with it. 455 made the loser's org ID resolve (`read::resolve_org`, and
`resolve_org_filters` at `crates/app/src/v1/mod.rs:1757` for `buyer`, `winner` and `bidder`). It did not touch
`identifier=`, and the survivor's row has no place for a second identifier.

Exhibit: Sellafield. Per 456, its PPON org 31627746 (`GB-PPON-PWYP-8439-MZWY`) was folded into the
company-number org 5718658 by 448's job 1787 (`match-org-identifiers altid` WET, finished 2026-10-01 10:08 UTC,
36 pairs merged). Read on the public API 2026-10-01 11:4x UTC:

| request | answer |
|---|---|
| `GET /v1/organizations/31627746` | `308`, `location: /v1/organizations/5718658`, `"merged_into":5718658` (455 works) |
| `GET /v1/organizations?identifier=GBPPONPWYP8439MZWY` | **`200`, `items: []`, `resolved_filters: null`, `ignored_filters: []`** |
| the same with `&kind=national` | the same empty page |
| `GET /v1/organizations/5718658` | `"identifier":"01002607"`, `national`, 889 mentions. No field names the PPON |
| `GET /v1/organizations?identifier=GBPPONPBZB4962TVLR` (Crown Commercial Service, never merged) | 1 item, 30914406 |

The empty page is well formed and certified complete. That is the silent-empty shape 455 fixed for `?winner=`.

**Scale.** `e2-altid` alone wrote 4,738 ledger rows: 50 (job 1717), 4,652 (1765) and 36 (1787). The fold's alias
reads them as 4,736 PPON keys (342, project 1798). Each of those PPONs is published on FTS notices, and a lookup by
it now answers nothing. `r2` and `r3` also fold an org whose literal can differ from the survivor's (another
spelling of one canonical key; evidence `$.loser_id`), so those spellings miss too. Their count needs a ledger read
and is not known. `e0` merges identical `(country, kind, identifier)` groups, so it loses no literal. `rekey`
(453) folds a wrong number, which is the subject of the exception below.

**Out of scope: spelling.** The filter matches the stored, normalised value only.
`?identifier=GB-PPON-PBZB-4962-TVLR`, the form FTS publishes, answers `[]` for live CCS, while `GBPPONPBZB4962TVLR`
answers 30914406. Likewise `01002607` answers 2 orgs and `GB-COH-01002607` answers none. That is the documented
behaviour ("matches the official identifier value"). Whether to normalise the asked value is a separate question.
This issue is about values the index HELD and a merge removed.

## Proposed fix

The root cause is that an org can carry several identifiers (a company number and a PPON, or two spellings), but
the identity index is one column on `organizations`. A merge has no place to keep the second one. Give it a place,
written where every merge already passes, and read it the way 455 reads the merge ledger.

1. **Table.** `organization_merged_identifiers(identifier, identifier_kind, country, org_id, loser, rule)`, with
   `PRIMARY KEY (identifier, loser)` and an index on `(org_id)`. STRICT. `org_id` is the live holder.
2. **Write, in `repoint_org_references`**, which takes the rule. Before the caller's `DELETE`:
   - repoint the loser's own merged identifiers (`UPDATE … SET org_id = keep WHERE org_id = loser`), so a chain
     A→B→C carries A's identifier to C;
   - insert the loser's own `identifier`, unless it is NULL or equal to the survivor's.
   - **Exception: `rekey`.** The loser's number is another company's (452/453), and a lookup by it must not answer
     this org. It is not written.
3. **Backfill, a one-shot job.** Walk `org_merge_log WHERE rule = 'e2-altid'` (4,738 rows, through the partial index
   `org_merge_log_e2_altid`). There `$.loser_id` is the PPON org's `organizations.identifier` as stored (the altid
   owner read, `canonical.rs:15558`). The evidence names no country or kind. Both sides of an altid pair are GB
   register values that normalise to `national` (`ingest/src/project.rs`,
   `the_uk_registers_normalise_to_national_identifiers_with_their_prefix`), so the backfill takes country and kind
   from the survivor's row. Resolve each `keep` through `resolve_org` to today's survivor. `r2`/`r3` rows carry the loser's literal in `$.loser_id` too, but `p0`/`p1`
   rows use that key for the numeric org id. So the backfill reads per rule and reports a count per rule. A walk
   over the whole ledger (5.76M `p0` rows) is a job, never `/v1/sql`.
4. **Read.** Both builders (`read.rs:3815` and `:3880`, kept in agreement by
   `org_name_search.rs::the_name_search_honours_identifier_and_buyer_filters`) match `o.identifier = ?` OR a merged
   row for the value. `kind` then constrains the matched identifier's kind. Both halves must be seeks:
   `organizations_identifier_id (identifier, id)` and the new primary key. A turso OR can fall back to a scan. If the
   plan guard shows one, seek the merged table first and union in Rust, the `resolve_org` shape. `walks()` must stay
   false for `identifier=`.
5. **Say so on the page**, as 455 does for org ids:
   `"resolved_filters":{"identifier":{"asked":"GBPPONPWYP8439MZWY","merged_into":[5718658]}}`. It is an array,
   because an identifier can answer several orgs (329). It is present only when an item matched through a merged
   row.
6. **Serve it on the survivor.** `json::organization` gains `merged_identifiers: [{identifier, identifier_kind,
   country}]` (empty when none), one seek on `(org_id)`. The detail and the list rows then agree.
7. **Docs + OpenAPI.** The lookups paragraph (`docs.rs:263`) and the id-stability paragraph from 455 say that
   `identifier=` also finds an identifier a merge folded into an org, and that the page reports it in
   `resolved_filters`. `merged_identifiers` goes into the Organization schema.

Tests that pin it:
- store, `crates/store/tests/merged_org.rs`, `a_merged_away_identifier_finds_its_survivor`:
  - a PPON org merged into a company-number org is found by its PPON, in both builders;
  - a second merge (A→B→C) carries it to C;
  - a live org's own identifier still matches, and an unseen value stays empty;
  - a `rekey` loser's number is not written.
- store, `the_altid_ledger_backfills_merged_identifiers`: ledger rows written before the table existed are
  backfilled to today's survivor.
- handler, `crates/app/tests/api.rs::a_merged_away_identifier_answers_its_survivor_with_resolved_filters`:
  - the survivor on the page, with `resolved_filters.identifier`;
  - `merged_identifiers` on the detail;
  - no `resolved_filters` for a live identifier.
- The plan guard covers both org builders with `identifier=` set.

## Verify

    curl -s 'https://tenders.zebreus.click/v1/organizations?identifier=GBPPONPWYP8439MZWY' | jq -c '[[.items[].id], .resolved_filters]'

- **open** (2026-10-01 11:46 UTC): `[[],null]`. Sellafield's merged PPON finds nothing.
- **done**: `[[5718658],{"identifier":{"asked":"GBPPONPWYP8439MZWY","merged_into":[5718658]}}]`. It finds the
  survivor, and the page says it followed a merge.

## 2026-10-01 20:3x UTC — deployed; dry backfill read

Dry job 1831 (18 s): 5,895,067 ledger rows walked (era floor 1786785861); 30,329 survivors would change.

| rule | rows | written | present | same as survivor | wrong number |
|---|---|---|---|---|---|
| e2-altid | 4,738 | 4,736 | 2 | 0 | 0 |
| r2 | 56,414 | 35,540 | 952 | 19,921 | 1 |
| e0 | 2,501 | 0 | 0 | 2,501 | 0 |
| r3 | 1,287 | 18 | 65 | 1,204 | 0 |

The 2 e2-altid `present` are the two PPONs that were merged twice (448: 4,738 rows, 4,736 keys). Wet job 1846 is
queued.
