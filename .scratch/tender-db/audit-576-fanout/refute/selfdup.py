import json, unicodedata, re, collections, random
d = json.load(open('refute/raw/04-r4_parent_one.json'))
new = json.load(open('box/raw/04-s04_newrows.json')); byid = {r[0]: dict(zip(new['columns'], r)) for r in new['rows']}
def org_name_norm(name):
    c = ' '.join(name.lower().split()); s = c
    while s and (s[-1] in '.,;' or s[-1].isspace()): s = s[:-1]
    return s if s else c
LABEL = r'\((?:\s*(?:punto de contacto|pc|otro punto de contacto|sede electr[oó]nica|registro(?: electr[oó]nico)?|[A-ZÁÉÍÓÚ.\s]{2,12})\s*)\)'
def fold(s, strip_paren=True):
    s = unicodedata.normalize('NFKD', s); s = ''.join(c for c in s if not unicodedata.combining(c)).lower()
    if strip_paren: s = re.sub(r'\([^)]*\)?', ' ', s)
    s = re.sub(r'[^\w]+', ' ', s); return ' '.join(s.split())
cls = collections.Counter(); ment = collections.Counter(); ex = collections.defaultdict(list)
for oid, parent in d['rows']:
    o = byid[oid]
    if parent is None:
        cls['no parent mention'] += 1; ment['no parent mention'] += o['tpo_mentions']; continue
    pid, pname = parent.split('|', 1)
    a, b = fold(o['name']), fold(pname)
    if org_name_norm(o['name']) == org_name_norm(pname):
        k = 'N1-equal to parent (should have aliased!)'
    elif a == b:
        k = 'equal to parent after accent/case/punct/paren fold'
    elif a and b and (b.startswith(a + ' ') or a.startswith(b + ' ')):
        k = 'prefix of / extends parent name'
    elif a and b and (a in b or b in a):
        k = 'contained in parent name'
    else:
        k = 'different'
    cls[k] += 1; ment[k] += o['tpo_mentions']; ex[k].append((oid, o['tpo_mentions'], o['name'], pname))
tot = sum(cls.values())
print('orgs', tot)
for k, v in cls.most_common(): print(f'  {k}: {v} orgs ({100*v/tot:.1f}%), {ment[k]} mentions')
rnd = random.Random('576-refute-selfdup')
for k in ['N1-equal to parent (should have aliased!)', 'equal to parent after accent/case/punct/paren fold', 'prefix of / extends parent name', 'contained in parent name']:
    print('\n##', k)
    xs = sorted(ex[k], key=lambda x: -x[1])
    for x in xs[:12]: print('   ', x[0], x[1], repr(x[2][:70]), '||', repr(x[3][:80]))
    if len(xs) > 12:
        print('   salted:')
        for x in rnd.sample(xs[12:], min(8, len(xs) - 12)): print('   ', x[0], x[1], repr(x[2][:70]), '||', repr(x[3][:80]))
json.dump({k: v for k, v in ex.items()}, open('refute/selfdup_classes.json', 'w'), ensure_ascii=False, indent=0)
