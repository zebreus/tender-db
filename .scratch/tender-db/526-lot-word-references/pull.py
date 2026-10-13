import json,sys,urllib.request,urllib.error,collections,time
out=open(sys.argv[2],'w'); errs=collections.Counter(); n=0
for line in open(sys.argv[1]):
    nid=line.strip()
    if not nid: continue
    d=None
    for attempt in range(6):
        try:
            d=json.load(urllib.request.urlopen(f'http://localhost:8080/v1/notices/{nid}/content',timeout=30)); break
        except urllib.error.HTTPError as e:
            if e.code==429: time.sleep(2*(attempt+1)); continue
            errs[repr(e)[:80]]+=1; break
        except Exception as e:
            errs[repr(e)[:80]]+=1; break
    time.sleep(0.2)
    if d is None: continue
    body=None; old=[]
    for s in d.get('sections',[]):
        for v in s.get('values',[]):
            if v.get('field_id')=='TXT-TX': body=v.get('value')
            if v.get('field_id')=='TED-OFFICIALNAME' and (s.get('parent_section_id') or '').startswith('RES'): old.append(v.get('value'))
    if body: out.write(json.dumps({'id':int(nid),'body':body,'old':old})+'\n'); n+=1
    else: errs['nobody']+=1
out.close(); print(sys.argv[1], n, dict(errs))
