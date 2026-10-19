# Issue 572 unit 1: assemble measure.json from the unit's own outputs (no requests).
# usage: python3 -I build_measure.py
# inputs: a_rows.json, a_exhaustive.json (measure_a.py), q1..q5 *.out.json (run.sh), q4_class_refs.summary.json
#         (analyze_q4.py), bodies/ (get.sh), fts/ (ftsget.sh), requests.log
import json, os, re, sys
from collections import Counter, defaultdict

D = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, '/home/user/tender-db/.scratch/tender-db/545-triage/u2a')
from keys import alt  # noqa: E402
import importlib.util  # noqa: E402
spec = importlib.util.spec_from_file_location('m', D + '/measure_a.py'); M = importlib.util.module_from_spec(spec); spec.loader.exec_module(M)


def load(p):
    return json.load(open(os.path.join(D, p)))


def rows_of(p):
    j = load(p)
    assert 'rows' in j and 'columns' in j and not j.get('truncated'), p
    return [dict(zip(j['columns'], r)) for r in j['rows']]


# ---------------- size ----------------
a_rows = load('a_rows.json')


def summ(rs, by):
    out = defaultdict(lambda: {'rows': 0, 'keys': set(), 'mentions': 0, 'orgs': set()})
    for r in rs:
        o = out[by(r)]; o['rows'] += 1; o['keys'].add((r['org'], r['key'])); o['mentions'] += r['n']; o['orgs'].add(r['org'])
    return {k: {'rows': v['rows'], 'keys': len(v['keys']), 'mentions': v['mentions'], 'orgs': len(v['orgs'])}
            for k, v in sorted(out.items(), key=lambda kv: -kv[1]['mentions'])}


part1 = {
    'what': "545's class-A rows: u2a/cand_judged.json (52), the unit-1 carried Pagabo row (u2a/verdicts.json), "
            "u2b/tail_judged.json (9), u2b/cand2b_judged.json (3). rows = 545 spelling rows, keys = distinct (org, keys.alt), "
            "mentions = sum n. Detector-bound: STRAY + HELD-2..6 + prefilter.",
    'total': summ(a_rows, lambda r: 'all')['all'],
    'by_source': summ(a_rows, lambda r: r['src']),
    'by_half': summ(a_rows, lambda r: r['half']),
    'by_direction': summ(a_rows, lambda r: r['direction']),
    'by_agent_direction': summ(a_rows, lambda r: r['agent'] + ' ' + r['direction']),
    'by_detector': summ(a_rows, lambda r: r['detector']),
}

q4 = load('q4_class_refs.summary.json')
k2 = defaultdict(set); m2 = Counter(); o2 = defaultdict(set); n2 = defaultdict(set)
for r in q4:
    g = r['agent'] + ' ' + r['direction']
    k2[g].add((r['org'], alt(r['name']))); m2[g] += 1; o2[g].add(r['org']); n2[g].add(r['notice'])
part2_by = {g: {'mentions': m2[g], 'keys': len(k2[g]), 'orgs': len(o2[g]), 'notices': len(n2[g])}
            for g in sorted(m2, key=lambda g: -m2[g])}
dir_tot = defaultdict(lambda: {'mentions': 0, 'keys': 0})
for g, v in part2_by.items():
    d = g.split()[1]; dir_tot[d]['mentions'] += v['mentions']; dir_tot[d]['keys'] += v['keys']
part2 = {
    'what': "Exhaustive within 545's slice reads (all 14,338 GBPPON + 31,486 GBCOH-keyed orgs) for the 15 agents of part 1, "
            "no K cap and no prefilter (measure_a.py part 2), re-read live with role references (q4_class_refs, analyze_q4.py). "
            "P@A = every non-agent, non-host, non-title name on the agent's own org; A@P = every agent-named mention on another org.",
    'total': {'mentions': len(q4), 'keys': sum(len(v) for v in k2.values()), 'notices': len({r['notice'] for r in q4})},
    'by_direction': dict(dir_tot),
    'by_agent_direction': part2_by,
    'beyond_part1': {
        'NHC': '11 member names / 16 mentions the prefilter dropped (shared token "housing": Solihull Community Housing x5, LIVIN x2, '
               'Arches, Believe, Blackpool Coastal, Housing 21, Islington & Shoreditch HA, L&Q, One Vision, South Lakes, Pioneer) '
               '(52 keys / 97 mentions against 545\'s 41 / 81)',
        'NWSSP P@A': 'Hywel Dda x12, Public Health Wales NHS Trust x9, Cardiff and Vale x10, Betsi x2, ABUHB x1, NWJCC x1 beyond 545\'s HEIW x6',
        'NWSSP A@P': 'Cardiff and Vale UHB x7 (held 7 > K=6)',
        'GIBBS': 'Hull Churches HA x2, Newlon Housing Trust x1 (prefilter: shared "housing" with English Rural HA)',
    },
}

# ---------------- agents ----------------
ex = load('a_exhaustive.json')
q3 = rows_of('q3_agent_orgs.out.json')
agents = []
for a, spec_ in M.AGENTS.items():
    own = [od for od in ex[a]['own_orgs'] if od.get('in_slices')]
    p1 = [r for r in a_rows if r['agent'] == a]
    p2 = {g: v for g, v in part2_by.items() if g.startswith(a + ' ')}
    host = sorted({r['org'] for r in q4 if r['agent'] == a and r['direction'] == 'A@P'})
    agents.append({
        'agent': spec_['label'], 'key': a, 'kind': spec_['kind'],
        'own_org_ids': [od['org'] for od in own],
        'own_orgs': [{'org': od['org'], 'identifier': od['ident'], 'head': od['head'], 'mentions': od['mentions'],
                      'agent_named': od['agent_named_mentions'], 'principal_named': od['principal_named_mentions'],
                      'principal_keys': od['principal_named_keys']} for od in own],
        'principal_orgs_holding_agent_name': host,
        'part1': summ(p1, lambda r: r['direction']) if p1 else {},
        'part2': p2,
    })

# ---------------- exemplars ----------------
EX = [  # (id, agent, direction, org, notice, tender)
    ('E1', 'NHC', 'P@A', 30953692, 46793765, 8641682), ('E2', 'NHC', 'P@A', 30953692, 46789522, 8636304),
    ('E3', 'NHC', 'P@A', 30953692, 46794092, 8677647), ('E4', 'NWSSP', 'A@P', 30919377, 46816840, 8654651),
    ('E5', 'NWSSP', 'P@A', 30913738, 46787268, 8698064), ('E6', 'STAR', 'A@P', 30917275, 46817944, 8655231),
    ('E7', 'ARDAL', 'A@P', 30913763, 46765017, 8621643), ('E8', 'WELLAND', 'P@A', 30913798, 46845134, 8677040),
    ('E9', 'GINGER', 'P@A', 30968739, 46737636, 8639202), ('E10', 'TMT', 'A@P', 30916821, 46750905, 8613154),
    ('E11', 'GLEEDS', 'A@P', 30914436, 46693105, 8580124), ('E12', 'GIBBS', 'P@A', 30913722, 46757368, 8596944),
    ('E13', 'RPJ3', 'P@A', 30914820, 46869370, 8687157),
]
q1 = rows_of('q1_exemplars.out.json')
orgheads = {}
for f in os.listdir(D + '/bodies'):
    if f.startswith('v1_organizations_') and f.endswith('.json'):
        o = json.load(open(D + '/bodies/' + f)); orgheads[o['id']] = o['name']
exemplars = []
for eid, a, d, org, notice, tid in EX:
    t = json.load(open(D + '/bodies/v1_tenders_%d.json' % tid))
    ver = [v for v in t['versions'] if v['caused_by_notice_id'] == notice][0]
    c = json.load(open(D + '/bodies/v1_notices_%d_content.json' % notice))
    roles = defaultdict(list)
    for s in c['sections']:
        for v in s['values']:
            if v['field_id'].startswith(('OPT-300', 'OPT-301')):
                roles[v.get('value')].append(v['field_id'].replace('OPT-300-', '').replace('OPT-301-', ''))
    bound = {r['b']: r['c'] for r in q1 if r['k'] == 'm|' + eid}
    secs = []
    for s in c['sections']:
        if s['kind'] != 'Organization':
            continue
        vals = defaultdict(list)
        for v in s['values']:
            vals[v['field_id']].append(v.get('value'))
        secs.append({'section': s['section_id'], 'name': (vals.get('BT-500-Organization-Company') or [None])[0],
                     'identifiers': vals.get('BT-501-Organization-Company', []), 'roles': roles.get(s['section_id'], []),
                     'bound_org': bound.get(s['section_id'])})
    raw = json.load(open(D + '/fts/%s.json' % ver['publication_id']))
    rel = raw['releases'][0]
    rparties = [{'id': p.get('id'), 'name': p.get('name'), 'roles': p.get('roles'),
                 'roleDetails': p.get('roleDetails'),
                 'UK_CA_TYPE': [x.get('id') for x in ((p.get('details') or {}).get('classifications') or [])]}
                for p in rel.get('parties', []) if set(p.get('roles') or []) & {'buyer', 'procuringEntity', 'centralPurchasingBody'}]
    desc = None
    for s in c['sections']:
        if s['section_id'] == 'PROCEDURE':
            for v in s['values']:
                if v['field_id'] == 'BT-24-Procedure':
                    desc = v.get('value')
    served = [p for p in t['parties'] if p['role'] == 'Procedure-Buyer']
    exemplars.append({
        'id': eid, 'agent': a, 'direction': d, 'notice_id': notice, 'publication_id': ver['publication_id'],
        'notice_subtype': ver['notice_subtype'], 'tender_id': tid, 'tender_version_of_notice': ver['seq'],
        'tender_current_version': t['version'], 'title': t['title'],
        'org_sections': secs, 'fts_source_buyer_side_parties': rparties,
        'source_has_procuringEntity': any('procuringEntity' in (p['roles'] or []) for p in rparties),
        'description_head': (desc or '')[:300],
        'served_procedure_buyers_current_version': served,
    })

# ---------------- role carried ----------------
nb = Counter(); nba = defaultdict(Counter)
for r in q4:
    k = '1 buyer section' if r['n_buyer_sections'] <= 1 else '2+ buyer sections'
    nb[k] += 1; nba[r['agent']][k] += 1
own_roles = Counter('+'.join(r['own_roles']) for r in q4)
q2 = rows_of('q2_roles.out.json')

# ---------------- requests ----------------
log = open(D + '/requests.log').read().splitlines()
req = {'sql': sum(' sql ' in l for l in log), 'api_get': sum(' api GET ' in l for l in log),
       'fts_get': sum(' fts GET ' in l for l in log), 'local_copy': sum('local-copy' in l for l in log),
       'non_200': [l for l in log if (' api GET ' in l or ' fts GET ' in l) and 'rc=200' not in l],
       'sql_nonzero_rc': [l for l in log if ' sql ' in l and 'rc=0 ' not in l]}

out = {
    'issue': 572, 'unit': '1 MEASURE', 'date': '2026-10-19',
    'requests': req,
    'size': {'part1_545_detector_bound': part1, 'part2_exhaustive_within_slices': part2},
    'agents': agents,
    'a_shaped_545_labelled_otherwise': [
        {'rows': 'University of Essex x2 + The Bolton Impact Trust x2 on EMBARK MULTI ACADEMY TRUST 31577532 (GBCOH11717748)',
         '545_label': 'F, PARK-shared-procurement (u2b/verdicts2b.json)',
         'evidence': "545-triage/u3/guards.md (b): notice 46851503 is Essex's own Energy Management Solutions Dynamic Market, "
                     "Dukefield Energy co-buyer 'the agent'; reviewer verdict 'stay PARKED (A or shared procurement)'; and a "
                     "different defect first: PYLY-4964-TXGH may be Essex's or Dukefield's PPON attached to Embark's org",
         'counted_in_A': False, 'mentions': 4},
        {'rows': 'Create Partnership Trust x1 on The Howard Partnership Trust 30917637 (GBCOH07597068)',
         '545_label': 'F, PARK-shared-procurement',
         'evidence': "545-triage/u3/guards.md (b): notice 46758153 prints Create PT with the holder's own number and broker "
                     "Ginger Energy (an A agent) as second Procedure-Buyer; reviewer: 'V-or-A, holder's own number'",
         'counted_in_A': False, 'mentions': 1},
    ],
    'role_carried': {
        'source_fts_exemplars': {'procuringEntity_party': sum(e['source_has_procuringEntity'] for e in exemplars),
                                 'buyer_only': sum(not e['source_has_procuringEntity'] for e in exemplars), 'read': len(exemplars)},
        'tender_db': "never distinct: crates/ingest/src/fts/parse.rs:249 maps roles 'buyer' AND 'procuringEntity' to "
                     "OPT-300-Procedure-Buyer; parties[].roleDetails is not read (Party struct) and parties[].details "
                     "(UK_CA_TYPE) is Ignored in checklist.rs:63. q2_roles: every class mention's party row is Procedure-Buyer "
                     "(except 2 Tenderer: Riverside on NHC, Hywel Dda on NWSSP).",
        'class_wide_proxy': {'what': 'Procedure-Buyer sections on the notice of each of the 191 part-2 mentions (q4_class_refs)',
                             'counts': dict(nb), 'by_agent': {a: dict(v) for a, v in nba.items()},
                             'own_role_of_the_A_mention': dict(own_roles)},
    },
    'served_today': {
        'rule': "issue 456 design 3: /v1/tenders/{id} parties[] serves organization_id + organization_name = the identifier "
                "holder's org and head, mention_name = the name the notice printed; parties[] is the CURRENT version's",
        'class_wide_current_version': load('q6_current.summary.json'),
        'principal_orgs_exist': {r['name']: [(x['id'], x['identifier'], x['provisional']) for x in rows_of('q5_principal_orgs.out.json') if x['q'] == r['q']]
                                 for r in [{'q': i + 1, 'name': n} for i, n in enumerate(
                                     ['birmingham city council', 'darlington borough council', 'karbon homes', 'karbon homes limited', '54north homes',
                                      'health education and improvement wales', 'health education and improvement wales (heiw)', 'rutland county council',
                                      'the howard partnership trust', 'english rural housing association', 'remarkable autism',
                                      'aneurin bevan university health board', 'welsh local government association', 'hywel dda university health board',
                                      'solihull community housing limited', 'the education alliance'])]},
    },
    'a_at_p_subtypes': {
        'own_buyer': {'mentions': 33, 'what': "the agent's name on the actual buyer's identifier: the bound org IS the buyer; only the printed name is the agent's",
                      'rows': {'NWSSP on health boards/PHW/Swansea Univ': 15, 'TMT on Maidstone/Tunbridge Wells': 11, 'Gleeds on College of Policing': 4,
                               'Scape on Arc Property Services': 2, 'PaLS on Northern HSC Trust': 1},
                      'read': 'E4 (ABUHB), E10 (Maidstone), E11 (CoP) read; the rest presumed from the same agent/holder pairs',
                      'caveat': 'NWSSP notice 46819472 crosses two names over two PPONs (ABUHB name on NWSSP PPON and v.v.)'},
        'host': {'mentions': 3, 'what': "the agent's name on its HOST council's identifier while buying for another member: the host is served as buyer",
                 'rows': {'Star Procurement on Trafford (for Rochdale)': 2, 'Ardal on Cardiff (for WLGA)': 1}, 'read': 'E6, E7 (FTS roleDetails: "Ardal is ... the procurement service run by Cardiff Council")'},
        'unrelated': {'mentions': 2, 'what': "the agent's name on an unrelated body's identifier (545-F-shaped)",
                      'rows': {'Pagabo on Cumbria, Northumberland, Tyne and Wear NHS FT (buying for The Education Alliance)': 2}, 'read': 'q4 refs only'},
    },
    'exemplars': exemplars,
}
json.dump(out, open(D + '/measure.json', 'w'), indent=1, ensure_ascii=False)
print('wrote measure.json:', len(exemplars), 'exemplars;', out['requests'])
print('part1', part1['total'], 'part2', part2['total'], dict(dir_tot))
print('role proxy', dict(nb), 'source PE', out['role_carried']['source_fts_exemplars'])
