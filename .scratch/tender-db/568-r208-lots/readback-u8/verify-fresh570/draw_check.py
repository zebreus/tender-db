import json, os, hashlib, sys
D='/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh570/changes_tender'
A,B=957854566,958764334
pages=[json.load(open(os.path.join(D,f))) for f in sorted(os.listdir(D)) if f.startswith('p') and f.endswith('.json')]
print('pages',len(pages))
ev=[e for p in pages for e in p['events']]
cur=[int(e['cursor']) for e in ev]
print('events',len(ev),'min',min(cur),'max',max(cur),'in range',sum(A<c<=B for c in cur))
gens={p.get('generation') for p in pages}; print('gens',gens, 'resets',[p.get('reset') for p in pages if p.get('reset')], 'ignored',{str(p.get('ignored_filters')) for p in pages})
print('last page last_cursor',pages[-1]['last_cursor'],'more',pages[-1].get('more'))
# continuity: each page's first cursor > previous last_cursor
for i in range(1,len(pages)):
    if pages[i]['events'] and int(pages[i]['events'][0]['cursor'])<=int(pages[i-1]['last_cursor']): print('overlap',i)
import collections
k=collections.Counter((e['entity'],e['op'],e['version'] is None) for e in ev if A<int(e['cursor'])<=B); print(k)
ids=sorted({int(e['id']) for e in ev if A<int(e['cursor'])<=B and e['entity']=='tender' and e['op']=='changed' and e['version'] is None})
print('ids',len(ids))
od=sorted(ids,key=lambda i: hashlib.sha256(('570-fresh30-'+str(i)).encode()).hexdigest())
print('order sha', hashlib.sha256(','.join(map(str,od)).encode()).hexdigest())
print('first40',od[:40])
w=json.load(open('/home/user/tender-db/.scratch/tender-db/568-r208-lots/readback-u8/fresh570/walk.json'))
print('walk ids', [x['id'] for x in w['walk']])
print('held', w['held_ids'])
print('match first', od[:len(w['walk'])]==[x['id'] for x in w['walk']])
# check event keys sample
print(ev[0])
