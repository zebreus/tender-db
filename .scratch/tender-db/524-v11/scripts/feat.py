exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/base.py').read())
import unicodedata, re
SPECIAL={'ł':'l','ø':'o','đ':'d','ð':'d','æ':'ae','œ':'oe','ß':'ss','þ':'th','ı':'i','ħ':'h','ŧ':'t','ŀ':'l','ĳ':'ij'}
def fold(s):
    out=[]
    for c in s.lower():
        if c in SPECIAL: out.append(SPECIAL[c]); continue
        dec=unicodedata.normalize('NFD',c)
        base=''.join(ch for ch in dec if not unicodedata.combining(ch))
        # only fold Latin letters; keep others (Cyrillic/Greek) w/o combining marks like Rust does for combining ranges
        if c.isascii() or ('LATIN' in unicodedata.name(c,'')):
            out.append(base)
        else:
            # Rust drops combining marks only when they're separate codepoints; precomposed Cyrillic й stays
            out.append(c if not unicodedata.combining(c) else '')
    return ''.join(out)
def T(s):
    return [t for t in re.split(r'[^\w]|_', fold(s)) if t]
FORM_WORDS=set('''sp spolka z ograniczona odpowiedzialnoscia jawna komandytowa akcyjna cywilna partnerska komandytowo spzoo spj spk ska sa sl slu sau scarl scrl srl sarl sas sasu eurl spa snc gmbh mbh ag kg ohg ug haftungsbeschrankt co ev ek eg ltd limited plc llp lp cic company bv nv vof cv ab oy oyj abp as asa aps doo dd sro spol kft zrt nyrt bt korlatolt felelossegu tarsasag uab vsia sia societe anonyme responsabilite limitee par actions simplifiee sociedad anonima limitada unipersonal societa per azioni responsabilita lda unipessoal'''.split())
def C(s): return [t for t in T(s) if t not in FORM_WORDS]
SEPS_B=[' - ',' – ',' — ',' / ','/',' | ']
def doubled_B(s):
    hits=[(sep,s.count(sep)) for sep in SEPS_B if sep in s]
    # 'splits on exactly one of' : the display splits on one separator into exactly two parts
    for sep in SEPS_B:
        if s.count(sep)==1:
            a,b=s.split(sep)
            ta,tb=T(a),T(b)
            if ta and ta==tb: return True
    t=T(s)
    if t and len(t)%2==0 and t[:len(t)//2]==t[len(t)//2:]: return True
    return False
SEPS_A=re.compile(r' [-–—/] ')
def doubled_A(s):
    c=C(s)
    n=len(c)
    if n and n%2==0 and c[:n//2]==c[n//2:] and any(len(x)>=3 for x in c): return True
    for m in SEPS_A.finditer(s):
        L_,R_=C(s[:m.start()]),C(s[m.end():])
        if not L_ or not R_: continue
        if not any(len(x)>=3 for x in R_): continue
        if L_==R_ or L_[:len(R_)]==R_ or L_[-len(R_):]==R_ or R_[:len(L_)]==L_ or R_[-len(L_):]==L_: return True
    return False
def dblA(old,new): return doubled_A(new) and not doubled_A(old)
def dblB(old,new): return doubled_B(new) and not doubled_B(old)
SEATP={'w','we','v','ve','vo'}
def seat_dropped(old,new):
    th,td=T(old),T(new)
    if len(td)<2 or len(th)<=len(td) or th[:len(td)]!=td: return False
    tail=th[len(td):]
    if not (2<=len(tail)<=3): return False
    if any(ch.isdigit() for t in tail for ch in t): return False
    if tail[0] not in SEATP: return False
    # preposition followed by whitespace in head text
    m=re.search(r'(?i)(?:^|[^\w])(%s)(\s)'%re.escape(tail[0]), old)
    # stricter: find the preposition occurrence after the display words
    ok=False
    for mm in re.finditer(r'(?i)(?<![\w])(%s)(?![\w])'%re.escape(tail[0]), old):
        nxt=old[mm.end():mm.end()+1]
        if nxt and nxt.isspace(): ok=True
    return ok
PLACE=re.compile(r'(?i)^(keineangabe|telefon)\d*$')
def placeholder_id(ident):
    if ident is None: return False
    return bool(PLACE.match(ident)) or (len(ident)>0 and set(ident)=={'0'})
def feats(e, asof=PLAN_ASOF):
    ta=age(e['top_newest'],asof)
    return dict(ta=ta, ha=age(e['head_newest'],asof), sh=e['second']>e['head'],
                dA=dblA(e['old_name'],e['new_name']), dB=dblB(e['old_name'],e['new_name']),
                seat=seat_dropped(e['old_name'],e['new_name']), ph=placeholder_id(e.get('identifier')))
