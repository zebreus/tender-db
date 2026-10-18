"""Issue 551 B5, step 6: the confirmation read's draw, plus the band rows outside the reference 20.

docs/operations.md "Path W v2", B5's runbook steps 5 and 6.

THE DRAW (fixed before any Tender is read; record the salt in issue 551 BEFORE running it):
  - population: the changed set of one or more /v1/changes?entity=tender cursor ranges given on the command line
    (`--range FROM TO`, the corpus drain's, and the cohort drain's if wanted). An event counts when
    FROM < cursor <= TO (the `since` convention), entity tender, op `changed`, `version` null (the corrections D3
    announces). The ranges are unioned and deduplicated.
  - excluded, skipped UNREAD: the 279 labelled ids (postdrain/read_result.json "final") and the 277 walked ids
    (unit4b/walk.jsonl + walk_fallback.jsonl), per operations.md step 6, plus any --exclude-file ids.
  - order: sha256("551-b5-confirm-" + decimal id) hex digest ascending, ties by id.
  - the walk: one GET /v1/tenders/{id} per id, in that order. Hold the Tender when it is NOW served
    `value_derivation.method == "lot-sum-awards"`, skip it otherwise. Stop once 30 are held.
    * A 404 is skipped and recorded.
    * Any other failed read stops the run, which is resumable: reads are cached and never re-fetched.
  - the pre-draw record (salt, ranges, population, exclusions, the full order and its sha256) is written to
    OUT.predraw.json BEFORE the first Tender read. A re-run checks that the population and order it recomputes match
    that record, and refuses when they differ: a different draw needs a new salt.

THE BAND (operations.md step 5): a bounded seek, GET /v1/tenders?min_value=10000000000&limit=1000, paged by
next_cursor. Its rows served `lot-sum-awards` are kept. Each row's EUR value is computed as the port of record does:
b3/rule_v2_b3.py `rate()` at `head_date(published_at)`, then merge_rule's `to_eur`. Listed:
  - every row outside the reference 20 with S_eur >= EUR 100 m (a stop and read);
  - every row outside the 20 that the server returned but the port puts below EUR 100 m, or cannot convert;
  - any of the 20 missing (a switch-off stop);
  - any of the 20 the port puts below EUR 100 m.
The band pages are re-read on every run that is not --offline.

    python3 -I -B confirm_draw.py run --range FROM TO [--range FROM2 TO2] [--n 30] [--cache DIR] [--out FILE]
                                      [--offline] [--max-reads N] [--no-band | --band-only] [--exclude-file F]
    python3 -I -B confirm_draw.py selftest [--out FILE]       offline, a fake changes feed / Tenders / band pages

Every GET goes through cohort_parity.Http: at most 1 per 1.5 s, sharing one stamp file with cohort_parity.py; a 408
is retried once. The cache (default scratchpad b5/confirm/) holds changes/, tenders/ and band/.

Exit codes, the first that applies, in this order:
  2  a read failed, or an input or pre-draw check refused. Nothing is decided.
  4  the band differs from the reference 20: rows outside it, or one of the 20 missing. Read the output before going on.
  3  the changed set ran out before N were held.
  0  N held, and the band is exactly the 20.
"""
import argparse
import hashlib
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import time
from urllib.parse import quote

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location('cohort_parity_b5', os.path.join(HERE, 'cohort_parity.py'))
CP = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(CP)

SALT = '551-b5-confirm-'
ORDER_RULE = 'sha256("551-b5-confirm-" + decimal id) hex digest ascending, ties by id'
METHOD = 'lot-sum-awards'
N_HOLD = 30
BAND_MIN_CENTS = 10_000_000_000                       # EUR 100,000,000.00
REF20 = (683951, 707424, 726354, 1102571, 1147190, 1148297, 5278856, 5842187, 6017838, 6057925, 6195903, 6370901,
         7001924, 7011414, 8426318, 6569442, 6603588, 6679375, 7139761, 7542368)
DEFAULT_CACHE = os.path.join(CP.B5_DIR, 'confirm')
MAX_CHANGE_PAGES = 5000
MAX_BAND_PAGES = 2000


class Refused(Exception):
    pass


def salted(i):
    return hashlib.sha256((SALT + str(int(i))).encode()).hexdigest()


def cursor_pos(c):
    """A /v1/changes cursor as its position: plain ("193055") or generation-qualified ("3:193055")."""
    return int(str(c).rsplit(':', 1)[-1])


def safe(s):
    return re.sub(r'[^0-9A-Za-z_.-]', '_', str(s))


def sha_json(obj):
    return hashlib.sha256(json.dumps(obj, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


class Reads:
    """Cached reads under `root`. An existing file is never re-fetched unless `refresh`. A 404 is remembered as
    `<file>.404` when the caller allows it. With `offline`, an uncached read raises."""

    def __init__(self, root, http=None, offline=False, max_reads=None):
        self.root, self.offline, self.max_reads = root, offline, max_reads
        self.http = http
        self.reads = 0
        self.log = []                       # (rel, 'cache' | 'get' | '404-cache')

    def get_json(self, rel, path, allow_404=False, refresh=False):
        p = os.path.join(self.root, rel)
        if not refresh:
            if os.path.exists(p):
                self.log.append((rel, 'cache'))
                return 200, CP.ld(p)
            if allow_404 and os.path.exists(p + '.404'):
                self.log.append((rel, '404-cache'))
                return 404, None
        if self.offline:
            raise Refused(f'offline: {rel} is not cached ({path})')
        if self.max_reads is not None and self.reads >= self.max_reads:
            raise Refused(f'--max-reads {self.max_reads} reached before {path}')
        if self.http is None:
            self.http = CP.Http()
        self.reads += 1
        self.log.append((rel, 'get'))
        code, body = self.http.get(path)
        if code == 200:
            obj = json.loads(body)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p + '.tmp', 'wb') as fh:
                fh.write(body)
            os.replace(p + '.tmp', p)
            with open(p[:-5] + '.meta', 'w') as fh:
                fh.write(json.dumps({'fetched_at': CP.now(), 'path': path}))
            return 200, obj
        if code == 404 and allow_404:
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p + '.404', 'w') as fh:
                fh.write(json.dumps({'fetched_at': CP.now(), 'path': path}))
            return 404, None
        raise Refused(f'{path}: HTTP {code} {(body or b"")[:200]!r}')


# ------------------------------------------------------------------------------------------------ the changed set
def changed_set(reads, lo, hi, max_pages=MAX_CHANGE_PAGES):
    lo_p, hi_p = cursor_pos(lo), cursor_pos(hi)
    if lo_p >= hi_p:
        raise Refused(f'--range {lo} {hi}: FROM is not before TO')
    since, events, pages, gens = str(lo), [], [], set()
    for _ in range(max_pages):
        rel = f'changes/since_{safe(since)}.json'
        path = f'/v1/changes?entity=tender&since={quote(since, safe=":")}&limit=1000'
        _, d = reads.get_json(rel, path)
        if not reads.offline and not d.get('more') and d.get('last_cursor') is not None \
                and cursor_pos(d['last_cursor']) < hi_p:
            _, d = reads.get_json(rel, path, refresh=True)          # a cached head page: the feed may have moved on
        if d.get('reset'):
            raise Refused(f'/v1/changes since={since}: reset {d["reset"]} (the cursor range is not readable)')
        if d.get('ignored_filters'):
            raise Refused(f'/v1/changes since={since}: ignored_filters {d["ignored_filters"]}')
        gens.add(d.get('generation'))
        if len(gens) > 1:
            raise Refused(f'pages from feed generations {sorted(map(str, gens))}: their cursors do not compose')
        evs = d.get('events') or []
        events += evs
        last = d.get('last_cursor')
        pages.append({'since': since, 'events': len(evs), 'last_cursor': last, 'more': bool(d.get('more')),
                      'generation': d.get('generation')})
        if last is None or cursor_pos(last) >= hi_p or not d.get('more'):
            break
        if cursor_pos(last) <= cursor_pos(since):
            raise Refused(f'/v1/changes since={since}: last_cursor {last} did not advance')
        since = str(last)
    else:
        raise Refused(f'more than {max_pages} change pages before cursor {hi}')
    if any(e.get('cursor') is None for e in events):
        raise Refused('an event carries no cursor')
    reach = max([cursor_pos(e['cursor']) for e in events] +
                [cursor_pos(p['last_cursor']) for p in pages if p['last_cursor'] is not None], default=None)
    if reach is None or reach < hi_p:
        raise Refused(f'the feed reaches cursor {reach}, short of TO {hi}: the changed set would be truncated')
    inside = [e for e in events if lo_p < cursor_pos(e['cursor']) <= hi_p]
    picked = [e for e in inside if e.get('entity') == 'tender' and e.get('op') == 'changed' and e.get('version') is None]
    tally = {}
    for e in inside:
        k = f"{e.get('entity')}/{e.get('op')}/{'version-null' if e.get('version') is None else 'version-set'}"
        tally[k] = tally.get(k, 0) + 1
    ids = sorted({int(e['id']) for e in picked})
    cur = [cursor_pos(e['cursor']) for e in inside]
    rec = {'from': str(lo), 'to': str(hi), 'rule': 'from < cursor <= to; entity tender, op changed, version null',
           'query': '/v1/changes?entity=tender&since=<cursor>&limit=1000, paged by last_cursor',
           'pages': len(pages), 'page_log': pages, 'generations': sorted(map(str, gens)), 'events_read': len(events),
           'events_in_range': len(inside), 'events_outside_range': len(events) - len(inside), 'in_range_by_kind': tally,
           'picked_events': len(picked), 'distinct_ids': len(ids), 'first_in_range': min(cur, default=None),
           'last_in_range': max(cur, default=None), 'read_reaches': reach, 'ids_sha256': sha_json(ids)}
    return ids, rec


def exclusions(extra_file=None):
    lab, walked = CP.cohort_sources()
    ex = {i: 'labelled-279' for i in lab}
    for i in walked:
        ex.setdefault(i, 'walked-277')
    if extra_file:
        d = CP.ld(extra_file)
        for i in (d['ids'] if isinstance(d, dict) else d):
            ex.setdefault(int(i), 'exclude-file')
    return ex


# ------------------------------------------------------------------------------------------------ EUR, as the port
def port_eur(R, value, published_at):
    """-> (S_eur cents or None, rate or None): the port's RatesLookup at the head's date and merge_rule.to_eur."""
    if not value or value.get('cents') is None or not value.get('currency'):
        return None, None
    cur, cents = value['currency'], int(value['cents'])
    r = R.rate(cur, R.head_date(published_at))
    if r is None:
        return None, None
    return (cents if cur == 'EUR' else R.MR.to_eur({cur: r}, cur, cents)), r


def row_of(R, x, **extra):
    eur, r = port_eur(R, x.get('value'), x.get('published_at'))
    return dict({'id': int(x['id']), 'value': x.get('value'), 'value_derivation': x.get('value_derivation'),
                 'published_at': x.get('published_at'), 'source': x.get('source'), 'version': x.get('version'),
                 'S_eur_port': eur, 'rate': r, 'band_port': eur is not None and eur >= BAND_MIN_CENTS,
                 'in_ref20': int(x['id']) in REF20}, **extra)


def band_seek(reads, R, fresh, max_pages=MAX_BAND_PAGES):
    cursor, pages, rows, items = None, [], [], 0
    for _ in range(max_pages):
        rel = f'band/cursor_{safe(cursor) if cursor else "start"}.json'
        path = f'/v1/tenders?min_value={BAND_MIN_CENTS}&limit=1000' + (f'&cursor={quote(cursor, safe="")}' if cursor else '')
        _, d = reads.get_json(rel, path, refresh=fresh)
        if d.get('ignored_filters'):
            raise Refused(f'band seek: ignored_filters {d["ignored_filters"]}')
        its = d.get('items') or []
        items += len(its)
        for it in its:
            if ((it.get('value_derivation') or {}).get('method')) == METHOD:
                rows.append(row_of(R, it))
        pages.append({'cursor': cursor, 'items': len(its), 'more': bool(d.get('more')), 'next_cursor': d.get('next_cursor')})
        if not d.get('more'):
            break
        nc = d.get('next_cursor')
        if not nc or nc == cursor:
            raise Refused(f'band seek: more=true with next_cursor {nc!r} after cursor {cursor!r}')
        cursor = nc
    else:
        raise Refused(f'band seek: more than {max_pages} pages')
    ids = [r['id'] for r in rows]
    found = set(ids)
    out = {
        'query': f'/v1/tenders?min_value={BAND_MIN_CENTS}&limit=1000, paged by next_cursor; kept: value_derivation.method '
                 f'== {METHOD}',
        'read_at': CP.now(), 'pages': len(pages), 'page_log': pages, 'items_seen': items,
        'lot_sum_awards_rows': len(rows), 'duplicate_ids': sorted({i for i in ids if ids.count(i) > 1}),
        'ref20_found': sorted(found & set(REF20)), 'ref20_missing': sorted(set(REF20) - found),
        'outside_ref20_band': [r for r in rows if not r['in_ref20'] and r['band_port']],
        'outside_ref20_server_only': [r for r in rows if not r['in_ref20'] and not r['band_port']],
        'ref20_port_below_100m': [r for r in rows if r['in_ref20'] and not r['band_port']],
        'rows': rows,
    }
    out['exactly_ref20'] = not out['ref20_missing'] and not out['outside_ref20_band'] and not out['outside_ref20_server_only']
    return out


# ------------------------------------------------------------------------------------------------ the run
def predraw_record(ranges, population, excl, order, n_hold, excl_file):
    in_pop = {i: excl[i] for i in population if i in excl}
    by = {}
    for v in in_pop.values():
        by[v] = by.get(v, 0) + 1
    return {
        'what': 'issue 551 B5 step 6: the confirmation draw, fixed before any Tender is read',
        'salt': SALT, 'order_rule': ORDER_RULE, 'n_hold': n_hold,
        'hold_rule': f'GET /v1/tenders/{{id}} in order; hold when value_derivation.method == {METHOD}; 404 skipped and '
                     'recorded; stop at n_hold',
        'ranges': ranges,
        'population': {'changed_ids': len(population), 'ids_sha256': sha_json(population)},
        'excluded': {'rule': 'the 279 labelled (postdrain/read_result.json "final") and the 277 walked (unit4b walk files)'
                             + (f' and {excl_file}' if excl_file else '') + ': skipped unread',
                     'labels_sha256': CP.LABELS_SHA, 'walk_sha256': {os.path.basename(f): CP.sha256_file(f) for f in CP.WALKS},
                     'in_population': len(in_pop), 'by_reason': by, 'ids': sorted(in_pop)},
        'eligible': len(order), 'order_sha256': sha_json(order), 'order': order,
        'script_sha256': CP.sha256_file(os.path.abspath(__file__)), 'fixed_at': CP.now(),
    }


def do_run(args, http=None):
    R = CP.load_port()
    reads = Reads(args.cache, http=http, offline=args.offline, max_reads=args.max_reads)
    out_p = args.out or os.path.join(args.cache, 'confirm_draw.json')
    pre_p = out_p[:-5] + '.predraw.json' if out_p.endswith('.json') else out_p + '.predraw.json'
    res = {'what': 'issue 551 B5 step 6: confirmation draw + band rows outside the reference 20 (docs/operations.md '
                   '"Path W v2")', 'started_at': CP.now(), 'salt': SALT, 'order_rule': ORDER_RULE,
           'ranges': [{'from': a, 'to': b} for a, b in (args.range or [])], 'cache': args.cache, 'offline': args.offline,
           'reference_band_20': sorted(REF20)}
    if not args.band_only:
        if not args.range:
            raise Refused('run needs --range FROM TO (or --band-only)')
        population, ranges = set(), []
        for a, b in args.range:
            ids, rec = changed_set(reads, a, b)
            population |= set(ids)
            ranges.append(rec)
        population = sorted(population)
        excl = exclusions(args.exclude_file)
        order = sorted((i for i in population if i not in excl), key=lambda i: (salted(i), i))
        pre = predraw_record(ranges, population, excl, order, args.n, args.exclude_file)
        if os.path.exists(pre_p):
            old = CP.ld(pre_p)
            keep = lambda d: {k: v for k, v in d.items() if k not in ('fixed_at', 'script_sha256')}  # noqa: E731
            if keep(old) != keep(pre):
                raise Refused(f'{pre_p} exists and differs from this run\'s population / order: a new draw needs a NEW '
                              'salt (edit SALT, record it in issue 551 first) or a new --out')
            pre = old
        else:
            CP.dump(pre_p, pre)                                           # BEFORE the first Tender read
        res['ranges'] = [{k: r[k] for k in ('from', 'to', 'rule', 'events_in_range', 'picked_events', 'distinct_ids',
                                            'first_in_range', 'last_in_range', 'read_reaches', 'generations')}
                         for r in pre['ranges']]
        res['pre_draw_file'] = pre_p
        res['pre_draw_sha256'] = CP.sha256_file(pre_p)
        res['population'] = pre['population']
        res['excluded'] = {k: pre['excluded'][k] for k in ('in_population', 'by_reason', 'ids')}
        res['eligible'] = len(order)
        held, skipped, nf, walked = [], [], [], 0
        for pos, tid in enumerate(order):
            if len(held) >= args.n:
                break
            walked += 1
            code, x = reads.get_json(f'tenders/{tid}.json', f'/v1/tenders/{tid}', allow_404=True)
            if code == 404:
                nf.append({'id': tid, 'order_pos': pos})
                continue
            if int(x.get('id', tid)) != tid:
                raise Refused(f'/v1/tenders/{tid} answered id {x.get("id")}')
            m = (x.get('value_derivation') or {}).get('method')
            if m == METHOD:
                held.append(row_of(R, x, order_pos=pos, salted=salted(tid)))
            else:
                skipped.append({'id': tid, 'order_pos': pos, 'method': m})
        served = walked - len(nf)
        res['draw'] = {
            'n_hold': args.n, 'held_count': len(held), 'walked': walked, 'not_found_404': nf,
            'skipped_not_lot_sum_awards': len(skipped), 'skipped_by_method': {str(k): sum(1 for s in skipped if s['method'] == k)
                                                                              for k in sorted({s['method'] for s in skipped}, key=str)},
            'exhausted_before_n': len(held) < args.n,
            'lot_sum_awards_share_of_walked': (round(len(held) / served, 4) if served else None),
            'held_ids': [h['id'] for h in held], 'held': held, 'skipped': skipped,
            'held_band_rows_outside_ref20': [h['id'] for h in held if h['band_port'] and not h['in_ref20']],
        }
    if not args.no_band:
        res['band'] = band_seek(reads, R, fresh=not args.offline)
        if 'draw' in res:
            seen = {r['id'] for r in res['band']['rows']}
            res['band']['held_band_rows_missing_from_seek'] = [i for i in res['draw']['held_band_rows_outside_ref20']
                                                               if i not in seen]
    res['gets'] = reads.reads
    short = 'draw' in res and res['draw']['exhausted_before_n']
    band_off = 'band' in res and (not res['band']['exactly_ref20'] or res['band'].get('held_band_rows_missing_from_seek'))
    res['exit'] = 4 if band_off else (3 if short else 0)
    res['status'] = {0: 'OK: N held; the band is exactly the reference 20',
                     3: 'SHORT: the changed set ran out before N were held',
                     4: 'BAND DIFFERS: rows outside the reference 20 (stop and read, operations.md steps 4-6) or one of '
                        'the 20 missing (switch off, step 7)'}[res['exit']]
    res['finished_at'] = CP.now()
    CP.dump(out_p, res)
    d = res.get('draw') or {}
    b = res.get('band') or {}
    print(f"draw: held {d.get('held_count')}/{d.get('n_hold')} walked {d.get('walked')} (404 {len(d.get('not_found_404', []))}) "
          f"of eligible {res.get('eligible')}; salt {SALT!r}; ranges {[(r['from'], r['to']) for r in res['ranges']]}")
    if b:
        print(f"band: {b['lot_sum_awards_rows']} lot-sum-awards rows; ref20 found {len(b['ref20_found'])}, missing "
              f"{b['ref20_missing']}; outside the 20 (port >= EUR 100 m) {[r['id'] for r in b['outside_ref20_band']]}; "
              f"server-only {[r['id'] for r in b['outside_ref20_server_only']]}")
    print(res['status'], '->', out_p)
    print(f'GATE-EXIT={res["exit"]}')
    return res['exit']


def cmd_run(args):
    try:
        return do_run(args)
    except Refused as e:
        print(f'REFUSED: {e}')
    except Exception as e:  # noqa: BLE001 - any other failure decides nothing either
        import traceback
        traceback.print_exc()
        print(f'ERROR: {e!r}')
    print('GATE-EXIT=2')
    return 2


# ------------------------------------------------------------------------------------------------ selftest (offline)
def _ev(cur, i, op='changed', version=None, entity='tender'):
    return {'changed_at': '2026-10-19T03:00:00Z', 'cursor': str(cur), 'entity': entity, 'id': i, 'op': op, 'version': version}


def build_world(root, band_kind):
    """A fake feed (two change pages), fake Tenders and fake band pages under root (a confirm_draw cache)."""
    syn = list(range(9_100_001, 9_100_121))                   # 120 synthetic Tenders
    lsa = {i for i in syn if i % 5 in (0, 2)}                 # 48 served lot-sum-awards
    vouched = {i for i in syn if i % 17 == 0 and i not in lsa}
    gone = 9_100_005                                          # answered 404
    # a lot-sum-awards head of EUR 150 m (a draw-side band row): the first one the salted order holds, so the draw
    # surely meets it; in the 'exact' world it serves a sub-band sum instead
    big = next(i for i in sorted(syn, key=lambda i: (salted(i), i)) if i in lsa and i != gone)
    big_cents = 15_000_000_000 if band_kind != 'exact' else 1_234_567
    real_excluded = [683951, 13292, 7001924]                  # labelled, walked, labelled: must be skipped UNREAD
    events, c = [_ev(5000, 9_300_001)], 5000                  # cursor == FROM: outside (from < cursor)
    for i in syn + real_excluded:
        c += 1
        events.append(_ev(c, i))
    for kw in ({'i': 9_300_002, 'version': 7}, {'i': 9_300_003, 'op': 'added', 'version': 1},
               {'i': 9_300_004, 'entity': 'lot'}, {'i': syn[0]}):        # daily ingest, an add, a lot, a duplicate
        c += 1
        events.append(_ev(c, **kw))
    beyond = [_ev(5401, 9_300_005), _ev(5402, 9_300_006), _ev(5403, 9_300_007)]   # past TO = 5400
    p1, p2 = events[:70], events[70:] + beyond
    CP.dump(os.path.join(root, 'changes', 'since_5000.json'),
            {'events': p1, 'generation': 2, 'ignored_filters': [], 'last_cursor': p1[-1]['cursor'], 'more': True})
    CP.dump(os.path.join(root, 'changes', f"since_{p1[-1]['cursor']}.json"),
            {'events': p2, 'generation': 2, 'ignored_filters': [], 'last_cursor': '5410', 'more': False})
    for i in syn:
        if i == gone:
            os.makedirs(os.path.join(root, 'tenders'), exist_ok=True)
            open(os.path.join(root, 'tenders', f'{i}.json.404'), 'w').write('{}')
            continue
        if i in lsa:
            v = {'cents': big_cents if i == big else 1_000_000 + 7_919 * (i % 1000), 'currency': 'CHF' if i % 3 == 0 else 'EUR'}
            if i == big:
                v['currency'] = 'EUR'
            dv = {'method': METHOD, 'summed': 'awards', 'lots': 2 + i % 4,
                  'vouched_by': {'field': 'TED-VAL_TOTAL', 'notice_id': 20_000_000 + i % 1000, 'cents': v['cents']}}
        elif i in vouched:
            v, dv = {'cents': 2_000_000, 'currency': 'EUR'}, {'method': 'lot-sum-vouched', 'summed': 'awards', 'lots': 2, 'rule': 537}
        else:
            v, dv = ({'cents': 500_000, 'currency': 'EUR'} if i % 2 else None), None
        CP.dump(os.path.join(root, 'tenders', f'{i}.json'),
                {'id': i, 'source': 'ted', 'version': 1, 'published_at': '2024-05-02', 'value': v, 'value_derivation': dv})

    def item(i, cents, cur='EUR', dv=METHOD):
        return {'id': i, 'published_at': '2024-05-02', 'source': 'ted', 'version': 1, 'value': {'cents': cents, 'currency': cur},
                'value_derivation': None if dv is None else {'method': dv, 'summed': 'awards', 'lots': 3}}
    ref = [item(i, 20_000_000_000 + k) for k, i in enumerate(REF20)]
    if band_kind == 'exact':
        pages = [ref[:12] + [item(9_400_001, 200_000_000_000, dv=None)], ref[12:] + [item(9_400_002, 30_000_000_000, dv='lot-sum-vouched')]]
    else:
        ref = [r for r in ref if r['id'] != 6679375]                                  # one of the 20 missing
        pages = [ref[:10] + [item(9_400_001, 200_000_000_000, dv=None), item(9_200_001, 15_000_000_000)],
                 ref[10:] + [item(9_200_002, 9_000_000_000, 'CHF'), item(big, 15_000_000_000),
                             item(9_400_002, 30_000_000_000, dv='lot-sum-vouched')]]
    CP.dump(os.path.join(root, 'band', 'cursor_start.json'),
            {'ignored_filters': [], 'items': pages[0], 'more': True, 'next_cursor': '500000'})
    CP.dump(os.path.join(root, 'band', 'cursor_500000.json'),
            {'ignored_filters': [], 'items': pages[1], 'more': False, 'next_cursor': None})
    return {'syn': syn, 'lsa': lsa, 'gone': gone, 'big': big, 'real_excluded': real_excluded}


def cmd_selftest(args):
    base = os.path.join(CP.B5_DIR, 'selftest_confirm')
    if os.path.exists(base):
        shutil.rmtree(base)
    os.makedirs(base)
    me = os.path.abspath(__file__)
    ev = {'started_at': CP.now(), 'checks': []}

    def check(name, ok, **kw):
        ev['checks'].append(dict(name=name, ok=bool(ok), **kw))
        print(('ok   ' if ok else 'FAIL ') + name, json.dumps(kw, default=str)[:500], flush=True)

    def run(cache, out, *extra):
        cmd = [sys.executable, '-I', '-B', me, 'run', '--cache', cache, '--out', out, '--offline'] + list(extra)
        p = subprocess.run(cmd, capture_output=True, text=True)
        return p.returncode, (CP.ld(out) if os.path.exists(out) else None), p.stdout[-2000:] + p.stderr[-2000:]

    A = os.path.join(base, 'A')
    w = build_world(A, 'differs')
    # the expected draw, computed independently of the script
    pop = sorted(set(w['syn']) | set(w['real_excluded']))
    elig = [i for i in pop if i not in w['real_excluded']]
    order = sorted(elig, key=lambda i: hashlib.sha256(f'551-b5-confirm-{i}'.encode()).hexdigest())
    exp_held, exp_404 = [], []
    for i in order:
        if len(exp_held) == 30:
            break
        if i == w['gone']:
            exp_404.append(i)
        elif i in w['lsa']:
            exp_held.append(i)
    code, r, log = run(A, os.path.join(A, 'out.json'), '--range', '5000', '5400')
    d, b = r['draw'], r['band']
    check('run A: the changed set is exactly the in-range tender/changed/version-null ids, deduplicated',
          r['population']['changed_ids'] == len(pop) == 123 and r['ranges'][0]['picked_events'] == 124
          and r['ranges'][0]['distinct_ids'] == 123 and r['ranges'][0]['events_in_range'] == 127,
          population=r['population'], range=r['ranges'][0])
    check('run A: the labelled / walked ids in the population are excluded unread',
          r['excluded']['ids'] == sorted(w['real_excluded']) and r['excluded']['by_reason'] == {'labelled-279': 2, 'walked-277': 1},
          excluded=r['excluded'])
    pre = CP.ld(r['pre_draw_file'])
    check('run A: the recorded order is sha256("551-b5-confirm-"+id) ascending (recomputed here)',
          pre['order'] == order and pre['salt'] == '551-b5-confirm-' and r['salt'] == '551-b5-confirm-'
          and r['ranges'][0]['from'] == '5000' and r['ranges'][0]['to'] == '5400', first5=pre['order'][:5])
    check('run A: held = the first 30 lot-sum-awards Tenders in that order, the 404 recorded',
          d['held_ids'] == exp_held and [x['id'] for x in d['not_found_404']] == exp_404 and d['held_count'] == 30
          and d['walked'] == order.index(exp_held[-1]) + 1, held=d['held_ids'][:8], n404=d['not_found_404'], walked=d['walked'])
    check('run A: band rows outside the 20 listed (port EUR >= 100 m), server-only row split out, the missing reference '
          'head named, exit 4',
          code == 4 and [x['id'] for x in b['outside_ref20_band']] == [9_200_001, w['big']]
          and [x['id'] for x in b['outside_ref20_server_only']] == [9_200_002] and b['ref20_missing'] == [6679375]
          and len(b['ref20_found']) == 19 and not b['exactly_ref20'] and b['items_seen'] == 24,
          exit=code, outside=[(x['id'], x['S_eur_port']) for x in b['outside_ref20_band']],
          server_only=[(x['id'], x['value'], x['S_eur_port'], x['rate']) for x in b['outside_ref20_server_only']],
          missing=b['ref20_missing'])
    check('run A: the draw holds a band row outside the 20, and the seek found it too',
          w['big'] in exp_held and d['held_band_rows_outside_ref20'] == [w['big']] and b['held_band_rows_missing_from_seek'] == [],
          held_band=d['held_band_rows_outside_ref20'])

    # the pre-draw record is on disk before the first Tender read (in-process, a fresh copy of the world)
    B = os.path.join(base, 'B')
    build_world(B, 'differs')
    out_b = os.path.join(B, 'out.json')
    seen = {}
    orig = Reads.get_json

    def spy(self, rel, path, allow_404=False, refresh=False):
        if rel.startswith('tenders/') and 'first' not in seen:
            seen['first'] = rel
            seen['predraw_existed'] = os.path.exists(out_b[:-5] + '.predraw.json')
            seen['predraw_order_len'] = len(CP.ld(out_b[:-5] + '.predraw.json')['order']) if seen['predraw_existed'] else None
        return orig(self, rel, path, allow_404=allow_404, refresh=refresh)
    Reads.get_json = spy
    try:
        ns = argparse.Namespace(cache=B, out=out_b, offline=True, max_reads=None, range=[['5000', '5400']], n=30,
                                band_only=False, no_band=True, exclude_file=None)
        code_b = do_run(ns)
    finally:
        Reads.get_json = orig
    check('the pre-draw record (full order) is written before the first Tender read',
          seen.get('predraw_existed') is True and seen['predraw_order_len'] == len(order) and code_b == 0, **seen)

    # resume: the same run again reproduces the draw and passes the pre-draw check; a changed population is refused
    code2, r2, _ = run(A, os.path.join(A, 'out.json'), '--range', '5000', '5400')
    check('a re-run with the same range reproduces the draw (the pre-draw check passes)',
          code2 == 4 and r2['draw']['held_ids'] == d['held_ids'] and r2['pre_draw_sha256'] == r['pre_draw_sha256'],
          exit=code2)
    code3, _, log3 = run(A, os.path.join(A, 'out.json'), '--range', '5000', '5100')
    check('a different population against the same pre-draw record is refused (exit 2)',
          code3 == 2 and 'NEW salt' in log3, exit=code3, log=log3.strip().splitlines()[-2:])
    # two ranges that tile 5000-5400 give the same population and draw
    code4, r4, _ = run(A, os.path.join(A, 'out_two.json'), '--range', '5000', '5069', '--range', '5069', '5400')
    check('two ranges tiling 5000-5400 (at the fake feed\'s page boundary 5069) give the same population and draw',
          r4 is not None and r4['population'] == r['population'] and r4['draw']['held_ids'] == d['held_ids'],
          exit=code4, ranges=r4 and [(x['from'], x['to'], x['distinct_ids']) for x in r4['ranges']])
    # a range the feed does not reach is refused, never truncated
    code5, _, log5 = run(A, os.path.join(A, 'out_short.json'), '--range', '5000', '6000')
    check('a TO beyond the feed head is refused (exit 2), never a truncated changed set',
          code5 == 2 and 'short of TO 6000' in log5, exit=code5, log=log5.strip().splitlines()[-2:])
    # the changed set running out before N
    code6, r6, _ = run(A, os.path.join(A, 'out_n1000.json'), '--range', '5000', '5400', '--n', '1000')
    check('--n 1000: every lot-sum-awards Tender held (47 = 48 - the 404), the draw marked short',
          r6['draw']['held_count'] == 47 and r6['draw']['exhausted_before_n'] and code6 == 4,
          exit=code6, held=r6['draw']['held_count'])
    code6b, r6b, _ = run(A, os.path.join(A, 'out_n1000_noband.json'), '--range', '5000', '5400', '--n', '1000', '--no-band')
    check('--n 1000 --no-band: exit 3 (short)', code6b == 3, exit=code6b)
    # the exact band
    C = os.path.join(base, 'C')
    build_world(C, 'exact')
    code7, r7, _ = run(C, os.path.join(C, 'out.json'), '--range', '5000', '5400')
    check('a band of exactly the 20 (non-lot-sum-awards rows ignored) and 30 held: exit 0',
          code7 == 0 and r7['band']['exactly_ref20'] and len(r7['band']['ref20_found']) == 20
          and r7['draw']['held_ids'] == d['held_ids'], exit=code7, band_rows=r7['band']['lot_sum_awards_rows'])
    code8, r8, _ = run(C, os.path.join(C, 'out_band.json'), '--band-only')
    check('--band-only on the exact band: exit 0, no draw', code8 == 0 and 'draw' not in r8, exit=code8)

    # the reader: 200 cached, 404 remembered, other codes refused, offline refuses an uncached read
    D = os.path.join(base, 'D')
    script = {'/v1/tenders/1': (200, b'{"id": 1}'), '/v1/tenders/2': (404, b'{}'), '/v1/tenders/3': (500, b'boom')}
    h = CP.Http(stamp=os.path.join(base, '.stamp'), gap=0, transport=lambda p: script[p], retry_408_sleep=0)
    rd = Reads(D, http=h)
    a = rd.get_json('tenders/1.json', '/v1/tenders/1', allow_404=True)
    bb = rd.get_json('tenders/2.json', '/v1/tenders/2', allow_404=True)
    try:
        rd.get_json('tenders/3.json', '/v1/tenders/3', allow_404=True)
        c3 = 'no raise'
    except Refused as e:
        c3 = str(e)
    rd2 = Reads(D, offline=True)
    a2 = rd2.get_json('tenders/1.json', '/v1/tenders/1', allow_404=True)
    b2 = rd2.get_json('tenders/2.json', '/v1/tenders/2', allow_404=True)
    try:
        rd2.get_json('tenders/4.json', '/v1/tenders/4')
        c4 = 'no raise'
    except Refused as e:
        c4 = str(e)
    check('Reads: 200 cached and re-served offline, 404 remembered, a 500 refused, offline never GETs',
          a == (200, {'id': 1}) and bb == (404, None) and 'HTTP 500' in c3 and a2 == a and b2 == (404, None)
          and c4.startswith('offline') and rd.reads == 3 and rd2.reads == 0, five_hundred=c3, offline=c4)

    # EUR as the port does: port_eur on B3's 73 ships (value = the port's S, B3's detail published_at) equals the
    # port's own S_eur, and the ones at >= EUR 100 m are exactly the reference 20
    import gzip
    R = CP.load_port()
    rows = [json.loads(line) for line in gzip.open(CP.B3_OUTCOMES, 'rt', encoding='utf-8')]
    ships = [x for x in rows if x['ship']]
    off, band = [], set()
    for x in ships:
        det = CP.ld(os.path.join(CP.B3_TENDERS, f"{x['id']}.json"))
        eur, _ = port_eur(R, {'cents': x['S'], 'currency': x['cur']}, det.get('published_at'))
        if eur != x['S_eur']:
            off.append((x['id'], eur, x['S_eur']))
        if eur is not None and eur >= BAND_MIN_CENTS:
            band.add(x['id'])
    check("port_eur reproduces the port's S_eur on B3's 73 ships; those >= EUR 100 m are exactly the reference 20",
          len(ships) == 73 and not off and band == set(REF20), ships=len(ships), mismatches=off,
          band_minus_ref=sorted(band - set(REF20)), ref_minus_band=sorted(set(REF20) - band))

    ev['finished_at'] = CP.now()
    ev['passed'] = all(c['ok'] for c in ev['checks'])
    ev['run_A'] ={'exit': code, 'salt': r['salt'], 'ranges': r['ranges'], 'population': r['population'],
                   'excluded': r['excluded'], 'held_ids': d['held_ids'], 'not_found_404': d['not_found_404'],
                   'walked': d['walked'], 'band': {k: b[k] for k in ('ref20_missing', 'outside_ref20_band',
                                                                     'outside_ref20_server_only', 'exactly_ref20')}}
    out = args.out or os.path.join(base, 'selftest.json')
    CP.dump(out, ev)
    print('SELFTEST', 'PASSED' if ev['passed'] else 'FAILED', '->', out)
    return 0 if ev['passed'] else 1


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    p = sub.add_parser('run')
    p.add_argument('--range', nargs=2, action='append', metavar=('FROM', 'TO'),
                   help='a /v1/changes cursor range, FROM < cursor <= TO (repeatable; unioned)')
    p.add_argument('--n', type=int, default=N_HOLD)
    p.add_argument('--cache', default=DEFAULT_CACHE)
    p.add_argument('--out')
    p.add_argument('--offline', action='store_true')
    p.add_argument('--max-reads', type=int)
    p.add_argument('--exclude-file')
    g = p.add_mutually_exclusive_group()
    g.add_argument('--no-band', action='store_true')
    g.add_argument('--band-only', action='store_true')
    p = sub.add_parser('selftest')
    p.add_argument('--out')
    args = ap.parse_args(argv)
    return {'run': cmd_run, 'selftest': cmd_selftest}[args.cmd](args)


if __name__ == '__main__':
    sys.exit(main())
