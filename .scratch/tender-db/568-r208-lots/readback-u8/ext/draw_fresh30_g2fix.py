"""568 unit 8 + 570 read-back: the fresh 30 (u7/decision-u7.md "Amendment (unit 8, 2026-10-18)", "The read-back bar",
"The draw"), drawn from the combined wet run's changed Tenders.

THIS IS A COPY of ../draw_fresh30.py (the original is not edited), made for the judge's O1 / O7
(readback-u8/readback.md "Open items and dispositions"). Two changes, nothing else of the walk is touched:

  1. THE G2 TEST IS CORRECTED (O7: the one-read winner-scope signal below fires on multi-CAN R2.0.9 chains, where a
     Tender-scope winner row from another, lot-less result makes a 567 same-notice result look like a 568 one; the
     judge found 3 false positives in the 30). A candidate is G2 when it is outside population.json AND some
     lot_result with a non-null `lot` comes from a notice that
       (a) has profile `ted-export-r208` in the Tender's /v1/notices?tender={id} listing (json::notice `profile`,
           every page followed), and
       (b) has no lot section of its own: /v1/notices/{id}/content holds no section of kind Lot / LotsGroup / Part
           (cross_notice_port.LOT_KINDS, the kinds remap_port.lot_sections and combined_port read; a notice with one
           places its results by 567's same-notice map, never by 568's cross-notice match).
     Reads, in order and only while needed: the Tender (1 GET); if it has a lot-placed result, its listing (1 GET per
     page); then, per lot-placed r208 notice in id order, its content until one has no lot section. All cached in
     --cache as check_live.Fetcher names them (tenders_<id>.json, notices_tender_<id>.json [+ _p<k> pages],
     notices_<id>_content.json). The old signal is still computed on every read Tender and recorded as
     `old_signal_hits`, for comparison only; it decides nothing.
  2. RESUME (O1: "keep the 27 ... continue the same walk from index 771"). `--resume DRAW.json --keep SPEC
     --from-index K` recomputes the salted order from --changed (same range, same exclusions), checks that its first
     K ids equal DRAW.json's `walked` ids one for one (else refuses), seeds `held` with the walk rows named by SPEC
     (e.g. 0-21,61,282,366,449,770; each must be a held row of DRAW.json), counts OUT from their `out` and G2 as
     `--keep-g2 SPEC` names (the judge's true-G2 rows), then walks on from index K under the same holding rule.
     The output's `draw` is the kept rows (walk order) followed by the new holds; `walked` holds only the rows from
     index K on, each with its absolute `index`.

The draw, fixed before any Tender is read:
  - population: the changed Tenders of the combined refold's cursor range (`--changed FILE`, see below);
  - excluded: the salted 150 (readback/u7_sample.json), the failed read-back's fresh 30 (readback/fresh30.json),
    unit 1's measured 77 (readback/measured77.json, measurement.json), 8327340, and the OOS 50 (u8-oos/draw.json);
  - order: sha256("568u8-fresh30-" + id) hex digest, ascending;
  - quotas over the 30 held: at least 10 OUT (the Tender's `published_at` year outside 2012-2015, the field the
    earlier fresh-30 draw read), and at least 8 G2 (outside readback/population.json AND serving an R2.0.8 block on a
    lot: the stratum 570's narrower G2 newly matches). A Tender can count for both.
  - the walk: each candidate in salted order is held unless holding it would leave fewer free slots than the quotas
    still need, counted as if no later Tender counted for both (30 - held - 1 >= need_out + need_g2 after it). So a
    Tender in neither class is held only while slots are spare, and the walk never skips a Tender a quota needs.
    A candidate that could only be held as a G2 Tender (the free slots all belong to the G2 quota) and is inside
    population.json is skipped UNREAD: it cannot be G2.

How one read tells "serves an R2.0.8 block on a lot" (decided here): the Tender's own /v1/tenders/{id} read, no
notice read. A lot_result with a non-null `lot` whose winners' `winner` party rows are Tender-scoped (`lot: null`)
and none of them on that result's lot. Why it works: `NoticeState::read` scopes an award block's winner to a lot only
through the notice's OWN lot sections (`raw_results.lot_of` + `lots.contains_key`); an award notice with no lot
section of its own (the R2.0.8 form, the only shape 568's cross-notice match places) files its winners at Tender
scope, while the match puts the result itself on the inherited lot. Measured on every cached full Tender read (the OOS
50, the fresh 30, 570's 211 and 567's exhibits): of the lot-placed results with a winner, 610 from notices with no lot
section all show the signal and 0 of 2,951 from notices with lot sections do. It misses a lot-placed R2.0.8 block
without a named winner (18 of 628 such results in those caches), so a Tender whose only such blocks are winnerless
counts as not G2: it lowers the G2 class's recall, never its precision.

`--changed FILE` accepts: plain ids (whitespace / comma separated); a JSON list of ids; a JSON object with `ids` or
`changed_null_version_ids` (readback/population.json's shape); or /v1/changes pages (a JSON object with `events`, a
list of such objects, or JSONL), from which the Tender events with op `changed` and `version: null` are taken (the
corrected Tenders D3 announces, as population.json was built); `--any-version` takes every Tender event instead.

`--from-cursor A --to-cursor B` (both or neither) keeps only the events with A < cursor <= B: the `since` convention
568's population used ("cursor in (956239230, 957852942]"), A the feed head before the combined refold and B the last
cursor it wrote. A cursor is read as its position, plain ("193055") or generation-qualified ("3:193055"). With a
range, FILE must be /v1/changes events, each with its cursor (an id list carries none and is refused), from one feed
`generation`, and reaching B (some event's cursor or a page's `last_cursor` >= B; else the read stops short of the
range and is refused). The range and its counts are recorded in the output's `cursor_range` (null without one;
check_live.py --bar refuses a draw without it).

Reads: one GET per candidate Tender, at most one per 1.5 s, a 408 retried once, cached in `--cache DIR` as
tenders_<id>.json (an existing file is never re-fetched). `--offline` makes no GET and stops at the first uncached
candidate it needs; `--max-reads N` caps the GETs. A candidate answered 404 (no longer served, e.g. merged away after
the refold) is skipped and recorded in `not_found_404`; any other failed read stops the walk.

Usage: python3 -I -B draw_fresh30_g2fix.py --changed FILE --cache DIR --out draw.json [--from-cursor A --to-cursor B]
       [--resume DRAW.json --keep SPEC [--keep-g2 SPEC] --from-index K]
       [--offline] [--max-reads N] [--any-version] [--gap 1.6]
"""
import collections
import hashlib
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))   # readback-u8/ext
RB = os.path.dirname(HERE)                          # readback-u8
ROOT = os.path.dirname(RB)                          # 568-r208-lots
sys.path.insert(0, RB)
import check_live as CL  # noqa: E402  (the rate-limited, cached reader; read-only use of the original)

LOT_KINDS = ('Lot', 'LotsGroup', 'Part')            # cross_notice_port.LOT_KINDS
R208 = 'ted-export-r208'

SALT = '568u8-fresh30-'
N, N_OUT, N_G2 = 30, 10, 8


def ld(p):
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)


def salted(i):
    return hashlib.sha256((SALT + str(i)).encode()).hexdigest()


def cursor_pos(c):
    """A /v1/changes cursor as its position: plain ("193055") or generation-qualified ("3:193055")."""
    return int(str(c).rsplit(':', 1)[-1])


def read_changed(path, any_version=False, rng=None):
    """(sorted Tender ids, cursor_range record or None). rng = (A, B) keeps only the events with A < cursor <= B."""
    text = open(path, encoding='utf-8').read()
    try:
        docs = [json.loads(text)]
    except ValueError:
        docs = []
        for line in text.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                docs.append(json.loads(line))
            except ValueError:
                docs = None
                break
        if docs is None:
            if rng:
                sys.exit(f'{path}: plain ids carry no cursor; --from-cursor / --to-cursor needs /v1/changes events')
            return sorted({int(x) for x in text.replace(',', ' ').split() if x.strip().isdigit()}), None
    ids = {int(d) for d in docs if isinstance(d, int)}
    events, heads, gens = [], [], set()
    for d in (d for d in docs if not isinstance(d, int)):
        pages = d if isinstance(d, list) and d and isinstance(d[0], dict) and 'events' in d[0] else [d]
        for page in pages:
            if isinstance(page, list):
                ids |= {int(x) for x in page if not isinstance(x, dict)}
                events += [e for e in page if isinstance(e, dict) and 'entity' in e]
            elif isinstance(page, dict) and 'events' in page:
                events += page['events']
                if page.get('last_cursor') is not None:
                    heads.append(page['last_cursor'])
                if page.get('generation') is not None:
                    gens.add(page['generation'])
            elif isinstance(page, dict) and 'entity' in page:
                events.append(page)
            elif isinstance(page, dict):
                ids |= {int(x) for x in page.get('ids') or page.get('changed_null_version_ids') or []}
    rec = None
    if rng:
        lo, hi = rng
        if ids:
            sys.exit(f'{path}: {len(ids)} plain id(s) carry no cursor; --from-cursor / --to-cursor needs /v1/changes '
                     'events only')
        if any(e.get('cursor') is None for e in events):
            sys.exit(f"{path}: {sum(1 for e in events if e.get('cursor') is None)} event(s) carry no cursor")
        if len(gens) > 1:
            sys.exit(f'{path}: pages from {len(gens)} feed generations {sorted(gens)}; their cursors do not compose')
        reach = max([cursor_pos(e['cursor']) for e in events] + [cursor_pos(h) for h in heads], default=None)
        if reach is None or reach < hi:
            sys.exit(f'{path}: the events reach cursor {reach}, short of --to-cursor {hi}: the changed set would be '
                     'truncated')
        inside = [e for e in events if lo < cursor_pos(e['cursor']) <= hi]
        cur = [cursor_pos(e['cursor']) for e in inside]
        rec = {'from': str(lo), 'to': str(hi), 'rule': 'from < cursor <= to', 'events_read': len(events),
               'events_in_range': len(inside), 'events_outside_range': len(events) - len(inside),
               'first_in_range': min(cur, default=None), 'last_in_range': max(cur, default=None),
               'read_reaches': reach, 'generations': sorted(gens)}
        events = inside
    ids |= {int(e['id']) for e in events if e.get('entity') == 'tender'
            and (any_version or (e.get('op') == 'changed' and e.get('version') is None))}
    return sorted(ids), rec


def exclusions():
    ex = {}
    for g in ld(os.path.join(ROOT, 'readback', 'u7_sample.json'))['groups']:
        for i in g['ids']:
            ex.setdefault(i, 's150')
    for x in ld(os.path.join(ROOT, 'readback', 'fresh30.json'))['draw']:
        ex.setdefault(x['id'], 'f30')
    for i in ld(os.path.join(ROOT, 'readback', 'measured77.json'))['served']:
        ex.setdefault(int(i), 'm77')
    for t in ld(os.path.join(ROOT, 'measurement.json'))['tenders']:
        ex.setdefault(t['tender_id'], 'm77')
    ex.setdefault(8327340, '8327340')
    for g in ld(os.path.join(ROOT, 'u8-oos', 'draw.json'))['groups']:
        for i in g['ids']:
            ex.setdefault(i, 'oos50')
    return ex


def old_signal(tender):
    """The ORIGINAL one-read signal (O7: imprecise on multi-CAN R2.0.9 chains). Recorded, decides nothing."""
    hits = []
    for r in tender.get('lot_results', []):
        if r.get('lot') is None:
            continue
        ws = {w.get('organization_id') for w in r.get('winners') or [] if w.get('organization_id') is not None}
        if not ws:
            continue
        scopes = {p.get('lot') for p in tender.get('parties', []) if p.get('role') == 'winner'
                  and p.get('organization_id') in ws}
        if None in scopes and r['lot'] not in scopes:
            hits.append(f"{r['notice_id']}/{r['key']}->{r['lot']}")
    return hits


def listing(f, tid):
    """{notice id: profile} from /v1/notices?tender={id}, every page followed; None when a page is not read."""
    prof, page, cursor = {}, 0, None
    while True:
        name = f'notices_tender_{tid}.json' if page == 0 else f'notices_tender_{tid}_p{page}.json'
        path = f'/v1/notices?tender={tid}' + ('' if cursor is None else f'&cursor={cursor}')
        out = os.path.join(f.cache, name)
        if not f._get(path, out):
            return None
        d = ld(out)
        for x in d.get('items', []):
            prof[int(x['id'])] = x.get('profile')
        if not d.get('more'):
            return prof
        cursor, page = d.get('next_cursor'), page + 1
        if cursor is None:
            return None


def corrected_g2(f, tender):
    """(True | False | None, evidence). None: a needed read failed (the walk stops). See the module doc, change 1."""
    placed = collections.Counter(r['notice_id'] for r in tender.get('lot_results', []) if r.get('lot') is not None)
    ev = {'lot_placed_results': sum(placed.values()), 'lot_placed_notices': len(placed)}
    if not placed:
        ev['why'] = 'no lot-placed result'
        return False, ev
    prof = listing(f, tender['id'])
    if prof is None:
        ev['why'] = 'profile listing not read'
        return None, ev
    ev['lot_placed_by_profile'] = dict(collections.Counter(str(prof.get(n)) for n in placed for _ in range(placed[n])))
    unlisted = sorted(n for n in placed if n not in prof)
    if unlisted:
        ev['lot_placed_notices_not_in_listing'] = unlisted
    r208 = sorted(n for n in placed if prof.get(n) == R208)
    ev['r208_lot_placed_notices'] = r208
    for n in r208:
        c = f.notice(n)
        if c is None:
            ev['why'] = f'content of {n} not read'
            return None, ev
        own = [s['section_id'] for s in c.get('sections', []) if s.get('kind') in LOT_KINDS]
        if not own:
            ev.update(qualifying_notice=n, qualifying_notice_lot_placed_results=placed[n],
                      qualifying_notice_lot_sections=0, why=f'{n}: {R208}, no lot section of its own')
            return True, ev
        ev.setdefault('r208_notices_with_own_lot_sections', {})[str(n)] = len(own)
    ev['why'] = ('no lot-placed result from a ted-export-r208 notice' if not r208 else
                 'every lot-placed ted-export-r208 notice has lot sections of its own')
    return False, ev


def parse_spec(spec):
    out = []
    for part in (spec or '').split(','):
        part = part.strip()
        if not part:
            continue
        if '-' in part:
            a, b = part.split('-')
            out += list(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return out


def main():
    args = sys.argv[1:]

    def opt(name, default=None):
        if name in args:
            i = args.index(name)
            v = args[i + 1]
            del args[i:i + 2]
            return v
        return default
    changed, cache, out = opt('--changed'), opt('--cache'), opt('--out')
    c_from, c_to = opt('--from-cursor'), opt('--to-cursor')
    resume, keep_spec, keep_g2_spec, from_index = opt('--resume'), opt('--keep'), opt('--keep-g2'), opt('--from-index')
    max_reads = int(opt('--max-reads', '1000000'))
    gap = float(opt('--gap', '1.6'))
    offline = '--offline' in args
    any_version = '--any-version' in args
    if not (changed and cache and out) or (c_from is None) != (c_to is None):
        sys.exit(__doc__)
    if (resume is None) != (from_index is None) or (resume and keep_spec is None):
        sys.exit('--resume, --keep and --from-index go together')
    rng = (cursor_pos(c_from), cursor_pos(c_to)) if c_from is not None else None
    if rng and rng[0] >= rng[1]:
        sys.exit(f'--from-cursor {c_from} is not before --to-cursor {c_to}')
    os.makedirs(cache, exist_ok=True)
    population = set(ld(os.path.join(ROOT, 'readback', 'population.json'))['changed_null_version_ids'])
    ex = exclusions()
    ids, cursor_range = read_changed(changed, any_version, rng)
    excluded = {i: ex[i] for i in ids if i in ex}
    order = sorted((i for i in ids if i not in ex), key=salted)
    f = CL.Fetcher(cache, gap, offline, [])
    held, walked, stop, not_found = [], [], None, []
    kept_rows, start = [], 0
    n_out = n_g2 = 0
    resumed = None
    if resume:
        prev = ld(resume)
        start = int(from_index)
        pw = prev['walked']
        if len(pw) < start or [w['id'] for w in pw[:start]] != order[:start]:
            sys.exit(f'{resume}: its walk does not equal the recomputed salted order on indices 0..{start - 1}')
        if prev.get('cursor_range') != cursor_range:
            sys.exit(f'{resume}: cursor_range differs from this read of --changed')
        keep, keep_g2 = parse_spec(keep_spec), set(parse_spec(keep_g2_spec))
        if not keep_g2 <= set(keep):
            sys.exit('--keep-g2 names a row --keep does not')
        for k in keep:
            w = pw[k]
            if not w.get('held'):
                sys.exit(f'{resume}: walk row {k} ({w["id"]}) was not held')
            g2 = k in keep_g2
            kept_rows.append({'index': k, 'id': w['id'], 'year': w['year'], 'out': w['out'], 'g2': g2,
                              'g2_original_tool': w.get('g2')})
            held.append(w['id'])
            n_out += bool(w['out'])
            n_g2 += g2
        dropped = [{'index': i, 'id': w['id'], 'g2_original_tool': w.get('g2')} for i, w in enumerate(pw[:start])
                   if w.get('held') and i not in set(keep)]
        resumed = {'from': os.path.abspath(resume), 'walk_rows_checked_equal': start, 'kept_indices': keep,
                   'kept_g2_indices': sorted(keep_g2), 'dropped_held_rows': dropped,
                   'seed': {'held': len(held), 'out': n_out, 'g2': n_g2}}
    for idx in range(start, len(order)):
        tid = order[idx]
        if len(held) == N:
            break
        need_out, need_g2 = max(0, N_OUT - n_out), max(0, N_G2 - n_g2)
        free_after = N - len(held) - 1
        inside = tid in population
        row = {'index': idx, 'id': tid, 'salt': salted(tid), 'in_population': inside}
        # Unread skip: only a G2 Tender could still be held (a non-G2 one, even OUT, leaves too few slots), and this
        # one is inside population.json, so it cannot be G2.
        if inside and free_after < need_g2 + max(0, need_out - 1):
            row.update(held=False, why='skipped unread: the free slots need a G2 Tender, and it is inside population.json')
            walked.append(row)
            continue
        if f.gets >= max_reads and not os.path.exists(os.path.join(cache, f'tenders_{tid}.json')):
            stop = f'max reads ({max_reads}) reached before {tid}'
            break
        t = f.tender(tid)
        if t is None and f.last_code == '404':
            not_found.append(tid)
            row.update(held=False, why='skipped: HTTP 404, the Tender is not served (recorded in not_found_404)')
            walked.append(row)
            continue
        if t is None:
            stop = f'could not read {tid}: {f.errors[-1] if f.errors else "?"}'
            break
        pub = str(t.get('published_at') or '')
        year = int(pub[:4]) if pub[:4].isdigit() else None
        out_era = year is not None and not (2012 <= year <= 2015)
        if inside:
            g2, ev = False, {'why': 'inside population.json'}
        else:
            g2, ev = corrected_g2(f, t)
            if g2 is None:
                stop = f'could not finish the G2 test of {tid}: {ev.get("why")}; {f.errors[-1] if f.errors else ""}'
                break
        old = old_signal(t)
        row.update(year=year, out=out_era, g2=g2, g2_evidence=ev, old_signal_hits=len(old))
        after_out, after_g2 = n_out + out_era, n_g2 + g2
        if free_after >= max(0, N_OUT - after_out) + max(0, N_G2 - after_g2):
            held.append(tid)
            n_out, n_g2 = after_out, after_g2
            row.update(held=True, why='held')
        else:
            row.update(held=False, why=f'skipped: holding it leaves {free_after} slots for '
                                       f'{max(0, N_OUT - after_out)} OUT + {max(0, N_G2 - after_g2)} G2 still needed')
        walked.append(row)
    new_rows = [{'index': w['index'], 'id': w['id'], 'year': w['year'], 'out': w['out'], 'g2': w['g2']}
                for w in walked if w.get('held')]
    result = {
        'salt': f'sha256("{SALT}"+id) ascending',
        'rule': 'decision-u7.md "Amendment (unit 8, 2026-10-18)", "The read-back bar", "The draw"; '
                f'{N} held, >= {N_OUT} OUT (published_at year outside 2012-2015), >= {N_G2} G2 (outside '
                'readback/population.json and serving an R2.0.8 block on a lot), the G2 test CORRECTED per '
                'readback.md O1/O7: a lot-placed result from a ted-export-r208 notice (/v1/notices?tender=) with no '
                'lot section of its own (/v1/notices/{id}/content)',
        'tool': os.path.abspath(__file__),
        'changed_file': os.path.abspath(changed), 'changed_tenders': len(ids),
        'version_filter': 'any' if any_version else 'op=changed, version=null (events input only)',
        'cursor_range': cursor_range,
        'excluded': {'n': len(excluded), 'by_set': {s: sum(1 for v in excluded.values() if v == s)
                                                     for s in sorted(set(excluded.values()))}},
        'candidates': len(order),
        'resumed': resumed,
        'draw': kept_rows + new_rows,
        'held': len(held), 'out': n_out, 'g2': n_g2,
        'complete': len(held) == N and n_out >= N_OUT and n_g2 >= N_G2,
        'stopped': stop, 'not_found_404': not_found, 'gets': f.gets, 'fetch_errors': f.errors,
        'walked_from_index': start, 'walked_to_index': walked[-1]['index'] if walked else None,
        'walked': walked,
    }
    with open(out, 'w', encoding='utf-8') as fh:
        json.dump(result, fh, ensure_ascii=False, indent=1)
    print(json.dumps({k: v for k, v in result.items() if k not in ('walked', 'draw', 'resumed')}, ensure_ascii=False))
    print(json.dumps([{k: v for k, v in r.items()} for r in result['draw']]))


if __name__ == '__main__':
    main()
