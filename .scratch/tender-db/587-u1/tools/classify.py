"""classify.py -- BARE / DEPT / OTHER over folded served names (partyname::fold port: lowercase, alnum runs,
store::fold_latin). Pure; imported by measure.py."""
import re
LAT = {}
for chars, base in [('àáâãäåāăąæ', 'a'), ('çćčĉ', 'c'), ('ďđ', 'd'), ('èéêëěęēė', 'e'), ('ğģ', 'g'), ('ìíîïıīį', 'i'), ('ķ', 'k'),
                    ('ĺľłļ', 'l'), ('ñńňņ', 'n'), ('òóôõöøőœ', 'o'), ('ŕř', 'r'), ('śšşș', 's'), ('ťţț', 't'), ('ùúûüůűūų', 'u'),
                    ('ýÿ', 'y'), ('źżž', 'z'), ('ß', 's')]:
    for ch in chars:
        LAT[ch] = base

def fold(name):
    out, gap = [], False
    for c in name:
        if c.isalnum():
            if gap and out:
                out.append(' ')
            gap = False
            out.append(''.join(LAT.get(x, x) for x in c.lower()))
        else:
            gap = True
    return ''.join(out)

PHRASES = ["direction de la commande publique", "service de la commande publique", "service des marches publics",
           "direction des achats", "service des achats", "unidad de contratacion", "servicio de contratacion",
           "mesa de contratacion", "organo de contratacion", "dzial zamowien publicznych", "zespol zamowien publicznych",
           "biuro zamowien publicznych", "afdeling inkoop", "team inkoop", "zentrale vergabestelle", "vergabestelle",
           "procurement department", "purchasing department", "procurement team", "ufficio gare"]

# Articles, prepositions, conjunctions (FR ES CA PL NL DE EN IT).
NOISE = set("""la le les l de du des d et en au aux a el los las del y e i o al lo un una
w z ze oraz dla na do
het van voor en der die das den dem und im fur zur zum am an bei beim
the of and for at
di da dei della delle degli dello del ed il gli per""".split())
# A generic suffix (task: "- Service" / "Département"), contact or role noise, and legal noise.
SUFFIX = set("""service services departement department dept direction secretariat secretariado secretaria responsable responsables
bureau cellule pole division unite unit section seccion servicio servicios unidad area oficina negociado
ufficio settore servizio sezione wydzial dzial zespol biuro sekcja referat sachgebiet abteilung fachbereich fachdienst
afdeling afd team cluster dienst
contact contacto punto point attention attn
sas sa sl slu sarl gmbh ag ev e v""".split())
# Generic unit vocabulary: procurement / purchasing / legal / logistics words that name a function, never a body.
GENERIC = set("""commande publique publiques public publics marches marche march achat achats achateurs acheteur contrats contrat
contractualisation juridique juridiques affaires assurances assurance logistique approvisionnement approvisionnements
moyens generaux ressources materielles financieres finances investissements transversaux centralises centralise mutualise
mutualisee commandes gestion administrative administratif administratifs patrimoine immobilier travaux fournitures
contratacion contrataciones contratos compras suministros convenios asuntos generales administrativa administrativas
administrativo patrimonial patrimonio centralizada economica economico financiera gestion adquisiciones
zamowien publicznych zaopatrzenia umow zamowienia logistyki inwestycji administracyjny
inkoop aanbestedingszaken aanbestedingen aanbesteding inkoopondersteuning facilitair financien contractmanagement
zentrale zentraler vergabestelle vergabe einkauf beschaffung beschaffungsstelle bau vergaben vertragsmanagement recht
procurement purchasing contracts contract commercial services supplies
gare appalti contratti forniture servizi acquisti provveditorato economato lavori
hotel ville mairie region centre administratif maison adjoints m mme monsieur madame mr""".split())
LEGAL = set("sas sa sl slu sarl gmbh ag ev bv nv spa srl ltd plc sp zoo".split())

def _phrase_hits(f):
    pad = f' {f} '
    return [p for p in PHRASES if f' {p} ' in pad]

def classify(name, phrases=None):
    """(class, detail): BARE_STRICT, BARE_EXT, OTHER_ORGAN, DEPT. `phrases`: the census row's own list, else found."""
    f = fold(name)
    held = phrases or _phrase_hits(f)
    rest = f' {f} '
    for p in sorted(held, key=len, reverse=True):
        rest = rest.replace(f' {p} ', ' ', 1)
    toks = rest.split()
    left = [t for t in toks if t not in NOISE and t not in SUFFIX]
    if not left:
        return 'BARE_STRICT', ''
    unknown = [t for t in left if t not in GENERIC]
    if not unknown:
        return 'BARE_EXT', ' '.join(left)
    lead = held[0] if held else ''
    if f.startswith(('mesa de contratacion', 'organo de contratacion')):
        return 'OTHER_ORGAN', ' '.join(unknown)
    return 'DEPT', ' '.join(unknown)

if __name__ == '__main__':
    for n in ["Direction de la Commande Publique", "Ville de Lyon - Direction de la commande publique", "Unidad de Contratación",
              "Dział Zamówień Publicznych i Zaopatrzenia", "Mesa de Contratación de X", "Service de la commande publique - Responsable du service",
              "Zentrale Vergabestelle", "Afd. Inkoop", "Direction des achats et de la commande publique"]:
        print(n, '->', classify(n))
