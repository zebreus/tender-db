# 284 — the REST name-prefix org search silently drops `identifier` and `buyer` while reporting them honoured

Status: FIXED in working tree (2026-08-26, owner), awaiting gate+deploy
Kind: correctness / API honesty
Severity: MEDIUM (unauthenticated; a confidently over-broad result)
Relates to: 217-B (the name-ordered builder), 216 (the index)
Found by: the 2026-08-26 read-path review; verified NEW.

## The bug

`/v1/organizations` routes any non-SSE request carrying `name_prefix` to
`organizations_by_name` (v1/mod.rs ~1051). That builder (read.rs ~2525) applied
only the name range plus `country` and `kind` — it never emitted `o.identifier=?`
or `o.id=?` (buyer), which the id-ordered `organizations_query` does. But both
`identifier` and `buyer` are in `Collection::Organizations.honoured_params()`
(read.rs ~518), and the handler computes `ignored_filters` by subtracting honoured
from provided — so on the name path they land in `honoured` and are NOT reported
as ignored.

Result: `GET /v1/organizations?name_prefix=siemens&identifier=DE811` returns EVERY
org named siemens* (identifier ignored) with `ignored_filters: []`, so the client
believes the identifier filter applied. Same for `&buyer=<id>`.

## Fix (shipped)

Apply `identifier` (`o.identifier = ?`) and `buyer` (`o.id = ?`) in
`organizations_by_name` exactly as `organizations_query` does. Test
`the_name_search_honours_identifier_and_buyer_filters` (org_name_search.rs) pins
both: prefix alone returns the slice, `+identifier` and `+buyer` each narrow to the
one matching org (red against the old builder).
