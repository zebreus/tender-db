-- ============================================================================
-- The 154 held-but-unparsed English originals (issue 84 residue).
--
-- The #29 dry-run refused to execute: 592,856 rows markable, but 154 in scope
-- whose English original IS in the corpus and did NOT parse. Those are real data
-- loss, not duplicates — marking them skipped-by-policy would record a parse
-- failure as a resolved duplicate, the one outcome worse than an overstated count.
--
-- This characterises them so the next question is answerable: FIXABLE PARSER GAP,
-- or genuinely unrecoverable?
--
-- PREDICTION, stated before the run so this is a falsification and not a fishing
-- trip. The internal-ojs parse path can reject with exactly four reasons
-- (r209/parse.rs + internal_ojs.rs): `unclaimed-content`, `no-original-form`,
-- `unexpected-root`, `unparsable-xml`, plus `not-utf8` before parsing.
--
--   * I expect the bulk to be `unclaimed-content` — an element or attribute no
--     rule claims. That is the SAME class as issues 31 and 35, both of which were
--     fixed by adding the missing vocabulary, so it is the FIXABLE arm. 154 out of
--     ~27k English 2008 notices (0.6%) is exactly the shape of a long-tail
--     vocabulary gap.
--   * `unexpected-root` or `not-utf8` would mean something else is in these files
--     entirely — a different finding, and not a parser gap.
--   * A spread across all four with no dominant reason would mean these are not
--     one problem, and the "fixable?" question has to be asked per bucket.
--
-- Q3 is the decisive one for effort: an unclaimed-content detail names the exact
-- path that was not claimed, so a small set of distinct paths = a small fix.
--
-- WHAT THIS PASS CANNOT ANSWER, and why the byte-length check does not fit here.
--
-- The ask was to fold in "is the original's presence VERIFIED by reading the bytes
-- (present, non-zero, matching recorded length), or only ASSERTED by a held flag?"
-- That is the right question and it cannot be asked of this database:
--
--   `notices` (lib.rs:97): "The payload is NOT stored: (fetch_id, member_path)
--   locates it inside the immutable raw archive."
--
-- So no snapshot query can read a payload byte. The bytes live in the archive
-- tarballs and reaching them means walking a .tar for 154 members — a different
-- operation, on the box, not a snapshot read, and heavier than every query here
-- combined. Writing it as SQL anyway would have produced a query that runs, returns
-- something, and answers a different question than its name claims.
--
-- What IS knowable in-DB, and its limit: every notice row carries a NOT NULL
-- `content_hash`, recorded at ingest from bytes that were read and hashed. So a row
-- existing is evidence the payload existed AT INGEST — it is not evidence the
-- payload exists NOW. `content_hash` presence is therefore vacuous as a check (the
-- column cannot be null) and is not asked. Q6 asks the one non-vacuous in-DB
-- version: does the English original's hash DIFFER from its sibling's? If they are
-- equal, the two files are byte-identical and the "per-language variant" framing is
-- wrong for that row — a finding in its own right.
--
-- The archive-side check remains worth doing and belongs in its own pass, scoped to
-- whatever subset these results make suspicious.
--
-- READ-ONLY, data pages: snapshot only, never the serving DB.
-- ============================================================================

.mode box
.headers on

-- The 154: in the 2008-DTD scope, non-EN, whose English original exists and is held.
CREATE TEMP VIEW residue AS
  SELECT q.member_path,
         replace(substr(replace(q.member_path, rtrim(q.member_path, replace(q.member_path,'/','')),''),
                        1,
                        length(replace(q.member_path, rtrim(q.member_path, replace(q.member_path,'/','')),'')) - 3),
                 '_','-') AS pub_id
    FROM quarantine q JOIN fetches f ON f.id = q.fetch_id
   WHERE q.reason = 'unparsable-xml' AND q.detail = 'XML with DTD detected'
     AND q.reprocessed_at IS NULL AND q.skipped_at IS NULL
     AND f.source='ted' AND f.kind='monthly' AND f.period LIKE '2008%'
     AND lower(replace(q.member_path, rtrim(q.member_path, replace(q.member_path,'.','')),'')) <> 'en';

-- The English originals of those siblings, held rather than parsed.
CREATE TEMP VIEW originals AS
  SELECT DISTINCT n.id, n.publication_id, n.parse_state, n.profile
    FROM residue r JOIN notices n
      ON n.source='ted' AND n.publication_id = r.pub_id
   WHERE +n.parse_state <> 'parsed';

SELECT '--- Q1. how many distinct originals, and their state ---' AS "";
SELECT parse_state, profile, COUNT(*) AS originals FROM originals GROUP BY 1,2 ORDER BY 3 DESC;

SELECT '--- Q2. WHY did they fail? (predict: unclaimed-content dominant) ---' AS "";
SELECT q.reason, COUNT(*) AS n
  FROM originals o JOIN quarantine q ON q.notice_id = o.id
 GROUP BY 1 ORDER BY 2 DESC;

SELECT '--- Q3. DECISIVE: how many DISTINCT unclaimed paths? (few = small fix) ---' AS "";
SELECT q.detail, COUNT(*) AS n
  FROM originals o JOIN quarantine q ON q.notice_id = o.id
 GROUP BY 1 ORDER BY 2 DESC LIMIT 30;

SELECT '--- Q4. are they clustered in time, or spread across 2008? ---' AS "";
SELECT f.period, COUNT(*) AS n
  FROM originals o JOIN notices n ON n.id = o.id JOIN fetches f ON f.id = n.fetch_id
 GROUP BY 1 ORDER BY 1;

-- Q6 is what remains of the byte-length check, and the header below says why the
-- rest of it cannot ride this pass.
SELECT '--- Q6. is the original distinguishable from its siblings at all? ---' AS "";
SELECT CASE WHEN n.content_hash = q.content_hash THEN 'EN hash == sibling hash (NOT a per-language variant)'
            ELSE 'EN hash differs from sibling (a genuine variant)' END AS relation,
       COUNT(*) AS n
  FROM residue r
  JOIN notices n ON n.source='ted' AND n.publication_id = r.pub_id
  JOIN quarantine q ON q.member_path = r.member_path
 GROUP BY 1 ORDER BY 2 DESC;

SELECT '--- Q5. reconcile: sibling rows vs distinct originals ---' AS "";
SELECT (SELECT COUNT(*) FROM residue)   AS sibling_rows_in_scope,
       (SELECT COUNT(*) FROM originals) AS distinct_held_originals;
