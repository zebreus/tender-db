# 391 — the documented contract drifts from what the server does: the spec's flagship "closes soon" query answers 400, the SQL schema's headline note teaches the silent-NULL idiom its own column notes warn about, and three published shapes are shapes nothing serves

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: docs (the served contract — `crates/app/data/openapi.json`, `/docs` in `crates/app/src/v1/docs.rs`, and the `/v1/sql/schema` notes in `crates/app/src/v1/sql.rs`; unit 1 is equally an app defect, because teaching `parse_instant` the word `now` is the other half of its fix)
Relates to: 216 (RESOLVED — it shipped `sort=deadline` + `deadline_after/_before`, and its status line records `deadline_after=now` as a "prod-verified flagship"; unit 1 is that record being impossible, since the parser it shipped rejects the literal), 227 (RESOLVED — the `/docs`-vs-spec guard `the_docs_page_names_the_whole_spec_surface`, an anchored byte-grep over parameter and path NAMES; it cannot see an example literal, an example block or a schema's field set, which is why units 1, 4 and 5 are invisible to it), 215 (RESOLVED — the OpenAPI drift cluster; it fixed four drifts in this same document and never touched component-schema breadth, unit 5), 239 (its "Two documentation defects found by running the endpoint's own advice (FIXED, `8519810`)" is the fix unit 2 shows is incomplete — `8519810` rewrote `EPOCH_NOTE` and the examples, never `notes[2]`), 50 (RESOLVED-VERIFIED 2026-08-17 — its acceptance clauses quote the reversed `strftime(col,'unixepoch')` form verbatim as satisfied (unit 2) and claim "every timestamp column carries an epoch-seconds note" (unit 3); both were blessed by a verification that never held), 370 (DONE units 1,2,3,5 — the class all five of these belong to: served claims that are hand-written consts with no gate re-deriving them from behaviour; its unit 2 restates `provisional` on four surfaces and its "the OpenAPI if it repeats it" clause is moot precisely because of unit 5, the spec never mentions the field), 218-B (RESOLVED, rev `4c3c367` — landed `/v1/notices/{id}/content`, the endpoint whose `/docs` example unit 4 is about and which unit 5 shows is missing from the `/v1` endpoint list), 292 (FIX DEPLOYED — the three lang-tag dialects; its `normalize_lang` applies at the fold boundary to `tender_version_texts`, NOT to `notice_texts`, which is why unit 4's `lang` half is "the source's tag verbatim" and not "uppercase 639-2/T"), 211 / 118 (RESOLVED — earlier spec-vs-served vocabulary drifts on `/v1/changes` and the ignored-filter array; neither covers schema breadth), 387 / 389 / 390 (filed by this fan-out — 390 unit 4 carries the `limit` clamp-vs-reject half of the spec mismatch, so it is deliberately out of scope here)

## What ties these five together

Every one of these is the system's own words about itself. No publisher fact appears in any of them, no stored row is wrong, and no fold is involved: in each unit a **served contract** — `openapi.json`, the `/docs` page, or the `/v1/sql/schema` notes — states something the server does not do. Three of the five are contradicted by the very same document three paragraphs away.

They are one issue because they share a cause worth naming once: **the served contract is a set of hand-written string constants, and the two guards that exist compare NAMES, never values.** `the_openapi_spec_matches_the_served_surface` (`crates/app/tests/api.rs:2104`) checks spec paths against the router; `the_docs_page_names_the_whole_spec_surface` (`api.rs:2193`) checks that every spec parameter and path name occurs somewhere in `docs.rs`. Neither reads an example literal, an example block, a description sentence, a component schema's property list, or a per-column note. So a description can advertise a query the parser rejects (unit 1), a note can teach the trap its neighbours warn about (unit 2), a promise of completeness can be 14/26 true (unit 3), an example can show a shape no parser can emit (unit 4), and a schema can declare 2 of 7 served fields (unit 5) — all while the gate stays green. This is issue 370's class, one layer down: 370 fixed the claims, and nothing yet re-derives them.

| unit | surface | it says | the server does | severity |
| --- | --- | --- | --- | --- |
| 1 | `openapi.json:796` (`deadline_after`) | "`deadline_after=now` … is the \"closes soon\" query" | 400 on `now` — one parser, all four bounds | medium |
| 2 | `/v1/sql/schema` `notes[2]` | "Filter/format with `strftime(col,'unixepoch')`" | that order returns NULL for every row, silently | medium |
| 3 | `notes[2]`, `/docs`, the OpenAPI `sqlSchema` description | "each timestamp column's note flags this" | 14 of 26 time columns are flagged; 12 are not | low |
| 4 | `/docs` notice-content example (`docs.rs:269-271`) | `"section_id": 1`, `"kind": "root"`, `"lang": "deu"`, notice 14327 | `"PROCEDURE"`, `"Notice"`, `"SPA"`, 404 | low |
| 5 | `components.schemas.{Tender,Lot,Organization,Notice}` | 6 / 2 / 2 / 2 properties | 16 / 8 / 7 / 11 keys; `/docs` leans on one the spec omits | low |

Fixing them together is one review of three files (`openapi.json`, `docs.rs`, `sql.rs`) plus one decision that applies to all five: **which of these strings should stop being strings.** Units 1, 3 and 5 each have a derivable form (run the spec's own examples; match `*_at`/`*_utc`/`*deadline*` by name; serialize one row per collection and diff its keys against the declared properties), and that is the shape of fix worth taking — issue 370's units 1 and 3 already argued it for the prose.

---

## Unit 1 — the spec's flagship "closes soon" query, `deadline_after=now`, answers 400

The served OpenAPI advertises a moving-instant literal as the headline example of the endpoint's headline query. `parse_instant()` has never accepted it, and issue 216's resolution note records it as prod-verified — a verification the parser cannot have passed.

### Observed (verified 2026-09-13 on prod, live rev `9e082fd`)

```
curl -sS 'https://tenders.zebreus.click/v1/tenders?deadline_after=now&limit=5'
-> HTTP 400 {"error":{"message":"deadline_after must be unix seconds or RFC 3339, not \"now\"","status":400}}
```

The sentence it contradicts, served at `/v1/openapi.json` (`components.parameters.deadline_after.description`; source `crates/app/data/openapi.json:796`, byte-identical to the served copy):

> "`deadline_after=now` with sort=deadline&order=asc is the \"closes soon\" query"

| request | status | result |
| --- | --- | --- |
| `?deadline_after=now&limit=5` | 400 | `deadline_after must be unix seconds or RFC 3339, not "now"` |
| `?deadline_after=now&sort=deadline&order=asc&limit=3` (the spec's exact flagship form) | 400 | same message |
| `?deadline_before=now` | 400 | same message, same parser |
| `?published_after=now` | 400 | same message, same parser |
| `?deadline_after=today` | 400 | same message — it is the generic parser, not a `deadline_after` special case |
| `?deadline_after=1789344001&sort=deadline&order=asc&limit=3` | 200 | 3 tenders, deadlines 2026-09-14T06:00+02:00, 07:00+02:00, 08:00+03:00, cursor `d1789362000.894503` |
| `?deadline_after=2026-09-13T00:00:00Z&deadline_before=2026-09-14T00:00:00Z&limit=5` | 200 | 5 rows, deadline asc, `next_cursor` `d1789318859.7921827` |

`1789344001` is 2026-09-14T00:00:01Z, i.e. the unix-seconds spelling of the same intent, and it returns exactly the page the spec promises `now` would.

Source: `parse_instant()` at `crates/app/src/v1/mod.rs:901-923` accepts only an `i64` or `chrono::parse_from_rfc3339`, and it is the parser for all four bounds (`mod.rs:648-651`). A repo-wide grep finds the literal `now` only in the spec data file and on the board; `git log -G'"now"' -- crates/app/src/v1/mod.rs` is empty, so it was never accepted and this is not a regression.

Two scope corrections from the reproduction, worth carrying into triage:

1. **`/docs` is not affected.** Its filter table only says "pair with `status=open` and `deadline_after`", and its worked example uses unix seconds (`deadline_after=1786910000`). The drift is confined to the OpenAPI description — and to the board.
2. **Issue 216's status line is the second carrier.** It reads "Prod-verified flagships: \"closes soon\" (`deadline_after=now`) returns tenders closing TONIGHT, soonest first, in 13 ms with a tie-safe composite cursor". The 13 ms measurement is real (the unix-seconds form does that), but the literal recorded beside it returns 400, so the board records a verified flagship that cannot have been run as written.

### Why it matters

This is the one place in the machine-readable contract where a *moving* bound is offered, and "which tenders close soon" is the query the endpoint exists for. A consumer who generates a client from the spec, or who reads the parameter description and hard-codes the documented example, deterministically gets a 400 on the endpoint's advertised headline use. The workaround (compute the instant client-side, pass unix seconds or RFC 3339) is trivial once the 400 body is read, which is why this is medium and not high — but the body is only reached by a human, and the spec is offered to "client generators and API tooling".

### Why this is ours, not the publisher's

The judge: this is a contract the system wrote itself — a description in our own `openapi.json` against a parser in our own `v1/mod.rs`, with no source-published shape anywhere in it. It is also not a stale sentence that behaviour drifted away from: `git log -G'"now"'` over the serving crate is empty, so the spec has been advertising a literal the code never handled since the day 216 landed. Both gates are blind to it by construction — 227's guard walks parameter NAMES, and `the_openapi_spec_matches_the_served_surface` walks paths and methods — so nothing was going to catch an example that does not run.

### Repro

Under 30 seconds, no token:

1. `curl -sS 'https://tenders.zebreus.click/v1/tenders?deadline_after=now&sort=deadline&order=asc&limit=3'` → 400.
2. `curl -sS 'https://tenders.zebreus.click/v1/openapi.json' | python3 -c "import json,sys; print(json.load(sys.stdin)['components']['parameters']['deadline_after']['description'])"` → the sentence that advertises it.
3. `curl -sS 'https://tenders.zebreus.click/v1/tenders?deadline_after=2026-09-16T00:00:00Z&sort=deadline&order=asc&limit=3'` → 200, the closes-soon page, proving the bound itself works.

### Done when

- One decision is taken and applied in one direction, not both: **either** `parse_instant()` accepts `now` (`Utc::now().timestamp()`) for all four bounds — the honest reading, since a moving instant is exactly what a "closes soon" query needs — **or** `crates/app/data/openapi.json:796` stops naming it and gives a form that works.
- If accepted: `GET /v1/tenders?deadline_after=now&sort=deadline&order=asc&limit=3` returns, at that moment, the same three ids as the unix-seconds form, and a unit test pins `now`, `deadline_before=now`, `published_after=now` and `published_before=now` (one parser, four bounds) plus the still-400 on `today`.
- If removed: `deadline_after=now` still answers 400 with the current self-describing message, and the spec's closes-soon example is a literal that returns 200.
- Issue 216's status line is corrected either way — the flagship it records as prod-verified must name a query the server accepts.
- A guard that reaches example literals: extract every `name=value` example from `openapi.json` descriptions and from `docs.rs`, fire each at the test server, and assert no 4xx. That is the only one of the five units' guards that also covers unit 4's dead `notice_id`, and it is what neither existing gate does.

---

## Unit 2 — `/v1/sql/schema`'s headline note prescribes the reversed `strftime` its own column notes call a silent-NULL trap

### Observed (verified 2026-09-14 on prod)

```
curl -sS https://tenders.zebreus.click/v1/sql/schema | jq -r '.notes[2]'
```

> "Time columns are Unix epoch seconds, NOT ISO — the REST API returns ISO, so the two disagree. Filter/format with strftime(col,'unixepoch'); each timestamp column's note flags this. WHERE published_at LIKE '2012%' silently matches nothing."

The same document, every one of its 14 timestamp column notes (one shared `EPOCH_NOTE`, `crates/app/src/v1/sql.rs:546-551`):

> "Put the FORMAT FIRST: strftime('%Y', published_at, 'unixepoch'). The reversed order returns NULL for every row without erroring — measured on prod, one NULL bucket holding all 7,924,659 Tenders (issue 239)."

| surface | the idiom it teaches |
| --- | --- |
| `/v1/sql/schema` `notes[2]` (`sql.rs:788`) | `strftime(col,'unixepoch')` — **reversed** |
| the 14 per-column notes (`EPOCH_NOTE`, `sql.rs:546-551`) | format first |
| the schema's `examples` array (its one strftime example) | format first — `strftime('%Y', current_published_at, 'unixepoch')` |
| `/docs` SQL-rules bullet (`docs.rs:379`) | format first, and names the reversed form as the trap |
| issue 50, line 5, quoted as a satisfied acceptance clause | `strftime(col,'unixepoch')` — **reversed** |

Exactly one served string in the schema is wrong: `notes[2]`. Semantics confirmed in local SQLite 3.45.1 with neutral column names (the prod box read was declined by the permission classifier): `strftime(c,'unixepoch')` → NULL on all 3 rows, no error; `GROUP BY` on it → one NULL bucket of 3; `strftime('%Y',c,'unixepoch')` → 2012 / 2015 / 2025. That is documented SQLite behaviour (the first argument is the format; `'unixepoch'` as a time value is invalid) and it matches issue 239's prod measurement exactly.

Second half, same const, cosmetic: **10 of the 14 timestamp notes name `published_at` in an example on a column that is not `published_at`**, because `EPOCH_NOTE` is one shared string attached by wildcard (`sql.rs:642-649`, e.g. `("*", "dispatched_at", EPOCH_NOTE)`) — `v_fetches.fetched_at`, `v_tender_dates.utc_seconds`, `changes.changed_at`, `notice_dates.utc_seconds`, `notices.ingested_at`, `notices.dispatched_at`, `quarantine.first_seen`, `quarantine.reprocessed_at`, `tender_version_dates.utc_seconds`, `tender_versions.dispatched_at`. The pattern taught is still correct, so this half is drift, not a trap.

### Why it matters

`notes[2]` is the third line of the first document an analyst reads on the SQL surface, and it prescribes the one idiom on this endpoint that returns a **wrong answer with no error**. Issue 239 measured what that costs: a year histogram collapsing into one NULL bucket holding all 7,924,659 Tenders, with nothing raised. A consumer who copies the headline note and groups by year gets a single empty bucket and no signal that anything went wrong; a consumer who reads down to a column note is told the opposite by the same JSON response.

### Why this is ours, not the publisher's

The judge: this is system-authored served documentation, not a publisher artefact, and the board actively records it as fixed. Issue 239 closed "Two documentation defects found by running the endpoint's own advice (FIXED, `8519810`)" — but `8519810` rewrote `EPOCH_NOTE` and the `examples` array and never touched line 788, and the pinning test `the_documented_sql_idioms_are_ones_that_actually_work` (`sql.rs:1663-1671`) asserts only over `EPOCH_NOTE`, so the residual has no coverage. Issue 50 is RESOLVED-VERIFIED and quotes the wrong form verbatim as a satisfied clause, so the verification blessed the defect rather than catching it. A fix recorded as complete that is incomplete is worse than an open one.

### Repro

Under a minute, no token:

1. `curl -sS https://tenders.zebreus.click/v1/sql/schema | jq -r '.notes[2]'` → the reversed form.
2. `curl -sS https://tenders.zebreus.click/v1/sql/schema | jq -r '.tables[] | select(.name=="tender_versions") | .columns[] | select(.name=="published_at") | .note'` → the format-first form, in the same response.
3. `sqlite3 :memory: "SELECT strftime(1376092800,'unixepoch'), strftime('%Y',1376092800,'unixepoch');"` → `|2013` — NULL and no error on the left, the answer on the right.

### Done when

- `crates/app/src/v1/sql.rs:788` teaches the format-first idiom, and no served string anywhere in `/v1/sql/schema` or `/docs` contains `strftime(col,` or an equivalent reversed example.
- `the_documented_sql_idioms_are_ones_that_actually_work` walks the **whole served schema document** — `notes`, `examples` and every column note — not just `EPOCH_NOTE`, and fails on a reversed-order occurrence. Red-demonstrate it by restoring line 788 once.
- Better, if cheap: the test runs each `examples` entry and each note's inline query against the test DB and asserts a non-NULL result, so an idiom that returns NULL for every row cannot be published as advice again.
- `EPOCH_NOTE` templates the column name into its example (or drops the column name), so the 10 notes naming `published_at` on other columns name their own.
- Issue 50's acceptance clause and issue 239's "FIXED" note are corrected to say what actually shipped.

---

## Unit 3 — 12 epoch-seconds columns carry no timestamp note, while three surfaces promise every one is flagged

### Observed (verified 2026-09-14 on prod)

```
curl -s https://tenders.zebreus.click/v1/sql/schema | python3 -c "import json,sys; d=json.load(sys.stdin); print([(t['name'],c['name'],c['type']) for t in d['tables'] for c in t['columns'] if any(s in c['name'] for s in ('_at','_utc','deadline','publish_after')) and 'epoch' not in (c.get('note') or '').lower()])"
-> [('notice_withheld_fields','publish_after','TEXT'), ('v_awards','decided_utc','TEXT'), ('v_lot_results','decided_utc','TEXT'), ('organizations','created_at','INTEGER'), ('quarantine','skipped_at','INTEGER'), ('quarantine','last_attempt_at','INTEGER'), ('tender_version_contracts','concluded_utc','INTEGER'), ('tender_version_contracts','decided_utc','INTEGER'), ('tender_version_lot_results','decided_utc','INTEGER'), ('tenders','created_at','INTEGER'), ('tenders','current_published_at','INTEGER'), ('tenders','current_deadline','INTEGER')]
```

```
ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@zebreus.click 'echo "SELECT typeof(created_at), typeof(current_published_at), typeof(current_deadline), created_at, current_published_at, current_deadline FROM tenders WHERE id BETWEEN 8000000 AND 8000500 AND current_deadline IS NOT NULL LIMIT 1" | /root/sq.sh'
-> ["integer","integer","integer",1789284922,1376092800,1378728000]

ssh ... 'echo "SELECT typeof(created_at), created_at FROM organizations WHERE id BETWEEN 1 AND 1000 LIMIT 1" | /root/sq.sh'
-> ["integer",1786785861]
```

12 of the 26 time-valued columns across the 48 listed objects carry no epoch note; 14 do.

| column | declared | evidence it is epoch seconds |
| --- | --- | --- |
| `tenders.created_at` | INTEGER | `typeof` = integer, `1789284922` (2026-09-13T07:35:22Z) |
| `tenders.current_published_at` | INTEGER | `typeof` = integer, `1376092800` (2013-08-10) |
| `tenders.current_deadline` | INTEGER | `typeof` = integer, `1378728000` (2013-09-09) |
| `organizations.created_at` | INTEGER | `typeof` = integer, `1786785861` (2026-08-15T09:24:21Z) |
| `tender_version_lot_results.decided_utc` | INTEGER | `typeof` = integer, `1375315200` (2013-08-01) |
| `v_lot_results.decided_utc`, `v_awards.decided_utc` | **TEXT** | raw selections of that same column (`canonical.rs:943, 984`) |
| `notice_withheld_fields.publish_after` | **TEXT** | literally `d.utc_seconds FROM notice_dates` (`store/src/lib.rs:388-391`) — an alias of a column that IS flagged |
| `quarantine.skipped_at`, `quarantine.last_attempt_at` | INTEGER | declared type only (no non-NULL row in the bounded ranges sampled) |
| `tender_version_contracts.decided_utc`, `.concluded_utc` | INTEGER | declared type only (no dated row in the ranges sampled) |

The three promises, all live: `notes[2]` — "each timestamp column's note flags this"; `/docs` (`docs.rs:379`) — "Each timestamp column is flagged in the schema"; the OpenAPI `sqlSchema` description — "per-column notes, timestamp flags". And the schema's own **example #2** formats `tenders.current_published_at` with `strftime('%Y', current_published_at, 'unixepoch')` — a column it does not flag.

Cause: `COLUMN_NOTES` (`crates/app/src/v1/sql.rs:640-649`) hand-lists eight column names — `published_at`, `dispatched_at`, `ingested_at`, `fetched_at`, `changed_at`, `first_seen`, `reprocessed_at`, `utc_seconds`. Everything added later by `ALTER TABLE` (`skipped_at` `lib.rs:809`, `last_attempt_at` `lib.rs:833`, `decided_utc` `lib.rs:657/662`, `current_deadline` `lib.rs:711`) and the original `created_at` / `current_published_at` were never added to the list.

### Why it matters

The sharpest case is the three TEXT-declared, unflagged columns: `v_lot_results.decided_utc`, `v_awards.decided_utc` and `notice_withheld_fields.publish_after` read as ISO strings to anyone typing the schema — TEXT plus no timestamp flag is exactly the signature of a date string, and they are epoch integers. An analyst writing `WHERE decided_utc LIKE '2013%'` against `v_awards` gets zero rows and no error, which is the failure `notes[2]` exists to prevent. The reason this is low rather than medium: `notes[2]` and the `/docs` bullet do tell every reader that *all* time columns are epoch, so the per-column flag is a safety net, and its absence misleads only a reader consulting one column's note in isolation — which is what the schema browser encourages.

### Why this is ours, not the publisher's

Judge: the cause is a hand-written list in our own `sql.rs`, and all 12 columns are epoch by our own DDL (`canonical.rs:113/128/696/788/791`, `lib.rs:205/216`) or by our own view definitions. The TEXT type on the three view columns is just how the endpoint reports view columns — `v_fetches.fetched_at` is TEXT and *is* flagged — so it is not a signal about the data. Issue 50 is RESOLVED-VERIFIED with the clause "every timestamp column carries an epoch-seconds note with a strftime example, plus a top-level schema note and a /docs bullet"; that clause never held for these 12, so this is a gap in a resolved issue's verification rather than a regression. Issue 370 already names `COLUMN_NOTES` as un-derived prose (its unit 3), but its 13-row table does not carry this claim.

### Repro

Under a minute, no token, no box access:

1. Run the `curl | python3` filter above → the 12 `(table, column, type)` triples.
2. `curl -s https://tenders.zebreus.click/v1/sql/schema | jq -r '.notes[2], (.examples[1].sql)'` → the completeness promise, and the example that formats the unflagged `current_published_at` as epoch.

### Done when

- The flag is **derived, not listed**: `COLUMN_NOTES` attaches `EPOCH_NOTE` by name pattern (`*_at`, `*_utc`, `*deadline*`, `publish_after`) or by the column's declared INTEGER type on a time-named column, so a column added by a later `ALTER TABLE` is flagged the day it appears.
- The filter in step 1 of the repro returns `[]` against the served schema.
- A test walks the served schema and asserts every time-named column carries `EPOCH_NOTE` — red-demonstrated by removing one pattern.
- The three view/alias columns (`v_awards.decided_utc`, `v_lot_results.decided_utc`, `notice_withheld_fields.publish_after`) are flagged, since TEXT + unflagged is the reading that actively misleads.
- Issue 50's clause is corrected, or re-verified against the derived flag.

---

## Unit 4 — the `/docs` notice-content example shows a shape no parser can emit, and its notice id is a 404

### Observed (verified 2026-09-14 on prod, live rev `9e082fd`)

The example, `crates/app/src/v1/docs.rs:269-271`, rendered on the live `/docs`:

```
"notice_id": 14327,
"section_id": 1, "kind": "root", "parent_section_id": null,
{"type": "text", "field_id": "BT-21", "ordinal": 0, "lang": "deu", "value": "…"}
```

The served envelope:

```
curl -sS https://tenders.zebreus.click/v1/notices/1/content | python3 -c "import json,sys; d=json.load(sys.stdin); print([{k:v for k,v in s.items() if k!='values'} for s in d['sections'] if s['parent_section_id'] is None]); print([v for s in d['sections'] for v in s['values'] if v['type']=='text'][0])"
-> [{'kind': 'Notice', 'parent_section_id': None, 'section_id': 'PROCEDURE'}]
-> {'field_id': 'BT-133-Lot', 'lang': 'SPA', 'ordinal': 0, 'type': 'text', 'value': 'SALON PLENOS DIPUTACIÓN'}
```

| field | `/docs` example | served (notice 1) | OpenAPI `NoticeContent.sections.items` |
| --- | --- | --- | --- |
| `section_id` | `1` (integer) | `"PROCEDURE"` (string); children `"LOT-0001"`, `"ORG-0002"` | `type: string`, "PROCEDURE for the root" |
| `kind` (root) | `"root"` | `"Notice"` | "(Lot, Organisation, LotResult, …)" |
| `lang` | `"deu"` | `"SPA"` | no case or length constraint |
| `notice_id` | `14327` | 404 — `{"error":{"message":"no such notice","status":404}}` | — |

Notice 1 serves 119 sections across 21 distinct kinds. The shape is structural, not per-era: notice 20000000 (`ted-export-r209`, 7 sections, kinds {Lot, Notice, Organization}), notice 996545 (text era, 2 sections), notice 31769934 (`fts:ocds-1.1`, 3 sections) all root at `PROCEDURE` / `Notice` with string ids. `store::Section` is `{id: String, kind: String, parent: Option<String>}` (`crates/store/src/lib.rs:3691`), `notice_sections.section_id` is TEXT, `json.rs:184-186` emits them verbatim, and **every** parser mints the root as `PROCEDURE` / `Notice` (`ingest/src/eforms/parse.rs:90`, `r209/parse.rs:115`, `text/parse.rs:1512`, `fts/parse.rs:75`). The string `"root"` is not a section kind anywhere in `crates/ingest` or `crates/store`.

One correction to carry into triage: **the `lang` fix is not "uppercase ISO 639-2/T".** `/content` serves the parse-layer `notice_texts.lang` verbatim from the source, and issue 292's `normalize_lang` applies at the fold boundary to `tender_version_texts`, not here. The four observed dialects are eForms → 3-letter uppercase (`SPA`), r209 → 2-letter uppercase (`BG`…`SV`), text era → `EN`/null, FTS → 2-letter lowercase (`en`). `"deu"` (lowercase 3-letter) matches none of them, so the example is still misleading, but the correct text is "the source's tag verbatim, e.g. `SPA` / `DE` / `en`".

### Why it matters

A reader who keys on `kind == "root"` to find the top of the tree, or who types `section_id` as an integer, builds a client that never matches anything — and the failure is a silent empty result, not a parse error, because the real values are perfectly valid JSON of a different shape. The machine spec is correct, so the harm falls on the reader of the human page, which is the page `/docs` exists to be. The dead `notice_id: 14327` compounds it: the one obvious way to check the example against reality answers 404.

### Why this is ours, not the publisher's

Judge: this is a self-inflicted docs-vs-reality contradiction with no publisher shape in it — our parsers choose `PROCEDURE`/`Notice`, our serializer emits them, and our own OpenAPI describes them correctly. Issue 227 authored this example block (RESOLVED, `a3139e3`, the "Notice content" section) and shipped the guard in the same commit, but that guard asserts only that parameter and path names occur in `docs.rs` — nothing pins an example block, so the block was never anchored to anything. A board grep for the example, for `"root"` as a kind, for `deu` or for `section_id: 1` finds nothing: it has stood unexamined since the day it was written.

### Repro

Under a minute:

1. Open `/docs` and find the "Notice content" example → `"section_id": 1`, `"kind": "root"`, `"lang": "deu"`, `"notice_id": 14327`.
2. `curl -sS https://tenders.zebreus.click/v1/notices/1/content | jq '[.sections[] | select(.parent_section_id==null) | {section_id, kind}]'` → `[{"section_id":"PROCEDURE","kind":"Notice"}]`.
3. `curl -sS -o /dev/null -w '%{http_code}\n' https://tenders.zebreus.click/v1/notices/14327/content` → `404`.

### Done when

- The `/docs` example block is replaced by a fragment of a real served response — string `section_id`, root `kind: "Notice"`, a `lang` value that some era actually emits — and its `notice_id` resolves.
- The example says in one line that `lang` is the source's tag verbatim and varies by era (`SPA` / `DE` / `EN` / `en`), linking issue 292's three dialects, rather than implying one vocabulary.
- A test deserializes the `/docs` example block against the same types the endpoint serializes (or, via unit 1's guard, fetches the example's own `notice_id`), so an example that cannot be served fails the gate.
- Nothing in `docs.rs` claims a section kind that no parser mints: assert the example's kinds are a subset of the kinds the parsers emit.

---

## Unit 5 — the OpenAPI row schemas declare 2–6 fields for rows that serve 7–16, and `/docs` leans on a field the spec never declares

### Observed (verified 2026-09-14 on prod)

```
curl -sS "https://tenders.zebreus.click/v1/organizations?limit=1"
-> keys: country, id, identifier, identifier_kind, mentions, name, provisional          (7)
curl -sS "https://tenders.zebreus.click/v1/tenders?limit=1"
-> keys: country, cpv, dispatched_at, id, kind, lots, notice_subtype, original_lang,
         procedure_key, publication_id, published_at, source, submission_deadline,
         title, value, version                                                          (16)
curl -sS "https://tenders.zebreus.click/v1/lots?limit=1"
-> keys: id, kind, lot_key, submission_deadline, tender_id, title, value, version       (8)
curl -sS "https://tenders.zebreus.click/v1/notices?limit=1"
-> keys: content_hash, declared_version, dispatched_at, id, ingested_at, member_path,
         parse_state, profile, publication_id, published_at, source                     (11)
```

| schema | declared properties | served keys | undocumented |
| --- | --- | --- | --- |
| `Tender` | `id, source, title, cpv, country, published_at` (6) | 16 | 10 |
| `Lot` | `id, tender_id` (2) | 8 | 6 |
| `Organization` | `id, name` (2) | 7 | 5 |
| `Notice` | `id, source` (2) | 11 | 9 |

The key sets are fixed by `TenderRow` / `LotRow` / `OrganizationRow` / `NoticeRow` (`crates/store/src/read.rs`) and the serializers at `crates/app/src/v1/json.rs:58-121` — a second row per collection fetched through a different filter (`organizations?country=DE` id 26, `tenders?sort=deadline` id 8210860, `lots?country=FR` id 110, `notices?source=ted` id 1) has identical keys — so this is every row, not a sampling artefact.

The spec contains the string `provisional` **zero** times (also zero for `identifier_kind`, `mentions`, `parse_state`, `submission_deadline`, `content_hash`, `declared_version`, `member_path`), while `/docs` "Codes and identities" (`docs.rs:644`) instructs:

> "The `provisional` flag on `/v1/organizations` tells you which kind you are looking at"

and `docs.rs:656` offers `/v1/openapi.json` "for client generators and API tooling".

The eight `TenderDetail` satellites (`lot_details`, `texts`, `amounts`, `dates`, `classifications`, `lot_results`, `bids`, `contracts`) are declared as bare `{type: object, additionalProperties: true}` with no properties, while `/v1/tenders/2` serves `texts {field, lang, lot, value}`, `dates {field, lot, value}`, `classifications {code, field, lot, scheme}`, `parties {lot, organization_id, organization_name, role}` and `versions` with 7 keys. (The four award satellites were empty on every id reached, so their shapes are unobserved here.)

Also in scope, same class: `curl -sS https://tenders.zebreus.click/v1` lists **13** endpoints against 23 spec paths, omitting `/v1/notices/{id}/content` (a data endpoint, landed by 218-B) plus the three webhook sub-routes. The list is hard-coded at `crates/app/src/v1/mod.rs:1372-1377` and the guard at `api.rs:2150` asserts only `endpoints ⊆ spec paths`, never the reverse.

Two sub-points from the reviewer are **dropped** after reproduction, and should not be re-raised in triage:

- **`identifier`/`kind` on `/v1/organizations`** — the `/docs` lookup instruction is about *query parameters*, which the spec does declare (`components.parameters.identifier`, `.kind`). `/docs` never names the response field `identifier_kind` (0 occurrences), so `provisional` is the only genuine docs-relies-on-an-undeclared-field case.
- **`limit=1001` clamping to 1000 with a 200** — issue 215-A chose clamping deliberately ("accepted or clamped to the documented max"). The docs half of that (clamp-vs-reject is written down nowhere) is issue 390 unit 4.

### Why it matters

The spec is offered by name to client generators, and a generated client sees none of `value`, `submission_deadline`, `kind`, `version`, `provisional`, `identifier`, `identifier_kind`, `mentions`, `parse_state` or `profile` — most of what the collections are useful for. `additionalProperties: true` on all four schemas means such a client *parses* rather than breaks, which is why this is low, but it loses typing on every field that matters, and the detail's satellites are untyped `object` so there is nothing to generate at all. The concrete bite: `/docs` tells a reader to branch on `provisional`, our own CONTEXT.md names it a core Organization concept, and the machine contract a code generator reads does not know it exists. Only `Tender` carries the "Fields beyond these evolve with the backfill; unknown fields must be tolerated" caveat — `Lot`, `Organization` and `Notice` carry no such note, so their silence reads as completeness.

### Why this is ours, not the publisher's

Judge: the served fields are fixed serializer output, not backfill-evolving shapes — `json.rs:58-121` names every one of them in code — so the "evolves with the backfill" caveat does not describe what is missing. The prose page this system publishes depends on a field this system's own machine contract omits, which is a self-introduced inconsistency with no publisher fact in it. The board does not cover it: 215 (RESOLVED) fixed the `limit` ceiling, `include_data`, `changes.more` and a docs label; 227's guard walks `components.parameters` and `paths`; `the_openapi_spec_matches_the_served_surface` checks paths against the router. **No guard has ever compared a component schema's properties with a served row's keys**, and a board grep for `additionalProperties`, client generators, row schemas or codegen finds no issue and no by-design note.

### Repro

Under two minutes, no token:

1. `curl -sS 'https://tenders.zebreus.click/v1/organizations?limit=1' | jq -r '.items[0] | keys | join(", ")'` → 7 keys.
2. `curl -sS https://tenders.zebreus.click/v1/openapi.json | jq -r '.components.schemas.Organization.properties | keys | join(", ")'` → `id, name`.
3. `curl -sS https://tenders.zebreus.click/v1/openapi.json | grep -c provisional` → `0`, against the `/docs` sentence that tells you to use it.
4. `curl -sS https://tenders.zebreus.click/v1 | jq -r '.endpoints | length'` → `13`, with `/v1/notices/{id}/content` absent.

### Done when

- `components.schemas.Tender`, `Lot`, `Organization` and `Notice` declare every property the serializer emits, with types — at minimum every field `/docs` names (`provisional`, `identifier`, `value`, `submission_deadline`, `parse_state`, `profile`, `version`).
- The eight `TenderDetail` satellite item schemas carry the properties they actually serve (`texts {field, lang, lot, value}`, `dates {field, lot, value}`, `classifications {code, field, lot, scheme}`, `parties {lot, organization_id, organization_name, role}`, and the four award satellites read off `json.rs` rather than off a sample, since they were empty in this probe).
- `Lot`, `Organization` and `Notice` either become complete or carry `Tender`'s explicit "unknown fields must be tolerated" caveat — silence stops reading as completeness.
- A drift test, mirroring 227's guard one level down: serialize one row per collection, assert every key is a declared property of that schema, and assert every declared property appears on the row. This is the guard that makes units 4 and 5 unable to recur.
- `/v1`'s `endpoints` list includes `/v1/notices/{id}/content`, and the guard at `api.rs:2150` checks both directions for data endpoints (a deliberate exclusion for the webhook sub-routes is fine if it is written down where the list is built).

## Units 1, 2 and 3 BUILT 2026-09-16 (owner)

### Unit 1 — the parser learns `now`, rather than the spec forgetting it

The `## Done when` offered both directions. **Accept it.** A moving instant is exactly what a
"closes soon" query needs, and striking it would make every caller compute a timestamp to ask the one
question the endpoint exists for — a literal that goes stale the moment they save it. `parse_instant`
now maps a case-insensitive `now` to `store::now_unix()`, so all four bounds
(`published_after/_before`, `deadline_after/_before`) take it from one parser.

Nothing else is a word, deliberately: `today`, `tomorrow` and friends stay 400, because each needs a
timezone and this API has no notion of the caller's. The error message now names `now` so a caller
who tried `today` is told what does work.

`openapi.json` names `now` on all four instant parameters (two phrasings, `(unix seconds or RFC
3339)` and the `(exclusive; …)` variant), and `/docs` says it beside the RFC-3339 sentence.

### Unit 2 — the headline note stops teaching the trap

It prescribed `strftime(col,'unixepoch')`, the reversed argument order that the same document's 14
column notes, its own example and `/docs` all name as the silent-NULL trap — SQLite's signature is
`strftime(FORMAT, timevalue, …)`, so the column is read as a format string and every row comes back
NULL without an error (issue 239 measured one NULL bucket holding all 7,924,659 Tenders). Issue 239's
fix rewrote `EPOCH_NOTE` and the examples and never touched this string.

**The notes are now a named `SCHEMA_NOTES` const rather than literals inside a `json!`,** which is
the point of the unit and not incidental: they were unreachable by any test, which is how the
contradiction survived. Two of the notes interpolate runtime limits, so those moved into
`schema_limit_notes(timeout_secs)` and are appended at serve time — the static ones are testable and
the dynamic ones stay honest.

### Unit 3 — derived, and one real exception found by checking

The claim "each timestamp column's note flags this" was 14 of 26 true. `column_note` now falls back
to `EPOCH_NOTE` for any time-shaped column name, so the promise holds by construction; the eight
hand-listed `("*", …)` rows are gone.

**By NAME, not by declared type**, and the reason is measured: SQLite carries no reliable type for a
VIEW's columns. `PRAGMA table_info` reports `TEXT` for `v_tenders.published_at`,
`v_fetches.fetched_at` and `v_tender_dates.utc_seconds`, all of which serve epoch integers (prod
values 1700611200, 1784489994, 1703232000). A type-driven rule would have un-flagged exactly the
friendly views a `/v1/sql` caller reaches for first.

The predicate was checked against the live schema before it was written: over the 344 queryable
columns it selects **26 and no others** — every time column, and neither `projection_epoch` (a
version counter) nor the `*_has_time` booleans.

**And one column is genuinely not epoch seconds.** `currency_rates.rate_date` is `TEXT` holding
`1993-01-04`. Blanket-flagging it would have been precisely the false claim this issue is about, so
it keeps an explicit note — which wins over the derivation — saying it is an ISO string, that `LIKE
'2012%'` works there and nowhere else, and not to wrap it in `strftime(…, 'unixepoch')`. The twelve
that were missing were `created_at`, `skipped_at`, `last_attempt_at`, `decided_utc` (×3),
`concluded_utc`, `current_published_at`, `current_deadline`, `rate_date` and `organizations.created_at`
— the newer ones, because a note has to be remembered and a derivation does not.

Tests: `the_time_bounds_accept_the_literal_now` (four bounds, case, the flagship query whole, five
refused words, and the two forms that already worked), `the_schema_headline_note_teaches_format_first_like_every_other_surface`,
and `every_timestamp_column_carries_the_epoch_note_and_rate_date_says_otherwise` (all 15 name shapes
from the live schema, the `rate_date` exception, the four look-temporal columns that must NOT match,
and an explicit note still winning).

### Still open

- **Units 4 and 5** are not built: the `/docs` notice-content example showing a shape nothing emits
  (`"section_id": 1`, `"kind": "root"`, notice 14327 → 404), and the component schemas declaring
  2–6 properties where 7–16 are served.
- **The example-runner guard** the issue asks for under unit 1 — extract every `name=value` from
  spec and docs, fire each at the test server, assert no 4xx — is NOT built. It is the one guard that
  would also catch unit 4, and it is worth doing properly rather than as a tail of this commit.
- Not deployed: the issue-397 text-era projection (job 1387) is running.

## The example-runner guard is BUILT 2026-09-16 (owner)

`every_published_example_is_a_request_the_server_accepts` — the guard the issue asks for under unit
1, and the one neither existing gate is. `the_openapi_spec_matches_the_served_surface` compares spec
paths to the router; `the_docs_page_names_the_whole_spec_surface` checks that every parameter NAME
occurs somewhere in `docs.rs`. Neither reads a VALUE, which is how `deadline_after=now` spent its
whole life answering 400 while both stayed green.

Two halves, because the surfaces publish examples in different shapes:

| half | what it extracts | count today |
| --- | --- | --- |
| `/docs` | whole `/v1/…?…` URLs, `&amp;` un-escaped | **15** |
| `openapi.json` | bare `name=value` in a parameter description, where `name` is a declared parameter | **5** |

All 20 are fired at the test server and none may answer 4xx. The `/docs` half covers
`?country=DE&status=open&limit=50&cursor=14327`, `?sort=deadline&status=open&country=DE&deadline_after=1786910000`,
`?identifier=RO42283735&kind=VAT`, `?publication_id=123456-2026`, `?name_prefix=m`, `?bidder=2` and
nine more; the spec half covers `deadline_after=now`, `order=asc`, `sort=published_at|deadline|id`.

**The spec half needed a correction mid-build.** The first version took a pair only from the
description of the parameter it names — the strict reading, so prose about a neighbour could not
invent an assertion. That found exactly ONE pair: the useful examples mostly live in a *different*
parameter's text (`sort=published_at` is published inside `deadline_after`'s "Implies
sort=published_at"). A published example is a claim about what the server accepts wherever it is
written, so the rule became "the KEY must be a declared parameter name" — which still keeps prose
from inventing parameters, and covers all five.

**It is not vacuous, and the test says so itself**: it asserts `deadline_after=now` is among the
pairs it fires and that the `/docs` extractor still finds at least 8 URLs. Both fail loudly if an
extractor silently stops seeing its page — the failure mode that makes a green example-runner worse
than no runner at all.

### What it deliberately does NOT cover

Stated so nobody reads more into a green run than is there: `{id}` templates (no concrete value),
token-gated paths (`/v1/me`, `/v1/sql`, `/v1/webhooks` — a 401 there is correct), and examples naming
a specific PROD id such as `/v1/tenders/14327`, which a fixture server cannot resolve.

**That last exclusion means this guard does NOT cover unit 4**, contrary to what the issue's unit-1
bullet hoped. Unit 4's defect is that `/docs` shows notice **14327**, whose `/content` is a 404 on
prod, alongside a response body (`"section_id": 1`, `"kind": "root"`, `"lang": "deu"`) that no parser
emits — the served shapes are `"PROCEDURE"`, `"Notice"`, `"SPA"`. Neither half is checkable against a
fixture: the id is prod-specific and the body is illustrative. Unit 4 wants the example rewritten to
a shape the fixture CAN produce, which is an edit, not a gate. Recorded here rather than left for the
next reader to rediscover.

### Still open on this issue

- **Unit 4** — rewrite the notice-content example to a shape that is true, per above.
- **Unit 5** — `components.schemas.{Tender,Lot,Organization,Notice}` declare 6/2/2/2 properties
  against 16/8/7/11 served keys. The derivable form the issue suggests — serialize one row per
  collection and diff its keys against the declared properties — is a natural second test beside this
  one, and is the right way to do it.

## Units 4 and 5 BUILT 2026-09-16 (owner) — issue 391 is now complete

### Unit 4 — the example shows shapes the parser emits

Every field in the old block was wrong, checked against a live
`/v1/notices/{id}/content`:

| field | published | actually served |
| --- | --- | --- |
| `section_id` | `1` | `"PROCEDURE"`, `"LOT-0001"` — the SOURCE's own ids, strings |
| `kind` | `"root"` | `"Notice"`, `"Lot"` — the parser's section vocabulary |
| `lang` | `"deu"` | `"DEU"` — uppercase ISO 639-2/T for eForms |
| `parent_section_id` | only `null` shown | `"PROCEDURE"` on a nested section |
| `notice_id` | `14327` | **404** |

The example now shows two sections (a root `PROCEDURE` and a child `LOT-0001`) so the parent link is
visible at all, and real value shapes — a `text` with `"lang": "DEU"`, an `integer`, and an `id`
carrying `scheme`/`is_ref`.

**The stale id is fixed by explaining it, not by swapping in a fresh one.** Entity ids are scoped to
the feed's `generation` and are REISSUED by a rebuild — which is exactly why 14327 died — so any
literal in a doc page is a future 404. The caption now says so and tells the reader to take an id
from `/v1/notices`. Swapping in today's id would have reset the same clock.

### Unit 5 — the schemas declare what they serve, and a gate keeps it that way

| schema | declared before | served | properties added |
| --- | --- | --- | --- |
| `Tender` | 6 | 16 | `kind`, `version`, `publication_id`, `procedure_key`, `notice_subtype`, `original_lang`, `dispatched_at`, `submission_deadline`, `lots`, `value` |
| `Lot` | 2 | 8 | `kind`, `lot_key`, `title`, `version`, `submission_deadline`, `value` |
| `Organization` | 2 | 7 | `country`, `identifier`, `identifier_kind`, `mentions`, `provisional` |
| `Notice` | 2 | 11 | `publication_id`, `published_at`, `dispatched_at`, `ingested_at`, `profile`, `declared_version`, `parse_state`, `content_hash`, `member_path` |

30 properties, each typed from a LIVE row rather than guessed — `value` is `Money`-or-null, `lots`
and `mentions` are integers, `provisional` is a boolean, the instants carry `format: date-time`.
`Notice.parse_state` states in bold that it, and not the presence of `quarantine`, says whether a
notice is held today (issue 398's contract, now in the machine-readable half too).

`every_served_key_is_declared_in_its_schema` serializes one real row per collection and diffs its
keys against the declaration, so a field added to a serializer fails here until the spec catches up.
It guards against passing vacuously in both directions: an empty page fails, and a row with fewer
than 7 keys fails as "the fixture has thinned out and this gate no longer covers the shape".

### Issue 391 is complete

All five units built, plus the example-runner guard. Two of the five are now DERIVED rather than
restated — unit 3's epoch note and unit 5's property set — which was the fix shape the issue argued
for. Not deployed: the issue-397 text-era projection (job 1387) is still folding.

Acceptance reads for after the deploy: `/v1/tenders?deadline_after=now&sort=deadline&order=asc&limit=3`
→ 200; `/v1/sql/schema` `notes[2]` containing "FORMAT FIRST" and no longer
"Filter/format with strftime(col,'unixepoch')"; `currency_rates.rate_date`'s note naming ISO;
`/v1/openapi.json` `components.schemas.Tender.properties` holding 16 keys.
