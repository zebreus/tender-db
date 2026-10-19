# Builds r11_dest.sql: destination lookups for the 2a-tail + 2b F keys (one name_norm range seek per prefix,
# same shape as u2a/gen_r8.py; name_norm = canonical.rs org_name_norm).
PREFIXES = [
 'royal college of psychiatrists', 'london borough of hounslow', 'hounslow', 'university of liverpool',
 'stockton on tees borough council', 'stockton-on-tees borough council', 'ballicom', 'uk centre for ecology',
 'centre for ecology', 'prosci', 'eastern counties scaf', 'pullman rail', 'tsi instruments', 'pwc',
 'pricewaterhousecoopers', 'capita resourcing', 'create partnership trust', 'county durham and darlington',
 'diabetes uk', 'hospital innovations', 'don whitley', 'london cactus', 'newcastle city council',
 'bedford borough council', 'autocross', 'bmbc', 'kingston upon hull city council', 'hull city council',
 'big life', 'monthind', 'london borough of haringey', 'haringey', 'accenture', 'tps fruit',
 'lincolnshire housing partnership', 'west yorkshire combined authority', 'action deafness',
 'land use consultants', 'jeremy benn', 'oteca', 'btme', 'medtree', 'wyre forest district council',
 'university of northumbria', 'northumbria university', 'ddm health', 'the bolton impact trust',
 'bolton impact trust', 'university of essex', 'generator power', 'dell corporation', 'ingeus', 'pulmonx',
 'crown paints', 'board of trustees of the royal botanic', 'royal botanic gardens', 'university of suffolk dental',
 'fcc waste', 'thomas bow',
]
def upper(p):
    return p[:-1] + chr(ord(p[-1]) + 1)
arms = []
for p in PREFIXES:
    assert "'" not in p
    s = (f"SELECT 'n:{p}' AS q, id, country, identifier, provisional, name, name_norm FROM organizations "
         f"WHERE name_norm >= '{p}' AND name_norm < '{upper(p)}'")
    if arms: s = s.replace(' AS q', '')
    arms.append(s)
open('r11_dest.sql', 'w').write('\nUNION ALL\n'.join(arms) + '\n')
print(len(arms), 'arms')
