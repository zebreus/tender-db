"""Second look, T2: is each new touchpoint org the buyer itself (or its department) under another spelling?
usage: python3 -I analyze_t2.py <second-look dir> <measurer 04-s04_newrows.json>"""
import json, sys, re, unicodedata, collections, os
S, NEWROWS = sys.argv[1], sys.argv[2]
d = json.load(open(os.path.join(S, 'raw', '02-t2_newparents.json')))
T2 = {r[0]: dict(zip(d['columns'], r)) for r in d['rows']}
n4 = json.load(open(NEWROWS))
ORG = {r[0]: dict(zip(n4['columns'], r)) for r in n4['rows'] if r[0] >= 31974972}

LEGAL = r'\b(s\.?\s?a\.?\s?u\.?|s\.?\s?l\.?\s?u\.?|s\.?\s?a\.?|s\.?\s?l\.?|sp\.?\s?z\s?o\.?\s?o\.?|spolka akcyjna|s\.?\s?p\.?\s?a\.?|s\.?\s?r\.?\s?l\.?|gmbh|ag|b\.?\s?v\.?|n\.?\s?v\.?|sas|oy|ab|as|e\.?\s?v\.?|mp|o\.?\s?a\.?)\b'
def fold(s):
    s = unicodedata.normalize('NFKD', s or '')
    s = ''.join(c for c in s if not unicodedata.combining(c)).lower()
    s = s.replace('ł', 'l').replace('ø', 'o').replace('ß', 'ss')
    s = re.sub(r'\(.*?\)', ' ', s)
    s = re.sub(r"[^\w]+", ' ', s)
    return ' '.join(s.split())
def loose(s):
    s = fold(s)
    s = re.sub(LEGAL, ' ', s)
    return ' '.join(s.split())
STOP = set('de la el los las del y e en a o i d l des du le les et van het der die und fur für w we z na do dla al a di da della per en of the and for to'.split())
def toks(s):
    return {t for t in loose(s).split() if t not in STOP and len(t) > 1}

DEPT = re.compile(r'\b(servicio|servei|unidad|area|departamento|seccion|negociado|mesa|junta de gobierno|alcaldia|pleno|'
                  r'contratacion|contractacio|compras|aprovisionamiento|direccion|gerencia|secretaria|registro|'
                  r'vergabestelle|beschaffung|einkauf|zentrale|inkoop|afdeling|dzial|biuro|wydzial|zespol|sekcja|'
                  r'service|direction|bureau|cellule|ufficio|settore|procurement|purchasing)\b')
REVIEW = re.compile(r'\b(tribunal|tribunale|trib|juzgado|jutjat|rechtbank|gerechtshof|kammer|vergabekammer|'
                    r'krajowa izba|kio|court|tar|tacrc|tarc|tacp|consell|comision|commissie|review|appeals?|complaints?|'
                    r'contestatiilor|rekurs|odwolawcza|beroep|arbitral|ombudsman|juzgados|audiencia|sala)\b')
rows = []
for oid, o in ORG.items():
    t = T2.get(oid)
    name = o['name']
    pp = (t or {}).get('p_printed') or ''
    pn = (t or {}).get('porg_name') or ''
    ln, lpp, lpn = loose(name), loose(pp), loose(pn)
    tn, tp = toks(name), toks(pp) | toks(pn)
    jac = len(tn & tp) / len(tn | tp) if (tn | tp) else 0.0
    cover = len(tn & tp) / len(tn) if tn else 0.0          # share of the new name's tokens found in the buyer's names
    pcover = len(tn & toks(pp)) / len(toks(pp)) if toks(pp) else 0.0   # share of the buyer's printed tokens in the new name
    if not t or t['porg'] is None:
        cls = 'no-parent'
    elif ln and (ln == lpp or ln == lpn):
        cls = 'SELF-spelling'            # the buyer's own name, differing only beyond org_name_norm
    elif ln and lpp and (lpp in ln or lpn in ln) and len(lpp) >= 6:
        cls = 'SELF-plus'                # buyer's full name + a department/suffix ("Stadt Fürth - Zentrale Vergabestelle")
    elif ln and (ln in lpp or (lpn and ln in lpn)) and len(ln) >= 6:
        cls = 'SELF-part'                # a part of the buyer's own name (acronym-free short form)
    elif REVIEW.search(fold(name)):
        cls = 'review-body'
    elif pcover >= 0.6 and cover >= 0.5:
        cls = 'SELF-overlap'             # most of the buyer's distinctive tokens and most of its own
    elif DEPT.search(fold(name)) and cover >= 0.34:
        cls = 'dept-of-buyer?'
    else:
        cls = 'other'
    rows.append(dict(id=oid, name=name, tpo=o['tpo_mentions'], cc=o['country'], cls=cls, jac=round(jac, 2),
                     cover=round(cover, 2), pcover=round(pcover, 2), porg=(t or {}).get('porg'), pp=pp, pn=pn,
                     pkind=(t or {}).get('pkind'), tp_cc=(t or {}).get('tp_cc'), own=(t or {}).get('tp_own_cc'),
                     pcc=(t or {}).get('porg_cc'), notice=(t or {}).get('notice_id'), sec=(t or {}).get('section_id')))
json.dump(rows, open(os.path.join(S, 'raw', 't2_classified.json'), 'w'), ensure_ascii=False, indent=0)
c = collections.Counter(r['cls'] for r in rows)
m = collections.Counter()
for r in rows:
    m[r['cls']] += r['tpo']
tot_o, tot_m = len(rows), sum(m.values())
print(f'orgs {tot_o}, their mentions {tot_m}')
for k, v in c.most_common():
    print(f'  {k:16} orgs {v:5} ({100*v/tot_o:4.1f}%)  mentions {m[k]:6} ({100*m[k]/tot_m:4.1f}%)')
pk = collections.Counter((r['cls'].startswith('SELF'), r['pkind'] is not None) for r in rows)
print('SELF x parent identified:', pk)
