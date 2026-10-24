"""notes_sql.py -> prints the markdown section listing every /v1/sql statement sent, with its local plans."""
import json, os
HERE = '/home/user/tender-db/.scratch/tender-db/586-u3'
st = json.load(open(os.path.join(HERE, 'raw', 'state', 'sql_state.json')))
meta = {}
for f in os.listdir(os.path.join(HERE, 'sqlsrc')):
    if f.endswith('.meta.json'):
        meta[f[:-10]] = json.load(open(os.path.join(HERE, 'sqlsrc', f)))
print('| # | name | purpose / window | sent (UTC) | secs | rows | answer |')
print('|---|---|---|---|---|---|---|')
purpose = {'v01_labels': 'A4: tender_version_parties roles of orgs 18044191 / 18044192 (two equality probes, UNION ALL)',
           's_mentions_b': 'organization_mentions of the 29 minted-sample notices (PK prefix probes, UNION ALL)',
           's_mentions_c': 'organization_mentions of the 29 aliased-sample notices (PK prefix probes, UNION ALL)',
           'x540072': 'C fixed: the aliased statement on tender 540072 alone (SARGA exhibit)'}
for e in st['log']:
    m = meta.get(e['name'])
    p = purpose.get(e['name'])
    if not p and m:
        kind = {'m': 'B minted general', 'n': 'B minted NL AddInfo/TenderReceipt', 'o': 'B minted other roles', 'a': 'C aliased'}[m['kind']]
        p = f"{kind} i={m['i']} (salt {m['salt']}, sha256 {m['sha256_hex16']}, s={m['s']}) ids {m['a']}..{m['b']}"
    ans = 'JSON rows+columns, truncated=false' if e['ok'] and e['truncated'] is False else f"NOT AN ANSWER {e.get('body_head','')}"
    print(f"| {e['n']} | {e['name']} | {p} | {e['sent_at']} | {e['secs']} | {e['rows']} | {ans} |")
print()
for e in st['log']:
    n = e['name']
    sql = open(os.path.join(HERE, 'raw', 'sql', n + '.sql')).read().strip()
    plan = open(os.path.join(HERE, 'raw', 'sql', n + '.plan')).read().rstrip()
    if n.startswith('s_mentions'):
        parts = sql.split(' UNION ALL ')
        sql = parts[0] + f' UNION ALL … ({len(parts)} branches, one per notice; full text in raw/sql/{n}.sql)'
        pl = plan.splitlines()
        plan = '\n'.join(pl[:5] + [f'  … the same SEARCH (notice_id=?) for each of the {len(parts)} branches'] + [l for l in pl if l.startswith('#')][1:2] + pl[pl.index('# plan-idx')+1:pl.index('# plan-idx')+5] + ['  …'])
    print(f'### {e["n"]}. {n}\n\n```sql\n{sql}\n```\n\nLocal EXPLAIN QUERY PLAN (turso plan-probe):\n\n```\n{plan}\n```\n')
