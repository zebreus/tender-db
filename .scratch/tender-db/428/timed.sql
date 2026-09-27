-- name: s0249-tribunal-miss
SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1
-- params: ["tribunal administratif", "ZZ"]
-- name: s0249-avenueweb-fr
SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1
-- params: ["avenue-web systèmes", "FR"]
-- name: s0249-empty-mt
SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1
-- params: ["", "MT"]
-- name: s0128-reports
SELECT body, computed_at FROM reports WHERE kind = ?
-- params: ["data-quality"]
-- name: s0133-fetches
SELECT id, period, path FROM fetches WHERE id IN (SELECT MAX(id) FROM fetches WHERE source = ? AND kind = ? AND (? IS NULL OR period = ?) GROUP BY period) ORDER BY period
-- params: ["ted", "daily", null, null]
-- name: s0239-parse-state
SELECT id, source, publication_id, profile FROM notices WHERE parse_state = 'parsed' AND id > ? AND id <= ? ORDER BY id LIMIT ?
-- params: [30000000, 30002000, 1000]
-- name: t421-cpv-join
SELECT t.id, c.code FROM tenders t JOIN tender_version_classifications c ON c.tender_id = t.id AND c.seq = t.current_seq AND c.scheme = 'cpv' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-title-join
SELECT t.id, x.value FROM tenders t JOIN tender_version_texts x ON x.tender_id = t.id AND x.seq = t.current_seq AND x.field = 'title' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-buyer-join
SELECT t.id, p.organization_id FROM tenders t JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq AND p.role LIKE '%uyer%' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-cpv-cross
SELECT t.id, c.code FROM tenders t CROSS JOIN tender_version_classifications c ON c.tender_id = t.id AND c.seq = t.current_seq AND c.scheme = 'cpv' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: s0249-hinted-tribunal-miss
SELECT id FROM organizations INDEXED BY organizations_name_country WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1
-- params: ["tribunal administratif", "ZZ"]
-- name: s0249-hinted-empty-miss
SELECT id FROM organizations INDEXED BY organizations_name_country WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1
-- params: ["", "ZZ"]
-- name: s0249-empty-miss
SELECT id FROM organizations WHERE name_norm = ? AND country = ? AND identifier IS NULL LIMIT 1
-- params: ["", "ZZ"]
-- name: c9299-countryless
SELECT id FROM organizations WHERE name_norm = ? AND country IS NULL AND identifier IS NULL LIMIT 1
-- params: ["tribunal administratif"]
-- name: c9950-name-group
SELECT id, country FROM organizations WHERE identifier IS NULL AND country IS NOT NULL AND name_norm = ? ORDER BY country
-- params: ["tribunal administratif"]
