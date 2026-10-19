import json,sys
# print procedure title/description, buyers, and sections whose name matches a pattern, with the roles that reference them
d=json.load(open(sys.argv[1])); pat=sys.argv[2].lower() if len(sys.argv)>2 else None
secs={s['section_id']:s for s in d['sections']}
refs={}
for s in d['sections']:
    for v in s['values']:
        if v.get('type')=='id' and v.get('is_ref') and str(v.get('value','')).startswith('ORG-'):
            refs.setdefault(v['value'],[]).append((v['field_id'], s['section_id']))
orgs=[s for s in d['sections'] if s['kind']=='Organization']
print('notice', d['notice_id'], 'org sections', len(orgs))
for s in d['sections']:
    if s['kind']=='Notice':
        for v in s['values']:
            if v['field_id'] in ('BT-21-Procedure','BT-24-Procedure','OPP-070-notice','BT-105-Procedure'):
                print(' ', v['field_id'], (v.get('value') or v.get('code'))[:1500])
for s in orgs:
    names=[v['value'] for v in s['values'] if v['field_id'].startswith('BT-500')]
    ids=[v['value'] for v in s['values'] if v['field_id'].startswith('BT-501')]
    roles=sorted(set(f for f,_ in refs.get(s['section_id'],[])))
    show = pat is None or any(pat in n.lower() for n in names) or any(r.endswith('Buyer') for r in roles)
    if show: print('  ORG', s['section_id'], names, ids, roles, 'nrefs=%d'%len(refs.get(s['section_id'],[])))
