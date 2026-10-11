# Census of every GB-NHS raw on the listed registry holders (513 design B).
import json, re, sys, collections, os
U1 = sys.argv[1]
rows = json.load(open(f'{U1}/adjudication-rows.json'))
mc = collections.defaultdict(dict)
for line in open(f'{U1}/mention-counts-by-org.jsonl'):
    for org, raw, n in json.loads(line)['rows']:
        mc[org][raw] = mc[org].get(raw, 0) + n
def norm(raw):
    return ''.join(c for c in (raw or '') if c.isascii() and c.isalnum()).upper()
def minted_today(v):
    # project.rs:8710-8720: len<4 / no digit / all-zero digits / single-char run refuse
    if len(v) < 4 or not any(c.isdigit() for c in v): return False
    if all(c == '0' for c in v if c.isdigit()): return False
    if all(c == v[0] for c in v[1:]): return False
    return True
reg = [r for r in rows if r['side'] == 'registry']
orgs = {}
for r in reg: orgs.setdefault(r['org_id'], []).append(r)
print('registry rows', len(reg), 'distinct orgs', len(orgs))
tails = collections.defaultdict(lambda: collections.Counter())
kinds = collections.Counter()
for org in orgs:
    for raw, n in mc[org].items():
        if raw is None: kinds['null'] += n; continue
        v = norm(raw)
        if v.startswith('GBNHS'):
            t = v[5:]
            if len(t) == 3:
                kinds['gbnhs3_mints_today' if minted_today(v) else 'gbnhs3_refused_today'] += n
                if not minted_today(v): tails[t][org] += n
            else:
                kinds[f'gbnhs_len{len(t)}_' + ('mint' if minted_today(v) else 'refused')] += n
        elif len(v) == 3:
            kinds['bare3'] += n
        else:
            kinds['other'] += n
print(dict(kinds))
listed = {r['code'] for r in reg}
print('distinct refused 3-char GBNHS tails on these orgs', len(tails), 'listed among them', len(set(tails) & listed))
extra = sorted(set(tails) - listed)
print('extra tails', len(extra))
for t in extra:
    print(' ', t, {o: (n, orgs[o][0]['name'][:50], [x['code'] for x in orgs[o]]) for o, n in tails[t].items()})
