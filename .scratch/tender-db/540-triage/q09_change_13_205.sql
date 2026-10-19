SELECT count(*) AS sections, count(DISTINCT notice_id) AS notices, min(notice_id) AS lo, max(notice_id) AS hi FROM notice_sections WHERE kind = 'Change' AND notice_id BETWEEN 13000000 AND 20500000
