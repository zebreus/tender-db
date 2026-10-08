-- name: tenders_range
SELECT id, current_seq, source FROM tenders WHERE id BETWEEN 8600000 AND 8600500
-- name: lots_range
SELECT id, tender_id, lot_key FROM lots WHERE tender_id BETWEEN 8600000 AND 8600500
-- name: tvl_range
SELECT tender_id, seq, lot_id, kind FROM tender_version_lots WHERE tender_id BETWEEN 8600000 AND 8600500
-- name: amounts_range
SELECT tender_id, seq, lot_id, field, cents, currency, eur_cents, quality FROM tender_version_amounts WHERE tender_id BETWEEN 8600000 AND 8600500
-- name: lot_results_range
SELECT tender_id, seq, lot_id, awarded_cents, awarded_currency FROM tender_version_lot_results WHERE tender_id BETWEEN 8600000 AND 8600500 AND awarded_cents IS NOT NULL
