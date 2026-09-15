# 387 — `/v1/organizations`: the `name_prefix` upper bound is silently dropped for a whole class of prefixes, and `kind` is a case-sensitive match on a lowercase vocabulary

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (app + store — the `/v1/organizations` read path; unit 2 is also a docs defect, in `/docs` and `openapi.json`)
Relates to: 217 / 217-B (the name-ordered builder and the `identifier`+`kind` lookup both units sit on;
its Verification line reads `GET /v1/organizations?identifier=DE123456789&kind=VAT returns the org`,
which is unit 2's wrong assumption written down as a passing check), 284 (RESOLVED — the same name
path silently dropping `identifier`/`buyer`; unit 1 is that shape again, one layer down), 227 (the
/docs-vs-openapi guard — it walks parameter NAMES, not example VALUES, so it cannot catch unit 2),
336 (CLOSED NOT-WORTH-IT — an unmatchable filter value is indistinguishable from no results; its
"lowercase is already handled" remark is about `country`, not `kind`), 370 (served claims with no
gate coupling them to behaviour — both units are that class), 117 (the other `successor()` caller,
the CPV code range: ASCII digits, unaffected)

## What ties these together

Two filters on one endpoint, both on the two builders `organizations_query` (`crates/store/src/read.rs`
~2891/2906) and `organizations_by_name` (~2943/2950). Both fail **silently**: HTTP 200,
`ignored_filters: []`, no 400. Both fail on the spelling the public surface itself prints.

| unit | filter | failure | what the client sees |
| --- | --- | --- | --- |
| 1 | `name_prefix` | the range's upper bound is dropped whenever the prefix's last character encodes to a `0xBF` tail byte (Greek `ο`, Cyrillic `п`, `¿`, `ÿ`, `ſ` …) | rows that do not match the prefix, `more:true` forever — pagination becomes a walk of the whole name index in byte order |
| 2 | `kind` | exact, case-sensitive match against a vocabulary that is 100% lowercase (`vat`, `national`) | an empty page for the exact query printed on `/docs`, which reads as "this org has no VAT identifier" |

They are opposite errors — unit 1 returns rows that should not be there, unit 2 returns nothing where
a row exists — with the same root habit: one parameter handed to the store without the care its
neighbours get. `currency` is folded with `to_ascii_uppercase`, `lang` with `normalize_lang`,
`name_prefix` with `to_lowercase` (and a comment saying why); `kind` is `self.kind.clone()`. The
prefix bound is computed on a raw byte where every other range in the file is computed on a value.
Filed as one issue because a maintainer fixing either will be inside the same two builders with the
same tests open.

## Why it matters

`/v1/organizations` is the documented front door: `/docs` tells a consumer holding a VAT number or a
name to start here and then pivot to `?buyer=`/`?winner=`/`?bidder=` for participation history. Both
units break that first step without saying so.

For unit 1, a client searching `name_prefix=Δήμο` (Greek neuter organization words end in -ο:
Υπουργείο, Νοσοκομείο, Πανεπιστήμιο, Δήμο) or any Cyrillic prefix ending in `п` gets a page of
confidently wrong organizations with `ignored_filters: []` — the field the API uses to admit it
ignored something is empty, so the only honest signal is absent. Worse, because the slice has no
upper bound, `more` never turns false at the prefix boundary: a client paging to exhaustion walks
every name that sorts after the prefix in byte order, across scripts. At `limit=100` the `яп` page
has already left Cyrillic and reached an em-dash name. A consumer joining organizations by name
search — the documented way to resolve a name to a canonical id — silently binds the wrong entity.

For unit 2, the query `/docs` prints verbatim returns an empty page. A reader who runs it concludes
the organization has no VAT identifier, when in fact org 2 carries `identifier_kind: "vat"` and the
only spelling that matches (`vat`) appears nowhere on the public surface — not in `/docs`, not in
`openapi.json`, which gives `kind` no enum. Everything a consumer can read tells them to write `VAT`.

## Why this is ours, not the publisher's

Nothing here comes from a source. Unit 1 is our own range computation: `successor()` bumps a raw
UTF-8 byte and then throws the result away when it is not valid UTF-8, and the two callers respond to
`None` by omitting the `AND o.name_norm < ?` clause entirely rather than refusing or falling back —
the in-code comment calls this "a 0xFF-tail edge", which is a misdiagnosis, since `0xFF` never occurs
in UTF-8 at all while `0xBF` is the tail byte of a large, ordinary class of characters. Unit 2's
lowercase vocabulary is minted by our own projector (`crates/ingest/src/project.rs:5622`, `:5694`),
not copied from a publisher; our app forwards `kind` unfolded into an exact comparison while
deliberately folding every neighbouring parameter; and our own served docs print the spelling that
cannot match. Every input needed to behave correctly is already in the process.

## Unit 1 — `name_prefix` drops its upper bound on a `0xBF` tail byte

Severity: HIGH as the lens rated it. The comparable over-broad-result defect on this same path (284)
was rated MEDIUM, so medium is defensible — triage's call.

### Observed (verified 2026-09-14 on prod, rev 9e082fd1)

    curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=%D1%8F%D0%BF&limit=5'   # prefix "яп"

| id | name | starts with `яп`? |
| --- | --- | --- |
| 23377732 | ЯПИ ГРУП ЕООД | yes |
| 12703610 | Яръмов АЦ ООД | no |
| 23562678 | ЯС-КА ЕООД | no |
| 22864344 | Ясин Ус | no |
| 20471808 | ЯСНА ПОЛЯНА | no |

`more: true`, `next_cursor: "20471808~ясна поляна"`, `ignored_filters: []`. The true `яп` slice is
exactly one row — the second row already sorts past the prefix — so four of five rows are spill.

    curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=%C2%BF&limit=3'        # prefix "¿"
    curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=zzzz%D0%BF&limit=3'   # prefix "zzzzп"

`¿` → 25607973 "×&Ð. Âáôßóôáò ÏÅ", 30876013 "×. ÂåñÝìçò-Otis ABETE", 30692901 "×. Èåïäüóçò ÁÂÅÅ",
`more:true` — every name starts with `×` (U+00D7, bytes `C3 97`, which sorts after `C2 BF`), 0/3
match. `zzzzп` → 21456281 "Z|IMMER BIOMET POLSKA Sp. z o.o.", 10878187 "Zábojník - contractors,
s.r.o.", 1513429 "ZÁBOJNÍK – contractors, s.r.o.", `more:true`, 0/3 match.

**Which prefixes break, by the last character's UTF-8 tail byte:**

| prefix | last char's bytes | `successor()` | page |
| --- | --- | --- | --- |
| `stadt` | ASCII `t` | `"stadu"` | 5/5 match; `limit=1000` → 1000/1000 match |
| `яр` | `d1 80` | `"яс"` | 1 item, 1/1 match, `more:false` |
| `яп` | `d0 bf` | `0xBF+1 = 0xC0`, invalid UTF-8 → `None` | 1/5 match, `more:true` |
| `ЯП` (uppercase input) | lowercased to `яп` first | `None` | identical result set to `яп` |
| `¿` | `c2 bf` | `None` | 0/3 match |
| `zzzzп` | `d0 bf` | `None` | 0/3 match |
| `δήμο` | `ce bf` | `None` | 5/5 match alone (the `δήμο` slice is deep); 0/5 with `&country=FR` |

The decisive probe is the last row: `name_prefix=δήμο&country=FR&limit=5` returns 5 items, 0
matching the prefix (10180992 "Δημοτική Επιχείρηση…", 4581983 "ΔΙΑΓΝΩΣΤΙΚΗ…", 8647082, 3375743,
4875034), `more:true`, `ignored_filters: []` — the correctly-bounded slice for that prefix is empty,
so every returned row is spill from the missing upper bound.

The unbounded walk is visible directly: `name_prefix=яп&limit=100` → 1 match / 99 non-matches,
`more:true`, and the 100th row is 6981834 "— Delepierre, …" — an em-dash (U+2014) name. One page has
already left Cyrillic.

**Mechanism**, `crates/store/src/read.rs:1386`:

```rust
fn successor(prefix: &str) -> Option<String> {
    let mut bytes = prefix.as_bytes().to_vec();
    while let Some(last) = bytes.pop() {
        if last < 0xFF {
            bytes.push(last + 1);
            return String::from_utf8(bytes).ok();
        }
    }
    None
}
```

A valid UTF-8 string ends in either an ASCII byte or a continuation byte `0x80..=0xBF`. Bumping
`0xBF` gives `0xC0`, which is never valid UTF-8, so `from_utf8` fails and `successor` returns `None`.
Both callers then emit `o.name_norm >= ?` with **no** `< ?` clause:

- `read.rs:2903-2906` (`organizations_query`, the id-ordered path) — its comment reads "upper bound is
  the prefix's successor, or unbounded for a 0xFF-tail edge".
- `read.rs:2943` (`organizations_by_name`, the REST path the handler routes every `name_prefix`
  request to).

`0xFF` cannot occur in UTF-8, so that branch is dead code. The live edge is `0xBF`, the tail byte of
every code point whose low six bits are `0x3F`: U+00BF `¿`, U+00FF `ÿ`, U+017F `ſ`, U+03BF `ο`,
U+043F `п`, U+047F … — and the handler lowercases the prefix first
(`crates/app/src/v1/mod.rs:653-662`), so `Ο` and `П` route into the same edge. The existing tests
(`crates/store/tests/org_name_search.rs`: "mü", "siemens"; `crates/app/tests/api.rs:1283`) all use
prefixes that bump cleanly — `ü` is `C3 BC`.

The documented contract this contradicts: `crates/app/src/v1/docs.rs:239` — `name_prefix` is "a
prefix match on the organization's name, case-insensitive across the whole of Unicode"; the filter
table at the same file calls it "Organization-name prefix, Unicode case-insensitive (`mü` matches
`MÜLLER`)".

Not measured: the SQL cross-check of the true `яп` count and the total length of the unbounded walk —
the box read was declined by the permission classifier and not retried. The empty-slice `country=FR`
probe, the `limit=100` walk and the bounded `яр`/`stadt` controls carry the finding without it.

### Repro

Under a minute, public API only, no snapshot.

1. `curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=%C2%BF&limit=3'` → three
   organizations whose names start with `×`, none with `¿`, `more:true`, `ignored_filters:[]`.
2. `curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=%D1%8F%D0%BF&limit=5'` →
   1 match, 4 non-matches, `next_cursor "20471808~ясна поляна"`.
3. Control, same script, tail byte `0x80`:
   `curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=%D1%8F%D1%80&limit=5'` →
   exactly 1 item, `more:false`. Bounded and clean, so the fault is the tail byte, not Cyrillic.
4. Empty-slice probe:
   `curl -sS 'https://tenders.zebreus.click/v1/organizations?name_prefix=%CE%B4%CE%AE%CE%BC%CE%BF&country=FR&limit=5'`
   → 5 rows, 0 matching.

### Done when

- `successor()` (`crates/store/src/read.rs:1386`) computes the bound on the last **char**, not the
  last byte: pop the last `char` and push `char::from_u32(c as u32 + 1)`, carrying past the surrogate
  gap (U+D7FF → U+E000) and past U+10FFFF, or carry across the encoded byte sequence. `None` is
  returned only for the genuine top-of-Unicode case, where unbounded is correct.
- The "0xFF-tail edge" comment at `read.rs:2903` is gone or corrected; `0xFF` cannot occur in UTF-8
  and the comment is what let this stand.
- Both callers — `organizations_query` (`read.rs:2906`) and `organizations_by_name` (`read.rs:2943`)
  — emit `AND o.name_norm < ?` for every prefix that has a successor; the unbounded branch is
  exercised by a test that proves it is the rare case, not the silent default.
- `crates/store/tests/org_name_search.rs` covers one prefix per class and asserts both "every row on
  the page matches" and "`more` is false at the end of the slice": `п`/`ο`/`¿` tail (`0xBF`), `ü`
  (`C3 BC`, the clean case already there), ASCII, and a multi-char prefix whose last char is the edge
  (`zzzzп`).
- An API-level test in `crates/app/tests/api.rs` pins the served shape: `name_prefix=яп&limit=100`
  returns only `яп` names, and a prefix with an empty slice under a companion filter returns an empty
  page rather than spill.
- Live after deploy: `name_prefix=%D1%8F%D0%BF&limit=5` → 1 item, `more:false`;
  `name_prefix=%C2%BF&limit=3` → 0 items; `name_prefix=%CE%B4%CE%AE%CE%BC%CE%BF&country=FR&limit=5`
  → 0 items; `name_prefix=stadt&limit=1000` still 1000/1000 (no regression on the clean path).

## Unit 2 — the `/docs` lookup example returns an empty page: `kind` is case-sensitive

Severity: MEDIUM. Deterministic, on the documented front-door lookup, silent-empty failure mode;
mitigated by the bare `identifier=` form working and being the next code block on the same page.

### Observed (verified 2026-09-14 on prod, rev 9e082fd1)

    curl -sS "https://tenders.zebreus.click/v1/organizations?identifier=RO42283735"
    → {"ignored_filters":[],"items":[{"country":"RO","id":2,"identifier":"RO42283735","identifier_kind":"vat","mentions":116272,"name":"Operator SEAP","provisional":false}],"more":false,"next_cursor":null}

    curl -sS "https://tenders.zebreus.click/v1/organizations?identifier=RO42283735&kind=VAT"
    → {"ignored_filters":[],"items":[],"more":false,"next_cursor":null}

The second query is not an invention: it is printed verbatim on the served `/docs` page, in the
"Lookups by real-world key" table (`crates/app/src/v1/docs.rs:233`, row "An organization's official
identifier (VAT & co.)"), and the filter table at `docs.rs:159` describes the parameter as "Tender/Lot
kind flag; on `/v1/organizations`, the identifier scheme (e.g. `VAT`)".

**Every spelling, all HTTP 200 with `ignored_filters: []`:**

| query | items |
| --- | --- |
| `identifier=RO42283735` | 1 — org 2, `identifier_kind: "vat"` |
| `identifier=RO42283735&kind=vat` | 1 — org 2 |
| `identifier=RO42283735&kind=VAT` (the documented example) | **0** |
| `identifier=RO42283735&kind=Vat` | **0** |
| `country=RO&kind=vat&limit=2` | 2, `more:true` |
| `country=RO&kind=VAT&limit=2` | **0** |
| `country=RO&kind=national&limit=2` | 2, `more:true` |
| `country=RO&kind=NATIONAL&limit=2` | **0** |

So the filter fails on its own, not only when paired with `identifier`.

**The stored vocabulary is lowercase-only** — no uppercase spelling can ever match:

    ssh root@zebreus.click 'echo "SELECT identifier_kind, COUNT(*) FROM organizations WHERE id BETWEEN 1 AND 100000 GROUP BY identifier_kind ORDER BY 2 DESC" | /root/sq.sh'

| id window | `null` | `national` | `vat` | any uppercase spelling |
| --- | --- | --- | --- | --- |
| 1–100,000 | 11,653 | 11,423 | 1,301 | none |
| 20,000,000–20,050,000 | 4,170 | 1,387 | 82 | none |
| 5,000,000–5,100,000 | 17,147 | 1,032 | 13 | none |

**Mechanism.** `crates/app/src/v1/mod.rs:648` passes `kind: self.kind.clone()` through untouched,
while the parameters immediately around it are folded on purpose — `currency` with
`to_ascii_uppercase`, `lang` with `normalize_lang`, `name_prefix` with `to_lowercase` under a comment
explaining why. It arrives at `crates/store/src/read.rs:2891`,
`q.push(" AND o.identifier_kind = ?", [t(kind)])` — exact — and the same line again at `:2950` on the
name path. The vocabulary itself is minted lowercase by the projector
(`crates/ingest/src/project.rs:5622`, `:5694`). `openapi.json:754` gives `kind` no enum: "A kind flag
whose meaning is collection-specific: `procedure`/`registration` for Tenders, the lot kind
(`Lot`/`LotsGroup`/`Part`) for Lots, the identifier kind for Organizations, and the mapping profile
for Notices." The spelling that works appears nowhere on the public surface.

Origin: issue 217's Verification line reads `GET /v1/organizations?identifier=DE123456789&kind=VAT
returns the org`, and its prod verification recorded "identifier + kind in 6 ms" without naming the
spelling — so the uppercase form was, on this evidence, never actually run. Issue 227's guard walks
`components.parameters` and `paths` against the docs source, i.e. parameter names, and cannot see
that an example's *value* is unmatchable.

Not in scope: `kind` on Tenders/Lots/Notices, which carry different vocabularies
(`procedure`/`registration`, `Lot`/`LotsGroup`/`Part`, profile ids) documented in their stored case.

### Repro

Under a minute, public API only.

1. `curl -sS "https://tenders.zebreus.click/v1/organizations?identifier=RO42283735"` → org 2, with
   `"identifier_kind":"vat"`.
2. `curl -sS "https://tenders.zebreus.click/v1/organizations?identifier=RO42283735&kind=VAT"` →
   `{"ignored_filters":[],"items":[],"more":false,"next_cursor":null}`. This is the query `/docs`
   prints.
3. `curl -sS "https://tenders.zebreus.click/v1/organizations?identifier=RO42283735&kind=vat"` → org 2.
4. Without `identifier`:
   `curl -sS "https://tenders.zebreus.click/v1/organizations?country=RO&kind=NATIONAL&limit=2"` → 0
   items; swap to `kind=national` → 2 items, `more:true`.

### Done when

- One of the two is true, not half of each: either `kind` is folded to ASCII lowercase **for the
  Organizations collection only** in `crates/app/src/v1/mod.rs` (a blanket fold would break the other
  collections' documented mixed-case vocabularies), or `docs.rs:159` and `docs.rs:233` print `vat`.
- `openapi.json:754` names the Organizations vocabulary — an enum or at minimum the literal spellings
  `vat` / `national` — so a client can discover the working value from the spec alone.
- A test asserts the documented lookup example resolves: the exact query string printed at
  `docs.rs:233` returns org 2. The natural generalisation, and the thing 227's guard is missing, is a
  gate that executes every `GET /v1/...` example the docs page prints and fails on an empty page.
- Live after deploy: the query as printed on `/docs` returns org 2, and
  `country=RO&kind=VAT&limit=2` either returns rows or is reported in `ignored_filters` — not a silent
  empty 200.
- Issue 217's Verification line is corrected where it records `&kind=VAT` as passing, so the wrong
  assumption stops being cited as evidence.

## BOTH UNITS BUILT 2026-09-15 (owner)

### Unit 1 — `successor` steps a character, not a byte

`crates/store/src/read.rs`. The old body incremented the last BYTE and then called
`String::from_utf8(…).ok()`, so a `0xBF` tail byte became `0xC0`, the string stopped being UTF-8, and
the `.ok()` that noticed turned it into `None` — which both call sites (`read.rs:2906` and the
name-ordered `organizations_by_name` at `:2943`) read as "no upper bound". The new body pops the last
CHARACTER, increments its scalar value, steps over the surrogate gap (`U+D7FF` → `U+E000`), and
carries into the previous character when the last one is `char::MAX`.

`None` now means what those call sites already assume: there genuinely is no upper bound. It happens
for the empty prefix (everything matches) and for a prefix that is entirely `char::MAX` (nothing
sorts above it). Neither widens a result, so no call site needed changing.

**Why the test is on the function and not end to end.** A too-wide range and a correct one differ
only in rows the page limit cut off, so an assertion on a served page cannot distinguish them — the
same reason issue 117's guard test sits on `prefix_ranges`. `the_prefix_successor_steps_a_character_not_a_byte`
asserts the two properties that make `>= prefix AND < successor` mean "has this prefix": the successor
is strictly greater than the prefix, and the prefix extended by `U+10FFFF` still sorts below it. It
covers one carrier per alphabet the corpus holds (`яп` `d0 bf`, `δήμο` `ce bf`, `¿` `c2 bf`), the
surrogate gap, and the two degenerate cases. It bites on the old code: `succ("яп")` returned `None`
where the test demands `Some("яр")`.

### Unit 2 — `kind` is folded at the edge

`crates/app/src/v1/mod.rs`: `kind: self.kind.clone()` → trimmed and ASCII-lowercased, in the same
block that already folds `name_prefix` (Unicode-lowercased) and `currency`. Every vocabulary this
parameter filters is lowercase-only in store — `vat`, `national` on organizations, `procedure` on
tenders — so no casing can be lost, and the uppercase spelling `/docs` prints as the front-door
identifier lookup now works.

Empty-string handling is deliberately UNCHANGED: `kind=` still yields `Some("")` and matches nothing,
as before. Turning it into `None` would silently drop a filter the response reports as applied, which
is the issue-118/284 failure mode one step over; if that shape is worth refusing it is its own change.

`organizations_can_be_looked_up_by_identifier` now runs the identifier+kind pairing in three casings
(lower, upper, title) and asserts the org comes back in each.

### Still open

- **Neither unit's served-side numbers have been re-measured on prod** — the fixes are built and
  gated but not deployed; the box is folding the issue-395 backfill. The `## Done when` bullets
  (`name_prefix=яп` returning 1/1, `δήμο&country=FR` returning an empty page, `kind=VAT` returning
  org 2) are the acceptance reads for the next idle window.
- The issue notes `successor`'s sibling risk: `prefix_ranges` (the case-variant expansion for
  `cpv`/`country`) calls `successor` too, at `read.rs:1381`. It is ASCII-only by construction — the
  guard declines non-ASCII prefixes outright (issue 117) — so it was never exposed to the defect, and
  it inherits the fix for free.
