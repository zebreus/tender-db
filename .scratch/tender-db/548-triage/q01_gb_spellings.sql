SELECT m.organization_id AS org, m.name AS spelling, count(*) AS n, min(n2.published_at) AS first_pub, max(n2.published_at) AS newest_pub FROM organization_mentions m JOIN notices n2 ON n2.id = m.notice_id WHERE m.organization_id = 10213161 GROUP BY m.organization_id, m.name
UNION ALL
SELECT m.organization_id AS org, m.name AS spelling, count(*) AS n, min(n2.published_at) AS first_pub, max(n2.published_at) AS newest_pub FROM organization_mentions m JOIN notices n2 ON n2.id = m.notice_id WHERE m.organization_id = 11628199 GROUP BY m.organization_id, m.name
LIMIT 200
