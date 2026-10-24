"""586 unit 1 check (check.md): the R cost of check.md's required fold changes on the saved answers.

python3 -I check_variants.py
Monkeypatches fold.py in memory only (fold.py on disk is the (a) half's and is not edited).
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fold  # noqa: E402

AUD = '/home/user/tender-db/.scratch/tender-db/audit-576-fanout'
t2 = json.load(open(f'{AUD}/second-look/raw/t2_classified.json'))
WIN = []
for f in ['03-t3_window_a.json', '05-t5_window_b.json']:
    w = json.load(open(f'{AUD}/second-look/raw/{f}'))
    for row in w['rows']:
        g = dict(zip(w['columns'], row))
        if g['porg'] is not None and g['org'] != g['porg']:
            WIN.append(g)

base_tier = fold.tier
BASE = {r['id']: base_tier(r['name'], r['pp']) for r in t2}
BASE_W = [base_tier(g['tp_printed'] or '', g['p_printed'] or '') for g in WIN]

# Change 1: no cross-word municipal equivalence in Polish (rural gmina vs town of the same name).
MUNI_NEW = fold.MUNI - {'gmina', 'miasto'}

# Change 2: a one-token entity only when the parent prints a legal form (a company under its organ).
def parent_has_form(parent):
    toks = fold.fold_tokens(parent, 'parent')
    cut = fold._drop_trailing(fold._drop_anywhere(toks, fold._LEGAL_ANY), fold._LEGAL_END)
    return len(cut) < len(toks)

# Change 3: a parent bracket is not cut when it names a department, registry or review organ.
DEPT_OR_REVIEW = re.compile(r'\b(contratacion|compras|servicio|servei|servizio|area|concejalia|regidoria|departament\w*|'
                            r'seccion|unidad|unitat|negociado|oficina|distrito|secretaria|delegacion|direccion|gerencia|'
                            r'tribunal|tarcas|tacrc|tarc|recurso\w*|registro|juzgado|mesa|junta de contratacion|'
                            r'abteilung|amt|afdeling|dienst|wydzial|biuro|ufficio|settore)\b')

# Change 4 (recommended): no organ cut whose entity is a government umbrella (a consejería-level department).
UMBRELLA = {'gobierno', 'govern', 'generalitat', 'xunta', 'principado', 'comunidad', 'comunitat', 'region', 'estado',
            'administracion', 'junta'}


def tier_new(tp, parent, c1=True, c2=True, c3=True, c4=True):
    pb = fold.parent_brackets(parent)
    if c3:
        keep = [b for b in pb if DEPT_OR_REVIEW.search(' '.join(fold.tokens_of(b)))]
        if keep:
            # the parent's department bracket stays in its key: re-key with those brackets left in
            outside, contents = fold.split_brackets(fold.char_fold(parent))
            parts = outside.split('\0')
            text = [parts[0]]
            for content, after in zip(contents, parts[1:]):
                text.append(' ' + content + ' ' if content in keep else ' ')
                text.append(after)
            parent_for_key = ''.join(text)
        else:
            parent_for_key = parent
    else:
        parent_for_key = parent
    t = fold.key_tokens(tp, 'tp', pb)
    p = fold.key_tokens(parent_for_key, 'parent')
    if not t or not p:
        return None
    if t == p:
        return 'A'
    if fold._organ_syn(t) == fold._organ_syn(p):
        return 'C0'
    organ, e = fold.organ_split(p)
    ok = fold._entity_ok(e)
    if c2 and len(e) == 1 and not parent_has_form(parent):
        ok = False
    if c4 and organ and e and e[0] in UMBRELLA:
        ok = False
    if organ and ok and t == e:
        return 'C1'
    muni = MUNI_NEW if c1 else fold.MUNI
    m = lambda toks: ['ayuntamiento'] + toks[1:] if toks and toks[0] in muni else list(toks)  # noqa: E731
    em, tm = m(e), m(t)
    if em[0] == 'ayuntamiento' and len(em) >= 2:
        if tm == em:
            return 'C2'
        if t == em[1:]:
            return 'C3'
    if tm[0] == 'ayuntamiento' and len(tm) >= 2 and p[0] not in muni and p == tm[1:]:
        return 'C3r'
    return None


def report(label, **kw):
    lost = [(r['id'], r['tpo'], BASE[r['id']], r['name'][:60], r['pp'][:70]) for r in t2
            if BASE[r['id']] and not tier_new(r['name'], r['pp'], **kw)]
    gained = [r['id'] for r in t2 if not BASE[r['id']] and tier_new(r['name'], r['pp'], **kw)]
    wl = sum(g['n'] for g, b in zip(WIN, BASE_W) if b and not tier_new(g['tp_printed'] or '', g['p_printed'] or '', **kw))
    print(f'{label}: P1 aliases lost {len(lost)} orgs / {sum(x[1] for x in lost)} mentions, gained {len(gained)}; '
          f'windows lost {wl} mentions')
    for x in lost:
        print('   ', x)


if __name__ == '__main__':
    off = dict(c1=False, c2=False, c3=False, c4=False)
    report('change 1 (MUNI without gmina/miasto)', **{**off, 'c1': True})
    report('change 2 (one-token entity needs a parent legal form)', **{**off, 'c2': True})
    report('change 3 (parent department/review bracket stays)', **{**off, 'c3': True})
    report('change 4 (no organ cut down to a government umbrella)', **{**off, 'c4': True})
    report('changes 1-4 together')
    for tp, parent in [('Miasto Kłodzko', 'Gmina Kłodzko'), ('Kłodzko', 'Gmina Kłodzko'), ('Madrid', 'Asamblea de Madrid'),
                       ('Ayuntamiento de Alcobendas', 'Ayuntamiento de Alcobendas (Contratación y Compras)'),
                       ('Ayuntamiento de Sevilla', 'Ayuntamiento de Sevilla (TARCAS)'),
                       ('Gobierno de Canarias', 'Vicepresidencia del Gobierno de Canarias'),
                       ('Principado de Asturias', 'Junta General del Principado de Asturias'),
                       ('GESPESA SAU', 'Gerencia de GESPESA, S.A.U.'),
                       ('Sociedad Aragonesa de Gestión Agroambiental S.L.U',
                        'Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)')]:
        print(f'  {str(base_tier(tp, parent)):4} -> {str(tier_new(tp, parent)):4}  T={tp!r} P={parent!r}')
