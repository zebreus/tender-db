"""Adversarial re-derivation of B6 U2's provisional L_changed from the saved change pages (independent of run_u2.py).

python3 -I -B lchanged_adv.py OUT.json
Reads only: u2/api/manifest.json + u2/api/changes/*.json, u2/api/notices/*.json, b5/investigate/served_lsa_rows.json,
b6/l_changed_571_ids.json, b6/l_changed_545_ids.json, the b6 tender sidecars (scratchpad b6/tenders/*.meta.json) and the
gzip tender bodies (to list named notices independently of the port). Writes OUT.json only.
"""
import collections
import datetime
import gzip
import hashlib
import json
import os
import sys

U2 = '/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/u2'
B6 = os.path.dirname(U2)
ROOT = os.path.dirname(B6)
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
TEN = SP + '/b6/tenders'
SINCE = 958787061


def ep(s):
    return datetime.datetime.fromisoformat(s.replace('Z', '+00:00')).timestamp()


def main(out):
    rows = json.load(open(os.path.join(ROOT, 'b5/investigate/served_lsa_rows.json')))
    ids = sorted(int(r[0]) for r in rows if int(r[0]) != 179732)
    assert len(ids) == 3502
    S = set(ids)
    m = json.load(open(os.path.join(U2, 'api/manifest.json')))
    ev = []
    since = SINCE
    for rec in sorted(m['changes'], key=lambda r: r['i']):
        body = open(os.path.join(U2, 'api', rec['file']), 'rb').read()
        assert hashlib.sha256(body).hexdigest() == rec['sha256']
        assert rec['path'] == f'/v1/changes?since={since}&entity=tender&limit=1000'
        d = json.loads(body)
        assert d['ignored_filters'] == [], d['ignored_filters']
        ev += d['events']
        since = int(d['last_cursor'])
        assert int(d['events'][-1]['cursor']) <= since
        last_more = d['more']
    assert last_more is False
    curs = [int(e['cursor']) for e in ev]
    assert curs == sorted(curs) and len(set(curs)) == len(curs) and curs[0] > SINCE
    ents = collections.Counter(e['entity'] for e in ev)
    # job stamps (contiguous)
    jobs = []
    for e in ev:
        if not jobs or jobs[-1]['changed_at'] != e['changed_at']:
            jobs.append({'changed_at': e['changed_at'], 'lo': int(e['cursor']), 'hi': int(e['cursor']), 'ids': set(),
                         'ops': collections.Counter()})
        j = jobs[-1]
        j['hi'] = int(e['cursor'])
        j['ids'].add(e['id'])
        j['ops'][(e['op'], e['version'] is None)] += 1
    stamps = collections.Counter(e['changed_at'] for e in ev)
    assert len(stamps) == len(jobs), 'a job stamp recurs non-contiguously'
    jobs_out = [{'changed_at': j['changed_at'], 'cursor': [j['lo'], j['hi']], 'rows': sum(j['ops'].values()),
                 'ids': len(j['ids']), 'ids_in_3502': sorted(j['ids'] & S)[:20], 'n_ids_in_3502': len(j['ids'] & S),
                 'ops': {f'{a}/{"vnull" if b else "ver"}': n for (a, b), n in j['ops'].items()}} for j in jobs]
    by_id = collections.defaultdict(list)
    for e in ev:
        by_id[e['id']].append(e)
    # 2630 = the first stamp; is it exactly the 3,502?
    j0 = jobs[0]
    j2630 = [e for e in ev if e['changed_at'] == j0['changed_at']]
    # ranges
    r541 = sorted({e['id'] for e in ev if 958809778 < int(e['cursor']) <= 958809905})
    r541_stamps = sorted({e['changed_at'] for e in ev if 958809778 < int(e['cursor']) <= 958809905})
    s571 = sorted(set(json.load(open(os.path.join(B6, 'l_changed_571_ids.json')))))
    s545 = sorted(set(json.load(open(os.path.join(B6, 'l_changed_545_ids.json')))))
    feed571 = sorted({e['id'] for e in ev if e['changed_at'] == '2026-10-19T05:53:39Z'})
    feed545 = sorted({e['id'] for e in ev if e['changed_at'] == '2026-10-19T11:06:21Z'})
    # per-id fetch time and literal criterion (a)
    fetched = {}
    for t in ids:
        fetched[t] = json.load(open(os.path.join(TEN, f'{t}.meta.json')))['fetched_at']
    lit, op_excl2630, any_non2630_head = [], [], []
    for t in ids:
        lim = ep(fetched[t]) + (0.999 if '.' not in fetched[t] else 0)
        mine = by_id.get(t, [])
        if any(ep(e['changed_at']) <= lim for e in mine):
            lit.append(t)
        non = [e for e in mine if e['changed_at'] != j0['changed_at']]
        if any(ep(e['changed_at']) <= lim for e in non):
            op_excl2630.append(t)
        if non:
            any_non2630_head.append(t)
    # named notices, independently of the port: versions caused_by + lot_results/bids/contracts notice ids + anything
    # else that looks like a notice id key anywhere in the tender JSON
    named_all = set()
    named_keys = collections.Counter()

    def walk(o, t):
        if isinstance(o, dict):
            for k, v in o.items():
                if ('notice' in k) and isinstance(v, int):
                    named_all.add(v)
                    named_keys[k] += 1
                walk(v, t)
        elif isinstance(o, list):
            for v in o:
                walk(v, t)
    top_keys = collections.Counter()
    fetch_epochs = []
    for t in ids:
        with gzip.open(os.path.join(TEN, f'{t}.json.gz'), 'rt', encoding='utf-8') as fh:
            x = json.load(fh)
        assert int(x['id']) == t
        top_keys.update(x.keys())
        walk(x, t)
        fetch_epochs.append(fetched[t])
    probes = {int(k): v['ingested_at'] for k, v in m['notices'].items()}
    for k, v in m['notices'].items():
        body = open(os.path.join(U2, 'api', v['file']), 'rb').read()
        assert hashlib.sha256(body).hexdigest() == v['sha256']
        assert json.loads(body)['ingested_at'] == v['ingested_at']
    newest = max(named_all)
    fe = sorted(fetch_epochs)
    hist = collections.Counter(f[:13] for f in fe)
    res = {
        'pages': len(m['changes']), 'events': len(ev), 'entities': dict(ents), 'head': since,
        'jobs': jobs_out,
        'j2630': {'stamp': j0['changed_at'], 'rows': len(j2630), 'ids_eq_3502': {e['id'] for e in j2630} == S,
                  'one_row_per_id': len(j2630) == len({e['id'] for e in j2630}),
                  'cursor': [int(j2630[0]['cursor']), int(j2630[-1]['cursor'])],
                  'all_changed_vnull': all(e['op'] == 'changed' and e['version'] is None for e in j2630)},
        'r541': {'n': len(r541), 'stamps': r541_stamps, 'in_3502': sorted(set(r541) & S)},
        's571': {'n': len(s571), 'eq_feed_0553': s571 == feed571, 'in_3502': sorted(set(s571) & S)},
        's545': {'n': len(s545), 'eq_feed_1106': s545 == feed545, 'in_3502': sorted(set(s545) & S),
                 'feed_1106_n': len(feed545)},
        'literal_a_n': len(lit), 'operative_a_excl2630_n': len(op_excl2630),
        'ids_of_3502_with_any_non2630_row_to_head': any_non2630_head,
        'union_excl2630': None,
        'named_notices_n': len(named_all), 'named_keys': dict(named_keys), 'newest_named': newest,
        'probes': probes, 'newest_probed': newest in probes,
        'tender_top_keys': dict(top_keys),
        'fetch_range': [fe[0], fe[-1]], 'fetch_hist_by_hour': dict(sorted(hist.items())),
        'n_fetched_before_2630_end': sum(1 for f in fe if f < '2026-10-19T03:37'),
        'n_fetched_in_0730_0900': sum(1 for f in fe if '2026-10-19T07:30' <= f < '2026-10-19T09:00'),
    }
    u = set(op_excl2630) | set(r541) | set(s571) | set(s545)
    res['union_excl2630'] = {'n': len(u), 'in_3502': sorted(u & S),
                             'sha256': hashlib.sha256(''.join(f'{i}\n' for i in sorted(u)).encode()).hexdigest()}
    with open(out, 'w') as fh:
        json.dump(res, fh, indent=1, sort_keys=True, default=list)
    print(json.dumps({k: v for k, v in res.items() if k not in ('jobs', 'probes', 'tender_top_keys')}, indent=1,
                     default=list))
    for j in jobs_out:
        print(j['changed_at'], j['cursor'], j['rows'], j['ids'], j['n_ids_in_3502'], j['ops'])


if __name__ == '__main__':
    main(sys.argv[1])
