# Builds r9_refs.sql: (1) mention counts of every destination row tied in verdicts.py's pools (the
# most-referenced tiebreak) — one organization_mentions_org range per id, single-table aggregate;
# (2) the sections and roles of unit 1's two twice-held F keys (WAVEMAKER x2 on MoJ, Charlton Park
# Academy x2 on HM Land Registry), which unit 1 never read: mentions by the org index filtered on the
# name, CROSS JOIN parties on the org index; (3) name_norm lookups for their destinations.
import json
need = sorted({i for v in json.load(open('verdicts.json')) for i in v.get('needs_refcount', [])})
arms = [f"SELECT 'rc|{i}' AS k, count(*) AS a, NULL AS b, NULL AS c, NULL AS d, NULL AS e, NULL AS f, NULL AS g, NULL AS h FROM organization_mentions WHERE organization_id = {i}" for i in need]
for org, name in [(30913753, 'WAVEMAKER LIMITED t/a WPP 650'), (30953754, 'Charlton Park Academy')]:
    arms.append(f"SELECT 'j|{org}' AS k, m.notice_id AS a, m.section_id AS b, m.name AS c, m.country AS d, p.tender_id AS e, p.seq AS f, p.lot_id AS g, p.role AS h FROM organization_mentions m LEFT JOIN tender_version_parties p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = {org} AND +m.name = '{name}'")
for p in ['wavemaker', 'charlton park academy']:
    up = p[:-1] + chr(ord(p[-1]) + 1)
    arms.append(f"SELECT 'n:{p}' AS k, id AS a, country AS b, identifier AS c, provisional AS d, name AS e, NULL AS f, NULL AS g, NULL AS h FROM organizations WHERE name_norm >= '{p}' AND name_norm < '{up}'")
open('r9_refs.sql', 'w').write('\nUNION ALL\n'.join(arms) + '\n')
print(len(need), 'refcounts', len(arms), 'arms')
