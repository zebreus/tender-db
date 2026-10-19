# 548 U1b: how often the change a held row would serve ADDS a consortium/role marker the head lacks (the shape of
# U1b's wrong 9168556 'Partner konsorcjum ...'). Listing-only, read-only. Per hold reason; T1 arm split out.
import json, gzip, re, collections
T = '/home/user/tender-db/.scratch/tender-db/548-triage/'
b = json.load(gzip.open(T + 'org-name-refresh-plan-2026-10-16.json.gz'))['body']; b = json.loads(b) if isinstance(b, str) else b
MARK = re.compile(r"\b(partner\w*|konsorc\w*|wsp[oó]lnik\w*|lider\w*|mandataire|cotraitant\w*|co-traitant\w*|groupement|chef de file|"
                  r"titulaire|arge|bietergemeinschaft\w*|consorzi\w*|consortium|consorcio|uni[oó]n temporal|ute|zajednica ponuditelja|"
                  r"(as )?consortium member|lead partner|in consortium)\b", re.I)
fresher = lambda r: r['rival_newest'] is not None and r['rival_newest'] > r['top_newest']
c = collections.Counter(); ex = collections.defaultdict(list)
for r in b['held']:
    if not r['new_name']: continue
    added = bool(MARK.search(r['new_name'])) and not MARK.search(r['old_name'])
    if not added: continue
    k = r['reason'] + (':T1_arm' if r['reason'] == 'top_stale' and not fresher(r) else ':T1_held' if r['reason'] == 'top_stale' else '')
    c[k] += 1; ex[k].append((r['org_id'], r['old_name'][:60], r['new_name'][:70]))
for k, v in c.most_common(): print(k, v, ex[k][:3])
arm = json.load(open(T + 'u1b/arm.json'))
print('T1 arm rows', len(arm), 'role-added', c['top_stale:T1_arm'])
json.dump(dict(counts=dict(c), examples={k: v[:10] for k, v in ex.items()}), open(T + 'u1b/role_added.json', 'w'), ensure_ascii=False, indent=1)
