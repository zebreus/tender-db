import json, sys, unicodedata, collections
d = json.load(open(sys.argv[1])); cols = d['columns']
rows = [dict(zip(cols, r)) for r in d['rows'] if 31974972 <= r[0] <= 31978722]
def org_name_norm(name):
    collapsed = ' '.join(name.lower().split())
    s = collapsed
    while s and (s[-1] in '.,;' or s[-1].isspace()):
        s = s[:-1]
    return s if s else collapsed
mism = [r for r in rows if org_name_norm(r['name']) != r['name_norm']]
print('stored name_norm != org_name_norm(name):', len(mism))
for r in mism[:20]: print('  ', r['id'], repr(r['name']), repr(r['name_norm']))
# NFC-equivalent duplicates
g = collections.defaultdict(list)
for r in rows: g[(unicodedata.normalize('NFC', r['name_norm']), r['country'])].append(r)
nfc = {k:v for k,v in g.items() if len(v) > 1}
print('NFC-equal name_norm groups:', len(nfc))
for k,v in nfc.items():
    for r in v: print('  ', r['id'], r['country'], r['tpo_mentions'], repr(r['name']), [hex(ord(c)) for c in r['name'] if ord(c) > 127])
# NFKC + casefold
g2 = collections.defaultdict(list)
for r in rows: g2[(' '.join(unicodedata.normalize('NFKC', r['name_norm']).casefold().split()), r['country'])].append(r)
print('NFKC/casefold-equal groups:', sum(1 for v in g2.values() if len(v)>1))
for k,v in g2.items():
    if len(v)>1:
        for r in v: print('  ', r['id'], repr(r['name']))
# all name_norm duplicates ignoring country
g3 = collections.defaultdict(list)
for r in rows: g3[r['name_norm']].append(r)
print('same name_norm, any country:', sum(1 for v in g3.values() if len(v)>1))
for k,v in g3.items():
    if len(v)>1: print('  ', [(r['id'], r['country'], r['tpo_mentions']) for r in v], repr(k))
print('countries null:', sum(1 for r in rows if r['country'] is None), 'identifier set:', sum(1 for r in rows if r['identifier'] is not None), 'non-provisional:', sum(1 for r in rows if r['provisional'] != 1))
print('mentions != tpo_mentions:', sum(1 for r in rows if r['mentions'] != r['tpo_mentions']))
