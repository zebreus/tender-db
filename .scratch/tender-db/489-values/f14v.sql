SELECT trim(rtrim(trim(x.value), ')')) AS sect, COUNT(*) AS blocks, COUNT(DISTINCT x.notice_id) AS notices
FROM notice_texts x
WHERE x.notice_id >= 21000000 AND x.notice_id < 21020000 AND x.field_id = 'TED-SECTION'
  AND trim(rtrim(trim(x.value), ')')) IN ('II.1.5','II.1.7','II.2.6','V.2.4')
  AND EXISTS (SELECT 1 FROM notice_texts y WHERE y.notice_id = x.notice_id AND y.section_id = x.section_id AND y.field_id = 'TED-NEW_VALUE.TEXT')
GROUP BY 1
