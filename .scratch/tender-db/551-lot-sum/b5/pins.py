"""Issue 551 B5, step 4: the pins of docs/operations.md's Verify list, read on the public API.

    python3 -I -B pins.py baseline OUT.json     before the corpus drain writes (record every pin as served)
    python3 -I -B pins.py check BASE.json OUT.json   after it: the expectations below, and the must-stay heads unchanged

Expectations after B5 (operations.md "Verify, in order" and "Path W v2"):
  - the 16 path-V pins serve their sums, `lot-sum-vouched`, rule 537 (928196: rule 492), cents/currency as listed;
  - 8426318 GBP 3,000,000,000.00, 585917 EUR 250,000.00, 5995030 PLN 1,549,511.60: `lot-sum-awards` with a vouched_by;
  - the Lyon-Turin trio 6450467/6450469/6450473 EUR 1,660,661,682.94, value_derivation null;
  - the 9 must-stay heads: value_derivation null and value equal to the baseline's.
Every GET goes through cohort_parity.Http (1 per 1.5 s, a 408 retried once).
"""
import importlib.util
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location('cohort_parity_b5', os.path.join(HERE, 'cohort_parity.py'))
CP = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(CP)

PATH_V = {  # id -> (cents, currency, rule)
    7427765: (300000000, 'EUR', 537), 5943121: (132157992, 'EUR', 537), 369057: (6237873629, 'EUR', 537),
    251619: (2433875808, 'EUR', 537), 380921: (724704000, 'PLN', 537), 392980: (3574004272, 'EUR', 537),
    425446: (154900719, 'EUR', 537), 706887: (7056079275, 'PLN', 537), 1024214: (340000000, 'EUR', 537),
    5779108: (212000000, 'EUR', 537), 6342034: (1083675445, 'EUR', 537), 6921980: (2493033611, 'EUR', 537),
    7044132: (238599200, 'EUR', 537), 7829010: (200000000, 'EUR', 537), 8514592: (17924893700, 'DKK', 537),
    928196: (4679746035, 'EUR', 492),
}
PATH_W = {8426318: (300000000000, 'GBP'), 585917: (25000000, 'EUR'), 5995030: (154951160, 'PLN')}
TRIO = {6450467: (166066168294, 'EUR'), 6450469: (166066168294, 'EUR'), 6450473: (166066168294, 'EUR')}
MUST_STAY = [8440393, 7487018, 5316712, 6885102, 4930240, 8822313, 4925871, 175096, 5625591]
ALL = sorted(set(PATH_V) | set(PATH_W) | set(TRIO) | set(MUST_STAY))


def read_all():
    http = CP.Http()
    out = {}
    for i in ALL:
        code, body = http.get(f'/v1/tenders/{i}')
        if code != 200:
            raise SystemExit(f'pins: GET /v1/tenders/{i} answered {code}: {body[:200]!r}')
        t = json.loads(body)
        out[str(i)] = {'version': t.get('version'), 'value': t.get('value'), 'derivation': t.get('value_derivation')}
    return out


def check(base, now):
    bad = []

    def val(i):
        v = now[str(i)]['value'] or {}
        return v.get('cents'), v.get('currency')

    for i, (c, cur, rule) in PATH_V.items():
        d = now[str(i)]['derivation'] or {}
        if val(i) != (c, cur) or d.get('method') != 'lot-sum-vouched' or str(d.get('rule')) != str(rule):
            bad.append((i, 'path-V pin', now[str(i)]))
    for i, (c, cur) in PATH_W.items():
        d = now[str(i)]['derivation'] or {}
        if val(i) != (c, cur) or d.get('method') != 'lot-sum-awards' or not d.get('vouched_by'):
            bad.append((i, 'path-W pin', now[str(i)]))
    for i, (c, cur) in TRIO.items():
        if val(i) != (c, cur) or now[str(i)]['derivation'] is not None:
            bad.append((i, 'Lyon-Turin trio', now[str(i)]))
    for i in MUST_STAY:
        if now[str(i)]['derivation'] is not None or now[str(i)]['value'] != base[str(i)]['value']:
            bad.append((i, 'must-stay', {'base': base[str(i)], 'now': now[str(i)]}))
    return bad


def main():
    mode = sys.argv[1]
    if mode == 'baseline':
        json.dump(read_all(), open(sys.argv[2], 'w'), indent=1, sort_keys=True)
        print(f'{len(ALL)} pins -> {sys.argv[2]}')
    elif mode == 'check':
        base = json.load(open(sys.argv[2]))
        now = read_all()
        bad = check(base, now)
        json.dump({'now': now, 'bad': bad}, open(sys.argv[3], 'w'), indent=1, sort_keys=True)
        print(f'{len(ALL)} pins read, {len(bad)} off')
        for b in bad:
            print(' ', json.dumps(b))
        sys.exit(1 if bad else 0)
    else:
        sys.exit(__doc__)


if __name__ == '__main__':
    main()
