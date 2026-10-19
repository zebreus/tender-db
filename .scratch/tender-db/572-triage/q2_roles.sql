SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30953692 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30913738 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30913798 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30916628 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30930308 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30968739 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30914820 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30913722 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30926924 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30919377 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30926792 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30928312 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30929723 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30913750 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30917275 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30913763 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30916821 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30967593 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30917832 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 31546270 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30974616 GROUP BY m.name, p.role
UNION ALL
SELECT p.organization_id AS org, m.name AS name, p.role AS role, count(DISTINCT p.mention_notice_id) AS notices, count(DISTINCT p.tender_id) AS tenders, count(*) AS rows FROM tender_version_parties p CROSS JOIN organization_mentions m ON m.notice_id = p.mention_notice_id AND m.section_id = p.mention_section_id WHERE p.organization_id = 30914436 GROUP BY m.name, p.role
