# Issue 572 unit 1 (a): size class "A" (buying agent / consortium <-> member or principal) from
# issue 545's judged candidates, both halves, plus an exhaustive per-agent read of 545's slice outputs.
#
# usage: python3 -I measure_a.py      (reads 545-triage files only; writes a_rows.json, a_agents.json,
#                                      a_exhaustive.json, a_stats.txt in this directory)
#
# Part 1 (detector-bound): every row 545 classed A — u2a/cand_judged.json (52), the unit-1 carried
#   Pagabo row (u2a/verdicts.json source=unit1-carried), u2b/tail_judged.json (9), u2b/cand2b_judged.json (3).
#   Rows = 545's spelling rows; keys = distinct (org, alt key) with keys.alt (the crosswalk port 545 used);
#   mentions = sum of n.
# Part 2 (exhaustive within the slices): for every agent in Part 1, every mention name on the agent's own
#   org(s) and every agent-named mention on any other org, over all 545 slice reads (u2a s1-s6 + unit 1 q3:
#   all 14,338 GBPPON-keyed orgs; u2b b1-b7 + unit 1 q8: all 31,486 GBCOH-keyed orgs), with no K cap and no
#   prefilter. This is what the detectors (K<=6, strays, prefilter) could not see.
import json, os, re, sys
from collections import defaultdict, Counter

T = '/home/user/tender-db/.scratch/tender-db/545-triage'
OUT = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, T + '/u2a')
from keys import alt  # noqa: E402  (545's port of crosswalk::altid_name_key)

# ---------- agents ----------
# kind: consortium / shared-service / framework-operator / broker / consultant
# own_orgs: the agent's own org rows (PPON/COH-keyed) as found in the slices (filled/checked below)
# pat: regex matched against a lower-cased mention name to say "this mention names the agent"
AGENTS = {
    'NHC':     dict(label='Northern Housing Consortium', kind='consortium (housing, member councils + HAs)',
                    pat=r'northern housing consortium|\bnhc\b'),
    'NWSSP':   dict(label='NHS Wales Shared Services Partnership (Procurement Services)', kind='shared-service (hosted by Velindre)',
                    pat=r'shared services? partnership|nwssp|partneriaeth cydwasanaethau'),
    'STAR':    dict(label='STAR Procurement', kind='shared-service (Stockport/Rochdale/Trafford/Tameside)',
                    pat=r'\bstar procurement\b|\bstar shared\b'),
    'ARDAL':   dict(label='Ardal', kind='shared-service (Cardiff-hosted, south-east Wales councils)',
                    pat=r'\bardal\b'),
    'WELLAND': dict(label='Welland Procurement', kind='shared-service (Rutland, Melton, Harborough, ...)',
                    pat=r'\bwelland\b'),
    'TMT':     dict(label='TMT Procurement Partnership', kind='shared-service (Tunbridge Wells/Maidstone/Tonbridge)',
                    pat=r'\btmt\b'),
    'PALS':    dict(label='BSO Procurement & Logistics Service (NI)', kind='shared-service (HSC)',
                    pat=r'procurement (and|&) logistics|\bpals\b|business services organisation'),
    'NHSLCH':  dict(label='NHS London Commercial Hub', kind='shared-service (London ICBs)',
                    pat=r'commercial hub'),
    'PAGABO':  dict(label='Pagabo', kind='framework-operator', pat=r'pagabo'),
    'SCAPE':   dict(label='SCAPE Group', kind='framework-operator', pat=r'\bscape\b'),
    'INVICTUS':dict(label='Invictus Partnership', kind='broker (schools/trusts)', pat=r'invictus'),
    'GINGER':  dict(label='Ginger Energy', kind='broker (energy, schools/trusts)', pat=r'ginger'),
    'RPJ3':    dict(label='RPJ3 Group', kind='broker (on behalf of trusts/schools)', pat=r'rpj3'),
    'GIBBS':   dict(label='Gibbs Laidler Consulting', kind='consultant (for housing associations)', pat=r'gibbs\s*laidler'),
    'GLEEDS':  dict(label='Gleeds Cost Management', kind='consultant (cost manager as co-buyer)', pat=r'gleeds'),
}

# Part 1 assignment: org -> (agent, direction). P@A = the principal's/member's name under the AGENT's
# identifier; A@P = the agent's name under a PRINCIPAL's identifier.
ASSIGN = {
    30953692: ('NHC', 'P@A'),        # Northern Housing Consortium Ltd, GBPPONPJXZ4423NDBT
    30913738: ('NWSSP', 'P@A'),      # NWSSP-Procurement Services, GBPPONPXWV6492CGMN (HEIW x6)
    30930308: ('INVICTUS', 'P@A'),   # INVICTUS PARTNERSHIP LTD (13647652)
    30968739: ('GINGER', 'P@A'),     # Ginger energy
    30914820: ('RPJ3', 'P@A'),       # RPJ3 Group Limited on Behalf of Aspirational ...
    30913798: ('WELLAND', 'P@A'),    # Welland Procurement
    30916628: ('NHSLCH', 'P@A'),     # NHS LONDON COMMERCIAL HUB
    30913722: ('GIBBS', 'P@A'),      # GIBBS LAIDLER CONSULTING LLP (GBCOHOC322760)
    30926792: ('NWSSP', 'A@P'),      # Cwm Taf Morgannwg UHB
    30928312: ('NWSSP', 'A@P'),      # Public Health Wales
    30929723: ('NWSSP', 'A@P'),      # Betsi Cadwaladr UHB
    30913750: ('NWSSP', 'A@P'),      # Swansea University
    30919377: ('NWSSP', 'A@P'),      # Aneurin Bevan UHB
    30917832: ('PALS', 'A@P'),       # Northern Health and Social Care Trust
    30917275: ('STAR', 'A@P'),       # Trafford Council
    30913763: ('ARDAL', 'A@P'),      # Cardiff Council
    30974616: ('SCAPE', 'A@P'),      # Arc Property Services Partnership
    30916821: ('TMT', 'A@P'),        # Maidstone BC
    30967593: ('TMT', 'A@P'),        # Tunbridge Wells BC
    30914436: ('GLEEDS', 'A@P'),     # College of Policing (GBCOH08235199)
    31546270: ('PAGABO', 'A@P'),     # Cumbria, Northumberland, Tyne and Wear NHS FT
}


# the agent's own org rows inside the slices (PPON/COH-keyed), hand-confirmed by head name
OWN = {
    'NHC': [30953692], 'NWSSP': [30913738], 'WELLAND': [30913798], 'PALS': [30914040, 30915904],
    'NHSLCH': [30916628], 'PAGABO': [31557760, 31648640], 'INVICTUS': [30930308],
    'GINGER': [30917636, 30968739, 31540891], 'RPJ3': [30914820, 31594875], 'GIBBS': [30913722],
    'GLEEDS': [30915130, 31615833],
    # STAR, ARDAL, TMT, SCAPE: no own PPON/COH-keyed org in the slices (keyed otherwise, or absent)
}
# names on an agent's own org that do NOT match its pattern and are not a principal:
#   V = the agent's own variant the pattern misses, U = the agent's host body, T = not an org name
OWN_CLASS = {
    (30953692, 'Stairlifts and Lifting Equipment Framework 2026'): 'T',
    (30913738, 'NHS Wales Shared Services and Partnership'): 'V',
    (30913738, 'NHS Wales Shared Services Partership'): 'V',
    (30913738, 'NHS Wales Shared Services'): 'V',
    (30913738, 'NHS Wales Shared Services (Hosted by Velindre University NHS Trust)'): 'V',
    (30913738, 'NHS Wales Shared Services Procurement Partnership'): 'V',
    (30913738, 'NHS Wales'): 'V',   # generic; NWSSP's own short form, not a principal
    (30913738, 'NHS'): 'T',
    (30913738, 'Velindre University NHS Trust'): 'U',   # NWSSP's legal host
    (30913738, 'Welsh Blood Service (Velindre University NHS Trust)'): 'U',
}


# orgs whose own name matches an agent pattern but which are neither the agent nor a principal
# (namesake companies, or the agent's group companies): left out of the 'agent named elsewhere' count
NAMESAKE = {30915474, 31621756,                      # Little Malvern and Welland PC; Welland House Care Centre
            31552499, 31545908,                      # TMT FIRST LIMITED; PALS LTD
            31584735, 31587101, 31573158, 30972060, 31968959, 31582484,   # other INVICTUS companies
            31609260, 31561726, 30974963,            # Ginger Nut Media; Gingerbread x2
            31595255, 31615808}                      # Gleeds Building Surveying; Gleeds Advisory (group cos)


def part1():
    rows = []
    for f, src in [('u2a/cand_judged.json', 'u2a'), ('u2b/tail_judged.json', '2a-tail'), ('u2b/cand2b_judged.json', '2b')]:
        for r in json.load(open(T + '/' + f)):
            if r['class'] == 'A':
                rows.append(dict(src=src, org=r['org'], ident=r['ident'], head=r['head'], name=r['name'],
                                 scheme=r['scheme'], n=r['n'], detector=r['detector'], org_mentions=r['org_mentions'],
                                 first_notice=r['first_notice'], last_notice=r['last_notice']))
    for r in json.load(open(T + '/u2a/verdicts.json')):
        if r['class'] == 'A' and r.get('source') == 'unit1-carried':
            rows.append(dict(src='unit1-carried', org=r['org'], ident=r['ident'], head=r['head'], name=r['name'],
                             scheme=r['scheme'], n=r['n'], detector=r['detector'], org_mentions=None,
                             first_notice=46844727, last_notice=None))  # verdicts.py:218 notices=[46844727]
    for r in rows:
        r['agent'], r['direction'] = ASSIGN[r['org']]
        r['key'] = alt(r['name'])
        r['half'] = 'GBCOH' if r['ident'].startswith('GBCOH') else 'GBPPON'
    return rows


def summarise(rows, by):
    out = defaultdict(lambda: {'rows': 0, 'keys': set(), 'mentions': 0, 'orgs': set()})
    for r in rows:
        k = by(r)
        o = out[k]; o['rows'] += 1; o['keys'].add((r['org'], r['key'])); o['mentions'] += r['n']; o['orgs'].add(r['org'])
    return {k: {'rows': v['rows'], 'keys': len(v['keys']), 'mentions': v['mentions'], 'orgs': len(v['orgs'])}
            for k, v in sorted(out.items(), key=lambda kv: -kv[1]['mentions'])}


# ---------- Part 2 ----------
SLICES = [T + '/u2a/s%d.out.json' % i for i in range(1, 7)] + [T + '/q3_slice.out.json'] + \
         [T + '/u2b/b%d.out.json' % i for i in range(1, 8)] + [T + '/q8_cohslice.out.json']


def load_slices():
    orgs = {}
    for p in SLICES:
        j = json.load(open(p))
        assert 'rows' in j and 'columns' in j and not j.get('truncated'), p
        for row in j['rows']:
            r = dict(zip(j['columns'], row))
            o = orgs.setdefault(r['id'], {'ident': r['identifier'], 'head': r['name'], 'names': Counter(),
                                          'first': {}, 'last': {}})
            nm = r['mname'] if r['mname'] is not None else ''
            o['names'][(nm, r['scheme'])] += r['n']
            o['first'][(nm, r['scheme'])] = min(o['first'].get((nm, r['scheme']), 10**12), r['first_notice'])
            if r.get('last_notice') is not None:
                o['last'][(nm, r['scheme'])] = max(o['last'].get((nm, r['scheme']), 0), r['last_notice'])
    return orgs


def part2(orgs):
    res = {}
    for a, spec in AGENTS.items():
        pat = re.compile(spec['pat'], re.I)
        # the agent's own org rows: hand-confirmed from the head names the pattern finds in the slices
        # (unrelated namesakes like 'WELLAND HOUSE CARE CENTRE', 'TMT FIRST', 'PALS LTD', 'Gingerbread ...'
        # and the other INVICTUS companies are excluded; see own_candidates in the output)
        own_candidates = sorted(oid for oid, o in orgs.items() if pat.search(o['head'] or ''))
        own = sorted(set(OWN.get(a, [])) | {oid for oid, (ag, d) in ASSIGN.items() if ag == a and d == 'P@A'})
        own_detail = []
        for oid in own:
            o = orgs.get(oid)
            if o is None:
                own_detail.append({'org': oid, 'in_slices': False}); continue
            agent_named = {f'{k[0]}|{k[1]}': v for k, v in o['names'].items() if pat.search(k[0])}
            other = {f'{k[0]}|{k[1]}': v for k, v in o['names'].items() if not pat.search(k[0])}
            cls = {k: OWN_CLASS.get((oid, k.rsplit('|', 1)[0].strip()), 'P') for k in other}
            principal = {k: v for k, v in other.items() if cls[k] == 'P'}
            own_detail.append({'org': oid, 'ident': o['ident'], 'head': o['head'], 'in_slices': True,
                               'mentions': sum(o['names'].values()),
                               'agent_named_mentions': sum(agent_named.values()),
                               'other_named_mentions': sum(other.values()),
                               'other_named_keys': len({alt(k.split('|')[0]) for k in other}),
                               'principal_named_mentions': sum(principal.values()),
                               'principal_named_keys': len({alt(k.rsplit('|', 1)[0]) for k in principal}),
                               'other_named_class': cls,
                               'agent_named': dict(sorted(agent_named.items(), key=lambda kv: -kv[1])),
                               'other_named': dict(sorted(other.items(), key=lambda kv: -kv[1]))})
        elsewhere = []
        for oid, o in orgs.items():
            if oid in own or oid in NAMESAKE:
                continue
            hits = {f'{k[0]}|{k[1]}': v for k, v in o['names'].items() if pat.search(k[0])}
            if hits:
                elsewhere.append({'org': oid, 'ident': o['ident'], 'head': o['head'], 'org_mentions': sum(o['names'].values()),
                                  'agent_named_mentions': sum(hits.values()),
                                  'agent_named': hits,
                                  'first': {f'{k[0]}|{k[1]}': o['first'][k] for k in o['names'] if pat.search(k[0])},
                                  'last': {f'{k[0]}|{k[1]}': o['last'].get(k) for k in o['names'] if pat.search(k[0])}})
        elsewhere.sort(key=lambda e: -e['agent_named_mentions'])
        res[a] = {'label': spec['label'], 'kind': spec['kind'], 'pattern': spec['pat'], 'own_orgs': own_detail,
                  'own_candidates': {oid: orgs[oid]['head'] for oid in own_candidates},
                  'elsewhere_orgs': len(elsewhere), 'elsewhere_mentions': sum(e['agent_named_mentions'] for e in elsewhere),
                  'elsewhere': elsewhere}
    return res


if __name__ == '__main__':
    rows = part1()
    rows.sort(key=lambda r: (r['agent'], r['org'], -r['n'], r['name']))
    json.dump(rows, open(OUT + '/a_rows.json', 'w'), indent=1, ensure_ascii=False)
    lines = []
    tot = summarise(rows, lambda r: 'ALL')['ALL']
    lines.append('PART 1 — 545 class A rows (detector-bound)')
    lines.append('total %s' % tot)
    for name, by in [('by source', lambda r: r['src']), ('by half', lambda r: r['half']),
                     ('by direction', lambda r: r['direction']), ('by agent', lambda r: r['agent']),
                     ('by agent+direction', lambda r: r['agent'] + ' ' + r['direction']),
                     ('by detector', lambda r: r['detector'])]:
        lines.append('-- ' + name)
        for k, v in summarise(rows, by).items():
            lines.append('   %-22s %s' % (k, v))
    orgs = load_slices()
    lines.append('')
    lines.append('slices loaded: %d orgs (%d GBPPON, %d GBCOH)' % (
        len(orgs), sum(1 for o in orgs.values() if o['ident'].startswith('GBPPON')),
        sum(1 for o in orgs.values() if o['ident'].startswith('GBCOH'))))
    ex = part2(orgs)
    json.dump(ex, open(OUT + '/a_exhaustive.json', 'w'), indent=1, ensure_ascii=False)
    lines.append('PART 2 — exhaustive within the slices (no K cap, no prefilter)')
    for a, v in ex.items():
        lines.append('== %s (%s)' % (a, v['label']))
        for od in v['own_orgs']:
            if not od['in_slices']:
                lines.append('   own org %s NOT IN SLICES' % od['org']); continue
            lines.append('   own org %s %s %r: %d mentions, agent-named %d, other-named %d (%d keys), of which principal-named %d (%d keys)' % (
                od['org'], od['ident'], od['head'], od['mentions'], od['agent_named_mentions'],
                od['other_named_mentions'], od['other_named_keys'], od['principal_named_mentions'], od['principal_named_keys']))
        lines.append('   agent-named mentions on %d other orgs: %d' % (v['elsewhere_orgs'], v['elsewhere_mentions']))
        for e in v['elsewhere'][:40]:
            lines.append('      %s %s %r: %s' % (e['org'], e['ident'], e['head'][:50], e['agent_named']))
    pa = {a: (sum(od.get('principal_named_mentions', 0) for od in v['own_orgs']),
              sum(od.get('principal_named_keys', 0) for od in v['own_orgs'])) for a, v in ex.items()}
    ap = {a: (v['elsewhere_mentions'], v['elsewhere_orgs']) for a, v in ex.items()}
    lines.append('')
    lines.append('PART 2 TOTALS (exhaustive within the slices)')
    lines.append('  P@A principal-named mentions on the agent\'s own org: %d mentions, %d keys  %s' % (
        sum(x[0] for x in pa.values()), sum(x[1] for x in pa.values()), {a: x for a, x in pa.items() if x[0]}))
    lines.append('  A@P agent-named mentions on principal orgs: %d mentions on %d orgs  %s' % (
        sum(x[0] for x in ap.values()), sum(x[1] for x in ap.values()), {a: x for a, x in ap.items() if x[0]}))
    open(OUT + '/a_stats.txt', 'w').write('\n'.join(lines) + '\n')
    print('\n'.join(lines))
