import json, sys
# usage: orgs.py file.json  -> per Organization: name, address, email, web, size, BT-501; plus TenderingParty and Tenderer refs, roles summary
d = json.load(open(sys.argv[1]))
secs = d['sections']
ids = {}
for s in secs:
    if s['kind'] == 'CompanyLegalEntity':
        ids.setdefault(s.get('parent_section_id'), []).append([v.get('value') for v in s['values']])
F = {'BT-500-Organization-Company': 'name', 'BT-510(a)-Organization-Company': 'street', 'BT-512-Organization-Company': 'pc',
     'BT-513-Organization-Company': 'town', 'BT-506-Organization-Company': 'email', 'BT-505-Organization-Company': 'web',
     'BT-165-Organization-Company': 'size', 'BT-503-Organization-Company': 'tel', 'BT-16-Organization-Company': 'dept',
     'BT-500-Organization-TouchPoint': 'tp_name', 'BT-510(a)-Organization-TouchPoint': 'tp_street', 'BT-513-Organization-TouchPoint': 'tp_town', 'BT-506-Organization-TouchPoint': 'tp_email'}
for s in secs:
    if s['kind'] == 'Organization' or s['section_id'].startswith('TPO'):
        out = {}
        for v in s['values']:
            k = F.get(v['field_id'])
            if k:
                out.setdefault(k, []).append((v.get('lang') or '') + ':' + str(v.get('value', v.get('code'))) if v.get('lang') else str(v.get('value', v.get('code'))))
        flags = [v['field_id'] for v in s['values'] if v['field_id'].startswith('OPP-05') or v['field_id'].startswith('OPT-30')]
        print(s['section_id'], s['kind'], {k: (vv if len(vv) > 1 else vv[0]) for k, vv in out.items()}, 'ID=', ids.get(s['section_id']), flags or '')
for s in secs:
    if s['kind'] in ('TenderingParty', 'Tenderer'):
        print('  ', s['kind'], s['section_id'], s.get('parent_section_id'), [(v['field_id'], v.get('value', v.get('code'))) for v in s['values']])
roles = {}
for s in secs:
    for v in s['values']:
        if v.get('is_ref') or (isinstance(v.get('value'), str) and v['value'].startswith(('ORG-', 'TPO-')) and v['field_id'].startswith('OPT-30')):
            roles.setdefault(v['value'], set()).add(v['field_id'] + '@' + s['kind'])
for k in sorted(roles):
    print('  ref', k, sorted(roles[k]))
