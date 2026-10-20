SELECT currency, rate_date, rate_to_eur, source FROM currency_rates WHERE currency = 'CZK' AND rate_date BETWEEN '2026-10-12' AND '2026-10-20' ORDER BY rate_date
