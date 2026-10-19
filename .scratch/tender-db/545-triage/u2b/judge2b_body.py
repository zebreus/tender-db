import json, sys
from collections import Counter

def run(rows, J, label):
    c = Counter(); missing = []
    for r in rows:
        try:
            r['class'] = J[(r['org'], r['name'])]; c[r['class']] += 1
        except KeyError:
            missing.append((r['org'], r['name']))
    assert not missing, missing
    unused = set(J) - {(r['org'], r['name']) for r in rows}
    assert not unused, unused
    keys = Counter(); km = Counter()
    seen = set()
    for r in rows:
        from keys import alt
        k = (r['org'], alt(r['name']))
        if k not in seen: seen.add(k); keys[r['class']] += 1
        km[r['class']] += r['n']
    print(label, 'rows', len(rows), 'distinct (org, alt key)', len(seen))
    for cl in sorted(c): print(f"   {cl:3s} rows {c[cl]:4d}  keys {keys[cl]:4d}  mentions {km[cl]:4d}")
    return rows

if __name__ == '__main__':
    import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    tail = [r for r in json.load(open('tail_k6.json')) if r['keep'] and r['detector'] not in ('STRAY', 'HELD-2')]
    b = [r for r in json.load(open('cand2b.json')) if r['keep']]
    json.dump(run(tail, TAIL, 'TAIL'), open('tail_judged.json', 'w'), indent=1, ensure_ascii=False)
    json.dump(run(b, B, 'B'), open('cand2b_judged.json', 'w'), indent=1, ensure_ascii=False)
