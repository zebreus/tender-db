WITH probe(x) AS (VALUES (31974975),(31974983),(31974992),(31974994),(31975022),(31975029),(31975030),(31975037),(31975055),(31975069),(31975071),(31975075),(31975088),(31975097),(31975128),(31975151),(31975157),(31975172),(31975198),(31975200),(31975249),(31975271),(31975275),(31975281),(31975284),(31975308),(31975334),(31975335),(31975365),(31975366),(31975384),(31975408),(31975411),(31975430),(31975447),(31975448),(31975450),(31975462),(31975468),(31975471),(31975478),(31975497),(31975523),(31975528),(31975560),(31975574),(31975578),(31975603),(31975608),(31975619),(31975621),(31975627),(31975634),(31975644),(31975652),(31975704),(31975705),(31975710),(31975711),(31975721),(31975748),(31975749),(31975764),(31975768),(31975781),(31975792),(31975794),(31975810),(31975811),(31975836),(31975846),(31975857),(31975872),(31975925),(31975933),(31975934),(31975935),(31975947),(31975963),(31975965),(31975971),(31975977),(31975984))
SELECT p.x AS org, b.organization_id AS parent_org, count(*) AS n, count(DISTINCT m.notice_id) AS notices,
 min(b.name) AS parent_name_min, max(b.name) AS parent_name_max,
 (SELECT po.identifier IS NOT NULL FROM organizations po WHERE po.id = b.organization_id) AS parent_identified,
 min((SELECT min(t.value) FROM notice_texts t WHERE t.notice_id = m.notice_id AND t.section_id = m.section_id AND t.field_id = 'BT-513-Organization-TouchPoint')) AS city_min,
 max((SELECT max(t.value) FROM notice_texts t WHERE t.notice_id = m.notice_id AND t.section_id = m.section_id AND t.field_id = 'BT-513-Organization-TouchPoint')) AS city_max
FROM probe p
CROSS JOIN organization_mentions m ON m.organization_id = p.x
CROSS JOIN notice_sections s ON s.notice_id = m.notice_id AND s.section_id = m.section_id
LEFT JOIN organization_mentions b ON b.notice_id = s.notice_id AND b.section_id = s.parent_section_id
GROUP BY +p.x, +b.organization_id
