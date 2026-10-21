"""Issue 551 B6: the switch-off route's id list and its verification (decision "Drain stop rules": switch off; point 11;
C7).

    python3 -I -B switchoff.py build  --flip-cursor C1 --cursor-now N
    python3 -I -B switchoff.py verify --flip-cursor C1 --cursor-now N2 [--band-route sql|api]

Route: (1) redeploy the W=false twin (the rev deployed before the flip, d617a78; NOT pre-gated: a redeploy re-gates
it, RUNBOOK §0.6/§S); (2) `refold-lot-sum` by tender_ids over every id serving lot-sum-awards, runs of <= 5,000 (dry ->
make_wet.py --jobs-dir u7/switchoff/jobs -> wet, which queues its project); (3) verify: the PK reads show 0
lot-sum-awards and the band seek shows 0 rows.

Point 11 / C7: the list is the drain ids ∪ every id with a change row since the flip cursor that serves lot-sum-awards.
So --flip-cursor C1 (the /health cursor recorded right after the flip deploy, U7.2) and --cursor-now N (read now) are
REQUIRED: a fire the registry never recorded (the run in flight at a D7 switch-off, a check that returned NOT A
MEASUREMENT or crashed) has a change row since C1 and is found there.

build   (box) PK-reads the candidates -- the 3,502 ∪ L_changed ∪ every fire the runs saw (u7/state/registry.json) ∪ the
        pins -- plus every live Tender with a change row in (C1, N] (<= 5,000 rows per statement; Tenders retired in the
        range are expected-absent), and keeps those serving lot-sum-awards. Writes u7/switchoff/switchoff.json and
        jobs/offNN_ids.json + offNN_dry.json.
verify  (box) after the drain-back's project(s): PK-reads every id of switchoff.json, the candidates again and every live
        Tender with a change row in (C1, N2] (N2 = the cursor after the drain-back), then the band seek (the /v1/sql
        statement, or the public API if it does not answer -- never a retry); PASS iff no id serves lot-sum-awards and
        the band holds no lot-sum-awards row.
Exit 0 / PASS; 2 a lot-sum-awards row left (another off run); 3 a read incomplete or a tool fault.
"""
import argparse
import json
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), 'u6'))
import kitlib as K  # noqa: E402

OFF = os.path.join(HERE, 'switchoff')
LSA = 'lot-sum-awards'


def candidates():
    ids3502 = K.the_3502()[0]
    lc = K.jload(os.path.join(K.U6, 'l_changed.json'))['l_changed']
    reg_p = os.path.join(HERE, 'state', 'registry.json')
    reg = K.jload(reg_p) if os.path.exists(reg_p) else {'fired': {}, 'retired': []}
    fired = [int(i) for i in reg.get('fired', {})]
    return sorted(set(ids3502) | set(lc) | set(fired) | set(K.pin_ids())), set(reg.get('retired', []))


def since_flip(box, flip_cursor, cursor_now, stamp, tag):
    """Live Tenders with a change row in (C1, N], and the Tenders retired there. None when the enumeration is
    incomplete."""
    crow, complete = K.enumerate_changes(box, flip_cursor, cursor_now, os.path.join(OFF, f'{tag}_changes_{stamp}'))
    if not complete:
        return None
    cls = K.classify_changes(crow)
    if cls['unknown_kinds'] or cls['unmapped_lots']['bad']:
        print(f'NOTE: unknown kinds {cls["unknown_kinds"]}, unexplained unmapped lot rows '
              f'{cls["unmapped_lots"]["bad"][:10]} in ({flip_cursor}, {cursor_now}] (switching off goes on regardless)')
    return cls


def cmd_build(a, fake=None):
    box = K.shared_box(fake)
    stamp = K.now_iso().replace(':', '')
    cand, retired_reg = candidates()
    cls = since_flip(box, a.flip_cursor, a.cursor_now, stamp, 'build')
    if cls is None:
        return 3
    retired = retired_reg | cls['retired']
    rows, f, m = K.pk_read(box, cand, 'off_cand', os.path.join(OFF, f'cand_{stamp}'), expected_absent=retired)
    if f or m:
        return 3
    lsa = {i for i, r in rows.items() if r.get('current_value_derivation') == LSA}
    extra = set()
    ch = sorted(cls['live'] - set(cand))
    if ch:
        r2, f, m = K.pk_read(box, ch, 'off_changed', os.path.join(OFF, f'changed_{stamp}'))
        if f or m:
            return 3
        extra = {i for i, r in r2.items() if r.get('current_value_derivation') == LSA}
    off = sorted(lsa | extra)
    jobs = os.path.join(OFF, 'jobs')
    os.makedirs(jobs, exist_ok=True)
    runs = []
    for k, part in enumerate(K.chunks(off, 5000)):
        name = f'off{k + 1:02d}'
        K.jdump(os.path.join(jobs, f'{name}_ids.json'), {'run': name, 'ids': part, 'n': len(part),
                                                          'ids_sha256': K.ids_sha(part)})
        with open(os.path.join(jobs, f'{name}_dry.json'), 'w') as fh:
            fh.write(json.dumps({'kind': 'refold-lot-sum', 'tender_ids': part}, separators=(',', ':')) + '\n')
        runs.append({'run': name, 'n': len(part)})
    K.jdump(os.path.join(OFF, 'switchoff.json'), {'at': K.now_iso(), 'n_candidates': len(cand), 'n_off': len(off),
                                                  'ids': off, 'ids_sha256': K.ids_sha(off), 'from_changes': sorted(extra),
                                                  'flip_cursor': a.flip_cursor, 'cursor_now': a.cursor_now,
                                                  'changed_since_flip': len(cls['live']), 'retired': sorted(retired),
                                                  'runs': runs})
    print(f'switch-off list: {len(off)} ids serving lot-sum-awards ({len(extra)} only from change rows since the flip '
          f'cursor {a.flip_cursor}); runs {[r["run"] for r in runs]} -- each: dry, make_wet.py <run> --jobs-dir {jobs}, '
          'wet (queues project); expect corrected = the run size')
    return 0


def cmd_verify(a, fake=None, http=None):
    box = K.shared_box(fake)
    so = K.jload(os.path.join(OFF, 'switchoff.json'))
    stamp = K.now_iso().replace(':', '')
    cand, retired_reg = candidates()
    cls = since_flip(box, a.flip_cursor, a.cursor_now, stamp, 'verify')
    if cls is None:
        return 3
    retired = retired_reg | cls['retired'] | set(so.get('retired', []))
    ids = sorted(set(so['ids']) | set(cand) | cls['live'])
    rows, f, m = K.pk_read(box, ids, 'off_verify', os.path.join(OFF, f'verify_{stamp}'), expected_absent=retired)
    if f or m:
        return 3
    left = sorted(i for i, r in rows.items() if r.get('current_value_derivation') == LSA)
    body, route = K.read_band(box, 'off_band', os.path.join(OFF, f'band_{stamp}.json'), route=a.band_route, http=http)
    if body is None:
        return 3
    band = K.rows_of(body)
    ok = not left and not band
    print(f'switch-off verify: {len(rows)} ids read ({len(cls["live"])} changed since the flip cursor), {len(left)} still '
          f'lot-sum-awards {left[:10]}; band rows {len(band)} (route {route}) -> {"PASS" if ok else "FAIL"}')
    return 0 if ok else 2


def main(argv=None, fake=None, http=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    for name in ('build', 'verify'):
        p = sub.add_parser(name)
        p.add_argument('--flip-cursor', type=int, required=True, help='C1: the /health cursor right after the flip deploy')
        p.add_argument('--cursor-now', type=int, required=True, help='the /health cursor read now')
        if name == 'verify':
            p.add_argument('--band-route', choices=['sql', 'api'], default='sql')
    a = ap.parse_args(argv)
    if a.cmd == 'build':
        return cmd_build(a, fake)
    return cmd_verify(a, fake, http)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
