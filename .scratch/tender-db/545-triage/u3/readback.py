import json,hashlib,subprocess,time,sys
c=json.load(open('cohort_final.json')); rec=json.load(open('cohort_build/record.json'))
tend={(r['org_id'],r['notice_id'],r['section_id']):r.get('tenders') or [] for r in rec['rows']}
reh=[v for v in c['verdicts'] if v['action']=='rehome' and tend.get((v['org_id'],v['notice_id'],v['section_id']))]
reh.sort(key=lambda v: hashlib.sha256(f"545-u4-{v['notice_id']}-{v['section_id']}".encode()).hexdigest())
pick=reh[:10]
jeol=[v for v in reh if v['target_org_id']==12991212]
for v in jeol:
    if v not in pick: pick.append(v)
out=[]
for v in pick:
    t=tend[(v['org_id'],v['notice_id'],v['section_id'])][0]
    time.sleep(2)
    b=subprocess.run(['curl','-s','--max-time','30',f'https://tenders.zebreus.click/v1/tenders/{t}'],capture_output=True,text=True).stdout
    open('readback.log','a').write(f"{time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())} GET /v1/tenders/{t}\n")
    d=json.loads(b)
    ps=d.get('parties',[])
    tgt=[p['role'] for p in ps if p.get('organization_id')==v['target_org_id']]
    hold=[p['role'] for p in ps if p.get('organization_id')==v['org_id']]
    ok=bool(tgt)
    out.append({'tender':t,'notice':v['notice_id'],'from':v['org_id'],'to':v['target_org_id'],'target_roles':tgt,'holder_roles':hold,'ok':ok})
    print(t, v['org_id'],'->',v['target_org_id'], 'target roles',tgt,'holder roles',hold, 'OK' if ok else 'MISSING')
json.dump(out,open('readback.json','w'),indent=1)
print('ok',sum(o['ok'] for o in out),'of',len(out))
