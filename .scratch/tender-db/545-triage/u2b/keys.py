import re, unicodedata
FAM = {"gmbh":"gmbh","mbh":"gmbh","ag":"ag","sarl":"sarl","eurl":"sarl","sas":"sas","sasu":"sas",
 "oy":"aktiebolag","oyj":"aktiebolag","ab":"aktiebolag","abp":"aktiebolag","uab":"aktiebolag","aps":"aps",
 "bv":"bv","nv":"nv","kft":"kft","zrt":"zrt","nyrt":"zrt","spa":"spa","srl":"srl","sia":"sia","sro":"sro","spzoo":"spzoo"}
SEQ = [(["spol","s","r","o"],"sro"),(["ges","m","b","h"],"gmbh"),(["sp","z","o","o"],"spzoo"),(["s","r","o"],"sro")]
def match_norm(s):
    s = s.lower()
    s = ''.join(c if c.isalnum() else ' ' for c in s)
    return ' '.join(s.split())
def n3(s):
    t = match_norm(s).split(); out=[]; i=0
    while i < len(t):
        hit=False
        for seq,f in SEQ:
            if t[i:i+len(seq)]==seq: out.append('§'+f); i+=len(seq); hit=True; break
        if hit: continue
        out.append('§'+FAM[t[i]] if t[i] in FAM else t[i]); i+=1
    return ' '.join(out)
def trim(name):
    low=name.lower(); cut=len(name)
    for m in [" t/a ","(t/a "," trading as ","(trading as "]:
        p=low.find(m)
        if p>=0: cut=min(cut,p)
    kept=name[:cut].rstrip()
    if kept.endswith(')') and '(' in kept:
        o=kept.rfind('('); before=kept[:o].rstrip().rstrip('.,')
        if any(before.lower().endswith(f) for f in [" ltd"," limited"," plc"," llp"]): kept=before
    return kept
def alt(name):
    out=[]; ini=''
    for t in n3(trim(name).replace('&',' and ')).split():
        if t=='the': continue
        if t=='and':
            if ini: out.append(ini); ini=''
            continue
        if len(t)==1 and t.isalpha(): ini+=t; continue
        if ini: out.append(ini); ini=''
        out.append({"ltd":"§ltd","limited":"§ltd","plc":"§plc","llp":"§llp","lp":"§lp","cic":"§cic","co":"company"}.get(t,t))
    if ini: out.append(ini)
    return ' '.join(out)
def vote_key(name):
    a = alt(name)
    kept=[t for t in a.split() if not t.startswith('§') and t not in FAM]
    return ' '.join(kept) if kept else a
def script(s):
    c={'L':0,'C':0,'G':0,'O':0}
    for ch in s:
        if not ch.isalpha(): continue
        n=unicodedata.name(ch,'')
        k='C' if n.startswith('CYRILLIC') else 'G' if n.startswith('GREEK') else 'L' if n.startswith('LATIN') else 'O'
        c[k]+=1
    if sum(c.values())==0: return None
    return max(c,key=c.get)
