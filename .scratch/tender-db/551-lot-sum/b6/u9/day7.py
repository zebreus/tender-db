"""Issue 551, path W on: the U9 day-7 read set builder (decision-b6.md U9 + C8; decision-w-on-drains.md H with W4).

    python3 -I -B day7.py build   [--out u9/day7] [--allow-not-chain]
    python3 -I -B day7.py fetched --ref "<551 entry>" [--out u9/day7] [--fetch-manifest FILE]

Run on 2026-10-28 after that morning's U9 check (gd5 class chain) PASSED and was advanced (so every R row is the one
read that morning), before the predraw, the draw and the fetch; no writer runs until the fetch is done (B1).
Two populations, each written in u8/delta.py's population shape and drawn with E5 (delta.py --population/--salt/--out):
  C8 (as pre-registered, NOT amended): heads whose first lot-sum-awards write came from a daily fold, not in P = the
     registry entries tagged chain-<date> (days 1-2: 97901, 8859070, 8859315; later chains: their counted daily fires).
     Drain and probe fires (drain-*, probe-*) and the U8 delta (lc runs) are never in it (H). A chain fire that has
     since stopped serving lot-sum-awards (registry `lost`) is listed, not drawn. Salt '551-b6-daily-' (U9).
  chain-MOVED: R heads a chain re-decided (a counted chain-MOVED move with a tender row, W4), read as a separate
     population under the same bar, named in the predraw. Salt '551-b6-daily-moved-' (record it in 551 before the
     predraw: C8 did not name one). chain-LOST heads are listed beside it (they no longer serve a derived sum).
Daily fires on DÖE Tenders registered before ALL THREE 539 step-2 runs (drain-539-a/b/c) advanced carry the flag
"possibly 539-driven" (A6/H; step 1, drain-539-1, is class Z and re-sorts no fold); gd5 stores the flag per entry and
the readers see it. day7.json names step2_done_segment (the segment that completed the step-2 set, or None with a
note). Writes <out>/population_c8.json, <out>/population_moved.json (each with its salt), <out>/day7.json and prints
the E5 commands.
fetched (offline, after both fetches) writes <out>/fetched.json -- the marker segment.py open needs on 10-28 (B1 / F
HOLD: no writer until the day-7 bodies are fetched): for c8 and moved, the delta_fetch_ids.json sha256 and every id
fetched in fetch_b6's manifest (scratchpad b6/fetch_manifest.json), plus day7.json's sha256 and the 551 ref. Refused
(exit 2) while any id is not fetched (its tender not 200 in the manifest, or notices still missing for it).
Exit 0; 2 refused (the registry's last segment is not a chain, unless --allow-not-chain; or it is frozen/open).
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wkit as W  # noqa: E402
import registry as REG  # noqa: E402

K = W.K
SALT_C8 = '551-b6-daily-'
SALT_MOVED = '551-b6-daily-moved-'


def population(reg, ids, what, salt):
    ids = sorted(ids)
    rows = {str(i): reg['entries'][str(i)]['row'] for i in ids}
    return {'what': what, 'at': K.now_iso(), 'ids': ids, 'n': len(ids), 'ids_sha256': K.ids_sha(ids), 'salt': salt,
            'band_rows': [i for i in ids if W.eur_ge_100m(rows[str(i)])], 'rows': rows,
            'tags': {str(i): reg['entries'][str(i)]['tag'] for i in ids},
            'flags': {str(i): reg['entries'][str(i)].get('flags', []) for i in ids
                      if reg['entries'][str(i)].get('flags')}, 'registry_K': reg['K'],
            'registry_entries_sha256': reg.get('entries_sha256')}


def cmd_build(a):
    reg = REG.load()
    if reg.get('frozen') or reg.get('open_segment'):
        print(f'REFUSED: the registry is frozen ({reg.get("frozen")}) or a segment is open ({reg.get("open_segment")})')
        return 2
    last = reg['history'][-1] if reg['history'] else None
    if not a.allow_not_chain and (last is None or last['class'] != 'chain'):
        print(f'REFUSED: the registry\'s last segment is {last and last["seg"]} ({last and last["class"]}), not the '
              'day-7 chain check (run gd5 class chain + registry advance first)')
        return 2
    c8 = sorted(int(i) for i, e in reg['entries'].items() if e['tag'].startswith('chain-'))
    lost_c8 = sorted(int(i) for i, e in reg['lost'].items() if e['tag'].startswith('chain-'))
    moved = sorted(int(i) for i, e in reg['entries'].items()
                   if any(m['kind'].startswith('chain-') for m in e.get('moves', [])))
    lost_chain = sorted(int(i) for i, e in reg['lost'].items() if str(e.get('lost_tag', '')).startswith('chain-'))
    step2_done, seen = None, set()
    for h in reg['history']:
        seen.add(h['tag'])
        if step2_done is None and set(W.STEP2_TAGS) <= seen:
            step2_done = h['seg']
    flagged = sorted(int(i) for i in c8 if reg['entries'][str(i)].get('flags'))
    os.makedirs(a.out, exist_ok=True)
    pc8 = population(reg, c8, 'issue 551 U9 day 7: C8 population (first lot-sum-awards write from a daily fold, not in P)',
                     SALT_C8)
    pmv = population(reg, moved, 'issue 551 U9 day 7: chain-MOVED R heads (W4: a separate population, same bar)',
                     SALT_MOVED)
    K.jdump(os.path.join(a.out, 'population_c8.json'), pc8)
    K.jdump(os.path.join(a.out, 'population_moved.json'), pmv)
    summary = {'at': K.now_iso(), 'registry_K': reg['K'], 'last_segment': last, 'c8': {'n': len(c8), 'ids': c8,
               'ids_sha256': pc8['ids_sha256'], 'by_tag': W.counts(pc8['tags'].values()), 'band_rows': pc8['band_rows'],
               'lost_since': lost_c8, 'flagged_539': flagged, 'salt': SALT_C8},
               'chain_moved': {'n': len(moved), 'ids': moved, 'ids_sha256': pmv['ids_sha256'],
                               'band_rows': pmv['band_rows'], 'salt': SALT_MOVED},
               'chain_lost': lost_chain, 'step2_done_segment': step2_done,
               'step2_note': None if step2_done else ('539 step 2 is not complete on day 7 (drain-539-a/b/c not all '
                                                      'advanced): DÖE daily fires stay flagged possibly 539-driven')}
    K.jdump(os.path.join(a.out, 'day7.json'), summary)
    print(f'C8 population {len(c8)} {summary["c8"]["by_tag"]} (sha256 {pc8["ids_sha256"][:16]}); band rows '
          f'{pc8["band_rows"]}; possibly 539-driven {flagged}; lost since {lost_c8}')
    print(f'chain-MOVED population {len(moved)} (sha256 {pmv["ids_sha256"][:16]}); band rows {pmv["band_rows"]}; '
          f'chain-LOST (listed) {lost_chain}')
    print('E5 (record each predraw in 551 BEFORE its draw):')
    for nm, pop, salt in (('c8', 'population_c8.json', SALT_C8), ('moved', 'population_moved.json', SALT_MOVED)):
        print(f'  python3 -I -B u8/delta.py predraw --population {a.out}/{pop} --salt {salt} --out {a.out}/{nm}')
        print(f'  python3 -I -B u8/delta.py draw    --population {a.out}/{pop} --salt {salt} --out {a.out}/{nm}')
    return 0


def cmd_fetched(a):
    d7 = os.path.join(a.out, 'day7.json')
    if not os.path.exists(d7):
        print(f'REFUSED: {d7} missing (day7.py build first)')
        return 2
    fm = K.jload(a.fetch_manifest) if os.path.exists(a.fetch_manifest) else {}
    rec = {'what': 'issue 551 U9 day 7: the fetch is done (segment.py open waits for this on 10-28)', 'at': K.now_iso(),
           'ref': a.ref, 'day7_sha256': K.sha_file(d7), 'parts': {}, 'complete': True}
    problems = []
    for nm in ('c8', 'moved'):
        fp = os.path.join(a.out, nm, 'delta_fetch_ids.json')
        if not os.path.exists(fp):
            problems.append(f'{fp} missing (predraw / draw first)')
            continue
        ids = sorted(int(i) for i in K.jload(fp)['ids'])
        tn = fm.get('tender_notices') or {}
        left = [i for i in ids if (fm.get('tenders') or {}).get(str(i), {}).get('status') != 200
                or (tn.get(str(i), {}).get('missing_now') not in ([],) and str(i) in tn)]
        rec['parts'][nm] = {'ids_file': fp, 'ids_sha256': K.sha_file(fp), 'n': len(ids), 'fetched': len(ids) - len(left),
                            'fetch_manifest': a.fetch_manifest}
        if left:
            problems.append(f'{nm}: {len(left)} id(s) not fetched yet {left[:10]} (fetch_b6 --ids {fp})')
    if problems:
        for pr in problems:
            print(f'REFUSED: {pr}')
        return 2
    K.jdump(os.path.join(a.out, 'fetched.json'), rec)
    print(f'day-7 fetch recorded: {rec["parts"]} ({a.ref}) -> {a.out}/fetched.json; writers may open again')
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    b = sub.add_parser('build')
    b.add_argument('--out', default=os.path.join(W.U9, 'day7'))
    b.add_argument('--allow-not-chain', action='store_true')
    f = sub.add_parser('fetched')
    f.add_argument('--out', default=os.path.join(W.U9, 'day7'))
    f.add_argument('--ref', required=True)
    f.add_argument('--fetch-manifest', default=os.path.join(K.SP, 'b6', 'fetch_manifest.json'))
    a = ap.parse_args(argv)
    return cmd_build(a) if a.cmd == 'build' else cmd_fetched(a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
