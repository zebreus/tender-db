import sys, json, re; sys.path.insert(0,'.')
from lib import *
from urllib.parse import urlparse
UR=json.load(open('urls_sdk.json'))
sdk=[t for t in U if t['doe_profile'].startswith('eforms-sdk-0.1')]
def norm(u):
    u=u.strip().lower()
    u=re.sub(r'^https?://','',u); u=re.sub(r'^www\.','',u); u=u.rstrip('/')
    return u
def keyids(u):
    """platform procedure-id tokens: long digit/alnum runs and uuids"""
    u=norm(u)
    ids=set(re.findall(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}',u))
    ids|=set(re.findall(r'\b[a-z]?\d{6,}\b',u.replace('/',' ').replace('=',' ').replace('?',' ').replace('&',' ')))
    ids|=set(re.findall(r'cxp[0-9a-z]{6,}',u))
    return ids
hosts=collections.Counter()
sdk_has=0
for t in sdk:
    us=[v for f,v in UR.get(str(t['doe']['notice_id']),[])]
    if us: sdk_has+=1
    for u in us: hosts[urlparse(u if '://' in u else 'http://'+u).netloc.lower()]+=1
print('sdk notices with a URI',sdk_has,'of',len(sdk)); print('sdk hosts',hosts.most_common(12))
ted_has=sum(1 for t in sdk for c in t['candidates'] if c['has_signals'] and UR.get(str(c['notice_id'])))
print('TED cand notices with BT-15', len({c['notice_id'] for t in sdk for c in t['candidates'] if c['has_signals'] and UR.get(str(c['notice_id']))}),'of',len({c['notice_id'] for t in sdk for c in t['candidates'] if c['has_signals']}))
# genericness: how many distinct notices share an exact normalized URL (across everything read)
by=collections.defaultdict(set)
for nid,rows in UR.items():
    for f,v in rows: by[norm(v)].add(nid)
shared=[(u,len(s)) for u,s in by.items() if len(s)>1]
print('distinct urls',len(by),'shared by >1 notice',len(shared)); 
for u,n in sorted(shared,key=lambda x:-x[1])[:15]: print('   ',n,u[:110])
# pair-level: exact URL eq / id-token overlap between sdk and its candidates
c=collections.Counter(); ex=[]
for t in sdk:
    su={norm(v) for f,v in UR.get(str(t['doe']['notice_id']),[])}
    sk=set().union(*[keyids(v) for f,v in UR.get(str(t['doe']['notice_id']),[])]) if su else set()
    for x in t['candidates']:
        if not x['has_signals']: continue
        tu={norm(v) for f,v in UR.get(str(x['notice_id']),[])}
        tk=set().union(*[keyids(v) for f,v in UR.get(str(x['notice_id']),[])]) if tu else set()
        if not su or not tu: c['missing']+=1; continue
        c['both']+=1
        if su&tu: c['url_eq']+=1; ex.append(('EQ',t,x,su&tu))
        elif sk&tk: c['id_overlap']+=1; ex.append(('ID',t,x,sk&tk))
print(sorted(c.items()))
for k,t,x,s in ex[:30]:
    f=features(t['doe'],x,dprefer='canonical',tprefer='canonical',d_t=x['delta_days'])
    print(k,list(s)[:2],'T',t['tender_id'],t['doe_profile'],'cand',x['tender_id'],x['subtype'],'dt',x['delta_days'],'src',x['census_sources'],'|',repr(f['td'][:60]),'|',repr(f['tt'][:60]))
print('--- shared URLs: do they span different Tenders?')
n2t={}; n2title={}
for t in sdk:
    n2t[str(t['doe']['notice_id'])]=('doe',t['tender_id']); n2title[str(t['doe']['notice_id'])]=title_of(t['doe'],'canonical')
    for x in t['candidates']:
        if x['has_signals']:
            n2t[str(x['notice_id'])]=('ted',x['tender_id']); n2title[str(x['notice_id'])]=title_of(x,'canonical')
cc=collections.Counter()
for u,s in by.items():
    if len(s)<2: continue
    tenders={n2t[n] for n in s}
    generic = '/' not in u and '?' not in u
    cc[('generic' if generic else 'specific', 'multi_tender' if len(tenders)>1 else 'same_tender')]+=1
    if len(tenders)>1 and not generic:
        print('  ',u[:100]); [print('      ',n2t[n],repr((n2title[n] or '')[:70])) for n in s]
print(cc)
