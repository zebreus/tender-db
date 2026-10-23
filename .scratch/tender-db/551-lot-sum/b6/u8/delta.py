"""Issue 551 B6 U8: the delta read set (decision U8 / D6, salt '551-b6-delta-' fixed in point 14 before the draw).

    python3 -I -B delta.py predraw      writes u8/delta_predraw.json: the population (u8/delta_population.json from
                                        verify.py check), its sha256, the salt, the draw rule AND the full bar (BAR:
                                        each population case and the band rule, plus which case applies) -- BEFORE
                                        any draw is shown; the same text goes into issue 551 before the draw
    python3 -I -B delta.py draw         refuses unless the predraw record exists and the population still matches it;
                                        writes u8/delta_read_set.json and u8/delta_fetch_ids.json (fetch_b6.py --ids)
    python3 -I -B delta.py predraw|draw --population FILE --salt SALT --out DIR
                                        E5 (W-on drains decision): the same predraw/draw for any population in
                                        delta_population.json's shape ({ids, n, ids_sha256, band_rows, rows}) -- a
                                        drain's NEW set (u9/segments/<seg>/new_population.json, salt
                                        '551-b6-drain-<issue>-') or the day-7 populations (u9/day7/, salt
                                        '551-b6-daily-'); DIR receives delta_predraw.json / delta_read_set.json /
                                        delta_fetch_ids.json. The predraw record fixes the salt and the population's
                                        sha256; the draw refuses if either changed (or if --salt differs). A
                                        population that records its salt (gd5's new_population.json: the segment's
                                        pre-registered salt; day7's populations) refuses any other --salt.

The population: every Tender serving lot-sum-awards after the drain that is in L_changed and not among the 3,502 (a
new fire the port never predicted). The draw:
  - population <= 30: read ALL of them;
  - population > 30: the first 30 in sha256('551-b6-delta-' + id) ascending order; the next 30 in the same order are
    listed as the continuation (used only if the first 30 hold exactly one wrong);
  - every delta band row (served EUR value >= EUR 100 m) is read in full whether or not it is drawn.
U5's bar applied to the delta (B-fresh, sequential; decision U8 "with U5's bar", D6):
  - population > 30: 0 wrong of the 30 -> PASS; exactly 1 wrong -> read the continuation 30 (or every remaining head
    if fewer), which must hold 0 wrong (<= 1 of 60); 2 or more wrong -> FAIL -> SWITCH OFF;
  - population <= 30: all are read; 0 wrong -> PASS; 1 or more wrong -> FAIL -> SWITCH OFF (no further heads exist
    to continue into: the C8 day-7 reading "fewer than 30: read all at 0 wrong");
  - neutral heads outside the band are allowed and counted; a delta band row must read 0 wrong of ANY class (neutral
    allowed), else SWITCH OFF;
  - readers get the served head (S = current_value_cents, currency, n = detail.lots, vouched_by), the Tender and its
    notices (fetch via fetch_b6.py --ids u8/delta_fetch_ids.json), a reader + an adversarial skeptic per head, a third
    reader on disagreement (wf_dc85fa7e-c00's rubric). No port prediction exists for these heads.
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), 'u6'))
import kitlib as K  # noqa: E402

N_READ = 30
# U5's bar as it applies to the delta (decision U8 "with U5's bar"; D6; C8's day-7 wording for a small population),
# fixed in the pre-draw record BEFORE the draw, as the salt is.
BAR = {
    'population_over_30': '0 wrong of the first 30 (sha256(salt + id) ascending) -> PASS; exactly 1 wrong -> read the '
                          'continuation (the next 30 in the same order, or every remaining head if fewer than 30 remain), '
                          'which must hold 0 wrong (<= 1 of 60 in all); 2 or more wrong -> FAIL -> SWITCH OFF',
    'population_31_to_59': 'as above; the continuation is every head that remains after the first 30 (fewer than 30)',
    'population_30_or_fewer': 'every head is read; 0 wrong -> PASS; 1 or more wrong -> FAIL -> SWITCH OFF (no further '
                              'heads exist to continue into: C8 "fewer than 30: read all at 0 wrong")',
    'neutral': 'neutral heads outside the band are allowed and counted',
    'band_rows': 'every delta band row (served EUR value >= EUR 100 m) is read in full whether drawn or not, and must '
                 'read 0 wrong of ANY class (neutral allowed), else SWITCH OFF (point 8 / B-band)',
    'protocol': 'a reader + an adversarial skeptic per head, a third reader on disagreement (wf_dc85fa7e-c00\'s rubric); '
                'readers get the served head (S = current_value_cents, currency, n = detail.lots, vouched_by), the '
                'Tender and its notices; no port prediction exists for these heads',
}


def _pop_path(out, population):
    return population or os.path.join(out, 'delta_population.json')


def cmd_predraw(out=HERE, population=None, salt=None):
    salt = salt or K.SALT_DELTA
    os.makedirs(out, exist_ok=True)
    pop = K.jload(_pop_path(out, population))
    if K.ids_sha(pop['ids']) != pop['ids_sha256'] or len(pop['ids']) != pop['n']:
        print('REFUSED: the population file\'s ids do not match its own n / ids_sha256')
        return 2
    if pop.get('salt') is not None and pop['salt'] != salt:
        print(f'REFUSED: --salt {salt!r} is not the salt pre-registered with the population\'s segment '
              f'({pop.get("seg") or "day 7"}: {pop["salt"]!r}; D.1)')
        return 2
    pre = os.path.join(out, 'delta_predraw.json')
    if os.path.exists(pre):
        print(f'{pre} exists; a predraw record is written once')
        return 2
    rec = {'what': 'issue 551 B6 delta read: pre-draw record (written before the draw)', 'at': K.now_iso(),
           'salt': salt, 'population_file': _pop_path(out, population),
           'population_n': pop['n'], 'population_ids_sha256': pop['ids_sha256'],
           'rule': 'all if <= 30, else the first 30 by sha256(salt + id) ascending (+ the next 30 as continuation); '
                   'every delta band row read in full', 'bar': BAR,
           'bar_applies': 'population_30_or_fewer' if pop['n'] <= N_READ else
                          ('population_31_to_59' if pop['n'] < 2 * N_READ else 'population_over_30'),
           'band_rows': pop['band_rows']}
    K.jdump(pre, rec)
    print(f'predraw: population {pop["n"]} (sha256 {pop["ids_sha256"][:16]}), band rows {pop["band_rows"]}; bar case '
          f'{rec["bar_applies"]}: {BAR[rec["bar_applies"]]}; band rows: {BAR["band_rows"]} -- record this in 551 now')
    return 0


def cmd_draw(out=HERE, population=None, salt=None):
    pre_p = os.path.join(out, 'delta_predraw.json')
    if not os.path.exists(pre_p):
        print('REFUSED: write the predraw record first')
        return 2
    pre = K.jload(pre_p)
    if not pre.get('bar'):
        print('REFUSED: the predraw record does not fix the bar (written by an older kit): a new record is needed')
        return 2
    pop = K.jload(_pop_path(out, population))
    salt = salt or K.SALT_DELTA
    if pop['ids_sha256'] != pre['population_ids_sha256'] or K.ids_sha(pop['ids']) != pop['ids_sha256'] \
            or pre['salt'] != salt:
        print('REFUSED: the population (or the salt) changed after the predraw record')
        return 2
    ids = pop['ids']
    order = K.salted(ids, salt)
    if len(ids) <= N_READ:
        drawn, cont, mode = sorted(ids), [], 'all'
    else:
        drawn, cont, mode = order[:N_READ], order[N_READ:2 * N_READ], 'salted-30'   # cont: < 30 when n < 60
    band = sorted(pop['band_rows'])
    read = sorted(set(drawn) | set(band))
    items = []
    for i in read:
        r = pop['rows'][str(i)]
        det = K.detail_of(r)
        items.append({'id': i, 'drawn': i in drawn, 'band_row': i in band, 'S': r.get('current_value_cents'),
                      'currency': r.get('current_value_currency'), 'eur_cents': r.get('current_value_eur_cents'),
                      'n': det.get('lots'), 'vouched_by': det.get('vouched_by'),
                      'bar': '0 wrong of any class (neutral allowed)' if i in band else 'U5 sequential bar'})
    rs = {'what': 'issue 551 B6 delta read set', 'at': K.now_iso(), 'mode': mode, 'salt': salt,
          'population_n': len(ids), 'population_ids_sha256': pop['ids_sha256'], 'order': order, 'drawn': drawn,
          'continuation': cont, 'band_rows': band, 'read': read, 'items': items, 'bar': pre.get('bar'),
          'bar_applies': pre.get('bar_applies')}
    K.jdump(os.path.join(out, 'delta_read_set.json'), rs)
    K.jdump(os.path.join(out, 'delta_fetch_ids.json'), {'ids': sorted(set(read) | set(cont)),
                                                          'n': len(set(read) | set(cont))})
    print(f'delta: population {len(ids)}, mode {mode}, read {len(read)} (drawn {len(drawn)}, band {len(band)}), '
          f'continuation {len(cont)}')
    return 0


def main(argv=None, out=HERE):
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['predraw', 'draw'])
    ap.add_argument('--population', help='E5: a population file in delta_population.json\'s shape')
    ap.add_argument('--salt', help='E5: the pre-registered salt (default 551-b6-delta-)')
    ap.add_argument('--out', help='E5: the directory for the predraw record and the read set (default u8/)')
    a = ap.parse_args(argv)
    if (a.population or a.salt or a.out) and not (a.population and a.salt and a.out):
        print('REFUSED: --population, --salt and --out go together (E5)')
        return 2
    out = a.out or out
    if a.cmd == 'predraw':
        return cmd_predraw(out, a.population, a.salt)
    return cmd_draw(out, a.population, a.salt)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
