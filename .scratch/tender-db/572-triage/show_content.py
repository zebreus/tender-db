# usage: python3 -I show_content.py <content.json>... — the Organization sections (name, ids, country) and
# every role reference (OPT-300/301) of a /v1/notices/{id}/content body, plus the procedure title/description head.
import json, sys
for f in sys.argv[1:]:
    d = json.load(open(f))
    print('== notice', d['notice_id'])
    roles = {}
    for s in d['sections']:
        for v in s['values']:
            if v['field_id'].startswith(('OPT-300', 'OPT-301')):
                roles.setdefault(v.get('value'), []).append(v['field_id'] + '@' + s['section_id'])
    for s in d['sections']:
        if s['kind'] == 'Organization':
            vals = {}
            for v in s['values']:
                vals.setdefault(v['field_id'], []).append(v.get('value') if v.get('value') is not None else v.get('code'))
            print('   ORG', s['section_id'], '| name', vals.get('BT-500-Organization-Company'), '| ids', vals.get('BT-501-Organization-Company'),
                  '| other fields', sorted(k for k in vals if k not in ('BT-500-Organization-Company', 'BT-501-Organization-Company', 'BT-514-Organization-Company')),
                  '| roles', roles.get(s['section_id']))
    for s in d['sections']:
        if s['section_id'] == 'PROCEDURE':
            for v in s['values']:
                if v['field_id'] in ('BT-21-Procedure', 'OPP-070-notice', 'BT-24-Procedure', 'BT-01-notice'):
                    print('   ', v['field_id'], json.dumps(v.get('value', v.get('code')), ensure_ascii=False)[:260])
