SELECT notice_id, field_id, COUNT(*) AS n FROM notice_texts WHERE notice_id IN (46869237, 46906609) GROUP BY notice_id, field_id LIMIT 200
