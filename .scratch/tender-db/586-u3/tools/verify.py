"""verify.py -> verify.json from the Verify GETs and statement v01_labels."""
import json, os
HERE = '/home/user/tender-db/.scratch/tender-db/586-u3'
def load(p):
    return json.load(open(os.path.join(HERE, p)))
def gets():
    out = {}
    for ln in open(os.path.join(HERE, 'raw', 'gets.log')):
        parts = ln.split()
        out.setdefault(parts[2], (parts[0], parts[4]))
    return out
G = gets()
t = load('raw/tender/540072.json')
ids = sorted({p['organization_id'] for p in t['parties'] if p['role'] in ('Procedure-Buyer', 'Lot-ReviewOrg')})
a1 = {'check': "GET /v1/tenders/540072: unique organization_id over Procedure-Buyer and Lot-ReviewOrg", 'expected': [22198704],
      'got': ids, 'pass': ids == [22198704], 'fetched_at': G['/v1/tenders/540072'][0], 'version': t['version'],
      'parties': [[p['role'], p['lot'], p['organization_id'], p['mention_name']] for p in t['parties'] if p['role'] in ('Procedure-Buyer', 'Lot-ReviewOrg')],
      'sql_x540072': 'the 4 Lot-ReviewOrg rows are anchored at ORG-0001 of notices 23563650 / 23576127 / 23589619 / 23658810 (one per lot); each lot names TPO-0001 "Sociedad Aragonesa de Gestión Agroambiental S.L.U" under ORG-0001 "Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)"; final.py tier A; s_mentions_c: notice 23563650 has no TPO-0001 mention (ORG-0001 -> 22198704)'}
o = load('raw/org/31974991.json')
a2 = {'check': 'GET /v1/organizations/31974991 ("Dirección Principal", 420 mentions while open)', 'expected': 'mentions 0 or 404',
      'got': o, 'http': G['/v1/organizations/31974991'][1], 'pass': o.get('error', {}).get('status') == 404 or o.get('mentions') == 0,
      'fetched_at': G['/v1/organizations/31974991'][0],
      'sweep_2772': 'has run: the organization row is gone (404). retire_mentions never deletes an organization (586-u1/decision.md, Unit 3), so a 404 means the orphan sweep removed it.'}
extra = {}
for oid in [31975096, 31975008, 31975059, 31975006, 31975019, 31975180, 31975009, 31975004, 31975568, 31975060, 31975949]:
    d = load(f'raw/org/{oid}.json')
    extra[str(oid)] = {'http': G[f'/v1/organizations/{oid}'][1], 'body': d}
a3 = {}
for tid in (346591, 361790):
    d = load(f'raw/tender/{tid}.json')
    hits = [[p['role'], p['lot'], p['organization_id'], p['mention_name']] for p in d['parties'] if p['organization_id'] in (18044191, 18044192)]
    a3[str(tid)] = {'version': d['version'], 'fetched_at': G[f'/v1/tenders/{tid}'][0], 'parties_on_18044191_or_18044192': hits,
                    'lot_role_hits': [h for h in hits if h[0].startswith('Lot-')], 'pass': not hits,
                    'parties': [[p['role'], p['lot'], p['organization_id'], p['mention_name']] for p in d['parties']]}
v = load('raw/sql/v01_labels.out')
rows = [dict(zip(v['columns'], r)) for r in v['rows']]
a4 = {'check': 'tender_version_parties by organization_id = 18044191 / 18044192 (all versions), roles', 'statement': 'v01_labels',
      'expected': 'only legacy further-information / tender-receipt rows, no Lot-*', 'rows': rows,
      'lot_rows': [r for r in rows if r['role'].startswith('Lot-')], 'pass': all(not r['role'].startswith('Lot-') for r in rows),
      'before_U5_refold': '576-u4/u5_buyer_cohort.json (16:45Z): 18044191 1,032 Lot-AddInfo + 193 further-information; 18044192 1,076 Lot-TenderReceipt + 175 tender-receipt'}
for k in ('18044191', '18044192', '4592524'):
    pass
out = {'generated': 'see fetch-notes.md', 'A1_540072': a1, 'A2_31974991': a2, 'A2_extra_hub_orgs_from_decision_unit4_list': extra,
       'A3_tenderned_label_tenders': a3, 'A4_label_org_roles': a4}
for k, path in (('A5_org_18044191_now', 'raw/extra/org-18044191.json'), ('A5_org_18044192_now', 'raw/extra/org-18044192.json'),
                ('A6_tender_4592524_buyer', 'raw/extra/tender-4592524.json')):
    if os.path.exists(os.path.join(HERE, path)):
        d = load(path)
        if k.startswith('A6'):
            d = {'version': d.get('version'), 'buyer_parties': [[p['role'], p['lot'], p['organization_id'], p['mention_name']] for p in d.get('parties', []) if p['role'] in ('buyer', 'Procedure-Buyer')],
                 'org_31431409_served': [[p['role'], p['lot'], p['organization_id']] for p in d.get('parties', []) if p['organization_id'] == 31431409]}
        out[k] = d
json.dump(out, open(os.path.join(HERE, 'verify.json'), 'w'), indent=1, ensure_ascii=False)
print(json.dumps({k: (v.get('pass') if isinstance(v, dict) and 'pass' in v else '-') for k, v in out.items()}))
