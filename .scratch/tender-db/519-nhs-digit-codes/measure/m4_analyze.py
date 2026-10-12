#!/usr/bin/env python3
"""519 M4 (offline): child-coded mentions per month and the share published after P's end.

Inputs: measure/m4-reads.json (m4_read.py), measure/child-map.json (build_child_map.py),
../sub-icb.json (name_icb per sub-ICB code), crates/ingest/sdk/nhs-ods.tsv (legal_end).
Writes measure/m4-results.json and prints markdown tables.

Date of a mention = its notice's published_at shifted by published_offset (minutes), as a civil
date (FTS publishes date+time in UK local time).
P (the chosen parent) = §1.2: the dated parent the holder's NAME points to (sub-icb.json name_icb)
when that is a dated parent; else the only dated parent; else the earliest allowlisted one.
P's end for the child = the earlier of the RE5/RE6 interval end and P's own legal_end (tsv).
"""
import collections, datetime, json, os
HERE = os.path.dirname(os.path.abspath(__file__))
U1 = os.path.dirname(HERE)
REPO = os.path.abspath(os.path.join(U1, '..', '..', '..'))
TODAY = datetime.date(2026, 10, 12)
MONTHS = [(2025, m) for m in range(10, 13)] + [(2026, m) for m in range(1, 11)]   # 2026-10 partial
A1 = 'B2M3M D4U1Y 15C D2P2L D9Y0V 16C 99C 03R M2L0M 11J 15F 42D 78H 01H 84H 02Y 36J 18C 92A'.split()
A1M = '13T 26A 15E 15N 93C'.split()
A3 = '03N 10Q 03W 01A 06Q 15M 06K 06N'.split()
REST = '15A 03K W2U3Z 14J 14N 03Q'.split()


def main():
    r = json.load(open(os.path.join(HERE, 'm4-reads.json')))
    cm = json.load(open(os.path.join(HERE, 'child-map.json')))
    sub = {s['code']: s for s in json.load(open(os.path.join(U1, 'sub-icb.json')))}
    legal_end = {}
    for l in open(os.path.join(REPO, 'crates', 'ingest', 'sdk', 'nhs-ods.tsv'), encoding='utf-8'):
        if l.startswith('#') or not l.strip():
            continue
        f = l.rstrip('\n').split('\t')
        legal_end[f[0]] = f[7] or None
    org_code = {o[0]: o[1][5:] for o in r['orgs']}
    notice = {n[0]: n for n in r['notices']}
    dated = collections.defaultdict(list)
    for row in cm['rows']:
        if row['parent']:
            dated[row['code']].append(row)
    role = {row['code']: row['child_role'] for row in cm['rows']}

    def cls(c):
        if role.get(c) == 'RO198':
            return 'B'
        for k, s in (('A1', A1), ('A1m', A1M), ('A3', A3), ('rest', REST)):
            if c in s:
                return k
        return 'A2'

    choice = {}
    for c in sorted(set(org_code.values())):
        ps = dated.get(c, [])
        names = sub.get(c, {}).get('name_icb')
        pick = [x for x in ps if x['parent'] == names] if names else []
        how = 'name'
        if not pick and len({x['parent'] for x in ps}) == 1:
            pick, how = ps, 'only'
        if not pick:
            allow = sorted((x for x in ps if x.get('parent_allow')), key=lambda x: x['from'])
            pick, how = (allow[:1], 'earliest-allow') if allow else ([], 'none')
        if not pick:
            choice[c] = dict(parent=None, how=how, end=None, rel_end=None, legal_end=None, dated=[])
            continue
        p = pick[0]['parent']
        ivs = [x for x in ps if x['parent'] == p]
        rel_end = None if any(x['to'] is None for x in ivs) else max(x['to'] for x in ivs)
        le = legal_end.get(p)
        ends = [e for e in (rel_end, le) if e]
        choice[c] = dict(parent=p, how=how, rel_end=rel_end, legal_end=le, end=min(ends) if ends else None,
                         dated=[[x['parent'], x['from'], x['to']] for x in ps])

    per_child = collections.defaultdict(lambda: dict(n=0, last12=0, after_end=0, after_end_last12=0,
                                                     after_end_names=collections.Counter(), months=collections.Counter(),
                                                     notices=set(), first=None, last=None))
    month = collections.defaultdict(lambda: collections.Counter())
    month_notices = collections.defaultdict(set)
    years = collections.Counter()
    uncoded = 0
    start12 = datetime.date(2025, 10, 1)
    end12 = datetime.date(2026, 9, 30)
    for m in r['mentions']:
        oid, nid, sec, name, country, raw, scheme = m
        c = org_code[oid]
        if c not in (raw or '').upper().replace(' ', ''):
            uncoded += 1
        n = notice[nid]
        dt = datetime.datetime.fromtimestamp(n[1] + (n[2] or 0) * 60, datetime.timezone.utc).date()
        pc = per_child[c]
        pc['n'] += 1
        pc['notices'].add(nid)
        pc['first'] = min(pc['first'] or dt, dt)
        pc['last'] = max(pc['last'] or dt, dt)
        years[(dt.year, cls(c) if cls(c) == 'B' else 'A')] += 1
        in12 = start12 <= dt <= end12
        if in12:
            pc['last12'] += 1
        ym = (dt.year, dt.month)
        if ym in MONTHS:
            k = cls(c)
            month[ym][k] += 1
            month[ym]['all'] += 1
            month_notices[ym].add(nid)
            pc['months'][ym] += 1
        e = choice[c]['end']
        if e and dt > datetime.date.fromisoformat(e):
            pc['after_end'] += 1
            pc['after_end_names'][name] += 1
            if in12:
                pc['after_end_last12'] += 1

    tot = sum(pc['n'] for pc in per_child.values())
    tot12 = sum(pc['last12'] for pc in per_child.values())
    after = sum(pc['after_end'] for pc in per_child.values())
    after12 = sum(pc['after_end_last12'] for pc in per_child.values())
    out = dict(total_mentions=tot, uncoded=uncoded, last12=tot12, after_end=after, after_end_last12=after12,
               months={f'{y}-{m:02d}': dict(month[(y, m)], notices=len(month_notices[(y, m)])) for y, m in MONTHS},
               years={f'{y}-{k}': v for (y, k), v in sorted(years.items())},
               children={c: dict(cls=cls(c), parent=choice[c], n=pc['n'], notices=len(pc['notices']), last12=pc['last12'],
                                 after_end=pc['after_end'], after_end_last12=pc['after_end_last12'],
                                 after_end_names=pc['after_end_names'].most_common(8),
                                 first=pc['first'].isoformat(), last=pc['last'].isoformat(),
                                 months={f'{y}-{m:02d}': v for (y, m), v in sorted(pc['months'].items())})
                         for c, pc in sorted(per_child.items())})
    json.dump(out, open(os.path.join(HERE, 'm4-results.json'), 'w'), indent=1, ensure_ascii=False)

    print(f'total {tot} uncoded {uncoded}; last 12 complete months (2025-10..2026-09) {tot12}; '
          f'after P end {after} (of which last12 {after12})')
    cols = ['A1', 'A1m', 'A2', 'A3', 'rest', 'B', 'all']
    print('| month | ' + ' | '.join(cols) + ' | notices |')
    print('|---|' + '---:|' * (len(cols) + 1))
    for y, m in MONTHS:
        print(f'| {y}-{m:02d} | ' + ' | '.join(str(month[(y, m)][k]) for k in cols) + f' | {len(month_notices[(y, m)])} |')
    print()
    print('| year | sub-ICB (A) | sites (B) |')
    for y in sorted({y for y, _ in years}):
        print(f'| {y} | {years[(y, "A")]} | {years[(y, "B")]} |')
    print()
    print('| child | class | P (how) | P end (rel / legal) | mentions | last 12 mo | after P end | after-end last 12 | first..last |')
    for c, pc in sorted(per_child.items(), key=lambda kv: -kv[1]['n']):
        ch = choice[c]
        print(f"| {c} | {cls(c)} | {ch['parent']} ({ch['how']}) | {ch['rel_end'] or '-'} / {ch['legal_end'] or '-'} | {pc['n']} | {pc['last12']} | "
              f"{pc['after_end']} | {pc['after_end_last12']} | {pc['first']}..{pc['last']} |")


if __name__ == '__main__':
    main()
