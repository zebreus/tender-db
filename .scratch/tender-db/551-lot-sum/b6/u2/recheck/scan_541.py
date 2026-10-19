"""Scan the 3,502's named notices: TED-FORM values (T01/T02), TED-COST_PARAMETERS carriers, layer use, newest ids."""
import collections, importlib.util, json, os, sys
sys.dont_write_bytecode = True
RC = '/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/u2/recheck/L/551-lot-sum/b6/rule_v2_g.py'
spec = importlib.util.spec_from_file_location('rule_scan', RC); R = importlib.util.module_from_spec(spec); spec.loader.exec_module(R)
rows = json.load(open('/home/user/tender-db/.scratch/tender-db/551-lot-sum/b5/investigate/served_lsa_rows.json'))
ids = sorted({int(r[0]) for r in rows} - {179732})
named = {}
for t in ids:
    named[t] = R.version_notice_ids(R.tender_detail(t))
alln = sorted({n for v in named.values() for n in v})
print('named notices', len(alln), 'max', alln[-1], 'top', alln[-8:])
layer = collections.Counter(); forms = collections.Counter(); cost = []; t0x = []
fieldhits = collections.Counter()
for n in alln:
    p = R.find_cached('notices', n)
    layer[os.path.dirname(p)] += 1
    c = R.notice_content(n)
    fs = set()
    for s in c.get('sections', []):
        for v in s.get('values', []):
            fs.add(v['field_id'])
            if v['field_id'] == 'TED-FORM':
                forms[str(R._val(v)).strip().upper()] += 1
                if str(R._val(v)).strip().upper() in ('T01', 'T02'):
                    t0x.append((n, str(R._val(v)).strip()))
    if any('COST_PARAM' in f for f in fs): cost.append(n)
    for f in fs:
        if 'MOVE' in f: fieldhits[f] += 1
print('layers', dict(layer))
print('T01/T02 notices', t0x, 'tenders', [t for t, v in named.items() if any(n == x for x, _ in t0x for n in v)])
print('COST_PARAM carriers', cost)
print('MOVE fields', dict(fieldhits))
print('TED-FORM top', forms.most_common(40))
# does the content endpoint carry text fields at all? sample check for any field with TXT or long text
