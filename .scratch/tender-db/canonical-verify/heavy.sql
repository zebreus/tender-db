-- HEAVY canonical-layer checks — STANDALONE, stock sqlite3, READ-ONLY.
-- Run during the service-STOP window (turso holds an exclusive lock while running,
-- even idle → "database is locked"; stop the service, or use a post-build snapshot):
--
--   sqlite3 -readonly "file:/data/db/tender-db.db?mode=ro" < heavy.sql
--   sqlite3 -readonly "file:/data/db/snapshots/<post-build>.db?mode=ro" < heavy.sql
--
-- Real SQLite has no turso hash cliff, so these high-cardinality GROUP BYs /
-- anti-joins / correlated scans run fine here (minutes at 6.96M/12.36M/30M rows).
-- Every "→ EXPECT 0 rows / 0 count" is a HARD-FAIL gate unless marked EYEBALL.

.headers on
.mode box
.print '== HEAVY canonical-layer checks (stock sqlite3, read-only) =='

.print ''
.print '-- 2.3 one notice → exactly ONE tender (anti junk-merge) → EXPECT 0 ROWS [GATE]'
SELECT caused_by_notice_id, COUNT(*) AS in_n_tenders
  FROM tender_versions GROUP BY caused_by_notice_id HAVING COUNT(*) > 1 LIMIT 50;

.print ''
.print '-- 2.4 head pointer matches its version publication date → EXPECT 0 [GATE]'
SELECT COUNT(*) AS head_pub_mismatch
  FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq
 WHERE t.current_published_at <> v.published_at;

.print ''
.print '-- 2.5 current_seq is the maximum seq → EXPECT 0 [GATE]'
SELECT COUNT(*) AS head_not_max
  FROM tenders t
 WHERE EXISTS (SELECT 1 FROM tender_versions v
                WHERE v.tender_id = t.id AND v.seq > t.current_seq);

.print ''
.print '-- 2.8 org identity uniqueness among identified profiles → EXPECT 0 ROWS [GATE]'
SELECT country, identifier_kind, identifier, COUNT(*) AS n
  FROM organizations WHERE identifier IS NOT NULL
 GROUP BY country, identifier_kind, identifier HAVING COUNT(*) > 1 LIMIT 50;

.print ''
.print '-- 3.2 top-30 tenders by version count → EYEBALL: legit framework vs junk-hub'
SELECT id, source, kind, procedure_key, island_notice_id, current_seq
  FROM tenders ORDER BY current_seq DESC LIMIT 30;

.print ''
.print '-- 3.4 top-30 orgs by mention count → EYEBALL: real big buyer vs over-merge'
SELECT organization_id, COUNT(*) AS mentions
  FROM organization_mentions GROUP BY organization_id ORDER BY mentions DESC LIMIT 30;

.print ''
.print '-- 3.5b tenders whose CURRENT version has no title → EYEBALL the count'
SELECT COUNT(*) AS current_versions_without_title
  FROM tenders t
 WHERE NOT EXISTS (SELECT 1 FROM tender_version_texts x
                    WHERE x.tender_id = t.id AND x.seq = t.current_seq AND x.field = 'title');

.print ''
.print '== 5. Referential integrity (orphans) — EXPECT 0 each [GATE] =='

.print '-- 5.1 satellites → tender_versions(tender_id,seq)'
SELECT COUNT(*) AS orphan_texts       FROM tender_version_texts x
  LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL;
SELECT COUNT(*) AS orphan_parties     FROM tender_version_parties x
  LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL;
SELECT COUNT(*) AS orphan_amounts     FROM tender_version_amounts x
  LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL;
SELECT COUNT(*) AS orphan_dates       FROM tender_version_dates x
  LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL;
SELECT COUNT(*) AS orphan_classif     FROM tender_version_classifications x
  LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL;
SELECT COUNT(*) AS orphan_lot_results FROM tender_version_lot_results x
  LEFT JOIN tender_versions v ON v.tender_id=x.tender_id AND v.seq=x.seq WHERE v.tender_id IS NULL;

.print '-- 5.2 tender_versions → tenders'
SELECT COUNT(*) AS orphan_versions FROM tender_versions v
  LEFT JOIN tenders t ON t.id=v.tender_id WHERE t.id IS NULL;

.print '-- 5.3 parties/winners → organizations'
SELECT COUNT(*) AS orphan_party_orgs FROM tender_version_parties p
  LEFT JOIN organizations o ON o.id=p.organization_id WHERE o.id IS NULL;
SELECT COUNT(*) AS orphan_winner_orgs FROM tender_version_result_winners w
  LEFT JOIN organizations o ON o.id=w.organization_id WHERE o.id IS NULL;

.print '-- 5.4 lots/results → tenders'
SELECT COUNT(*) AS orphan_lots       FROM lots l       LEFT JOIN tenders t ON t.id=l.tender_id WHERE t.id IS NULL;
SELECT COUNT(*) AS orphan_lotresults FROM lot_results r LEFT JOIN tenders t ON t.id=r.tender_id WHERE t.id IS NULL;

.print '-- 5.5 mentions → organizations (the ~30M table; slowest anti-join)'
SELECT COUNT(*) AS orphan_mention_orgs FROM organization_mentions m
  LEFT JOIN organizations o ON o.id=m.organization_id WHERE o.id IS NULL;

.print ''
.print '-- 5.6 OPTIONAL authoritative whole-DB FK sweep (slow at 254GB; uncomment to run):'
.print '--   PRAGMA foreign_key_check;'
-- PRAGMA foreign_key_check;

.print ''
.print '== HEAVY checks done. Any nonzero [GATE] count / any returned dup-row = FAIL. =='
