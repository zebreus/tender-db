"""Issue 551 unit 4b, M1: the salted shadow walk of v2 over the failed drain's 15,352 path-W corrections.

    python3 -I -B walk.py            walk (resumes from walk.jsonl; stops at 30 v2 ships)
    python3 -I -B walk.py --report   write M1-walk.md's results and M1-walk-30.json from walk.jsonl

Pre-draw record: M1-walk.md (salt, order rule, population, skip set, rule sha256, per-id procedure, stop rule).
"""
import collections
import hashlib
import importlib.util
import json
import math
import os
import sys
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
RULE = os.path.join(HERE, 'rule_v2.py')
RULE_SHA = '39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce'
SCRATCH = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
POP = SCRATCH + '/v551/changed_tenders.json'
POP_SHA = '930422d4bd2d6f36ba15558a61946c534501d60294f575bf3e3808d133a02d44'
LABELS = os.path.join(HERE, '..', 'postdrain', 'read_result.json')
LABELS_SHA = '048a62c1981b5c6271a34b471807a8fbc754c6a38225cb32c91d5bdd471a4306'
SALT = 0x551B4
MULT = 2654435761
TARGET = 30
LOG = os.path.join(HERE, 'walk.jsonl')
FACTS_DIR = SCRATCH + '/m551/walk_facts'
PINS = {585917, 5995030, 467176, 294968, 7256284, 5977060, 8426318, 6450467, 6450469, 6450473, 8831876}
N_POP = 15352


def sha(p):
    return hashlib.sha256(open(p, 'rb').read()).hexdigest()


def order_key(i):
    return ((i ^ SALT) * MULT) % (1 << 32)


def walk_order():
    if sha(POP) != POP_SHA:
        raise SystemExit('walk: population file changed')
    if sha(LABELS) != LABELS_SHA:
        raise SystemExit('walk: label file changed')
    pop = [int(i) for i in json.load(open(POP))]
    assert len(pop) == len(set(pop)) == N_POP
    lab = {int(k) for k in json.load(open(LABELS))['final']}
    assert len(lab) == 279
    return sorted(pop, key=lambda i: (order_key(i), i)), lab


def load_rule():
    got = sha(RULE)
    if got != RULE_SHA:
        raise SystemExit(f'walk: rule_v2.py sha256 {got} is not the recorded {RULE_SHA}; the draw would be void')
    spec = importlib.util.spec_from_file_location('rule_v2_551', RULE)
    R = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(R)
    return R


def read_log():
    if not os.path.exists(LOG):
        return []
    return [json.loads(l) for l in open(LOG) if l.strip()]


def walk():
    order, lab = walk_order()
    R = load_rule()
    gets = [0]
    real_get = R._get

    def counting_get(path):
        gets[0] += 1
        return real_get(path)
    R._get = counting_get                                  # counts GETs only; behaviour unchanged
    os.makedirs(FACTS_DIR, exist_ok=True)
    done = read_log()
    rows = [d for d in done if 'id' in d]
    expect = [i for i in order if i not in lab][:len(rows)]
    if [d['id'] for d in rows] != expect:
        raise SystemExit('walk: walk.jsonl is not a prefix of the salted order')
    ships = sum(1 for d in rows if d['outcome'] == 'ship')
    out = open(LOG, 'a')
    if not done:
        out.write(json.dumps({'meta': {'salt': SALT, 'order': '((id XOR salt) * 2654435761) mod 2^32, then id',
                                       'rule_v2_sha256': RULE_SHA, 'population_sha256': POP_SHA,
                                       'labels_sha256': LABELS_SHA, 'target_ships': TARGET,
                                       'started_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}}) + '\n')
        out.flush()
    pos = {i: k for k, i in enumerate(order)}
    seq = len(rows)
    for i in order:
        if ships >= TARGET:
            break
        if i in lab or i in {d['id'] for d in rows}:
            continue
        seq += 1
        g0, t0 = gets[0], time.time()
        row = {'seq': seq, 'order_pos': pos[i], 'order_key': order_key(i), 'id': i, 'pin': i in PINS}
        try:
            f = R.facts(i, refresh=True)
        except R.FetchError as e:
            row.update(outcome='unread', error=repr(e.args)[:400])
            f = None
        except Exception:                                  # noqa: BLE001 - logged, not counted
            row.update(outcome='port_error', error=traceback.format_exc()[-800:])
            f = None
        if f is not None:
            with open(os.path.join(FACTS_DIR, f'{i}.json'), 'w') as fh:
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
        if row['outcome'] == 'ship':
            ships += 1
        print(f"{seq:3d} {i:>9} {row['outcome']:<10} {row.get('first') or '':<16} S={row.get('S')} {row.get('currency')} "
              f"n={row.get('n')} gets={row['gets']} ships={ships}", flush=True)
    out.write(json.dumps({'end': {'ships': ships, 'walked': len(rows),
                                  'ended_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}}) + '\n')
    out.close()


# ---------------------------------------------------------------- report
def binom_cdf(k, n, p):
    if k < 0:
        return 0.0
    if k >= n:
        return 1.0
    return sum(math.comb(n, j) * p ** j * (1 - p) ** (n - j) for j in range(k + 1))


def solve(fn, target, lo=0.0, hi=1.0, increasing=True):
    for _ in range(200):
        mid = (lo + hi) / 2
        v = fn(mid)
        if (v < target) == increasing:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def clopper_pearson(x, n, a=0.05):
    lo = 0.0 if x == 0 else solve(lambda p: 1 - binom_cdf(x - 1, n, p), a / 2, increasing=True)
    hi = 1.0 if x == n else solve(lambda p: binom_cdf(x, n, p), a / 2, increasing=False)
    return lo, hi


def inverse_sampling(r, N, a=0.05):
    """Exact interval for p when trials stop at the r-th success, observed on trial N."""
    lo = solve(lambda p: 1 - binom_cdf(r - 1, N, p), a / 2, increasing=True)
    hi = 1.0 if r == N else solve(lambda p: binom_cdf(r - 1, N - 1, p), a / 2, increasing=False)
    return lo, hi


def money(c, cur):
    if c is None:
        return 'null'
    return f'{cur} {c // 100:,}.{c % 100:02d}'


def eur(c):
    return 'n/a' if c is None else f'EUR {int(round(c)) // 100:,}.{int(round(c)) % 100:02d}'


def summary(f):
    """The facts summary for M1-walk-30.json (and M2's readers)."""
    v1 = f['v1']
    return {
        'id': f['id'], 'source': f.get('source'), 'published_at': f.get('published_at'), 'head_date': v1['date'],
        'version': f.get('version'), 'S': f['S'], 'currency': f['currency'], 'n': f['n'], 'S_eur_cents': f.get('S_eur'),
        'S_display': money(f['S'], f['currency']), 'S_eur_display': eur(f.get('S_eur')),
        'rate_per_eur': 1.0 if f['currency'] == 'EUR' else v1['rates'].get(f['currency']),
        'served_head': v1['served'], 'served_derivation': v1['served_derivation'],
        'port_max_election': v1['today'], 'served_matches_election': v1['served_matches_election'],
        'v1_why': v1['why'],
        'summed': [{k: s.get(k) for k in ('key', 'notice_id', 'result_key', 'cents', 'decision', 'published_lot_no',
                                          'published_lot_no_src', 'contractor_ids', 'contractor_names')}
                   for s in f['summed']],
        'vouched_by': f.get('vouched_by'), 'vouches': f.get('vouches'),
        'stated_totals': [{k: t.get(k) for k in ('notice_id', 'field', 'section_kind', 'cents', 'currency', 'r3_kind',
                                                 'vouches', 'form', 'equals_its_single_lot_award')}
                          for t in f.get('stated_totals', [])],
        'bids_in_summed_notice_by_lot': f.get('bids_by_summed_notice'),
        'checks': {k: v for k, v in (f.get('pred') or {}).items()},
        'n_notices': len(f['notices']), 'notices': f['notices'], 'flags': f['flags'],
    }


def report():
    order, lab = walk_order()
    log = read_log()
    meta = next(d['meta'] for d in log if 'meta' in d)
    end = next((d['end'] for d in log if 'end' in d), None)
    rows = [d for d in log if 'id' in d]
    assert [d['id'] for d in rows] == [i for i in order if i not in lab][:len(rows)]
    ships = [d for d in rows if d['outcome'] == 'ship']
    refused = [d for d in rows if d['outcome'] == 'refuse']
    dropped = [d for d in rows if d['outcome'] == 'dropped']
    bad = [d for d in rows if d['outcome'] in ('unread', 'port_error')]
    v1f = len(ships) + len(refused)
    counted = v1f + len(dropped)
    last_pos = rows[-1]['order_pos'] if rows else None
    skipped_lab = [i for i in order[:last_pos + 1] if i in lab] if rows else []
    gets = sum(d.get('gets', 0) for d in rows)

    x = len(ships)
    cp = clopper_pearson(x, v1f)
    inv = inverse_sampling(x, v1f) if x else (0, 0)
    proj = N_POP * x / v1f
    cp_d = clopper_pearson(x, counted)
    proj_d = N_POP * x / counted
    lab_ships = 47              # M1-port-check.json: v2 ships 47 of the 279 labelled ids (all in the population)
    proj_s = (N_POP - 279) * x / counted + lab_ships
    cp_s = ((N_POP - 279) * cp_d[0] + lab_ships, (N_POP - 279) * cp_d[1] + lab_ships)

    first_hist = collections.Counter(d['first'] for d in refused)
    all_hist = collections.Counter(k for d in refused for k in d.get('refusals', []))
    drop_hist = collections.Counter((d.get('v1_why') or '').split(':')[0] for d in dropped)
    band = [d for d in ships if d.get('S_eur') is not None and d['S_eur'] >= 10_000_000_000]
    ten = [d for d in ships if d.get('S_eur') is not None and 1_000_000_000 <= d['S_eur'] < 10_000_000_000]
    near = [d for d in rows if any(fl.startswith('within_2pct') for fl in d.get('flags') or [])]
    scale = [d for d in rows if 'scale_rules_head_only' in (d.get('flags') or [])]
    pins = [d for d in rows if d.get('pin')]

    def fmt_row(d):
        o = d['outcome']
        chk = d.get('first') or ''
        if o == 'refuse' and len(d.get('refusals', [])) > 1:
            chk += ' (' + ', '.join(d['refusals'][1:]) + ')'
        if o in ('unread', 'port_error'):
            chk = d.get('error', '')[:80].replace('|', '/').replace('\n', ' ')
        s = money(d.get('S'), d.get('currency')) if d.get('S') is not None else '-'
        return (f"| {d['seq']} | {d['order_pos']} | {d['id']} | {o} | {chk} | {s} | {d.get('n') if d.get('n') is not None else '-'} | "
                f"{eur(d.get('S_eur')) if d.get('S_eur') is not None else '-'} | {d.get('n_notices', '-')} |")

    L = []
    L.append('## Result\n')
    status = 'COMPLETE' if x >= TARGET else f'STOPPED EARLY at {x} ships'
    L.append(f'**{status}.** Walk started {meta["started_at"]}' + (f', ended {end["ended_at"]}' if end else '') +
             f'. {len(rows)} ids walked (order positions 0..{last_pos}; {len(skipped_lab)} labelled ids skipped on the '
             f'way), {gets} GETs.\n')
    L.append('| outcome | ids |\n|---|---|')
    L.append(f'| v2 ship | {x} |\n| v2 refuse | {len(refused)} |\n| dropped (v1 does not fire) | {len(dropped)} |')
    L.append(f'| unread / port error (not counted) | {len(bad)} |\n| **walked** | **{len(rows)}** |\n')
    L.append(f'- **v1-fire rate among walked ids:** {v1f} of {counted} counted ids ({100 * v1f / counted:.1f} %)'
             + (f'; {len(bad)} unread/port-error ids excluded.' if bad else '.'))
    L.append(f'- **v2 ship rate among v1 fires:** {x} of {v1f} ({100 * x / v1f:.1f} %); refused {len(refused)} '
             f'({100 * len(refused) / v1f:.1f} %).')
    L.append(f'- **Projected v2 fire count (the task\'s formula, 15,352 x ships / v1-fires-walked):** '
             f'**{proj:,.0f}**; 95 % interval {N_POP * cp[0]:,.0f}–{N_POP * cp[1]:,.0f} (Clopper-Pearson on {x}/{v1f}). '
             f'The exact interval for a walk that stops at the 30th ship (inverse sampling) is '
             f'{N_POP * inv[0]:,.0f}–{N_POP * inv[1]:,.0f}.')
    if dropped:
        L.append(f'- Drop-adjusted (15,352 x ships / counted walked ids, since {len(dropped)} walked ids were not v1 fires): '
                 f'{proj_d:,.0f}; 95 % {N_POP * cp_d[0]:,.0f}–{N_POP * cp_d[1]:,.0f}.')
    L.append(f'- Stratified (the 15,073 unlabelled ids at the walked rate {x}/{counted}, plus the 47 labelled ids the port '
             f'ships): {proj_s:,.0f}; 95 % {cp_s[0]:,.0f}–{cp_s[1]:,.0f}. The 279 include the value-selected band, so '
             f'they are not a uniform part of the population; this line keeps them at their known outcomes.')
    L.append(f'- The decision expected about 9,800 (95 % 7,550–11,830; plan for 6,000–10,000), about 47 ids walked and '
             f'about 36 % refused.\n')

    L.append('### v2 ships at or above EUR 100 m (the band read needs them)\n')
    if band:
        L.append('| id | S | n | S_eur | vouched by |\n|---|---|---|---|---|')
        for d in band:
            vb = d.get('vouched_by') or {}
            L.append(f"| {d['id']} | {money(d['S'], d['currency'])} | {d['n']} | {eur(d['S_eur'])} | "
                     f"{vb.get('field')} of notice {vb.get('notice_id')}: {money(vb.get('cents'), d['currency'])} |")
    else:
        L.append('**None.** No walked v2 ship has S_eur at or above EUR 100,000,000.00, so the band set for M2 stays the '
                 'judge\'s 15.')
    L.append('')
    L.append('### v2 ships in [EUR 10 m, EUR 100 m) (R10 territory under amendment (a))\n')
    if ten:
        L.append('| id | S | n | S_eur | vouched by |\n|---|---|---|---|---|')
        for d in ten:
            vb = d.get('vouched_by') or {}
            L.append(f"| {d['id']} | {money(d['S'], d['currency'])} | {d['n']} | {eur(d['S_eur'])} | "
                     f"{vb.get('field')} of notice {vb.get('notice_id')}: {money(vb.get('cents'), d['currency'])} |")
    else:
        L.append('None.')
    L.append('')

    walked10 = [d for d in rows if d.get('S_eur') is not None and d['S_eur'] >= 1_000_000_000]
    L.append('### Notes for M2 and B5\n')
    vouched = [d for d in ships if d.get('vouched_by')]
    nulls = [d for d in ships if not d.get('served')]
    ratios = []
    for d in ships:
        sv = d.get('served') or {}
        if sv.get('cents') and sv.get('currency') == d['currency']:
            ratios.append((d['S'] / sv['cents'], d['id']))
    srcs = collections.Counter()
    for d in ships:
        f = json.load(open(os.path.join(FACTS_DIR, f"{d['id']}.json")))
        for s in f['summed']:
            srcs[(s.get('published_lot_no_src') or 'none').split(' of ')[0].rsplit(' ', 1)[0]
                 if s.get('published_lot_no') is not None else 'none'] += 1
    top = max(ships, key=lambda d: d.get('S_eur') or 0) if ships else None
    L.append(f'- **Largest ship:** {top["id"]} at {eur(top["S_eur"])}.' if top else '- No ships.')
    L.append(f'- **Walked ids at or above EUR 10 m, any outcome:** '
             + (', '.join(f"{d['id']} ({d['outcome']}, {d.get('first')}, {eur(d['S_eur'])}, R10 "
                          f"{'fires' if 'R10' in d.get('refusals', []) else 'does not fire'})" for d in walked10)
                if walked10 else 'none') + '.')
    L.append(f'- **Vouched ships** (a stated total within 1 % of S, at any size): {len(vouched)} of {len(ships)}: '
             + ', '.join(f"{d['id']} ({d['vouched_by']['field']})" for d in vouched) + '. The rest are below EUR 10 m, '
             'where R10 does not apply.')
    L.append(f'- **Served head null today:** {len(nulls)} of {len(ships)} ships ({", ".join(str(d["id"]) for d in nulls)}).')
    if ratios:
        ratios.sort()
        L.append(f'- **S over today\'s served head** (same currency, {len(ratios)} ships): {ratios[0][0]:.2f}x ({ratios[0][1]}) '
                 f'to {ratios[-1][0]:.2f}x ({ratios[-1][1]}); median {ratios[len(ratios) // 2][0]:.2f}x.')
    L.append(f'- **Published lot numbers of the {sum(srcs.values())} summed lots:** '
             + ', '.join(f'{k} {v}' for k, v in srcs.most_common()) + '.')
    L.append(f'- **B5 stop band** (0.5x–2x of this projection): {0.5 * proj:,.0f}–{2 * proj:,.0f} corrections.\n')

    L.append('## The 30 shipped ids\n')
    L.append('| # | id | source | S | currency | n | S_eur | vouched by | served head today |\n|---|---|---|---|---|---|---|---|---|')
    for k, d in enumerate(ships, 1):
        vb = d.get('vouched_by')
        vbs = f"{vb['field']} ({vb['notice_id']})" if vb else '-'
        sv = d.get('served') or {}
        svs = money(sv.get('cents'), sv.get('currency')) if sv else 'null'
        L.append(f"| {k} | {d['id']} | {d.get('source')} | {money(d['S'], d['currency'])} | {d['currency']} | {d['n']} | "
                 f"{eur(d.get('S_eur'))} | {vbs} | {svs} |")
    L.append('')
    L.append(f"Ids: {', '.join(str(d['id']) for d in ships)}.\n")
    L.append('`M1-walk-30.json` holds each one\'s facts summary (summed lots with notice, published lot number and winners; '
             'stated totals; vouches; served head; flags; notices).\n')

    L.append('## Refusals\n')
    L.append(f'**First refusing check** ({len(refused)} refused):\n')
    L.append('| check | ids |\n|---|---|')
    for k, c in sorted(first_hist.items(), key=lambda kv: (-kv[1], kv[0])):
        L.append(f'| {k} | {c} |')
    L.append('')
    L.append('**Every refusing check** (an id counts once per check that refuses it):\n')
    L.append('| check | ids |\n|---|---|')
    for k, c in sorted(all_hist.items(), key=lambda kv: (-kv[1], kv[0])):
        L.append(f'| {k} | {c} |')
    L.append('')
    if dropped:
        L.append('**Dropped (v1 does not fire today), by v1 reason:**\n')
        L.append('| v1 reason | ids |\n|---|---|')
        for k, c in sorted(drop_hist.items(), key=lambda kv: (-kv[1], kv[0])):
            L.append(f'| {k} | {c} |')
        L.append('')

    L.append('## Flags\n')
    L.append(f"- **`scale_rules_head_only`** (a figure of EUR 1 bn or more; ScalePartners reads the head only): "
             + (', '.join(f"{d['id']} ({d['outcome']})" for d in scale) if scale else 'none') + '.')
    L.append(f"- **Within 2 % of EUR 10 m or EUR 100 m:** "
             + (', '.join(f"{d['id']} ({d['outcome']}, {eur(d.get('S_eur'))}, {[fl for fl in d['flags'] if fl.startswith('within')]})" for d in near) if near else 'none') + '.')
    L.append(f"- **Served head differs from the port's max election (fails closed as W/G0-served):** "
             + (', '.join(str(d['id']) for d in rows if d.get('served_matches_election') is False) or 'none') + '.')
    L.append(f"- **Pins walked** (not labelled, so walked as any id): "
             + (', '.join(f"{d['id']} ({d['outcome']})" for d in pins) if pins else 'none') + '.')
    L.append(f"- **Unread / port errors:** "
             + ('; '.join(f"{d['id']}: {d['outcome']} {d.get('error', '')[:120]}" for d in bad) if bad else 'none') + '.\n')

    L.append('## Every walked id, in salted order\n')
    L.append('`order_pos` is the position in the full salted order of the 15,352 (labelled ids included); labelled ids '
             'were skipped, not logged. The check column gives the first refusing check, then the others in brackets; '
             'for a dropped id it gives v1\'s reason.\n')
    L.append('| # | order_pos | id | outcome | check | S | n | S_eur | notices |\n|---|---|---|---|---|---|---|---|---|')
    for d in rows:
        L.append(fmt_row(d))
    L.append('')
    L.append(f'Labelled ids skipped between order positions 0 and {last_pos}: '
             + (', '.join(str(i) for i in skipped_lab) if skipped_lab else 'none') + '.\n')
    L.append('## Files\n')
    L.append(f'- `unit4b/walk.py` (this walk and report), sha256 `{sha(os.path.abspath(__file__))}`.')
    L.append(f'- `unit4b/walk.jsonl` (one line per walked id), sha256 `{sha(LOG)}`.')
    L.append('- `unit4b/M1-walk-30.json` (the 30 ships\' facts summaries).')
    L.append(f'- Full facts of every walked id: `{FACTS_DIR}/{{id}}.json` (scratch).')
    L.append('- Reproduce: `python3 -I -B walk.py --report` rewrites this results part from walk.jsonl.')

    md = open(os.path.join(HERE, 'M1-walk.md')).read()
    cut = md.find('\n## Result\n')
    head = md if cut < 0 else md[:cut]
    head = head.replace('**Status: IN PROGRESS. This header was written',
                        f'**Status: {status}. The pre-draw header below was written')
    open(os.path.join(HERE, 'M1-walk.md'), 'w').write(head.rstrip('\n') + '\n\n' + '\n'.join(L) + '\n')

    out = []
    for d in ships:
        f = json.load(open(os.path.join(FACTS_DIR, f"{d['id']}.json")))
        s = summary(f)
        s.update(seq=d['seq'], order_pos=d['order_pos'], fetched_at=d['at'])
        out.append(s)
    json.dump({'salt': SALT, 'order': meta['order'], 'rule_v2_sha256': RULE_SHA, 'ships': out},
              open(os.path.join(HERE, 'M1-walk-30.json'), 'w'), indent=1, ensure_ascii=False, default=str)
    print(json.dumps({'walked': len(rows), 'ships': x, 'refused': len(refused), 'dropped': len(dropped), 'bad': len(bad),
                      'v1f': v1f, 'proj': round(proj), 'cp': [round(N_POP * cp[0]), round(N_POP * cp[1])],
                      'inv': [round(N_POP * inv[0]), round(N_POP * inv[1])], 'first_hist': first_hist,
                      'band': [d['id'] for d in band], 'ten': [d['id'] for d in ten], 'gets': gets}, default=str))


if __name__ == '__main__':
    if sys.argv[1:] == ['--report']:
        report()
    else:
        walk()
