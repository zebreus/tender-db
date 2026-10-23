-- name: r2_summary
SELECT country, COUNT(*), SUM(created_at > 1788739200), MAX(id), SUM(identifier IS NOT NULL), MIN(created_at), MAX(created_at) FROM organizations WHERE country IN ('GP','MQ','GF','RE','YT','PM','BL','MF','WF','AX','GL','SJ') GROUP BY country
-- name: r3_post358_rows
SELECT o.id, o.country, o.name_norm, o.created_at, o.identifier, (SELECT COUNT(*) FROM organization_mentions m WHERE m.organization_id = o.id) FROM organizations o WHERE o.country IN ('GP','MQ','GF','RE','YT','PM','BL','MF','WF','AX','GL','SJ') AND o.created_at > 1788739200 LIMIT 300
