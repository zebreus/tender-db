SELECT m.organization_id AS org, m.name AS spelling, count(*) AS n, count(DISTINCT m.notice_id) AS notices, count(DISTINCT n2.publication_id) AS pubs, max(n2.published_at) AS newest_pub FROM organization_mentions m JOIN notices n2 ON n2.id = m.notice_id WHERE m.organization_id = 20415415 GROUP BY m.organization_id, m.name
UNION ALL
SELECT m.organization_id AS org, m.name AS spelling, count(*) AS n, count(DISTINCT m.notice_id) AS notices, count(DISTINCT n2.publication_id) AS pubs, max(n2.published_at) AS newest_pub FROM organization_mentions m JOIN notices n2 ON n2.id = m.notice_id WHERE m.organization_id = 3728305 GROUP BY m.organization_id, m.name
UNION ALL
SELECT m.organization_id AS org, m.name AS spelling, count(*) AS n, count(DISTINCT m.notice_id) AS notices, count(DISTINCT n2.publication_id) AS pubs, max(n2.published_at) AS newest_pub FROM organization_mentions m JOIN notices n2 ON n2.id = m.notice_id WHERE m.organization_id = 10037554 GROUP BY m.organization_id, m.name
UNION ALL
SELECT m.organization_id AS org, m.name AS spelling, count(*) AS n, count(DISTINCT m.notice_id) AS notices, count(DISTINCT n2.publication_id) AS pubs, max(n2.published_at) AS newest_pub FROM organization_mentions m JOIN notices n2 ON n2.id = m.notice_id WHERE m.organization_id = 7081895 GROUP BY m.organization_id, m.name
UNION ALL
SELECT m.organization_id AS org, m.name AS spelling, count(*) AS n, count(DISTINCT m.notice_id) AS notices, count(DISTINCT n2.publication_id) AS pubs, max(n2.published_at) AS newest_pub FROM organization_mentions m JOIN notices n2 ON n2.id = m.notice_id WHERE m.organization_id = 18287356 GROUP BY m.organization_id, m.name
LIMIT 200
