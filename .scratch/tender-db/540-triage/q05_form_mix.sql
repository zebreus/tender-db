SELECT code, count(*) AS n, min(notice_id) AS lo, max(notice_id) AS hi FROM notice_codes WHERE notice_id BETWEEN 17786000 AND 17789000 AND field_id = 'TED-FORM' GROUP BY +code LIMIT 100
