import json,sys
d=json.load(open(sys.argv[1]))
print('notice', d['notice_id'])
for s in d['sections']:
    print('--', s['kind'], s['section_id'], 'parent=', s['parent_section_id'])
    for v in s['values']:
        val = {k:v[k] for k in v if k not in ('field_id','ordinal','type')}
        print('    ', v['field_id'], v.get('type'), json.dumps(val, ensure_ascii=False)[:300])
