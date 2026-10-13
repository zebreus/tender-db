import json,sys,re,collections
rows=[json.loads(l) for l in open(sys.argv[1])]
pref=re.compile(r'^\s*(lot(e?s)?)\b[^A-Za-zÀ-ÿ]*', re.I)
same=diff=0; gained=collections.Counter(); lost=collections.Counter(); newfreq=collections.Counter(); oldfreq=collections.Counter()
nold=nnew=0; lotpref_old=lotpref_new=0
for d in rows:
    o,n=d['old'],d['new']; nold+=len(o); nnew+=len(n)
    lotpref_old+=sum(1 for x in o if re.match(r'^\s*lot',x,re.I)); lotpref_new+=sum(1 for x in n if re.match(r'^\s*lot',x,re.I))
    for x in set(n): newfreq[x]+=1
    for x in set(o): oldfreq[x]+=1
    if o==n: same+=1; continue
    diff+=1
    for x in set(n)-set(o): gained[x]+=1
    for x in set(o)-set(n): lost[x]+=1
print(sys.argv[1], 'notices',len(rows),'same',same,'diff',diff,'names old',nold,'new',nnew,'lot-prefixed old',lotpref_old,'new',lotpref_new)
print('hub names new (>=3 notices):',[(k,v) for k,v in newfreq.most_common(25) if v>=3])
print('hub names old (>=3 notices):',[(k,v) for k,v in oldfreq.most_common(15) if v>=3])
sus=[x for x in gained if re.match(r'^\s*lot',x,re.I) or re.search(r'\d\)',x) or len(x)<4 or x[:1].islower() or re.match(r'^\d',x)]
print('suspicious gained',len(sus)); 
for x in sorted(sus)[:150]: print('   ',repr(x))
