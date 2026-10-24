"""Emit the judged hit table for gate.md (NEW population), one row per org."""
import re, sys
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/586-u1')
from pop import new_orgs, fold
from gate import stems, gate_new
TRIB = re.compile(r'tribunal|juzgado|\btar\b')
def judge(r, ms):
    n = r['name']; f = fold(n)
    if 'WHOLE' in ms:
        return 'label, no body'
    if 'HONORIFIC' in ms:
        return 'office holder by title (UGAP mediator)' if 'diateur' in n else 'person'
    if 'PLATFORM' in ms:
        return 'platform / portal label'
    if TRIB.search(f):
        return 'prose/compound naming a tribunal (T)'
    return 'prose'
rows = []
for r in new_orgs():
    s = stems(r['name'])
    if s:
        ms = {m for m, _ in s}
        rows.append((r['n'], r['key'], r['name'].replace('|', '/').replace('\n', ' ')[:95], ','.join(sorted(ms)),
                     'v' if gate_new(r['name'], found=False) else 'f', judge(r, ms), ' '.join((r['parent'] or '').split())[:45].replace('|', '/')))
rows.sort(key=lambda x: (-x[0], x[1]))
print('| mentions | org | name | mechanism | list | parent (first) | judgment |')
print('|---:|---|---|---|---|---|---|')
for n, k, name, m, v, j, p in rows:
    print(f'| {n} | {k} | {name} | {m} | {v} | {p} | {j} |')
print(f'\n{len(rows)} orgs, {sum(r[0] for r in rows)} mentions')
