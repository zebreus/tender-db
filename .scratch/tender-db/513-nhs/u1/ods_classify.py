# 513 unit 1: join the stored registry_nhs_listing (job 2236) with each code's ODS ORD record.
import json, re, collections, os
L = json.load(open('nhs-listing-2236.json'))['listing']
roles = {r['id']: r['displayName'] for r in json.load(open('ods/_roles.json'))['Roles']}
def ods(code):
    d = json.load(open(f'ods/{code}.json'))
    b = d['body']
    if d['http'] != '200' or not b or 'Organisation' not in b:
        return {'found': False, 'http': d['http']}
    o = b['Organisation']
    rs = o.get('Roles', {}).get('Role', [])
    prim = [roles.get(r['id'], r['id']) for r in rs if r.get('primaryRole')]
    act = [roles.get(r['id'], r['id']) for r in rs if r.get('Status') == 'Active']
    succ = [s.get('Target', {}).get('OrgId', {}).get('extension') for s in o.get('Succs', {}).get('Succ', []) if s.get('Type') == 'Successor']
    return {'found': True, 'name': o['Name'], 'status': o['Status'], 'rc': o.get('orgRecordClass'),
            'primary': prim[0] if prim else None, 'active_roles': act, 'successors': succ}
STOP = {'nhs','the','and','of','trust','foundation','ft','icb','integrated','care','board','limited','ltd','&','-','plc','for'}
def toks(n):
    return {t for t in re.findall(r'[a-z0-9]+', (n or '').lower().replace('&',' and ')) if t not in STOP}
def sim(a, b):
    A, B = toks(a), toks(b)
    return len(A & B) / max(1, len(A | B))
rows = []
for x in L:
    code = x['value'][5:]
    o = ods(code)
    rh = x['registry_holders']; ph = x['ppon_holders']
    on = o.get('name')
    rows.append({**{k: x[k] for k in ('class','literal','value','identity','ppon','witnesses','registry_holder_count','ppon_holder_count')},
                 'code': code, 'shape': re.sub(r'[0-9]','9',re.sub(r'[A-Z]','A',code)), 'ods': o,
                 'reg_names': [(h['org_id'], h['name'], h['identifier'], h['provisional']) for h in rh],
                 'ppon_names': [(h['org_id'], h['name'], h['identifier'], h['provisional']) for h in ph],
                 'reg_sim': [round(sim(on, h['name']), 2) for h in rh] if on else [],
                 'ppon_sim': [round(sim(on, h['name']), 2) for h in ph] if on else []})
json.dump(rows, open('nhs-ods-joined.json', 'w'), indent=1)
c = collections.Counter()
for r in rows:
    o = r['ods']
    c[(r['class'], r['shape'], o.get('rc') if o['found'] else 'NOTFOUND', o.get('status'), o.get('primary'))] += 1
for k, v in sorted(c.items(), key=lambda kv: (kv[0][0], -kv[1])): print(v, k)
