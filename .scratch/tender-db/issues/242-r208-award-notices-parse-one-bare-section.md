# 242 — r2.0.8 contract-award notices that parse to a single bare `Notice` section, so their results never exist

Status: RESOLVED 2026-08-19 — not a parser gap. Diagnosed from the archive, report fixed to say so.
Kind: publication-quality finding + a metric that could not express it (was filed as: parser gap)
Blocked by: — (the corpus-scale split lands with 235's deployed run)
Relates to: 235 (the metric that can now see this), 10 (r208 profile), 139 (r209 DTD strip — a
neighbouring shallow-parse class), 174/177 (r208 fields that never projected), 13 (results layer)

## What

The repaired results-materialisation metric was run over one 83k-version window (tender_id
6.00–6.04M) and r2.0.8 read **4,250 award notices, 4,188 with results — 98.5 %**, the first
sub-100 % reading section 3 has ever produced. The 62-version shortfall is not a projection
failure:

    gap_versions | parsed_a_result_section
              62 | 0

**None of the 62 parsed a result section at all**, so the projection had nothing to write and the
old metric excluded them from both halves. This is precisely the failure class issue 235 was about.

## What the 62 look like

Sampled 28 of them (same window, narrowed to tender_id ≤ 6.02M):

| `TD_DOCUMENT_TYPE` | `TED-FORM` | count |
|---|---|---|
| `7` (contract award) | *absent* | 26 |
| `7` (contract award) | `6` (F06 utilities award) | 2 |

And every sampled notice has exactly ONE section, of kind `Notice`:

    publication_id | member_path                   | sections | kinds
    339168-2017    | 20170830_165/339168_2017.xml  | 1        | Notice
    342936-2017    | 20170901_167/342936_2017.xml  | 1        | Notice
    342942-2017    | 20170901_167/342942_2017.xml  | 1        | Notice

So: a notice that declares itself a contract award, carries its `TD` code and its PROCEDURE-level
codes, and then has no structural sections beyond the root shell. The award data is in the source
document; the parse layer has none of it.

Two sub-classes, and they may have different causes:

1. **No `TED-FORM` recorded (26 of 28).** The form is what the r208 parser routes on (issue 177's
   `VALUE_COST` work is form-aware), so a notice with no recorded form plausibly falls through every
   form-specific extraction and keeps only the root. Whether the source really has no form element,
   or the parser failed to record one it has, is the first thing to check.
2. **`TED-FORM = 6`, F06 utilities contract award (2 of 28).** A form that IS recorded and IS an
   award form, yet produced no result section — so this one is not explained by the missing-form
   story and points at the r208 award-block extraction not covering F06.

Note the dates: these are **2017 publications sitting in the `ted-export-r208` profile**, which is
worth a look on its own — r2.0.9 is the 2014+ export. Either the profile assignment is right and
r208 legitimately carried 2017 documents, or these notices are mis-profiled and the missing
extraction is a consequence rather than the cause.

## Reproduce

    -- the 62 (bounded window, ~10 s: keep the window this size or smaller)
    WITH v AS (
      SELECT tv.tender_id tid, tv.caused_by_notice_id nid
        FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id
       WHERE tv.tender_id > 6000000 AND tv.tender_id <= 6040000
         AND n.profile = 'ted-export-r208'
         AND EXISTS(SELECT 1 FROM notice_codes c
                     WHERE c.notice_id = tv.caused_by_notice_id AND c.section_id = 'PROCEDURE'
                       AND c.field_id = 'TED-TD_DOCUMENT_TYPE' AND c.code IN ('7','J','K','R','V'))
         AND NOT EXISTS(SELECT 1 FROM lot_results lr
                         WHERE lr.tender_id = tv.tender_id AND lr.notice_id = tv.caused_by_notice_id))
    SELECT (SELECT COUNT(*) FROM v) AS gap_versions,
           (SELECT COUNT(*) FROM v
             WHERE EXISTS(SELECT 1 FROM notice_sections s
                           WHERE s.notice_id = v.nid AND s.kind IN ('LotResult','TenderResult'))) AS parsed_a_result_section

`339168-2017` and `342936-2017` are concrete cases to pull from the archive and parse locally.

## Steps

1. Fetch one of the named notices from its package member and parse it with the r208 profile locally.
   Establish which of the two stories is true: no form element in the source, or a form the parser
   drops.
2. Check the profile assignment for 2017 r208 documents — if these are mis-dispatched, fix that first,
   because the extraction gap may vanish with the right profile.
3. Whichever it is, add the era's award extraction (or the dispatch fix) with a fixture from one of
   the named notices, then reprocess the affected notices.
4. **Corpus scale is not yet known**: 62 in one 83k-version window says nothing reliable about the
   whole era (1.07M versions). Issue 235's deployed run measures every window and its section 3
   output IS the corpus count — read it before sizing a reprocess.

## Why this matters beyond 62 notices

r2.0.8 holds 1.07M versions. If the shortfall rate holds, that is ~15k award notices whose winners,
values and contract dates exist in the published document and in no queryable form here. And the only
reason anyone can see them is that the denominator stopped being drawn from the projection's own
output — which is the argument for keeping that property in every metric the report grows.


---

## RESOLVED (2026-08-19, owner) — the premise was wrong, and the fix is in the report

The filing above assumed the 62 shortfall versions were award data we failed to extract. Chased to
the primary source, they are award notices that **publish no award data at all**. Nothing to extract,
nothing to fix in the parser — and the report now says which half of any gap is ours.

### What the archive says

**1. The no-form class (26 of 28 sampled) publishes prose.** `339168-2017`, pulled from
`/data/archive/ted/monthly/2017-08.tar`:

- namespace `ted/R2.0.8.S03/publication`, `COMMENTS: From Convertor` — so the r2.0.8 profile
  assignment is CORRECT for these 2017 documents; the "mis-dispatched?" question in the filing is
  answered, no.
- `<TD_DOCUMENT_TYPE CODE="7">Contract award notice</TD_DOCUMENT_TYPE>` — it really does announce an
  award, in its own words.
- `AC_AWARD_CRIT CODE="Z"` ("Not specified"), `TY_TYPE_BID CODE="9"` ("Not applicable").
- The body is `OTH_NOT` / `FD_OTH_NOT`: 24 language versions of `BLK_BTX`/`<P>` paragraphs and no
  structured award element anywhere. There is a `FORM_SECTION` but no `FORM` attribute, which is
  exactly why no `TED-FORM` code is recorded — the absence IS the signal, not a parser miss.

The winner and value exist only inside prose, in 24 languages. Out of scope for anything short of NLP.

**2. The F06 class (2 of 28) publishes an EMPTY award container.** `017037-2017` carries
`<AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES/>` — self-closing. And the wrapper is not unmapped:

- `AWARD_CONTRACT_CONTRACT_AWARD_UTILITIES` is r208-only (XSD inventory) and sits in `Rule::Group`,
  which looks like the bug. Its CHILD, `AWARD_AND_CONTRACT_VALUE`, is already in the r208
  `Section(Kind::LotResult)` group.
- Verified against prod: `002856-2017` (populated container) HAS one `LotResult` section, with
  `CONTRACT_NO`, `DATE_OF_CONTRACT_AWARD`, the contractor org ref, and the awarded value
  17,850,000 PLN kept distinct from the 18,000,000 PLN pre-award estimate. `017037-2017` (empty
  container) has none.
- In `2017-01.tar` the container is populated 73 times and empty 13. Promoting the wrapper to a
  section would nest a second, empty result section inside every populated F06.

Both notices are now fixtures (`f06-002856-2017.xml`, `f06-017037-2017.xml`) with tests that pin the
transparent-wrapper decision and the empty-container behaviour, so the next reader who finds a
sub-100 % density does not "fix" the parser. Both pass the ADR-0004 exhaustiveness sweep.

**3. Nothing else is affected.** Grouping the r2.0.8 shortfall by form, over two 200k-notice windows:
`form 6 → 465 / 21`, `no form → 425 / 307`, `T02 → 0 / 3`, and nothing else. TD codes `J`, `K`, `R`,
`V` show zero shortfall. r2.0.9 shows zero in a 100k slice — its award blocks use the plain
`AWARD_CONTRACT` element, which is mapped, which is why the gap is r2.0.8-only. The remaining
`Rule::Group` names that look like award blocks (`AWARD_CONTRACT_MOVE`, `AWARD_CONTRACT_PI_MOVE`,
`AWARD_NOTIFICATION`, `AWARD_PRIZES`) cost nothing measurable today — they would have surfaced as
another form code in that breakdown. Left alone deliberately, unmeasured rather than guessed at; the
pre-2014 monthly tars nest per-day ZIPs, so a raw scan for them needs a different probe than the one
used here.

### The real defect was in the metric, and it is fixed

Section 3 could report "98.5 %" without being able to say whether that 1.5 % was a publisher shipping
prose or us dropping data. It now measures a third number, `awards_barren` — award-typed versions
whose notice parsed with no result block at all — and prints both halves:

    era                     award-notices   with lot_results   density   no content pub.
    TED_EXPORT r2.0.8               4,250              4,188     98.5%                62
    no content published: 62 award notice(s) parsed with no result block at all …
    Unmaterialised award notices that DID publish a result block, i.e. the projection's
    own shortfall: 0.

JSON carries `no_award_content` and `unprojected` per era. On the window that started this issue,
**the projection's shortfall is zero** — every r2.0.8 award notice that published a result block
projected one.

This deliberately reads `notice_sections`, which issue 235 forbade for the DENOMINATOR. The rule it
keeps: a denominator must not derive itself from the projection's own output. Comparing the published
type against the parse is not that — the comparison is the finding.

### What is left

Nothing here. The corpus-scale split (how many award notices publish nothing, per era) arrives with
issue 235's deployed run, and belongs to that issue's first-run readout rather than to this one.
