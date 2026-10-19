# Builds r10_sec.sql: for every F row of the 2a-tail and 2b (tail_judged.json, cand2b_judged.json), EVERY
# mention of that exact name on that org (all notices, not just first/last: a HELD-3..6 key has up to 6), each
# with the org's party rows (j arms) and bid-party rows (k arms) bound to that (notice, section). The party
# tables are seeked on their organization_id index (a party row's org is its mention's org); the mention side
# on organization_mentions_org, filtered on +m.name. LEFT JOIN keeps mention-only rows (unbound mentions).
import json
rows = [r for f in ['tail_judged.json', 'cand2b_judged.json'] for r in json.load(open(f)) if r['class'] == 'F']
keys = sorted({(r['org'], r['name']) for r in rows})
arms = []
for i, (org, name) in enumerate(keys):
    q = name.replace("'", "''")
    sel = "SELECT '{t}|{org}|{i}' AS k, m.notice_id AS a, m.section_id AS b, m.name AS c, m.country AS d, p.tender_id AS e, p.seq AS f, p.{lot} AS g, p.role AS h"
    for t, tab, lot in (('j', 'tender_version_parties', 'lot_id'), ('k', 'tender_version_bid_parties', 'bid_id')):
        s = sel.format(t=t, org=org, i=i, lot=lot)
        if arms: s = s.replace(' AS k', '').replace(' AS a', '').replace(' AS b', '').replace(' AS c', '').replace(' AS d', '').replace(' AS e', '').replace(' AS f', '').replace(' AS g', '').replace(' AS h', '')
        arms.append(s + f" FROM organization_mentions m LEFT JOIN {tab} p ON p.organization_id = m.organization_id AND p.mention_notice_id = m.notice_id AND p.mention_section_id = m.section_id WHERE m.organization_id = {org} AND +m.name = '{q}'")
json.dump([[i, org, name] for i, (org, name) in enumerate(keys)], open('r10_keys.json', 'w'), ensure_ascii=False)
open('r10_sec.sql', 'w').write('\nUNION ALL\n'.join(arms) + '\n')
print(len(keys), 'F (org, name) keys', len(arms), 'arms', len(open('r10_sec.sql').read().encode()), 'bytes')
