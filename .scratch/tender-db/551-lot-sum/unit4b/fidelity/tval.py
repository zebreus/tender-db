"""For each summed lot of the derivation: the award notice's own value(s) for that result -- eForms: the referenced
LotTender(s) BT-720 and the LotResult amounts; TED: the award block amounts -- against the API's awarded cents."""
import json, sys
sys.path.insert(0, '.')
import derive
D = json.load(open(derive.BASE + '/fid/derived.json'))
for t in sys.argv[1:]:
    F = D[t]
    print('==', t)
    for s in F['summed']:
        nf = derive.read_notice(s['notice'])
        res = nf['results'].get(s['result_key'])
        tv = []
        for tr in res['tender_refs']:
            tdr = nf['tenders'].get(tr) or {}
            tv.append((tr, tdr.get('lot'), tdr.get('amounts')))
        notice_vals = [c for tr in tv for f, c, cu in (tr[2] or []) if f == 'BT-720-Tender'] or [c for f, c, cu in res['amounts'] if f in ('TED-VAL_TOTAL',)]
        ok = (sum(notice_vals) == s['cents']) if notice_vals else None
        print('  ', s['key'], 'api', s['cents'], '| notice', notice_vals, 'sum', sum(notice_vals) if notice_vals else None, 'MATCH' if ok else ('NO VALUE IN BLOCK' if ok is None else 'DIFF'), '| block', res['amounts'], '| tenders', tv)
