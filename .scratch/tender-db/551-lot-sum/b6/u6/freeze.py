"""Issue 551 B6 U6 step 3: freeze the prediction (decision U6 "Record"; C4/C6; B3; D-rules' inputs).

    python3 -I -B freeze.py [--pfinal u6/P_final.json] [--lchanged u6/l_changed.json] [--out u6/freeze.json]

Writes u6/freeze.json and two id files:
  - P: the final P's n, ids_sha256 and the sha256 of P_final.json (per id S, currency, n, S_eur_cents, vouched_by,
    vouches live there); Rg = the 3,502 minus P (D1: must compare identical);
  - B_P: P ids whose port EUR value S_eur_cents >= EUR 100 m (D4's expected band set), and every P id within 2 % of
    EUR 100 m (D4: a difference on one of those is stop-and-explain, not a switch-off), plus U5's watch id 899272;
  - band_reread (point 8 / B3 / B-band): any of U2's 29 band heads (the 29 already read) whose predicted S, n or
    vouched_by changed or that left P, AND every head of the final B_P that is not among the 29 (a new band head: an
    L_changed ∩ 3,502 refetch that raised S, or the refreshed rates -- 899272 sits at ~EUR 98.95 m). Each must read 0
    wrong of any class BEFORE the flip deploy; a non-empty list exits 2 with READ lines;
  - pins_expected (D3): 585917 EUR 250,000.00 / 5995030 PLN 1,549,511.60 / 8426318 GBP 3,000,000,000.00 as
    lot-sum-awards with the port's vouched_by (accepted: any of the port's G-filtered vouches, A4); the Lyon-Turin
    trio 6450467/69/73 EUR 1,660,661,682.94 with no derivation; the 16 path-V pins byte-identical to the pre snapshot
    and equal to their recorded sums (lot-sum-vouched, rule 537 / 928196 rule 492); the 9 must-stay heads with no
    derivation and their pre-snapshot EUR value; 6465510 not lot-sum-awards. A pin expectation that the port itself
    contradicts (a path-W pin not in P with that S/currency) is a STOP before any deploy;
  - L = the 3,502 ∪ L_changed (u6/L_ids.json) in runs of <= 5,000; the snapshot list (u6/snapshot_ids.json) = L ∪ the
    pins ∪ 7511890 ∪ the 61 (stage 1 names them all).
Exit 0; 2 STOP (a STOP line) or READ (band heads to read before the deploy; freeze.json is written either way).
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402

BAND9 = [6718092, 8407303, 7167278, 8460630, 6548237, 5769515, 7461921, 7196993, 6435162]  # B5's band9 (read 9/9)


def build(pf, lc, the3502):
    stops = []
    if pf.get('stops') or pf.get('partial') or pf.get('errors'):
        stops.append('P_final.json is not clean (stops / partial / errors)')
    ships = {int(e['id']): e for e in pf['ships']}
    if K.ids_sha(ships) != pf['ids_sha256']:
        stops.append('P_final ids_sha256 does not match its ships')
    ids3502 = set(the3502)
    if not set(ships) <= ids3502:
        stops.append('P_final holds ids outside the 3,502')
    Rg = sorted(ids3502 - set(ships))
    if Rg != sorted(pf['Rg']):
        stops.append('Rg != the 3,502 minus P')
    bp = sorted(t for t, e in ships.items() if e['S_eur_cents'] is not None and e['S_eur_cents'] >= K.EUR100M)
    near = sorted(t for t, e in ships.items() if e['S_eur_cents'] is not None
                  and abs(e['S_eur_cents'] - K.EUR100M) <= 0.02 * K.EUR100M)
    u2P = K.p_u2()
    u2_bp = sorted(t for t, e in u2P.items() if e['S_eur_cents'] is not None and e['S_eur_cents'] >= K.EUR100M)
    # U2/U5: B_P was exactly the 29 read band heads (read18's 20 + band9) under every guard set
    assert len(u2_bp) == 29 and set(BAND9) <= set(u2_bp), (len(u2_bp), sorted(set(BAND9) - set(u2_bp)))
    reread = []
    for t in u2_bp:
        e, u = ships.get(t), u2P[t]
        if e is None:
            reread.append({'id': t, 'why': 'left P'})
        elif (e['S'], e['n'], e['vouched_by']) != (u['S'], u['n'], u['vouched_by']):
            reread.append({'id': t, 'why': 'S, n or vouched_by changed', 'u2': [u['S'], u['n'], u['vouched_by']],
                           'u6': [e['S'], e['n'], e['vouched_by']]})
    for t in sorted(set(bp) - set(u2_bp)):
        e = ships[t]
        reread.append({'id': t, 'why': 'new B_P head, not among the 29 read', 'u6': [e['S'], e['currency'], e['n'],
                                                                                    e['vouched_by']],
                       'S_eur_cents': e['S_eur_cents']})
    pins = {'path_w': {}, 'trio': {}, 'path_v': {}, 'must_stay': K.MUST_STAY, 'not_lsa': K.NOT_LSA}
    for t, (c, cur) in K.PATH_W.items():
        e = ships.get(t)
        if e is None or e['S'] != c or e['currency'] != cur:
            stops.append(f'pin {t}: the port does not predict {cur} {c} lot-sum-awards ({e and [e["S"], e["currency"]]})')
        pins['path_w'][str(t)] = {'cents': c, 'currency': cur, 'derivation': 'lot-sum-awards',
                                  'vouched_by': e and e['vouched_by'], 'accepted_vouches': e and e['vouches'],
                                  'n': e and e['n']}
    for t, (c, cur) in K.TRIO.items():
        pins['trio'][str(t)] = {'eur_cents': c, 'currency': cur, 'derivation': None}
    for t, (c, cur, rule) in K.PATH_V.items():
        pins['path_v'][str(t)] = {'cents': c, 'currency': cur, 'derivation': 'lot-sum-vouched', 'rule': rule,
                                  'byte_identical_to': 'the pre snapshot'}
    for t in K.NOT_LSA:
        if t in ships:
            stops.append(f'{t} is in P: D3 requires it not to serve lot-sum-awards')
    L = sorted(ids3502 | set(lc['l_changed']))
    snap = sorted(set(L) | set(K.pin_ids()) | {K.NOT_REACHABLE} | set(K.the_61()))
    out = {
        'what': 'issue 551 B6 U6: the frozen prediction (written before the flip deploy; every U7/U8 checker reads it)',
        'P': {'n': len(ships), 'ids_sha256': pf['ids_sha256'], 'P_final_sha256': None,
              'u2_n': len(u2P), 'u2_ids_sha256': K.P_U2_IDS_SHA},
        'Rg': Rg, 'n_Rg': len(Rg), 'Rg_u2': K.rg_u2(), 'Rg_equals_u2': Rg == K.rg_u2(),
        'B_P': bp, 'n_B_P': len(bp), 'B_P_u2': u2_bp, 'B_P_equals_u2': bp == u2_bp,
        'near_eur100m_2pct': near, 'near_watch_outside_P': [t for t in K.NEAR_2PCT_WATCH if t not in ships],
        'band_reread': reread,
        'pins_expected': pins,
        'u9_pins_in_P': {str(t): t in ships for t in K.U9_PINS},
        'L': {'n': len(L), 'ids_sha256': K.ids_sha(L), 'runs_of_5000': [[r[0], r[-1], len(r)] for r in K.chunks(L, 5000)]},
        'l_changed': {'n': lc['n'], 'ids_sha256': lc['ids_sha256'], 'n_in_3502': lc['n_in_3502']},
        'snapshot_ids': {'n': len(snap), 'ids_sha256': K.ids_sha(snap)},
        'stops': stops,
    }
    return out, L, snap


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--pfinal', default=os.path.join(K.U6, 'P_final.json'))
    ap.add_argument('--lchanged', default=os.path.join(K.U6, 'l_changed.json'))
    ap.add_argument('--out', default=os.path.join(K.U6, 'freeze.json'))
    a = ap.parse_args(argv)
    pf, lc = K.jload(a.pfinal), K.jload(a.lchanged)
    if lc.get('stops'):
        print(f'STOP: l_changed.json carries STOPs: {lc["stops"]}')
        return 2
    ids, _ = K.the_3502()
    out, L, snap = build(pf, lc, ids)
    out['P']['P_final_sha256'] = K.sha_file(a.pfinal)
    out['l_changed']['file_sha256'] = K.sha_file(a.lchanged)
    d = os.path.dirname(os.path.abspath(a.out))
    K.jdump(os.path.join(d, 'L_ids.json'), {'ids': L, 'n': len(L), 'ids_sha256': K.ids_sha(L)})
    K.jdump(os.path.join(d, 'snapshot_ids.json'), {'ids': snap, 'n': len(snap), 'ids_sha256': K.ids_sha(snap)})
    K.jdump(a.out, out)
    print(f'P {out["P"]["n"]} ({out["P"]["ids_sha256"][:16]}), Rg {out["n_Rg"]} (=U2 {out["Rg_equals_u2"]}), '
          f'B_P {out["n_B_P"]} (=U2 {out["B_P_equals_u2"]}), near {out["near_eur100m_2pct"]}, band re-read '
          f'{len(out["band_reread"])}, L {out["L"]["n"]}, snapshot ids {out["snapshot_ids"]["n"]}')
    for r in out['band_reread']:
        print(f'  READ (B-band, before the flip deploy, 0 wrong of any class): {r["id"]} -- {r["why"]}')
    for s in out['stops']:
        print('  STOP', s)
    return 2 if (out['stops'] or out['band_reread']) else 0


if __name__ == '__main__':
    sys.exit(K.run_main(main))
