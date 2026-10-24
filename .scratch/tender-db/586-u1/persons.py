"""586 unit 1 (b): bare personal names among the new orgs (out of scope for the gate; counted). Heuristic + hand list."""
import re, sys
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/586-u1')
from pop import new_orgs, fold
from gate import stems, commercial
ORGW = set('''ayuntamiento ajuntament concello consejo consejeria conselleria servicio servicios departamento departament direccion direction tribunal gemeente stadt gmina comune junta area unidad seccion oficina registro secretaria gerencia hospital universidad universitat fundacion consorcio instituto agencia autoridad ministerio ministere ministerul diputacion cabildo presidencia alcaldia empresa sociedad mutua centro distrito afdeling team inkoop vergabestelle stelle amt dienst service pole cellule bureau biuro dzial zespol wydzial urzad sekcja referat abteilung fachbereich sachgebiet procurement purchasing contracts legal juridische juridico juridica administracion contratacion contracting compras aprovisionamiento economico economica tesoreria intervencion county council city borough office division unit group grupo groupe agence commune mairie region province provincia provincie waterschap stichting vereniging university college school academy board authority commission comision comite committee rechtbank court vergabekammer kammer gericht energia aguas agua puerto port aeropuerto parque museo teatro palacio casa edificio sede plaza calle avenida mancomunidad comarca'''.split())
PARTICLES = {'van', 'de', 'der', 'den', 'von', 'di', 'da', 'del', 'la', 'le', 'ter', 'ten', 'y', 'e'}
def person_shaped(name):
    f = fold(name); toks = f.split()
    if stems(name) or not (2 <= len(toks) <= 4): return False
    if any(t in ORGW for t in toks) or commercial(f) or any(c.isdigit() for c in f): return False
    raw = re.findall(r"[^\s,]+", name.strip())
    if not all(re.match(r"^[A-ZÀ-ÖØ-Þ][\w'’\-\.]*$", w) or w.lower() in PARTICLES for w in raw): return False
    if all(w.isupper() for w in raw if w.lower() not in PARTICLES): return False  # all-caps: mostly bodies/acronyms
    return True
if __name__ == '__main__':
    c = [r for r in new_orgs() if person_shaped(r['name'])]
    print(len(c), sum(r['n'] for r in c))
    for r in sorted(c, key=lambda r: -r['n']): print(r['key'], r['n'], repr(r['name']), '||', r['parent'][:50])

# Hand read of the heuristic's 296 candidates (persons.out): the ones that are a natural person's name, no title.
HAND_PERSONS = '''Michaela Cechova|Melinda Wartan Borbely|Miranda Vleeming|Rob van der Kooi|Jolanda van Ollefen|Marcel van Kallen|
Melinda Wartan-Borbely|Lucia Marinangeli|Sonja van der Eijk|Emma Kooij|Sandra Louwersheimer|Hrafnhildur Sigurðardóttir|
Bart van Santen|Christelle CORON|Louis Johannes|Jan Kampman|Giuseppe Daniele|B.Bervoets|Bart Wichers|Maria Amélia Sobral|
Ger Staal|Katarzyna Brodzik|Paul Belt|Sandrine AUBRY|Lisanne Koot|Ben Bouwman|Berfin Yardak|Maysa Sloekers|Jan Schraven|
Katerina Lisa|Silvan Blokzijl|Jorrit Kuipers|Geena den Daas|María José Cervigón|Alex Becker|Linda Neijensteijn|Eeuwe Winkel|
Sabrina Slanzi|Edith Bouwmeester|Roeland Winter|Fernando Valverde Sánchez|Angelo Gambardella|Judith COMMINSOLI|Joost Likkel|
Daan van Nieuwmegen|Jean-Frédéric LASSARA|Stéphane MAIN|Jos Peters|Evelyne POIGNON|Jeroen Noot|François TARONI|
Ing, Domenico Maiello|Caia Vincenzo|D.E. Blischke|Carmelina Fusco|Prete Fernanda|RUP   Prete Fernanda|Orri Jónsson|Julien LAMARRE|
Catarina Poço|Giacalone Maurizio|Daniel Esteves|Giorgio Monaci|Dario Canuto|Maurice Breg|Loredana Ruiu|Salvatore Tecleme|
Peter Horup|Ten. Col. Lorenzo Mancini|Elena Colombo|Giovanni Gangi|Emilia Marcovecchio|Nicola Ladu|K. Gaasbeek|
Domenichetti Maria Alessandra|Koos Maris|María Ángeles Carrero Martínez'''.replace('\n', '').split('|')


def hand_count():
    rows = [r for r in new_orgs() if r['name'].strip() in HAND_PERSONS]
    return len(rows), sum(r['n'] for r in rows), sorted({r['name'].strip() for r in rows}) == sorted(set(HAND_PERSONS))
