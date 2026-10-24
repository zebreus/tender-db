import json, unicodedata, re, sys, collections
d = json.load(open(sys.argv[1]))
cols = d['columns']
rows = [dict(zip(cols, r)) for r in d['rows']]
new = [r for r in rows if 31974978 <= r['id'] <= 31978722]
pre6 = [r for r in rows if 31974972 <= r['id'] <= 31974977]
print('new', len(new), 'pre6', len(pre6))
def loose(s):
    s = unicodedata.normalize('NFKD', s)
    s = ''.join(c for c in s if not unicodedata.combining(c))
    s = s.lower()
    s = re.sub(r'\([^)]*\)?', ' ', s)
    s = re.sub(r'[^\w]+', ' ', s)
    return ' '.join(s.split())
def n2(name):  # ingest::project::match_norm (Latin accents NOT folded)
    out=[]; gap=False
    for c in name:
        if c.isalnum():
            if gap and out: out.append(' ')
            gap=False; out.append(c.lower())
        else: gap=True
    return ''.join(out)
g = collections.defaultdict(list)
for r in new + pre6:
    g[(loose(r['name']), r['country'])].append(r)
cl = {k:v for k,v in g.items() if len([x for x in v if x['id']>=31974978])>=2}
tot = sum(len([x for x in v if x['id']>=31974978]) for v in cl.values())
print('clusters', len(cl), 'orgs', tot, 'surplus', tot-len(cl))
gn2 = collections.defaultdict(list)
for r in new: gn2[(n2(r['name']), r['country'])].append(r)
cn2 = {k:v for k,v in gn2.items() if len(v)>=2}
print('N2 clusters', len(cn2), 'orgs', sum(len(v) for v in cn2.values()))
out=[]
for k,v in sorted(cl.items(), key=lambda kv:(-len(kv[1]), kv[0])):
    n2keys = len({n2(x['name']) for x in v})
    out.append({'key':k[0],'country':k[1],'n':len(v),'n2_distinct':n2keys,'members':[[x['id'],x['tpo_mentions'],x['name']] for x in v]})
json.dump(out, open(sys.argv[2],'w'), ensure_ascii=False, indent=1)
for c in out[:40]:
    print(c['n'], c['country'], c['n2_distinct'], c['key'])
    for m in c['members']: print('    ', m)
