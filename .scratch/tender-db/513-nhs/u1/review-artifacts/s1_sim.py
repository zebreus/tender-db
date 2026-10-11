import json, re, sys, collections
rows = json.load(open(sys.argv[1]))
GENERIC = set('nhs the and of trust foundation integrated care board ft icb limited ltd university hospital hospitals teaching healthcare health services service partnership community mental group commissioning support unit csu for in t as trading c o'.split())
def toks(n):
    s = (n or '').lower().replace('&', ' and ').replace("'", '')
    return {t for t in re.findall(r'[a-z0-9]+', s) if t not in GENERIC}
by = collections.defaultdict(list)
for r in rows:
    if r['side'] == 'registry': by[r['code']].append(r)
moved = flagged = 0
flag_by_class = collections.Counter(); moved_by_class = collections.Counter()
for code, rs in by.items():
    top = max(rs, key=lambda r: r['coded'])['name']
    T = toks(top)
    for r in rs:
        moved += r['coded']; moved_by_class[r['final']] += r['coded']
        A = toks(r['name'])
        if A and T and not (A & T):
            flagged += r['coded']; flag_by_class[r['final']] += r['coded']
print('moved coded mentions', moved, 'S1-flagged', flagged, f'{100*flagged/moved:.2f}%')
print('flagged by class', dict(flag_by_class)); print('moved by class', dict(moved_by_class))
