"""Issue 551 B6 U6 kit, prep only: the bounded reads that confirm the table/column names and statement shapes the kit
sends (budget 5, its own state dir u6/box_confirm/). Run once at kit-build time; never part of the runbook.

    python3 -I -B confirm_schema.py
"""
import json
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402


def main():
    box = K.Box(os.path.join(K.U6, 'box_confirm'), budget=5)
    out = {}
    b = box.sq(K.sql_changes(K.ZERO_DRIFT_CURSOR - 10, K.ZERO_DRIFT_CURSOR), 'confirm_changes')
    out['changes'] = b
    if b is None:
        return report(out)
    b = box.sq(K.sql_tenders_pk([585917, 5995030, 8426318, 6465510, 909]), 'confirm_tenders')
    out['tenders'] = b
    if b is None:
        return report(out)
    b = box.sq(K.sql_notice_probe(47400000, 47600000, 20000), 'confirm_notice_probe')
    out['notice_probe'] = b
    if b is None:
        return report(out)
    rows = K.rows_of(b)
    after = [r for r in rows if r['ingested_at'] is not None and r['ingested_at'] >= K.CUTOFF]
    n = after[0]['nid'] if after else rows[-1]['nid']
    b = box.sq(K.sql_notice_tenders(n, n + 5), 'confirm_notice_tenders')
    out['notice_tenders'] = b
    if b is None:
        return report(out)
    plan = K.jload(K.PLAN545)
    nids = sorted({m['notice'] for m in json.loads(plan['body'])['plan']})
    b = box.sq(K.sql_notices_to_tenders(nids), 'confirm_545_notice_tenders')
    out['545'] = {'notices': nids, 'body': b}
    return report(out)


def report(out):
    K.jdump(os.path.join(K.U6, 'box_confirm', 'confirm_result.json'), out)
    for k, v in out.items():
        body = v.get('body') if k == '545' else v
        if body is None:
            print(k, 'NO ANSWER')
            continue
        print(k, 'columns', body['columns'], 'rows', len(body['rows']))
        for r in body['rows'][:6]:
            print('   ', r)
    return 0


if __name__ == '__main__':
    sys.exit(main())
