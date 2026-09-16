# 394 — DÖE serves two publisher strings as its own keys: 7,158 notices keyed on TED's placeholder publication id `00000000-1900`, and sdk-0.1 CPV codes in four shapes under one `scheme`

Status: ready-for-agent — unit 1's GUARD built and gated 2026-09-16 (the election refuses the all-zero placeholder; blast radius re-measured corpus-wide as exactly one value, 7,177 rows, all `doe`). The **re-key of those 7,177 carriers is the next unit and is now unblocked**, since issue 290 — which this issue required settled first — was resolved the same day. Unit 2 (sdk-0.1 CPV shapes) untouched. Filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (ingest → fold boundary, source `doe`) — unit 1 is the publication-id election in `crates/ingest/src/profile.rs`, unit 2 is the unnormalised classification code from `crates/ingest/src/eforms/value.rs` through `crates/ingest/src/project.rs`; both land on a served identity/vocabulary field and on the documented filter over it
Relates to: 12 (RESOLVED — the DÖE source, eForms-DE + sdk-0.1 profiles, the parent of both units), 217 (RESOLVED & VERIFIED 2026-08-16/17 — it shipped `publication_id=` on `/v1/notices` and `/v1/tenders` as "the keys real consumers hold"; unit 1 is 7,158 rows where that key is a placeholder shared by the whole cohort), 290 (ANALYSIS, open — "a parser change that shifts `publication_id` derivation makes `reparse_notice` silently no-op (counted as benign `unmatched`)", filed at LOW confidence it ever bites: unit 1's re-key is exactly the case that makes it bite, and it must be checked before the re-key, not after), 369 (DONE — a published BT-04 taken verbatim as the Tender group key, gated by `is_placeholder_key`; unit 1 is the same placeholder-as-key shape one field over), 366 (DONE — its unit-6 sentinel discovery sweep is DQ report section 10, but it sweeps AMOUNTS, so an all-zero identity STRING is invisible to it; its unit 3 is also the precedent against a second, display-side implementation of a fold rule, which unit 2's "done when" keeps), 365 (DONE — "any ≥4-character alphanumeric string containing a digit becomes an Organization merge key": the same any-string-is-a-key class on the org layer), 364 (the legacy OJS closure weld and its weld gauge `c0c2581` — the only broad board hit near DÖE publication identity, and unrelated to this cohort), 29 (VERIFIED on prod 2026-08-18 — the sdk-0.1 projection gap; it split the residual value/CPV out to 231), 231 (CLOSED 2026-08-27 — closed the sdk-0.1 CPV half on PRESENCE only, 93.8 % from 0.0 %, and never looked at representation; unit 2 is precisely what a presence measure cannot see), 172 (CURRENCY half CLOSED as ADR-0014, CLASSIFICATION half OPEN — and that half is codelist VINTAGE drift, 2003-vs-2008 meanings, explicitly not string shape; its closed half's answer, an alias map at the fold, is the pattern unit 2 wants), 292 (FIX DEPLOYED 2026-08-26 — `normalize_lang` at the fold boundary, the precedent in terms: "each new source adds a dialect unless a normalization layer exists"), 319 (org layer DONE 2026-08-30 — the country column held alpha-3 codes and free text; same normalise-at-the-boundary shape), 171 (its `/docs` #caveats deliverable, shipped 2026-08-23, today naming only CPV-2003/2008 coexistence — where unit 2's division-level-code caveat belongs), 118 (RESOLVED — `ignored_filters`; note `cpv` DOES narrow tenders and lots, so unit 2's glued rows are not an ignored filter, they are a filter that runs and misses), ADR-0003, ADR-0004 (the per-profile mapped-or-ignored checklist), ADR-0014 (the alias-map precedent), CONTEXT.md (TED owns publication identity), `docs/research/eforms-de-profile.md` §2
Blocked by: nothing

Two DÖE defects, one per DÖE profile island, found by different lenses of the same fan-out and filed
together because they are one mechanism: a string the publisher wrote is carried verbatim into a field
this system KEYS or FILTERS on — `publication_id` in unit 1 (eForms-DE), `classifications.code` in unit 2
(sdk-0.1) — with no normalisation step at the fold boundary, so the documented lookup over that field
degenerates for the DÖE cohort while every other source and era behaves. Both are bounded and exactly
counted, neither loses content, and both take the same fix shape: a guard at the election/emit point, a
fixture per shape, then a bounded refold of the affected island. The board has decided this class three
times in this direction already (292 lang tags, 172/ADR-0014 currency, 319 country), and unit 1 repeats
369's placeholder-key shape one field over. They differ in urgency, not in kind: unit 1 is LIVE in the
daily ingest (9 of 1,395 DÖE notices in the 2026-09-13 tick), unit 2 is a fixed historical island that
grows only with new sdk-0.1 publication.

## Unit 1 — 7,158 DÖE notices are keyed on TED's placeholder publication id

### Observed (verified 2026-09-14 on prod)

    curl -s "https://tenders.zebreus.click/v1/notices?publication_id=00000000-1900&limit=1000"

→ 1000 items, `more:true`, every row `source=doe`, profile `eforms:eforms-de-1.1`, every `member_path` a
real uuid-version file (id 26200469 → `eb131bee-d78c-4a9c-9f4a-9fded9b01a21-03.xml`, 26200972 →
`0100e34e-…-01.xml`, 26200977 → `bc647464-…-01.xml`), all with `publication_id` `00000000-1900`.

    curl -s https://tenders.zebreus.click/v1/notices/26447665

→ `{"member_path":"a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01.xml","publication_id":"00000000-1900","profile":"eforms:eforms-de-1.2","parse_state":"parsed","source":"doe"}`

    curl -s https://tenders.zebreus.click/v1/tenders/1499198

→ `"publication_id":"00000000-1900"`, `versions[0].caused_by_notice_id` 26447665,
`versions[0].publication_id` `"00000000-1900"`.

    ssh root@zebreus.click 'echo "SELECT count(*) AS n, min(id) AS min_id, max(id) AS max_id, count(DISTINCT content_hash) AS distinct_hashes, count(DISTINCT profile) AS profiles, min(profile) AS p_min, max(profile) AS p_max FROM notices WHERE publication_id = \"00000000-1900\"" | /root/sq.sh'

→ `[7158, 26200469, 31767359, 7158, 4, "eforms:eforms-de-1.1", "eforms:eforms-de-2.1"]`

| the cohort | |
| --- | --- |
| notices carrying `00000000-1900` | **7,158** (of ~14.4M, ~0.05 %) |
| id range | 26,200,469 – 31,767,359 |
| distinct content hashes | 7,158 — the `(source, publication_id, content_hash)` identity (`process.rs:6`) has collapsed nothing, it has just degenerated to content-hash alone for this cohort |
| profiles | 4 — `eforms-de-1.1`, `1.2`, `2.0`, `2.1` |
| other zero-prefixed values on `source=doe` | none: a range scan `publication_id >= '00000000-' AND < '00000000.'` GROUP BY returns exactly one value, so the cohort is exactly bounded |

| live in the current ingest — id window 31,700,000–31,767,359 (the 2026-09-13 tick) | |
| --- | --- |
| DÖE notices in the window | 1,395 |
| carrying `00000000-1900` | **9 (0.65 %)** |
| carrying their member stem | 1,386 |
| anything else | 0 |
| newest carrier | 31767359, `ingested_at` 2026-09-13T07:35:11Z, profile `eforms-de-2.1` |

| tender side — `tender_id` 1,400,000–1,600,000 | |
| --- | --- |
| `tender_versions` carrying the placeholder | 158, on 158 distinct tenders |
| `/v1/tenders?publication_id=00000000-1900&limit=200` | 200 items, `more:true` |
| …including tenders whose CURRENT `publication_id` is a real TED number | 316 → `00517500-2024`, 391 → `00772873-2024` — the placeholder also sits inside version chains later joined to a TED-published notice |
| `/v1/tenders?publication_id=a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01` | **0 items** — tender 1499198 cannot be found by its real id |
| control notice 26403505 | serves `920ee1cf-ddd8-41c9-9cb5-aad114899550-01` = its own stem |
| control `/v1/tenders?publication_id=920ee1cf-ddd8-41c9-9cb5-aad114899550-01` | finds tender 2056 |

Mechanism, confirmed in code: `crates/ingest/src/profile.rs:337` elects
`first_text(doc, "NoticePublicationID")` first, ahead of the DÖE fallback, and the comment directly above
it states the design assumption — *"DÖE exports carry neither (`efac:Publication` is TED-side metadata),
so their identity is the notice id plus its declared version … which is also the member file name's
stem"*. `curl -s https://tenders.zebreus.click/v1/notices/26447665/content` shows the source leaf
`DE1-Publication-NoticePublicationID`, scheme `ojs-notice-id`, value `00000000-1900`, sitting beside
`DE1-ID` `a4406a20-3edd-4ddc-921e-fcd05fc6fd5c` and `DE1-VersionID` `01`: the identity the code's own
comment names is in the same file, and loses the election.

One mechanism correction worth keeping, because it points the fix at the right line: the stem form does
NOT come from `publication_id_from_name` (`profile.rs:603`), which parses only the TED
`<number>_<year>.xml` filename and can never produce a `<uuid>-<version>` stem. It comes from the third
fallback `notice_id_and_version` (`profile.rs:590`, root ID + VersionID). Either way the placeholder is
elected ahead of it. `grep -rn '00000000-1900' crates/` → 0: no fixture, no test, nothing covers a DÖE
file carrying the placeholder.

### Why it matters

`publication_id` is the key 217 shipped because it is the key real consumers hold. For this cohort it
answers with the wrong rows in both directions. A consumer looking up the DÖE notice they have in hand —
`a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01` — gets 0 items and concludes tender-db does not have it; a
consumer who reads `publication_id` off a served record and looks it up gets ≥1,000 notices and ≥200
tenders from unrelated procurements under one "official number", including tenders whose current
publication id is a genuine TED number. Nothing in the payload marks the value as a stand-in. And it is
not historical: 0.65 % of every daily DÖE tick still lands in the cohort.

### Why this is ours, not the publisher's

The publisher did write `00000000-1900` — it is the eForms-DE stand-in in `efbc:NoticePublicationID`
before TED assigns an OJS number — but it never published it as this notice's identity. The real
identity, UUID + declared version, is in the same file, is what 1,386 of 1,395 sibling notices in the same
ingest window actually get, and is what `profile.rs`'s own comment and `docs/research/eforms-de-profile.md`
§2 say a DÖE notice's identity is. CONTEXT.md puts publication identity on TED's side, with DÖE's
`efac:Publication` at best a copy. So the system elects a value its own design says is not an identity,
serves it on a public field and keys the identity triple on it. That is a system-introduced mis-keying,
the same conviction 369 reached for a published BT-04 taken verbatim as a group key.

### Repro

1. `curl -s "https://tenders.zebreus.click/v1/notices?publication_id=00000000-1900&limit=1000"` —
   1000 items, `more:true`, all `source=doe`, every `member_path` a different uuid-version file.
2. `curl -s https://tenders.zebreus.click/v1/notices/26447665` — `member_path`
   `a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01.xml`, `publication_id` `00000000-1900`.
3. `curl -s "https://tenders.zebreus.click/v1/tenders?publication_id=a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01"`
   → 0 items, then `curl -s https://tenders.zebreus.click/v1/tenders/1499198` → that tender, serving the
   placeholder. Control: `…?publication_id=920ee1cf-ddd8-41c9-9cb5-aad114899550-01` → tender 2056.
4. `sed -n '330,345p;585,610p' crates/ingest/src/profile.rs` — `NoticePublicationID` elected at :337,
   `notice_id_and_version` at :590, and the comment above :337 naming the intended identity.

### Done when

- The election refuses the all-zero placeholder — either a shape guard on `NoticePublicationID` (an
  all-zero OJS number is not a publication number) or, for `source=doe`, `notice_id_and_version` ahead of
  it. Whichever is chosen is recorded here with its blast radius on non-DÖE profiles.
- A fixture exists: a DÖE member carrying `<efbc:NoticePublicationID schemeName="ojs-notice-id">00000000-1900</…>`
  beside its `DE1-ID`/`DE1-VersionID` folds to `<uuid>-<version>`. Today `grep -rn '00000000-1900' crates/` → 0.
- The 7,158 `notices` rows and the `tender_versions.publication_id` rows carrying the placeholder are
  re-keyed, and **290 is settled before that runs, not after**: `reparse_notice` matches on identity, so a
  derivation shift is exactly the silent `unmatched` no-op 290 describes. The re-key reports a carrier
  count that matches 7,158, or explains the difference.
- `GET /v1/notices/26447665` serves `publication_id` `a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01`;
  `/v1/tenders/1499198` serves the same; `/v1/tenders?publication_id=a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01`
  returns that tender instead of 0 items.
- `/v1/notices?publication_id=00000000-1900` returns 0 items, and `/v1/tenders?publication_id=00000000-1900`
  no longer returns tenders 316 and 391 (whose current ids are real TED numbers).
- The daily re-check is clean: in the newest 100k-id window, DÖE notices carrying the placeholder = 0,
  against 9 of 1,395 on 2026-09-13.
- A recurrence is visible without a human looking: the DQ report counts identity STRINGS repeated across
  N unrelated notices, not only amounts — 366's section-10 sweep is amount-shaped and cannot see this.

## Unit 2 — sdk-0.1 CPV codes are served in four shapes under one `scheme`

### Observed (verified 2026-09-14 on prod)

    curl -s 'https://tenders.zebreus.click/v1/tenders?source=doe&published_after=2025-01-06T00:00:00Z&published_before=2025-01-09T00:00:00Z&limit=1000' | python3 -c "import json,sys,collections,re;it=[i for i in json.load(sys.stdin)['items'] if i['notice_subtype'] is None];print(len(it));print('only2',sum(all(len(c)==2 for c in i['cpv']) for i in it if i['cpv']),'dashed',sum(any(re.fullmatch(r'\d{8}-\d',c) for c in i['cpv']) for i in it),'bare8',sum(any(re.fullmatch(r'\d{8}',c) for c in i['cpv']) for i in it),'none',sum(not i['cpv'] for i in it));print([(i['id'],c) for i in it for c in i['cpv'] if not re.fullmatch(r'\d{8}(-\d)?|\d{2}',c)])"

→ 976 island rows; `only2 304 dashed 370 bare8 232 none 65`; and five rows whose single `code` is several
space-separated CPVs:

    (1542904,'45324000-4  45421146-9 45422000-1'), (1542885,'45421100-5  45421110-8 45421100-5'),
    (1542784,'79413000-2 79416000-3'), (1542728,'45324000-4 45320000-6  45321000-3'),
    (1542599,'45262120-8  45262100-2  45262110-5')

Shape mix of the sdk-0.1 island (`source=doe`, `notice_subtype` null), by window:

| window (island rows) | 2-digit only | dashed | bare 8 | none | glued |
| --- | --- | --- | --- | --- | --- |
| 2025-01-06..09 (976) | 304 (31.1 %) | 370 (37.9 %) | 232 (23.8 %) | 65 (6.7 %) | 5 (0.5 %) |
| 2025-09-01..04 (987) | 302 (30.6 %) | 397 (40.2 %) | 226 (22.9 %) | 60 (6.1 %) | 2 (0.2 %) |
| 2024-03 | 37.8 % | 26.7 % | 28.4 % | — | — |
| 2026-06 | 33.9 % | 45.5 % | 16.3 % | — | — |

The 2025-01 and 2025-09 windows were each re-run independently; the 2024-03 and 2026-06 rows are the
reporter's measurement. Per ENTRY rather than per row in the 2025-01 window: len 2 = 2,024, len 8 = 348,
len 10 = 635, glued 5. The 2025-09 glued rows are 1634789 `'79611000-0 75314000-0'` and 1634393
`'32322000-6 30231310-3 30230000-0'`.

Every other era and source sampled through the same list echo is one shape:

| control | codes | bare 8-digit |
| --- | --- | --- |
| TED eForms, 2025-01-06 window (500 rows) | 1,197 | 1,197 (100 %) |
| TED, 2015-01-06 window (500 rows) | 1,082 | 1,082 (100 %) |
| TED text era, 2004 sample (300 tenders) | 628 | 628 (100 %) |
| `eforms-de` rows INSIDE the same DÖE 2025-01 window | 44 | 44 (100 %) |

So the mix is confined to the sdk-0.1 island, and the same code is served two ways across it:

    curl -s https://tenders.zebreus.click/v1/tenders/1173962   # ted, subtype 1  -> cpv ['09123000']
    curl -s https://tenders.zebreus.click/v1/tenders/1723219   # doe, subtype null -> cpv ['09000000-3','09123000-7']

Inside the island in one 3-day window, 1542903 carries `45421146-9` while 1542574 carries `45421146`.
Within `doe` alone, 182 distinct codes occur both bare and dashed across the 2024-03, 2025-01 and 2026-06
windows (16311100, 16700000, 18100000, 30200000, 31500000 among them); 494 codes across sources.

The 2-digit shape at both scopes:

    curl -s https://tenders.zebreus.click/v1/tenders/1431255 | python3 -c "import json,sys;d=json.load(sys.stdin);print([(x['field'],x['code']) for x in d['classifications'] if x['lot'] is None and x['scheme']=='cpv'])"

→ `main '50'` plus `additional '51'…'98'` (14 two-digit codes). Source-published: notice 26288951's content
carries `code "50"`, `scheme "cpv"` under
`SDK01-ProcurementProject-MainCommodityClassification-ItemClassificationCode`. The glued and dashed shapes
are equally the publisher's text (notice 26544162 publishes `45324000-4  45421146-9 45422000-1` verbatim;
26966109 publishes `09123000-7`).

The documented filter is `c.code LIKE '<cpv>%'` (`crates/store/src/read.rs:899`), so it reaches bare and
dashed alike and the 2-digit codes too — its only failure is the glued rows' 2nd..nth codes:

| `?cpv=45421146`, 2025-01 window | returns 1542903 (`45421146-9`), 1542902 (`45421146-9`), 1542574 (`45421146`) |
| --- | --- |
| 1542904, whose `main` string holds `45421146-9` second | **absent** |
| `?cpv=45324000` (that string's FIRST code) | 9 rows, including 1542904 |

Nothing in the pipeline normalises or splits: `crates/ingest/src/eforms/value.rs:37` emits
`Value::Classification { scheme: "cpv", code: text.into() }` verbatim, `crates/ingest/src/project.rs:240-249`
maps the four SDK01 `ItemClassificationCode` field ids straight to main/additional with no transformation,
`project.rs:3108` copies `code` unchanged into `Fact::Classification`, and `crates/store/src/read.rs:1977`
`split_codes` splits the echo on `,` only, so a space-glued string survives as one entry. No cpv-specific
helper exists anywhere in the tree.

The contract says one vocabulary: `docs.rs:151` — *"CPV code prefix, e.g. `45` (construction)"*;
`sql.rs:674` — *"One of: cpv, nuts."*; the `/docs` #caveats block (`docs.rs:640`) mentions only CPV-2003
/2008 coexistence, nothing about check-digit suffixes, division-level codes or several codes in one string.

One leg is NOT independently re-measured: a shape count over `tender_version_classifications` bounded to a
400-id range hit the 10 s cap (408, not retried) and two follow-ups were refused 503 while the box's
workers were pinned. `/v1/tenders/{id}.classifications` reads the same table and shows the verbatim codes,
so the served side is confirmed; the SQL census belongs in "done when".

### Why it matters

The island is ~656k tenders and the product's stated use is market analysis. SQL-endpoint equality or
`GROUP BY code` splits one CPV code into two strings on 27–45 % of its rows (`'45000000'` and
`'45000000-7'` are different strings), so a spend-by-category read over the island undercounts silently
and gives no hint it did. A cross-source read is worse: the same code answers as `09123000` on TED and
`09123000-7` on DÖE, with one `scheme` label and no shape marker to join on. And on 0.2–0.5 % of island
rows the published extra codes are unreachable through the documented prefix filter while being counted as
one code — `?cpv=45421146` misses tender 1542904, which publishes exactly that code.

### Why this is ours, not the publisher's

All three deviant shapes are the publisher's bytes, verified in the source notices, and the parser is
faithfully verbatim. The defect is what this system then does with them: it labels a multi-code string as
one `code`, and it serves one CPV code under two spellings inside one `scheme` with no marker, in a schema
whose every other era and source is bare 8-digit. The board's own precedent puts source-dialect variety on
the fold's side of the line — 292 in terms ("each new source adds a dialect unless a normalization layer
exists"), 172/ADR-0014's currency alias map, 319's country column — and the CPV check digit is derivable
from the 8 digits, so stripping it loses nothing. 231 closed this era's CPV on presence (93.8 %) and never
looked at representation; 172's open half is vintage, not shape; 171's caveats line covers only 2003/2008.

### Repro

1. Run the first curl above (`source=doe`, 2025-01-06..09, `limit=1000`) — 976 island rows,
   `only2 304 dashed 370 bare8 232 none 65`, and the five glued ids.
2. `curl -s https://tenders.zebreus.click/v1/tenders/1173962` vs `…/1723219` — `09123000` against
   `09123000-7` for the same code, different sources.
3. `curl -s https://tenders.zebreus.click/v1/tenders/1431255` — `main '50'`, `additional '51'…'98'`.
4. `curl -s 'https://tenders.zebreus.click/v1/tenders?source=doe&published_after=2025-01-06T00:00:00Z&published_before=2025-01-09T00:00:00Z&cpv=45421146&limit=1000'`
   — 1542903, 1542902, 1542574; 1542904 absent. Then `?cpv=45324000` → 9 rows including 1542904.

### Done when

- A CPV normalisation step exists at the fold boundary: strip a trailing `-<digit>` after 8 digits, and
  split a whitespace-separated multi-code string into separate classification rows. It lives beside
  `normalize_lang` (292), not in `read.rs` — a display-side split would be the second-implementation shape
  366 unit 3 retired, and `split_codes` (`read.rs:1977`) splitting on `,` only is where that would land.
- Fixtures pin all four shapes through the fold: bare `45421146`, dashed `45421146-9`, division `50`, and
  the glued `'45324000-4  45421146-9 45422000-1'` → three rows.
- The sdk-0.1 island is refolded (precedented on exactly this era — jobs 917/918 and 388/389), and the
  before/after shape counts for both measured windows are recorded here.
- `?cpv=45421146` returns 1542904; `/v1/tenders/1723219` and `/v1/tenders/1173962` serve the same string
  for the same code; dashed and glued are 0 of island rows in the 2025-01-06..09 and 2025-09-01..04
  windows; the 182 codes that today occur in both shapes within `doe` collapse to one spelling each.
- The 2-digit division codes are settled as a DECISION and written down either way: they are valid CPV
  division prefixes the publisher wrote and the prefix filter already matches them, so the choice is
  pad-or-mark versus a `/docs` #caveats line beside the CPV-2003/2008 one (171's deliverable). This issue
  records which, rather than leaving 31 % of the island's rows undescribed.
- The `tender_version_classifications` / `v_tender_classifications` shape census is run once the box is not
  pinned and its numbers recorded here — the 408/503 leg above is the one claim not independently measured.
- ADR-0004's per-profile mapped-or-ignored checklist for `sdk-0.1` names CPV representation, so the next
  source's dialect cannot arrive unnormalised without a decision.


## Unit 1 GUARD BUILT 2026-09-16 — the election refuses the placeholder; the re-key still to run

Status: unit 1 half done. The derivation is fixed and gated; the 7,177 standing carriers are NOT yet
re-keyed. Unit 2 (the sdk-0.1 CPV shapes) is untouched.

### The decision, with its blast radius

The `## Done when` offered a shape guard on `NoticePublicationID` or, for `source=doe`,
`notice_id_and_version` ahead of it. **Taken: the shape guard.**

`is_placeholder_ojs_number` (`crates/ingest/src/profile.rs`) refuses an id whose number half is ALL
ZEROS and whose year half is all digits; `dispatch_eforms` filters the `NoticePublicationID` leg
through it, so the election falls through to the file-name stem and then to
`notice_id_and_version` — the identity the code's own comment already named.

Why the guard and not the reordering: the reordering fixes these rows and leaves the next publisher
that emits a placeholder to be found the same way, one census at a time. **The placeholder is what is
wrong, whoever emits it.**

Blast radius, measured corpus-wide on 2026-09-16 rather than assumed (the filing measurement was
scoped to `source=doe`):

    SELECT source, publication_id, count(*) FROM notices
     WHERE publication_id >= '00000000-' AND publication_id < '00000000.'
     GROUP BY source, publication_id
    -> [["doe", "00000000-1900", 7177]]

**Exactly one value across all 14.4M notices, all of it `doe`.** Nothing outside the cohort is
touched. (7,158 when 394 was filed on 2026-09-14 — the cohort was still growing at the measured ~9
per 1,395 DÖE notices per tick, which is itself the daily re-check the `## Done when` asks for, and
it will read 0 once this is deployed.)

The predicate is written for ANY all-zero number half, not only the 8-digit spelling that was
measured, so a shorter one cannot slip past later. A probe for the short spellings was attempted and
returned 408 (a `LIKE` forces a scan); per `docs/agents/prod-box-reads.md` a 408 is never retried, so
the claim above is bounded to what the range scan actually measured and the predicate is written
wider than the measurement rather than narrower.

### The fixture the issue asked for

`grep -rn '00000000-1900' crates/` returned 0. It now returns the fixture and its test.
`crates/ingest/tests/fixtures/doe/eforms-de-1.1-cn-placeholder-pubid-7d69b0f7.xml` is the real
`eforms-de-1.1` member with the live cohort's element inserted where the cohort carries it —
`<efac:Publication><efbc:NoticePublicationID schemeName="ojs-notice-id">00000000-1900</…>` inside the
eForms extension — beside its own `<cbc:ID schemeName="notice-id">` and `<cbc:VersionID>`.

`a_placeholder_publication_number_loses_to_the_notices_own_id` (`crates/ingest/tests/doe.rs`) asserts
it dispatches to `7d69b0f7-2605-448f-9495-676458dcddc2-01`, and that the SAME member without the
placeholder keys identically — so the guard changed nothing about how a DÖE notice is identified, it
only stopped one value from winning. Run red first: it failed with
`left: "00000000-1900"`, the live symptom exactly.

`only_an_all_zero_ojs_number_is_a_placeholder` pins the shape, and pins the narrowness that makes it
safe for every source: `00001505-2024` and `00000001-2024` are admitted (real TED numbers are
zero-PADDED, which a lazier `starts_with("0000")` guard would have eaten — 7.3M notices look like
that), DÖE stems are not OJS numbers in either direction, and a missing or non-numeric year half is
not this rule's business.

### What is still owed on unit 1

- **The re-key of the 7,177 standing carriers.** Now unblocked: the `## Done when` said "290 is
  settled before that runs, not after", and **issue 290 is RESOLVED as of today** — `reparse_notice`
  falls back to `(source, content_hash)` when the triple misses and that hash names exactly one
  notice, ADOPTS the new `publication_id`, and reports the count as `re-keyed` in the job summary.
  The cohort has 7,177 distinct content hashes, so every carrier satisfies the uniqueness condition.
  The re-parse re-keys them and the summary's `re-keyed` count can be **checked against 7,177 rather
  than assumed** — which is exactly what the `## Done when` asked for ("reports a carrier count that
  matches 7,158, or explains the difference"; it is 7,177 now, and the drift is the daily tick).
- The live acceptance reads: `/v1/notices/26447665` and `/v1/tenders/1499198` serving
  `a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01`; `?publication_id=00000000-1900` returning 0 on both
  collections; tenders 316 and 391 no longer answering it; the newest-100k daily re-check at 0.
- The DQ report's repeated-identity-string sweep (the "recurrence is visible without a human looking"
  bullet) — not started.

## Unit 1 RE-KEY IN PROGRESS 2026-09-16 — 701 of 7,177 done, the mechanism proven

Deployed on rev `6612b2b`; the re-key runs as an ordinary `reparse` over the four carrier profiles,
`reclaim_only: true` so the era folds once at the end rather than after every chunk.

**Job 1388 (3 packages, the probe):**

    re-parsed 37145 notices across 3 packages (64549 members walked, 0 unmatched, 701 re-keyed,
    5 now failing and left untouched); stamped 246265 tender(s) epoch-stale; 85 package(s) held
    back — continue with {"after": 403} — NOTE: 701 re-keyed by content hash, so this run CHANGED
    publication_id derivation (issue 290). Intended for a re-key run; a regression otherwise

The `## Done when` asked for a re-key that "reports a carrier count that matches 7,158, or explains
the difference". It reports one, and it is checkable against the corpus rather than trusted:

| | |
| --- | --- |
| carriers before | **7,177** |
| `re-keyed` reported by the job | **701** |
| carriers after | **6,476** |
| 7,177 − 701 | **6,476** ✓ |

`0 unmatched` is the number that says issue 290's fix is load-bearing: without the content-hash
fallback all 701 would have been counted benign and kept their old parse.

**Job 1389 (45 packages, `after: 403`) is running.** Remaining after that: ~40 packages.

### The acceptance is that the count reaches ZERO

Not "a small number". A residue would mean some carriers are being skipped rather than re-keyed —
and per issue 290's own note, a DISPATCH-level failure is invisible to the walk's counters, so the
cohort count is the only detector. Re-run
`SELECT count(*) FROM notices WHERE publication_id = '00000000-1900'` after the last chunk; if it is
not 0, do not close this, investigate the residue.

Then the live reads still owed: `/v1/notices/26447665` and `/v1/tenders/1499198` serving
`a4406a20-3edd-4ddc-921e-fcd05fc6fd5c-01` (the tender half needs the fold);
`?publication_id=00000000-1900` returning 0 on both collections; tenders 316 and 391 no longer
answering it; the newest-100k daily re-check at 0.

### Noticed in passing, not this issue's

`5 now failing` per 3 packages — DÖE members the CURRENT eForms parser quarantines while a stored
parse exists. They are parse-level (the publication-id guard is a dispatch-stage decision and cannot
reach the parser), so they are pre-existing and nothing was lost — the re-parse leaves their stored
layer alone by design. Worth a census once the re-key is done, since nobody has looked at DÖE's
parse-failure residue and this run is the first thing to count it.

### Chunk 2 done, 281 carriers left (2026-09-16)

**Job 1389 / internal 2298:**

    re-parsed 401228 notices across 45 packages (764302 members walked, 0 unmatched, 6195 re-keyed,
    3 now failing and left untouched); stamped 246265 tender(s) epoch-stale; 40 package(s) held
    back — continue with {"after": 477} — NOTE: 6195 re-keyed by content hash …

The arithmetic keeps closing exactly:

| | |
| --- | --- |
| carriers at the start | 7,177 |
| chunk 1 re-keyed | 701 |
| chunk 2 re-keyed | **6,195** |
| carriers now | **281** |
| 7,177 − 701 − 6,195 | **281** ✓ |

`0 unmatched` on both chunks. **Job 1411 is enqueued for the final 40 packages**, this time WITHOUT
`reclaim_only`, so job 1412 folds it — the era's last chunk pays for its own projection rather than
leaving the corpus with un-projected notices.

The acceptance stands: the cohort must reach **0**, not 281 or any other small number. A residue
would mean carriers are being skipped rather than re-keyed, and per issue 290's own note a
DISPATCH-level failure is invisible to the walk's counters, so this count is the only detector.

`now failing` is 3 this chunk against 5 in three packages last chunk — it scales with packages and
not with carriers, which is what "pre-existing parse-level residue" looks like and is the second
reason to believe the guard is not causing it.
