SELECT id, current_value_eur_cents, current_value_cents, current_value_currency, current_value_derivation_detail FROM tenders WHERE current_value_eur_cents >= 10000000000 AND current_value_derivation = 'lot-sum-awards' ORDER BY current_value_eur_cents DESC LIMIT 200
-- plan: SEARCH tenders USING INDEX tenders_current_value_eur (current_value_eur_cents>=?)
