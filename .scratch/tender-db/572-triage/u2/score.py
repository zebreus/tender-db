# usage: python3 -I score.py — scores R1 on the saved GET /v1/tenders/{id} bodies (u2/bodies/) with decision.md's rubric:
#   R: a party in the recorded role, organization_id = the recorded holder, mention_name byte-equal to the printed name;
#   N: the tender gained a version since 2026-10-19 and no longer carries the row (salted rows: replaced by the next
#      row of the stratum in salted order);
#   W: anything else (missing on an unchanged version, mention_name null or different, org different).
# The 3 v1-only exemplars are expected N: their class row is on an older version, so the current parties[] must not
# carry it. Writes r1.json.
import json, os, sys
D = os.path.dirname(os.path.abspath(__file__))
S = json.load(open(D + '/r1_sample.json'))
CUTOFF = '2026-10-19'
def body(t):
    f = D + '/bodies/v1_tenders_%d.json' % t
    return json.load(open(f)) if os.path.exists(f) else None
def score_row(r, b):
    ps = b['parties']
    hit = [p for p in ps if p['role'] == r['role'] and p['organization_id'] == r['org'] and p['mention_name'] == r['name']]
    vers = b.get('versions') or []
    new_vers = [v for v in vers if (v.get('published_at') or '') >= CUTOFF]
    cur_notice = max(vers, key=lambda v: v['seq'])['caused_by_notice_id'] if vers else None
    info = dict(version=b.get('version'), n_versions=len(vers), current_caused_by=cur_notice,
                versions_published_since_cutoff=[(v['seq'], v['published_at'], v['caused_by_notice_id']) for v in new_vers])
    if hit:
        p = hit[0]
        return 'R', dict(info, party=dict(role=p['role'], organization_id=p['organization_id'], organization_name=p['organization_name'], mention_name=p['mention_name'], lot=p.get('lot')))
    same_org = [p for p in ps if p['organization_id'] == r['org']]
    same_name = [p for p in ps if p['mention_name'] == r['name']]
    detail = dict(info, same_org_parties=same_org, same_name_parties=same_name)
    if new_vers:
        return 'N', detail
    return 'W', detail
def key(r): return dict(tender_id=r['tender_id'], cur=r['cur'], org=r['org'], role=r['role'], notice_id=r['notice_id'], section_id=r['section_id'], name=r['name'], direction=r['direction'], agent=r['agent'], new_since_q4=not r['in_q4'])
out = dict(exemplars=[], v1_only=[], salted=[], replacements=[], missing_bodies=[])
for t, e in S['exemplars'].items():
    t = int(t); b = body(t)
    if b is None: out['missing_bodies'].append(t); continue
    if e['kind'] == 'current':
        rows = [r for r in e['class_rows'] if r['cur'] == 1]
        for r in rows:
            s, d = score_row(r, b)
            out['exemplars'].append(dict(key(r), score=s, detail=d))
    else:
        for r in e['class_rows']:
            ps = b['parties']
            carried = [p for p in ps if p['role'] == r['role'] and p['organization_id'] == r['org'] and p['mention_name'] == r['name']]
            out['v1_only'].append(dict(key(r), score='N' if not carried else 'carried (not N)', expected='N',
                                       current_version=b.get('version'),
                                       current_buyer_parties=[p for p in ps if 'uyer' in p['role']]))
for stratum in ('A@P', 'P@A'):
    queue = list(S['replacement_queue'][stratum])
    for r in S['pick'][stratum]:
        b = body(r['tender_id'])
        if b is None: out['missing_bodies'].append(r['tender_id']); continue
        s, d = score_row(r, b)
        out['salted'].append(dict(key(r), stratum=stratum, salt=r['salt'], score=s, detail=d))
from collections import Counter
c = Counter(x['score'] for x in out['exemplars'] + out['salted'])
out['tally'] = dict(R=c.get('R', 0), N=c.get('N', 0), W=c.get('W', 0), v1_only=Counter(x['score'] for x in out['v1_only']))
json.dump(out, open(D + '/r1.json', 'w'), indent=1, ensure_ascii=False)
print('tally', out['tally'], 'missing bodies', len(out['missing_bodies']))
for x in out['exemplars']: print('EX', x['tender_id'], x['score'], x['org'], repr(x['name']), '->', (x['detail'].get('party') or {}).get('organization_name'))
for x in out['v1_only']: print('V1', x['tender_id'], x['notice_id'], x['score'], 'v%s' % x['current_version'], [(p['organization_id'], p['mention_name']) for p in x['current_buyer_parties']])
for x in out['salted']: print('S', x['stratum'], x['tender_id'], x['score'], x['org'], repr(x['name']))
for x in out['exemplars'] + out['salted']:
    if x['score'] != 'R': print('NON-R DETAIL', json.dumps(x, ensure_ascii=False)[:1500])
