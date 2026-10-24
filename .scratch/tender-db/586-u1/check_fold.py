"""586 unit 1 adversarial check (check.md), half (a): fold.py's tiers on the saved answers, fresh salt.

python3 -I check_fold.py <section>   sections: salted | review | brackets | organs | muni | dept | articles |
                                                legal | windows | synth | all
Local only: reads audit-576-fanout's saved answers and fold.py; writes nothing.
"""
import hashlib
import json
import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fold  # noqa: E402

AUD = '/home/user/tender-db/.scratch/tender-db/audit-576-fanout'
SALT = '586-u1-check'
t2 = json.load(open(f'{AUD}/second-look/raw/t2_classified.json'))
for r in t2:
    r['tier'] = fold.tier(r['name'], r['pp'])
ALIASED = [r for r in t2 if r['tier']]


def salted(rows, salt, n=40, key=lambda r: r['id']):
    return sorted(rows, key=lambda r: hashlib.sha256(f"{salt}:{key(r)}".encode()).hexdigest())[:n]


def show(r, extra=''):
    print(f"  {r['id']} {r['tier'] or '-':3} m={r['tpo']:<4} cls={r['cls']:14} P={r['pp'][:95]!r}\n"
          f"{'':12}T={r['name'][:95]!r} {extra}")


def sec_salted():
    a = [r for r in ALIASED if r['tier'] == 'A']
    c = [r for r in ALIASED if r['tier'] != 'A']
    for name, rows in [('A', a), ('C', c)]:
        print(f'\n== salted {name} ({SALT}-tier{name}), 40 of {len(rows)}')
        for i, r in enumerate(salted(rows, f'{SALT}-tier{name}'), 1):
            print(f'{i:2}', end='')
            show(r)


REVIEW = re.compile(r'tribunal|recurs|juzgado|registr|vergabe|tarc|tacrc|review|comision juridica|consell de garanties|'
                    r'mediat|appeal|kammer|izba|rechtbank|tar |court|cour |defensor|ombud|oarc|kio|organo administrativo',
                    re.I)


def sec_review():
    print('\n== aliased pairs where either side names a review / registry / court word')
    for r in ALIASED:
        if REVIEW.search(fold.char_fold(r['name'])) or REVIEW.search(fold.char_fold(r['pp'])):
            show(r)
    print('\n== aliased pairs second-look classed review-body / dept-of-buyer? / other')
    for r in ALIASED:
        if r['cls'] in ('review-body', 'dept-of-buyer?'):
            show(r)


def sec_brackets():
    print('\n== aliased pairs whose PARENT prints a bracket (all cut on the parent side)')
    for r in ALIASED:
        pb = fold.parent_brackets(r['pp'])
        if pb:
            show(r, f'parent brackets={pb}')
    print('\n== aliased pairs whose TOUCHPOINT prints a bracket (cut by the allow-list)')
    for r in ALIASED:
        tb = fold.split_brackets(fold.char_fold(r['name']))[1]
        if tb:
            show(r, f'tp brackets={tb}')


def sec_organs():
    cnt = Counter()
    rows = defaultdict(list)
    for r in ALIASED:
        if r['tier'] != 'C1':
            continue
        p = fold.key_tokens(r['pp'], 'parent')
        organ, e = fold.organ_split(p)
        cnt[organ] += 1
        rows[organ].append((r, e))
    print('\n== C1 by organ cut:', cnt.most_common())
    risky = {'presidencia', 'vicepresidencia', 'direccion', 'direccion general', 'gerencia', 'asamblea', 'junta general',
             'consejo general', 'consejo gobierno', 'direccion ejecutiva', 'presidente', 'presidenta', 'director',
             'directora', 'director general', 'directora general', 'gerente', 'consejo directivo', 'comite directivo',
             'junta directiva', 'consejero delegado', 'consejera delegada', 'consejeria delegada', 'rector', 'rectora',
             'administrador unico', 'organo administracion', 'vicepresidente', 'vicepresidenta', 'comision ejecutiva',
             'comision permanente', 'comision gestora', 'junta rectora', 'consejo comarcal'}
    for organ in sorted(rows, key=str):
        if organ not in risky:
            continue
        print(f'\n-- organ {organ!r} ({len(rows[organ])})')
        for r, e in rows[organ]:
            show(r, f'E={" ".join(e)!r}')
    print('\n== C1 with a one-token entity')
    for organ, lst in rows.items():
        for r, e in lst:
            if len(e) == 1:
                show(r, f'organ={organ!r} E={e}')


def sec_muni():
    print('\n== C2/C3/C3r whose municipal word is not Spanish (or differs between the sides)')
    es = {'ayuntamiento', 'ajuntament', 'concello', 'ayto', 'aj'}
    for r in ALIASED:
        if r['tier'] not in ('C2', 'C3', 'C3r'):
            continue
        t = fold.key_tokens(r['name'], 'tp', fold.parent_brackets(r['pp']))
        p = fold.key_tokens(r['pp'], 'parent')
        _, e = fold.organ_split(p)
        heads = {x[0] for x in (t, e, p) if x and x[0] in fold.MUNI}
        if heads - es or len(heads) > 1:
            show(r, f'heads={sorted(heads)}')
    print('\n== every C3 / C3r (a bare place)')
    for r in ALIASED:
        if r['tier'] in ('C3', 'C3r'):
            show(r)


DEPT = re.compile(r'\b(concejal|regidor|area|servicio|servei|distrito|districte|secretar|delegacion|delegacio|departament|'
                  r'unidad|unitat|seccion|seccio|negociado|oficina|direccion provincial|direccion territorial|'
                  r'subdireccion|mesa|junta de contratacion|abteilung|amt|referat|afdeling|dienst|wydzial|biuro|'
                  r'ufficio|settore|servizio|direction|service)', re.I)


def sec_dept():
    print('\n== aliased pairs where one side carries a department word the other lacks (raw, char-folded)')
    for r in ALIASED:
        pt, tt = fold.char_fold(r['pp']), fold.char_fold(r['name'])
        dp = set(m.group(1) for m in DEPT.finditer(pt))
        dt = set(m.group(1) for m in DEPT.finditer(tt))
        if dp != dt:
            show(r, f'P-only={sorted(dp - dt)} T-only={sorted(dt - dp)}')


def sec_articles():
    print('\n== aliased pairs whose raw word sets differ in a dropped one-letter token or numeral-like token')
    one = re.compile(r'(?<![\w.])([a-z]|[ivx]{1,4}|\d+)(?![\w.])')
    for r in ALIASED:
        pt = set(one.findall(fold.char_fold(r['pp'])))
        tt = set(one.findall(fold.char_fold(r['name'])))
        if pt != tt:
            show(r, f'P-only={sorted(pt - tt)} T-only={sorted(tt - pt)}')


def sec_legal():
    print('\n== tier A pairs where the legal form differs between the sides (raw tails)')
    for r in ALIASED:
        if r['tier'] != 'A':
            continue
        pt = fold.fold_tokens(r['pp'], 'parent')
        tt = fold.fold_tokens(r['name'], 'tp', fold.parent_brackets(r['pp']))
        pl = fold._drop_trailing(fold._drop_anywhere(pt, fold._LEGAL_ANY), fold._LEGAL_END)
        tl = fold._drop_trailing(fold._drop_anywhere(tt, fold._LEGAL_ANY), fold._LEGAL_END)
        if (len(pt) - len(pl)) != (len(tt) - len(tl)) or pt[len(pl):] != tt[len(tl):]:
            show(r, f'P-tail={pt[len(pl):]} T-tail={tt[len(tl):]}')


def sec_windows():
    for f in ['03-t3_window_a.json', '05-t5_window_b.json']:
        w = json.load(open(f'{AUD}/second-look/raw/{f}'))
        G = [dict(zip(w['columns'], row)) for row in w['rows']]
        al = []
        for g in G:
            if g['porg'] is None or g['org'] == g['porg']:
                continue
            t = fold.tier(g['tp_printed'] or '', g['p_printed'] or '')
            if t:
                g['tier'] = t
                al.append(g)
        print(f'\n== {f}: {len(al)} aliased groups, {sum(g["n"] for g in al)} mentions; salted 40 ({SALT}-{f[:2]})')
        for i, g in enumerate(salted(al, f'{SALT}-{f[:2]}', key=lambda g: f"{g['org']}:{g['porg']}"), 1):
            print(f"{i:2} {g['tier']:3} n={g['n']:<3} org={g['org']} ({g['oname'][:60]!r}) porg={g['porg']}\n"
                  f"{'':6}P={g['p_printed'][:95]!r}\n{'':6}T={g['tp_printed'][:95]!r}")
        print(f'-- {f}: aliased groups naming a review / registry word')
        for g in al:
            if REVIEW.search(fold.char_fold(g['tp_printed'])) or REVIEW.search(fold.char_fold(g['oname'] or '')):
                print(f"   {g['tier']} n={g['n']} org={g['org']} {g['oname'][:60]!r} P={g['p_printed'][:70]!r} "
                      f"T={g['tp_printed'][:70]!r}")


SYNTH = [
    # Polish: rural gmina vs the town of the same name (two legal entities)
    ('Miasto Kłodzko', 'Gmina Kłodzko', None, 'urban vs rural gmina: two entities'),
    ('Gmina Kłodzko', 'Miasto Kłodzko', None, 'urban vs rural gmina: two entities'),
    ('Gmina Chełm', 'Miasto Chełm', None, 'rural gmina vs city with powiat rights'),
    ('Kłodzko', 'Gmina Kłodzko', '?', 'bare town under a rural gmina'),
    ('Miasto Siedlce', 'Burmistrz Gminy Siedlce', None, ''),
    # German: Stadt vs Landkreis / Verbandsgemeinde
    ('Landkreis Fürth', 'Stadt Fürth', None, 'district vs town'),
    ('Stadt Fürth', 'Gemeinde Fürth', '?', 'Stadt vs Gemeinde, same town'),
    # department in the PARENT's bracket: the entity lands on the department (T1's W shape)
    ('Ayuntamiento de Alcobendas', 'Ayuntamiento de Alcobendas (Contratación y Compras)', None, 'parent bracket dept'),
    ('Servicio Murciano de Salud', 'Servicio Murciano de Salud (Gerencia de Área IV)', None, 'parent bracket dept'),
    ('Ayuntamiento de Sevilla', 'Ayuntamiento de Sevilla (TARCAS)', None, 'parent bracket review organ'),
    ('Universidad de Granada', 'Universidad de Granada (Servicio de Contratación)', None, 'parent bracket dept'),
    # broader government under a department-like "organ"
    ('Gobierno de Canarias', 'Vicepresidencia del Gobierno de Canarias', None, 'department of the government'),
    ('Generalitat Valenciana', 'Presidencia de la Generalitat Valenciana', '?', 'Presidència = a department'),
    ('Junta de Andalucía', 'Dirección General de la Junta de Andalucía', '?', ''),
    ('Principado de Asturias', 'Junta General del Principado de Asturias', None, 'parliament vs government'),
    ('Madrid', 'Asamblea de Madrid', None, 'bare place under a regional parliament'),
    ('Extremadura', 'Asamblea de Extremadura', None, 'bare place under a regional parliament'),
    ('Poder Judicial', 'Consejo General del Poder Judicial', '?', ''),
    ('Universidad', 'Rectorado de la Universidad', '?', ''),
    ('Tribunal Administrativo de Recursos Contractuales', 'Presidencia del Tribunal Administrativo de Recursos '
     'Contractuales', 'C1', 'the tribunal under its own organ (R)'),
    # review organ spelled into the parent
    ('Tribunal Administrativo de Recursos Contractuales de Castilla y León',
     'Consejo Consultivo de Castilla y León', None, ''),
    # company vs owning council
    ('Empresa Municipal de Aguas de Málaga', 'Ayuntamiento de Málaga', None, ''),
    ('Aguas de Alicante, Empresa Mixta', 'Aguas de Alicante', '?', 'empresa mixta vs short name'),
    ('Fira de Barcelona', 'Fira de Barcelona, S.A.', 'A', ''),
    ('Puerto de Vigo', 'Autoridad Portuaria de Vigo', None, ''),
    # one-letter discriminators dropped as articles
    ('Gerencia de Atención Primaria Área I', 'Gerencia de Atención Primaria Área', None, 'roman numeral I = ca article'),
    ('Zona A', 'Zona', None, ''),
    ('Lote E Servicios', 'Lote Servicios', None, ''),
    ('Hospital Universitario Carlos I', 'Hospital Universitario Carlos', None, ''),
    ('Escuela Oficial de Idiomas I', 'Escuela Oficial de Idiomas', None, ''),
    # Unicode
    ('İSTANBUL BÜYÜKŞEHİR BELEDİYESİ', 'Istanbul Büyükşehir Belediyesi', 'A', 'Turkish dotted I'),
    ('ISTANBUL BUYUKSEHIR BELEDIYESI', 'İstanbul Büyükşehir Belediyesi', 'A', 'Turkish dotted I (parent)'),
    ('Straße der Stadt', 'STRASSE DER STADT', 'A', 'ß vs SS'),
    ('ΔΗΜΟΣ ΑΘΗΝΑΙΩΝ', 'Δήμος Αθηναίων', 'A', 'Greek final sigma + tonos'),
    ('Col·legi Oficial d\'Arquitectes', 'Col.legi Oficial d\'Arquitectes', 'A', 'Catalan l·l'),
    ('Coŀlegi Oficial d\'Arquitectes', 'Col·legi Oficial d\'Arquitectes', 'A', 'Catalan ŀ (U+0140)'),
    ('Il·lustre Col·legi d\'Advocats', 'Col·legi d\'Advocats', '?', 'il dropped as Italian article'),
    ('Gestión Agroambiental', 'Gestión Agroambiental', 'A', 'decomposed accent'),
    ('ＳＡＲＧＡ', 'SARGA', 'A', 'fullwidth'),
    ('Gmina Panki;', 'Gmina Panki;', 'A', 'U+037E'),
    ('Sociedad Aragonesa de Gestión Agroambiental S.L.U', 'Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)',
     'A', 'SARGA'),
    # legal form swallowing a distinct word
    ('Hotel Balneario Spa', 'Hotel Balneario', '?', 'spa as a legal form'),
    ('Agencia Tributaria AS', 'Agencia Tributaria', '?', ''),
    ('Correos y Telégrafos', 'Sociedad Estatal Correos y Telégrafos, S.A.', None, ''),
    ('Organismo Autónomo Local Patronato de Deportes', 'Patronato de Deportes', None, ''),
    ('O.A.L. Patronato de Deportes', 'Patronato de Deportes', 'A', 'O.A.L. are three articles'),
    # C3r with a non-town bare parent
    ('Comune di Roma', 'Roma', 'C3r', ''),
    ('Stadt Bayern', 'Bayern', 'C3r', 'nonsense, but a bare Land'),
    ('Gmina Mazowsze', 'Mazowsze', 'C3r', 'bare region as parent'),
]


def sec_synth():
    print('\n== synthetic pairs (want: None = must not alias; ? = judgement)')
    for tp, parent, want, note in SYNTH:
        got = fold.tier(tp, parent)
        flag = '' if want == '?' or got == want else '  <-- differs'
        print(f'  {str(got):4} want {str(want):4} {note:42} T={tp!r} P={parent!r}{flag}')
        if flag or want == '?':
            print(f"{'':10}keys T={fold.key_a(tp, 'tp', parent)!r} P={fold.key_a(parent)!r}")


if __name__ == '__main__':
    secs = sys.argv[1:] or ['all']
    table = dict(salted=sec_salted, review=sec_review, brackets=sec_brackets, organs=sec_organs, muni=sec_muni,
                 dept=sec_dept, articles=sec_articles, legal=sec_legal, windows=sec_windows, synth=sec_synth)
    for s in (table if secs == ['all'] else secs):
        table[s]()
