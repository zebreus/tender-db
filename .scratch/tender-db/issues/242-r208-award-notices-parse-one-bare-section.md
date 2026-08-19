# 242 — r2.0.8 contract-award notices that parse to a single bare `Notice` section, so their results never exist

Status: needs-triage — found 2026-08-19 by the repaired section 3 (issue 235), on the first window that could express it
Kind: parser gap (award data present in the source, absent from the parse layer)
Blocked by: 235's deploy (for the corpus-scale count; the defect itself is confirmed)
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
