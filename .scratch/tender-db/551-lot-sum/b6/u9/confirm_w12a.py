"""Issue 551 W-on kit, build time only: W12 (a) -- does prod have `changes_entity_cursor`? -- and one E1 confirmation,
in at most 3 statements through its own box state (u9/box_confirm, budget 3). Never part of a runbook segment.

    python3 -I -B confirm_w12a.py [--out u9/confirm_w12a.json]

Why not the decision's "free sqlite_master read": /v1/sql reads only its public allow-list (crates/app/src/v1/sql.rs
ALLOWED: 48 tables and views; sqlite_master / sqlite_schema are not on it), so that statement would come back as an
error body. The equivalent through an allow-listed table: E1 itself with `INDEXED BY changes_entity_cursor`. turso
resolves the hint at PREPARE, so an absent index answers "no such index: changes_entity_cursor" before any data page
is touched (plan-probed both ways locally: plan_w.db SEARCHes the index; plan.db, which lacks it, refuses at parse).
When the index exists, the statement is one bounded E1 page over (959179090, 959180852] -- exactly the last chunk of
the archived all-kind enumeration (u9/20261023/build_changes_...tar.xz, 1,762 cursor values), so its tender rows
must equal that chunk's tender rows (cursor, entity_id, op) and its PK-subquery derivation must equal the archived
PK reads of 18:51Z (cand/changed) unless a later fold moved them.

  statement 1  E1 with the hint over the archived chunk.
  if it does not answer: the body is recorded and NOT retried. statement 2 = health (SELECT 1, clears the flag).
               If the body said "no such index": stop (W12 (a) FAILS: E1 cannot be used anywhere; 546 U4 after 11-04).
               Otherwise (the clause itself refused, or anything else): statement 3 = the same page WITHOUT the hint,
               which tells whether E1's shape answers at all (kitlib.E1_HINT=False is then the kit's setting).
Exit 0 confirmed; 2 not confirmed (read the record); 3 a tool fault.
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wkit as W  # noqa: E402

K = W.K
LO, HI = 959179090, 959180852       # the archive's last chunk (1,762 cursor values; 0175_changes_959179090_959180852)


def compare(rows, arch_rows, pk):
    want = [(int(r['cursor']), int(r['entity_id']), r['op']) for r in arch_rows
            if r['entity_kind'] == 'tender' and LO < int(r['cursor']) <= HI]
    got = [(int(r['cursor']), int(r['entity_id']), r['op']) for r in rows]
    d_diff = []
    for r in rows:
        t = int(r['entity_id'])
        a = pk.get(t)
        if a is not None and a.get('current_value_derivation') != r.get('d'):
            d_diff.append({'id': t, 'archived': a.get('current_value_derivation'), 'now': r.get('d')})
    return {'tender_rows_equal': got == want, 'n_e1': len(got), 'n_archive': len(want),
            'only_e1': sorted(set(got) - set(want))[:20], 'only_archive': sorted(set(want) - set(got))[:20],
            'ids_without_archived_row': sorted({int(r['entity_id']) for r in rows} - set(pk))[:20],
            'd_differs': d_diff[:20], 'n_d_differs': len(d_diff)}


def main(argv=None, fake=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--out', default=os.path.join(W.U9, 'confirm_w12a.json'))
    ap.add_argument('--state-dir', default=os.path.join(W.U9, 'box_confirm'))
    a = ap.parse_args(argv)
    box = W.w_box(fake, state_dir=a.state_dir, budget=3)
    rec = {'what': 'issue 551 W-on kit: W12 (a) + one E1 confirmation (build time)', 'at': K.now_iso(),
           'range': [LO, HI], 'plan_db': K.PLAN_DB_W,
           'plan_hint': K.plan_check(K.sql_tender_keyset(LO, HI, hint=True), 'w12a_plan', a.state_dir, K.PLAN_DB_W),
           'plan_nohint': K.plan_check(K.sql_tender_keyset(LO, HI, hint=False), 'w12a_plan', a.state_dir,
                                       K.PLAN_DB_W)}
    arch = W.archived_changes()
    pk = W.archived_pk(W.ARCH_CAND)
    pk.update(W.archived_pk(W.ARCH_CHANGED))
    body = box.sq(K.sql_tender_keyset(LO, HI, hint=True), 'w12a_e1_hint')
    if body is not None:
        rows = K.rows_of(body)
        rec.update(index_exists=True, hint_accepted=True, statements=1, compare=compare(rows, arch, pk))
        ok = rec['compare']['tender_rows_equal']
        rec['verdict'] = ('CONFIRMED: changes_entity_cursor exists on prod (INDEXED BY resolved), E1 answered and its '
                          'tender rows equal the archived all-kind chunk' if ok else
                          'NOT CONFIRMED: E1 answered but its tender rows differ from the archived chunk (read compare)')
        K.jdump(a.out, rec)
        print(rec['verdict'])
        print(f'compare: {rec["compare"]}')
        return 0 if ok else 2
    raw_dir = os.path.join(a.state_dir, 'raw')
    raw = sorted(f for f in os.listdir(raw_dir) if f.endswith('_w12a_e1_hint.json'))
    text = open(os.path.join(raw_dir, raw[-1]), encoding='utf-8').read() if raw else ''
    rec.update(hint_body=text[:500], statements=1)
    h = box.health()
    rec['statements'] = 2
    rec['health_ok'] = h is not None
    if 'no such index' in text:
        rec.update(index_exists=False, verdict='W12 (a) FAILS: prod has no changes_entity_cursor -- E1 is unusable; '
                                               'every reader stays all-kind; 546 U4 after 11-04 (decision I.2)')
        K.jdump(a.out, rec)
        print(rec['verdict'])
        return 2
    if h is None:
        rec['verdict'] = 'NOT CONFIRMED: the hinted E1 did not answer and health did not answer either; stop'
        K.jdump(a.out, rec)
        print(rec['verdict'])
        return 2
    body = box.sq(K.sql_tender_keyset(LO, HI, hint=False), 'w12a_e1_nohint')
    rec['statements'] = 3
    if body is None:
        rec['verdict'] = 'NOT CONFIRMED: neither E1 form answered (bodies in u9/box_confirm/raw); the index is unknown'
        K.jdump(a.out, rec)
        print(rec['verdict'])
        return 2
    rows = K.rows_of(body)
    rec.update(hint_accepted=False, index_exists=None, compare=compare(rows, arch, pk),
               verdict='PARTLY: /v1/sql refused the INDEXED BY form but answered E1 without it; set kitlib.E1_HINT = '
                       'False. Index existence is NOT shown by an answer (turso may walk the rowid range instead).')
    K.jdump(a.out, rec)
    print(rec['verdict'])
    print(f'compare: {rec["compare"]}')
    return 2


if __name__ == '__main__':
    sys.exit(K.run_main(main))
