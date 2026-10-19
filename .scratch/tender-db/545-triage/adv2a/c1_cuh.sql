SELECT 'ident' AS q, id, country, identifier, provisional, name FROM organizations WHERE identifier IN ('GBNHSRGT', 'RGT', 'GB-NHS-RGT', 'NHSRGT')
UNION ALL
SELECT 'n:addenbrooke' AS q, id, country, identifier, provisional, name FROM organizations WHERE name_norm >= 'addenbrooke' AND name_norm < 'addenbrookf' AND provisional = 0
UNION ALL
SELECT 'n:cuh' AS q, id, country, identifier, provisional, name FROM organizations WHERE name_norm >= 'cuh' AND name_norm < 'cui' AND provisional = 0
UNION ALL
SELECT 'n:cambridge university hosp' AS q, id, country, identifier, provisional, name FROM organizations WHERE name_norm >= 'cambridge university hosp' AND name_norm < 'cambridge university hosq' AND provisional = 0
