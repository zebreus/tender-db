# Builds r12_refs.sql: mention counts of every destination row tied in verdicts2b.py's pools (most-referenced
# tiebreak): one organization_mentions_org range per id, single-table aggregate (same shape as u2a/gen_r9.py arm 1).
import json
need = sorted({i for v in json.load(open('verdicts2b.json')) for i in v.get('needs_refcount', [])})
arms = [f"SELECT 'rc|{i}' AS k, count(*) AS a FROM organization_mentions WHERE organization_id = {i}" for i in need]
arms = arms[:1] + [a.replace(' AS k', '').replace(' AS a', '') for a in arms[1:]]
open('r12_refs.sql', 'w').write('\nUNION ALL\n'.join(arms) + '\n')
print(len(need), 'refcounts')
