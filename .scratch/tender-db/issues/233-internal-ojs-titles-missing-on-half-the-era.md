# 233 — INTERNAL_OJS 2008 carries a title on only 43.7 % of its versions

Status: FIXED in code 2026-08-19 (38ae9bc) — needs the era re-folded (26,955 notices) to materialise
Kind: projection mapping gap, one small era
Blocked by: —
Relates to: 187 (INTERNAL_OJS awards 100 % unchained — same era, the linkage half), 193/194
(internal-ojs alias work), 230 (the measurement that found this), 58 (the legacy OJS adjacency
machinery, same era's chains)

## What

From the first full-corpus data-quality measurement (job 731):

    era                  versions   title  buyer  value    cpv deadline winner
    INTERNAL_OJS 2008      26,955   43.7%  91.5%  24.3% 100.0%    79.4%   36.1%

Every other era in the report measures ≥ 96 % title; most measure 100.0 %. This one measures
**43.7 %** — so roughly 15,000 of 26,955 versions have no canonical title at all, while the same
notices carry a buyer 91.5 % of the time and a CPV 100 % of the time. A notice whose CPV and
buyer both resolved is a notice that parsed and projected; the title specifically is being lost.

## Why it is worth a look despite the small volume

The era is small (0.2 % of the corpus) but the shape is diagnostic rather than cosmetic: a
notice with a buyer and a CPV and no title is not a thin source record, it is a mapping miss on
one field. INTERNAL_OJS is a single-year 2008 island with its own dialect, and the likely cause
is the same class as issues 29 and 177 — a title element under a stem the projection's `TEXTS`
table does not know, present in some document shapes and not others (which is what a 43.7 %
rather than 0 % share suggests: one form family maps, another does not).

That also makes it cheap to diagnose: compare a titled version against an untitled one from the
same era and the differing element name is the answer.

    -- both bounded, both indexed; run against a snapshot, not the serving DB
    SELECT tv.tender_id, tv.seq, tv.caused_by_notice_id
      FROM tender_versions tv JOIN notices n ON n.id = tv.caused_by_notice_id
     WHERE n.profile = 'internal-ojs'
       AND NOT EXISTS (SELECT 1 FROM tender_version_texts t
                        WHERE t.tender_id = tv.tender_id AND t.seq = tv.seq AND t.field = 'title')
     LIMIT 5;

Then read those notices' `notice_sections` beside a titled one's.

## Acceptance

- The element/stem carried by untitled internal-ojs notices is identified and recorded here.
- Either mapped (with an era unit test), or recorded as genuinely title-less source records —
  a documented 43.7 % that is correct beats an undocumented one that looks like a bug.
- After a re-projection, the data-quality report's `INTERNAL_OJS 2008` title share reflects
  whichever answer was true.


---

## Diagnosed and fixed (2026-08-19, owner) — the titles were never lost; those notices have none

Answered entirely from the committed fixtures, no prod reads needed. Of the six internal-ojs
fixtures, **two carry `TITLE_CONTRACT` and four do not** — 33 %, which is the corpus-wide 43.7 % in
miniature, and it says the split is by FORM FAMILY, not by mapping luck.

`114238_2008.en` is the shape that has no title:

    <INTERNAL_OJS HEADING="02A0"> … <NAT_NOTICE>G</NAT_NOTICE> … <EEIG LG="EN" CATEGORY="TRANSLATION">

An **EEIG registration** — a European Economic Interest Grouping notice, not a procurement form.
There is no `TITLE_CONTRACT`, no `TITLE`, no title element of any kind in the document. The guess in
the filing above ("a title element under a stem the projection's `TEXTS` table does not know") was
wrong: there is no stem to add.

What every notice in the era DOES carry is the heading the Official Journal published it under:

    <TI_DOC><P>NL-Amsterdam: Eurys Consult EESV</P><P>2008/S 85-114238</P></TI_DOC>

parsed as two `TED-TI_DOC` text values. So the fix is a projection fallback, not a mapping row:
when a notice maps no title of its own, take the OJ heading.

### Two rules, and the second one is the interesting one

- **Fallback only.** It fires only when no `title` fact was mapped, so the 44 % of the era that do
  publish a title are untouched — and so are r2.0.x and eForms, which carry `TI_DOC` too and would
  otherwise gain a second, competing title on millions of versions. That is also why `TED-TI_DOC` is
  deliberately NOT added to the `TEXTS` table, where it would have been a one-line change.
- **Never the reference paragraph.** The second `<P>` is `2008/S 85-114238` — `NO_DOC_OJS` again. A
  positional "first paragraph wins" rule reads correctly on this notice and would eventually title a
  tender `2008/S 85-114238`; the paragraph is excluded by comparing it against the notice's own id
  instead. Both halves are asserted.

### Tests

- `a_notice_with_no_title_element_takes_the_oj_heading` — the EEIG notice projects
  `NL-Amsterdam: Eurys Consult EESV`, and no title anywhere matches `2008/S%`.
- `the_oj_heading_never_displaces_a_published_title` — `115908_2008.en` keeps its
  `TITLE_CONTRACT`, ends with exactly ONE tender-level title, and the heading is still retrievable in
  the notice layer.
- The full projection suite (36 tests), the golden fold and the equivalence check all pass unchanged.

### Left to do

Re-fold the era so the titles materialise: 26,955 notices, the smallest era in the corpus. The
expected reading afterwards is title completeness at or near 100 % for INTERNAL_OJS 2008 in the
data-quality report — which is also the check that this worked.

Not addressed here, and worth keeping separate: an EEIG registration is arguably not a *tender* at
all (issue 187 notes the same era's awards are 100 % unchained, for related reasons — they are not
procurement chains). Giving these notices a title makes them findable and honest; deciding whether
non-procurement 2008 notices belong in the Tender population at all is a policy question for the
era, not a mapping one.
