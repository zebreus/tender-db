"""Issue 551, path W on: A and A_d -- the dry refold-lot-sum admitted count over a segment's changed live Tenders
(decision-w-on-drains.md D.3, H, A8; 0 statements: dry jobs write nothing).

    python3 -I -B ajobs.py build <name> --from-seg SEG
    python3 -I -B ajobs.py build <name> --ids-file FILE
    python3 -I -B ajobs.py build days12 --archive [--after-cursor 959026689]
    python3 -I -B ajobs.py count <name> [--summary FILE ...]

build  writes u9/ajobs/<name>/A01_ids.json + A01_dry.json, ... ({"kind":"refold-lot-sum","tender_ids":[...]}, dry by
       default, <= 5,000 ids each) and manifest.json. Sources: a gd5 read's changed live ids (--from-seg), an ids file
       ({"ids": [...]}), or -- for U9 days 1-2 measured after the fact (H) -- the archived all-kind enumeration
       u9/20261023/build_changes_*.tar.xz: the live Tenders with a change row after --after-cursor. The default is U7's
       end (959026689, lc04's CA): the chains only, 9,975 Tenders in 2 jobs. --after-cursor 959004090 (C1) gives the
       decision's literal 13,453 (3 jobs), which also holds U7's own drained Tenders (U9 counts chain segments only).
count  parses each dry job's summary (`/root/aj.sh "/admin/jobs?limit=100" | jq -r '.recent[] | select(.job_id==ID)
       | .counts'`, one file per job IN JOB ORDER: A01, A02, ...) with u7/make_wet.parse: named must equal the job's
       size and found + not-admitted must equal named. Writes u9/ajobs/<name>/A.json {A = Σ found, complete, and the
       counted list's ids_sha256 / n / source} for gd5 check --a-from, which refuses an A whose ids_sha256 is not the
       segment's changed live ids (NOT A MEASUREMENT). A list of 0 ids has 0 jobs: count takes no --summary and
       writes A = 0, complete (cap 10) -- a class-W segment whose job changed no live Tender can still PASS.
       Exit 0; 2 a summary that does not fit its job (A.json complete = false).
"""
import argparse
import json
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wkit as W  # noqa: E402

K = W.K


def adir(name):
    return os.path.join(W.AJOBS_DIR, name)


def write_jobs(name, ids, source):
    d = adir(name)
    if os.path.exists(os.path.join(d, 'manifest.json')):
        raise SystemExit(f'{d}: A jobs exist for {name}; a measurement is built once')
    os.makedirs(d, exist_ok=True)
    jobs = []
    for k, part in enumerate(K.chunks(ids, 5000)):
        job = f'A{k + 1:02d}'
        K.jdump(os.path.join(d, f'{job}_ids.json'), {'run': job, 'ids': part, 'n': len(part),
                                                      'ids_sha256': K.ids_sha(part)})
        with open(os.path.join(d, f'{job}_dry.json'), 'w') as fh:
            fh.write(json.dumps({'kind': 'refold-lot-sum', 'tender_ids': part}, separators=(',', ':')) + '\n')
        jobs.append({'job': job, 'n': len(part)})
    K.jdump(os.path.join(d, 'manifest.json'), {'name': name, 'n': len(ids), 'ids_sha256': K.ids_sha(ids),
                                                'jobs': jobs, 'source': source, 'at': K.now_iso()})
    print(f'A jobs {name}: {len(ids)} changed live Tenders in {len(jobs)} dry job(s) {[j["job"] for j in jobs]} '
          f'({source}); POST each {d}/AXX_dry.json (dry), save each summary, then ajobs.py count {name}')
    return 0


def cmd_build(a):
    if a.from_seg:
        p = os.path.join(W.SEG_DIR, a.from_seg, 'read', 'changed_live_ids.json')
        ids = K.jload(p)['ids']
        return write_jobs(a.name, sorted(ids), f'gd5 read of {a.from_seg}: {p}')
    if a.ids_file:
        return write_jobs(a.name, sorted({int(i) for i in K.jload(a.ids_file)['ids']}), f'ids file {a.ids_file}')
    if a.archive:
        rows = [r for r in W.archived_changes() if int(r['cursor']) > a.after_cursor]
        cls = K.classify_changes(rows)
        return write_jobs(a.name, sorted(cls['live']),
                          f'archived all-kind enumeration, cursor > {a.after_cursor} (retired {len(cls["retired"])} '
                          f'left out)')
    raise SystemExit('build needs --from-seg, --ids-file or --archive')


def cmd_count(a):
    sys.path.insert(1, os.path.join(W.B6, 'u7'))
    import make_wet as MW
    d = adir(a.name)
    man = K.jload(os.path.join(d, 'manifest.json'))
    a.summary = a.summary or []
    if not man['jobs']:
        if a.summary or man['n'] != 0:
            raise SystemExit(f'{a.name}: a list of {man["n"]} ids with no jobs takes no --summary')
        cap, _ = W.cap_of(0)
        rec = {'name': a.name, 'A': 0, 'complete': True, 'per_job': [], 'n_named': 0, 'expected_fires': 0.0,
               'three_x': 0.0, 'cap_W': cap, 'ids_sha256': man['ids_sha256'], 'n': 0, 'source': man.get('source'),
               'at': K.now_iso()}
        K.jdump(os.path.join(d, 'A.json'), rec)
        print(f'A {a.name} = 0 (no changed live Tender, no dry job); class-W cap {cap:.2f} -> {d}/A.json')
        return 0
    if len(a.summary) != len(man['jobs']):
        raise SystemExit(f'{len(a.summary)} summaries for {len(man["jobs"])} jobs: one per job, in job order')
    per, ok, total = [], True, 0
    for job, sf in zip(man['jobs'], a.summary):
        with open(sf, encoding='utf-8') as fh:
            txt = fh.read()
        try:
            p = MW.parse(txt)
        except ValueError as e:
            per.append({'job': job['job'], 'error': str(e)})
            ok = False
            continue
        bad = []
        if p['named'] != job['n']:
            bad.append(f'named {p["named"]} != job size {job["n"]} (a different list was posted)')
        if p['found'] + p['not_admitted_n'] != p['named']:
            bad.append('found + not admitted != named')
        ok = ok and not bad
        total += p['found']
        per.append({'job': job['job'], 'n': job['n'], 'found': p['found'], 'notices': p['notices'],
                    'not_admitted': p['not_admitted_n'], 'problems': bad, 'summary_sha256': K.sha_file(sf)})
    x = total * W.RATE[0] / W.RATE[1]
    cap, _ = W.cap_of(total)
    rec = {'name': a.name, 'A': total, 'complete': ok, 'per_job': per, 'n_named': man['n'],
           'expected_fires': x, 'three_x': 3 * x, 'cap_W': cap, 'ids_sha256': man['ids_sha256'], 'n': man['n'],
           'source': man.get('source'), 'at': K.now_iso()}
    K.jdump(os.path.join(d, 'A.json'), rec)
    for j in per:
        print(f'  {j}')
    print(f'A {a.name} = {total} admitted of {man["n"]} named ({"complete" if ok else "INCOMPLETE"}); x 1.0974 % = '
          f'{x:.2f} (3x {3 * x:.2f}; class-W cap {cap:.2f}) -> {d}/A.json')
    return 0 if ok else 2


def main(argv=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    b = sub.add_parser('build')
    b.add_argument('name')
    b.add_argument('--from-seg')
    b.add_argument('--ids-file')
    b.add_argument('--archive', action='store_true')
    b.add_argument('--after-cursor', type=int, default=W.U7_END)
    c = sub.add_parser('count')
    c.add_argument('name')
    c.add_argument('--summary', action='append', default=[])
    a = ap.parse_args(argv)
    return cmd_build(a) if a.cmd == 'build' else cmd_count(a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
