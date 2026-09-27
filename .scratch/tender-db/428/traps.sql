-- name: t421-cpv-join
SELECT t.id, c.code FROM tenders t JOIN tender_version_classifications c ON c.tender_id = t.id AND c.seq = t.current_seq AND c.scheme = 'cpv' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-cpv-cross
SELECT t.id, c.code FROM tenders t CROSS JOIN tender_version_classifications c ON c.tender_id = t.id AND c.seq = t.current_seq AND c.scheme = 'cpv' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-title-join
SELECT t.id, x.value FROM tenders t JOIN tender_version_texts x ON x.tender_id = t.id AND x.seq = t.current_seq AND x.field = 'title' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-title-cross
SELECT t.id, x.value FROM tenders t CROSS JOIN tender_version_texts x ON x.tender_id = t.id AND x.seq = t.current_seq AND x.field = 'title' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-buyer-join
SELECT t.id, p.organization_id FROM tenders t JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq AND p.role LIKE '%uyer%' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t421-buyer-cross
SELECT t.id, p.organization_id FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq AND p.role LIKE '%uyer%' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t417-notice-codes
SELECT t.id, c.code FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq JOIN notice_codes c ON c.notice_id = v.caused_by_notice_id AND c.field_id = 'TXT-NC' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t417-notice-codes-plus
SELECT t.id, c.code FROM tenders t JOIN tender_versions v ON v.tender_id = t.id AND v.seq = t.current_seq JOIN notice_codes c ON c.notice_id = v.caused_by_notice_id AND +c.field_id = 'TXT-NC' WHERE t.id BETWEEN 4000000 AND 4002000
-- name: t329-changes-in
SELECT cursor, op FROM changes WHERE entity_kind = 'tender' AND entity_id IN (4000001, 4000002, 4000003, 4000004, 4000005, 4000006, 4000007, 4000008, 4000009, 4000010, 4000011, 4000012, 4000013, 4000014, 4000015, 4000016, 4000017, 4000018, 4000019, 4000020)
-- name: t323-parse-state
SELECT COUNT(*) FROM notices WHERE parse_state = 'parsed' AND id > 30000000 AND id <= 30002000
-- name: t-fetch-groupby
SELECT profile, COUNT(*) FROM notices WHERE fetch_id = 12000 GROUP BY profile
-- name: t-kind-in
SELECT COUNT(*) FROM tenders WHERE kind IN ('procedure', 'Lot') AND id BETWEEN 4000000 AND 4100000
-- name: t-winner-awards
SELECT t.id, t.current_seq FROM tender_version_result_winners w JOIN tenders t ON t.id = w.tender_id AND w.seq = t.current_seq WHERE w.organization_id = 357 LIMIT 100
-- name: t-buyer-country-top
SELECT o.id, COUNT(*) FROM tender_version_parties p JOIN organizations o ON o.id = p.organization_id WHERE p.role = 'buyer' AND o.country = 'MT' GROUP BY o.id ORDER BY 2 DESC LIMIT 20
-- name: t-lots-results-winners
SELECT l.id, w.organization_id FROM lots l JOIN lot_results r ON r.tender_id = l.tender_id JOIN tender_version_result_winners w ON w.lot_result_id = r.id WHERE l.tender_id BETWEEN 4000000 AND 4000500
-- name: t-cpv-code-tenders
SELECT t.id FROM tender_version_classifications c JOIN tenders t ON t.id = c.tender_id AND t.current_seq = c.seq WHERE c.scheme = 'cpv' AND c.code = '45233140' LIMIT 200
-- name: t-org-name-parties
SELECT p.tender_id FROM organizations o JOIN tender_version_parties p ON p.organization_id = o.id WHERE o.name_norm >= 'stadt wien' AND o.name_norm < 'stadt wiep' LIMIT 200
-- name: t-v-tenders-country
SELECT * FROM v_tenders WHERE id BETWEEN 4000000 AND 4002000
