"""586 unit 1, decision.md: the FINAL alias fold and gate (fold.md / gate.md as amended by check.md), as one oracle.

    python3 -I final.py [--selftest] [--report] [--vectors vectors_final.json] [--jsonl DIR]

Imports fold.py, gate.py and pop.py unchanged (the halves' files) and layers the decision's amendments on top:

Fold (D586-1):
  F1  MUNI without `gmina` / `miasto` (check.md required 1: a Polish rural gmina and its town are two units).
  F2  C1's one-token entity only when the parent prints a legal form (check.md required 2: "Madrid" under
      "Asamblea de Madrid").
  F3  a parent bracket that names a department, registry or review organ stays in the parent key (check.md
      required 3: "Ayuntamiento de Sevilla" under "Ayuntamiento de Sevilla (TARCAS)").
  F4  no C1 cut under a presidency / direction / general-assembly organ when the entity is a whole government
      (check.md recommended: "Gobierno de Canarias" under "Vicepresidencia del Gobierno de Canarias").
  C4  the touchpoint IS the parent's own leading organ, or the town-hall word of the parent's entity ("Pleno"
      under "Pleno de la Diputación Provincial de Huelva", "Ayuntamiento" under "Alcaldía del Ayuntamiento de X")
      (check.md gate required 2, preferred fix).
Gate (D586-2):
  G1  LEGALREF does not fire on a name that cites an EU act, nor on one holding a public-law stem or a commercial
      form (check.md gate required 1: the Stadt Dortmund mandate name).
  G2  WHOLE organ and registry labels (check.md gate required 2: the stems, the fallback where C4 does not alias).
  G3  HONORIFIC does not fire when the word after the title is an office word (check.md gate recommended: "Ing.
      Büro Schneider").
"""
import hashlib
import json
import os
import re
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fold  # noqa: E402
import gate  # noqa: E402
from pop import fold as pfold  # noqa: E402  (partyname::fold)

AUD = '/home/user/tender-db/.scratch/tender-db/audit-576-fanout'

# ------------------------------------------------------------------------------------------------ fold amendments

MUNI = fold.MUNI - {'gmina', 'miasto'}                                                    # F1

# F3: a parent bracket whose folded tokens hold one of these words stays in the parent key (check_variants.py).
DEPT_OR_REVIEW = re.compile(
    r'\b(contratacion|compras|servicio|servei|servizio|area|concejalia|regidoria|departament\w*|seccion|unidad|'
    r'unitat|negociado|oficina|distrito|secretaria|delegacion|direccion|gerencia|tribunal|tarcas|tacrc|tarc|'
    r'recurso\w*|registro|juzgado|mesa|junta de contratacion|abteilung|amt|afdeling|dienst|wydzial|biuro|ufficio|'
    r'settore)\b')

# F4: the organs that, over a whole government, are a department or another institution, not an organ of it.
UMBRELLA_ORGANS = {'presidencia', 'vicepresidencia', 'direccion', 'direccion general', 'direccion ejecutiva',
                   'junta general', 'asamblea', 'asamblea general', 'asamblea representantes', 'consejo general',
                   'presidente', 'presidenta', 'vicepresidente', 'vicepresidenta', 'director', 'directora',
                   'director general', 'directora general'}
UMBRELLA_HEADS = {'gobierno', 'govern', 'generalitat', 'xunta', 'principado', 'comunidad', 'comunitat', 'region',
                  'estado', 'administracion', 'junta'}


def parent_has_form(parent):                                                              # F2
    toks = fold.fold_tokens(parent, 'parent')
    cut = fold._drop_trailing(fold._drop_anywhere(toks, fold._LEGAL_ANY), fold._LEGAL_END)
    return len(cut) < len(toks)


def parent_key_tokens(parent):                                                            # F3
    """The parent's key with its department / registry / review brackets left in (every other bracket cut)."""
    outside, contents = fold.split_brackets(fold.char_fold(parent))
    keep = [b for b in contents if DEPT_OR_REVIEW.search(' '.join(fold.tokens_of(b)))]
    if not keep:
        return fold.key_tokens(parent, 'parent')
    parts = outside.split('\0')
    text = [parts[0]]
    for content, after in zip(contents, parts[1:]):
        text.append(' ' + content + ' ' if content in keep else ' ')
        text.append(after)
    return fold.key_tokens(''.join(text), 'parent')


def _muni(toks):
    return ['ayuntamiento'] + toks[1:] if toks and toks[0] in MUNI else list(toks)


def _syn_whole(toks):
    """C4's organ canonicalisation: ORGAN_SYNONYMS applied to the WHOLE token list too ('junta gobierno local' alone
    is 'junta gobierno'), which fold._organ_syn never does (it keeps something after the organ)."""
    for k, v in fold._SYN:
        if toks[:len(k)] == k:
            return v + toks[len(k):]
    return list(toks)


def tier(tp, parent):
    """'A', 'C0', 'C1', 'C2', 'C3', 'C3r', 'C4' or None (the touchpoint mints or meets the gate)."""
    pb = fold.parent_brackets(parent)
    t = fold.key_tokens(tp, 'tp', pb)
    p = parent_key_tokens(parent)
    if not t or not p:
        return None
    if t == p:
        return 'A'
    if fold._organ_syn(t) == fold._organ_syn(p):
        return 'C0'
    organ, e = fold.organ_split(p)
    ok = fold._entity_ok(e) and not (len(e) == 1 and not parent_has_form(parent))       # F2
    if organ in UMBRELLA_ORGANS and e and e[0] in UMBRELLA_HEADS:                        # F4
        ok = False
    if organ and ok and t == e:
        return 'C1'
    em, tm = _muni(e), _muni(t)
    if em[0] == 'ayuntamiento' and len(em) >= 2:
        if tm == em:
            return 'C2'
        if t == em[1:]:
            return 'C3'
    if tm[0] == 'ayuntamiento' and len(tm) >= 2 and p[0] not in MUNI and p == tm[1:]:
        return 'C3r'
    # C4: the touchpoint is the parent's own leading organ (synonyms canonicalised), or the bare town-hall word of
    # the parent's entity (after the organ, if any, is cut).
    if organ and _syn_whole(t) == _syn_whole(organ.split()):
        return 'C4'
    if tm == ['ayuntamiento'] and em[0] == 'ayuntamiento' and len(em) >= 2:
        return 'C4'
    return None


# ------------------------------------------------------------------------------------------------ gate amendments

# G2: a bare organ, officer, town-hall or registry word. Tested on the D586-1 key of the name alone (touchpoint side,
# no parent: contact-label, province and legal-only brackets cut, articles, honorifics and legal forms dropped), so
# "El Pleno", "Junta Gobierno Local", "Excmo. Ayuntamiento" all meet it. C4 aliases such a name when it is the
# parent's own organ or town-hall word; otherwise it is dropped (minting it would bind a cross-buyer row).
BARE_LABELS = ({' '.join(p) for p in fold._ORGANS} | {' '.join(k) for k, _ in fold._SYN} | set(fold.MUNI)
               | {'registro', 'registro general', 'registro electronico', 'sede electronica', 'secretaria general'})


def bare_key(name):
    """G2's key: the D586-1 key steps 1-4, 7 and 8 (touchpoint side, no parent), WITHOUT the legal-form steps 5-6 (a
    town named like a form must survive: "Gemeente Epe" is no bare "gemeente")."""
    toks = fold.fold_tokens(name, 'tp', ())
    return ' '.join(fold.join_initials([t for t in toks if t not in fold.ARTICLES and t not in fold.HONORIFICS]))


def bare_label(name):
    return bare_key(name) in BARE_LABELS


# G1: an EU act cited in the name (folded tokens / phrases).
EU_ACT = re.compile(r'(?:^| )(?:verordnung|richtlinie|regulation|directive|reglamento|regolamento|directiva|'
                    r'direttiva|eg|ewg|eu|ce|ue|cee)(?: |$)')
# G1b: LEGALREF fires only beside a statute cue: a law word, a numbered act or sub-article (`39/2015`, `16.4`), or a
# review-body word (the T class). A body named after an article ("Fundación Artículo 1", "Art-3 Gallery") has none.
LAW_CUE = re.compile(r'(?:^| )(?:ley|lcsp|trlcsp|lrjsp|lpacap|decreto|rd|lgs|gesetz|gwb|vgv|ustawa|pzp|loi|codice|'
                     r'tribunal|tar|juzgado|vergabekammer)(?: |$)')
NUMBERED = re.compile(r'\d\s*[/.]\s*\d')
# G3: the word after the title that makes it an office's name, not a person's.
OFFICE_NEXT = {'buro', 'buero', 'bureau', 'studio'}


def stems(name):
    """Every gate stem `name` meets after the amendments, as [(mechanism, stem)]."""
    f = pfold(name)
    out = []
    for mech, stem in gate.stems(name):
        if mech == 'LEGALREF' and (EU_ACT.search(f) or gate.public(f) or gate.commercial(f)):     # G1
            continue
        if mech == 'LEGALREF' and not (LAW_CUE.search(f) or NUMBERED.search(gate.unbracketed(name))):  # G1b
            continue
        if mech == 'HONORIFIC':                                                                    # G3
            rest = gate.HONORIFIC_RX.sub('', name, count=1).split()
            if rest and pfold(rest[0]).split()[:1] and pfold(rest[0]).split()[0] in OFFICE_NEXT:
                continue
        out.append((mech, stem))
    if bare_label(name):                                                                          # G2
        out.append(('BARE', bare_key(name)))
    return out


def decision(tp, parent):
    """The fold's verdict on one touchpoint name under its parent (the gate after the alias, as today)."""
    from pop import org_name_norm
    if org_name_norm(tp) == org_name_norm(parent):
        return 'norm'
    t = tier(tp, parent)
    if t:
        return t
    return 'drop' if stems(tp) else 'mint'


# ------------------------------------------------------------------------------------------------ report

def _rows(path):
    d = json.load(open(path))
    return [dict(zip(d['columns'], r)) for r in d['rows']]


def report():
    t2 = json.load(open(f'{AUD}/second-look/raw/t2_classified.json'))
    base = {r['id']: fold.tier(r['name'], r['pp']) for r in t2}
    res = defaultdict(list)
    for r in t2:
        r['final'] = tier(r['name'], r['pp'])
        res[r['final']].append(r)
    print('P1, the 3,751 new orgs: final tier (orgs / their touchpoint mentions)')
    for k in ['A', 'C0', 'C1', 'C2', 'C3', 'C3r', 'C4', None]:
        rows = res.get(k, [])
        print(f'  {str(k):5} {len(rows):5} {sum(r["tpo"] for r in rows):6}')
    a = res['A']
    c = [r for k in ['C0', 'C1', 'C2', 'C3', 'C3r', 'C4'] for r in res.get(k, [])]
    print(f'  tier A {len(a)} / {sum(r["tpo"] for r in a)}; tier C {len(c)} / {sum(r["tpo"] for r in c)}')
    moved = [(r['id'], base[r['id']], r['final'], r['name'][:60], r['pp'][:60]) for r in t2 if base[r['id']] != r['final']]
    print(f'  changed against fold.py: {len(moved)}')
    for m in moved:
        print('   ', m)
    # gate over the rows that do not alias
    hit = [r for r in t2 if not r['final'] and stems(r['name'])]
    print(f'P1 gate (rows that do not alias): {len(hit)} orgs / {sum(r["tpo"] for r in hit)} mentions')
    old = [r for r in t2 if not r['final'] and gate.stems(r['name'])]
    lost = sorted({r['name'] for r in old} - {r['name'] for r in hit})
    gained = sorted({r['name'] for r in hit} - {r['name'] for r in old})
    print(f'  against gate.py: no longer hit {lost}; newly hit {gained}')
    mint = [r for r in t2 if not r['final'] and not stems(r['name'])]
    print(f'P1 still minted: {len(mint)} orgs / {sum(r["tpo"] for r in mint)} mentions')
    # windows
    for f in ['03-t3_window_a.json', '05-t5_window_b.json']:
        G = _rows(f'{AUD}/second-look/raw/{f}')
        cnt, tot, c4, drops = Counter(), 0, [], []
        for g in G:
            tot += g['n']
            if g['porg'] is None or g['org'] == g['porg']:
                continue
            t = tier(g['tp_printed'] or '', g['p_printed'] or '')
            side = 'new' if g['org'] >= 31974972 else 'old'
            if t:
                cnt[(side, t if t == 'C4' else t[0])] += g['n']
                if t == 'C4':
                    c4.append((g['n'], g['org'], g['tp_printed'], g['p_printed']))
            elif stems(g['tp_printed'] or ''):
                drops.append((g['n'], g['org'], side, g['tp_printed'], [s for s in stems(g['tp_printed'] or '')]))
        print(f'P2 {f}: {tot} mentions; alias by (side, tier): {dict(cnt)}')
        for x in c4:
            print('   C4', x)
        print(f'   gate drops: {sum(d[0] for d in drops)} mentions')
        for d in drops:
            if d[2] == 'old':
                print('    old-row drop', d)
    # bars on the other populations
    from pop import all_pops, r3_top60
    r3 = [r for r in r3_top60() if stems(r['name'] or '')]
    print(f'R3 top 60: {len(r3)} hits {[r["name"] for r in r3]}')
    agg = Counter()
    for r in all_pops():
        for m, s in stems(r['name'] or ''):
            if m == 'BARE':
                agg[(r['pop'], r['name'])] += r['n'] or 0
    print('G2 organ/registry stem hits by population:', dict(agg))
    # G1 on real org names (check_gate.py's population)
    import check_gate
    names = check_gate.real_org_names()
    hits = [(n, stems(n)) for n in names if n and stems(n)]
    print(f'real organization names: {len(names)}; final stem hits {len(hits)}')
    for n, s in sorted(hits):
        print('  ', s, n[:110])
    print('synthetic keep list (want kept):')
    for n in check_gate.SYNTH_KEEP:
        s = stems(n)
        if s:
            print('   HIT', n, s)
    print('MUST_KEEP hits:', [n for n in gate.MUST_KEEP if stems(n)])
    print('MUST_DROP misses:', [n for n in getattr(gate, 'MUST_DROP', []) if not stems(n)])


SELF = [
    # unit 2's named tests
    ('Sociedad Aragonesa de Gestión Agroambiental S.L.U', 'Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)', 'A'),
    ('AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (Punto de contacto)', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', 'A'),
    ('AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (PUNTO DE CONTACTO)', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', 'A'),
    ('AGENCIA DE VIVIENDA Y REHABILITACION DE ANDALUCIA (PC)', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', 'A'),
    ('AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (otro punto de contacto)', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', 'A'),
    ('AGENCIA DE VIVIENDA DE REHABILITACION DE ANDALUCIA (punto de contacto)', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', 'A'),
    ('AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCIA', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', 'A'),
    ('DIRECCIÓN PROVINCIAL DE AVRA EN SEVILLA (punto de contacto)', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', None),
    ('Gmina Panki ul. Tysiąclecia 5; 42-140 Panki', 'Gmina Panki ul. Tysiąclecia 5; 42-140 Panki', 'A'),
    ('Gmina Panki ul. Tysiąclecia 5; 42-140 Panki', 'Gmina Panki', None),
    ('TARCAS. AYUNTAMIENTO DE SEVILLA', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
    ('Ayuntamiento de Sevilla - TARCAS', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
    ('Ayuntamiento de Sevilla (TARCAS)', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
    ('Registro del Tribunal Administrativo de Recursos Contractuales (TARCAS)', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
    ('Stadt Fürth - Zentrale Vergabestelle', 'Stadt Fürth', None),
    # check.md's must-not-alias additions (F1-F4)
    ('Miasto Kłodzko', 'Gmina Kłodzko', None),
    ('Kłodzko', 'Gmina Kłodzko', None),
    ('Madrid', 'Asamblea de Madrid', None),
    ('Ayuntamiento de Sevilla', 'Ayuntamiento de Sevilla (TARCAS)', None),
    ('Ayuntamiento de Alcobendas', 'Ayuntamiento de Alcobendas (Contratación y Compras)', None),
    ('Gobierno de Canarias', 'Vicepresidencia del Gobierno de Canarias', None),
    ('Principado de Asturias', 'Junta General del Principado de Asturias', None),
    # kept by the amendments
    ('GESPESA SAU', 'Gerencia de GESPESA, S.A.U.', 'C1'),
    ('Marugán', 'Pleno del Ayuntamiento de Marugán', 'C3'),
    ('Ajuntament de Biar', 'Junta de Gobierno del Ayuntamiento de Biar', 'C2'),
    ('Mairie de Mennecy', 'Commune de Mennecy', 'C2'),
    ('COMUNE DI LAURINO', 'LAURINO', 'C3r'),
    ('Ayuntamiento de Sevilla', 'Ayuntamiento de Sevilla (Sevilla)', 'A'),
    ('Consorcio X', 'Presidencia del Consorcio X', 'C1'),
    # C4
    ('Pleno', 'Pleno de la Diputación Provincial de Huelva', 'C4'),
    ('Rectorado', 'Rectorado de la Universidad Politécnica de Madrid', 'C4'),
    ('Junta de Gobierno Local', 'Junta de Gobierno del Ayuntamiento de La Nucía', 'C4'),
    ('Ayuntamiento', 'Alcaldía del Ayuntamiento de Biar', 'C4'),
    ('Ayuntamiento', 'Ayuntamiento de Biar', 'C4'),
    ('Alcaldía', 'Pleno del Ayuntamiento de Biar', None),          # another organ: the WHOLE stem drops it
    ('Pleno', 'Alcaldía del Ayuntamiento de Biar', None),
    ('Ayuntamiento', 'Concejalía de Hacienda del Ayuntamiento de Benalmádena', None),
    ('Registro General', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
]
GATE_SELF = [
    ('Dirección Principal', True), ('Principal', True), ('Palacio Provincial', True), ('Ayuntamiento', True),
    ('Monsieur Anthony LINTILHAC', True), ('DOTT. ANGELO GAMBARDELLA', True), ('TACRC y art. 16 Ley 39/2015', True),
    ('Ver apartado 17 de la HRC. Rector de la Universidad de Vigo', True), ('TUTTOGARE RACCONIGI', True),
    ('Platforma Zakupowa Logintrade', True), ('Registro General', True), ('Pleno', True), ('Alcaldía', True),
    ('Junta de Gobierno Local', True), ('Rectorado', True), ('Junta Gobierno Local', True), ('El Pleno', True),
    ('Secretaría General', True), ('Excmo. Ayuntamiento', True), ('Director', True), ('Gmina', True),
    ('Registro General (Registro del TARCAS)', False), ('Gemeente Epe', False), ('Secretaría General del Ayuntamiento de Sevilla', False),
    ('Stadt Dortmund, vertreten durch den Zweckverband Verkehrsverbund Rhein-Ruhr, dieser wiederum vertreten durch '
     'die Verkehrsverbund Rhein-Ruhr AöR, handelnd als Gruppe von Behörden nach Art. 2 b) Verordnung (EG) Nr. '
     '1370/2007', False),
    ('Ing. Büro Schneider', False), ('Ing. Büro SoFah GmbH & Co. KG', False), ('M. I. Ayuntamiento de Elche', False),
    ('Principal Financial Group', False), ('Palacio de Congresos de Valencia', False), ('Art 21 Gallery', False),
    ('Tribunal Administrativo Central de Recursos Contractuales (Según artículo 50.1.a) y b) de la LCSP)', False),
    ('Stadt Fürth - Zentrale Vergabestelle', False), ('Agentur Wieseke', False),
    ('Fundación Artículo 1', False), ('Art-3 Gallery', False), ('Asociación Art. 27 de la Constitución', False),
    ('Tribunal Administrativo de Contratación Pública de Galicia artículo 44', True),
    ('Tribunal Administrativo Central de Recursos Contractuales', False), ('Pleno de la Diputación de Huelva', False),
    ('Registro de la Propiedad de Madrid', False), ('Gerencia de Urbanismo de Sevilla', False),
]


def selftest():
    bad = 0
    for tp, parent, want in SELF:
        got = tier(tp, parent)
        if got != want:
            bad += 1
            print('FAIL tier', repr(tp), '|', repr(parent), 'want', want, 'got', got)
    for name, want in GATE_SELF:
        got = bool(stems(name))
        if got != want:
            bad += 1
            print('FAIL gate', repr(name), 'want', want, 'got', got, stems(name))
    n = len(SELF) + len(GATE_SELF)
    print(f'final selftest: {n - bad}/{n} pass')
    return bad == 0


if __name__ == '__main__':
    a = sys.argv[1:]
    if '--selftest' in a:
        ok = selftest()
        if not ok:
            sys.exit(1)
    if '--report' in a:
        report()
    if '--vectors' in a:
        out = a[a.index('--vectors') + 1]
        t2 = json.load(open(f'{AUD}/second-look/raw/t2_classified.json'))
        vec = []
        for r in sorted(t2, key=lambda r: r['id']):
            vec.append(dict(id=r['id'], tp=r['name'], parent=r['pp'],
                            key_tp=' '.join(fold.key_tokens(r['name'], 'tp', fold.parent_brackets(r['pp']))),
                            key_parent=' '.join(parent_key_tokens(r['pp'])), tier=tier(r['name'], r['pp']),
                            gate=[m for m, _ in stems(r['name'])]))
        json.dump(vec, open(out, 'w'), ensure_ascii=False, indent=0)
        print(f'wrote {len(vec)} vectors to {out}')
    if '--jsonl' in a:
        # unit 2's fixtures: vectors.jsonl ([touchpoint, parent, tier or null, gated-and-not-aliased]) and
        # selftest.jsonl (["tier", touchpoint, parent, want] / ["gate", name, null, want]).
        d = a[a.index('--jsonl') + 1]
        t2 = json.load(open(f'{AUD}/second-look/raw/t2_classified.json'))
        with open(os.path.join(d, 'vectors.jsonl'), 'w') as f:
            for r in sorted(t2, key=lambda r: r['id']):
                t = tier(r['name'], r['pp'])
                f.write(json.dumps([r['name'], r['pp'], t, (not t) and bool(stems(r['name']))], ensure_ascii=False) + '\n')
        with open(os.path.join(d, 'selftest.jsonl'), 'w') as f:
            for tp, parent, want in SELF:
                f.write(json.dumps(['tier', tp, parent, want], ensure_ascii=False) + '\n')
            for name, want in GATE_SELF:
                f.write(json.dumps(['gate', name, None, want], ensure_ascii=False) + '\n')
        print(f'wrote {d}/vectors.jsonl and {d}/selftest.jsonl')
