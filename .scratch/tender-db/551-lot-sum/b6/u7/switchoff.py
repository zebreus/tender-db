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
        Tender with a change row in (N, N2] (N = the build's cursor, N2 = the cursor after the drain-back), then the band
        seek (the /v1/sql statement, or the public API if it does not answer -- never a retry); PASS iff no id serves
        lot-sum-awards and the band holds no lot-sum-awards row.
E4 (W-on drains decision, B2/F "SWITCH OFF means"): the registry route, the default once u9/registry/R.json exists --
    python3 -I -B switchoff.py build  --registry u9/registry/R.json --base-cursor K --cursor-now N
    python3 -I -B switchoff.py verify --registry u9/registry/R.json --base-cursor K --cursor-now N2 [--band-route ..]
  candidates = R ∪ Rg ∪ pins (the registry's entries and watch rows, plus kitlib's pins) ∪ the live Tenders with a
  tender row in (K, N] -- E1 (u6/kitlib.tender_keyset) for spans > 50k cursor values, else the all-kind reader. K is the
  registry's checkpoint (refused if it differs from R.json's K, unless --force-base); reads go through the switch-off
  box (below). E1's PK subquery already reads each changed Tender's derivation, so only the changed ids serving
  lot-sum-awards are PK-read beside the candidates (the all-kind route PK-reads every live changed Tender). The C1 route
  (--flip-cursor) stays as the fallback (a missing or unreadable registry; K frozen past a STOP whose range is unread).
Both routes, build and verify, read through the switch-off's OWN box (u7/switchoff/box: no lifetime cap, 150/h counted
over every kit box -- segment readers stop at 125/h, so a switch-off always has >= 25/h -- plan-probed on plan_w.db):
never u6/box's remaining 300 and never the readers' budget. Every read directory is fixed per (route, base, N) -- a
re-run after a non-answer (box_health first) RESUMES (a 408'd page or chunk halved, never re-sent as it was); a cap
refusal only waits for the rolling hour. Pass the SAME --cursor-now N on a re-run (read N once, after the twin deploy).
verify reads only what build did not: switchoff.json must be this route's build (the same base cursor, its N <= N2);
it walks (N, N2] and PK-reads switchoff.json's ids ∪ the candidates ∪ the changed live Tenders of (N, N2] (under the
W=false twin nothing turns lot-sum-awards after N without a change row in (N, N2]). An unproven E1 (W12) or a small span
degrades to the all-kind reader with a NOTE: the switch-off is never refused for its reader.
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


def _wkit():
    sys.path.insert(0, os.path.join(os.path.dirname(HERE), 'u9'))
    import wkit
    return wkit


def off_box(fake=None):
    return _wkit().switchoff_box(fake)


def since_flip(box, flip_cursor, cursor_now, tag):
    """Live Tenders with a change row in (lo, N], and the Tenders retired there. None when the enumeration is
    incomplete. The directory is fixed per (tag, lo, N): a re-run resumes."""
    crow, complete = K.enumerate_changes(box, flip_cursor, cursor_now,
                                         os.path.join(OFF, f'{tag}_changes_{flip_cursor}_{cursor_now}'))
    if not complete:
        return None
    cls = K.classify_changes(crow)
    if cls['unknown_kinds'] or cls['unmapped_lots']['bad']:
        print(f'NOTE: unknown kinds {cls["unknown_kinds"]}, unexplained unmapped lot rows '
              f'{cls["unmapped_lots"]["bad"][:10]} in ({flip_cursor}, {cursor_now}] (switching off goes on regardless)')
    return cls


def candidates_registry(reg_path):
    """E4: R ∪ Rg ∪ pins from the registry (entries + watch rows) and kitlib's pins; and the Tenders the registry knows
    retired."""
    reg = K.jload(reg_path)
    ids = {int(i) for i in reg['entries']} | {int(i) for i in reg.get('watch', {})} | set(K.pin_ids())
    return sorted(ids), {int(i) for i in reg.get('retired', [])}, reg


def since_base(box, base, now, tag, route):
    W = _wkit()
    d = os.path.join(OFF, f'{tag}_since_{base}_{now}')
    ch = W.changed_since(box, base, now, d, route=route, label=f'off_{tag}', degrade=True)
    if not ch['complete']:
        return None
    if ch.get('cls') and (ch['cls']['unknown_kinds'] or ch['cls']['unmapped_lots']['bad']):
        print(f'NOTE: unknown kinds {ch["cls"]["unknown_kinds"]}, unexplained unmapped lot rows '
              f'{ch["cls"]["unmapped_lots"]["bad"][:10]} in ({base}, {now}] (switching off goes on regardless)')
    return ch


def _check_base(a, reg):
    if int(reg['K']) != a.base_cursor and not a.force_base:
        raise SystemExit(f'--base-cursor {a.base_cursor} != the registry\'s K {reg["K"]} (pass --force-base only when '
                         'the registry\'s K is known to be wrong)')


def cmd_build_registry(a, fake=None):
    box = off_box(fake)
    cand, retired_reg, reg = candidates_registry(a.registry)
    _check_base(a, reg)
    ch = since_base(box, a.base_cursor, a.cursor_now, 'build', a.route)
    if ch is None:
        return 3
    retired = retired_reg | ch['retired']
    key = f'{a.base_cursor}_{a.cursor_now}'
    rows, f, m = K.pk_read(box, cand, 'off_cand', os.path.join(OFF, f'cand_{key}'), expected_absent=retired,
                           resume=True)
    if f or m:
        return 3
    lsa = {i for i, r in rows.items() if r.get('current_value_derivation') == LSA}
    pool = (ch['lsa'] if ch['lsa'] is not None else ch['live']) - set(cand)
    extra = set()
    if pool:
        r2, f, m = K.pk_read(box, sorted(pool), 'off_changed', os.path.join(OFF, f'changed_{key}'), resume=True)
        if f or m:
            return 3
        extra = {i for i, r in r2.items() if r.get('current_value_derivation') == LSA}
    return write_off(a, sorted(lsa | extra), extra, cand, retired,
                     {'registry': a.registry, 'registry_K': reg['K'], 'base_cursor': a.base_cursor,
                      'route': ch['route'], 'changed_since_base': len(ch['live'])})


def write_off(a, off, extra, cand, retired, meta):
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
    rec = {'at': K.now_iso(), 'n_candidates': len(cand), 'n_off': len(off), 'ids': off, 'ids_sha256': K.ids_sha(off),
           'from_changes': sorted(extra), 'cursor_now': a.cursor_now, 'retired': sorted(retired), 'runs': runs}
    rec.update(meta)
    K.jdump(os.path.join(OFF, 'switchoff.json'), rec)
    base = meta.get('base_cursor', meta.get('flip_cursor'))
    print(f'switch-off list: {len(off)} ids serving lot-sum-awards ({len(extra)} only from change rows since cursor '
          f'{base}); runs {[r["run"] for r in runs]} -- each: dry, make_wet.py <run> --jobs-dir {jobs}, wet (queues '
          'project); expect corrected = the run size')
    if len(off) > 4500:
        print(f'NOTE: {len(off)} ids: past ~4,500 re-check the D7 budget (more than one off run; decision F step 3)')
    return 0


def _verify_base(a, so, key, value):
    if so.get(key) != value:
        raise SystemExit(f'switchoff.json was built with {key} {so.get(key)}, not {value}: verify reads only what that '
                         'build did not (rebuild, or pass the build\'s base)')
    n = int(so['cursor_now'])
    if a.cursor_now < n:
        raise SystemExit(f'--cursor-now {a.cursor_now} < the build\'s N {n}')
    return n


def _verify_finish(a, box, ids, retired, label, http):
    rows, f, m = K.pk_read(box, ids, 'off_verify', os.path.join(OFF, label), expected_absent=retired, resume=True)
    if f or m:
        return None
    left = sorted(i for i, r in rows.items() if r.get('current_value_derivation') == LSA)
    body, route = K.read_band(box, 'off_band', os.path.join(OFF, f'band_{label}.json'), route=a.band_route, http=http)
    if body is None:
        return None
    return rows, left, K.rows_of(body), route


def cmd_verify_registry(a, fake=None, http=None):
    box = off_box(fake)
    so = K.jload(os.path.join(OFF, 'switchoff.json'))
    cand, retired_reg, reg = candidates_registry(a.registry)
    _check_base(a, reg)
    n = _verify_base(a, so, 'base_cursor', a.base_cursor)
    ch = since_base(box, n, a.cursor_now, 'verify', a.route)
    if ch is None:
        return 3
    retired = retired_reg | ch['retired'] | set(so.get('retired', []))
    pool = ch['lsa'] if ch['lsa'] is not None else ch['live']
    ids = sorted(set(so['ids']) | set(cand) | set(pool))
    got = _verify_finish(a, box, ids, retired, f'verify_{a.base_cursor}_{n}_{a.cursor_now}', http)
    if got is None:
        return 3
    rows, left, band, route = got
    ok = not left and not band
    print(f'switch-off verify (registry route, K {a.base_cursor}, build N {n}): {len(rows)} ids read ({len(ch["live"])} '
          f'changed in (N, N2], route {ch["route"]}), {len(left)} still lot-sum-awards {left[:10]}; band rows '
          f'{len(band)} (route {route}) -> {"PASS" if ok else "FAIL"}')
    return 0 if ok else 2


def cmd_build(a, fake=None):
    if a.registry:
        return cmd_build_registry(a, fake)
    if a.flip_cursor is None:
        raise SystemExit('switchoff.py build needs --registry R --base-cursor K (E4) or --flip-cursor C1 (the C1 route)')
    box = off_box(fake)
    cand, retired_reg = candidates()
    cls = since_flip(box, a.flip_cursor, a.cursor_now, 'build')
    if cls is None:
        return 3
    retired = retired_reg | cls['retired']
    key = f'c1_{a.flip_cursor}_{a.cursor_now}'
    rows, f, m = K.pk_read(box, cand, 'off_cand', os.path.join(OFF, f'cand_{key}'), expected_absent=retired,
                           resume=True)
    if f or m:
        return 3
    lsa = {i for i, r in rows.items() if r.get('current_value_derivation') == LSA}
    extra = set()
    ch = sorted(cls['live'] - set(cand))
    if ch:
        r2, f, m = K.pk_read(box, ch, 'off_changed', os.path.join(OFF, f'changed_{key}'), resume=True)
        if f or m:
            return 3
        extra = {i for i, r in r2.items() if r.get('current_value_derivation') == LSA}
    return write_off(a, sorted(lsa | extra), extra, cand, retired,
                     {'flip_cursor': a.flip_cursor, 'changed_since_flip': len(cls['live']), 'route': 'C1'})


def cmd_verify(a, fake=None, http=None):
    if a.registry:
        return cmd_verify_registry(a, fake, http)
    if a.flip_cursor is None:
        raise SystemExit('switchoff.py verify needs --registry R --base-cursor K (E4) or --flip-cursor C1 (the C1 route)')
    box = off_box(fake)
    so = K.jload(os.path.join(OFF, 'switchoff.json'))
    n = _verify_base(a, so, 'flip_cursor', a.flip_cursor)
    cand, retired_reg = candidates()
    cls = since_flip(box, n, a.cursor_now, 'verify') if a.cursor_now > n else K.classify_changes([])
    if cls is None:
        return 3
    retired = retired_reg | cls['retired'] | set(so.get('retired', []))
    ids = sorted(set(so['ids']) | set(cand) | cls['live'])
    got = _verify_finish(a, box, ids, retired, f'verify_c1_{a.flip_cursor}_{n}_{a.cursor_now}', http)
    if got is None:
        return 3
    rows, left, band, route = got
    ok = not left and not band
    print(f'switch-off verify: {len(rows)} ids read ({len(cls["live"])} changed in (N, N2]), {len(left)} still '
          f'lot-sum-awards {left[:10]}; band rows {len(band)} (route {route}) -> {"PASS" if ok else "FAIL"}')
    return 0 if ok else 2


def main(argv=None, fake=None, http=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    for name in ('build', 'verify'):
        p = sub.add_parser(name)
        p.add_argument('--flip-cursor', type=int, help='C1: the /health cursor right after the flip deploy (C1 route)')
        p.add_argument('--registry', help='E4: u9/registry/R.json (the registry route)')
        p.add_argument('--base-cursor', type=int, help='E4: K, the registry\'s checkpoint')
        p.add_argument('--force-base', action='store_true', help='E4: accept a --base-cursor that differs from R.json K')
        p.add_argument('--route', choices=['auto', 'e1', 'all'], default='auto', help='E4: the change reader for (K, N]')
        p.add_argument('--cursor-now', type=int, required=True, help='the /health cursor read now')
        if name == 'verify':
            p.add_argument('--band-route', choices=['sql', 'api'], default='sql')
    a = ap.parse_args(argv)
    if a.registry and a.base_cursor is None:
        raise SystemExit('--registry needs --base-cursor K (E4)')
    if a.registry and a.flip_cursor is not None:
        raise SystemExit('pass either the registry route (--registry/--base-cursor) or the C1 route (--flip-cursor)')
    if a.cmd == 'build':
        return cmd_build(a, fake)
    return cmd_verify(a, fake, http)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
