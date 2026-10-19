# usage: python3 -I show_fts.py <package.json>... — per release: buyer ref, each party's id, name, roles,
# identifier + additionalIdentifiers, details (UK_CA_TYPE classifications etc.), and any tender-level
# procuringEntity / text fields mentioning "behalf".
import json, sys, re
for f in sys.argv[1:]:
    d = json.load(open(f))
    for r in d['releases']:
        t = r.get('tender') or {}
        print('==', f.split('/')[-1], r.get('id'), r.get('tag'), '| buyer ref', r.get('buyer'), '| tender.procuringEntity', t.get('procuringEntity'))
        for p in r.get('parties', []):
            ids = [p.get('identifier', {}).get('scheme', '') + '-' + str(p.get('identifier', {}).get('id', ''))] + \
                  [i.get('scheme', '') + '-' + str(i.get('id', '')) for i in p.get('additionalIdentifiers', [])]
            det = p.get('details') or {}
            cls = [c.get('id') for c in det.get('classifications', [])] if isinstance(det, dict) else det
            other = {k: v for k, v in det.items() if k != 'classifications'} if isinstance(det, dict) else {}
            print('   PARTY id=%s | %r | roles=%s | ids=%s | class=%s | details=%s' % (
                p.get('id'), p.get('name'), p.get('roles'), ids, cls, json.dumps(other)[:160]))
        blob = json.dumps(r, ensure_ascii=False)
        for m in re.finditer(r'.{0,120}(on behalf|behalf of|acting for|agent).{0,120}', blob, re.I):
            print('   BEHALF:', m.group(0)[:260])
