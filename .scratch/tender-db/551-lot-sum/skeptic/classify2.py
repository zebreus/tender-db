import json, os, collections
D = os.path.dirname(os.path.abspath(__file__))
rows = {json.loads(l)['id']: json.loads(l) for l in open(os.path.join(D, '..', 'results.jsonl'))}
SKIP = {'TED-VALUE', 'BT-118-NoticeResult', 'BT-1118-NoticeResult', 'TED-INFO_MODIFICATIONS.VAL_TOTAL_BEFORE'}
out = []
for t in [int(x) for x in open(os.path.join(D, 'class_ids.txt')).read().split()]:
    r = rows[t]
    d = json.load(open(os.path.join(D, 'api', f't{t}.json')))
    n = [v['caused_by_notice_id'] for v in d['versions'] if v['seq'] == d['version']][0]
    nc = json.load(open(os.path.join(D, 'api', f'n{n}.json')))
    proc, tedvalue, b118 = set(), set(), set()
    for s in nc['sections']:
        for v in s['values']:
            if v['type'] != 'amount' or s['kind'] not in ('Notice', 'Modification'):
                continue
            k = (v.get('currency'), v.get('cents'))
            if v['field_id'] == 'TED-VALUE': tedvalue.add(k)
            elif v['field_id'] in ('BT-118-NoticeResult', 'BT-1118-NoticeResult'): b118.add(k)
            elif v['field_id'] not in SKIP: proc.add(k)
    e = r['elected'] or {}
    ek = (e.get('currency'), e.get('cents'))
    status = {}
    for x in r['reasons']:
        if x['scope'] == 'procedure' or x['scope'].startswith('award-copy'):
            status.setdefault((x['currency'], x['cents']), set()).add(x['status'] + ('(' + x['detail'][:10] + ')' if x['status'] in ('refused', 'not-electable') and x['detail'] else ''))
    def bad(st): return any(s.startswith('refused') or s.startswith('not-electable') for s in st)
    if not proc:
        cls = 'NO-PROC'
    elif ek in proc:
        cls = 'PROC==HEAD'
    elif all(bad(status.get(p, set())) for p in proc):
        cls = 'PROC-REFUSED/NE'
    else:
        cls = 'PROC-OTHER'
    # lot arithmetic from the API (head lots)
    lotvals = collections.defaultdict(dict)
    for a in d['amounts']:
        if a['lot'] and a['field'] in ('estimated_value', 'framework_maximum') and a['value']['currency'] == e.get('currency'):
            lotvals[a['field']][a['lot']] = max(lotvals[a['field']].get(a['lot'], 0), a['value']['cents'])
    dbl = False
    if e.get('lot_key') and e.get('field') in lotvals:
        others = sum(v for k, v in lotvals[e['field']].items() if k != e['lot_key'])
        dbl = ek[1] and abs(others - ek[1]) <= max(100, ek[1] // 1000)
    rat = (r.get('ratio') or {})
    out.append(dict(cls=cls, id=t, pop=r['pop'], grp='band' if r.get('in_band') else ('537' if r.get('r537_refused') else 'R'),
                    scope=r['elected_scope'][:9], field=e.get('field'), cur=e.get('currency'), cents=e.get('cents'),
                    same=round(rat.get('same') or 0, 3), uni=any((r.get('uniform') or {}).values()),
                    proc=sorted(proc)[:3], st=[sorted(status.get(p, {'(not stored)'})) for p in sorted(proc)[:3]],
                    tedvalue=sorted(tedvalue)[:2], b118=sorted(b118)[:2], head_eq_sum_others=dbl, nlot=r['lots']['Lot'], title=d['title'][:70]))
json.dump(out, open(os.path.join(D, 'classify2.json'), 'w'))
for o in sorted(out, key=lambda o: (o['cls'], o['grp'], o['id'])):
    print(o['cls'], o['id'], o['grp'], o['scope'], o['field'], o['cur'], o['cents'], 'same=%s' % o['same'], 'uni' if o['uni'] else '-', 'DBL' if o['head_eq_sum_others'] else '', o['proc'], o['st'], 'TV=%s' % o['tedvalue'], 'n=%d' % o['nlot'], '|', o['title'])
print(collections.Counter((o['cls'], o['grp']) for o in out))
print('uniform by cls', collections.Counter((o['cls'], o['uni']) for o in out))
