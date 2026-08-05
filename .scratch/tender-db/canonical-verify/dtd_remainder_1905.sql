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
-- the second. That is a hypothesis, not the answer; this query tests it.
--
-- ============================ READING KEY ============================
-- Written BEFORE the query runs, so the numbers cannot choose their own meaning
-- (the discipline from the 407 triage and issue 138 itself).
--
--   detail_has_suffix        — the LIKE-vs-exact difference. BENIGN and expected:
--                              these are the same defect with a more specific
--                              message. They are legitimately out of the marker's
--                              scope, because the scope was verified against the
--                              exact-detail population. They must stay outstanding
--                              until someone verifies THEM.
--                              => explains the remainder; no action for #29.
--
--   outside_2008_ted_monthly — DTD-declined rows from some other fetch. NOT benign
--                              by default: the whole duplicate-sibling argument was
--                              established over the 2008 TED monthly corpus, and a
--                              population outside it has no such evidence. A large
--                              count here is a FOURTH population under the label
--                              and blocks sign-off.
--                              => report the periods; do not mark.
--
--   english_original         — the deliberate `.en` carve-out. I measured 7 of
--                              these earlier. If this bucket is ~7 it corroborates
--                              that measurement; if it is much larger, my earlier
--                              "7 .en (0.0012%)" was wrong and the language guard
--                              needs re-examining before anything is marked.
--
--   in_scope                 — must equal 592,856 + 154 = 593,010. If it does NOT,
--                              the marker predicate and this attribution disagree,
--                              and THAT is the finding — it would mean the dry-run
--                              number and this query are measuring different sets,
--                              which invalidates the remainder arithmetic entirely.
--                              This is the load-bearing bucket: it is the one that
--                              can falsify the whole reconciliation.
--
-- The four buckets are mutually exclusive (CASE is first-match) and exhaustive
-- (the ELSE catches everything), so they MUST sum to the outstanding total. If
-- they do not sum to 594,915, the outstanding figure itself is from a different
-- population and needs re-deriving before any of this means anything.
-- =====================================================================
--
-- SAFETY: read-only, no writes. Scans `quarantine` filtered on `reason`/`detail`,
-- so it reads data pages — snapshot only, never the serving DB, per
-- docs/agents/prod-box-reads.md.
--
-- RUNNER: turso, against a snapshot. Deliberately carries NO sqlite3 dot-commands
-- (`.mode`, `.headers`): those are sqlite3-specific, and the standing owner rule is
-- turso-only, one engine. They were in the first draft of this file out of habit
-- from the deprecated canonical-verify scripts, and removed — an artifact that only
-- runs under the banned tool is not a portable inconvenience, it is a file nobody
-- is allowed to execute.
--
-- This is a ONE-OFF diagnostic that gates a sign-off, not a standing check, so it
-- stays SQL rather than becoming an in-app Spec. If it ever needs re-running on a
-- cadence, that is the signal to move it in-app like the rest of #28.

-- 1. The attribution. Bucket order defines precedence; do not reorder casually.
SELECT CASE
         WHEN q.detail <> 'XML with DTD detected'
           THEN 'detail_has_suffix'
         WHEN NOT EXISTS (SELECT 1 FROM fetches f
                           WHERE f.id = q.fetch_id
                             AND f.source = 'ted' AND f.kind = 'monthly'
                             AND f.period LIKE '2008%')
           THEN 'outside_2008_ted_monthly'
         WHEN lower(replace(q.member_path,
                            rtrim(q.member_path, replace(q.member_path, '.', '')), '')) = 'en'
           THEN 'english_original'
         ELSE 'in_scope'
       END                AS bucket,
       COUNT(*)           AS rows
  FROM quarantine q
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL
 GROUP BY 1
 ORDER BY 2 DESC;

-- 2. Name the suffixes. A bucket count is not an explanation — if these are all
--    one variant message the remainder is a labelling detail; if they are many
--    distinct parser messages, the label 'XML with DTD detected%' is covering
--    more than one failure and that is worth knowing before it is reasoned about
--    as a single population.
SELECT q.detail        AS detail_variant,
       COUNT(*)        AS rows
  FROM quarantine q
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.detail <> 'XML with DTD detected'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL
 GROUP BY 1
 ORDER BY 2 DESC
 LIMIT 25;

-- 3. Name the out-of-2008 fetches, if any. This is the bucket that can block
--    sign-off, so it reports WHICH corpus rather than how many — "1,900 rows from
--    somewhere" is not an explanation anyone can act on.
SELECT f.source,
       f.kind,
       f.period,
       COUNT(*) AS rows
  FROM quarantine q
  JOIN fetches f ON f.id = q.fetch_id
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL
   AND NOT (f.source = 'ted' AND f.kind = 'monthly' AND f.period LIKE '2008%')
 GROUP BY 1, 2, 3
 ORDER BY 4 DESC
 LIMIT 25;

-- 4. The sum check, run as its own statement so it cannot be eyeballed wrong.
--    Must equal the 594,915 the outstanding measure reported. A mismatch means
--    the two measures are over different populations and the remainder
--    arithmetic (594,915 - 592,856 - 154 = 1,905) does not hold.
SELECT COUNT(*) AS total_outstanding_dtd_like
  FROM quarantine q
 WHERE q.reason = 'unparsable-xml'
   AND q.detail LIKE 'XML with DTD detected%'
   AND q.reprocessed_at IS NULL
   AND q.skipped_at IS NULL;
