"""Issue 551, path W on: W12 (b), E1's replay before E1 replaces the all-kind reader anywhere.

    python3 -I -B replay.py read  [--dir u9/replay]       (box, ~4-5 statements: E1 over (959004090, 959180852])
    python3 -I -B replay.py check [--dir u9/replay]       (offline)

read   E1 (kitlib.tender_keyset through the W box) over (C1, K0] = (959004090, 959180852], the range the archived
       all-kind enumeration u9/20261023/build_changes_2026-10-23T184801Z.tar.xz covers (18,216 tender rows there:
       4 pages). Resumable; a page that does not answer is recorded and never retried (a 408'd page resumes halved).
check  (1) E1's tender rows (cursor, entity_id, op) must EQUAL the archive's tender rows -- the change log is
       append-only, so any difference is an E1 defect; (2) its lot-sum-awards set (the live ids whose PK subquery reads
       lot-sum-awards) must equal R0's 3,478 minus the ids the registry has recorded LOST since, plus nothing but ids
       a later segment registered (a Tender re-decided after K0 reads its CURRENT derivation). PASS = both.
Exit 0 PASS; 2 FAIL (E1 stays off: the all-kind reader everywhere, 546 U4 after 11-04); 3 incomplete / tool fault.
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wkit as W  # noqa: E402
import registry as REG  # noqa: E402

K = W.K


def cmd_read(a, fake=None):
    box = W.w_box(fake)
    u0 = W.box_used(box)
    rows, ok = K.tender_keyset(box, W.C1, W.K0, os.path.join(a.dir, 'e1'), label='replay')
    print(f'replay read: {len(rows)} tender rows over ({W.C1}, {W.K0}] in {W.box_used(box) - u0} statement(s); '
          f'{"complete" if ok else "INCOMPLETE (box_health, then re-run: a transport failure or shed resumes the same page; a 408 resumes with the page halved, never the same statement)"}')
    return 0 if ok else W.NOT_MEASURED


def check(e1_rows, arch_rows, r0_ids, lost_ids, later_ids):
    want = [(int(r['cursor']), int(r['entity_id']), r['op']) for r in arch_rows if r['entity_kind'] == 'tender']
    got = [(int(r['cursor']), int(r['entity_id']), r['op']) for r in sorted(e1_rows, key=lambda r: int(r['cursor']))]
    c = K.classify_tender_rows(e1_rows)
    lsa = c['lsa']
    missing = sorted(set(r0_ids) - lsa - set(lost_ids))
    extra = sorted(lsa - set(r0_ids) - set(later_ids))
    out = {'tender_rows_equal': got == want, 'n_e1': len(got), 'n_archive': len(want),
           'only_e1': sorted(set(got) - set(want))[:20], 'only_archive': sorted(set(want) - set(got))[:20],
           'lsa_e1': len(lsa), 'r0': len(r0_ids), 'lost_since': sorted(set(r0_ids) - lsa)[:20],
           'r0_missing_unexplained': missing[:20], 'lsa_outside_registry': extra[:20]}
    out['pass'] = out['tender_rows_equal'] and not missing and not extra
    return out


def cmd_check(a):
    man_p = os.path.join(a.dir, 'e1', 'manifest.json')
    if not os.path.exists(man_p) or not K.jload(man_p).get('complete'):
        print('replay check: NOT A MEASUREMENT (the E1 read is missing or incomplete)')
        return W.NOT_MEASURED
    man = K.jload(man_p)
    rows = []
    for key in sorted(man['pages'], key=int):
        rows += K.rows_of(K.jload(os.path.join(a.dir, 'e1', f'page_{key}.json')))
    reg = REG.load()
    r0 = [int(i) for i in K.jload(W.R0_FILE)['ids']]
    later = [int(i) for i, e in reg['entries'].items() if e.get('seg') != 'R0']
    res = check(rows, W.archived_changes(), r0, [int(i) for i in reg['lost']], later)
    res['at'] = K.now_iso()
    K.jdump(os.path.join(a.dir, 'replay.json'), res)
    print(f'replay: tender rows E1 {res["n_e1"]} vs archive {res["n_archive"]} -> '
          f'{"EQUAL" if res["tender_rows_equal"] else "DIFFERENT"}; lot-sum-awards {res["lsa_e1"]} vs R0 {res["r0"]} '
          f'(lost since {res["lost_since"]}; unexplained missing {res["r0_missing_unexplained"]}; outside the registry '
          f'{res["lsa_outside_registry"]}) -> {"PASS" if res["pass"] else "FAIL"}')
    return 0 if res['pass'] else 2


def main(argv=None, fake=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['read', 'check'])
    ap.add_argument('--dir', default=os.path.join(W.U9, 'replay'))
    a = ap.parse_args(argv)
    return cmd_read(a, fake) if a.cmd == 'read' else cmd_check(a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
