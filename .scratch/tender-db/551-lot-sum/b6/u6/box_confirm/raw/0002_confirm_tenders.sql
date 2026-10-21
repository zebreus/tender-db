SELECT t.id, t.source, t.current_seq, t.current_published_at, t.current_value_derivation, t.current_value_cents, t.current_value_currency, t.current_value_eur_cents, t.current_value_derivation_detail, t.projection_epoch FROM tenders t WHERE t.id IN (909, 585917, 5995030, 6465510, 8426318)
-- plan: SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
