"""570 read-back (g): `lot removed` rows in the combined refold's window, and which of them 570 caused (expected 0).

Reads the saved /v1/changes?entity=lot pages (page_changes.py lot ...), keeps 957854566 < cursor <= 958764334 (one
feed generation, the read reaching 958764334), and counts the events by op and version. /v1/changes has no op filter
(crates/app/src/v1/mod.rs `changes`: only since / limit / entity), so every lot event of the window is paged.

Also cross-checks the held 30 of (e): each held Tender's served lot ids (lot_details[].id) against the window's lot
events (a remap-touched Tender must show no lot added or removed; its head lots are announced `changed`, version null).

For each removed lot (if any): its Tender is attributed by cursor adjacency to the window's tender events (D3 writes a
Tender's rows together; the convention is measured on the held 30, whose lot ids are known), and 570 caused the
removal only if the remap-on replay of that Tender's chain lacks a key the remap-off replay has (a key B swept).

Usage: python3 -I -B g570.py LOT_PAGES_DIR TENDER_PAGES_DIR CACHE_DIR WALK.json OUT.json
"""
import bisect
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
A, B = 957854566, 958764334


def pages(d):
    out = []
    for f in sorted(os.listdir(d)):
        if f.startswith('p') and f.endswith('.json'):
            out.append(json.load(open(os.path.join(d, f))))
    return out


def window(ps, entity):
    gens = {p.get('generation') for p in ps}
    reach = max([int(p['last_cursor']) for p in ps if p.get('last_cursor') is not None] +
                [int(e['cursor']) for p in ps for e in p['events']])
    assert len(gens) == 1, gens
    assert reach >= B, f'{entity} pages reach {reach} < {B}'
    ev = [e for p in ps for e in p['events'] if A < int(e['cursor']) <= B]
    assert all(e['entity'] == entity for e in ev)
    return ev, {'pages': len(ps), 'generation': sorted(gens), 'reach': reach,
                'events_read': sum(len(p['events']) for p in ps), 'events_in_window': len(ev)}


def main():
    lot_dir, tender_dir, cache, walk_path, out = sys.argv[1:6]
    lev, lrec = window(pages(lot_dir), 'lot')
    tev, trec = window(pages(tender_dir), 'tender')
    by = {}
    for e in lev:
        k = f"{e['op']}/{'null' if e['version'] is None else 'versioned'}"
        by[k] = by.get(k, 0) + 1
    removed = [e for e in lev if e['op'] == 'removed']
    added = [e for e in lev if e['op'] == 'added']
    lot_ids = {}
    for e in lev:
        lot_ids.setdefault(int(e['id']), []).append(e)
    dup = sum(1 for v in lot_ids.values() if len(v) > 1)
    # adjacency convention, measured on the held 30
    tcur = sorted((int(e['cursor']), int(e['id'])) for e in tev)
    tpos = [c for c, _ in tcur]
    w = json.load(open(walk_path))
    held = []
    conv = {'lot_after_its_tender': 0, 'lot_before_its_tender': 0, 'other': 0}
    for tid in w['held_ids']:
        t = json.load(open(os.path.join(cache, f'tenders_{tid}.json')))
        ids = [int(l['id']) for l in t.get('lot_details') or []]
        tc = [c for c, i in tcur if i == tid]
        evs = {i: lot_ids.get(i, []) for i in ids}
        ops = {}
        for i, es in evs.items():
            for e in es:
                ops[e['op']] = ops.get(e['op'], 0) + 1
            if not es:
                ops['no-event'] = ops.get('no-event', 0) + 1
        for i, es in evs.items():
            for e in es:
                c = int(e['cursor'])
                j = bisect.bisect_left(tpos, c)
                prev_t = tcur[j - 1][1] if j > 0 else None
                nxt_t = tcur[j][1] if j < len(tcur) else None
                if prev_t == tid:
                    conv['lot_after_its_tender'] += 1
                elif nxt_t == tid:
                    conv['lot_before_its_tender'] += 1
                else:
                    conv['other'] += 1
        held.append({'tender_id': tid, 'tender_events': tc, 'served_lots': len(ids), 'lot_event_ops': ops,
                     'all_changed_null': all(len(es) == 1 and es[0]['op'] == 'changed' and es[0]['version'] is None
                                             for es in evs.values())})
    attributed = []
    for e in removed + added:
        c = int(e['cursor'])
        j = bisect.bisect_left(tpos, c)
        attributed.append({'event': e, 'prev_tender_event': tcur[j - 1] if j > 0 else None,
                           'next_tender_event': tcur[j] if j < len(tcur) else None})
    res = {'window': {'from': A, 'to': B, 'rule': 'from < cursor <= to'}, 'lot_pages': lrec, 'tender_pages': trec,
           'lot_events_by_op_version': by, 'removed': len(removed), 'added': len(added),
           'distinct_lot_ids': len(lot_ids), 'lot_ids_with_two_or_more_events': dup,
           'adjacency_convention_on_held30': conv, 'held30_lot_events': held,
           'removed_and_added_events': attributed}
    with open(out, 'w') as fh:
        json.dump(res, fh, indent=1)
    print(json.dumps({k: v for k, v in res.items() if k not in ('held30_lot_events', 'removed_and_added_events')}))
    print('held30 all changed/null:', sum(h['all_changed_null'] for h in held), 'of', len(held))
    for h in held:
        if not h['all_changed_null']:
            print('  ', h)
    for a in attributed[:50]:
        print('  ', a)


if __name__ == '__main__':
    main()
