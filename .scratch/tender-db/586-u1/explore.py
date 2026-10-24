import sys, re, collections
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/586-u1')
from pop import all_pops, fold
pops = all_pops()
probes = {
 'honorific-any': re.compile(r'(?i)(^|[\s,;/(])(monsieur|madame|mademoiselle|mme|mlle|m\.|dott\.?(ssa)?|sig\.?(ra)?|herr|frau|mr\.?|mrs\.?|ms\.?|ing\.|dr\.?|dra\.?|sr\.?|sra\.?|d\.|dª|doña|don|pan|pani|mgr|ir\.?|dhr\.?|mevr\.?|mw\.?)\s'),
 'art': re.compile(r'(?i)\b(art\.?|art[ií]culo|artikel|articolo|article)\s*\d'),
 'prose': re.compile(r'(?i)(procede|ver apartado|en los lugares|apartado|recurso especial|seg[uú]n|conforme|ante el|véase|vease|consultar|indicad|establecid)'),
 'platform': re.compile(r'(?i)(tutto ?gare|plataforma|platforma|platform|e-?zam[oó]wieni|logintrade|achizitii|achiziții|sede electr|perfil del contratante|portal)'),
 'label-words': re.compile(r'(?i)^(direcci[oó¿]n|direccion|sede|info|información|informacion|contact|kontakt|recursos?|palacio|edificio|principal|postal|oficina|registro|presentaci[oó]n|central|procurement|ayuntamiento|ajuntament|concello)\b'),
}
for k, rx in probes.items():
    print('=====', k)
    seen = collections.defaultdict(lambda: [0, set()])
    for p in pops:
        if rx.search(p['name']):
            s = seen[p['name']]; s[0] += p['n'] or 0; s[1].add(p['pop'])
    for name, (n, ps) in sorted(seen.items(), key=lambda kv: -kv[1][0]):
        print(f'  {n:5d} {",".join(sorted(ps)):18s} {name[:110]!r}')
