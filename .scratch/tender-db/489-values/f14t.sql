SELECT x.notice_id, trim(rtrim(trim(x.value), ')')) AS sect, y.value AS new_text
FROM notice_texts x JOIN notice_texts y ON y.notice_id = x.notice_id AND y.section_id = x.section_id AND y.field_id = 'TED-NEW_VALUE.TEXT'
WHERE x.notice_id >= 21000000 AND x.notice_id < 21020000 AND x.field_id = 'TED-SECTION'
  AND trim(rtrim(trim(x.value), ')')) IN ('II.1.5','II.1.7','II.2.6','V.2.4')
LIMIT 200
