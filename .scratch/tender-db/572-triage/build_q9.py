# usage: python3 -I build_q9.py a|b — writes q9a_principal_orgs.sql (a) or q9b_routed.sql (b; needs q9a's output).
# Critic amendment B2 (+ S1). For every P@A principal printed on a CURRENT class row (q6_current), find every
# identified org that could be its own record — exact name_norm seeks on the printed name and a few plausible
# spellings, plus the ids already known from q5 / q4 / measure_a — and measure:
#   q9a  own           = count(DISTINCT tender) where the org is a buyer on the CURRENT version: the predicate of
#                        `/v1/tenders?buyer=<org>` (read.rs:1300-1305: p.seq = current, role LIKE '%Buyer%');
#                        plus S1's two arms, Birmingham's mention counts on 30914707 (PPON) and 1291257 (provisional).
#   q9b  routed        = count(DISTINCT tender) where the AGENT org is a buyer on the current version and the
#                        anchoring mention printed the principal's name (q6's numerator, by tender);
#        routed_absent = the routed tenders on which NONE of the principal's identified orgs is also a current buyer.
# Identified only: `+o.identifier IS NOT NULL` (the unary + keeps the planner on the name_norm index).
# /v1/sql takes at most 64 KiB of body (sql.rs MAX_BODY) and 15 s, so the read is split in two.
import json, os, sys
D = os.path.dirname(os.path.abspath(__file__))
NHC, NWSSP, WELLAND, NHSLCH, INVICTUS, GINGER, RPJ3, GIBBS = 30953692, 30913738, 30913798, 30916628, 30930308, 30968739, 30914820, 30913722
# (label, agent, [printed variants], [name_norm lookups], [explicit ids])
P = [
 ('Arches HA', NHC, ['ARCHES HOUSING LIMITED'], ['arches housing limited', 'arches housing'], []),
 ('Believe Housing', NHC, ['BELIEVE HOUSING LIMITED'], ['believe housing limited', 'believe housing'], []),
 ('Blackpool Coastal Housing', NHC, ['BLACKPOOL COASTAL HOUSING LIMITED'], ['blackpool coastal housing limited', 'blackpool coastal housing'], []),
 ('Bernicia', NHC, ['Bernicia Group'], ['bernicia group', 'bernicia group limited', 'bernicia homes', 'bernicia'], []),
 ('Birmingham CC', NHC, ['Birmingham City Council'], ['birmingham city council'], [30914707]),
 ('Bristol CC', NHC, ['Bristol City Council'], ['bristol city council'], []),
 ('Carmarthenshire CC', NHC, ['Carmarthenshire County Council'], ['carmarthenshire county council'], []),
 ('City of Lincoln', NHC, ['City of Lincoln Council'], ['city of lincoln council'], []),
 ('City of York', NHC, ['City of York Council'], ['city of york council'], []),
 ('Darlington BC', NHC, ['Darlington Borough Council'], ['darlington borough council'], [30914393]),
 ('East Riding', NHC, ['East Riding of Yorkshire Council'], ['east riding of yorkshire council'], []),
 ('East Suffolk', NHC, ['East Suffolk Council'], ['east suffolk council'], []),
 ('Gateshead', NHC, ['Gateshead Borough Council'], ['gateshead borough council', 'gateshead council', 'gateshead metropolitan borough council'], []),
 ('Housing 21', NHC, ['HOUSING 21'], ['housing 21'], []),
 ('High Peak BC', NHC, ['High Peak Borough Council'], ['high peak borough council'], []),
 ('Islington & Shoreditch HA', NHC, ['ISLINGTON & SHOREDITCH HOUSING ASSOCIATION LIMITED'], ['islington & shoreditch housing association limited', 'islington and shoreditch housing association limited', 'islington & shoreditch housing association'], []),
 ('Karbon Homes', NHC, ['Karbon Homes (54North Homes)'], ['karbon homes'], [30915057, 30913799, 31599199, 31619955]),
 ('LIVIN Housing', NHC, ['LIVIN HOUSING LIMITED'], ['livin housing limited', 'livin housing'], []),
 ('L&Q', NHC, ['LONDON & QUADRANT HOUSING TRUST'], ['london & quadrant housing trust', 'london & quadrant housing trust limited', 'london and quadrant housing trust'], []),
 ('Lancaster CC', NHC, ['Lancaster City Council'], ['lancaster city council'], []),
 ('Leeds CC', NHC, ['Leeds City Council'], ['leeds city council'], []),
 ('Newham', NHC, ['London Borough of Newham'], ['london borough of newham', 'newham council'], []),
 ('Bury', NHC, ['Metropolitan borough of Bury'], ['metropolitan borough of bury', 'bury metropolitan borough council', 'bury council'], []),
 ('Moat Homes', NHC, ['Moat Homes Limited'], ['moat homes limited', 'moat homes'], []),
 ('Newark & Sherwood', NHC, ['Newark And Sherwood District Council'], ['newark and sherwood district council', 'newark & sherwood district council'], []),
 ('Newcastle CC', NHC, ['Newcastle City Council'], ['newcastle city council', 'newcastle upon tyne city council'], []),
 ('North Northamptonshire', NHC, ['North Northamptonshire Council'], ['north northamptonshire council'], []),
 ('North Tyneside', NHC, ['North Tyneside Borough Council'], ['north tyneside borough council', 'north tyneside council'], []),
 ('One Manchester', NHC, ['ONE MANCHESTER LIMITED'], ['one manchester limited', 'one manchester'], []),
 ('One Vision Housing', NHC, ['ONE VISION HOUSING LIMITED'], ['one vision housing limited', 'one vision housing'], []),
 ('Onward Homes', NHC, ['ONWARD HOMES LIMITED'], ['onward homes limited', 'onward homes'], []),
 ('Orbit Group', NHC, ['ORBIT GROUP LIMITED'], ['orbit group limited', 'orbit group'], []),
 ('Ongo Homes', NHC, ['Ongo Homes Limited'], ['ongo homes limited', 'ongo homes'], []),
 ('Powys CC', NHC, ['Powys County Council'], ['powys county council'], []),
 ('Rochdale', NHC, ['Rochdale Borough Council'], ['rochdale borough council', 'rochdale metropolitan borough council'], [30916230]),
 ('RBKC', NHC, ['Royal Borough of Kensington and Chelsea'], ['royal borough of kensington and chelsea'], []),
 ('St Leger Homes', NHC, ['ST LEGER HOMES OF DONCASTER LIMITED'], ['st leger homes of doncaster limited', 'st leger homes of doncaster'], []),
 ('Sandwell MBC', NHC, ['Sandwell Metropolitan Borough Council'], ['sandwell metropolitan borough council'], []),
 ('Solihull CH', NHC, ['Solihull Community Housing Limited'], ['solihull community housing limited'], [5539040]),
 ('South Derbyshire DC', NHC, ['South Derbyshire District Council'], ['south derbyshire district council'], []),
 ('South Lakes Housing', NHC, ['South Lakes Housing'], ['south lakes housing', 'south lakes housing limited'], []),
 ('South Tyneside', NHC, ['South Tyneside Borough Council'], ['south tyneside borough council', 'south tyneside council'], []),
 ('Stockton-on-Tees', NHC, ['Stockton on Tees Borough Council'], ['stockton on tees borough council', 'stockton-on-tees borough council'], []),
 ('Stoke-on-Trent', NHC, ['Stoke on Trent City Council'], ['stoke on trent city council', 'stoke-on-trent city council'], []),
 ('Guinness Partnership', NHC, ['THE GUINNESS PARTNERSHIP LIMITED'], ['the guinness partnership limited', 'the guinness partnership'], []),
 ('Tamworth BC', NHC, ['Tamworth Borough Council'], ['tamworth borough council'], []),
 ('Kirklees', NHC, ['The Council of the Borough of Kirklees'], ['the council of the borough of kirklees', 'kirklees council'], []),
 ('Pioneer Housing', NHC, ['The Pioneer Housing and Community Group Limited'], ['the pioneer housing and community group limited', 'pioneer housing and community group limited'], []),
 ('Thurrock', NHC, ['Thurrock Council'], ['thurrock council'], []),
 ('Warwick DC', NHC, ['Warwick District Council'], ['warwick district council'], []),
 ('Wigan', NHC, ['Wigan Council'], ['wigan council', 'wigan metropolitan borough council'], []),
 ('ABUHB', NWSSP, ['Aneurin Bevan University Bevan Health Board'], ['aneurin bevan university health board'], [30919377, 30914825]),
 ('Betsi Cadwaladr UHB', NWSSP, ['Betsi Cadwaladr University Health Board'], ['betsi cadwaladr university health board'], [30929723]),
 ('Cardiff and Vale UHB', NWSSP, ['Cardiff and Vale University Health Board', 'Cardiff and Vale University Health Board\t', 'Cardiff & Vale University Health Board'], ['cardiff and vale university health board'], [30926924]),
 ('HEIW', NWSSP, ['Health Education and Improvement Wales (HEIW)'], ['health education and improvement wales (heiw)', 'health education and improvement wales'], [30917293, 31539418]),
 ('Hywel Dda UHB', NWSSP, ['Hywel Dda University Health Board', 'Hywel Dda University Health Board ', 'Hywel Dda University Health Board  '], ['hywel dda university health board'], [30926960, 31544733]),
 ('NWJCC', NWSSP, ['NHS Wales Joint Commissioning Committee (NWJCC) hosted by Cwm Taf Morgannwg University Health Board '], ['nhs wales joint commissioning committee', 'nhs wales joint commissioning committee (nwjcc)'], []),
 ('Public Health Wales', NWSSP, ['Public Health Wales NHS Trust'], ['public health wales nhs trust', 'public health wales'], [30928312]),
 ('Rutland CC', WELLAND, ['Rutland County Council'], ['rutland county council'], [30915335]),
 ('NCL ICB', NHSLCH, ['NHS NORTH CENTRAL LONDON INTEGRATED CARE BOARD'], ['nhs north central london integrated care board', 'nhs north central london icb', 'north central london integrated care board'], []),
 ('CORE Education Trust', INVICTUS, ['CORE Education Trust'], ['core education trust'], []),
 ('William Farr School', INVICTUS, ['William Farr C of E Comprehensive School'], ['william farr c of e comprehensive school', 'william farr school', 'william farr church of england comprehensive school'], []),
 ('Howard PT', GINGER, ['The Howard Partnership Trust'], ['the howard partnership trust'], [30917637]),
 ('Keys Academy Trust', GINGER, ['The Keys Academy Trust'], ['the keys academy trust', 'keys academy trust'], []),
 ('Remarkable Autism', RPJ3, ['Remarkable Autism'], ['remarkable autism'], []),
 ('English Rural HA', GIBBS, ['English Rural Housing Association'], ['english rural housing association'], []),
 ('Hull Churches HA', GIBBS, ['Hull Churches Housing Association'], ['hull churches housing association', 'hull churches housing association limited'], []),
 ('Newlon HT', GIBBS, ['Newlon Housing Trust'], ['newlon housing trust', 'newlon housing trust limited'], []),
 ('YMCA Saint Pauls', GIBBS, ['YMCA Saint Pauls Group'], ['ymca saint pauls group', 'ymca st pauls group'], [15069937]),
]
def q(s): return "'" + s.replace("'", "''") + "'"
OWN = ("(SELECT count(DISTINCT p.tender_id) FROM tender_version_parties p JOIN tenders t ON t.id=p.tender_id"
       " WHERE p.organization_id=o.id AND p.seq=t.current_seq AND p.role LIKE '%Buyer%')")
mode = sys.argv[1]
if mode == 'a':
    norms = sorted({n for _, _, _, ns, _ in P for n in ns})
    ids = sorted({i for _, _, _, _, ii in P for i in ii} | {NHC})
    arms = [
        f"SELECT 'norm' AS k, o.id, o.identifier, o.name, o.name_norm, {OWN} AS own FROM organizations o"
        f" WHERE o.name_norm IN ({', '.join(q(n) for n in norms)}) AND +o.identifier IS NOT NULL",
        f"SELECT 'id' AS k, o.id, o.identifier, o.name, o.name_norm, {OWN} AS own FROM organizations o"
        f" WHERE o.id IN ({', '.join(map(str, ids))})",
        # S1: Birmingham's split, mentions per org (single-table aggregate over 2 keys)
        "SELECT 'S1 mentions' AS k, o.id, o.identifier, o.name, o.name_norm,"
        " (SELECT count(*) FROM organization_mentions m WHERE m.organization_id=o.id) AS own"
        " FROM organizations o WHERE o.id IN (30914707, 1291257)",
    ]
    sql = '\nUNION ALL\n'.join(arms) + '\n'
    open(D + '/q9a_principal_orgs.sql', 'w').write(sql)
    print(len(norms), 'name_norms,', len(ids), 'ids,', len(sql), 'bytes')
else:
    j = json.load(open(D + '/q9a_principal_orgs.out.json'))
    rows = [dict(zip(j['columns'], r)) for r in j['rows']]
    found = {}
    for label, agent, names, norms, ids in P:
        s = {r['id'] for r in rows if r['k'] in ('norm', 'id') and (r['name_norm'] in norms or r['id'] in ids)}
        found[label] = sorted(s)
    arms = []
    for label, agent, names, norms, ids in P:
        orgs = found[label] or [-1]   # -1: no identified row; then every routed tender is absent
        inn = ', '.join(q(n) for n in names)
        arms.append(
            f"SELECT {q(label)} AS principal, count(DISTINCT p.tender_id) AS routed,"
            f" count(DISTINCT CASE WHEN NOT EXISTS (SELECT 1 FROM tender_version_parties x WHERE x.tender_id=p.tender_id"
            f" AND x.seq=p.seq AND x.organization_id IN ({', '.join(map(str, orgs))}) AND x.role LIKE '%Buyer%') THEN p.tender_id END) AS routed_absent"
            f" FROM tender_version_parties p JOIN tenders t ON t.id=p.tender_id"
            f" JOIN organization_mentions m ON m.notice_id=p.mention_notice_id AND m.section_id=p.mention_section_id"
            f" WHERE p.organization_id={agent} AND p.seq=t.current_seq AND p.role LIKE '%Buyer%' AND m.name IN ({inn})")
    sql = '\nUNION ALL\n'.join(arms) + '\n'
    open(D + '/q9b_routed.sql', 'w').write(sql)
    json.dump(found, open(D + '/q9b_found_orgs.json', 'w'), indent=1)
    print(len(arms), 'arms,', len(sql), 'bytes')
json.dump([dict(label=l, agent=a, printed=n, norms=no, ids=i) for l, a, n, no, i in P],
          open(D + '/q9_principal_spec.json', 'w'), indent=1, ensure_ascii=False)
