-- The 1,905 remainder — attribution, not a count (issue 84 / #29, criterion in issue 138).
--
-- sdk-vendor's sign-off blocks on this: 594,915 measured outstanding DTD rows
-- minus 592,856 markable minus 154 guard-rejected leaves 1,905 unaccounted, and
-- "an unexplained remainder is a finding, not a rounding error". Agreed — so this
-- attributes every one of the 1,905 to the specific predicate condition that
-- excluded it, rather than counting them again.
--
-- WHY THE GAP EXISTS AT ALL — the discrepancy is between two different predicates,
-- and that is the first thing to rule in or out:
--
--   * the OUTSTANDING measure uses  detail LIKE 'XML with DTD detected%'
--   * my marker scope uses          detail  =   'XML with DTD detected'   (exact)
--
-- so any row whose detail carries a suffix is counted by the first and excluded by
-- the second. That is a hypothesis, not the answer; these queries test it.
--
-- ============================ READING KEY ============================
-- Written BEFORE the queries run, so the numbers cannot choose their own meaning
-- (the discipline from the 407 triage and issue 138 itself).
--
--   detail_has_suffix        — the LIKE-vs-exact difference. BENIGN and expected:
--                              the same defect with a more specific message. They
--                              are legitimately out of the marker's scope, because
--                              the scope was verified against the exact-detail
--                              population, and must stay outstanding until someone
--                              verifies THEM.  => explains the remainder, no action.
--
--   outside_2008_ted_monthly — DTD-declined rows from some other fetch. NOT benign:
--                              the duplicate-sibling argument was established over
--                              the 2008 TED monthly corpus and a population outside
--                              it has no such evidence. A large count here is a
--                              FOURTH population under the label and BLOCKS sign-off.
--
--   english_original         — the deliberate `.en` carve-out. I measured 7 earlier.
--                              ~7 corroborates that; much larger means my "7 .en
--                              (0.0012%)" was wrong and the language guard needs
--                              re-examining before anything is marked.
--
--   in_scope                 — must equal 592,856 + 154 = 593,010. If it does NOT,
--                              the marker predicate and this attribution disagree,
--                              which would mean the dry-run number and this query
--                              measure different sets and the remainder arithmetic
--                              is void. THE LOAD-BEARING BUCKET: it is the one that
--                              can falsify my own predicate.
--
-- The four buckets are mutually exclusive (CASE is first-match) and exhaustive (the
-- ELSE catches everything), so they MUST sum to Q0's total. If they do not, the
-- outstanding figure is over a different population and needs re-deriving first.
-- =====================================================================
--
-- RUNNER: `POST /v1/sql` against the LIVE database, authorized by team-lead
-- 2026-08-05 as a bounded single-table aggregate (the app reading its own DB
-- in-process was never governed by the external-reader rules; the Class B shed
-- bounds it). NOT sqlite3, and not a snapshot: turso cannot open one read-only.
--
-- Three endpoint constraints shape the SQL below, all verified in
-- `crates/app/src/v1/sql.rs` before running rather than discovered by a 400:
--
--   1. ONE STATEMENT PER REQUEST — the parser accepts exactly one bare SELECT.
--      So these are four separate calls, Q0..Q3, not one script.
--   2. `fetches` IS NOT ON THE ALLOW-LIST; `v_fetches` (path-free, issue 45) is.
--      Every join goes through the view. Adding `fetches` to the allow-list to
--      run a diagnostic would be a public data-surface change, and the allow-list
--      is positive-by-default precisely because a deny-list re-opened once and
--      exposed webhook secrets (issue 43/45). Not worth a one-off query.
--   3. A 10-SECOND CAP with a 408 backstop. Hence the ordering: Q0 is the cheap
--      total that also measures the scan cost, so a timeout there tells us the
--      whole approach needs a different bed BEFORE the expensive one runs.
--
-- The 2008 test uses `IN (SELECT id FROM v_fetches …)` rather than a correlated
-- EXISTS: the subquery is evaluated once over a small table instead of per row of
-- a 2.4M-row scan. `quarantine.fetch_id` is NOT NULL and the subquery returns no
-- NULLs, so `NOT IN` is safe here — worth stating, since NOT IN against a
-- NULL-bearing set silently returns nothing and would read as "no such rows".

-- Q0 — the total, and the cost probe. Must equal the 594,915 the outstanding
-- measure reported; a mismatch means the two measures are over different
-- populations and the remainder arithmetic (594,915 - 592,856 - 154 = 1,905)
-- does not hold, which is a finding on its own.
SELECT COUNT(*) AS total_outstanding_dtd_like
  FROM quarantine q
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL;

-- Q1 — the attribution. Bucket order defines precedence; do not reorder casually.
SELECT CASE
         WHEN q.detail <> 'XML with DTD detected'
           THEN 'detail_has_suffix'
         WHEN q.fetch_id NOT IN (SELECT id FROM v_fetches
                                  WHERE source = 'ted' AND kind = 'monthly'
                                    AND period LIKE '2008%')
           THEN 'outside_2008_ted_monthly'
         WHEN lower(replace(q.member_path,
                            rtrim(q.member_path, replace(q.member_path, '.', '')), '')) = 'en'
           THEN 'english_original'
         ELSE 'in_scope'
       END      AS bucket,
       COUNT(*) AS n_rows
  FROM quarantine q
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL
 GROUP BY 1
 ORDER BY 2 DESC;

-- Q2 — name the suffixes. A bucket count is not an explanation: if these are one
-- variant message the remainder is a labelling detail; if they are many distinct
-- parser messages then 'XML with DTD detected%' covers more than one failure, and
-- that must be known before the group is reasoned about as a single population.
SELECT q.detail AS detail_variant,
       COUNT(*) AS n_rows
  FROM quarantine q
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.detail <> 'XML with DTD detected'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL
 GROUP BY 1
 ORDER BY 2 DESC
 LIMIT 25;

-- Q3 — name the out-of-2008 fetches, if any. This is the bucket that can block
-- sign-off, so it reports WHICH corpus rather than how many: "1,900 rows from
-- somewhere" is not an explanation anyone can act on.
SELECT f.source,
       f.kind,
       f.period,
       COUNT(*) AS n_rows
  FROM quarantine q
  JOIN v_fetches f ON f.id = q.fetch_id
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL
   AND NOT (f.source = 'ted' AND f.kind = 'monthly' AND f.period LIKE '2008%')
 GROUP BY 1, 2, 3
 ORDER BY 4 DESC
 LIMIT 25;

-- ============================== RESULTS ==============================
-- Run 2026-08-05 against the LIVE DB via POST /v1/sql on the box (localhost),
-- token from /root/tdb-diag-token, team-lead authorized. Q0 5.99s, Q1 5.23s,
-- Q3 2.49s — all under the 10s cap, none truncated.
--
--   Q0  total_outstanding_dtd_like  594,915   <- matches the outstanding measure
--                                                 EXACTLY, so the two predicates
--                                                 are over the same population and
--                                                 the remainder arithmetic holds.
--   Q1  in_scope                    593,010   <- EXACTLY 592,856 + 154. The
--                                                 load-bearing bucket passes: my
--                                                 marker predicate and the dry-run
--                                                 measure the same set.
--       outside_2008_ted_monthly      1,898
--       english_original                  7   <- corroborates the earlier ".en = 7"
--       detail_has_suffix                 0   <- ZERO. See below.
--
--   1,898 + 7 = 1,905. The remainder is fully attributed, with nothing left over.
--
-- MY HYPOTHESIS WAS WRONG, and in the unflattering direction. I predicted the
-- remainder was the benign LIKE-vs-exact difference — rows whose detail carries a
-- suffix. That bucket is EMPTY. Not "small": zero. The detail string had nothing to
-- do with it, and had the reading key not been fixed in advance it would have been
-- very easy to read 1,898 out-of-2008 rows as "roughly the labelling noise I
-- predicted" and move on. The pre-registration is what stopped a wrong explanation
-- from being accepted because it was already written down.
--
-- WHAT THE 1,898 ACTUALLY ARE (Q3, then a language breakdown):
--   * ALL of them come from ONE fetch: ted / monthly / 2010-03. A single month,
--     and not the 2008 corpus at all.
--   * Their member paths end in `.xml` with NO language code (the suffix extraction
--     returns 'xml' for all 1,898, where the 2008 corpus returns a 2-letter
--     language). So they are NOT language siblings.
--
-- Therefore the duplicate-sibling argument DOES NOT APPLY to them, and this is a
-- FOURTH population under the 'XML with DTD detected' label — exactly what the
-- reading key said would block calling the remainder benign.
--
-- It does NOT block the execute's safety: the marker scope requires a 2008 TED
-- monthly fetch, so these 1,898 are excluded BY CONSTRUCTION and cannot be
-- mis-marked. They stay outstanding, which is the correct outcome. What it blocks
-- is the claim that the remainder is explained-and-harmless: it is explained, and
-- it is a separate unresolved population needing its own investigation.
-- =====================================================================
