import re, sys, json, collections
LAT = {}
for chars, rep in [("àáâãäåāăąæ","a"),("çćčĉ","c"),("ďđ","d"),("èéêëěęēė","e"),("ğģ","g"),("ìíîïıīį","i"),("ķ","k"),("ĺľłļ","l"),("ñńňņ","n"),("òóôõöøőœ","o"),("ŕř","r"),("śšşș","s"),("ťţț","t"),("ùúûüůűūų","u"),("ýÿ","y"),("źżž","z"),("ß","s")]:
    for c in chars: LAT[c]=rep
def match_norm(s):
    out=[]; gap=False
    for c in s:
        if c.isalnum():
            if gap and out: out.append(' ')
            gap=False; out.append(c.lower())
        else: gap=True
    return ''.join(out)
def fold(s): return ''.join(LAT.get(c,c) for c in match_norm(s))
# trailing parenthetical lot qualifier on the RAW trimmed name
PAREN = re.compile(r"\s*\(\s*(?:lot|lots|lote|lotes|lotto|lotti)\b[^()]*\)[\s.,;:]*$", re.I)
def strip_q(s):
    s=s.strip()
    return PAREN.sub("", s)

VOID_PHRASES = ["infructu","infructeu","sans suite","non attribu","declarado desiert","declarada desiert","queda desiert","nessuna aggiudicazione"]
POINTER_PHRASES = ["would prejudice","not applicable","see section","perfil del contratante","perfil de contratante","voir autres informations","voir renseignements","ver informaci"]
VOID_WHOLE = ["desierto","desierta","deserto","deserta","desiertos","desiertas",
  "lote desierto","lotto deserto","gara deserta","non aggiudicato","non aggiudicata","lotto non aggiudicato",
  "not awarded","lot not awarded","no award","no award made","contract not awarded","niet gegund"]
POINTER_WHOLE = ["various"]
EXTRA = sys.argv[1] if len(sys.argv)>1 else "all"
def classify(name):
    f = fold(strip_q(name))
    if not f: return None, None
    pad = " " + f
    for p in VOID_PHRASES:
        if (" "+p) in pad: return "void", "P:"+p
    for p in POINTER_PHRASES:
        if (" "+p) in pad: return "pointer", "P:"+p
    if f in VOID_WHOLE: return "void", "W:"+f
    if f in POINTER_WHOLE: return "pointer", "W:"+f
    return None, None
# text rule as written
SUB = ["WOULD PREJUDICE","NOT APPLICABLE","INFRUCTU","SANS SUITE","NON ATTRIBU","PERFIL DEL CONTRATANTE","PERFIL DE CONTRATANTE","SEE SECTION","VOIR AUTRES INFORMATIONS","VOIR RENSEIGNEMENTS","VER INFORMACI","DECLARADO DESIERT","QUEDA DESIERT","NESSUNA AGGIUDICAZIONE"]
WHOLE = ["VARIOUS","DESIERTO","DESIERTA","DESERTO","DESERTA"]
def ascii_upper(s): return "".join(ch.upper() if ord(ch)<128 else ch for ch in s)
def text_rule(name):
    n=name.strip()
    if ascii_upper(n) in WHOLE: return True
    up=ascii_upper(n)
    return any(p in up for p in SUB)
