SELECT o.id,
 (SELECT b.organization_id || '|' || coalesce(b.name, '') FROM organization_mentions m
   CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id
   CROSS JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
   WHERE m.organization_id = o.id LIMIT 1) AS parent
FROM organizations o WHERE o.id BETWEEN 31974972 AND 31978722
