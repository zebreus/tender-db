SELECT count(*) AS sections, min(notice_id) AS lo, max(notice_id) AS hi FROM notice_sections WHERE kind = 'Change' AND notice_id BETWEEN 1 AND 20500000
