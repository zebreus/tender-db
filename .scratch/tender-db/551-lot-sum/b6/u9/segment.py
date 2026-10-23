"""Issue 551, path W on: single-writer segments (decision-w-on-drains.md B1 with W10, B6, D.1).

    python3 -I -B segment.py open   <seg> --class D|Z|N|W|I --tag TAG --cb-label L1 --cb-label2 L2 --jobs FILE
                                          --lock TEXT --runs-left N --minutes-per-run M --read-minutes R
                                          --switchoff-gate covered|uncovered|from-clean [--switchoff-read-statements S]
                                          [--expect-new N] [--expect-moves TEXT] [--salt SALT] [--bar TEXT]
                                          [--cursor CB] [--now ISO]
    python3 -I -B segment.py close  <seg> --ca-label L1 --ca-label2 L2 --jobs FILE --own ID[,ID...] [--corrected N]
                                          [--cursor CA] [--redo]
    python3 -I -B segment.py abort  <seg> --cb-label L1 --cb-label2 L2 --jobs FILE --reason TEXT
    python3 -I -B segment.py chain  <seg> --date YYYY-MM-DD --ca-label L1 --ca-label2 L2 --jobs FILE
                                          [--allow-missing fetch-rates|reveal-recheck ...] [--switchoff-gate G]
                                          [--jobs-mark ID] [--cursor CA] [--now ISO]
    python3 -I -B segment.py weekly <seg> --date YYYY-MM-DD --ca-label L1 --ca-label2 L2 --jobs FILE [...as chain]
    python3 -I -B segment.py gap    <seg> --ca-label L1 --ca-label2 L2 --jobs FILE --reason TEXT [--since-job ID]
    python3 -I -B segment.py probe-jobs <seg>
    python3 -I -B segment.py show   <seg>

FILE is a saved `GET /admin/jobs?limit=100` body (`ssh root@zebreus.click '/root/aj.sh "/admin/jobs?limit=100"' >
FILE`): {current, queued, recent[{job_id, kind, params, started_at, finished_at, outcome, counts}], measured_at}.
Cursors are never typed: L1 / L2 are u6/health.log labels (`u6/health.py --label L`), read just before and just after
saving FILE. Both must exist, read the same cursor, and bracket FILE's measured_at (+-120 s clock skew); --cursor is
only a cross-check that must equal it.

The jobs mark (registry `jobs_mark`: the CA jobs snapshot of the segment that last advanced K; at R0 K0's /health
read 10-23 18:48:01Z) is where every jobs window starts. A job since the mark that is not a refold-lot-sum DRY run (the
A / A_d jobs and dry posts write nothing; no other dry run is exempt) belongs to no closed segment. A saved body whose
recent[] (limit=100) no longer reaches back to the mark cannot prove the window single-writer: NOT A MEASUREMENT (3).

open   (offline) just before the probe's / drain's first POST that writes (539 step 1: before its dry run, which writes
       its mechanical rows as walked). Refused (exit 2 HOLD, or 10 when status says SWITCH OFF) unless: the registry
       says READY (no open segment, no NEW read pending, K not frozen); CB == K (writes in (K, CB] belong to no
       segment: close them first -- `gap`); no non-dry job since the mark (else `gap`); /admin/jobs idle; the start
       rule says GO for the segment's own runs (D.1: --runs-left / --minutes-per-run / --read-minutes / --switchoff-gate;
       its switch-off instant is recorded as the segment's read deadline and gap end); it is not the day-7 Berlin
       date 2026-10-28 before u9/day7/fetched.json exists (`day7.py fetched`); on a weekday, issue 222's TED catch-up
       loop has ended. The salt is derived from the tag: drain-<issue>-<run> -> '551-b6-drain-<issue>-' (a different
       --salt is refused); a probe-<rev> tag has none (null). Prints the writer-lock line.
close  (offline) after the job's project: `.current` and `.queued` empty (never read while a job runs), the two CA reads
       equal and bracketing the jobs body, CA >= CB -- every refusal leaves the segment OPEN, so it can be closed
       again. The jobs started in (CB, CA] must be EXACTLY --own (W10): a foreign job (another agent's, the weekly
       tick, a catch-up tick after a restart, a dry run not listed) or a missing own job is a STOP (exit 2). corrected
       is parsed from the own jobs' saved counts ("N corrected"); a typed --corrected must equal it. A Z segment whose
       own jobs report no corrected count takes --corrected 0 only when they are all repair-notice-instants. --redo
       re-closes a closed segment whose read has not run (a CA recorded wrongly).
abort  (offline) an open segment that wrote nothing (the wet POST refused, the start rule flipped): /health == CB == K
       (twice, around the jobs body), the queue idle, NO job since CB (a job that ran, even a failed one, takes the
       normal close / gd5 path). Clears open_segment without touching K.
chain  (offline) the U9 daily chain as a segment: it starts by the clock (09:35 Berlin) but its CHECK never does (B6):
       ready only when /admin/jobs shows that day's probe, process, fetch-rates, rederive-eur-recent, project and
       reveal-recheck after the tick (--allow-missing only fetch-rates / reveal-recheck: the two the supervisor skips
       when one is already pending), the queue empty and -- on a weekday -- issue 222's TED catch-up loop ended (no
       catch-up probe and measured >= tick + 7 min; or a trailing daily folded after it; or tick + 3 h 07 passed).
       A job after the tick that is not a chain kind is foreign (STOP); every non-dry job between the mark and the tick
       is foreign too (another agent's writer, a catch-up tick after a restart, the weekly jobs of an unclosed weekly
       range): STOP, explained in 551. Refused while K is frozen.
weekly (offline) the Sunday 03:10 Berlin report tick's range as its own class-Z segment (W3): ready only when the jobs
       body was measured after the tick and at least one weekly job started after it (with one queue, a weekly kind
       already pending still starts after the tick); pre-tick jobs as for chain.
gap    (offline) writes or jobs since the mark that belong to no segment (open refused): every non-dry job since the
       mark is recorded as foreign (with none, CA > K is an 'unattributed' writer); the record is class Z with
       full_required (no zero-change shortcut: D.2's own-jobs condition cannot hold) and corrected unknown. Exit 2:
       gd5 then STOPs on the foreign jobs until 551 explains them (foreign_jobs); an in-place W-input kind also needs
       its C9' pre-step (w_input_prestep + --prediction) or it is SWITCH OFF.
probe-jobs  the job files of a W-on probe (B4: refold-lot-sum over R ∪ Rg ∪ pins, one run of <= 5,000).
Exit 0; 2 HOLD / STOP / not ready (read the line); 10 SWITCH OFF (open, from status); 3 a tool fault / not a
measurement.
"""
import argparse
import datetime
import json
import os
import re
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wkit as W  # noqa: E402
import registry as REG  # noqa: E402
import start_rule as SR  # noqa: E402

K = W.K
CHAIN_KINDS = ('probe', 'process', 'fetch-rates', 'rederive-eur-recent', 'project', 'reveal-recheck')
CHAIN_ALLOWED = CHAIN_KINDS + ('fetch',)          # a TED probe may queue fetches
CHAIN_SKIPPABLE = ('fetch-rates', 'reveal-recheck')   # supervisor.rs enqueue_daily: already_pending guards
WEEKLY_KINDS = ('disk-census', 'ghost-census', 'member-twin-census', 'registry-contiguity', 'data-quality',
                'rehash-probe', 'build-org-match-keys', 'org-merge-health', 'scan-org-match-keys', 'audit-fts-ids')
TAG_RX = re.compile(r'^(drain-(\d+)-[a-z0-9]+|probe-[0-9a-f]{7,40})$')
HEALTH_LOG = os.path.join(K.U6, 'health.log')
SKEW = 120                                        # s: health.log is container time, measured_at the box's
CATCHUP_PARAMS = 'ted daily (catch-up)'           # supervisor.rs issue 222
TICK_PROBE_PARAMS = 'ted daily (probe)'
CATCHUP_POLL, CATCHUP_WINDOW, CATCHUP_MARGIN = 300, 3 * 3600, 120
DAY7_FETCHED = os.path.join(W.U9, 'day7', 'fetched.json')


def seg_dir(seg):
    if not re.match(r'^[A-Za-z0-9._-]+$', seg):
        raise SystemExit(f'segment name {seg!r}: letters, digits, . _ - only')
    return os.path.join(W.SEG_DIR, seg)


def rec_path(seg):
    return os.path.join(seg_dir(seg), 'segment.json')


def load_rec(seg):
    p = rec_path(seg)
    if not os.path.exists(p):
        raise SystemExit(f'{p}: no segment record')
    return K.jload(p)


def jobs_snapshot(path):
    j = K.jload(path)
    for k in ('current', 'queued', 'recent', 'measured_at'):
        if k not in j:
            raise SystemExit(f'{path}: not an /admin/jobs body (no {k!r})')
    recent = j['recent'] or []
    ids = [int(r['job_id']) for r in recent if r.get('job_id') is not None]
    return {'measured_at': j.get('measured_at'), 'current': j['current'], 'queued': j['queued'],
            'max_job_id': max(ids) if ids else None, 'recent': recent, 'file_sha256': K.sha_file(path),
            'file': path}


def idle(snap):
    return not snap['current'] and not snap['queued']


def job_rec(r):
    return {'job_id': r.get('job_id'), 'kind': r.get('kind'), 'params': r.get('params'),
            'started_at': r.get('started_at'), 'finished_at': r.get('finished_at'), 'outcome': r.get('outcome'),
            'counts': r.get('counts') or ''}


def started_after(snap, max_job_id, measured_at):
    out = []
    for r in snap['recent']:
        jid = r.get('job_id')
        if (jid is not None and max_job_id is not None and int(jid) > int(max_job_id)) or \
                (measured_at is not None and int(r.get('started_at') or 0) > int(measured_at)):
            out.append(job_rec(r))
    return out


def slim_snap(s):
    return {k: s[k] for k in ('measured_at', 'current', 'queued', 'max_job_id', 'file_sha256')}


# ---------------------------------------------------------------- cursors from u6/health.log (never typed)
def health_read(label):
    if not label:
        raise SystemExit('a /health label is required (u6/health.py --label L)')
    best = None
    try:
        with open(HEALTH_LOG, encoding='utf-8') as fh:
            for ln in fh:
                parts = ln.split()
                if len(parts) >= 3 and parts[1] == label:
                    best = parts
    except FileNotFoundError:
        pass
    if best is None:
        raise SystemExit(f'{HEALTH_LOG}: no /health line labelled {label!r} (read it: python3 -I -B u6/health.py '
                         f'--label {label})')
    kv = dict(p.split('=', 1) for p in best[2:] if '=' in p)
    if kv.get('cursor') in (None, 'None'):
        raise SystemExit(f'{HEALTH_LOG} {label}: no cursor on the line')
    return {'label': label, 'at': best[0], 'epoch': K.epoch_of(best[0]), 'cursor': int(kv['cursor']),
            'rev': kv.get('rev'), 'line': ' '.join(best)}


def bracket(l1, l2, snap, cross=None):
    """-> (cursor, [problems]): two /health reads around the saved jobs body, equal, l1 <= measured_at <= l2."""
    probs = []
    if l1['cursor'] != l2['cursor']:
        probs.append(f'the two /health reads differ ({l1["label"]} {l1["cursor"]} != {l2["label"]} {l2["cursor"]}): a '
                     'job wrote between them; read both again around a fresh jobs body')
    m = snap.get('measured_at')
    if m is None or not (l1['epoch'] - SKEW <= int(m) <= l2['epoch'] + SKEW):
        probs.append(f'the jobs body (measured_at {m}) is not between {l1["label"]} ({l1["at"]}) and {l2["label"]} '
                     f'({l2["at"]}) (+-{SKEW} s): read /health, save the jobs body, read /health again')
    if cross is not None and cross != l1['cursor']:
        probs.append(f'--cursor {cross} != the logged /health cursor {l1["cursor"]}')
    return l1['cursor'], probs


# ---------------------------------------------------------------- the jobs mark and the window since it
def jobs_mark(reg, override=None):
    m = dict(reg.get('jobs_mark') or {})
    if not m and reg.get('history'):
        try:
            cj = load_rec(reg['history'][-1]['seg']).get('ca_jobs') or {}
            m = {'seg': reg['history'][-1]['seg'], 'max_job_id': cj.get('max_job_id'),
                 'measured_at': cj.get('measured_at')}
        except SystemExit:
            m = {}
    if override is not None:
        m = {'seg': 'override', 'max_job_id': int(override), 'measured_at': None}
    return m or None


def covers(snap, mark):
    """recent[] (limit=100) reaches back to the mark (else a job since it may have rolled off)."""
    ids = [int(r['job_id']) for r in snap['recent'] if r.get('job_id') is not None]
    if mark.get('max_job_id') is not None:
        return bool(ids) and min(ids) <= int(mark['max_job_id']) + 1
    at = mark.get('measured_at')
    return at is not None and any(int(r.get('started_at') or 0) <= int(at) for r in snap['recent'])


def since_mark(snap, mark):
    jobs = started_after(snap, mark.get('max_job_id'), mark.get('measured_at'))
    return [j for j in jobs if not W.is_dry_job(j)], [j for j in jobs if W.is_dry_job(j)]


def not_covered(snap, mark):
    print(f'NOT A MEASUREMENT: the saved /admin/jobs?limit=100 body no longer reaches back to the jobs mark {mark} '
          f'(oldest job {min((int(r["job_id"]) for r in snap["recent"] if r.get("job_id") is not None), default=None)}):'
          ' a job since the mark may have rolled off; save a body that covers it (or record the window in 551)')
    return W.NOT_MEASURED


# ---------------------------------------------------------------- issue 222's weekday catch-up loop
def catchup_state(snap, tick):
    """-> (ready, state). The supervisor re-probes TED every 5 min for 3 h after a weekday tick when the package has
    not landed, then runs a second daily: (a) the loop never entered (no catch-up probe, measured >= tick + 7 min);
    (b) it ended with a late fold (a trailing 'ted daily (probe)' after the last catch-up probe, its project after
    it); (c) its window is over (measured >= tick + 3 h 07). Anything else: not ready."""
    t0 = int(tick.timestamp())
    m = int(snap.get('measured_at') or 0)
    rec = [r for r in snap['recent'] if int(r.get('started_at') or 0) >= t0]
    cu = [r for r in rec if r.get('params') == CATCHUP_PARAMS]
    pend = [q for q in list(snap.get('queued') or []) + ([snap['current']] if snap.get('current') else [])
            if (q or {}).get('params') == CATCHUP_PARAMS]
    if pend:
        return False, 'a catch-up probe is queued or running'
    if not cu:
        if m >= t0 + CATCHUP_POLL + CATCHUP_MARGIN:
            return True, '(a) the catch-up loop never entered'
        return False, f'measured {m - t0} s after the tick: the loop\'s first check is at +300 s; wait for +420 s'
    last = max(int(r['job_id']) for r in cu)
    trail = [r for r in rec if int(r.get('job_id') or 0) > last and r.get('params') == TICK_PROBE_PARAMS]
    if trail:
        tp = min(int(r['job_id']) for r in trail)
        if any(r.get('kind') == 'project' and int(r.get('job_id') or 0) > tp for r in rec):
            return True, f'(b) a late daily folded after catch-up probe {last} (trailing probe {tp})'
    if m >= t0 + CATCHUP_WINDOW + CATCHUP_POLL + CATCHUP_MARGIN:
        return True, '(c) the catch-up window is over'
    return False, (f'catch-up probe(s) {[int(r["job_id"]) for r in cu]} ran and no trailing daily yet: the loop may '
                   'still push a second daily until tick + 3 h 05')


def weekday_tick(day):
    return day.weekday() < 5


def salt_of(tag, given):
    m = TAG_RX.match(tag)
    if m and m.group(2):
        want = f'551-b6-drain-{m.group(2)}-'
        if given is not None and given != want:
            raise SystemExit(f'--salt {given!r} != the tag\'s pre-registered salt {want!r} (D.1)')
        return want
    if given is not None:
        raise SystemExit(f'--salt {given!r} on a probe tag: 551 defines no probe salt (record one there first)')
    return None


def day7_hold(day):
    if day.isoformat() != W.DAY7_DATE:
        return None
    if not os.path.exists(DAY7_FETCHED):
        return (f'HOLD: it is {W.DAY7_DATE} (Berlin) and the day-7 fetch is not recorded ({DAY7_FETCHED}; '
                'day7.py fetched): no writer until the day-7 bodies are fetched (B1, F HOLD)')
    try:
        f = K.jload(DAY7_FETCHED)
        d7 = os.path.join(os.path.dirname(DAY7_FETCHED), 'day7.json')
        if f.get('day7_sha256') != K.sha_file(d7) or not f.get('complete'):
            return f'HOLD: {DAY7_FETCHED} does not match the current day7.json (or is incomplete): run day7.py fetched'
    except (OSError, ValueError) as e:
        return f'HOLD: {DAY7_FETCHED} unreadable ({e})'
    return None


def berlin_date(epoch):
    return datetime.datetime.fromtimestamp(int(epoch), datetime.timezone.utc).astimezone(SR._berlin()).date()


# ---------------------------------------------------------------- open / close / abort
def cmd_open(a):
    reg = REG.load()
    code, lines = REG.status(reg)
    if code != W.PASS:
        for ln in lines:
            print(ln)
        print(f'{"SWITCH OFF" if code == W.SWITCH_OFF else "HOLD"}: {a.seg} may not open')
        return code
    if a.klass not in W.SEG_CLASSES:
        raise SystemExit(f'--class {a.klass}: open is for {W.SEG_CLASSES}; chain/weekly have their own subcommands')
    if not TAG_RX.match(a.tag):
        raise SystemExit(f'--tag {a.tag!r}: drain-<issue>-<run> or probe-<rev> (B2 source tags)')
    salt = salt_of(a.tag, a.salt)
    if os.path.exists(rec_path(a.seg)):
        print(f'REFUSED: {rec_path(a.seg)} exists; a segment opens once')
        return 2
    snap = jobs_snapshot(a.jobs)
    if not idle(snap):
        print(f'HOLD: /admin/jobs is not idle at CB (current {snap["current"]}, queued {len(snap["queued"])}): B1 needs '
              '.current and .queued empty before the wet POST')
        return 2
    l1, l2 = health_read(a.cb_label), health_read(a.cb_label2)
    cb, probs = bracket(l1, l2, snap, a.cursor)
    if probs:
        for pr in probs:
            print(f'HOLD: {pr}')
        return 2
    if cb != int(reg['K']):
        print(f'HOLD: CB {cb} != K {reg["K"]}: the writes in ({reg["K"]}, {cb}] belong to no closed segment (a chain '
              'whose U9 check has not run, the weekly tick, another writer): close them first (segment.py gap)')
        return 2
    mark = jobs_mark(reg)
    if mark is None:
        print('HOLD: the registry has no jobs mark (an older R.json): re-init or record the mark')
        return 2
    if not covers(snap, mark):
        return not_covered(snap, mark)
    wet, dry = since_mark(snap, mark)
    if wet:
        print(f'HOLD: job(s) since the last closed segment ({mark.get("seg")}) that belong to no segment: '
              f'{[(j["job_id"], j["kind"]) for j in wet]} -- close them with segment.py gap (B1/W10)')
        return 2
    h = day7_hold(berlin_date(SR.parse_now(a.now).timestamp() if a.now else snap['measured_at']))
    if h:
        print(h)
        return 2
    mday = berlin_date(snap['measured_at'])
    tick_today = SR.berlin_at(mday, *SR.CHAIN_TICK)
    if weekday_tick(mday) and int(tick_today.timestamp()) <= int(snap['measured_at']):
        ready, st = catchup_state(snap, tick_today)
        if not ready:
            print(f'HOLD: today\'s TED catch-up loop (issue 222) may still write: {st}')
            return 2
    if a.klass in ('W', 'I') and a.read_minutes is None:
        raise SystemExit(f'class {a.klass}: --read-minutes is required (the NEW reads must finish before the next '
                         'writer, W9: about 45 for <= 30 heads)')
    now = SR.parse_now(a.now)
    gate = SR.gate_of(a)
    if gate is None:
        raise SystemExit('--switchoff-gate (or --switchoff-gate-minutes) is required (B3: covered 0 / uncovered 35)')
    ev = SR.evaluate(a.runs_left, now, a.minutes_per_run, a.fixed_minutes, gate, a.read_minutes or 0,
                     a.switchoff_read_statements)
    print(f'  start rule (D.1): {SR.line(ev, a.runs_left)}')
    if not ev['go']:
        print(f'HOLD: start rule NO-GO for the {a.runs_left} run(s) of {a.seg}: it moves to the next gap (D7)')
        return 2
    fmt = '%Y-%m-%dT%H:%M:%SZ'
    rec = {'seg': a.seg, 'class': a.klass, 'tag': a.tag, 'status': 'open', 'K_open': int(reg['K']), 'CB': cb,
           'cb_jobs': slim_snap(snap), 'cb_health': [l1['line'], l2['line']], 'dry_since_mark': dry,
           'lock': a.lock, 'opened_at': now.strftime(fmt), 'minutes_per_run': a.minutes_per_run,
           'read_minutes': a.read_minutes or 0, 'runs_left': a.runs_left,
           'switchoff_by': ev['switchoff_by'].strftime(fmt), 'tick': ev['tick'].strftime(fmt),
           'tick_kind': ev['tick_kind'], 'deadline': ev['deadline'].strftime(fmt),
           'prereg': {'class': a.klass, 'expected_new': a.expect_new, 'expected_moves': a.expect_moves,
                      'salt': salt, 'bar': a.bar}}
    os.makedirs(seg_dir(a.seg), exist_ok=True)
    K.jdump(rec_path(a.seg), rec)
    reg['open_segment'] = a.seg
    REG.save(reg)
    print(f'{a.seg} OPEN (class {a.klass}, tag {a.tag}) at CB = K = {cb}; /admin/jobs idle (newest job '
          f'{snap["max_job_id"]}); latest switch-off start {rec["switchoff_by"]} ({ev["tick_kind"]} tick {rec["tick"]})')
    print(f'  writer lock (551 and the drain\'s issue, before the wet POST): {a.lock}')
    print(f'  pre-registered: expected NEW {a.expect_new}; served moves {a.expect_moves!r}; salt {salt!r}; '
          f'bar {a.bar!r}')
    return 0


def own_corrected(started, own):
    vals = [W.parse_corrected(s.get('counts')) for s in started if s['job_id'] is not None and int(s['job_id']) in own]
    vals = [v for v in vals if v is not None]
    return sum(vals) if vals else None


def cmd_close(a):
    rec = load_rec(a.seg)
    if rec['status'] == 'closed' and a.redo:
        if os.path.exists(os.path.join(seg_dir(a.seg), 'read')) or os.path.exists(
                os.path.join(seg_dir(a.seg), 'result.json')):
            print(f'REFUSED: {a.seg}: its read has run; a re-close would change the range under it')
            return 2
    elif rec['status'] != 'open':
        print(f'REFUSED: {a.seg} is {rec["status"]}' + (' (--redo re-closes it while its read has not run)'
                                                        if rec['status'] == 'closed' else ''))
        return 2
    snap = jobs_snapshot(a.jobs)
    if not idle(snap):
        print(f'NOT YET: /admin/jobs is not idle (current {snap["current"]}, queued {len(snap["queued"])}): read CA '
              'after the project, never while a job runs (the segment stays open)')
        return 2
    l1, l2 = health_read(a.ca_label), health_read(a.ca_label2)
    ca, probs = bracket(l1, l2, snap, a.cursor)
    if ca < int(rec['CB']):
        probs.append(f'CA {ca} < CB {rec["CB"]}')
    if probs:
        for pr in probs:
            print(f'NOT YET: {pr} (the segment stays open)')
        return 2
    started = started_after(snap, rec['cb_jobs']['max_job_id'], rec['cb_jobs']['measured_at'])
    own = sorted(int(x) for x in a.own.split(',') if x.strip())
    got = sorted(int(s['job_id']) for s in started if s['job_id'] is not None)
    foreign = [s for s in started if s['job_id'] is None or int(s['job_id']) not in own]
    missing = sorted(set(own) - set(got))
    parsed = own_corrected(started, own)
    corrected = a.corrected
    if parsed is not None:
        if a.corrected is not None and a.corrected != parsed:
            print(f'REFUSED: --corrected {a.corrected} != {parsed}, the own jobs\' saved counts (the segment stays open)')
            return 2
        corrected = parsed
    elif rec['class'] == 'Z' and a.corrected == 0:
        kinds = {s['kind'] for s in started if s['job_id'] is not None and int(s['job_id']) in own}
        if not kinds or kinds - {'repair-notice-instants'}:
            print(f'REFUSED: --corrected 0 on a Z segment whose own jobs {sorted(kinds)} report no corrected count: '
                  'only repair-notice-instants (which re-projects nothing) takes a typed 0 (the segment stays open)')
            return 2
    verdict = 'STOP' if (foreign or missing) else 'PASS'
    rec.update(status='closed', CA=ca, ca_jobs=slim_snap(snap), ca_health=[l1['line'], l2['line']], started=started,
               own=own, foreign=foreign, missing_own=missing, corrected=corrected, corrected_parsed=parsed,
               jobs_verdict=verdict, closed_at=K.now_iso())
    K.jdump(rec_path(a.seg), rec)
    print(f'{a.seg} CLOSED at CA {ca}: jobs in (CB, CA] {[(s["job_id"], s["kind"]) for s in started]}; own {own}; '
          f'corrected {corrected}' + (' (from the saved counts)' if parsed is not None else ''))
    if foreign:
        print(f'STOP: foreign job(s) in (CB, CA] (W10): {[(s["job_id"], s["kind"]) for s in foreign]} -- another writer '
              'ran inside the segment; gd5 check reads STOP')
    if missing:
        print(f'STOP: own job(s) {missing} not among the jobs started in (CB, CA]')
    return 0 if verdict == 'PASS' else 2


def cmd_abort(a):
    rec = load_rec(a.seg)
    reg = REG.load()
    if rec['status'] != 'open':
        print(f'REFUSED: {a.seg} is {rec["status"]}: only an open segment that wrote nothing is aborted')
        return 2
    if reg.get('open_segment') != a.seg:
        print(f'REFUSED: the registry\'s open segment is {reg.get("open_segment")}, not {a.seg}')
        return 2
    if reg.get('frozen') and reg['frozen'].get('seg') != a.seg:
        print(f'REFUSED: K is frozen by {reg["frozen"]["seg"]}')
        return 2
    snap = jobs_snapshot(a.jobs)
    if not idle(snap):
        print('REFUSED: /admin/jobs is not idle')
        return 2
    l1, l2 = health_read(a.cb_label), health_read(a.cb_label2)
    c, probs = bracket(l1, l2, snap)
    if c != int(rec['CB']) or c != int(reg['K']):
        probs.append(f'/health {c} != CB {rec["CB"]} / K {reg["K"]}: something wrote')
    started = started_after(snap, rec['cb_jobs']['max_job_id'], rec['cb_jobs']['measured_at'])
    if started:
        probs.append(f'jobs ran since CB {[(s["job_id"], s["kind"]) for s in started]}: close it and run gd5 instead')
    if probs:
        for pr in probs:
            print(f'REFUSED: {pr}')
        return 2
    rec.update(status='aborted', aborted_at=K.now_iso(), abort_reason=a.reason, abort_jobs=slim_snap(snap),
               abort_health=[l1['line'], l2['line']])
    K.jdump(rec_path(a.seg), rec)
    reg['open_segment'] = None
    REG.save(reg)
    print(f'{a.seg} ABORTED ({a.reason}): nothing was written (CB = K = {c}, no job since CB); K unchanged. Clear the '
          'writer-lock line in 551 and on the drain\'s issue.')
    return 0


# ---------------------------------------------------------------- chain / weekly / gap
def _record_deadline(a):
    now = SR.parse_now(getattr(a, 'now', None))
    gate = SR.gate_of(a)
    ev = SR.evaluate(0, now, switchoff_gate_minutes=SR.GATE_MINUTES['uncovered'] if gate is None else gate)
    return ev


def _tick_record(a, klass, tag, tick, kinds_needed, allowed):
    reg = REG.load()
    if reg.get('frozen'):
        print(f'HOLD: K is frozen by {reg["frozen"]["seg"]} ({reg["frozen"]["verdict"]}); resolve it first: the {klass} '
              f'check covers (K, CA] and would absorb that segment\'s writes')
        return 2, None
    if reg.get('open_segment') and reg['open_segment'] != a.seg:
        print(f'HOLD: segment {reg["open_segment"]} is open')
        return 2, None
    snap = jobs_snapshot(a.jobs)
    if not idle(snap):
        print(f'NOT READY: /admin/jobs is not idle (current {snap["current"]}, queued {len(snap["queued"])}): the check '
              'never starts by the clock (B6)')
        return 2, None
    l1, l2 = health_read(a.ca_label), health_read(a.ca_label2)
    ca, probs = bracket(l1, l2, snap, a.cursor)
    if ca < int(reg['K']):
        probs.append(f'CA {ca} < K {reg["K"]}')
    if probs:
        for pr in probs:
            print(f'NOT READY: {pr}')
        return 2, None
    mark = jobs_mark(reg, a.jobs_mark)
    if mark is None:
        print('HOLD: no jobs mark (pass --jobs-mark <the newest job id at K, as recorded in 551>)')
        return 2, None
    if not covers(snap, mark):
        return not_covered(snap, mark), None
    t0 = int(tick.timestamp())
    if klass == 'weekly' and int(snap['measured_at']) < t0:
        print(f'NOT READY: the jobs body was measured before the weekly tick {tick:%Y-%m-%dT%H:%MZ} (W3: the weekly '
              'range is closed after its jobs, never before)')
        return 2, None
    after = [r for r in snap['recent'] if int(r.get('started_at') or 0) >= t0]
    kinds = {r.get('kind') for r in after}
    missing = [k for k in kinds_needed if k not in kinds and k not in (a.allow_missing or [])]
    if missing:
        print(f'NOT READY: no {missing} job finished after the {klass} tick {tick:%Y-%m-%dT%H:%MZ}')
        return 2, None
    if klass == 'weekly' and not (kinds & set(WEEKLY_KINDS)):
        print(f'NOT READY: no weekly job started after the tick {tick:%Y-%m-%dT%H:%MZ} (with one queue a weekly kind '
              'already pending still starts after it): wait for the weekly jobs')
        return 2, None
    cu_state = None
    if klass == 'chain' and weekday_tick(tick.astimezone(SR._berlin()).date()):
        ready, cu_state = catchup_state(snap, tick)
        if not ready:
            print(f'NOT READY: issue 222\'s weekday TED catch-up loop may still push a daily: {cu_state}')
            return 2, None
    pre_all = [j for j in started_after(snap, mark.get('max_job_id'), mark.get('measured_at'))
               if int(j.get('started_at') or 0) < t0]
    pre_dry = [j for j in pre_all if W.is_dry_job(j)]
    pre_wet = [dict(j, pre_tick=True) for j in pre_all if not W.is_dry_job(j)]
    foreign = [job_rec(r) for r in after if r.get('kind') not in allowed] + pre_wet
    bad = [{'job_id': r.get('job_id'), 'kind': r.get('kind'), 'outcome': r.get('outcome')} for r in after
           if r.get('outcome') not in ('ok', None)]
    weekly_pre = [j for j in pre_wet if j.get('kind') in WEEKLY_KINDS]
    ev = _record_deadline(a)
    fmt = '%Y-%m-%dT%H:%M:%SZ'
    rec = {'seg': a.seg, 'class': klass, 'tag': tag, 'status': 'closed', 'K_open': int(reg['K']), 'CB': None,
           'CA': ca, 'tick': tick.strftime(fmt), 'ca_jobs': slim_snap(snap), 'ca_health': [l1['line'], l2['line']],
           'started': [job_rec(r) for r in after], 'pre_tick_dry': pre_dry, 'jobs_mark': mark,
           'foreign': foreign, 'missing_own': [], 'non_ok': bad, 'jobs_verdict': 'STOP' if foreign else 'PASS',
           'allowed_missing': sorted(a.allow_missing or []) if klass == 'chain' else [],
           'catchup': cu_state, 'weekly_unclosed': weekly_pre,
           'switchoff_by': ev['switchoff_by'].strftime(fmt), 'next_tick': ev['tick'].strftime(fmt),
           'closed_at': K.now_iso(), 'date': a.date}
    os.makedirs(seg_dir(a.seg), exist_ok=True)
    K.jdump(rec_path(a.seg), rec)
    reg['open_segment'] = a.seg
    REG.save(reg)
    print(f'{a.seg} ({klass} {a.date}, tick {tick:%Y-%m-%dT%H:%MZ}): READY for gd5 over (K {reg["K"]}, CA {ca}]; '
          f'{len(after)} jobs after the tick {W.counts(r.get("kind") for r in after)}'
          + (f'; catch-up: {cu_state}' if cu_state else '') + f'; gap ends {rec["switchoff_by"]}')
    if bad:
        print(f'NOTE: jobs that did not end ok: {bad}')
    if pre_dry:
        print(f'NOTE: dry refold-lot-sum jobs since the mark (write nothing): {[j["job_id"] for j in pre_dry]}')
    if weekly_pre:
        print(f'STOP: the weekly range was not closed on its own (W3): its jobs {[j["job_id"] for j in weekly_pre]} ran '
              f'before this {klass} tick and are foreign here -- explain them in 551 (foreign_jobs) and read every fire '
              'of this check before advance')
    if foreign:
        print(f'STOP: jobs that are not this {klass}\'s own (after the tick: not a {klass} kind; between the mark and '
              f'the tick: any non-dry job): {[(j["job_id"], j["kind"]) for j in foreign]}')
        return 2, rec
    return 0, rec


def cmd_chain(a):
    d = datetime.date.fromisoformat(a.date)
    tick = SR.berlin_at(d, *SR.CHAIN_TICK)
    code, _ = _tick_record(a, 'chain', f'chain-{a.date}', tick, CHAIN_KINDS, CHAIN_ALLOWED)
    return code


def cmd_weekly(a):
    d = datetime.date.fromisoformat(a.date)
    if d.weekday() != SR.WEEKLY_TICK[0]:
        raise SystemExit(f'{a.date} is not a Sunday')
    tick = SR.berlin_at(d, SR.WEEKLY_TICK[1], SR.WEEKLY_TICK[2])
    a.allow_missing = []
    code, _rec = _tick_record(a, 'weekly', f'weekly-{a.date}', tick, (), WEEKLY_KINDS)
    return code


def cmd_gap(a):
    reg = REG.load()
    if reg.get('frozen') or reg.get('open_segment'):
        print(f'HOLD: frozen {reg.get("frozen")} / open segment {reg.get("open_segment")}')
        return 2
    if os.path.exists(rec_path(a.seg)):
        print(f'REFUSED: {rec_path(a.seg)} exists')
        return 2
    snap = jobs_snapshot(a.jobs)
    if not idle(snap):
        print('NOT READY: /admin/jobs is not idle (never read while a job runs)')
        return 2
    l1, l2 = health_read(a.ca_label), health_read(a.ca_label2)
    ca, probs = bracket(l1, l2, snap, a.cursor)
    if probs:
        for pr in probs:
            print(f'NOT READY: {pr}')
        return 2
    mark = jobs_mark(reg, a.since_job)
    if mark is None:
        print('HOLD: no jobs mark: pass --since-job <the newest job id at K, as recorded in 551>')
        return 2
    if not covers(snap, mark):
        return not_covered(snap, mark)
    wet, dry = since_mark(snap, mark)
    if ca < int(reg['K']):
        print(f'REFUSED: CA {ca} < K {reg["K"]}')
        return 2
    if ca == int(reg['K']) and not wet:
        print(f'nothing to close: CA {ca} == K and no job since the mark')
        return 2
    foreign = wet or [{'job_id': None, 'kind': 'unattributed', 'note': f'CA {ca} > K {reg["K"]} with no job since the '
                                                                          f'mark {mark}'}]
    w_input = [j for j in wet if W.is_w_input_kind(j.get('kind'))]
    ev = _record_deadline(a)
    rec = {'seg': a.seg, 'class': 'Z', 'tag': f'gap-{a.seg}', 'status': 'closed', 'K_open': int(reg['K']), 'CB': None,
           'CA': ca, 'ca_jobs': slim_snap(snap), 'ca_health': [l1['line'], l2['line']], 'jobs_mark': mark,
           'started': wet, 'dry_since_mark': dry, 'own': [], 'foreign': foreign, 'missing_own': [], 'corrected': None,
           'full_required': True, 'w_input': w_input, 'jobs_verdict': 'STOP', 'reason': a.reason,
           'switchoff_by': ev['switchoff_by'].strftime('%Y-%m-%dT%H:%M:%SZ'), 'closed_at': K.now_iso()}
    os.makedirs(seg_dir(a.seg), exist_ok=True)
    K.jdump(rec_path(a.seg), rec)
    reg['open_segment'] = a.seg
    REG.save(reg)
    print(f'{a.seg}: the range ({reg["K"]}, {ca}] and the jobs since the mark closed as class Z ({a.reason}); foreign '
          f'{[(j.get("job_id"), j.get("kind")) for j in foreign]}')
    print(f'STOP: a writer that belongs to no segment (B1/W10): record it in 551 (foreign_jobs), then gd5.py read {a.seg} '
          '--class Z (always the full read) and check --explained'
          + (f'; in-place W-input kind(s) {[j["kind"] for j in w_input]}: without their C9\' pre-step '
             '(w_input_prestep + --prediction) the check is SWITCH OFF' if w_input else ''))
    return 2


def cmd_probe_jobs(a):
    reg = REG.load()
    ids = sorted({int(i) for i in reg['entries']} | {int(i) for i in reg['watch']} | set(K.pin_ids()))
    if len(ids) > 5000:
        raise SystemExit(f'{len(ids)} ids: a probe is one run of <= 5,000 (B4); split it and re-check D7')
    jobs = os.path.join(seg_dir(a.seg), 'jobs')
    os.makedirs(jobs, exist_ok=True)
    K.jdump(os.path.join(jobs, f'{a.seg}_ids.json'), {'run': a.seg, 'ids': ids, 'n': len(ids),
                                                       'ids_sha256': K.ids_sha(ids), 'K': reg['K'],
                                                       'what': 'B4 probe: R ∪ Rg ∪ pins'})
    with open(os.path.join(jobs, f'{a.seg}_dry.json'), 'w') as fh:
        fh.write(json.dumps({'kind': 'refold-lot-sum', 'tender_ids': ids}, separators=(',', ':')) + '\n')
    print(f'{a.seg}: {len(ids)} ids (R {len(reg["entries"])} ∪ watch {len(reg["watch"])} ∪ pins), ids_sha256 '
          f'{K.ids_sha(ids)[:16]}; wrote {jobs}/{a.seg}_ids.json and {a.seg}_dry.json')
    return 0


def cmd_show(a):
    print(K.jbytes(load_rec(a.seg)).decode())
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    o = sub.add_parser('open')
    o.add_argument('seg')
    o.add_argument('--class', dest='klass', required=True, choices=list(W.SEG_CLASSES))
    o.add_argument('--tag', required=True)
    o.add_argument('--cb-label', required=True)
    o.add_argument('--cb-label2', required=True)
    o.add_argument('--cursor', type=int, help='a cross-check: must equal the logged /health cursor')
    o.add_argument('--jobs', required=True)
    o.add_argument('--lock', required=True)
    o.add_argument('--runs-left', type=int, required=True)
    o.add_argument('--minutes-per-run', type=int, required=True)
    o.add_argument('--read-minutes', type=int)
    o.add_argument('--fixed-minutes', type=int, default=SR.FIXED_MINUTES)
    o.add_argument('--switchoff-read-statements', type=int, default=0)
    g = o.add_mutually_exclusive_group()
    g.add_argument('--switchoff-gate-minutes', type=int)
    g.add_argument('--switchoff-gate', choices=sorted(SR.GATE_MINUTES))
    o.add_argument('--expect-new', type=int, default=0)
    o.add_argument('--expect-moves', default='0')
    o.add_argument('--salt')
    o.add_argument('--bar')
    o.add_argument('--now', help='test only: the clock')
    c = sub.add_parser('close')
    c.add_argument('seg')
    c.add_argument('--ca-label', required=True)
    c.add_argument('--ca-label2', required=True)
    c.add_argument('--cursor', type=int, help='a cross-check: must equal the logged /health cursor')
    c.add_argument('--jobs', required=True)
    c.add_argument('--own', required=True)
    c.add_argument('--corrected', type=int)
    c.add_argument('--redo', action='store_true')
    ab = sub.add_parser('abort')
    ab.add_argument('seg')
    ab.add_argument('--cb-label', required=True)
    ab.add_argument('--cb-label2', required=True)
    ab.add_argument('--jobs', required=True)
    ab.add_argument('--reason', required=True)
    for name in ('chain', 'weekly', 'gap'):
        t = sub.add_parser(name)
        t.add_argument('seg')
        if name != 'gap':
            t.add_argument('--date', required=True)
            t.add_argument('--jobs-mark', type=int, help='the jobs mark when the registry has none (551 records it)')
        else:
            t.add_argument('--reason', required=True)
            t.add_argument('--since-job', type=int, help='the jobs mark when the registry has none (551 records it)')
        t.add_argument('--ca-label', required=True)
        t.add_argument('--ca-label2', required=True)
        t.add_argument('--cursor', type=int, help='a cross-check: must equal the logged /health cursor')
        t.add_argument('--jobs', required=True)
        tg = t.add_mutually_exclusive_group()
        tg.add_argument('--switchoff-gate-minutes', type=int)
        tg.add_argument('--switchoff-gate', choices=sorted(SR.GATE_MINUTES))
        t.add_argument('--now', help='test only: the clock')
        if name == 'chain':
            t.add_argument('--allow-missing', action='append', choices=list(CHAIN_SKIPPABLE))
    p = sub.add_parser('probe-jobs')
    p.add_argument('seg')
    s = sub.add_parser('show')
    s.add_argument('seg')
    a = ap.parse_args(argv)
    return {'open': cmd_open, 'close': cmd_close, 'abort': cmd_abort, 'chain': cmd_chain, 'weekly': cmd_weekly,
            'gap': cmd_gap, 'probe-jobs': cmd_probe_jobs, 'show': cmd_show}[a.cmd](a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
