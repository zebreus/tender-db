"""label.py -- issue 587 U1 decision oracle: `bare_label(name)`, the whole-name contracting-unit label test that
unit 2 ports to Rust (`crates/ingest/src/project/unit_labels_587.rs`). Pure; no network.

    python3 -I tools/label.py --selftest      # MUST_HIT / MUST_KEEP / town cases
    python3 -I tools/label.py --report        # apply to the 6,010 listed orgs (measure.json listing_orgs) -> label.out

Rule (decision.md D587-1): fold the name as `partyname::fold` does (lowercase, alphanumeric runs joined by one space,
Latin diacritics folded); drop CONNECT and NOISE tokens; what is left must be non-empty, every token in UNIT, FUNC or
COMPOUND, and it must hold (a UNIT token and a FUNC token) or a COMPOUND token. Nothing else: no prefix, no
'contains', no bracket cut (a bracket naming a body leaves its words, so the name is not bare).
"""
import json, os, sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(HERE, 'tools'))
from classify import fold, classify  # noqa: E402  (the measurer's partyname::fold port)

# Articles, prepositions, conjunctions (fr es ca gl pt it de nl en pl). Never a body's distinguishing word.
CONNECT = frozenset("""
la le les l de du des d et au aux a en
el los las lo del al y e i o
dels els
da do das dos
di da dei della delle degli dello dell ed il gli per
der die das den dem und fur im
het van voor
the of and for
w z ze oraz dla
""".split())

# Contact noise a publisher appends to a unit label ('… : responsable du service', 'Contacto').
NOISE = frozenset("responsable responsables contact contacto contacte punto point attn attention".split())

# Organizational-unit words. Not listed on purpose: junta, central/centrale (as a unit), centrale unica, stazione,
# groupement, agence, union, consejo, comision, secretaria, gerencia, alcaldia, intendencia (an organ or a body of
# its own, or 586's tier-C / BARE territory).
UNIT = frozenset("""
service services servicio servicios servizio servizi servei
direction direccion direccio
unidad unitat unite unidade
departement department departamento departament dept
division bureau cellule pole section seccion area oficina negociado
ufficio settore sezione
wydzial dzial zespol biuro sekcja referat
abteilung sachgebiet fachbereich fachdienst
afdeling afd team dienst
mesa organo secretariat
""".split())

# Procurement-function words: what a unit does, never whom it belongs to. Not listed on purpose: etat, estado,
# federal, finances, general (alone), regional/departmental, committenza, appaltante, consultiva, independiente,
# any place, person or body word -- each of those can be what makes a name a body ('Direction des Achats de l'Etat').
FUNC = frozenset("""
contratacion contrataciones contratos contrato contractacio compras compra suministros suministro adquisiciones
patrimonio patrimonial convenios asuntos generales administrativa administrativo administrativas administrativos
centralizada gestion economica economico financiera
commande commandes publique publiques public publics marches marche achats achat approvisionnement
approvisionnements contrats contrat juridique juridiques affaires logistique moyens generaux
zamowien zamowienia publicznych zaopatrzenia
inkoop aanbestedingen aanbestedingszaken aanbesteding
zentrale zentraler vergabe einkauf beschaffung
procurement purchasing contracts contract
gare appalti contratti acquisti provveditorato economato forniture lavori
assurances assurance transversaux ressources materielles travaux fournitures investissements mutualise mutualisee
umow bau inkoopondersteuning
""".split())

# One word that is a unit and its function at once.
COMPOUND = frozenset("vergabestelle vergabestellen beschaffungsstelle".split())


def label_tokens(name):
    return [t for t in fold(name).split() if t not in CONNECT and t not in NOISE]


def bare_label(name):
    """True when the whole folded name is a contracting-unit label that names no body."""
    toks = label_tokens(name)
    if not toks:
        return False
    if any(t not in UNIT and t not in FUNC and t not in COMPOUND for t in toks):
        return False
    has_unit = any(t in UNIT for t in toks)
    has_func = any(t in FUNC for t in toks)
    return any(t in COMPOUND for t in toks) or (has_unit and has_func)


def town_key(town):
    """D587-2's town guard key: folded tokens minus 'cedex' and pure digits."""
    return [t for t in fold(town or '').split() if t != 'cedex' and not t.isdigit()]


def same_town(a, b):
    """No evidence against (either side prints no town) or one key is a token prefix of the other."""
    ka, kb = town_key(a), town_key(b)
    if not ka or not kb:
        return True
    n = min(len(ka), len(kb))
    return ka[:n] == kb[:n]


MUST_HIT = [
    "Servicio de Contratación", "SERVICIO DE CONTRATACION", "Servicio de Contratacion", "Servicio de Contratación Administrativa",
    "Direction de la commande publique", "Direction de la Commande Publique", "Service de la Commande publique",
    "Service des marchés publics", "Service des marchés", "Service des achats et de la commande publique",
    "Zentrale Vergabestelle", "Vergabestelle", "Servicio de Contratación y Compras", "Unidad de Contratación",
    "Unidad de Contratacion", "Servicio de Contratación y Asuntos Generales", "Dział Zamówień Publicznych",
    "Dział Zamówień Publicznych i Zaopatrzenia", "Órgano de Contratación", "Mesa de contratación", "Ufficio gare",
    "Afdeling Inkoop", "Afd. Inkoop", "Team Inkoop", "Procurement Department", "Purchasing Department", "Procurement Team",
    "Direction des achats", "Service des achats", "Servicio de Contratación y Suministros",
    "Servicio de Contratación y Patrimonio", "Servicio de Contratación Centralizada",
    "Service de la commande publique: responsable du service", "Service des achats et approvisionnements",
    "Servicio de Contratación y Convenios", "Unidad de Contratación Administrativa",
    "Direction de la commande publique - service des marchés", "Servicio de Contratación, Compras y Patrimonio",
    "Servicio de Contratación y Gestión Patrimonial", "Service de la commande publique et des marchés",
    "Zespół Zamówień Publicznych", "Biuro Zamówień Publicznych", "Ufficio gare e contratti", "Direction des affaires juridiques",
    "Vergabestelle Bau", "Direction des achats, marchés et assurances", "Ufficio Gare Servizi e Forniture",
    "Direction de la commande publique et achats transversaux", "Direction de la commande publique : secrétariat",
]

# Real bodies, buyer departments under their own name (D576-1's accepted N), coordinating bodies, organs that are
# other bodies, and 586 territory. None may hit.
MUST_KEEP = [
    "Direction des Achats de l'État", "Direction des achats de l'Etat (DAE)", "Service des achats de l'État",
    "Union des groupements d'achats publics (UGAP)", "UGAP", "Centrale d'achat du numérique",
    "Junta de Contratación Centralizada", "Junta Consultiva de Contratación Administrativa",
    "Dirección General de Racionalización y Centralización de la Contratación", "Central de Contratación",
    "Oficina Independiente de Regulación y Supervisión de la Contratación",
    "Plataforma de Contratación del Sector Público", "Dirección General de Patrimonio del Estado",
    "Centrale Unica di Committenza", "Stazione Unica Appaltante", "Groupement de commandes",
    "Beschaffungsamt des BMI", "Bundesbeschaffung GmbH", "Zentrale Vergabestelle des Landes Brandenburg",
    "Stadt Fürth - Zentrale Vergabestelle", "Stadt Erlangen - Zentrale Vergabestelle",
    "Ville de Lyon - Direction de la commande publique", "Servicio de Contratación del Ayuntamiento de Vigo",
    "Unidad de Contratación del Hospital Universitario Ramón y Cajal", "Órgano de contratación de ADIF",
    "Mesa de Contratación de la Diputación de Cádiz", "Vergabestelle Landeshauptstadt Stuttgart",
    "Team Inkoop gemeente Westland", "Afdeling Inkoop Gemeente Utrecht", "Dział Zamówień Publicznych UJ",
    "Service Public Fédéral Finances", "Service public de Wallonie", "Servicio Andaluz de Salud",
    "Área Metropolitana de Barcelona", "Bureau Veritas", "Secretaría General", "Dirección General", "Ayuntamiento",
    "Pleno", "Direction générale", "Service", "Inkoop", "Zentrale Vergabe", "Einkauf", "Procurement",
    "Tribunal Administrativo de Recursos Contractuales de Andalucía", "Commercial Services", "Crown Commercial Service",
    "Service des marchés publics Eric Baleine", "Direction de la commande publique 1 place Salvador Allende 94010 Créteil",
    "Servicio de Contratación (Ayuntamiento de Sevilla)", "Unidad de Contratación 2", "Junta de Contratación",
    "Direction de la Commande Publique de la Ville de Paris", "DCP", "La Poste - Direction des achats",
    "Service des marchés publics: Hôtel du Département", "Bau GmbH", "Travaux Publics de l'Ouest", "Secrétariat général",
    "Fournitures Hospitalières SA", "Assurances du Crédit Mutuel",
]

TOWN_CASES = [
    ("Paris Cedex 07", "Paris", True), ("Wuppertal", "Wuppertal", True), ("Saint-Denis", "Saint-Étienne", False),
    ("", "Sevilla", True), ("Ratzeburg", "Mölln", False), ("STRASBOURG CEDEX", "Strasbourg", True),
]


def selftest():
    bad = [n for n in MUST_HIT if not bare_label(n)] + [n for n in MUST_KEEP if bare_label(n)]
    bad += [f'town {a!r}/{b!r}' for a, b, want in TOWN_CASES if same_town(a, b) != want]
    for b in bad:
        print('FAIL', b)
    total = len(MUST_HIT) + len(MUST_KEEP) + len(TOWN_CASES)
    print(f'selftest {total - len(bad)}/{total}')
    return not bad


def report():
    m = json.load(open(os.path.join(HERE, 'measure.json')))
    orgs = m['prefix_listings'] if isinstance(m['prefix_listings'], list) else None
    names = json.load(open(os.path.join(HERE, 'raw', 'names.json')))
    orgs = names['listing_orgs']
    out = {'listed_orgs': len(orgs), 'by_measured_class': {}, 'hit_by_key': {}, 'hits_outside_bare': [],
           'bare_misses_top': [], 'hits_top': []}
    def key(o):
        if o.get('identifier'):
            return 'identified'
        return 'name-keyed (country)' if o.get('country') else 'no id, NULL country'
    misses = []
    hits = []
    for o in orgs:
        h = bare_label(o['name'])
        c = o['cls']
        slot = out['by_measured_class'].setdefault(c, {'orgs': 0, 'mentions': 0, 'hit_orgs': 0, 'hit_mentions': 0})
        slot['orgs'] += 1
        slot['mentions'] += o['mentions']
        if h:
            slot['hit_orgs'] += 1
            slot['hit_mentions'] += o['mentions']
            k = out['hit_by_key'].setdefault(key(o), {'orgs': 0, 'mentions': 0})
            k['orgs'] += 1
            k['mentions'] += o['mentions']
            hits.append(o)
            if not c.startswith('BARE'):
                out['hits_outside_bare'].append([o['id'], o['name'], c, o['mentions']])
        elif c.startswith('BARE'):
            misses.append(o)
    misses.sort(key=lambda o: -o['mentions'])
    hits.sort(key=lambda o: -o['mentions'])
    out['bare_misses_top'] = [[o['id'], o['name'], o['cls'], key(o), o['mentions']] for o in misses[:60]]
    out['bare_miss_totals'] = {'orgs': len(misses), 'mentions': sum(o['mentions'] for o in misses),
                               'name_keyed_country': sum(o['mentions'] for o in misses if key(o) == 'name-keyed (country)')}
    out['hits_top'] = [[o['id'], o['name'], key(o), o['mentions']] for o in hits[:40]]
    out['hit_totals'] = {'orgs': len(hits), 'mentions': sum(o['mentions'] for o in hits)}
    # The 37 measured bare orgs and the 4 controls: hit or not.
    measured = {b['org_id']: b for b in m['buyers_beside']}
    out['measured'] = [[i, b['name'], b['verdict_class'], b['mentions'], bare_label(b['name'])] for i, b in measured.items()]
    json.dump(out, open(os.path.join(HERE, 'label.out.json'), 'w'), indent=1, ensure_ascii=False)
    with open(os.path.join(HERE, 'label.out'), 'w') as fh:
        fh.write(json.dumps({k: v for k, v in out.items() if k not in ('bare_misses_top', 'hits_top', 'measured',
                                                                        'hits_outside_bare')}, indent=1, ensure_ascii=False))
        fh.write('\n\n## hits outside the measured BARE classes\n')
        for r in out['hits_outside_bare']:
            fh.write(f'{r}\n')
        fh.write('\n## measured orgs (id, name, verdict, mentions, hit)\n')
        for r in out['measured']:
            fh.write(f'{r}\n')
        fh.write('\n## top hits\n')
        for r in out['hits_top']:
            fh.write(f'{r}\n')
        fh.write('\n## top BARE misses (residue)\n')
        for r in out['bare_misses_top']:
            fh.write(f'{r}\n')
    print(open(os.path.join(HERE, 'label.out')).read()[:6000])


if __name__ == '__main__':
    if '--selftest' in sys.argv:
        sys.exit(0 if selftest() else 1)
    if '--report' in sys.argv:
        report()
