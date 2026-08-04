-- TRIAGE for standing_gate.sh's `negative_amount` failure (task #33, issue 121).
-- READ-ONLY, snapshot-only. Runs inside the Phase 1 confinement cgroup, so it
-- inherits the memory cap under test and cannot evict the live service beyond it.
--
--   sqlite3 -readonly "file:/data/db/snapshots/<pinned>.db?mode=ro" < negative_amount_triage.sql
--
-- THE QUESTION IS NOT "HOW MANY". The gate already answered that: 17,738 rows with
-- tender_version_amounts.cents < 0, against a hard-fail invariant (run_light 3.7)
-- agreed on 2026-07-25 and unenforced since because run_light's pinned totals
-- rotted. What this has to establish is WHICH OF TWO THINGS IS WRONG:
--
--   H1  A PARSE/SIGN DEFECT. Expect the rows to CLUSTER — concentrated in one
--       source, profile or era; a systematic pattern (e.g. every row of one
--       declared_version); magnitudes that mirror plausible positive siblings.
--       Fix would be to the parser, and the data is wrong.
--
--   H2  GENUINELY PUBLISHED NEGATIVE VALUES. Procurement corrections, credit
--       adjustments and withdrawn awards do carry negative money. Expect the rows
--       to SCATTER across sources/profiles/eras and to correlate with
--       correction/amendment notice subtypes. Fix would be to the CHECK, and
--       run_light 3.7 has been wrong for ten days.
--
-- The gate firing is evidence the INVARIANT is violated. It is not evidence the
-- DATA is broken — that is the amplify-a-firing reflex, the mirror of amplifying
-- a green, and this file exists to keep the two apart.
--
-- Every query is bounded by the 17,738 failing rows (filtered first, then joined),
-- not by the ~30M-row parent tables.

.headers on
.mode box
.print '== negative_amount triage — H1 (cluster => parser) vs H2 (scatter => check) =='

.print ''
.print '-- 0. restate the headline, so this file does not depend on remembering it'
SELECT COUNT(*) AS negative_rows,
       COUNT(DISTINCT tender_id) AS distinct_tenders,
       MIN(cents) AS most_negative,
       MAX(cents) AS least_negative
  FROM tender_version_amounts WHERE cents < 0;

.print ''
.print '-- 1. by FIELD and CURRENCY. A sign defect usually lives in one field''s'
.print '--    mapping; a real adjustment can appear in any of them.'
SELECT field, currency, COUNT(*) AS n, MIN(cents) AS min_c, MAX(cents) AS max_c
  FROM tender_version_amounts WHERE cents < 0
 GROUP BY field, currency ORDER BY n DESC LIMIT 30;

.print ''
.print '-- 2. by SOURCE. H1 predicts concentration (one ingest path); H2 predicts'
.print '--    a spread roughly tracking each source''s share of the corpus.'
SELECT t.source, COUNT(*) AS n, COUNT(DISTINCT a.tender_id) AS tenders
  FROM tender_version_amounts a JOIN tenders t ON t.id = a.tender_id
 WHERE a.cents < 0 GROUP BY t.source ORDER BY n DESC;

.print ''
.print '-- 3. by PROFILE and DECLARED VERSION — the sharpest H1 discriminator.'
.print '--    All rows under one profile/version = a parser. Spread = not a parser.'
SELECT n.profile, n.declared_version, COUNT(*) AS n
  FROM tender_version_amounts a
  JOIN tender_versions v ON v.tender_id = a.tender_id AND v.seq = a.seq
  JOIN notices n ON n.id = v.caused_by_notice_id
 WHERE a.cents < 0 GROUP BY n.profile, n.declared_version ORDER BY n DESC LIMIT 30;

.print ''
.print '-- 4. by ERA (publication year). A defect introduced by one era''s format'
.print '--    concentrates; genuine corrections track publication volume.'
SELECT strftime('%Y', v.published_at, 'unixepoch') AS yr, COUNT(*) AS n
  FROM tender_version_amounts a
  JOIN tender_versions v ON v.tender_id = a.tender_id AND v.seq = a.seq
 WHERE a.cents < 0 GROUP BY yr ORDER BY yr;

.print ''
.print '-- 5. by NOTICE SUBTYPE — the H2 discriminator. Correction/amendment/'
.print '--    withdrawal subtypes carrying the negatives supports "genuinely published".'
SELECT COALESCE(v.notice_subtype, '(null)') AS subtype, COUNT(*) AS n
  FROM tender_version_amounts a
  JOIN tender_versions v ON v.tender_id = a.tender_id AND v.seq = a.seq
 WHERE a.cents < 0 GROUP BY subtype ORDER BY n DESC LIMIT 30;

.print ''
.print '-- 6. MAGNITUDE histogram. A sign flip mirrors the positive distribution;'
.print '--    real adjustments skew small relative to contract values.'
SELECT CASE
         WHEN -cents < 100        THEN 'a <1'
         WHEN -cents < 10000      THEN 'b <100'
         WHEN -cents < 1000000    THEN 'c <10k'
         WHEN -cents < 100000000  THEN 'd <1M'
         ELSE                          'e >=1M'
       END AS magnitude_eur, COUNT(*) AS n
  FROM tender_version_amounts WHERE cents < 0 GROUP BY magnitude_eur ORDER BY magnitude_eur;

.print ''
.print '-- 7. THE MIRROR TEST. Does a negative row have a positive sibling of the'
.print '--    same magnitude on the same (tender, seq)? A sign defect often leaves'
.print '--    the correct value beside the wrong one; a real adjustment does not.'
SELECT COUNT(*) AS negatives_with_mirrored_positive
  FROM tender_version_amounts a
 WHERE a.cents < 0
   AND EXISTS (SELECT 1 FROM tender_version_amounts b
                WHERE b.tender_id = a.tender_id AND b.seq = a.seq
                  AND b.cents = -a.cents);

.print ''
.print '-- 8. CONCENTRATION. If a handful of tenders hold most of the rows this is a'
.print '--    few pathological documents, not a systemic property of either kind.'
SELECT tender_id, COUNT(*) AS n FROM tender_version_amounts
 WHERE cents < 0 GROUP BY tender_id ORDER BY n DESC LIMIT 15;

.print ''
.print '== READING IT: concentration in 3/4 + mirrored positives in 7 => H1, fix the parser.'
.print '== Spread in 2/3/4 + correction subtypes in 5 + no mirrors => H2, fix the CHECK.'
.print '== Mixed => say so and do not force a verdict; both can be true of different rows. =='
