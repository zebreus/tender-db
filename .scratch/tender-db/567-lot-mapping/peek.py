import json, sys
sys.path.insert(0, '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m567')
import api
pub = sys.argv[1]
r = api.get(f"/v1/notices?source=ted&publication_id={pub}", cache=f"lookup/{pub}.json")
for n in r['items']:
    print(n['id'], n['profile'], n['published_at'])
    c = api.get(f"/v1/notices/{n['id']}/content", cache=f"content/{n['id']}.json")
    for s in c['sections']:
        if s['kind'] in ('Lot','LotResult','LotsGroup','Part') or s['section_id']=='PROCEDURE':
            vals = [(v['field_id'], v.get('value', v.get('code'))) for v in s['values'] if v['field_id'] in ('TED-ITEM','TED-LOT_NO','TED-LOT_NUMBER','TED-TD_DOCUMENT_TYPE','TED-CONTRACT_NO','TED-CONTRACT_NUMBER')]
            print(' ', s['section_id'], s['kind'], s['parent_section_id'], vals)
