"""Issue 551 B6 U7: the wet job JSON of one run, from its dry job's summary (expect = the dry's admitted count).

    python3 -I -B make_wet.py <run> --summary-file FILE [--jobs-dir DIR] [--now ISO]

FILE holds the dry job's summary text: `GET /admin/jobs?limit=N` lists finished jobs under `.recent[]`, and the text
is that job's `.counts` (RUNBOOK conventions: `/root/aj.sh "/admin/jobs?limit=100" | jq -r '.recent[] |
select(.job_id==<ID>) | .counts'`). There is no per-job GET.

The dry summary reads (supervisor.rs run_refold_lot_sum):
  "refold-lot-sum (issue 551) DRY RUN — nothing written: N of M named Tender(s) whose head could serve a derived lot
   sum, K notice(s); every named Tender is in the cohort; run it with ..."   or "...; J named Tender(s) not in the
   cohort (...): id,id,..."
Checks: M == the run's id count; found + not admitted == M; for stage1/stage2 no shown not-admitted id is of the
3,502 and the not-admitted count is at most the named ids outside the 3,502 (the summary lists at most 20 ids; past
that D1 -- a predicted id that does not fire -- is the check); on stage1, 7511890 is expected not admitted; for an
lcNN run the admitted count is D6's input.
Start rule / D7 (decision U7): for a drain run (stage*, lc*), the wet JSON is written only when start_rule.py says GO
for the runs still to do (stages.json order minus the runs u7/state/registry.json has checked, this one included);
NO-GO refuses (exit 2, no wet JSON). A switch-off run (off*, or --jobs-dir under u7/switchoff) is never held by the
start rule: switching off must always be possible. Writes jobs/<run>_wet.json and jobs/<run>_dry_result.json.
W mode (W-on drains decision, E6; u9/RUNBOOK-W.md): `--runs-left N` (with --minutes-per-run, --read-minutes,
--switchoff-gate covered|uncovered|from-clean or --switchoff-gate-minutes) evaluates the start rule for the W segment's
own runs instead of U7's stages.json, and skips U7's registry check: a probe (P0/P1) or a drain run under u9/segments/.
A run under u9/ (or any run not named stage*, lcNN, off*) WITHOUT --runs-left is a STOP: U7's stages.json would
otherwise judge it (all six U7 runs are done, so it would say GO for 0 runs left).
Exit 0; 2 STOP (read the output).
"""
import argparse
import json
import os
import re
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), 'u6'))
sys.path.insert(1, HERE)
import kitlib as K  # noqa: E402
import start_rule as SR  # noqa: E402

RX = re.compile(r'DRY RUN\s*\S\s*nothing written: (\d+) of (\d+) named Tender\(s\) whose head could serve a derived lot '
                r'sum, (\d+) notice\(s\)(.*?); run it with', re.S)
U7_RUN_RX = re.compile(r'^(stage\d+|lc\d\d|off\d*)$')
RX_LEFT = re.compile(r'(\d+) named Tender\(s\) not in the cohort \([^)]*\): ([0-9,…]+)')


def parse(summary):
    m = RX.search(summary)
    if not m:
        raise ValueError(f'not a scoped refold-lot-sum dry summary: {summary[:200]!r}')
    found, named, notices, tail = int(m.group(1)), int(m.group(2)), int(m.group(3)), m.group(4)
    left_n, left_ids, truncated = 0, [], False
    if 'every named Tender is in the cohort' not in tail:
        lm = RX_LEFT.search(tail)
        if not lm:
            raise ValueError(f'unparsed not-admitted clause: {tail[:200]!r}')
        left_n = int(lm.group(1))
        truncated = lm.group(2).endswith('…')
        left_ids = [int(x) for x in lm.group(2).rstrip('…').split(',') if x]
    return {'found': found, 'named': named, 'notices': notices, 'not_admitted_n': left_n,
            'not_admitted_shown': left_ids, 'not_admitted_truncated': truncated}


def runs_left(base=HERE):
    """-> (the drain runs still to do: stages.json order minus the runs the registry has checked, the runs done)."""
    order = K.jload(os.path.join(base, 'stages.json'))['order']
    reg_p = os.path.join(base, 'state', 'registry.json')
    done = {r['run'] for r in (K.jload(reg_p)['runs'] if os.path.exists(reg_p) else [])}
    return [r for r in order if r not in done], done


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('run')
    ap.add_argument('--summary-file', required=True)
    ap.add_argument('--jobs-dir', default=os.path.join(HERE, 'jobs'))
    ap.add_argument('--now', help='test only: the clock the start rule reads')
    ap.add_argument('--runs-left', type=int, help='W mode: the runs of this W segment still to do, this one included')
    SR.add_args(ap)
    a = ap.parse_args(argv)
    run = K.jload(os.path.join(a.jobs_dir, f'{a.run}_ids.json'))
    with open(a.summary_file, encoding='utf-8') as fh:
        summary = fh.read()
    try:
        d = parse(summary)
    except ValueError as e:
        print(f'STOP: {e}')
        return 2
    stops = []
    if d['named'] != run['n']:
        stops.append(f'the dry job named {d["named"]} Tenders, the run has {run["n"]}: wrong list posted')
    ids3502 = set(K.the_3502()[0])
    notes = []
    if a.run.startswith('stage'):
        outside = [i for i in run['ids'] if i not in ids3502]
        bad = [i for i in d['not_admitted_shown'] if i in ids3502]
        if bad:
            stops.append(f'ids of the 3,502 not admitted: {bad}')
        if d['not_admitted_n'] > len(outside):
            stops.append(f'{d["not_admitted_n"]} named ids not admitted but only {len(outside)} named ids are outside '
                         'the 3,502: at least one of the 3,502 is not in the cohort')
        if d['not_admitted_truncated']:
            notes.append(f'the not-admitted list is truncated at 20 ({d["not_admitted_n"]} in all); none shown is of '
                         'the 3,502; D1 (a predicted id that does not fire) is the check on the rest')
        if a.run == 'stage1' and K.NOT_REACHABLE in run['ids'] and not d['not_admitted_truncated'] \
                and K.NOT_REACHABLE not in d['not_admitted_shown']:
            notes.append('7511890 was ADMITTED (B3/2620 found it unreachable): read its post row')
        other = [i for i in d['not_admitted_shown'] if i != K.NOT_REACHABLE]
        if other:
            notes.append(f'not admitted, outside the 3,502 (pins whose head no lot sum can reach): {other}')
    if d['found'] + d['not_admitted_n'] != d['named']:
        stops.append('found + not admitted != named')
    switchoff = a.run.startswith('off') or os.path.abspath(a.jobs_dir).startswith(os.path.join(HERE, 'switchoff'))
    w_run = (os.path.abspath(a.jobs_dir).startswith(os.path.join(os.path.dirname(HERE), 'u9') + os.sep)
             or not U7_RUN_RX.match(a.run))
    if not switchoff and w_run and a.runs_left is None:
        stops.append(f'{a.run} is a W-mode run (jobs under u9/, or not a U7 stage*/lc*/off* run): W mode needs '
                     '--runs-left (with --minutes-per-run, --read-minutes, --switchoff-gate); U7\'s stages.json is '
                     'never its start rule')
    elif not switchoff and a.runs_left is not None:
        ev = SR.evaluate(a.runs_left, SR.parse_now(a.now), a.minutes_per_run, a.fixed_minutes, SR.gate_of(a),
                         a.read_minutes, a.switchoff_read_statements)
        print(f'  start rule (W mode, E6): {SR.line(ev, a.runs_left)}')
        if not ev['go']:
            stops.append(f'start rule NO-GO for the {a.runs_left} run(s) left of this W segment: do not post this wet '
                         f'run (HOLD: it has not started, so it moves to the next gap, after the {ev["tick_kind"]} tick '
                         f'{ev["tick"]:%Y-%m-%dT%H:%MZ} and its check; D7). A run already in flight is judged by '
                         f'u9/registry.py status (SWITCH OFF only for an open run that cannot finish)')
    elif not switchoff:
        left, done = runs_left()
        if a.run in done:
            stops.append(f'{a.run} was already checked (u7/state/registry.json): a wet run is posted once')
        ev = SR.evaluate(len(left), SR.parse_now(a.now))
        print(f'  start rule (D7): {SR.line(ev, len(left))}')
        if not ev['go']:
            stops.append(f'start rule NO-GO for the {len(left)} run(s) left {left}: do not post this wet run; if the '
                         f'drain has begun, switch off (RUNBOOK §S) so it finishes before {ev["tick"]:%Y-%m-%dT%H:%MZ}')
    for n in notes:
        print(f'  NOTE {n}')
    K.jdump(os.path.join(a.jobs_dir, f'{a.run}_dry_result.json'), dict(d, run=a.run, summary=summary.strip(),
                                                                         at=K.now_iso(), stops=stops, notes=notes))
    if stops:
        for s in stops:
            print('STOP', s)
        try:
            os.remove(os.path.join(a.jobs_dir, f'{a.run}_wet.json'))
        except OSError:
            pass
        return 2
    body = json.dumps({'kind': 'refold-lot-sum', 'tender_ids': run['ids'], 'dry_run': False, 'expect': d['found']},
                      separators=(',', ':'))
    with open(os.path.join(a.jobs_dir, f'{a.run}_wet.json'), 'w') as fh:
        fh.write(body + '\n')
    print(f'{a.run}: dry admitted {d["found"]} of {d["named"]} ({d["notices"]} notices); wrote {a.run}_wet.json '
          f'(expect {d["found"]})')
    return 0


if __name__ == '__main__':
    sys.exit(K.run_main(main))
