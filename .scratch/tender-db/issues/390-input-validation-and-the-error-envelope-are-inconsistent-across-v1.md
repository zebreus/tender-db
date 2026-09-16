# 390 — `/v1`: input validation and the error envelope are inconsistent across the surface — a filter value is a LIKE pattern, `limit` is silently clamped, and three routes sit outside the contracts `/docs` publishes

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (app — the `/v1` serving layer in `crates/app/src/v1/mod.rs`, plus one predicate in `crates/store/src/read.rs`; units 2, 4 and 5 are also docs defects, in `/docs` and `openapi.json`)
Relates to: 117 (RESOLVED-VERIFIED — it analysed `?country=_E`, `%`, `d%` and the empty string, but only far enough to make the reachability guard DECLINE on them; its own comment says "Nothing upstream validates these ... the answer is to decline the guard rather than to interpret the pattern", and unit 1 is the input path it deferred), 118 (RESOLVED — introduced `ignored_filters`; unit 1 is the case that array cannot describe, because it names parameter NAMES, not values), 336 (CLOSED NOT-WORTH-IT — the opposite direction, an unmatchable value returning an EMPTY page; its closing argument "an empty list for a filter that matches nothing is conventional" does not extend to a value returning EVERYTHING), 284 (RESOLVED — the same org-search path silently dropping filters while reporting them honoured), 387 (filed by this fan-out — `name_prefix`/`kind` on `/v1/organizations`; unit 1 is the same class one parameter over), 215 (RESOLVED — the OpenAPI drift cluster; 215-A raised the `limit` ceiling in the spec from 500 to 1000 but never said the value is clamped, and never touched the floor), 51 (RESOLVED-VERIFIED — the uniform JSON error envelope; unit 3 is the status its three tests never exercised), 218-B (RESOLVED — landed `/v1/notices/{id}/content`, rev `4c3c367`, one day after the CORS grant; it never mentions CORS), 227 (RESOLVED — the `/docs`-vs-spec guard; it walks parameter NAMES, so it cannot catch units 4 or 5), 49 (RESOLVED-VERIFIED — made `?tender=` honoured on `/v1/notices`), 220 (RESOLVED — the performance of that same branch), 370 (served claims with no gate coupling them to behaviour — units 2, 4 and 5 are all that class)

## What ties these five together

All five sit on the boundary between what a client sends and what the served contract says will happen to it, and none is a data defect: no publisher fact is involved and no stored row is wrong. In each one the `/v1` layer either accepts something `openapi.json` declares invalid, or answers in a shape `/docs` says it will never answer in. They cluster in two functions and one router: `Params::filter()` and `Params::limit()` (`crates/app/src/v1/mod.rs`), `is_public_surface()` / `public_cors()` in the same file, and the `/v1` router's missing method fallback.

They are one issue because they share a cause worth naming once: **`Params` shape-checks some of its parameters and not others, and the router has a fallback for unmatched paths but not for unmatched methods.** `currency`, `lang`, `status` and `name_prefix` are validated and 400 on junk; `country` and `cpv` are `.clone()`d straight through into a SQL `LIKE` pattern, and `limit` is `clamp`ed rather than rejected. Fixing them together is one review of those three places.

| unit | surface | a client sends | it gets | the contract it breaks |
| --- | --- | --- | --- | --- |
| 1 | `/v1/tenders`, `/v1/lots`, `/v1/changes` | `country=_E`, `cpv=%`, `country=` | 200, wrong or unfiltered rows, `ignored_filters []` | `country` is documented as "a NUTS place-code prefix", `cpv` as "CPV code prefix, e.g. 45" |
| 2 | `/v1/notices/{id}/content` | `Origin:` + a preflight | 200 with no ACAO; OPTIONS → bare 405 | "Every endpoint that needs no token is CORS-open to any origin" |
| 3 | every routed `/v1` path | `POST`/`PUT`/`DELETE` on a GET route | 405, `content-length: 0`, no body | "Errors share one shape ... with the matching HTTP status" |
| 4 | the 5 endpoints sharing `limit` | `limit=0`, `limit=-5`, `limit=1001` | 200 with the value rewritten to 1 or 1000 | spec declares `"minimum": 1, "maximum": 1000`; same endpoint 400s `limit=abc` |
| 5 | `/v1/notices?tender=<id>` | `Accept: text/event-stream` | 200 `application/json` | "Send Accept: text/event-stream to any collection endpoint (with any filters)" |

Severity: unit 1 medium (a documented public filter silently serves wrong rows as filtered), unit 2 medium (total failure of the documented browser-client use case on one route), units 3–5 low (contract/consistency gaps with no wrong data and no leakage).

---

## Unit 1 — `cpv`/`country` are bound verbatim into `LIKE`, so a user value carries wildcard semantics

**This is not SQL injection, and should not be triaged as one.** The value is a bound parameter: `read.rs` binds `c.code LIKE ?` with `t(format!("{country}%"))`, so nothing is ever interpolated into SQL text and no statement structure is reachable from the input. The defect is narrower and quieter: SQLite's `LIKE` treats `%` and `_` **inside the bound value** as wildcards, the predicate carries no `ESCAPE` clause, and `format!("{country}%")` of an empty string is the pattern `%`. So a user value silently changes the filter's semantics — `_` becomes "any one character", `%` becomes "anything", and an empty value matches every row — and every one of those cases is answered **200 with `ignored_filters: []`**, which tells the client the filter was honoured.

### Observed (verified 2026-09-14 on prod)

```
curl -s 'https://tenders.zebreus.click/v1/tenders?cpv=%25&limit=2'
curl -s 'https://tenders.zebreus.click/v1/tenders?country=_E&limit=2'
curl -s 'https://tenders.zebreus.click/v1/tenders?country=&limit=2'
```

| request | status | items | `ignored_filters` | reading |
| --- | --- | --- | --- | --- |
| `?cpv=%25&limit=2` | 200 | id 2 (cpv 72000000…), id 3 (cpv 71221000) | `[]` | unfiltered: two unrelated CPV divisions |
| `?country=_E&limit=2` | 200 | id 2 `["DE91C"]`, id 16 `["DE714"]` | `[]` | `_` matched the `D` of `DE` |
| `?country=&limit=2` | 200 | id 2 `["DE91C"]`, id 4 `["PL622"]` | `[]` | two different countries: the filter is a no-op |

Controls, same firing: `country=DE` → DE rows, `country=ZZ` → 0 rows (literals work), `country=_L` → PL622/PL912, `country=D_9` → DE91C/DE911, `country=%5F` → id 2, `cpv=_2` → 72xxx codes, `cpv=` → the unfiltered page, and `/v1/lots?country=_E` → lots 2 and 118 (tenders 2 and 16, both DE). Every one is 200 with `ignored_filters []`.

The sibling parameter on the same surface is strict:

```
curl -s 'https://tenders.zebreus.click/v1/organizations?name_prefix=&limit=1'
→ 400 {"error":{"message":"name_prefix must not be empty; pass at least one character","status":400}}
```

Source: `crates/store/src/read.rs:891-905` `version_predicates()` binds `c.code LIKE ?` with `t(format!("{country}%"))` / `t(format!("{cpv}%"))`, no `ESCAPE`; `prefix_ranges()` at `read.rs:1350-1359` states in its own comment "Nothing upstream validates these — Params passes country and cpv through verbatim" and declines only the range guard; `crates/app/src/v1/mod.rs:595-596` clones both values into `Filter` with no shape check, while `mod.rs:655-663` rejects an empty `name_prefix` with 400 ("An empty prefix would be an unbounded ... dump — refuse it").

### Why it matters

A consumer filtering by `country=` (an empty form field, a template that interpolated a missing variable) is served **the whole collection as a filtered page** and told every filter applied. A consumer whose NUTS or CPV value happens to contain `_` — not exotic, it is one keystroke from a hyphen and appears in hand-typed codes — gets rows from other countries presented as matches. Both failures are silent in exactly the way `ignored_filters` was built (issue 118) to prevent, and both are unauthenticated and deterministic. `/v1/lots` shares the path by direct test and `/v1/changes` SSE snapshots share the same `Filter` struct (`mod.rs:595-596`), so a subscription can be built on a filter that means something other than what it says.

One honest correction from the judge, worth carrying into triage so the fix aims at the right thing: **`ignored_filters: []` is per contract here and is not itself the defect.** Per issues 118 and 336 that array names parameter NAMES the collection does not honour, and `country` *was* applied — as a pattern. The defect is the pattern semantics and the missing shape check; the array is only what makes it invisible.

### Why this is ours, not the publisher's

This is a read-layer pattern predicate built over verbatim client input — behaviour tender-db introduced in its own serving layer, with no publisher fact anywhere in it. It is also internally inconsistent with the API's own validation posture two lines away: `currency`, `lang`, `status` and `name_prefix` are shape-checked and 400 on junk. Issue 117 saw these exact inputs and deliberately left them: it made the reachability guard DECLINE on `%`/`_E`/`""` so the guard can never be narrower than the `LIKE`, and `tenders_shortcircuit.rs` pins wildcards-match-everything as a **guard-consistency invariant**, not as a product decision that wildcards are a supported input. Issue 336 closed the opposite direction (an unmatchable value returning an empty page) on the argument that "an empty list for a filter that matches nothing is conventional" — which says nothing about a value returning everything.

### Repro

≤2 minutes, no token, no SQL:

1. `curl -s 'https://tenders.zebreus.click/v1/tenders?country=_E&limit=2'` → ids 2 (DE91C) and 16 (DE714).
2. `curl -s 'https://tenders.zebreus.click/v1/tenders?country=&limit=2'` → ids 2 (DE91C) and 4 (PL622), i.e. two countries.
3. `curl -s 'https://tenders.zebreus.click/v1/tenders?country=ZZ&limit=2'` → 0 items, proving the filter functions for literals.
4. `curl -s 'https://tenders.zebreus.click/v1/organizations?name_prefix=&limit=1'` → 400, the posture the fix should match.

### Done when

- `Params::filter()` shape-checks `country` and `cpv` the way `currency`/`lang` are checked — non-empty, ASCII alphanumeric (NUTS is letters+alnum, CPV is digits) — and anything else answers 400 in the standard envelope.
- And/or the predicate stops being a user-controlled pattern: `LIKE ? ESCAPE '\'` with `%`/`_` escaped in the bound value, or the range form.
- `GET /v1/tenders?country=_E` no longer returns DE rows: either 400, or 200 with 0 items.
- `GET /v1/tenders?country=` and `?cpv=` answer 400 rather than an unfiltered page.
- Controls unchanged: `country=DE` → DE rows, `country=ZZ` → 200 with 0 items, `cpv=45` → 45xxxxxx rows.
- `/v1/lots` and a `/v1/changes` SSE snapshot each get one test, since both share the `Filter`.
- `tenders_shortcircuit.rs`'s `%` / `_E` / `""` assertions are retired or re-pointed: they currently pin the old semantics as an invariant, so the fix fails the suite until they move.

---

## Unit 2 — `/v1/notices/{id}/content` is token-free but not CORS-open

### Observed (verified 2026-09-14 on prod, live rev `9e082fd`)

```
curl -sS -I 'https://tenders.zebreus.click/v1/notices/1/content' -H 'Origin: https://example.org'
curl -sS -X OPTIONS -D - -o /dev/null 'https://tenders.zebreus.click/v1/notices/1/content' \
     -H 'Origin: https://example.org' -H 'Access-Control-Request-Method: GET'
```

| request | status | headers |
| --- | --- | --- |
| `HEAD /v1/notices/1/content` | 200 | `content-type: application/json`, `content-length: 63002`, **no `access-control-allow-origin`** |
| `OPTIONS /v1/notices/1/content` | 405 | `allow: GET,HEAD`, `content-length: 0`, no CORS headers |
| `OPTIONS /v1/tenders` (control) | 204 | `access-control-allow-origin: *`, `access-control-allow-methods: GET, OPTIONS`, `access-control-allow-headers: Accept, Content-Type, Last-Event-ID` |
| `HEAD /v1/notices/30000000/content` | 404 | no ACAO |
| `HEAD /v1/notices/30000000` (control) | 404 | `access-control-allow-origin: *`, `access-control-expose-headers: Retry-After` |
| `OPTIONS /v1/notices/30000000/content` | 405 | vs 204 + full preflight set on `/v1/notices/30000000` |

So the gap is route-level and id-independent. Source: `crates/app/src/v1/mod.rs:225-227`, `is_public_surface()`'s id-detail arm grants the form only when `rest` after `/v1/notices/` contains no `/`, which excludes the `/content` sub-resource; `public_cors()` (`mod.rs:240`) then neither answers the preflight (it falls through to axum's 405) nor stamps ACAO on the GET. `notice_content()` (`mod.rs:1238`) takes no `AuthUser` and the OpenAPI path carries no `security`, so the route genuinely needs no token. The route list at `mod.rs:151-199` shows `/v1/notices/{id}/content` is the **only** v1 route with a segment after `{id}`, so it is the sole endpoint in the gap.

### Why it matters

Both documentation surfaces promise the opposite. `/docs` (`docs.rs:118`): "every endpoint that needs no token is callable from browser JavaScript on any origin (`Access-Control-Allow-Origin: *`) ... build a client-side app directly against the API". The live OpenAPI `info.description`: "Every endpoint that needs no token is CORS-open to any origin". A browser client that follows that advice cannot read notice content at all — the preflight fails with 405 and the GET response carries no ACAO, so the fetch is blocked twice over. Issue 218-B built this endpoint precisely so the parsed payload would be reachable without `/v1/sql`; for the browser audience the docs invite, it is not.

### Why this is ours, not the publisher's

Pure serving-layer surface, no publisher fact involved, and the history explains it exactly: the CORS grant landed 2026-08-15 (issue 204 references it), the content route landed 2026-08-16 under issue 218-B (rev `4c3c367`), and the completeness unit test's "deeper paths are not public" case (`/v1/tenders/1/x`) had already codified the one-segment rule that then silently swallowed the new legitimate deeper route. Neither `the_public_surface_is_exactly_the_credential_free_routes` (`mod.rs:274-278`) nor the e2e `the_unauthenticated_surface_is_cors_open` (`tests/api.rs:2217`) lists the content path, so nothing caught it. Issue 218 is RESOLVED and never mentions CORS; issue 227 added the "Notice content" section to `/docs` and thereby deepened the mismatch.

### Repro

1. `curl -sS -I 'https://tenders.zebreus.click/v1/notices/1/content' -H 'Origin: https://example.org'` → 200, no `access-control-allow-origin`.
2. `curl -sS -X OPTIONS -D - -o /dev/null 'https://tenders.zebreus.click/v1/notices/1/content' -H 'Origin: https://example.org' -H 'Access-Control-Request-Method: GET'` → 405.
3. Same two against `/v1/notices/1` → 200 with ACAO `*`, and 204 with the full preflight set.

### Done when

- `is_public_surface()` grants `/v1/notices/<id>/content`, so the GET carries `access-control-allow-origin: *` and OPTIONS answers 204 with the same preflight headers as `/v1/tenders`.
- The 404 path carries the grant too (`/v1/notices/30000000/content` matches `/v1/notices/30000000`'s header set).
- `the_public_surface_is_exactly_the_credential_free_routes` lists the content path, so the grant cannot silently narrow again.
- `the_unauthenticated_surface_is_cors_open` (`tests/api.rs`) exercises the content path alongside the others.
- A rule, not a list: the test asserts every route in `mod.rs:151-199` that takes no `AuthUser` is in the public surface, so the next deeper route is not swallowed the way this one was.

---

## Unit 3 — a 405 on `/v1` is the one error with no envelope and no content-type

### Observed (verified 2026-09-14 on prod)

```
curl -sS -X POST -D - 'https://tenders.zebreus.click/v1/tenders'
curl -sS -X OPTIONS -D - 'https://tenders.zebreus.click/v1/notices/1/content'
```

| request | status | body |
| --- | --- | --- |
| `POST /v1/tenders` | 405 `allow: GET,HEAD` | `content-length: 0`, no content-type, no body |
| `DELETE /v1/tenders/1` | 405 `allow: GET,HEAD` | empty |
| `PUT /v1/notices` | 405 | empty |
| `GET /v1/sql` | 405 `allow: POST` | empty |
| `OPTIONS /v1/sql` | 405 | empty |
| `OPTIONS /v1/notices/1/content` | 405 | empty |
| `GET /v1/nope` (contrast) | 404 | `{"error":{"message":"no such endpoint","status":404}}`, `content-type: application/json` |
| `GET /v1/tenders/abc` (contrast) | 400 | `{"error":{"message":"invalid path parameter: expected an integer id","status":400}}` |
| `POST /v1/sql` no token (contrast) | 401 | `{"error":{"message":"send an API token as Authorization: Bearer tdb_…","status":401}}` (the literal message wraps the header name in backticks) |

6 of 6 wrong-method requests measured came back bodiless. `crates/app/src/v1/mod.rs:150-168` registers each path with `get(...)`/`post(...)` and adds a `/v1/{*rest}` `any(unknown_endpoint)` catch-all whose own comment reads "Static routes above are more specific, so this only catches the genuinely unmatched" — which is the point: the catch-all cannot fire when the static path already matched, and no `Router::method_not_allowed_fallback` is installed (a grep for `method_not_allowed`/`MethodNotAllowed` across `crates/app/src` returns nothing). The `allow: GET,HEAD` spelling (no space) and the bodiless response are axum 0.8.9's `MethodRouter` default, not nginx — an nginx 405 carries an HTML body.

Scope, corrected from the raw finding: `public_cors` (`mod.rs:235-248`) short-circuits OPTIONS with a 204 for paths in `is_public_surface`, so the empty 405 covers **every non-OPTIONS wrong method on every served GET-only path**, plus OPTIONS on served paths outside the public surface (`/v1/notices/{id}/content` per unit 2, `/v1/sql`, `/v1/me`, webhooks).

### Why it matters

A client that parses error bodies unconditionally — the shape both `/docs` (`docs.rs:116`: "Errors share one shape: `{"error":{"status":…,"message":…}}` with the matching HTTP status") and the OpenAPI `Error` schema ("The one error shape, matching the HTTP status") tell it to expect — throws on a zero-length body with no content-type, for this one status only. It is the least dangerous kind of contract break and the most annoying: the failure surfaces in the client's error handler, which is the code path least likely to be tested.

### Why this is ours, not the publisher's

Entirely this system's router: axum's default fallback fires because `/v1` installs a path fallback and not a method one, and nothing on the board or in the code marks a bare 405 as by design. Issue 51 is RESOLVED-VERIFIED with acceptance "every /v1 error is JSON with no internal leakage", but its three verifying tests (`malformed_inputs_return_the_json_error_envelope`, `unknown_v1_paths_are_json_404`, `the_rate_limit_429_is_our_json_envelope`) cover 400/404/429 only, so this is a gap 51's closure missed rather than a regression. The only 405 assertion in `crates/app/tests/api.rs` (line 2137, `the_openapi_spec_matches_the_served_surface`) guards spec↔router method parity, never the 405 body. The spec mentions neither 405 nor "method not allowed" anywhere.

### Repro

1. `curl -sS -X POST -D - 'https://tenders.zebreus.click/v1/tenders'` → 405, `content-length: 0`.
2. `curl -sS -D - 'https://tenders.zebreus.click/v1/nope'` → 404 with the JSON envelope.

### Done when

- `Router::method_not_allowed_fallback` is installed on the `/v1` router with an `ApiError` 405 constructor, so `POST /v1/tenders` answers `{"error":{"status":405,"message":"method not allowed"}}` with `content-type: application/json`.
- The `Allow` header survives the fallback (`allow: GET,HEAD` still present on `POST /v1/tenders`, `allow: POST` on `GET /v1/sql`).
- A test `wrong_methods_on_served_paths_are_json_405` sits beside issue 51's three, covering a GET-only collection path, an id path, `/v1/sql`, and an OPTIONS on a non-public path.
- `openapi.json` either lists the 405 response or the docs state the envelope covers every status; today it mentions 405 nowhere.

---

## Unit 4 — `limit` outside 1..1000 is clamped, not rejected, while the spec declares the range

### Observed (verified 2026-09-14 on prod)

```
curl -s 'https://tenders.zebreus.click/v1/tenders?limit=0'
curl -s 'https://tenders.zebreus.click/v1/tenders?limit=-5'
curl -s 'https://tenders.zebreus.click/v1/tenders?limit=abc'
curl -s 'https://tenders.zebreus.click/v1/tenders?limit=99999999999999999999'
```

| request | status | result |
| --- | --- | --- |
| `?limit=0` | 200 | 1 item (id 2), `next_cursor "2"`, `more true` |
| `?limit=-5` | 200 | identical: 1 item (id 2), `next_cursor "2"`, `more true` |
| `?limit=1` (control) | 200 | 1 item (id 2), byte-identical shape to `limit=0` |
| `?limit=1001` | 200 | 1000 items, `next_cursor 1026` |
| `?limit=abc` | 400 | `{"error":{"message":"Failed to deserialize query string: limit: invalid digit found in string","status":400}}` |
| `?limit=99999999999999999999` | 400 | "number too large to fit in target type" |
| `/v1/notices?limit=0`, `?limit=-1` | 200 | 1 item (id 1) each |

`crates/app/src/v1/mod.rs:570` declares `limit: Option<i64>` (signed, so negatives deserialize) and `mod.rs:676` does `self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, read::MAX_PAGE)` with `MAX_PAGE = 1000` (`crates/store/src/read.rs:2828`). It is a shared `Params` method, so all five endpoints taking `limit` (tenders, lots, organizations, notices, changes) behave identically; two were measured directly.

Against that, `crates/app/data/openapi.json:821` declares the shared `#/components/parameters/limit` as `{"type":"integer","default":100,"maximum":1000,"minimum":1}` with description "Page size.", and `/docs` line 168 says "Page size, default 100, max 1000." Neither says the value is clamped in either direction.

### Why it matters

Both bounds are soft on the server and hard in the spec, which points a spec-generated client and the server in opposite directions: the client refuses to send `limit=0` that the server quietly accepts, and a client asking for 0 rows gets 1 with no signal that its request was rewritten. The same parameter on the same endpoint 400s `limit=abc`, and `/docs` (`docs.rs:171-173`) states the posture explicitly — bad query input is "rejected with 400 rather than silently ignored". `limit` is the one parameter where that is not true. No wrong data is served; this is drift in the lenient direction.

### Why this is ours, not the publisher's

Contract drift the system created and then documented over. Issue 215-A is the near miss: it raised the spec and docs **maximum** from 500 to 1000 to match `MAX_PAGE`, and its verification accepted "accepted or clamped to the documented max" — but neither surface was ever updated to say values are clamped, and the floor was never looked at. The board has nothing else: greps for `limit=0`, negative limit, clamp, `MAX_PAGE` and the limit floor across the 389 issues return only 215 (ceiling value) and 336 (unmatchable filter values, unrelated). The project's own precedent supports filing: 215-A treated the analogous `limit` spec-vs-behaviour drift as low severity and fixed it rather than dismissing it.

### Repro

1. `curl -s 'https://tenders.zebreus.click/v1/tenders?limit=0'` → 200 with 1 item.
2. `curl -s 'https://tenders.zebreus.click/v1/tenders?limit=-5'` → the same.
3. `curl -s 'https://tenders.zebreus.click/v1/tenders?limit=abc'` → 400, the contrast.

### Done when

One of the two, decided and then true in both places — the point is that the spec and the server agree, not which way:

- **Reject:** `Params::limit()` answers 400 for `limit < 1` and `limit > 1000` in the standard envelope, matching `limit=abc`'s posture, and `?limit=0` on all five endpoints is a 400.
- **Or document:** `openapi.json`'s `limit.description` and `docs.rs:168` both state that out-of-range values are clamped to 1..1000, and the spec's `minimum`/`maximum` are reconciled with that (an OpenAPI `minimum` that the server does not enforce is the drift, whichever way it is resolved).
- Either way a test pins the chosen behaviour at both bounds on at least one shared-`Params` endpoint, so the floor cannot drift back the way it did after 215-A fixed only the ceiling.

---

## Unit 5 — `Accept: text/event-stream` on `/v1/notices?tender=<id>` answers JSON

### Observed (verified 2026-09-14 01:59Z on prod)

```
curl -N -sS -D - -o body.json -H 'Accept: text/event-stream' \
     'https://tenders.zebreus.click/v1/notices?tender=2'
```

| request, all with `Accept: text/event-stream` | status | content-type |
| --- | --- | --- |
| `/v1/notices?tender=2` | 200 | **`application/json`**, `content-length: 901`, a page envelope (`ignored_filters`/`items`/`more`/`next_cursor`) |
| `/v1/notices?tender=1000000` | 200 | `application/json`, 895 bytes |
| `/v1/notices?limit=2` | 200 | `text/event-stream`, `cache-control: no-store`, `event: change` frames, held open |
| `/v1/notices?publication_id=00710890-2023` | 200 | `text/event-stream`, one `added` event, held open |
| `/v1/lots?tender=2` | 200 | `text/event-stream` |
| `/v1/tenders?cursor=2:605108675` | 200 | `text/event-stream`, `cache-control: no-store`, one keep-alive `:` at ~15 s |

So `tender` is not a JSON-only parameter — `/v1/lots?tender=2` streams — and `/v1/notices` is not a JSON-only endpoint. Only the `/v1/notices?tender=` branch is. Non-existent ids (1, 605108675) return 404 JSON, which is every error path's behaviour and not part of the claim.

It is deliberate code: `crates/app/src/v1/mod.rs:1180-1195`, `notices()` returns `tender_notices()` before ever reaching `collection()`, and only `collection()` (line 864) consults `wants_events()`. The branch's own comment at 1185-1186 says "A lookup, not a subscription, so it is JSON regardless of Accept."

### Why it matters

A browser `EventSource` opened on the documented URL fails on the MIME type, and a script reading SSE framing gets a JSON object instead. The decision is recorded nowhere a client can see it: `/docs` (`docs.rs:308-309`) says "Send Accept: text/event-stream to any collection endpoint (with any filters)", lines 333-334 say "Notices have no live diffs (snapshot then silence)", the `/v1` root (`mod.rs:1378`) repeats "send Accept: text/event-stream to any collection endpoint", and `openapi.json`'s `/v1/notices` 200 lists `text/event-stream` (the `EventStream` schema) while its `tender` parameter reads "on /v1/notices, list the Notices that caused that Tender's versions" with no JSON-only caveat. The workaround is trivial (drop the header, or walk `/v1/tenders/{id}.versions[].caused_by_notice_id`), which is why this is low and not medium.

### Why this is ours, not the publisher's

An inconsistency introduced in the serving layer, unrelated to any publisher data: one filter branch silently changes the response media type. The project already has the right pattern for a REST-only shape and did not apply it here — `docs.rs:214-217` documents that `sort`/`order` on a stream is a 400, an explicit, discoverable refusal. The `tender` branch does neither: no 400, no doc line. Board check finds no cover: issues 49 (made `?tender=` honoured), 118 (`ignored_filters`), 220 (its performance), 227 (docs caught up — and its guard `the_docs_page_names_the_whole_spec_surface` only checks that parameter names appear, so it cannot catch this) and 215 (the OpenAPI drift cluster) all touch this branch or the docs, and none mentions its Accept behaviour.

### Repro

1. `curl -N -sS -D - -o /dev/null -H 'Accept: text/event-stream' 'https://tenders.zebreus.click/v1/notices?tender=2'` → `content-type: application/json`.
2. Same header on `/v1/notices?limit=2` → `text/event-stream`, connection held open.
3. Same header on `/v1/lots?tender=2` → `text/event-stream`. The parameter is not the cause; the branch is.

### Done when

One of the two, and the docs say so either way:

- **Stream it:** the lookup emits a snapshot-then-silence stream like every other `/v1/notices` shape (one `added` per item, a `live` marker), so `Accept: text/event-stream` on `?tender=` answers `text/event-stream`.
- **Or refuse it visibly:** `?tender=` plus `Accept: text/event-stream` answers 400 the way `sort`/`order` on a stream does, and the reason is in the envelope.
- `/docs` and `openapi.json`'s `tender` parameter description state the chosen rule, so a reader of either surface can predict the response media type without trying it.
- A test pins it, since 227's name-walking guard structurally cannot.

## Units 1 and 2 BUILT 2026-09-15 (owner)

### Unit 1 — validated at the boundary, not escaped in the predicate

The `## Done when` offered "shape-check in `Params::filter()`" **and/or** "stop the predicate being a
user-controlled pattern". I took the first and deliberately did **not** take the second, which is a
decision worth recording rather than an omission.

`Params::filter()` now runs `country` and `cpv` through one `shaped_prefix` helper: present means
non-empty and ASCII alphanumeric, anything else is a 400 in the standard envelope naming the
parameter. That is exactly the two vocabularies — NUTS is letters and digits (`DE91C`), CPV is
digits — and it is the posture `currency`, `lang` and `name_prefix` already take within twenty lines
of the same function. `%`, `_`, `\`, the empty string and a whitespace-only value can no longer
reach the store.

**Why not `LIKE ? ESCAPE '\'`.** The API is the contract boundary and the place the vocabulary is
documented; escaping in the read layer would instead make `?country=%` a *valid request for a
literal percent sign*, which is not a NUTS code and not something any client wants. It would also
change a hot predicate for no gain once the metacharacters are unreachable. Crucially the store
guard stays as it is: `read::prefix_ranges` still DECLINES on `%`/`_`/`\` (issue 117), because the
store is reachable independently of `/v1` by the supervisor and the censuses, and a guard that
assumed the API had already validated would be a guard that is wrong for its other callers. The two
layers are now belt and braces rather than one relying on the other.

**Case is not folded**, and that is deliberate: SQLite's `LIKE` is ASCII-case-insensitive, so
`?country=de` matches `DE300` today and `prefix_ranges` generates both case variants precisely to
stay consistent with it. Uppercasing "while validating" would have been a silent behaviour change.

**`tenders_shortcircuit.rs` needed no change**, contrary to the `## Done when`'s expectation that the
fix would fail it. Its `%` / `_E` assertions call the STORE directly, not the API, so they still pin
what they were written to pin — the guard-consistency invariant that a range can never be narrower
than the `LIKE`. That invariant is still live and still correct for internal callers; the fix removed
the input path, not the semantics it guards.

Tests: `a_code_prefix_filter_rejects_patterns_instead_of_reinterpreting_them` — nine refused shapes
per parameter (`%`, `_E`, empty, whitespace, `D%`, `4_`, `a\b`, `DE-91`, `DE 91`), the envelope
naming the parameter, and controls that matter as much: `DE`, `de`, `PL62`, `45`, `45000000` all
still 200 on `/v1/tenders` AND `/v1/lots`, and `country=ZZ` is still an empty 200 page rather than a
400 (issue 336 settled that an unmatchable value is conventional; only an unfiltered page dressed as
a filtered one was ever the defect). The SSE half is covered on `/v1/tenders` and `/v1/lots`, where
the `Filter` is built before the stream opens — a long-lived subscription silently carrying the wrong
rows is worse than one wrong page.

**A finding fell out of writing that test, filed as issue 399.** The draft asserted
`/v1/changes?country=_E` → 400 and it came back 200. `/v1/changes` is registered `get(changes)` with
no SSE branch, and `changes()` never calls `params.filter()` — it uses `since`, `limit` and `entity`
and silently drops every other filter, with no `ignored_filters` array to say so. Verified on prod:
`source=nonesuch`, `status=open`, `min_value=999999999999`, `buyer=1` and `country=ZZ` all leave the
answer unchanged while `entity=tender` changes it. So unit 1 introduces a visible asymmetry —
`/v1/tenders?country=_E` 400 vs `/v1/changes?country=_E` 200 — and that asymmetry is issue 399's
symptom, not a regression here. Fixing it inside this commit would have meant adding a whole
`ignored_filters` contract to the changes response, which is its own review.

### Unit 2 — the one sub-resource is named, not a loosened depth rule

`is_public_surface()` grants `/v1/notices/{id}/content` explicitly. The route takes no `AuthUser` and
carries no `security` in the spec, so both doc surfaces' promise — "every endpoint that needs no
token is CORS-open to any origin" — already covered it; the code was the stale half. The
one-extra-segment rule predates the route (CORS grant 2026-08-15, content route 2026-08-16 under
issue 218-B) and swallowed it silently, so a browser client was blocked twice: 405 on the preflight
and no ACAO on the GET, on the very endpoint 218-B built to make the parsed payload reachable without
`/v1/sql`.

Named exactly rather than by allowing depth ≥ 2, because the completeness test's `/v1/tenders/1/x`
case must stay non-public and this is the only route in the surface with a segment after `{id}`. A
future sub-resource should have to say so here. The negative cases are now pinned too:
`/v1/tenders/1/content`, `/v1/notices//content`, `/v1/notices/1/2/content`,
`/v1/notices/1/content/x` and `/v1/notices/1/contents` all stay out, and the e2e asserts the
preflight is answered for the content route and NOT for `/v1/tenders/1/content`.

Docs: `openapi.json`'s `country` and `cpv` parameters now state the letters-and-digits rule and the
400. The CORS sentences needed no change — after unit 2 they are simply true.

### Still open on this issue

- **Units 3, 4 and 5 are not built**: the bodyless 405, `limit` clamping instead of 400ing, and the
  `Accept: text/event-stream` gap on `/v1/notices?tender=<id>`. All three are the low-severity tier
  and none is touched here.
- Not deployed. Rides the next deploy with 392, 385 unit 2, 395 and 398.
- Acceptance reads for after the deploy: `?country=_E`, `?country=`, `?cpv=%25` → 400 on
  `/v1/tenders` and `/v1/lots`; `?country=DE`, `?country=de`, `?cpv=45`, `?country=ZZ` → 200 with the
  same rows as today; `HEAD /v1/notices/1/content` with an `Origin` → ACAO `*`; `OPTIONS` on it → 204
  with the preflight set.

## Units 1 and 2 DEPLOYED AND VERIFIED 2026-09-16 — rev `5affd75`

**Unit 1, refused (all 400):** `/v1/tenders?country=_E`, `?country=`, `?cpv=%25`; `/v1/lots?country=_E`,
`?cpv=%25`. The envelope is the standard one and names the parameter:

```
{"error":{"message":"country must be a NUTS place code (e.g. DE, PL62) — letters and digits only, not \"_E\"","status":400}}
```

**Unit 1, controls (all 200, unchanged):** `country=DE`, `country=de` (case still not folded),
`country=PL62`, `cpv=45` on `/v1/tenders`; `country=DE` on `/v1/lots`. And `country=ZZ` is still a
**200 with 0 items**, not a 400 — issue 336's settled position survives the fix.

**Unit 2:** `GET /v1/notices/1/content` with an `Origin` now carries
`access-control-allow-origin: *`, and its `OPTIONS` preflight answers **204**. The negative control
`OPTIONS /v1/tenders/1/content` is **404** (no such route) rather than 204 — the grant is one named
sub-resource, not a loosened depth rule.

Units 1 and 2 are done. **Units 3, 4 and 5 remain open** (the bodyless 405, `limit` clamping instead
of 400ing, and the SSE `Accept` gap on `/v1/notices?tender=<id>`) — all three are the low-severity
tier and none was touched.

## Units 3, 4 and 5 BUILT 2026-09-16 (owner) — the issue is now complete

### Unit 3 — `method_not_allowed_fallback`

`/v1` installed a PATH fallback and not a METHOD one, so a matched-path/wrong-method request fell to
axum's `MethodRouter` default: 405, `content-length: 0`, no content-type. One line on the router
(`.method_not_allowed_fallback(method_not_allowed)`) plus an `ApiError` constructor fixes it. The
`Allow` header survives, because the fallback replaces the BODY and not the routing verdict — the
test asserts `allow: GET…` on `POST /v1/tenders` and `allow: POST` on `GET /v1/sql`, since a 405
without `Allow` would trade one contract break for another.

`wrong_methods_on_served_paths_are_json_405` covers a collection path, an id path, `/v1/sql`, and an
OPTIONS on a non-public path (where `public_cors` does not short-circuit with its 204). `/docs`'
errors bullet now says the one shape covers *every* status, naming the 405 and its `Allow` header.

### Unit 4 — reject, not document

The `## Done when` left the direction open. **Reject**, for two reasons. `/docs` already states the
API's posture — bad query input is "rejected with 400 rather than silently ignored" — and `limit=abc`
on the same endpoint already 400s, so `limit` was the single parameter contradicting its own
endpoint. And documenting the clamp leaves a spec-generated client and the server pointing in
opposite directions: the client refuses to send `limit=0` that the server quietly accepts.

`Params::limit()` returns `Result<i64, ApiError>` and 400s outside `1..=1000`, naming the maximum so
a caller can correct itself rather than paginating forever wondering why its pages are short.

**The check moved ABOVE the `wants_events` branch**, which the issue did not ask for and which
matters: `collection()` validated after the SSE early-return, so `?limit=0` would have been a 400 as
JSON and accepted as a stream. An API whose validation switches on a request header is the
inconsistency this whole issue is about. A test pins it.

`an_out_of_range_limit_is_rejected_rather_than_clamped` runs all five `Params`-sharing endpoints
against `0`, `-5`, `1001`, and — the assertion that matters more — `1` and `1000` still 200, because
an off-by-one at the bounds would be worse than the clamp ever was.

### Unit 5 — refuse the stream visibly

`/v1/notices?tender=` answers 400 to `Accept: text/event-stream` instead of silently returning JSON.
Refused rather than streamed: the branch is a lookup and the comment at its head has always said so;
building a snapshot stream for it would be inventing a feature to fix a documentation gap.

**400 and not 406**, deliberately, though 406 is the more literal status for "I cannot produce that
media type". This API has exactly one refusal shape for "that request shape is not servable here" —
`reject_sort` uses it for `sort`/`order` on a stream — and introducing a second status for the same
class of refusal would be a new inconsistency inside the issue that exists to remove them.

The controls are the interesting half and all three are pinned: the same endpoint WITHOUT `?tender=`
still streams, `tender` on `/v1/lots` still streams, and the lookup still answers JSON to a JSON
client. Neither the endpoint nor the parameter was ever the cause — only their combination.

### Docs

`openapi.json`: `limit.description` states the 1–1000 rule and that out-of-range is a 400, not a
clamp; the `tender` parameter says the `/v1/notices` form is a lookup and a stream request is a 400.
`/docs`: the same two, plus the errors bullet covering 405.

### Status

**Units 1–5 are all built.** Not deployed — the issue-397 text-era re-parse (job 1386) is running;
this rides the next deploy with issue 399.

Acceptance reads for after: `POST /v1/tenders` → 405 with the JSON envelope and an `Allow` header;
`/v1/tenders?limit=0`, `?limit=-5`, `?limit=1001` → 400 and `?limit=1`, `?limit=1000` → 200;
`Accept: text/event-stream` on `/v1/notices?tender=<id>` → 400 while `/v1/notices?limit=2` and
`/v1/lots?tender=<id>` still stream.

## Units 3, 4 and 5 DEPLOYED AND VERIFIED 2026-09-16 — rev `a5db49e`

| unit | request | answer |
| --- | --- | --- |
| 3 | `POST /v1/tenders` | `405`, `content-type: application/json`, `allow: GET,HEAD`, `{"error":{"message":"method not allowed","status":405}}` |
| 3 | `DELETE /v1/tenders` | the same |
| 4 | `?limit=0` / `-5` / `1001` | `400` — `limit must be between 1 and 1000, not 0` |
| 4 | `?limit=1` / `?limit=1000` | `200` (the bounds themselves still valid) |
| 5 | `Accept: text/event-stream` on `/v1/notices?tender=2` | `400`, `application/json` |
| 5 | same header on `/v1/notices?limit=2` | `200 text/event-stream` |
| 5 | same header on `/v1/lots?tender=2` | `200 text/event-stream` |

The unit-5 controls are the ones that matter: the endpoint still streams without `?tender=`, and
`tender` still streams on another collection, so the refusal is scoped to the one branch that was
never a subscription rather than to an endpoint or a parameter.

**Issue 390 is complete — all five units built, deployed and verified.**
