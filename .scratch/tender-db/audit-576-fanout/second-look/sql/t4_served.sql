WITH probe(x) AS (VALUES (2261),(22077939),(2720295),(1209767),(3708),(6041),(1689043),(1217875),(1598093),(3547223),(16276441),(1255752),(1947793),(1241184),(1384164),(31975008),(31974991),(31975031),(31975696))
SELECT p.x, o.name, o.country, o.identifier_kind, o.created_at,
  (SELECT count(*) FROM organization_mentions m WHERE m.organization_id = p.x) AS total,
  (SELECT count(*) FROM organization_mentions m WHERE m.organization_id = p.x AND m.rowid BETWEEN 50437501 AND 50671000) AS band_2759,
  (SELECT count(DISTINCT b.organization_id) FROM organization_mentions m
     CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id
     CROSS JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
    WHERE m.organization_id = p.x AND m.rowid BETWEEN 50437501 AND 50671000) AS band_parent_orgs
FROM probe p CROSS JOIN organizations o ON o.id = p.x
