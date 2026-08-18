# 233 — INTERNAL_OJS 2008 carries a title on only 43.7 % of its versions

Status: needs-triage — measured 2026-08-18 against prod (job 731, rev `a79540e`)
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
