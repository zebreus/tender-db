# usage: python3 -I build_frozen.py — writes triage_frozen_lists.json, the two frozen, dated lists behind the class-A
# triage rule (decision.md rule 4; critic amendments B1 and S4). Keys are (org id, name_norm) with name_norm as
# org_name_norm (canonical.rs:7366): lowercase, whitespace collapsed, trailing . , ; stripped.
import json, os, importlib.util
D = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('m', D + '/measure_a.py'); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
def norm(s):
    c = ' '.join(s.lower().split()); t = c.rstrip('.,; ')
    return t or c
M = json.load(open(D + '/q4_class_refs.summary.json'))
TRANSFERRED = {46844727, 46844778, 46798381, 46776118, 46768416}
pa = {}
for r in M:
    if r['direction'] == 'P@A' and r['own_roles'] == ['OPT-300-Procedure-Buyer'] and r['notice'] not in TRANSFERRED:
        k = (r['org'], norm(r['name']))
        pa.setdefault(k, dict(org=r['org'], agent=r['agent'], name_norm=k[1], printed=r['name'], mentions=0, first_notice=r['notice']))
        pa[k]['mentions'] += 1
nonp = [dict(org=o, name_norm=norm(n), printed=n, kind={'V': "agent's own variant", 'U': 'host trust (Velindre): own procurements, P@A-shaped, destination unsettled (3 of 3 read)', 'T': 'not an organization name'}[c])
        for (o, n), c in m.OWN_CLASS.items()]
pairs = [  # (agent, holder org, holder, proof notice, what the read shows)
    ('NWSSP', 30919377, 'Aneurin Bevan UHB', 46816840, 'E4: FTS procuringEntity "acting on behalf of Aneurin Bevan University Health Board"'),
    ('NWSSP', 30926792, 'Cwm Taf Morgannwg UHB', 46784422, 'BT-24 "Cwm Taf Morgannwg University Health Board (CTMUHB) has awarded ..."'),
    ('NWSSP', 30926924, 'Cardiff and Vale UHB', 46742484, 'BT-24 "Cardiff and Vale University Health Board, acting on behalf of the All Wales Genomics Service ..."'),
    ('NWSSP', 30929723, 'Betsi Cadwaladr UHB', 46697948, 'BT-24 "Betsi Cadwaladr University Health Board is seeking ..."'),
    ('TMT', 30916821, 'Maidstone BC', 46750905, 'E10: BT-24 "Maidstone Borough Council (MBC) is looking to appoint ..."'),
    ('TMT', 30967593, 'Tunbridge Wells BC', 46789370, 'BT-24 "Tunbridge Wells Borough Council is looking to have the large Pond ... relined"'),
    ('GLEEDS', 30914436, 'College of Policing', 46693105, 'E11: CoP printed as buyer in its own section; Gleeds is procuringEntity "Specification preparation/ evaluation"'),
    ('SCAPE', 30974616, 'Arc Property Services Partnership', 46806701, 'BT-24 "SCAPE, on behalf of Arc Partnership (the Client) ..."'),
    ('GIBBS', 15069937, 'YMCA Saint Pauls Group (outside the slices)', 46801228, 'BT-21 "... as required by YMCA Saint Pauls Group"'),
    ('STAR', 30917275, 'Trafford Council (HOST)', 46817944, 'E6: Rochdale is buyer; Star (Trafford PPON) is procuringEntity "Managing the Tender"'),
    ('ARDAL', 30913763, 'Cardiff Council (HOST)', 46765017, 'E7: WLGA is buyer; Ardal (Cardiff PPON) is procuringEntity "the procurement service run by Cardiff Council"'),
]
not_frozen = [
    ('NWSSP', 30928312, 'Public Health Wales', 46823552, 'BT-24 names "NHS Wales Performance and Improvement", not Public Health Wales: not proven'),
    ('PALS', 30917832, 'Northern HSC Trust', 46762943, 'BT-24 "Patient information for consent leaflets": names no body, not proven'),
    ('NWSSP', 30913750, 'Swansea University', 46768416, 'a Swansea Bay UHB contract: F-shaped, to 545'),
    ('PAGABO', 31546270, 'CNTW NHS FT', 46844727, 'buying for The Education Alliance: F-shaped, to 545'),
]
out = dict(
    dated='2026-10-19', source='572-triage/decision.md rule 4 (critic amendments B1, S4)',
    list_a_reviewed_names_on_agent_accounts=dict(
        agent_orgs={'30953692': 'NHC', '30913738': 'NWSSP', '30913798': 'Welland', '30913722': 'Gibbs Laidler', '30930308': 'Invictus',
                    '30968739': 'Ginger', '30914820': 'RPJ3', '30916628': 'NHS London Commercial Hub'},
        agent_name_patterns={k: v['pat'] for k, v in m.AGENTS.items()},
        principal_keys=sorted(pa.values(), key=lambda r: (r['org'], r['name_norm'])),
        non_principal_names=nonp),
    list_b_proven_a_at_p_pairs=[dict(agent=a, holder_org=o, holder=h, proof_notice=n, read=w) for a, o, h, n, w in pairs],
    not_frozen_reviewed_as_usual=[dict(agent=a, holder_org=o, holder=h, notice=n, why=w) for a, o, h, n, w in not_frozen],
)
json.dump(out, open(D + '/triage_frozen_lists.json', 'w'), indent=1, ensure_ascii=False)
print('principal keys', len(pa), 'mentions', sum(r['mentions'] for r in pa.values()), '| non-principal names', len(nonp), '| proven pairs', len(pairs))
