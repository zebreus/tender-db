# Builds r7_sec.sql: for every F candidate mention (and 456's exhibit), the org's mention rows on that
# notice (section, name, raw identifier) and every party / bid-party row the org holds from that notice
# (role, tender, lot, section). Each arm is a seek: mentions by PK (notice_id) with +organization_id,
# parties and bid parties by their organization_id index.
import json
rows = [r for r in json.load(open('cand_judged.json')) if r['class'] == 'F']
pairs = set()
for r in rows:
    for n in {r['first_notice'], r['last_notice']}:
        assert n is not None
        pairs.add((r['org'], n))
pairs.add((5718658, 46966800))  # issue 456: Schneider Electric on Sellafield's PPON, tender 8751605
arms = []
for org, n in sorted(pairs):
    arms.append(f"SELECT 'm|{org}|{n}' AS k, notice_id AS a, section_id AS b, organization_id AS c, name AS d, country AS e, raw_identifier AS f, scheme AS g FROM organization_mentions WHERE notice_id = {n} AND +organization_id = {org}")
    arms.append(f"SELECT 'p|{org}|{n}' AS k, tender_id AS a, seq AS b, lot_id AS c, role AS d, mention_notice_id AS e, mention_section_id AS f, organization_id AS g FROM tender_version_parties WHERE organization_id = {org} AND mention_notice_id = {n}")
    arms.append(f"SELECT 'b|{org}|{n}' AS k, tender_id AS a, seq AS b, bid_id AS c, role AS d, mention_notice_id AS e, mention_section_id AS f, organization_id AS g FROM tender_version_bid_parties WHERE organization_id = {org} AND mention_notice_id = {n}")
open('r7_sec.sql', 'w').write('\nUNION ALL\n'.join(arms) + '\n')
print(len(pairs), 'org-notice pairs', len(arms), 'arms')
