#!/usr/bin/env python3
"""519 M7 reads: the notices behind 13T's refused names and behind every phase-A rehome candidate M1 lists.

For each target mention (holder, notice, section) three public GETs, each cached under m7-raw/ and paced 0.5 s:
1. https://tenders.zebreus.click/v1/notices/<id>          (metadata: publication_id, published_at)
2. https://tenders.zebreus.click/v1/notices/<id>/content  (the stored parse: Organization sections, OPT-300/301 roles)
3. https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/<publication_id>
                                                           (the published OCDS: party roles, contact, address, buyer)
Targets:
- 13T (31600907): every mention whose name M1 refuses under QHM (15 CSU-named + 1 NHS England), plus a
  comparison sample of NENC-named 13T mentions (3 oldest, 3 newest, and 8 inside the CSU-named window
  2024-04-16..2024-05-16).
- Every refused mention on the other phase-A holders (A1m / A1 with a PPON survivor: 26A, 15E, 15C, 15N, 93C)
  and on the RO198 sites whose trust has a PPON survivor (RTR54, X7V4Q, RNZ91).
- 11X's one party that co-publishes a PPON (513 census `ppons` 1): its "Rural Practice Network" mention.
Stops at the first non-2xx / non-JSON answer (never retries).
"""
import datetime, json, os, sys, time, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, 'm7-raw')
HOLDERS = {'13T', '26A', '15E', '15C', '15N', '93C', 'RTR54', 'X7V4Q', 'RNZ91', '11X'}
ONLY_NAMES = {'11X': {'Rural Practice Network'}}


def get(url, path):
    if os.path.exists(path):
        return json.load(open(path))
    req = urllib.request.Request(url, headers={'accept': 'application/json', 'user-agent': 'tender-db-519-m7'})
    with urllib.request.urlopen(req, timeout=90) as r:
        body = r.read()
    d = json.loads(body)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path + '.tmp', 'w') as f:
        json.dump(d, f)
    os.replace(path + '.tmp', path)
    time.sleep(0.5)
    return d


def civil(n):
    return datetime.datetime.fromtimestamp(n[1] + 60 * (n[2] or 0), datetime.timezone.utc).date().isoformat()


def targets():
    m1 = json.load(open(os.path.join(HERE, 'm1.json')))
    reads = json.load(open(os.path.join(HERE, 'm4-reads.json')))
    nt = {n[0]: n for n in reads['notices']}
    out = []
    for h in m1['holders']:
        if h['child'] not in HOLDERS:
            continue
        refused = set()
        for nd in h['name_detail']:
            ch = nd['check'].get(h['chosen_parent'] or '', {})
            if ch.get('verdict') != 'admitted':
                refused.add(nd['name'])
        if h['child'] in ONLY_NAMES:
            refused &= ONLY_NAMES[h['child']]
        ms = [m for m in reads['mentions'] if m[0] == h['org_id']]
        for m in ms:
            if m[3] in refused:
                out.append({'code': h['child'], 'org_id': h['org_id'], 'notice_id': m[1], 'section_id': m[2],
                            'name': m[3], 'date': civil(nt[m[1]]), 'why': 'refused'})
        if h['child'] == '13T':
            nenc = sorted((m for m in ms if m[3] not in refused), key=lambda m: (nt[m[1]][1], m[1]))
            pick = {m[1]: 'nenc-oldest' for m in nenc[:3]}
            pick.update({m[1]: 'nenc-newest' for m in nenc[-3:]})
            window = [m for m in nenc if '2024-04-16' <= civil(nt[m[1]]) <= '2024-05-16']
            step = max(1, len(window) // 8)
            pick.update({m[1]: 'nenc-csu-window' for m in window[::step][:8]})
            for m in nenc:
                if m[1] in pick:
                    out.append({'code': '13T', 'org_id': h['org_id'], 'notice_id': m[1], 'section_id': m[2],
                                'name': m[3], 'date': civil(nt[m[1]]), 'why': pick[m[1]]})
    return out


def main():
    ts = targets()
    json.dump(ts, open(os.path.join(RAW, 'targets.json'), 'w'), indent=1)
    print(f'{len(ts)} target mentions')
    for t in ts:
        nid = t['notice_id']
        meta = get(f'https://tenders.zebreus.click/v1/notices/{nid}', os.path.join(RAW, 'notice', f'{nid}.json'))
        get(f'https://tenders.zebreus.click/v1/notices/{nid}/content', os.path.join(RAW, 'content', f'{nid}.json'))
        pub = meta['publication_id']
        get(f'https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages/{pub}',
            os.path.join(RAW, 'fts', f'{pub}.json'))
    print('done')


if __name__ == '__main__':
    main()
