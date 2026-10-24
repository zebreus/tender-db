"""gen_sql.py -- the 587 U1 statements (UNION ALL of per-org equality seeks), written to sql/*.sql."""
import json, os
HERE = '/home/user/tender-db/.scratch/tender-db/587-u1'
TOP15 = [1164742, 1171563, 1166638, 3443, 1181455, 1165758, 2997642, 1241271, 1223007, 1214290, 3975457, 1244992,
         1193769, 22506338, 1397643]
EXTRA = [9327844, 1369148, 2005009, 3801469, 19463077, 1416875, 19553]   # exhibit + one bare label per language
CONTROL = [1606217, 1182792, 5766, 1515541]                              # DEPT / OTHER_ORGAN controls
ORGS = TOP15 + EXTRA + CONTROL

def roles(orgs, cap=20000):
    arms = []
    for x in orgs:
        arms.append(
            f"SELECT {x} AS org, q.role AS role, q.sec AS sec, count(*) AS party_rows, count(DISTINCT q.tid) AS tenders, "
            f"count(DISTINCT q.nid) AS notices FROM (SELECT p.role AS role, substr(p.mention_section_id, 1, 3) AS sec, "
            f"p.tender_id AS tid, p.mention_notice_id AS nid FROM tender_version_parties p CROSS JOIN tenders t ON t.id = p.tender_id "
            f"WHERE p.organization_id = {x} AND p.seq = t.current_seq LIMIT {cap}) q GROUP BY q.role, q.sec")
    return ' UNION ALL '.join(arms)

def buyers(orgs, cap=200):
    arms = []
    for x in orgs:
        arms.append(
            f"SELECT {x} AS org, b.organization_id AS buyer, b.role AS brole, o.name AS bname, o.country AS bcountry, "
            f"o.identifier AS bident, count(DISTINCT q.tid) AS tenders FROM (SELECT DISTINCT p.tender_id AS tid, p.seq AS sq "
            f"FROM tender_version_parties p CROSS JOIN tenders t ON t.id = p.tender_id WHERE p.organization_id = {x} "
            f"AND p.seq = t.current_seq LIMIT {cap}) q CROSS JOIN tender_version_parties b ON b.tender_id = q.tid AND b.seq = q.sq "
            f"CROSS JOIN organizations o ON o.id = b.organization_id WHERE b.role IN ('Procedure-Buyer', 'buyer') "
            f"GROUP BY b.organization_id, b.role")
    return ' UNION ALL '.join(arms)

def sampled(orgs, cap=200):
    """How many distinct served Tenders the 200-cap sample holds per org (the denominator)."""
    arms = []
    for x in orgs:
        arms.append(
            f"SELECT {x} AS org, count(*) AS sampled FROM (SELECT DISTINCT p.tender_id AS tid, p.seq AS sq "
            f"FROM tender_version_parties p CROSS JOIN tenders t ON t.id = p.tender_id WHERE p.organization_id = {x} "
            f"AND p.seq = t.current_seq LIMIT {cap}) q")
    return ' UNION ALL '.join(arms)

def mentions(orgs):
    arms = []
    for x in orgs:
        arms.append(
            f"SELECT {x} AS org, substr(m.section_id, 1, 3) AS sec, count(*) AS n, count(DISTINCT m.notice_id) AS notices, "
            f"count(DISTINCT m.name) AS spellings, count(DISTINCT m.country) AS countries FROM organization_mentions m "
            f"WHERE m.organization_id = {x} GROUP BY substr(m.section_id, 1, 3)")
    return ' UNION ALL '.join(arms)

if __name__ == '__main__':
    os.makedirs(os.path.join(HERE, 'sql'), exist_ok=True)
    BIG = [1164742, 1171563, 1182792, 5766, 1166638]
    stm = {'s01a_roles': roles(BIG), 's01b_roles': roles([x for x in ORGS if x not in BIG]), 's02_buyers': buyers(ORGS) + ' UNION ALL ' + sampled(ORGS).replace(
        'count(*) AS sampled', "NULL AS buyer, 'SAMPLED' AS brole, NULL AS bname, NULL AS bcountry, NULL AS bident, count(*) AS tenders"),
        's03_mentions': mentions(ORGS)}
    for k, v in stm.items():
        open(os.path.join(HERE, 'sql', f'{k}.sql'), 'w').write(v + '\n')
        print(k, len(v))
    json.dump({'top15': TOP15, 'extra': EXTRA, 'control': CONTROL}, open(os.path.join(HERE, 'sql', 'orgs.json'), 'w'))
