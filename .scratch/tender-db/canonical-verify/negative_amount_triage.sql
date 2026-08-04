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
.print '== P4 (proj-fix, DECISIVE) — did the FOLD introduce any negative? =========='
.print '-- proj-fix established on code paths that the fold is a pure copy with no'
.print '-- arithmetic, and that value::cents deliberately keeps a published minus sign'
.print '-- (`sign * c`, overflow-guarded). Confirmed independently here by reading'
.print '-- eforms/value.rs:81-103. If that holds, EVERY folded negative traces to a'
.print '-- notice that already carried a negative in the PARSE layer.'
.print '--'
.print '-- Joined on notice rather than on field name: tender_version_amounts.field is'
.print '-- the canonical name while notice_amounts.field_id is the SDK id, so a'
.print '-- name-join would silently under-match and manufacture a false positive for'
.print '-- "the fold did it". Notice-level is the claim that actually discriminates.'
.print '--'
.print '-- COMPARED AGAINST THE CHAIN, NOT THE CAUSING NOTICE. The fold CARRIES FACTS'
.print '-- FORWARD: fold() clones the previous version''s facts and supersede() replaces'
.print '-- only the fields the new notice republishes (project.rs:1882-1935, read'
.print '-- directly, not inherited). So a negative published once at seq 1 exists at'
.print '-- seqs 1..N, and the notices of seqs 2..N carry no negative at all. Comparing'
.print '-- to the version''s OWN notice would flag every one of those as fold-introduced'
.print '-- — a false indictment of the fold on any multi-version tender, which is most'
.print '-- of them. (proj-fix, whose area it is; my both-ways fixture could not have'
.print '-- caught it because carry-forward needs >=2 versions.)'
.print '--'
.print '-- RESIDUAL, so a 0 is not over-read: even corrected, 0 means NO NEGATIVE'
.print '-- APPEARED FROM NOWHERE. It does not prove the fold correct. A fold that'
.print '-- flipped a sign on a chain that legitimately carries some other negative'
.print '-- stays invisible here. Negatives are rare so the residual is probably small,'
.print '-- but "0" and "the fold is proven right" are different claims.'

.print ''
.print '-- 9. folded negatives whose SOURCE NOTICE has NO negative at all -> EXPECT 0.'
.print '--    Any nonzero = the fold introduced a sign the parse layer never had, proj-fix''s'
.print '--    analysis is wrong, and the defect is theirs rather than the invariant''s.'
SELECT COUNT(*) AS folded_negatives_with_no_negative_anywhere_in_chain
  FROM tender_version_amounts a
 WHERE a.cents < 0
   AND NOT EXISTS (SELECT 1 FROM tender_versions v2
                     JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                    WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq
                      AND na.cents < 0);

.print ''
.print '-- 10. stronger form: EXACT magnitude present in the parse layer for that notice.'
.print '--     High share = faithful copy. A large gap between 9 and 10 would mean the'
.print '--     notice had SOME negative but not THIS value — worth a look, not a verdict.'
SELECT COUNT(*) AS folded_negatives_with_exact_match_in_chain
  FROM tender_version_amounts a
 WHERE a.cents < 0
   AND EXISTS (SELECT 1 FROM tender_versions v2
                 JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq
                  AND na.cents = a.cents);

.print ''
.print '-- 11. P1 — the text era emits no Amount variant, so it should contribute NOTHING.'
.print '--     A text-profile negative means amounts reach this table by a path proj-fix'
.print '--     has not found, and their code-path argument is incomplete.'
SELECT n.profile, COUNT(*) AS n
  FROM tender_version_amounts a
  JOIN tender_versions v ON v.tender_id = a.tender_id AND v.seq = a.seq
  JOIN notices n ON n.id = v.caused_by_notice_id
 WHERE a.cents < 0 AND n.profile NOT LIKE 'eforms%'
 GROUP BY n.profile ORDER BY n DESC;

.print ''
.print '== 12-13: THE 51 SPECIFICALLY (proj-fix, issue 132) ======================='
.print '-- Queries 9/10 answered P4 for the WHOLE set. Those are AGGREGATE figures and'
.print '-- they do NOT settle the 51 ceiling negatives — an over-read I made in my own'
.print '-- issue file before catching it. proj-fix leads 132 with this question because'
.print '-- it is nearly free here and decisive before any distribution work.'

.print ''
.print '-- 12. of the 51 (negatives NOT on result_value), how many have NO negative'
.print '--     anywhere in their chain? -> EXPECT 0 if the fold is faithful for them too.'
.print '--     NONZERO = the fold introduced these specific signs, and 132 becomes a code'
.print '--     defect rather than a data question.'
SELECT COUNT(*) AS ceiling_negatives_with_no_negative_in_chain
  FROM tender_version_amounts a
 WHERE a.cents < 0 AND a.field <> 'result_value'
   AND NOT EXISTS (SELECT 1 FROM tender_versions v2
                     JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                    WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq
                      AND na.cents < 0);

.print ''
.print '-- 13. and how many have an EXACT magnitude match in their chain? High = faithful'
.print '--     copy of a published negative ceiling, which is the interesting case: it'
.print '--     would mean the SOURCE published a negative ceiling, not that we made one.'
SELECT COUNT(*) AS ceiling_negatives_with_exact_match_in_chain
  FROM tender_version_amounts a
 WHERE a.cents < 0 AND a.field <> 'result_value'
   AND EXISTS (SELECT 1 FROM tender_versions v2
                 JOIN notice_amounts na ON na.notice_id = v2.caused_by_notice_id
                WHERE v2.tender_id = a.tender_id AND v2.seq <= a.seq
                  AND na.cents = a.cents);

.print ''
.print '-- 14. do the 51 CLUSTER once isolated from the 17,687? Whole-set scatter does'
.print '--     not rule out a cluster inside the subset — 132 open question 1.'
SELECT n.profile, a.field, COUNT(*) AS n
  FROM tender_version_amounts a
  JOIN tender_versions v ON v.tender_id = a.tender_id AND v.seq = a.seq
  JOIN notices n ON n.id = v.caused_by_notice_id
 WHERE a.cents < 0 AND a.field <> 'result_value'
 GROUP BY n.profile, a.field ORDER BY n DESC;

.print ''
.print '== READING IT: concentration in 3/4 + mirrored positives in 7 => H1, fix the parser.'
.print '== Spread in 2/3/4 + correction subtypes in 5 + no mirrors => H2, fix the CHECK.'
.print '== Mixed => say so and do not force a verdict; both can be true of different rows.'
.print '== AND 9 IS DECISIVE OVER ALL OF IT: nonzero there means the fold introduced the'
.print '== sign, which is a code defect, and no distribution argument survives it. =='
