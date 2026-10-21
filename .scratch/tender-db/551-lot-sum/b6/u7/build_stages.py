"""Issue 551 B6 U7: the staged drain's id lists and the exact job JSON (decision U7 + C6 + point 9).

    python3 -I -B build_stages.py [--freeze ../u6/freeze.json] [--pfinal ../u6/P_final.json]
                                  [--lchanged ../u6/l_changed.json] [--out-dir jobs]

Runs, in order (each: refold-lot-sum dry -> wet with "expect" = the dry's count, which queues its project):
  stage1  the 61 (b5/investigate/read_items.json), the pins (16 path-V, 3 path-W, the trio, the 9 must-stay, 6465510,
          and point 12's eight standing-rule pins), 7511890 (not reachable: the dry must list it as not admitted), every
          id in Rg (the final 3,502 minus P; U2's R_single ∪ R_excess also named, in case L_changed moved one), and a
          salted 300 of P: P minus the ids above, ordered by sha256('551-b6-stage1-' + id) ascending, first 300 (the
          salt and the population digest are recorded in stages.json before any job runs);
  stage2  the rest of the 3,502;
  lcNN    L_changed minus everything already named, ascending, in runs of <= 5,000.
Writes jobs/<run>_ids.json, jobs/<run>_dry.json ({"kind":"refold-lot-sum","tender_ids":[...]}: dry_run defaults to
true), and stages.json (per run: n, ids_sha256, |run ∩ P|, |run ∩ Rg|, B_P ids, pins, ids outside the 3,502).
The wet JSON is written by make_wet.py from the dry job's summary (expect = its admitted count).
"""
import argparse
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), 'u6'))
import kitlib as K  # noqa: E402

CAP = 5000


def build(freeze, pf, lc, the3502):
    ids3502 = set(the3502)
    P = {int(e['id']) for e in pf['ships']}
    Rg = set(freeze['Rg'])
    rg_u2 = set(K.rg_u2())
    fixed = set(K.the_61()) | set(K.pin_ids()) | {K.NOT_REACHABLE} | Rg | rg_u2
    population = sorted(P - fixed)
    order = K.salted(population, K.SALT_STAGE1)
    drawn = order[:K.STAGE1_SALTED_N]
    stage1 = sorted(fixed | set(drawn))
    stage2 = sorted(ids3502 - set(stage1))
    rest = sorted(set(lc['l_changed']) - set(stage1) - set(stage2))
    runs = [('stage1', stage1), ('stage2', stage2)]
    runs += [(f'lc{k + 1:02d}', part) for k, part in enumerate(K.chunks(rest, CAP))]
    bp = set(freeze['B_P'])
    pins = set(K.pin_ids())
    meta = []
    for name, ids in runs:
        assert 0 < len(ids) <= CAP, (name, len(ids))
        meta.append({'run': name, 'n': len(ids), 'ids_sha256': K.ids_sha(ids), 'n_P': len(set(ids) & P),
                     'n_Rg': len(set(ids) & Rg), 'B_P': sorted(set(ids) & bp), 'pins': sorted(set(ids) & pins),
                     'n_outside_3502': len(set(ids) - ids3502), 'first': ids[0], 'last': ids[-1]})
    assert sorted(set(stage1) | set(stage2)) == sorted(set(stage1) | ids3502) and ids3502 <= set(stage1) | set(stage2)
    out = {'what': 'issue 551 B6 U7: the staged drain (stage 1, stage 2, then the L_changed runs)',
           'freeze_P_ids_sha256': freeze['P']['ids_sha256'], 'l_changed_sha256': lc['ids_sha256'],
           'stage1_salt': K.SALT_STAGE1, 'stage1_salted_population': {'n': len(population),
                                                                      'ids_sha256': K.ids_sha(population)},
           'stage1_salted_drawn': drawn, 'stage1_fixed': {'the_61': len(K.the_61()), 'pins': len(K.pin_ids()),
                                                          '7511890': 1, 'Rg': len(Rg), 'Rg_u2_extra': sorted(rg_u2 - Rg)},
           'runs': meta, 'order': [m['run'] for m in meta],
           'job_flow': 'per run: POST the dry JSON; read its summary; make_wet.py <run> (expect = admitted); POST the wet '
                       'JSON (queues project); after project: dchecks.py read + all; next run only on PASS'}
    return out, runs


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--freeze', default=os.path.join(K.U6, 'freeze.json'))
    ap.add_argument('--pfinal', default=os.path.join(K.U6, 'P_final.json'))
    ap.add_argument('--lchanged', default=os.path.join(K.U6, 'l_changed.json'))
    ap.add_argument('--out-dir', default=os.path.join(HERE, 'jobs'))
    a = ap.parse_args(argv)
    freeze, pf, lc = K.jload(a.freeze), K.jload(a.pfinal), K.jload(a.lchanged)
    if freeze.get('stops') or lc.get('stops') or pf.get('stops'):
        print('REFUSED: freeze / l_changed / P_final carry STOPs')
        return 2
    if K.sha_file(a.pfinal) != freeze['P']['P_final_sha256']:
        print('REFUSED: P_final.json is not the one freeze.json froze')
        return 2
    ids, _ = K.the_3502()
    out, runs = build(freeze, pf, lc, ids)
    os.makedirs(a.out_dir, exist_ok=True)
    for name, rids in runs:
        K.jdump(os.path.join(a.out_dir, f'{name}_ids.json'), {'run': name, 'ids': rids, 'n': len(rids),
                                                               'ids_sha256': K.ids_sha(rids)})
        body = json.dumps({'kind': 'refold-lot-sum', 'tender_ids': rids}, separators=(',', ':'))
        with open(os.path.join(a.out_dir, f'{name}_dry.json'), 'w') as fh:
            fh.write(body + '\n')
    K.jdump(os.path.join(HERE, 'stages.json'), out)
    for m in out['runs']:
        print(f'{m["run"]}: {m["n"]} ids (P {m["n_P"]}, Rg {m["n_Rg"]}, B_P {len(m["B_P"])}, pins {len(m["pins"])}, '
              f'outside the 3,502 {m["n_outside_3502"]})')
    return 0


if __name__ == '__main__':
    sys.exit(K.run_main(main))
