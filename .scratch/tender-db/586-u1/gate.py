"""586 unit 1 (b): the touchpoint gate stems, applied to every touchpoint name in the saved answers.

Read-only and local: no box, no network, no cargo. Run: python3 gate.py [--hits]
Populations: pop.py. Mechanisms (each a pure function of the name, as `touchpoint_gate` is):

  WHOLE      the folded name IS the label             fold(name) == phrase
  HONORIFIC  a person addressed by title              raw name starts with a listed title, then a word,
                                                      and the name holds no commercial legal form
  PROSE      a filing / pointer sentence              folded phrase anywhere, whole words
  LEGALREF   a statute article reference              folded `art|articulo|...` then a number token
  PLATFORM   an e-procurement platform's name         folded phrase anywhere, whole words

`fold` is `partyname::fold` (= `role_census::fold`): match_norm + Latin diacritics.
"""
import collections
import json
import re
import sys

sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/586-u1')
from pop import all_pops, clusters, fold, r3_top60  # noqa: E402

# ---------------------------------------------------------------- the stems (the decision's candidate lists)

# WHOLE: the name is a contact label, no body's name. Whole-name only, so a body that merely holds the word
# ("Principal Financial Group", "Palacio de Congresos de Valencia", "Dirección General de Recursos Humanos",
# "Ayuntamiento de Alcobendas", "Procurement Investment International") never matches.
WHOLE_VERDICT = [
    'principal',                        # 31975008, 715, ~4 bodies (hub)
    'sede tibidabo',                    # 31975019, 621, 1 body
    'direccion principal',              # 31974991, 420, 4+ bodies (hub)
    'info adicional',                   # 31975180, 294
    'recursos',                         # 31975009, 268
    'direccion postal',                 # 31975006, 221, 9 organs of one Cabildo
    'direccion recepcion de recursos',  # 31975004, 143
    'palacio provincial',               # 31975059, 139, 2 Diputaciones (hub)
    'direccion recurso',                # 31975568, 105
    'ayuntamiento',                     # 1689043, +125 from 26 parents (older hub)
    'procurement department',           # 31978475, 1 (Museros Town Council)
]
# Found in the same populations, same shape (see gate.md "Beyond the verdict's list").
WHOLE_FOUND = [
    'direcci n principal',              # 'Direcci¿n Principal' (U+00BF mojibake of the Madrid label), 7
    'direccion de recursos',            # 31975751, 5 (EMVS Madrid)
    'presentacion de recursos',         # 31975949, 30 (Valdemorillo)
    'direccion informacion adicional',  # 31978098, 3 (Distrito de Retiro: the Madrid label family)
    'informacion adicional',            # 0 seen; the spelled-out twin of 'info adicional'
    'contact',                          # 31976137, 8 (NL)
    'beschaffende stelle',              # 31975060, 53, 4 DB companies (hub)
    # eForms role labels pasted as the name (measure.md's MK examples, and an ES one)
    'body responsible for mediation procedures',                                          # 28
    'service from which information about the review procedure may be obtained',         # 26
    'organizacion que proporciona informacion adicional sobre el procedimiento de contratacion',  # 2
    'direccion para presentacion de recursos',                                            # 2
]

# HONORIFIC: a leading title, matched on the RAW name (case-insensitive) so the dot can disambiguate:
# folded `m` / `ing` / `sig` / `ms` lead real names ("M-30", "ING Bank", "SIG Combibloc"), so those need the dot.
# Mr/Mrs without a dot only in title case ("MRS Logística" is a company). Not listed on purpose: Dr. ("Dr. Bauer &
# Partner", "Dr. Negrín"), D./Don/Doña ("Valencia de Don Juan", "Puebla de Don Fadrique", initials), Excmo./Ilmo.
# (they lead bodies: "Excmo. Ayuntamiento de Algeciras").
HONORIFIC_RX = re.compile(
    r'^\s*(?:'
    r'(?i:monsieur|madame|mademoiselle|herr|frau|mme\.?|mlle\.?|dott\.?ssa|dott\.?|sig\.ra|sig\.|ing\.|m\.(?!\s*\w\.)|mrs\.|mr\.|ms\.)'
    r'|Mrs|Mr'
    r')(?=\s+\w)'
)
# role_census::PUBLIC_STEMS (a word starting with one): the second honorific guard ("M. I. Ayuntamiento de …").
PUBLIC_STEMS = ['gmina', 'miasto', 'miejsk', 'powiat', 'wojewodztw', 'urzad', 'ministerstw', 'ministry', 'ministere',
                'ministerio', 'ministero', 'instytut', 'institut', 'instituto', 'istituto', 'uniwersyte', 'universit',
                'politechni', 'publiczn', 'panstwow', 'stadt', 'gemeinde', 'landkreis', 'landratsamt', 'kommun',
                'comune', 'ajuntament', 'ayuntamiento', 'municipal', 'municipio', 'commune', 'obec', 'mesto']

# PROSE: folded whole-word phrases anywhere.
PROSE = [
    'procede recurso',      # "Procede recurso especial en materia de contratación…", "Procede recurso de reposición…"
    'ver apartado',         # "Ver apartado 17 de la HRC…" ×6, "VER APARTADO CORRESPONDIENTE PCAP"
    'en los lugares',       # "En los lugares establecidos en el artículo 16.4…", "…indicados en el artículo 51.3…"
    'lugares establecidos',
    'lugares indicados',
    'lugares previstos',
    'lugares habilitados',
]
# LEGALREF: an article number. Folded, so "art.", "art-", "ART", "artículo(s)" all meet; `article` (EN/FR/CA) is
# left out for the NGO "ARTICLE 19".
# Read OUTSIDE brackets: a bracketed reference annotates a body's name ("Tribunal Administrativo Central de Recursos
# Contractuales (Según artículo 50.1.a) y b) de la LCSP)", which binds the older row 22186722), an unbracketed one is
# part of a sentence. The abbreviation needs its dot or hyphen ("art. 16", "art- 51.3"), or a Spanish article or
# conjunction before it ("el art 44", "y art 16"): a bare "Art 21" leads a gallery's name.
LEGALREF_RAW_RX = re.compile(r'(?i)(?<!\w)(?:art\s*[.\-]\s*|art[ií]culos?\s+|artigo\s+|articolo\s+|artikel\s+)\d')
LEGALREF_FOLD_RX = re.compile(r'(?:^| )(?:el|del|y|en|segun|al) art \d')
BRACKET_RX = re.compile(r'\([^()]*\)')


def unbracketed(name):
    prev = None
    while prev != name:
        prev, name = name, BRACKET_RX.sub(' ', name)
    return name


def legalref(name):
    bare = unbracketed(name)
    m = LEGALREF_RAW_RX.search(bare) or LEGALREF_FOLD_RX.search(fold(bare))
    return m.group(0).strip() if m else None

# PLATFORM: folded whole-word phrases anywhere.
PLATFORM_VERDICT = ['tuttogare', 'plataforma electronica de contratacion']
PLATFORM_FOUND = ['platforma zakupowa', 'platforma przetargowa', 'e zamowienia', 'ezamowienia', 'logintrade',
                  'achizitii md']

# The census's commercial forms (role_census::COMMERCIAL_FORMS), the honorific guard
# (`role_census::has_commercial_form`).
COMMERCIAL_FORMS = [
    'sp z o o', 'sp z oo', 'spolka z ograniczona odpowiedzialnoscia', 'spolka akcyjna', 'spolka komandytowa',
    'spolka jawna', 'sp k', 'sp j', 's a', 'sa', 'gmbh', 'ag', 'kg', 's r o', 'spol s r o', 'a s', 'as', 'asa', 'ltd',
    'limited', 'plc', 'llp', 'llc', 'inc', 'b v', 'bv', 'n v', 'nv', 's r l', 'srl', 's p a', 'spa', 's l', 'sl',
    's l u', 'slu', 'sas', 's a s', 'sarl', 's a r l', 'eurl', 'oy', 'oyj', 'ab', 'aps', 'kft', 'zrt', 'nyrt', 'd o o',
    'doo', 'uab', 'sia',
]

# ---------------------------------------------------------------- today's gate (D576-1 (iv)), to split old from new

# role_census NAME_PATTERNS, the Platform and Portal lists `portal_name` reads.
PORTAL_PHRASES = ['european dynamics', 'eu supply', 'mercell', 'vortal', 'cosinex', 'subreport',
                  'deutsches vergabeportal', 'tenderned', 'digitaal via', 'deze aanbesteding', 'negometrix',
                  'achatpublic', 'achatpublic com', 'plataforma de contratacion del sector publico']


def words_in(folded, phrase):
    return bool(folded) and f' {phrase} ' in f' {folded} '


def commercial(folded):
    return any(words_in(folded, f) for f in COMMERCIAL_FORMS)


def public(folded):
    return any(w.startswith(st) for w in folded.split() for st in PUBLIC_STEMS)


def gate_today(name):
    n = name.strip()
    head, _, num = n.partition('-')
    section = head.upper() in ('ORG', 'TPO', 'UBO') and num.isdigit() and num.isascii()
    low = n.lower()
    return section or '://' in low or 'www.' in low or '@' in n or any(words_in(fold(n), p) for p in PORTAL_PHRASES)


def stems(name, found=True):
    """Every new stem `name` meets, as [(mechanism, stem)]."""
    f = fold(name)
    hits = []
    whole = WHOLE_VERDICT + (WHOLE_FOUND if found else [])
    if f in whole:
        hits.append(('WHOLE', f))
    m = HONORIFIC_RX.match(name)
    if m and not commercial(f) and not public(f):
        hits.append(('HONORIFIC', m.group(0).strip()))
    for p in PROSE:
        if words_in(f, p):
            hits.append(('PROSE', p))
    ref = legalref(name)
    if ref:
        hits.append(('LEGALREF', ref))
    for p in PLATFORM_VERDICT + (PLATFORM_FOUND if found else []):
        if words_in(f, p):
            hits.append(('PLATFORM', p))
    return hits


def gate_new(name, found=True):
    return bool(stems(name, found))


# ---------------------------------------------------------------- bars

# Refute's classes, reconstructed from clusters.json by its own totals: N = 18 clusters / 37 orgs / 853 mentions
# (exact); X = the tribunal clusters (verdict: 23 / 50 / 340; the review-body pool below is a superset: every
# cluster whose key names a court, tribunal, review or appeals body, 25 clusters).
CLASS_N = [8, 29, 41, 51, 54, 55, 78, 86, 87, 89, 90, 91, 92, 106, 107, 118, 119, 120]
REVIEW_RX = re.compile(r'tribunal|juzgado|vergabekammer|vergabeprufstelle|mediateur|competition agency|'
                       r'αρχη|recursos contractuales|recursos de contratacion|tarcas')
REVIEW_POOL_EXCLUDE = set(CLASS_N)  # the N compounds that name a tribunal ("TACRC y art. 16 …") are N, not X

# Names that must never drop: the bar's own examples and their neighbours (synthetic, plus the test lists).
MUST_KEEP = [
    'Principal Financial Group', 'Principal Global Investors (Europe) Limited', 'Palacio de Congresos de Valencia',
    'Palacio de Festivales de Cantabria', 'Palacio de Exposiciones y Congresos de Granada',
    'Dirección General de Recursos Humanos', 'Recursos Humanos y Organización', 'Ayuntamiento de Alcobendas',
    'Excmo. Ayuntamiento de Algeciras', 'Procurement Investment International', 'Procurement Department of the NHS',
    'Sede Electrónica del Ayuntamiento de Teruel', 'Dirección Provincial del SEPE de Alicante',
    'Information Adicional S.L.', 'Ing. Büro SoFah GmbH & Co. KG', 'ING Bank N.V.', 'M-30 Madrid Calle 30 S.A.',
    'MRS Logística', 'SIG Combibloc', 'Dr. Bauer & Partner', 'Hospital Universitario Doctor Negrín',
    'Mr. Bricolage SA', 'Herrera de Pisuerga', 'Frauenklinik Hamburg', 'Madame Tussauds Amsterdam B.V.',
    'Plataforma Oceánica de Canarias (PLOCAN)', 'Plataforma Logística de Zaragoza PLAZA S.A.',
    'ARTICLE 19', 'Art 21 Gallery', 'Galería Art 21', 'M. I. Ayuntamiento de Elche', 'M.I. AYUNTAMIENTO DE LLIRIA',
    'Tribunal Administrativo Central de Recursos Contractuales (Según artículo 50.1.a) y b) de la LCSP)',
    'Hospital Real', 'Edificio San Francisco', 'Sede Central OAR', 'Sede principal del Patronato', 'Fundación Lugares Patrimonio', 'Valencia de Don Juan', 'Gerencia de Área de Salud Don Benito',
    'Tribunal Administrativo Central de Recursos Contractuales',
    'Tribunal Administrativo Central de Recursos Contractuales (TACRC) o Tribunal Superior de Justicia (TSJ).',
    'Tribunal Administrativo de Contratación Pública de la Comunidad de Madrid', 'Rechtbank Noord-Holland',
    'Vergabekammer Baden-Württemberg', 'Krajowa Izba Odwoławcza', 'TAR Calabria', 'Rechtbank',
    'Tribunal Català de Contractes del Sector Públic', 'Stadt Fürth - Zentrale Vergabestelle', 'Agentur Wieseke',
    'TARCAS. AYUNTAMIENTO DE SEVILLA', 'Bundesverwaltungsgericht', 'Nachprüfungsstelle', 'Bezwaarcommissie',
]
MUST_DROP = [
    'Principal', 'PRINCIPAL', 'Dirección Principal', 'Direccion Principal', 'DIRECCIÓN PRINCIPAL:', 'Sede Tibidabo',
    'Info Adicional', 'RECURSOS', 'Dirección Postal', 'Dirección Recepción de Recursos', 'Palacio Provincial',
    'Dirección Recurso', 'Ayuntamiento', 'Procurement Department', 'Monsieur Anthony LINTILHAC',
    'DOTT. ANGELO GAMBARDELLA', 'Dott.ssa Maria Rossi', 'dott marco lesto', 'M. Damien SIMON', 'Mme Julie ROLLIN',
    'Herr Thomas Müller', 'Frau Anna Schmidt', 'Mr John Smith', 'Mrs. Jane Doe', 'Ing. Pamela Comi', 'Sig. Mario Bianchi',
    'Procede recurso especial en materia de contratación, de acuerdo con lo establecido en los artículos 44 y siguientes',
    'Ver apartado 17 de la HRC. Rector de la Universidad de Vigo',
    'En los lugares establecidos en el artículo 16.4 de la Ley 39/2015', 'TACRC y art. 16 Ley 39/2015',
    'TRIBUNAL ADMINISTRATIVO CENTRAL DE RECURSOS CONTRACTUALES DE ACUERDO CON EL ART 44 DE LA LCSP',
    "TAR territorialmente competente ai sensi dell'art. 120 del D. Lgs. n. 104/2010", 'M. le Médiateur interne de l UGAP',
    'TUTTOGARE RACCONIGI', 'Plataforma Electrónica de Contratación de la Autoridad Portuaria de Barcelona',
]


def main(show_hits):
    pops = all_pops()
    out = []
    p = lambda *a: out.append(' '.join(str(x) for x in a))  # noqa: E731

    # 1. Every hit, per population, deduplicated by (population, name).
    agg = collections.OrderedDict()
    for r in pops:
        hits = stems(r['name'])
        if not hits:
            continue
        k = (r['pop'], r['name'])
        a = agg.setdefault(k, dict(n=0, orgs=set(), parents=set(), hits=hits, today=gate_today(r['name']),
                                   verdict_only=gate_new(r['name'], found=False)))
        a['n'] += r['n'] or 0
        if r.get('org'):
            a['orgs'].add(r['org'])
        if r.get('parent'):
            a['parents'].add(r['parent'][:60])
    by_pop = collections.defaultdict(lambda: [0, 0, 0, 0])
    for (pop, name), a in agg.items():
        b = by_pop[pop]
        b[0] += 1
        b[1] += a['n']
        if a['verdict_only']:
            b[2] += 1
            b[3] += a['n']
    totals = collections.Counter(r['pop'] for r in pops)
    weight = collections.Counter()
    for r in pops:
        weight[r['pop']] += r['n'] or 0
    p('## Hits per population (distinct names; weight = mentions, refs or rows as the population counts)')
    p(f'{"pop":6s} {"names":>6s} {"hits":>5s} {"hit-weight":>10s} {"of":>7s} {"verdict-list hits":>17s} {"weight":>7s}')
    for pop in totals:
        b = by_pop.get(pop, [0, 0, 0, 0])
        p(f'{pop:6s} {totals[pop]:6d} {b[0]:5d} {b[1]:10d} {weight[pop]:7d} {b[2]:17d} {b[3]:7d}')

    # 2. New orgs: by mechanism.
    new_hits = [(name, a) for (pop, name), a in agg.items() if pop == 'NEW']
    mech = collections.defaultdict(lambda: [0, 0])
    for name, a in new_hits:
        for m in sorted({h[0] for h in a['hits']}):
            mech[m][0] += len(a['orgs'])
            mech[m][1] += a['n']
    p('\n## New orgs (3,751) hit, by mechanism (an org can meet two)')
    for m, (o, n) in sorted(mech.items()):
        p(f'  {m:10s} {o:4d} orgs {n:5d} mentions')
    p(f'  ALL        {sum(len(a["orgs"]) for _, a in new_hits):4d} orgs {sum(a["n"] for _, a in new_hits):5d} mentions'
      f'   (verdict list only: {sum(len(a["orgs"]) for _, a in new_hits if a["verdict_only"])} orgs,'
      f' {sum(a["n"] for _, a in new_hits if a["verdict_only"])} mentions)')

    # 3. Bars.
    p('\n## Bar 1: R3 top 60 (576 unit 1)')
    for r in r3_top60():
        s, t = stems(r['name']), gate_today(r['name'])
        if s or t:
            p(f'  row {r["key"]:2d} refs {r["n"]:3d} today={t} new={s} {r["name"][:90]!r}')
    p(f'  new-stem hits: {sum(1 for r in r3_top60() if stems(r["name"]))} of 60')

    cl = json.load(open('/home/user/tender-db/.scratch/tender-db/audit-576-fanout/refute/clusters.json'))
    pool = [i for i, c in enumerate(cl) if REVIEW_RX.search(c['key']) and i not in REVIEW_POOL_EXCLUDE]
    p(f'\n## Bar 2: refute clusters. Class N reconstructed: {len(CLASS_N)} clusters, '
      f'{sum(cl[i]["n"] for i in CLASS_N)} orgs, {sum(m[1] for i in CLASS_N for m in cl[i]["members"])} mentions '
      f'(verdict: 18 / 37 / 853).')
    p(f'  Review-body pool (superset of class X): {len(pool)} clusters, {sum(cl[i]["n"] for i in pool)} orgs, '
      f'{sum(m[1] for i in pool for m in cl[i]["members"])} mentions (verdict X: 23 / 50 / 340).')
    pool_hits = [(i, m) for i in pool for m in cl[i]['members'] if stems(m[2])]
    p(f'  pool members hit: {len(pool_hits)}')
    for i, m in pool_hits:
        p(f'    cluster {i} {m}')
    hit_clusters = collections.defaultdict(list)
    for r in clusters():
        if stems(r['name']):
            hit_clusters[r['cluster']].append((r['org'], r['n'], r['name'][:80]))
    p('  every cluster with a hit (class):')
    for i, ms in sorted(hit_clusters.items()):
        p(f'    {i:3d} {"N" if i in CLASS_N else ("POOL" if i in pool else "-"):4s} {cl[i]["key"][:70]!r} {ms}')

    p('\n## Bar 3: names that must keep / must drop')
    bad_keep = [(n, stems(n)) for n in MUST_KEEP if stems(n)]
    bad_drop = [n for n in MUST_DROP if not stems(n)]
    p(f'  MUST_KEEP hit (want 0): {len(bad_keep)} {bad_keep}')
    p(f'  MUST_DROP missed (want 0): {len(bad_drop)} {bad_drop}')

    # 4. Honorific near-misses: every name led by a title-like token, hit or not, so the guards are visible.
    lead = re.compile(r'(?i)^\s*(monsieur|madame|mme|mlle|m\.|dott|sig|herr|frau|mr|mrs|ms|ing|dr|d\.|don|do[nñ]a)(?=\s|\b|\.)')
    p('\n## Title-led names in every population (hit = dropped by HONORIFIC)')
    seen = set()
    for r in pops:
        if lead.match(r['name']) and r['name'] not in seen:
            seen.add(r['name'])
            p(f'  {"HIT " if any(h[0] == "HONORIFIC" for h in stems(r["name"])) else "keep"} {r["pop"]:5s} {r["name"][:90]!r}')

    if show_hits:
        p('\n## Every hit (population, weight, orgs, mechanism:stem, today\'s gate, verdict-list?, name, parents)')
        for (pop, name), a in sorted(agg.items(), key=lambda kv: (kv[0][0], -kv[1]['n'])):
            hs = ','.join(f'{m}:{s}' for m, s in a['hits'])
            p(f'  {pop:5s} {a["n"]:5d} orgs={len(a["orgs"]):2d} {hs:45s} today={int(a["today"])} '
              f'v={int(a["verdict_only"])} {name[:100]!r} || {sorted(a["parents"])[:3]}')
    print('\n'.join(out))


if __name__ == '__main__':
    main('--hits' in sys.argv)
