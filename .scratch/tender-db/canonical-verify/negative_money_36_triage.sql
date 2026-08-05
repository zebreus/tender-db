-- TRIAGE for task #36 — the ~72,000 negative-money rows on the two surfaces the
-- gate had never checked. READ-ONLY, snapshot-only, runs inside the confinement.
--
--   sqlite3 -readonly "file:<pinned snapshot>?mode=ro" < negative_money_36_triage.sql
--
--   tender_version_bids.cents        < 0  ->  36,987
--   tender_version_lot_results.awarded_cents < 0  ->  35,001
--
-- BIDS FIRST, and not for tidiness. proj-fix established that `awarded_cents` is
-- either `r.direct_cents` — a published value — OR `single_currency_total(winning
-- bids)`, a COMPUTED SUM (project.rs:2375-2399). So a negative bid propagates into
-- awarded_cents mechanically. 36,987 and 35,001 are close enough that one root
-- cause seen twice is the leading hypothesis, and investigating them independently
-- would chase it under two names.
--
-- AND ISSUE 131's CONCLUSION CANNOT BE REUSED HERE, which is the thing to hold on
-- to. That finding was "the source publishes negatives and the fold copies them
-- faithfully" — established for `tender_version_amounts`. It is NOT available for
-- a value that ARRIVES FROM ARITHMETIC: "the source said so" cannot explain a sum.
-- So a negative on the derived arm needs its own explanation and may be a genuine
-- defect, unlike the 17,687 that turned out legitimate.
--
-- Queries are bounded by the failing rows (filtered before joining), never by the
-- parent tables.

.headers on
.mode box
.print '== #36 triage — 72k negative-money rows on two previously-unchecked surfaces =='

.print ''
.print '-- 1. THE OVERLAP TEST, and the cheapest discriminator in the file.'
.print '--    If awarded negatives are CAUSED by bid negatives through the sum, they'
.print '--    should sit on the same tenders. High overlap => one root cause, fix bids.'
.print '--    Low overlap => two independent findings and the propagation story is wrong.'
SELECT
  (SELECT COUNT(DISTINCT tender_id) FROM tender_version_bids        WHERE cents < 0)         AS tenders_with_neg_bids,
  (SELECT COUNT(DISTINCT tender_id) FROM tender_version_lot_results WHERE awarded_cents < 0) AS tenders_with_neg_awarded,
  (SELECT COUNT(*) FROM (
      SELECT DISTINCT tender_id FROM tender_version_bids WHERE cents < 0
      INTERSECT
      SELECT DISTINCT tender_id FROM tender_version_lot_results WHERE awarded_cents < 0)) AS tenders_in_both;

.print ''
.print '-- 2. BIDS: P4 against the chain — does the parse layer already carry a negative?'
.print '--    EXPECT 0. Nonzero = the fold introduced the sign for those rows, which'
.print '--    outranks every distribution result below.'
.print '--    Chain-scoped (seq <= this row''s seq) because the fold CARRIES FACTS'
.print '--    FORWARD: a value published once at seq 1 exists at seqs 1..N while the'
.print '--    notices of 2..N carry nothing. Comparing to the row''s own notice would'
.print '--    falsely indict the fold on every multi-version tender.'
SELECT COUNT(*) AS neg_bids_with_no_negative_in_chain
  FROM tender_version_bids b
 WHERE b.cents < 0
   AND NOT EXISTS (SELECT 1 FROM tender_versions v2
                     JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                    WHERE v2.tender_id = b.tender_id AND v2.seq <= b.seq AND na.cents < 0);

.print ''
.print '-- 3. BIDS: exact magnitude present in the chain -> faithful copy of a published value'
SELECT COUNT(*) AS neg_bids_with_exact_match_in_chain
  FROM tender_version_bids b
 WHERE b.cents < 0
   AND EXISTS (SELECT 1 FROM tender_versions v2
                 JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                WHERE v2.tender_id = b.tender_id AND v2.seq <= b.seq AND na.cents = b.cents);

.print ''
.print '-- 4. AWARDED: the same two questions. A LOW exact-match rate here, against a'
.print '--    high one for bids, is the signature of the DERIVED arm — a sum has no'
.print '--    published counterpart to match, by construction.'
SELECT
  (SELECT COUNT(*) FROM tender_version_lot_results a WHERE a.awarded_cents < 0
     AND NOT EXISTS (SELECT 1 FROM tender_versions v2 JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                      WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq AND na.cents < 0))       AS awarded_no_chain_negative,
  (SELECT COUNT(*) FROM tender_version_lot_results a WHERE a.awarded_cents < 0
     AND EXISTS (SELECT 1 FROM tender_versions v2 JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                  WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq AND na.cents = a.awarded_cents)) AS awarded_exact_match;

.print ''
.print '-- 5. MAGNITUDE, both surfaces. The 17,687 legitimate ones clustered at EUR 1-100.'
.print '--    A different shape here is a different phenomenon.'
SELECT 'bids' AS surface,
       SUM(-cents < 100) AS "a <1", SUM(-cents >= 100 AND -cents < 10000) AS "b <100",
       SUM(-cents >= 10000 AND -cents < 1000000) AS "c <10k",
       SUM(-cents >= 1000000) AS "d >=10k", MIN(cents) AS most_negative
  FROM tender_version_bids WHERE cents < 0
UNION ALL
SELECT 'awarded',
       SUM(-awarded_cents < 100), SUM(-awarded_cents >= 100 AND -awarded_cents < 10000),
       SUM(-awarded_cents >= 10000 AND -awarded_cents < 1000000),
       SUM(-awarded_cents >= 1000000), MIN(awarded_cents)
  FROM tender_version_lot_results WHERE awarded_cents < 0;

.print ''
.print '-- 6. PROFILE / ERA spread — a cluster points at a mapping fault, a spread does not.'
SELECT n.profile, COUNT(*) AS neg_bids
  FROM tender_version_bids b
  JOIN tender_versions v ON v.tender_id = b.tender_id AND v.seq = b.seq
  JOIN notices n ON n.id = v.caused_by_notice_id
 WHERE b.cents < 0 GROUP BY n.profile ORDER BY neg_bids DESC LIMIT 15;

.print ''
.print '== READING IT'
.print '== q1 high overlap + q2/q3 bids faithful  => one root cause, source-published, fix the CHECK'
.print '== q2 nonzero                             => the fold introduced it; a code defect, outranks all'
.print '== q4 low exact-match vs high for bids    => the derived arm; 131 cannot explain it and it'
.print '==                                           needs its own account rather than inheriting one'
.print '== q5 shape unlike EUR 1-100              => a different phenomenon from the 17,687 =='
