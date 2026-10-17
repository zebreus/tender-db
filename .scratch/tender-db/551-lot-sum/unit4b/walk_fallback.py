"""Issue 551 unit 4b, M2 fallback: "vouched everywhere" (v2 with R10 at every size), from the SAME salted walk.

    python3 -I -B walk_fallback.py            continue the salted order after walk.jsonl; stop at 30 vouched ships
    python3 -I -B walk_fallback.py --report   print the tallies from walk.jsonl + walk_fallback.jsonl

A vouched ship is a v2 ship (rule_v2.py, sha256 pinned by walk.py) whose facts carry a vouching stated total
(f['vouched_by'] set: a stated total within 1 % of S that equals no single summed award and is not a single-lot
notice's own award) -- exactly what R10 requires, applied at every size. walk.jsonl's 54 rows are reused as they
stand (5 vouched ships, already read in M2); new rows go to walk_fallback.jsonl. Pre-draw record: M2-fallback-walk.md.
"""
import importlib.util
import json
import os
import sys
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('walk_551', os.path.join(HERE, 'walk.py'))
W = importlib.util.module_from_spec(spec)
spec.loader.exec_module(W)

LOG = os.path.join(HERE, 'walk_fallback.jsonl')
TARGET = 30


def rows_of(path):
    if not os.path.exists(path):
        return []
    return [d for d in (json.loads(l) for l in open(path) if l.strip()) if 'id' in d]


def vouched_ship(d):
    return d.get('outcome') == 'ship' and bool(d.get('vouched_by'))


def walk():
    order, lab = W.walk_order()
    R = W.load_rule()
    gets = [0]
    real_get = R._get

    def counting_get(path):
        gets[0] += 1
        return real_get(path)
    R._get = counting_get
    m1 = rows_of(W.LOG)
    new = rows_of(LOG)
    rows = m1 + new
    expect = [i for i in order if i not in lab][:len(rows)]
    if [d['id'] for d in rows] != expect:
        raise SystemExit('walk_fallback: walk.jsonl + walk_fallback.jsonl is not a prefix of the salted order')
    vouched = sum(1 for d in rows if vouched_ship(d))
    out = open(LOG, 'a')
    if not new:
        out.write(json.dumps({'meta': {'continues': 'walk.jsonl', 'reused_rows': len(m1),
                                       'reused_vouched_ships': vouched, 'target_vouched_ships': TARGET,
                                       'rule_v2_sha256': W.RULE_SHA, 'salt': W.SALT,
                                       'started_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}}) + '\n')
        out.flush()
    pos = {i: k for k, i in enumerate(order)}
    seen = {d['id'] for d in rows}
    seq = len(rows)
    for i in order:
        if vouched >= TARGET:
            break
        if i in lab or i in seen:
            continue
        seq += 1
        g0, t0 = gets[0], time.time()
        row = {'seq': seq, 'order_pos': pos[i], 'order_key': W.order_key(i), 'id': i, 'pin': i in W.PINS}
        try:
            f = R.facts(i, refresh=True)
        except R.FetchError as e:
            row.update(outcome='unread', error=repr(e.args)[:400])
            f = None
        except Exception:                                  # noqa: BLE001 - logged, not counted
            row.update(outcome='port_error', error=traceback.format_exc()[-800:])
            f = None
        if f is not None:
            with open(os.path.join(W.FACTS_DIR, f'{i}.json'), 'w') as fh:
                json.dump(f, fh, ensure_ascii=False, default=str)
            v1 = f['v1']
            row.update(v1_fires=R.v1_fires(f), v1_why=v1['why'], source=f.get('source'),
                       head_date=v1['date'], n_notices=len(f['notices']), flags=f['flags'],
                       served=v1['served'], served_derivation=v1['served_derivation'],
                       served_matches_election=v1['served_matches_election'])
            try:
                ship, first, S, cur, n = R.v2_decision(f)
                if not R.v1_fires(f):
                    row.update(outcome='dropped', first=first)
                else:
                    row.update(outcome='ship' if ship else 'refuse', first=first, refusals=R.v2_refusals(f))
                row.update(S=S, currency=cur, n=n, S_eur=f.get('S_eur'),
                           rate=(1.0 if cur == 'EUR' else v1['rates'].get(cur)) if cur else None,
                           vouched_by=f.get('vouched_by'), vouches=f.get('vouches'))
            except Exception:                              # noqa: BLE001
                row.update(outcome='port_error', error=traceback.format_exc()[-800:])
        row.update(gets=gets[0] - g0, secs=round(time.time() - t0, 1),
                   at=time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()))
        out.write(json.dumps(row, ensure_ascii=False, default=str) + '\n')
        out.flush()
        rows.append(row)
        seen.add(i)
        if vouched_ship(row):
            vouched += 1
        print(f"{seq:4d} {i:>9} {row['outcome']:<10} {row.get('first') or '':<10} "
              f"vouched={'Y' if row.get('vouched_by') else '-'} S={row.get('S')} {row.get('currency')} "
              f"gets={row['gets']} vouched_ships={vouched}", flush=True)
    out.write(json.dumps({'end': {'vouched_ships': vouched, 'walked': len(rows),
                                  'ended_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}}) + '\n')
    out.close()


def report():
    rows = rows_of(W.LOG) + rows_of(LOG)
    c = {}
    for d in rows:
        c[d['outcome']] = c.get(d['outcome'], 0) + 1
    v1 = sum(1 for d in rows if d.get('v1_fires'))
    vs = [d for d in rows if vouched_ship(d)]
    print(json.dumps({'walked': len(rows), 'outcomes': c, 'v1_fires': v1, 'vouched_ships': len(vs),
                      'projected_fallback_fires': round(W.N_POP * len(vs) / v1) if v1 else None,
                      'vouched_ship_ids': [d['id'] for d in vs]}, indent=1))


if __name__ == '__main__':
    report() if '--report' in sys.argv else walk()
