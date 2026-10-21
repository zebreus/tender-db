"""Issue 551 B6 U6 step 4 (and every post-run read of U7/U8): bounded PK reads of an id list.

    python3 -I -B snapshot.py --ids u6/snapshot_ids.json --label pre [--out u6/snapshot/pre]

Reads `tenders` by primary key (<= 2,000 ids per statement, IN-list on the rowid: plan-probed SEARCH ... INTEGER
PRIMARY KEY) and stores, per id: source, current_seq, current_published_at, current_value_derivation,
current_value_cents, current_value_currency, current_value_eur_cents, current_value_derivation_detail,
projection_epoch. Writes <out>/rows.json and <out>/manifest.json (complete = every id answered, no failed request).
A failed request sets the box flag and is NOT retried: re-run after `python3 -I -B box_health.py` answers.
Exit 0 complete; 3 incomplete (a failed request or an id with no row: not a measurement -- box_health, then re-run
the same command: an incomplete read is overwritten, a complete one never); 2 STOP (the PRE snapshot shows lot-sum-awards, or a complete snapshot would be overwritten).
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402


def load_ids(p):
    d = K.jload(p)
    return [int(i) for i in (d['ids'] if isinstance(d, dict) else d)]


def main(argv=None, fake=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--ids', required=True)
    ap.add_argument('--label', required=True)
    ap.add_argument('--out')
    a = ap.parse_args(argv)
    ids = load_ids(a.ids)
    out = a.out or os.path.join(K.U6, 'snapshot', a.label)
    if os.path.exists(os.path.join(out, 'manifest.json')) and K.jload(os.path.join(out, 'manifest.json')).get('complete'):
        print(f'{out} already holds a complete read; refusing to overwrite a snapshot (use a new --label)')
        return 2
    box = K.shared_box(fake)
    rows, failures, missing = K.pk_read(box, ids, a.label, out)
    print(f'{a.label}: {len(rows)} of {len(set(ids))} ids read; failures {len(failures)}; missing '
          f'{len(missing) if missing is not None else "n/a"}')
    if missing:
        print(f'  ids with no row (no such Tender): {missing[:20]}')
    lsa = sorted(i for i, r in rows.items() if r.get('current_value_derivation') == 'lot-sum-awards')
    print(f'  serving lot-sum-awards: {len(lsa)} {lsa[:10]}  (the PRE snapshot must show 0: path W is off)')
    if a.label == 'pre' and lsa:
        print('  STOP: the pre snapshot is not the W=false state')
        return 2
    return 0 if (not failures and not missing) else 3


if __name__ == '__main__':
    sys.exit(K.run_main(main))
