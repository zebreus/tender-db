# compact view of a /v1/notices/{id}/content body: organizations (name, id, country, other fields), procedure texts, role refs
import json, sys
d = json.load(open(sys.argv[1]))
print('== notice', d['notice_id'])
for s in d['sections']:
    vals = s['values']
    if s['kind'] == 'Organization':
        out = []
        for v in vals:
            f = v['field_id']; x = v.get('value', v.get('code'))
            if isinstance(x, dict): x = x.get('value') or x
            out.append(f"{f.split('-Organization')[0]}={x if 'value' not in v else v['value']}{'('+v['scheme']+')' if v.get('scheme') else ''}")
        print('  ORG', s['section_id'], '|', ' ; '.join(out))
    else:
        for v in vals:
            f = v['field_id']
            if f.startswith(('BT-21', 'BT-24', 'OPT-300', 'OPT-301', 'BT-500', 'OPP-070', 'BT-01-', 'BT-760', 'BT-759', 'BT-710', 'BT-711', 'BT-720')) or 'Buyer' in f or 'Tenderer' in f:
                print('  ', s['kind'], s['section_id'], f, json.dumps(v.get('value', v.get('code')), ensure_ascii=False)[:400])
