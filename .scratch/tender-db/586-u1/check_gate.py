"""586 unit 1 adversarial check (check.md), half (b): gate.py's stems against real organization names.

python3 check_gate.py [orgs|salted|synth|all]
Local only: imports gate.py / pop.py (the (b) half's), reads the saved answers, writes nothing.
"""
import hashlib
import json
import sys
from collections import defaultdict

sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/586-u1')
from gate import stems  # noqa: E402
from pop import all_pops  # noqa: E402

AUD = '/home/user/tender-db/.scratch/tender-db/audit-576-fanout'
SALT = '586-u1-check'


def rows(path):
    d = json.load(open(path))
    return [dict(zip(d['columns'], r)) for r in d['rows']]


def real_org_names():
    """Names printed as ORGANIZATION sections or served as org heads: real bodies, never touchpoint labels."""
    out = defaultdict(set)
    for r in json.load(open(f'{AUD}/second-look/raw/t2_classified.json')):
        out[r['pp']].add('t2.pp')
        out[r['pn']].add('t2.pn')
    for f in ['03-t3_window_a.json', '05-t5_window_b.json']:
        for r in rows(f'{AUD}/second-look/raw/{f}'):
            out[r['p_printed']].add(f[:2] + '.p_printed')
            out[r['porg_name']].add(f[:2] + '.porg_name')
            if r['org'] < 31974972:
                out[r['oname']].add(f[:2] + '.old_oname')
    for r in json.load(open(f'{AUD}/refute/raw/04-r4_parent_one.json'))['rows']:
        for v in r:
            if isinstance(v, str) and len(v) > 3:
                out[v].add('r4')
    out.pop(None, None)
    return out


def sec_orgs():
    names = real_org_names()
    hits = [(n, src, stems(n)) for n, src in names.items() if n and stems(n)]
    print(f'== real organization names (parents printed, served heads, older rows): {len(names)} distinct; '
          f'stem hits: {len(hits)}')
    for n, src, s in sorted(hits):
        print(f'  {s} {n[:110]!r} {sorted(src)}')


def sec_salted():
    agg = {}
    for r in all_pops():
        s = stems(r['name'] or '')
        if s:
            a = agg.setdefault(r['name'], dict(n=0, pops=set(), s=s, parent=r.get('parent')))
            a['n'] += r['n'] or 0
            a['pops'].add(r['pop'])
    names = sorted(agg, key=lambda n: hashlib.sha256(f'{SALT}:{n}'.encode()).hexdigest())
    print(f'\n== salted 40 ({SALT}) of the {len(agg)} distinct hit names over every touchpoint population')
    for i, n in enumerate(names[:40], 1):
        a = agg[n]
        print(f'{i:2} {a["n"]:4} {sorted(a["pops"])} {a["s"]}\n     {n[:120]!r}\n     parent {str(a["parent"])[:90]!r}')


SYNTH_KEEP = [
    # real bodies whose names start with a title word, or hold a stem as a word
    'Ing. Büro Schneider', 'Ing.-Büro Schneider', 'Ing. Büro für Tragwerksplanung Müller', 'Herr & Frau Bau GmbH',
    'Frau Holle Kindergarten e.V.', 'Madame Tussauds', 'Monsieur Bricolage', 'Mr. Wash', 'Mr Bean Foundation',
    'M. Pons Construcciones', 'Sig. Rossi Costruzioni', 'Dott. Rossi & Partners Studio Associato',
    'Mme Figaro Edition', 'Herr der Ringe Stiftung', 'M. Ayuntamiento de Marbella', 'M.I. Ayuntamiento de Alicante',
    'Ms. Ware Software',
    # legal references in real names
    'Artículo 14 Asociación', 'Fundación Artículo 1', 'ART. 25 Arquitectos SLP', 'Teatro del Art 5',
    'Galería del Art 21', 'Artikel 1 Gleichstellungsbüro', 'Art-3 Gallery', 'Asociación Art. 27 de la Constitución',
    # platform / prose phrases in real names
    'Logintrade S.A.', 'Tuttogare S.r.l.', 'Studio Amica TUTTOGARE', 'Plataforma Electrónica de Contratación S.L.',
    'Centro de Recursos Ver Apartado', 'Asociación En los Lugares', 'Biuro e Zamówienia Sp. z o.o.',
    # labels as part of a name
    'Principal', 'Palacio Provincial de Cáceres', 'Sede Tibidabo Mutua Universal', 'Contact Software GmbH',
    'Contact', 'Recursos', 'Ayuntamiento', 'Beschaffende Stelle der Stadt Köln',
]


def sec_synth():
    print('\n== synthetic real-body names (want: kept; a hit is a candidate W)')
    for n in SYNTH_KEEP:
        s = stems(n)
        print(f"  {'HIT ' if s else 'keep'} {n!r} {s or ''}")


if __name__ == '__main__':
    secs = sys.argv[1:] or ['all']
    table = dict(orgs=sec_orgs, salted=sec_salted, synth=sec_synth)
    for s in (table if secs == ['all'] else secs):
        table[s]()
