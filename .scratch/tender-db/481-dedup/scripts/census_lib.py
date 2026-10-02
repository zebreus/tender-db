import json, re, collections
APR0, MAY0 = 1743465600, 1746057600
def shape(p):
    if re.match(r'^\d{8}-\d{4}$', p): return 'ted8'
    if re.match(r'^\d{6}-\d{4}$', p): return 'fts6'
    if re.match(r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}-\d+$', p): return 'doe-uuid'
    if re.match(r'^\d+-\d+$', p): return 'doe-num'
    return 'other'
def load():
    T = collections.defaultdict(dict)  # tender -> seq -> version dict
    for l in open('census_rows.jsonl'):
        mode, tid, seq, nid, pub, pat, sub = json.loads(l)
        T[tid][seq] = dict(seq=seq, notice_id=nid, publication_id=pub, published_at=pat, subtype=sub, shape=shape(pub), mode=mode)
    return T
