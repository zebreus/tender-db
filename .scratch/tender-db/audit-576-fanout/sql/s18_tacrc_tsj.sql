SELECT count(*) AS mentions, sum(m.section_id LIKE 'TPO-%') AS tpo_mentions, count(DISTINCT m.notice_id) AS notices FROM organization_mentions m WHERE m.organization_id = 22077939
