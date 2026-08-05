-- TRIAGE for the LARGE-MAGNITUDE residue of #36 — the negatives at >= EUR 10k that
-- #36 deliberately left flagged rather than waving through. READ-ONLY, snapshot-only,
-- runs inside the SAME confinement and against the SAME snapshot class as the #36
-- triage: same envelope, so per the standing rule this is "tell, don't re-ask".
--
--   sqlite3 -readonly "file:<pinned snapshot>?mode=ro" < negative_money_407_triage.sql
--
-- WHY THIS BLOCKS #37, stated so the sequencing risk is visible from inside the file.
-- #37 re-specs the negative-money gate with a DERIVED magnitude threshold — narrow,
-- not flatten. proj-fix's argument in issue 134 is that if this residue is a
-- delta-mapped-as-an-absolute fault, then NO threshold of any shape should be tuned
-- to admit it, and a re-spec calibrated against it would be calibrated to accept a
-- bug. So the threshold cannot be derived until this population has an account.
-- Running the re-spec first would not be merely premature; it would bake the defect
-- into the check meant to catch it.
--
-- THE COUNT IS NOT HARDCODED. It was carried as "407" in conversation, and whether
-- that was bids, awarded, or the two summed is exactly the sort of remembered number
-- this suite keeps finding to be wrong. Q0 recomputes the population per surface and
-- the reader reconciles; nothing downstream depends on the remembered figure.
--
-- WHAT WOULD MAKE THIS A DEFECT rather than more published noise. #131 established
-- "the source publishes negatives and the fold copies them faithfully" for
-- tender_version_amounts. That account is NOT transferable here, twice over:
--   * awarded_cents is EITHER r.direct_cents (published) OR single_currency_total of
--     the winning bids (project.rs:2375-2378) — a COMPUTED SUM. "The source said so"
--     cannot explain a sum.
--   * these are large. The benign mass in #33 was 99.6% between EUR 1 and 100, the
--     shape of rounding and adjustment noise. EUR 10k+ is not that shape.
--
-- BOUNDING. Every query filters to negatives before joining, and the large-magnitude
-- subset before doing anything per-row. The parent tables are never driven from.
--
-- VALIDATED BEFORE IT EVER TOUCHES THE SNAPSHOT, and validated for DISCRIMINATION
-- rather than for parsing. Running it first against an empty schema proved only that
-- it parses — which is this suite's own vacuity trap ("zero rows violate X" is
-- trivially true of an empty table), and accepting that as a pass would have repeated
-- the exact defect the standing gate's presence-checks exist to prevent. So it was run
-- against a fixture with KNOWN answers, and every arm of every query was made to fire:
--
--   a genuine delta (prior 50k, revision -20k)      -> plausible_delta = 1
--   a negative exceeding its prior (10k, -50k)      -> exceeds_prior_value = 1
--   a first-ever-negative with no predecessor       -> first_ever = 1, prior_cents NULL
--   a -5k row, below the threshold                  -> excluded from all seven
--   a chain publishing the exact magnitude          -> exact_published_match = 1
--   two chains publishing nothing negative          -> no_negative_in_chain = 2
--   a negative bid on the same (tender,seq)         -> same_version_has_negative_bid = 1
--
-- Predictions were written down BEFORE running and all seven matched. Q3/Q5 initially
-- reported 0 and no rows purely because `notices`/`notice_amounts` were unpopulated —
-- a zero that meant "untested", not "clean". That is the same permissive shape the
-- suite keeps finding, caught here in my own instrument, and it is why the join arms
-- were then exercised deliberately rather than assumed from a green.

.headers on
.mode box
.print '== residue triage — negative money at >= EUR 10k (100,000,000 cents... see Q0) =='
.print '== threshold: 10000 EUR = 1000000 cents. Stated once, used everywhere below. =='

.print ''
.print '-- Q0. THE POPULATION, recomputed. Reconcile against the remembered 407 before'
.print '--     reading anything else. A mismatch means the residue was mis-carried and'
.print '--     every downstream conclusion is about a different set than intended.'
SELECT 'bids'    AS surface, COUNT(*) AS rows_ge_10k, MIN(cents)         AS most_negative
  FROM tender_version_bids        WHERE cents         < 0 AND -cents         >= 1000000
UNION ALL
SELECT 'awarded', COUNT(*),            MIN(awarded_cents)
  FROM tender_version_lot_results WHERE awarded_cents < 0 AND -awarded_cents >= 1000000;

.print ''
.print '-- Q1. THE DELTA TEST, opening move: does the negative even HAVE a predecessor?'
.print '--     A delta is meaningless with nothing to apply it to. If a large negative is'
.print '--     the FIRST awarded value ever published for its lot_result, the'
.print '--     delta-mapped-as-absolute story is IMPOSSIBLE for that row, not merely'
.print '--     unsupported. lot_result_id is stable across seqs, so the chain is walkable.'
.print '--     EXPECT, if delta: has_prior = all of them. If sign-fault: many with none.'
SELECT
  SUM(has_prior)     AS with_earlier_value,
  SUM(1 - has_prior) AS first_ever_value_is_negative,
  COUNT(*)           AS total
FROM (
  SELECT EXISTS (
           SELECT 1 FROM tender_version_lot_results p
            WHERE p.tender_id = a.tender_id AND p.lot_result_id = a.lot_result_id
              AND p.seq < a.seq AND p.awarded_cents IS NOT NULL) AS has_prior
    FROM tender_version_lot_results a
   WHERE a.awarded_cents < 0 AND -a.awarded_cents >= 1000000);

.print ''
.print '-- Q2. THE DELTA TEST proper. For those WITH a predecessor, take the most recent'
.print '--     earlier non-null value and ask whether the negative behaves like a'
.print '--     revision applied to it. A genuine downward delta satisfies prior+neg >= 0'
.print '--     (you cannot revise away more than was there). A sign-flip or a mis-parse'
.print '--     has no such relation and will scatter across both columns.'
.print '--     A HIGH plausible_delta count is the finding that BLOCKS #37 outright.'
WITH neg AS (
  SELECT a.tender_id, a.seq, a.lot_result_id, a.awarded_cents AS neg_cents,
         (SELECT p.awarded_cents FROM tender_version_lot_results p
           WHERE p.tender_id = a.tender_id AND p.lot_result_id = a.lot_result_id
             AND p.seq < a.seq AND p.awarded_cents IS NOT NULL
           ORDER BY p.seq DESC LIMIT 1) AS prior_cents
    FROM tender_version_lot_results a
   WHERE a.awarded_cents < 0 AND -a.awarded_cents >= 1000000)
SELECT
  SUM(prior_cents + neg_cents >= 0)                      AS plausible_delta,
  SUM(prior_cents + neg_cents <  0)                      AS exceeds_prior_value,
  SUM(prior_cents = -neg_cents)                          AS exact_sign_flip_of_prior,
  COUNT(*)                                               AS with_prior
FROM neg WHERE prior_cents IS NOT NULL;

.print ''
.print '-- Q3. WHICH ARM produced it — published, or computed sum? This decides whether'
.print '--     #131''s account is even a candidate. An exact magnitude match in the'
.print '--     chain''s parse layer means a published value was copied (131 applies).'
.print '--     No match points at the summed arm, where 131 CANNOT apply by construction.'
.print '--     Chain-scoped (seq <= a.seq) because the fold carries facts forward.'
SELECT
  COUNT(*) AS ge10k_awarded_negatives,
  SUM(EXISTS (SELECT 1 FROM tender_versions v2
                JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
               WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq
                 AND na.cents = a.awarded_cents))            AS exact_published_match,
  SUM(NOT EXISTS (SELECT 1 FROM tender_versions v2
                    JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                   WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq
                     AND na.cents < 0))                      AS no_negative_anywhere_in_chain
  FROM tender_version_lot_results a
 WHERE a.awarded_cents < 0 AND -a.awarded_cents >= 1000000;

.print ''
.print '-- Q4. PROPAGATION: is the awarded negative just the bid negatives summed? If the'
.print '--     computed arm is responsible, awarded_cents should equal the sum of that'
.print '--     lot''s winning bids. Matching rows are ONE defect (in bids) seen twice,'
.print '--     not two — and fixing bids fixes both.'
WITH neg AS (
  SELECT a.tender_id, a.seq, a.lot_result_id, a.awarded_cents
    FROM tender_version_lot_results a
   WHERE a.awarded_cents < 0 AND -a.awarded_cents >= 1000000)
SELECT
  COUNT(*) AS ge10k_awarded,
  SUM(EXISTS (SELECT 1 FROM tender_version_bids b
               WHERE b.tender_id = neg.tender_id AND b.seq = neg.seq AND b.cents < 0))
        AS same_version_has_negative_bid
FROM neg;

.print ''
.print '-- Q5. IS THE RESIDUE A DIFFERENT POPULATION, or the tail of the benign mass?'
.print '--     This is issue 134''s question stated as a measurement. If the large ones'
.print '--     cluster on profiles the small ones do not touch, they are a distinct'
.print '--     phenomenon and no single threshold separates signal from noise — which is'
.print '--     precisely the case in which #37 must NOT be calibrated to admit them.'
SELECT n.profile,
       SUM(-a.awarded_cents >= 1000000) AS ge_10k,
       SUM(-a.awarded_cents <  1000000) AS lt_10k
  FROM tender_version_lot_results a
  JOIN tender_versions v ON v.tender_id = a.tender_id AND v.seq = a.seq
  JOIN notices n         ON n.id = v.caused_by_notice_id
 WHERE a.awarded_cents < 0
 GROUP BY n.profile
 HAVING ge_10k > 0
 ORDER BY ge_10k DESC LIMIT 20;

.print ''
.print '-- Q6. SPECIMENS. Twenty actual rows, largest first, with their predecessor. The'
.print '--     aggregates above can all be consistent with more than one story; reading'
.print '--     real rows is what has repeatedly settled which. Bounded by LIMIT.'
SELECT a.tender_id, a.seq, a.lot_result_id,
       a.awarded_cents AS neg_cents, a.awarded_currency,
       (SELECT p.awarded_cents FROM tender_version_lot_results p
         WHERE p.tender_id = a.tender_id AND p.lot_result_id = a.lot_result_id
           AND p.seq < a.seq AND p.awarded_cents IS NOT NULL
         ORDER BY p.seq DESC LIMIT 1) AS prior_cents
  FROM tender_version_lot_results a
 WHERE a.awarded_cents < 0 AND -a.awarded_cents >= 1000000
 ORDER BY a.awarded_cents ASC LIMIT 20;

.print ''
.print '== READING IT — decided BEFORE the numbers arrive, so the reading is not fitted to them'
.print '== Q1 first_ever_value_is_negative high  => delta story IMPOSSIBLE for those rows'
.print '== Q2 plausible_delta high               => DEFECT. #37 blocked; no threshold may admit these'
.print '== Q2 exceeds_prior high, Q3 match high  => published nonsense faithfully copied; 131-shaped,'
.print '==                                          and then #37 may proceed on a derived threshold'
.print '== Q3 exact_published_match ~0           => the SUMMED arm; 131 cannot apply, needs its own account'
.print '== Q4 same_version_negative_bid high     => ONE root cause in bids, seen twice. Fix bids.'
.print '== Q5 distinct profiles for ge_10k       => a separate population; a single threshold cannot'
.print '==                                          separate them and #37 must not pretend otherwise'
