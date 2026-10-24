#!/usr/bin/env python3
"""519 M6: review.json + verdicts.json + packet.json -> reading.md (one line per row) and counts.json."""
import collections, json, os

D = '/home/user/tender-db/.scratch/tender-db/519-m6'
rev = json.load(open(os.path.join(D, 'review.json')))
body = {v['org_id']: v for v in json.load(open(os.path.join(D, 'verdicts.json')))['verdicts']}
pk = {c['org']: c for c in json.load(open(os.path.join(D, 'packet.json')))['cases']}
ORDER = ['body-ppon', 'ppon-value', 'body-no-ppon', 'new-body-no-ppon', 'coh-value', 'company-coh', 'nhse-523',
         'site-or-hosted', 'junk']


def esc(s):
    return str(s).replace('|', '\\|').replace('\t', ' ').replace('\n', ' ')


def posted_kind(v, b):
    if b is None:
        return 'not posted (D8: literal over 64 chars)'
    c = b['correct_identifier']
    shape = v['expected']['shape']
    if b['confidence'] != 'high' and shape in ('merge', 'move'):
        shape = 'hold'
    tgt = f" -> {v['expected']['target']}" if v['expected']['target'] else ''
    return f"wrong{(' + ' + c) if c else ''} [{shape}{tgt if shape in ('merge', 'hold') else ''}] {b['confidence']}"


lines = []
for v in sorted(rev, key=lambda v: (ORDER.index(v['route']) if v['route'] in ORDER else 99, v['org'])):
    c = pk[v['org']]
    cls = 'E' if c['ods'] == '404' else 'F'
    b = body.get(v['org'])
    first = v['rationale'].split('. ')[0].rstrip('.') + '.'
    ev = v['evidence']
    agree = 'agrees' if v['challenger_agrees'] else 'DISAGREES'
    lines.append(f"| {v['org']} | `{esc(v['identifier'])}` | {cls} | {esc(v['route'])} | {esc(c['head'][:60])} ({c['mentions']}) | "
                 f"{esc(posted_kind(v, b))} | {esc(first)} Evidence: {esc(ev)}. Challenger {agree}. |")

cnt = collections.Counter()
for v in rev:
    b = body.get(v['org'])
    if b is None:
        cnt['not posted (D8)'] += 1
        continue
    shape = v['expected']['shape']
    if b['confidence'] != 'high' and shape in ('merge', 'move'):
        shape = 'hold'
    cc = b['correct_identifier'] or ''
    kind = ('none' if not cc else 'ppon' if cc.startswith('GBPPON') else 'ods' if shape == 'unkeyed'
            else 'company')
    cnt[f"{b['verdict']} / {b['confidence']} / {shape}" + (f" / {kind}" if b['correct_identifier'] else '')] += 1
json.dump(dict(cnt), open(os.path.join(D, 'counts.json'), 'w'), indent=1)
by_route = collections.Counter(v['route'] for v in rev)
mentions = sum(pk[v['org']]['mentions'] for v in rev)
open(os.path.join(D, 'reading-rows.md'), 'w').write('\n'.join(lines) + '\n')
print(json.dumps(dict(cnt), indent=1))
print(dict(by_route), 'mentions', mentions)
