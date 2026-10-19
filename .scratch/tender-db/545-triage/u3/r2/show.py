import json,sys
B='/home/user/tender-db/.scratch/tender-db/545-triage/u3/bodies/'
def short(v):
    t=v.get('type')
    if t=='text': return v.get('value')
    if t=='id': return f"{v.get('scheme')}:{v.get('value')}"+(" (ref)" if v.get('is_ref') else "")
    if t=='code': return v.get('code')
    if t=='classification': return v.get('code')
    if t in ('date','number','amount','bool','indicator'): return v.get('value') if 'value' in v else {k:v[k] for k in v if k not in ('field_id','ordinal','type')}
    return {k:v[k] for k in v if k not in ('field_id','ordinal','type')}
for n in sys.argv[1:]:
    d=json.load(open(B+f'v1_notices_{n}_content.json'))
    print('=== notice', n, 'sections', len(d['sections']))
    for s in d['sections']:
        vals=[(v['field_id'], short(v)) for v in s['values']]
        if s['kind'] in ('Lot','LotResult','Part') and len(sys.argv)>1 and '--all' not in sys.argv:
            pass
        print('--', s['kind'], s['section_id'], 'parent=', s['parent_section_id'])
        for f,v in vals:
            sv=json.dumps(v,ensure_ascii=False) if not isinstance(v,str) else v
            print('    ', f, sv[:200])
