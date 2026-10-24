WITH probe(x) AS (VALUES (31975008),(31975019),(31974991),(31975180),(31975009),(31975006),(31975004),(31975568),(31975031),(31975696))
SELECT p.x, count(*) AS tpo_mentions, count(DISTINCT m.notice_id) AS notices, count(DISTINCT b.organization_id) AS parent_orgs, count(b.organization_id) AS with_parent_mention
FROM probe p CROSS JOIN organization_mentions m ON m.organization_id = p.x
CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id
LEFT JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
GROUP BY +p.x
