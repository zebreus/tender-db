# reviewer 4: compact dump of a saved notice-content body: one line per section, its values field=value
import json, sys
for path in sys.argv[1:]:
    d = json.load(open(path))
    print('#### notice', d.get('notice_id'), path.rsplit('/',1)[-1])
    for k, v in d.items():
        if k not in ('notice_id', 'sections'):
            print('  [top]', k, '=', json.dumps(v)[:300])
    for s in d['sections']:
        vals = []
        for v in s.get('values', []):
            val = v.get('value', v.get('code'))
            extra = ''
            if v.get('scheme'): extra += '{' + str(v['scheme']) + '}'
            if v.get('is_ref'): extra += '(ref)'
            vals.append(f"{v['field_id']}{extra}={json.dumps(val, ensure_ascii=False)[:160]}")
        print(f"  <{s['kind']}> {s['section_id']} (parent {s.get('parent_section_id')})")
        for x in vals:
            print('      ', x)
