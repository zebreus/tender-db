"""570 read-back (e): a fresh 30 of remap-touched corrected Tenders from the combined refold (2616 + 617).

decision.md (570-lot-keys) "Units" 6 (e), as the read-back workflow's stage "fresh570" states it:
  - population: the corrected Tenders = /v1/changes?entity=tender events with op=changed, version=null and
    957854566 < cursor <= 958764334 (draw_fresh30.read_changed, the verified reader, over the saved pages);
  - order: sha256("570-fresh30-" + id) hex digest ascending, written to ORDER before any Tender is read; no exclusion;
  - the walk: read /v1/tenders/{id} and every chain notice's /v1/notices/{id}/content (fresh, after the drain; one GET
    per 1.6 s, a 408 retried once, via check_live.Fetcher); the Tender is HELD when combined_port.remap_chain with the
    remap on moves at least one lot section of a chain notice (a "lot section remapped by 570"), and skipped and
    recorded otherwise; a chain of one notice cannot be remapped (no earlier lot-publishing notice: remap_port.remap's
    'anchor' with no carried lot) and is skipped without reading its notice. The walk stops at 30 held.

Usage:
  python3 -I -B walk570.py order PAGES_DIR ORDER.json
  python3 -I -B walk570.py walk ORDER.json CACHE_DIR WALK.json [--max-reads N]
"""
import hashlib
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
U8 = os.path.dirname(HERE)  # readback-u8
sys.path.insert(0, U8)
import combined_port as C   # noqa: E402
import check_live as CL     # noqa: E402
import draw_fresh30 as DF   # noqa: E402

SALT = '570-fresh30-'
A, B = 957854566, 958764334
N = 30


def salted(i):
    return hashlib.sha256((SALT + str(i)).encode()).hexdigest()


def order(pages_dir, out):
    pages = []
    for f in sorted(os.listdir(pages_dir)):
        if f.startswith('p') and f.endswith('.json'):
            pages.append(json.load(open(os.path.join(pages_dir, f))))
    combined = os.path.join(pages_dir, 'all_pages.json')
    with open(combined, 'w') as fh:
        json.dump(pages, fh)
    ids, rng = DF.read_changed(combined, False, (A, B))
    # independent recount (op / version / entity), never trusting one reader alone
    ev = [e for p in pages for e in p['events'] if A < int(e['cursor']) <= B]
    ops = {}
    for e in ev:
        k = f"{e['entity']}/{e['op']}/{'null' if e['version'] is None else 'v'}"
        ops[k] = ops.get(k, 0) + 1
    ids2 = sorted({int(e['id']) for e in ev if e['entity'] == 'tender' and e['op'] == 'changed' and e['version'] is None})
    assert ids == ids2, 'read_changed and the recount disagree'
    dup = len(ev) - len({(e['entity'], e['id']) for e in ev})
    od = sorted(ids, key=salted)
    res = {'salt': f'sha256("{SALT}"+id) hex ascending', 'pages': len(pages), 'cursor_range': rng,
           'events_by_kind': ops, 'duplicate_entity_ids_in_range': dup, 'population': len(ids),
           'order': [{'id': i, 'salt': salted(i)} for i in od]}
    with open(out, 'w') as fh:
        json.dump(res, fh, indent=0)
    print(json.dumps({k: v for k, v in res.items() if k != 'order'}))
    print('first 40:', [x['id'] for x in res['order'][:40]])


def touched(tender, contents):
    """(held?, per-notice remap outcomes, moved sections) from combined_port.remap_chain (remap on, K3 included)."""
    chain = C.chain_of(tender)
    maps = C.remap_chain(chain, contents, True)
    hows, moved = {}, {}
    for n, (tg, how) in maps.items():
        if n in contents and C.R.lot_sections(contents[n]):
            hows[str(n)] = how
        mv = {k: v for k, v in tg.items() if k != v}
        if mv:
            moved[str(n)] = mv
    return bool(moved), hows, moved


def walk(order_path, cache, out, max_reads):
    od = [x['id'] for x in json.load(open(order_path))['order']]
    f = CL.Fetcher(cache, 1.6, False, [])
    held, walked, stop, nf = [], [], None, []
    for tid in od:
        if len(held) == N:
            break
        if f.gets >= max_reads:
            stop = f'max reads {max_reads} reached before {tid}'
            break
        row = {'id': tid, 'salt': salted(tid)}
        t = f.tender(tid)
        if t is None:
            if f.last_code == '404':
                nf.append(tid)
                row.update(held=False, why='HTTP 404: Tender not served')
                walked.append(row)
                continue
            stop = f'tender {tid}: {f.errors[-1] if f.errors else "?"}'
            break
        chain = C.chain_of(t)
        row.update(source=t.get('source'), published_at=t.get('published_at'), chain=chain,
                   lots=len(t.get('lot_details') or []), lot_results=len(t.get('lot_results') or []))
        if len(set(chain)) < 2:
            row.update(held=False, why='single-notice chain: no earlier lot-publishing notice, 570 cannot remap')
            walked.append(row)
            continue
        contents, missing = {}, []
        for n in chain:
            c = f.notice(n)
            if c is None:
                missing.append((n, f.last_code))
            else:
                contents[n] = c
        if missing:
            if all(code == '404' for _, code in missing):
                row.update(held=False, why=f'undetermined: chain notices 404 {missing}')
                walked.append(row)
                continue
            stop = f'tender {tid}: chain notices not read {missing}: {f.errors[-3:]}'
            break
        is_t, hows, moved = touched(t, contents)
        row.update(legacy_notices=[n for n in chain if C.is_legacy(contents[n])], notice_remap=hows, moved=moved)
        if is_t:
            held.append(tid)
            row.update(held=True, why='held: a lot section remapped by 570')
        else:
            row.update(held=False, why='not touched: no lot section moved by the remap')
        walked.append(row)
        print(f"{len(walked):4d} {tid} chain={len(chain)} {'HELD' if is_t else 'skip'} {row['why']}"
              f" {hows if hows else ''} gets={f.gets}", flush=True)
    res = {'rule': 'walk570.py docstring', 'walked': len(walked), 'held': len(held), 'complete': len(held) == N,
           'stopped': stop, 'not_found_404': nf, 'gets': f.gets, 'fetch_errors': f.errors,
           'held_ids': held, 'walk': walked}
    with open(out, 'w') as fh:
        json.dump(res, fh, indent=1)
    print(json.dumps({k: v for k, v in res.items() if k != 'walk'}))


if __name__ == '__main__':
    a = sys.argv[1:]
    mr = 10 ** 9
    if '--max-reads' in a:
        i = a.index('--max-reads')
        mr = int(a[i + 1])
        del a[i:i + 2]
    if a[0] == 'order':
        order(a[1], a[2])
    elif a[0] == 'walk':
        walk(a[1], a[2], a[3], mr)
    else:
        sys.exit(__doc__)
