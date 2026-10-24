SELECT 'org' AS k, 'GBPPONPJXZ4423NDBT' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPJXZ4423NDBT'
UNION ALL
SELECT 'org' AS k, 'GBPPONPXWV6492CGMN' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPXWV6492CGMN'
UNION ALL
SELECT 'org' AS k, 'GBPPONPVMQ1845NYPP' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPVMQ1845NYPP'
UNION ALL
SELECT 'org' AS k, 'GBCOHOC322760' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBCOHOC322760'
UNION ALL
SELECT 'org' AS k, 'GBPPONPGGP8622HHRX' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPGGP8622HHRX'
UNION ALL
SELECT 'org' AS k, 'GBPPONPLMN9829JXLV' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPLMN9829JXLV'
UNION ALL
SELECT 'org' AS k, 'GBPPONPRYY6263NQHG' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPRYY6263NQHG'
UNION ALL
SELECT 'org' AS k, 'GBPPONPBWD3998QHRQ' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPBWD3998QHRQ'
UNION ALL
SELECT 'org' AS k, 'GBPPONPGJB3773RCMH' AS q, id AS org, name AS name, provisional AS n, NULL AS lo, NULL AS hi FROM organizations WHERE identifier = 'GBPPONPGJB3773RCMH'
UNION ALL
SELECT 'm' AS k, NULL AS q, organization_id AS org, name AS name, count(*) AS n, min(notice_id) AS lo, max(notice_id) AS hi FROM organization_mentions WHERE organization_id IN (SELECT id FROM organizations WHERE identifier IN ('GBPPONPJXZ4423NDBT', 'GBPPONPXWV6492CGMN', 'GBPPONPVMQ1845NYPP', 'GBCOHOC322760', 'GBPPONPGGP8622HHRX', 'GBPPONPLMN9829JXLV', 'GBPPONPRYY6263NQHG', 'GBPPONPBWD3998QHRQ')) GROUP BY organization_id, name
