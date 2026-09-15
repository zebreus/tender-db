# 397 — text-era titles are served with the source's hard line break inside them, and in 1993–1994 with the OJ boilerplate footnote `(Only the original text is authentic)`

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (ingest — the text-era `TI` rule in `crates/ingest/src/text/rules.rs:104` and the newline join in `crates/ingest/src/text/parse.rs:1344`, served verbatim through `crates/ingest/src/project.rs:144`; the current value is pinned by `crates/ingest/tests/text.rs:209`)
Relates to: 199 (RESOLVED 2026-08-15 — the SAME TED ~72-column wrapper, and it already classifies the wrapper's output as layout: "all but 4 are TED's own line-wrapper emitting a wrapped tail flush-left". It fixed only column-0 orphan lines in `unclaimed-content`; it never touches the newline that sits INSIDE `TI`'s own value, which is where this one lives), 368 (ready-for-agent — unmapped source vocabulary; it names `TXT-TI` twice, at lines 294 and 612, but only as a mapping question — "which element becomes the title", never what the title string contains), 343 (FIXED 2026-09-02, DEPLOYED 2026-09-03 — the fold-vs-read tie-break) and 292 (FIX DEPLOYED 2026-08-26 — the inert English pick) and 340 (CLOSED — the original-language leg): all three decide WHICH title is picked; this is the content of the one that wins, so none of them can catch it, 364 (unit 6 done 2026-09-13 — its units 5–6 re-parsed r208 and re-folded the text era, which is exactly the follow-on this fix needs and shows it is a routine operation), 11 (resolved — the text-era profile), 232 (the text-era buyer/value/winner campaign whose sweeps a text-era re-parse would ride), `docs/research/ted-legacy-mapping.md` §7, which is headed "Text era (1993–2010) — quick assessment only" and records no decision to preserve wraps in a title

## Observed (verified 2026-09-14 on prod)

```
curl -sS 'https://tenders.zebreus.click/v1/tenders?published_after=1993-03-05T00:00:00Z&published_before=1993-03-06T00:00:00Z&limit=100'
curl -sS https://tenders.zebreus.click/v1/notices/17424/content
```

The list page for 1993-03-05 returns 100 items, **66 of whose `title` strings contain a literal `\n`**:

```
8037963  "D-Herzogenrath: sewage-treatment plant\n(Only the original text is authentic)"
8040731  "F-Lyons: batteries\n(Supply contract - Only the original text is authentic)"   (1994-03-05)
8053406  "D-Naumburg: general construction work for buildings (new work and\nrenovation)"  (1996-03-05)
8155044  "NO-Tromsø: architectural, engineering, construction and related technical\nconsultancy services"  (2005-03-05)
```

`GET /v1/tenders/8037963` serves the same string, so this is not a list-layer artefact. Further carriers: 7969382 (the GATT continuation line), 8080589 and 2436807 (1999-03-05), 8102091, 8216629.

Per-day API samples, `limit=100`:

| published day | titles with `\n` | ending in `(… Only the original text is authentic)` | ending in `(with participation by GATT countries)` |
| --- | --- | --- | --- |
| 1993-03-05 | 66 | 59 | 7 |
| 1994-03-05 | 59 | 50 | — |
| 1996-03-05 | 5 | 0 | 0 |
| 1999-03-05 | 2 | 0 | 0 |
| 2002-03-05 | 10 | 0 | 0 |
| 2005-03-05 | 11 | 0 | 0 |
| 2008-03-05 | 3 | 0 | 0 |
| 2010-03-05 | 0 | 0 | 0 |
| 2015-03-05 | 0 | 0 | 0 |
| 2024-03-05 | 0 | 0 | 0 |

The stored column carries it, not just the read layer:

```
ssh … root@zebreus.click 'echo "SELECT COUNT(*) AS tenders, SUM(instr(current_title, char(10)) > 0) AS title_with_newline, SUM(current_title LIKE \"%original text is authentic%\") AS title_with_footnote, MIN(strftime(\"%Y\", current_published_at, \"unixepoch\")), MAX(strftime(\"%Y\", current_published_at, \"unixepoch\")) FROM tenders WHERE id BETWEEN 7960000 AND 8059999" | /root/sq.sh'
```

| id range | era spanned | tenders | `current_title` with `\n` | with the authenticity footnote |
| --- | --- | --- | --- | --- |
| 7,960,000–8,059,999 | 1993–2026 | 99,741 | **11,769 (11.8%)** | **8,579** |
| 8,100,000–8,199,999 | 2001–2021 | 100,000 | **8,671 (8.7%)** | 0 |
| 8,060,000–8,099,999 | 1996–2011 | 40,000 | 2,412 (6.0%) | 0 (27 carry the GATT line) |
| 5,000,000–5,099,999 (control, XML/eForms) | 2013–2024 | 100,000 | **4 (0.004%)** | 0 |

The four control-range rows (5076998, 5088993, 5095378, 5098520) are publisher-written multi-line titles carrying `\r\n`. They are source-published and must survive any fix; the text-era `\n` is not.

It is a fixed-width wrap, not content: on the 2005-03-05 page the wrapped first lines are 65–74 characters and the longest unwrapped title is 72.

The newline is already in the parse layer, not introduced at fold time. `GET /v1/notices/17424/content`, section `PROCEDURE`:

```
field_id TXT-TI  value "F-Paris: lighting supports\n(Only the original text is authentic)"
```

Mechanism, three lines of code:

| file:line | what it does |
| --- | --- |
| `crates/ingest/src/text/rules.rs:104` | `("TI", Prose(Some("EN")))` — the title is parsed as a prose body |
| `crates/ingest/src/text/parse.rs:1344` | `let text = lines.join("\n");` — `Rule::Prose` joins continuation lines with a newline |
| `crates/ingest/src/project.rs:144` | `("TXT-TI", "title")` — `TXT-TI` becomes `title` with no transform |

## Why it matters

A consumer reading `title` gets a field whose shape depends on which era the tender comes from, and nothing in the served contract warns them: `openapi.json` documents `Tender.title` as a plain nullable string, and neither `CONTEXT.md` nor `/docs` says a title can contain a newline or an OJ footnote. So:

- **Display** breaks at an arbitrary column in ~6–12% of text-era titles, and in 50–66% of 1993–1994 ones the visible title is two lines of which the second is boilerplate.
- **Sorting and prefix search** compare against a string with `\n` and a footnote in it — `"D-Herzogenrath: sewage-treatment plant\n(Only the original text is authentic)"` sorts and matches differently from the title it renders as.
- **Dedup on title** never matches an equivalent title from another era, because 8,579 rows in one 100k-id range end in the same 36-character constant and the eForms/XML eras end in none.
- The constant is pure noise for anyone counting distinct titles: the same footnote is the tail of thousands of otherwise unrelated procurements.

No fact is lost or wrong — this is a representation defect, which is why the severity is medium rather than high.

## Why this is ours, not the publisher's

The source delivered a hard-wrapped tagged-text line with an indented continuation, and this system decided what to do with it: `rules.rs:104` chooses `Prose` for `TI` and `parse.rs:1344` joins with `\n`. The same parser already documents the wrap as a transport artefact one screen earlier — `parse.rs:586-587`, "TED's wrapper breaks both the heading and the value at ~72 columns" — and carries a `flatten()` helper at `parse.rs:810` ("Flatten TED's ~72-column wrap into one line") that undoes it for facts derived from `TX`. The headline title simply never goes through it. Issue 199 reached the same verdict about the same wrapper and fixed it only for column-0 orphan lines. And the footnote is the OJ's own authenticity notice printed under every 1993–1994 title, not title text — the pinning test's own comment calls it "the authenticity note continuation" (`tests/text.rs:209`). The publisher published a title and a notice under it; this system concatenated them.

## Repro

Under two minutes, no box access needed for steps 1–2.

1. `curl -sS 'https://tenders.zebreus.click/v1/tenders?published_after=1993-03-05T00:00:00Z&published_before=1993-03-06T00:00:00Z&limit=100' | jq '[.items[]|select(.title|contains("\n"))]|length'` → **66**; swap the day for 1994-03-05 → 59, 2005-03-05 → 11, 2015-03-05 → 0.
2. `curl -sS https://tenders.zebreus.click/v1/tenders/8037963 | jq -r .title` → the two-line string, so the detail route agrees with the list; `curl -sS https://tenders.zebreus.click/v1/notices/17424/content | jq '.sections[].values[]|select(.field_id=="TXT-TI")'` → the newline is already in the parse layer.
3. The two `SELECT`s above over ids 7,960,000–8,059,999 and 8,100,000–8,199,999 → `[99741, 11769, 8579, "1993", "2026"]` and `[100000, 8671, 0, "2001", "2021"]`. Both are bounded id-range counts over a metadata column, so they are free reads under `docs/agents/prod-box-reads.md`; run them against a snapshot if the serving DB is busy.
4. `grep -n 'TI", Prose' crates/ingest/src/text/rules.rs` → line 104; `sed -n '1344p' crates/ingest/src/text/parse.rs` → `let text = lines.join("\n");`; `sed -n '209p' crates/ingest/tests/text.rs` → the assertion pinning the current value.

Caveat carried forward for whoever fixes it: `crates/ingest/tests/text.rs:209` asserts the two-line string today, so the join is a deliberate parser choice and the fix is red-first against that test, not a bug-fix against an accident.

## Done when

- `TI` no longer joins with `\n`: wrapped continuation lines are space-joined (in the rule, or by routing the title through `parse.rs:810`'s `flatten()`), and `crates/ingest/tests/text.rs:209` is re-pinned red-first to the one-line value.
- The two OJ annotation families are decided explicitly and the decision is written down: `(Only the original text is authentic)` / `(Supply contract - Only the original text is authentic)` is boilerplate and leaves the title; `(with participation by GATT countries)` is substantive and is kept as a flag rather than as title text, not silently dropped with the other.
- `GET /v1/notices/17424/content` serves `TXT-TI` as `F-Paris: lighting supports` with no newline and no footnote.
- After the text-era re-parse and re-fold (the 364 units 5–6 shape): the `SELECT` over ids 7,960,000–8,059,999 returns `title_with_newline` 0 and `title_with_footnote` 0, and over 8,100,000–8,199,999 returns `title_with_newline` 0.
- The control range is untouched: ids 5,000,000–5,099,999 still shows 4 newline titles, and 5076998 / 5088993 / 5095378 / 5098520 still carry their publisher-written `\r\n`.
- `curl` on 1993-03-05 `limit=100` returns 0 of 100 titles containing `\n`; the 8037963 title reads `D-Herzogenrath: sewage-treatment plant`.
- The CPV and `TX`/`AB` bodies keep their wraps — this change touches `TI` only, and a test asserts a `TX` body still carries its newlines.
- `openapi.json` / `/docs` say what `Tender.title` is now guaranteed to be (single line), or a gate asserts it, so the guarantee is not just true but stated.
