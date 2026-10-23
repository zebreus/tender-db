"""Issue 551, path W on: offline tests of the W-on drains kit (no network, no box, no write outside the w-kit sandbox).

    python3 -I -B test_w.py

Builds a sandbox copy of the kit under scratchpad w-kit/sandbox (u6/u7/u8/u9 *.py copied; the R0 inputs symlinked
read-only: u9/20261023/, u9/confirm_w12a.json, u6/freeze.json, u6/P_final.json, u8/delta_read_set.json,
u8/delta_read_result.json), re-runs itself inside it, and drives every script end to end against a FAKE box seeded
with the REAL 10-23 archives (the 176,762 change rows of (C1, K0] and the 27,177 PK-read Tenders), then simulated
segments after K0. Cursors come from a sandbox u6/health.log (two labelled reads around every saved jobs body); the
clock is start_rule.CLOCK (set per phase), never the wall clock.
  T0  start rule: zoneinfo vs supervisor.rs's EU-DST rule on every day of 2026-2027; 09:35 Berlin = 07:35Z through
      10-24 and 08:35Z from 10-25; the Sunday 03:10 Berlin weekly tick; the weekly window; the gate options; the
      switch-off's own read minutes; the old tests' verdicts unchanged;
  T1  registry init (R0, digest, refusals; --force only over a pristine R0);
  T2  the box: E1 plan; paging; a 408 is capped and never re-sent (the resume halves the page / chunk); a transport
      failure is re-sent after health; one lock across boxes; no lost counts; a corrupt state refused; no lifetime cap
      (a warning); the rolling hour over every kit box (readers 125/h, switch-off 150/h); plan readiness;
  T3  W12: E1 gated until the replay PASSES (auto -> all, explicit e1 refused, both allowed, small spans always
      all-kind), the replay (b) and W12 (a)'s confirmation script on every branch;
  T4  the U9 day-3 chain as a class-chain segment plus every chain-table verdict; the jobs window from the mark
      (pre-tick writers, the dry exemption, coverage, the unclosed weekly range, R0's floor); issue 222's catch-up loop;
  T5  P0 (class D, --route both), make_wet's W-mode guard, open/close with logged cursors, the start rule at open,
      and every segment-table verdict (SILENT on the full value columns, pins first, 6908248 read-gated, W5 pin
      flips, the gap-end rule, class I over R heads, the STOP clearing keys);
  T6  539 step 1 (class Z): the shortcut and its refusals, the corrected rules, close refusals that keep the segment
      open, --redo, abort;
  T7  539 step 2 run a (class W): A tied to the segment, the cap, read-pending NEW with their recorded deadline (W9),
      E5's salt checks, D7 as an advisory (never a freeze), D7 in flight (registry status), A = 0;
  T8  the weekly tick (readiness, pre-tick writers) and gap segments (foreign jobs, W-input pre-step, no shortcut);
  T9  switchoff.py's registry route on its own box: resume, the narrowed verify, the band 408 falling back to the API;
  T10 day 7: the populations, the 539 step-2 flag, the fetched marker and the 10-28 writer hold;
  T11 tool faults map to exit 3.
"""
import bisect
import datetime
import importlib
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time
import types

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
REAL_B6 = os.path.dirname(HERE)
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
WK = os.path.join(SP, 'w-kit')
SB = os.path.join(WK, 'sandbox')
SB_B6 = os.path.join(SB, 'tender-db-scratch', '551-lot-sum', 'b6')
FAILS = []


def ok(cond, what):
    print(('  ok   ' if cond else '  FAIL ') + what, flush=True)
    if not cond:
        FAILS.append(what)


def build_sandbox():
    if os.path.exists(SB):
        shutil.rmtree(SB)
    for u in ('u6', 'u7', 'u8', 'u9'):
        os.makedirs(os.path.join(SB_B6, u))
        for f in os.listdir(os.path.join(REAL_B6, u)):
            if f.endswith('.py'):
                shutil.copyfile(os.path.join(REAL_B6, u, f), os.path.join(SB_B6, u, f))
    os.symlink(os.path.join(REAL_B6, 'u9', '20261023'), os.path.join(SB_B6, 'u9', '20261023'))
    os.symlink(os.path.join(os.path.dirname(REAL_B6), 'b5'), os.path.join(os.path.dirname(SB_B6), 'b5'))
    for u, f in (('u6', 'freeze.json'), ('u6', 'P_final.json'), ('u8', 'delta_read_set.json'),
                 ('u8', 'delta_read_result.json'), ('u9', 'confirm_w12a.json')):
        os.symlink(os.path.join(REAL_B6, u, f), os.path.join(SB_B6, u, f))


# ---------------------------------------------------------------- the fake box
COLS = ['id', 'source', 'current_seq', 'current_published_at', 'current_value_derivation', 'current_value_cents',
        'current_value_currency', 'current_value_eur_cents', 'current_value_derivation_detail', 'projection_epoch']


class FakeBox:
    def __init__(self):
        self.changes, self.cursors = [], []
        self.tcur, self.trows = [], []        # tender rows only (E1)
        self.tenders, self.lots = {}, {}
        self.sent, self.api_calls = [], []
        self.fail_on = None                   # a substring: the next such statement gets a 408 body (capped)
        self.fail_transport = None            # a substring: the next such statement fails in transport (ssh)

    @staticmethod
    def body(cols, rows):
        return json.dumps({'columns': cols, 'rows': rows, 'row_count': len(rows), 'truncated': False})

    def cursor(self):
        return self.cursors[-1] if self.cursors else 0

    def add(self, kind, eid, at, op='changed', cursor=None, vs=None):
        c = cursor if cursor is not None else self.cursor() + 1
        assert c > self.cursor(), (c, self.cursor())
        r = {'cursor': c, 'entity_kind': kind, 'entity_id': eid, 'version_seq': vs, 'op': op, 'changed_at': at}
        self.changes.append(r)
        self.cursors.append(c)
        if kind == 'tender':
            self.tcur.append(c)
            self.trows.append(r)
        return c

    def seed(self, rows):
        for r in rows:
            self.add(r['entity_kind'], r['entity_id'], r['changed_at'], r['op'], cursor=r['cursor'], vs=r['version_seq'])
            if r['entity_kind'] == 'lot' and r.get('lot_tender_id') is not None:
                self.lots[r['entity_id']] = r['lot_tender_id']

    def n(self, pat):
        return sum(1 for s in self.sent if re.search(pat, s))

    def answer(self, sql):
        self.sent.append(sql)
        if self.fail_transport and self.fail_transport in sql:
            self.fail_transport = None
            return '', 'ssh: Connection closed by UNKNOWN port 65535'
        if self.fail_on and self.fail_on in sql:
            self.fail_on = None
            return '{"error":"statement interrupted at the 15 s cap","status":408}'
        if sql.strip().upper() == 'SELECT 1':
            return self.body(['1'], [[1]])
        m = re.search(r"FROM changes c(?: INDEXED BY changes_entity_cursor)? WHERE c\.entity_kind = 'tender' AND "
                      r"c\.cursor > (\d+) AND c\.cursor <= (\d+) ORDER BY c\.cursor LIMIT (\d+)", sql)
        if m:
            lo, hi, lim = (int(x) for x in m.groups())
            i, j = bisect.bisect_right(self.tcur, lo), bisect.bisect_right(self.tcur, hi)
            rows = []
            for r in self.trows[i:j][:lim]:
                t = self.tenders.get(r['entity_id'])
                rows.append([r['cursor'], r['entity_id'], r['op'], t['current_value_derivation'] if t else None])
            return self.body(['cursor', 'entity_id', 'op', 'd'], rows)
        m = re.search(r'FROM changes c WHERE c\.cursor > (\d+) AND c\.cursor <= (\d+)', sql)
        if m:
            lo, hi = int(m.group(1)), int(m.group(2))
            i, j = bisect.bisect_right(self.cursors, lo), bisect.bisect_right(self.cursors, hi)
            rows = [[r['cursor'], r['entity_kind'], r['entity_id'], r['version_seq'], r['op'], r['changed_at'],
                     self.lots.get(r['entity_id']) if r['entity_kind'] == 'lot' else None] for r in self.changes[i:j]]
            return self.body(['cursor', 'entity_kind', 'entity_id', 'version_seq', 'op', 'changed_at', 'lot_tender_id'],
                             rows)
        m = re.search(r'FROM tenders t WHERE t\.id IN \(([\d, ]+)\)', sql)
        if m:
            ids = [int(x) for x in m.group(1).split(',')]
            return self.body(COLS, [[self.tenders[i].get(c) for c in COLS] for i in ids if i in self.tenders])
        if "current_value_derivation = 'lot-sum-awards'" in sql and 'ORDER BY current_value_eur_cents DESC' in sql:
            rows = sorted(([t['id'], t['current_value_eur_cents'], t['current_value_cents'], t['current_value_currency'],
                            t['current_value_derivation_detail']] for t in self.tenders.values()
                           if t['current_value_derivation'] == 'lot-sum-awards'
                           and (t['current_value_eur_cents'] or 0) >= 10_000_000_000), key=lambda r: -r[1])[:200]
            return self.body(['id', 'current_value_eur_cents', 'current_value_cents', 'current_value_currency',
                              'current_value_derivation_detail'], rows)
        raise AssertionError(f'fake box: unknown statement shape: {sql[:200]}')

    def get(self, path):
        self.api_calls.append(path)
        m = re.match(r'^/v1/tenders\?min_value=10000000000&limit=1000(?:&cursor=(\d+))?$', path)
        if not m:
            return 404, b'{"error":"not found"}'
        after = int(m.group(1) or 0)
        band = sorted((t for t in self.tenders.values() if (t['current_value_eur_cents'] or 0) >= 10_000_000_000
                       and t['id'] > after), key=lambda t: t['id'])
        page = band[:50]
        items = []
        for t in page:
            det = json.loads(t['current_value_derivation_detail']) if t['current_value_derivation_detail'] else {}
            vd = dict(det, method=t['current_value_derivation']) if t['current_value_derivation'] else None
            items.append({'id': t['id'], 'value': {'cents': t['current_value_cents'], 'currency':
                                                   t['current_value_currency'] or 'EUR'}, 'value_derivation': vd})
        more = len(band) > len(page)
        return 200, json.dumps({'items': items, 'more': more, 'next_cursor': str(page[-1]['id']) if more else None,
                                'ignored_filters': []}).encode()


def trow(i, src='ted', seq=1, pub=1792800000, deriv=None, cents=None, cur=None, eur=None, detail=None):
    return {'id': i, 'source': src, 'current_seq': seq, 'current_published_at': pub, 'current_value_derivation': deriv,
            'current_value_cents': cents, 'current_value_currency': cur, 'current_value_eur_cents': eur,
            'current_value_derivation_detail': detail, 'projection_epoch': 4}


def lsa_row(i, cents, src='ted', cur='EUR', eur=None, lots=2, field='BT-118-NoticeResult', nid=47500000, vb=None,
            pub=1792800000, seq=1):
    det = {'summed': 'awards', 'lots': lots, 'vouched_by': {'field': field, 'notice_id': nid, 'cents': vb or cents}}
    return trow(i, src=src, seq=seq, pub=pub, deriv='lot-sum-awards', cents=cents, cur=cur,
                eur=cents if eur is None else eur, detail=json.dumps(det, separators=(',', ':')))


def jobs_body(recent, measured_at, current=None, queued=()):
    return {'current': current, 'queued': list(queued), 'recent': sorted(recent, key=lambda r: -r['job_id']),
            'measured_at': measured_at}


def job(jid, kind, started, outcome='ok', counts='', params=None):
    return {'id': jid + 100000, 'job_id': jid, 'kind': kind, 'params': params or kind, 'started_at': started,
            'finished_at': started + 30, 'outcome': outcome, 'counts': counts}


def dry_summary(found, named, left=()):
    tail = ('; every named Tender is in the cohort' if not left else
            f'; {len(left)} named Tender(s) not in the cohort (no such Tender, or a head no lot sum can reach): '
            + ','.join(map(str, list(left)[:20])) + (',…' if len(left) > 20 else ''))
    return (f'refold-lot-sum (issue 551) DRY RUN — nothing written: {found} of {named} named Tender(s) whose head '
            f'could serve a derived lot sum, {found * 3} notice(s){tail}; run it with "dry_run":false,"expect":'
            f'{found} with the same tender_ids to stamp and re-queue them (a `project` is queued behind the wet run)')


def dry_job(jid, started):
    return job(jid, 'refold-lot-sum', started, counts=dry_summary(10, 12, [1, 2]))


# ---------------------------------------------------------------- inside the sandbox
def inside():
    for u in ('u6', 'u7', 'u8', 'u9'):
        sys.path.insert(0, os.path.join(SB_B6, u))
    import kitlib as K
    import wkit as W
    assert W.U9.startswith(SB) and K.U6.startswith(SB), (W.U9, K.U6)
    SR = importlib.import_module('start_rule')
    REG = importlib.import_module('registry')
    SEG = importlib.import_module('segment')
    G = importlib.import_module('gd5')
    AJ = importlib.import_module('ajobs')
    D7 = importlib.import_module('day7')
    RP = importlib.import_module('replay')
    CW = importlib.import_module('confirm_w12a')
    MW = importlib.import_module('make_wet')
    SO = importlib.import_module('switchoff')
    DL = importlib.import_module('delta')
    utc = datetime.timezone.utc
    T = SR.parse_now
    tmp = os.path.join(SB, 'tmp')
    os.makedirs(tmp)
    assert SEG.HEALTH_LOG.startswith(SB) and K.BOX_LOCK.startswith(SB), (SEG.HEALTH_LOG, K.BOX_LOCK)

    def clock(iso_or_epoch):
        SR.CLOCK = T(iso_or_epoch) if isinstance(iso_or_epoch, str) else \
            datetime.datetime.fromtimestamp(int(iso_or_epoch), utc)

    def jf(name, body):
        p = os.path.join(tmp, name)
        K.jdump(p, body)
        return p

    def tf(name, text):
        p = os.path.join(tmp, name)
        with open(p, 'w', encoding='utf-8') as fh:
            fh.write(text)
        return p

    hl_n = [0]

    def hl_pair(cursor, at, cursor2=None, lag=10):
        """two /health lines around a jobs body measured at `at` -> (label, label2)."""
        hl_n[0] += 1
        l1, l2 = f'h{hl_n[0]}a', f'h{hl_n[0]}b'
        with open(SEG.HEALTH_LOG, 'a', encoding='utf-8') as fh:
            for lb, t, c in ((l1, at - lag, cursor), (l2, at + lag, cursor if cursor2 is None else cursor2)):
                fh.write(f'{K.iso_of(t)} {lb} rev=512b2976f8c5d65de6b72d642202311e3308349a cursor={c} '
                         'last_job_kind=None last_job_outcome=None last_job_started_at=None last_job_finished_at=None\n')
        return l1, l2

    def at_of(jobs_path):
        return int(K.jload(jobs_path)['measured_at'])

    def seg_open(seg, klass, tag, cb, jobs_path, lock='L', runs='1', mpr='45', reads=None, gate='covered', now=None,
                 extra=(), cb2=None):
        l1, l2 = hl_pair(cb, at_of(jobs_path), cb2)
        args = ['open', seg, '--class', klass, '--tag', tag, '--cb-label', l1, '--cb-label2', l2, '--jobs', jobs_path,
                '--lock', lock, '--runs-left', str(runs), '--minutes-per-run', str(mpr), '--switchoff-gate', gate,
                '--now', now or K.iso_of(at_of(jobs_path))]
        if reads is not None:
            args += ['--read-minutes', str(reads)]
        return K.run_main(SEG.main, args + list(extra))

    def seg_close(seg, ca, jobs_path, own, extra=(), ca2=None, lag=10):
        l1, l2 = hl_pair(ca, at_of(jobs_path), ca2, lag)
        return K.run_main(SEG.main, ['close', seg, '--ca-label', l1, '--ca-label2', l2, '--jobs', jobs_path, '--own',
                                     own] + list(extra))

    def seg_tick(cmd, seg, date, ca, jobs_path, extra=()):
        l1, l2 = hl_pair(ca, at_of(jobs_path))
        return K.run_main(SEG.main, [cmd, seg, '--date', date, '--ca-label', l1, '--ca-label2', l2, '--jobs', jobs_path,
                                     '--now', K.iso_of(at_of(jobs_path))] + list(extra))

    def seg_gap(seg, ca, jobs_path, reason='x', extra=()):
        l1, l2 = hl_pair(ca, at_of(jobs_path))
        return K.run_main(SEG.main, ['gap', seg, '--ca-label', l1, '--ca-label2', l2, '--jobs', jobs_path, '--reason',
                                     reason, '--now', K.iso_of(at_of(jobs_path))] + list(extra))

    def g_read(seg, k, ca, klass, jobs_path, fake, extra=(), now=None):
        return K.run_main(G.main, ['read', seg, '--k', str(k), '--cursor-after', str(ca), '--class', klass, '--jobs',
                                   jobs_path, '--now', K.iso_of(now or at_of(jobs_path))] + list(extra), fake=fake)

    def g_check(seg, k, ca, klass, extra=()):
        return K.run_main(G.main, ['check', seg, '--k', str(k), '--cursor-after', str(ca), '--class', klass]
                          + list(extra))

    def upd(d, k, v):
        x = dict(d)
        x[k] = v
        return x

    def unset_open():
        reg = REG.load()
        reg['open_segment'] = None
        REG.save(reg)

    # ------------------------------------------------------------ T0 start rule
    print('T0 start rule: zoneinfo Europe/Berlin, the weekly tick, the gate options, the switch-off reads')

    def last_sunday(year, month):
        d = datetime.date(year, month, 31)
        return d - datetime.timedelta(days=(d.weekday() + 1) % 7)

    def sup_tick(day, hour, minute):
        """supervisor.rs berlin_tick_on: the offset at 12:00 UTC of the UTC day (last Sundays, 01:00 UTC)."""
        noon = datetime.datetime(day.year, day.month, day.day, 12, tzinfo=utc)
        start = datetime.datetime.combine(last_sunday(day.year, 3), datetime.time(1), tzinfo=utc)
        end = datetime.datetime.combine(last_sunday(day.year, 10), datetime.time(1), tzinfo=utc)
        off = 2 if start <= noon < end else 1
        return datetime.datetime(day.year, day.month, day.day, hour, minute, tzinfo=utc) - datetime.timedelta(hours=off)

    bad = [d for d in (datetime.date(2026, 1, 1) + datetime.timedelta(days=k) for k in range(730))
           if SR.berlin_at(d, 9, 35) != sup_tick(d, 9, 35) or SR.berlin_at(d, 3, 10) != sup_tick(d, 3, 10)]
    ok(not bad, f'zoneinfo == supervisor.rs berlin_tick_on at 09:35 and 03:10 on all 730 days of 2026-2027 ({bad[:3]})')
    ok(SR.next_chain(T('2026-10-24T06:00:00Z')) == T('2026-10-24T07:35:00Z'), 'chain 10-24: 07:35Z (CEST)')
    ok(SR.next_chain(T('2026-10-24T07:36:00Z')) == T('2026-10-25T08:35:00Z'), 'after the 10-24 tick: 10-25 08:35Z (CET)')
    n25 = T('2026-10-25T07:40:00Z')
    old = n25.replace(hour=7, minute=35, second=0) + datetime.timedelta(days=1)   # the hard-coded 07:35 UTC rule
    ok(SR.next_chain(n25) == T('2026-10-25T08:35:00Z') and old == T('2026-10-26T07:35:00Z'),
       'W2: at 10-25 07:40Z the chain is 08:35Z the same day (the old 07:35 UTC rule said 10-26 07:35Z)')
    ev = SR.evaluate(1, n25)
    ok(not ev['go'] and ev['deadline'] == T('2026-10-25T06:35:00Z'),
       'W2: 10-25 07:40Z, one run: NO-GO (deadline 06:35Z); the old rule would have said GO')
    ok(SR.next_chain(T('2026-10-26T09:00:00Z')) == T('2026-10-27T08:35:00Z'), 'chain 10-27: 08:35Z')
    ok(SR.next_chain(T('2027-03-28T05:00:00Z')) == T('2027-03-28T07:35:00Z')
       and SR.next_chain(T('2027-03-27T05:00:00Z')) == T('2027-03-27T08:35:00Z'), 'spring 2027: 08:35Z -> 07:35Z')
    ok(SR.next_weekly(T('2026-10-23T20:00:00Z')) == T('2026-10-25T02:10:00Z'), 'weekly tick: Sun 10-25 02:10Z')
    ok(SR.next_weekly(T('2026-10-25T02:10:00Z')) == T('2026-11-01T02:10:00Z')
       and SR.next_weekly(T('2026-10-26T09:00:00Z')) == T('2026-11-01T02:10:00Z'), 'weekly tick: then 11-01 02:10Z')
    ok(SR.next_weekly(T('2027-03-27T12:00:00Z')) == T('2027-03-28T01:10:00Z'), 'weekly tick on the spring DST Sunday: 01:10Z')
    ev = SR.evaluate(3, T('2026-10-24T20:30:00Z'), 60, 30, 0, 45)
    ok(ev['tick_kind'] == 'weekly' and ev['deadline'] == T('2026-10-25T00:10:00Z') and not ev['go']
       and ev['switchoff_by'] == T('2026-10-25T01:40:00Z'),
       'W3: 10-24 20:30Z, 3 runs x (60 + 45 read): the weekly tick binds (deadline 00:10Z, switch-off by 01:40Z): NO-GO')
    ok(SR.evaluate(3, T('2026-10-24T20:30:00Z'), 60, 30, 0, 45)['finish'] < T('2026-10-25T06:35:00Z'),
       '... while against the chain alone (08:35Z - 2 h) it would have fitted')
    ev = SR.evaluate(1, T('2026-10-25T03:00:00Z'), 20, 30, 0)
    ok(ev['in_weekly_window'] and ev['switchoff_minutes'].get('weekly_queue_wait') == 120
       and ev['need'] == datetime.timedelta(minutes=20 + 30 + 120),
       'B6: inside the weekly jobs\' window (02:10-04:10Z) both the runs and a switch-off count +2 h queue wait')
    ok(not SR.evaluate(1, T('2026-10-25T04:20:00Z'))['in_weekly_window'], 'the window closes at 04:10Z')
    ok(SR.main(['--runs-left', '4', '--now', '2026-10-21T10:00:00Z']) == 0
       and SR.main(['--runs-left', '4', '--now', '2026-10-22T04:00:00Z']) == 2, 'the old start-rule verdicts are unchanged')
    for g, m in (('covered', 0), ('uncovered', 35), ('from-clean', 40)):
        a = types.SimpleNamespace(switchoff_gate=g, switchoff_gate_minutes=None)
        ok(SR.gate_of(a) == m, f'--switchoff-gate {g} = {m} min')
    ok(K.run_main(SR.main, ['--runs-left', '1', '--switchoff-gate', 'covered', '--switchoff-gate-minutes', '3']) == 2,
       'the two gate options are exclusive (argparse exit 2)')
    ok(SR.parse_now('2026-10-25T07:40:00') == n25, 'a --now without a zone is UTC')
    e0 = SR.evaluate(0, T('2026-10-24T12:00:00Z'), switchoff_gate_minutes=0)
    e1 = SR.evaluate(0, T('2026-10-24T12:00:00Z'), switchoff_gate_minutes=0, switchoff_read_statements=760)
    ok(e1['switchoff_minutes'].get('box_reads') == 304 and e0['switchoff_by'] - e1['switchoff_by']
       == datetime.timedelta(minutes=304),
       'the switch-off\'s own reads are budgeted: 760 statements x 24 s (150/h) = 304 min earlier switch-off start')
    clock('2026-10-24T05:00:00Z')
    ok(SR.parse_now(None) == T('2026-10-24T05:00:00Z'), 'tests run on start_rule.CLOCK, never the wall clock')

    # ------------------------------------------------------------ T1 registry init
    print('T1 registry init (R0)')
    rc = REG.main(['init'])
    reg = REG.load()
    ok(rc == 0 and reg['ids_sha256'] == W.R0_IDS_SHA and reg['n'] == 3478 and reg['K'] == W.K0,
       f'R0: 3,478 entries, ids_sha256 {reg["ids_sha256"][:16]}, K0 {reg["K"]}')
    ok(reg['tags'] == {'P': 3467, 'delta-U8': 8, 'chain-2026-10-22': 1, 'chain-2026-10-23': 2},
       f'tags {reg["tags"]}: 97901 from the 10-22 chain, 8859070 / 8859315 from 10-23')
    ok(len(reg['B_R']) == 29 and len(reg['watch']) == 63 and reg['watch']['6465510']['pin'] == 'not-lsa',
       'B_R = B_P 29; watch rows = Rg 35 + 28 pins outside R (6465510 an Rg row and the not-lot-sum-awards pin)')
    ok(reg['jobs_mark'] == {'seg': 'R0', 'max_job_id': None, 'measured_at': W.K0_AT,
                            'basis': reg['jobs_mark']['basis']} and W.K0_AT == K.epoch_of('2026-10-23T18:48:01Z'),
       'the jobs mark starts at K0\'s /health read (10-23 18:48:01Z)')
    p0e = reg['entries']['585917']
    ok(p0e['pin'] == 'path-W' and reg['entries']['6908248']['pin'] == 'double-count'
       and reg['entries']['6679375']['pin'] == 'partial-sum', 'pin roles carried on the R entries')
    ok(reg['entries']['909']['vouches'] == [['BT-1118-NoticeResult', 25124767, 40000000],
                                            ['BT-118-NoticeResult', 25124767, 40000000]],
       'P entries carry the port\'s vouches (membership, A4)')
    e1sha = reg['entries_sha256']
    ok(REG.main(['init']) == 2, 'init writes R0 once')
    ok(REG.main(['init', '--force']) == 0 and REG.load()['entries_sha256'] == e1sha,
       f'--force over a pristine R0: the content digest is stable ({e1sha[:16]})')
    REG.freeze('zz', 'SWITCH OFF', 'a test freeze at version 0')
    before = open(W.REG_FILE, 'rb').read()
    ok(REG.main(['init', '--force']) == 2 and open(W.REG_FILE, 'rb').read() == before
       and REG.main(['status', '--no-plan-check']) == 10,
       'init --force refuses a version-0 registry frozen by a SWITCH OFF (R.json untouched, status stays 10)')
    ok(REG.main(['init', '--force', '--discard-version', '0']) == 0 and REG.load()['frozen'] is None
       and any(f.startswith('R_forced_v0000_') for f in os.listdir(os.path.join(W.REG_DIR, 'history'))),
       '... only --discard-version 0 discards it, after copying it to history/R_forced_v0000_*')
    so_bad = jf('so_bad.json', dict(K.jload(W.R0_FILE), ids=K.jload(W.R0_FILE)['ids'][1:]))
    real = W.R0_FILE
    W.R0_FILE = so_bad
    ok(K.run_main(REG.main, ['init', '--force']) == 3, 'an R0 that is not the recorded one is refused (exit 3)')
    W.R0_FILE = real
    ok(REG.main(['status']) == 0, 'status: READY at R0 (the plan probe ready, 0 statements)')

    # ------------------------------------------------------------ T2 the box
    print('T2 the box: E1 plan, paging, a 408 capped and never re-sent, transport, the lock, the hour, readiness')
    e1 = K.sql_tender_keyset(W.C1, W.K0)
    ok(K.E1_HINT and 'INDEXED BY changes_entity_cursor' in e1, 'E1 names its index (INDEXED BY)')
    plan = K.plan_check(e1, 'e1', tmp, K.ensure_plan_db_w())
    ok(plan[0].startswith('SEARCH c USING INDEX changes_entity_cursor (entity_kind=? AND cursor>? AND cursor<=?)')
       and any('SEARCH t USING INTEGER PRIMARY KEY' in s for s in plan), f'E1 plan on plan_w.db: {plan}')
    ok(K.plan_check(K.sql_tender_keyset(W.C1, W.K0, hint=False), 'e1n', tmp, K.PLAN_DB_W) == plan,
       'the plan is the same without the hint')
    try:
        K.plan_check(e1, 'e1x', tmp)
        ok(False, 'plan.db (no index) refuses E1 at plan time')
    except K.PlanRefused:
        ok(True, 'plan.db (no index) refuses E1 at plan time (PLAN-ERR no such index)')
    fb = FakeBox()
    for k in range(12000):
        fb.add('tender', 70_000_000 + k, 1)
        fb.add('lot', 80_000_000 + k, 1)
    for k in range(12000):
        fb.tenders[70_000_000 + k] = trow(70_000_000 + k)
    bx = W.w_box(fb, state_dir=os.path.join(tmp, 'box_e1'))
    rows, comp = K.tender_keyset(bx, 0, fb.cursor(), os.path.join(tmp, 'e1a'))
    ok(comp and len(rows) == 12000 and fb.n('entity_kind = .tender.') == 3, '12,000 tender rows in 3 pages of <= 5,000')
    fb.tcur, fb.trows = fb.tcur[:10000], fb.trows[:10000]
    rows, comp = K.tender_keyset(bx, 0, fb.cursor(), os.path.join(tmp, 'e1b'))
    ok(comp and len(rows) == 10000 and fb.n('entity_kind = .tender.') == 6, 'exactly 10,000: a third (empty) page ends it')
    c2 = fb.trows[4999]['cursor']
    capped_sql = ' '.join(K.sql_tender_keyset(c2, fb.cursor()).split())
    fb.fail_on = 'c.cursor > ' + str(c2)
    rows, comp = K.tender_keyset(bx, 0, fb.cursor(), os.path.join(tmp, 'e1c'))
    st = K.jload(os.path.join(tmp, 'box_e1', 'state.json'))
    man = K.jload(os.path.join(tmp, 'e1c', 'manifest.json'))
    ok(not comp and st['flag'] and fb.n('entity_kind = .tender.') == 8 and man['pages'][str(c2)]['class'] == 'capped'
       and K.stmt_sha(capped_sql) in st['capped'], 'page 2 408s: recorded as capped, flag set, not retried')
    try:
        K.tender_keyset(bx, 0, fb.cursor(), os.path.join(tmp, 'e1c'))
        ok(False, 'the flag refuses the next read')
    except K.BoxRefused:
        ok(True, 'the flag refuses the next read until health')
    bx.health()
    try:
        bx.sq(capped_sql, 'again')
        ok(False, 'the identical capped statement is refused after health')
    except K.BoxRefused as e:
        ok('capped' in str(e), 'the identical capped statement is never sent again, health or not (never retry a 408)')
    n408 = len(fb.sent)
    rows, comp = K.tender_keyset(bx, 0, fb.cursor(), os.path.join(tmp, 'e1c'))
    man = K.jload(os.path.join(tmp, 'e1c', 'manifest.json'))
    ok(comp and len(rows) == 10000 and fb.n('entity_kind = .tender.') == 10 and man['pages'][str(c2)]['limit'] == 2500
       and not any(' '.join(s.split()) == capped_sql for s in fb.sent[n408:])
       and len({int(r['cursor']) for r in rows}) == 10000,
       'resume: page 2 re-read with its LIMIT halved (2,500), then on; no gap, no overlap; the 408\'d statement sent once')
    bt = W.w_box(fb, state_dir=os.path.join(tmp, 'box_t'))
    fb.fail_transport = 'c.cursor > ' + str(c2)
    rows, comp = K.tender_keyset(bt, 0, fb.cursor(), os.path.join(tmp, 'e1t'))
    st = K.jload(os.path.join(tmp, 'box_t', 'state.json'))
    ok(not comp and 'transport' in st['flag'] and not st['capped']
       and K.jload(os.path.join(tmp, 'e1t', 'manifest.json'))['pages'][str(c2)]['class'] == 'transport',
       'a transport failure (empty stdout, ssh stderr) is recorded as transport (not capped)')
    bt.health()
    n0 = len(fb.sent)
    rows, comp = K.tender_keyset(bt, 0, fb.cursor(), os.path.join(tmp, 'e1t'))
    ok(comp and len(rows) == 10000 and ' '.join(fb.sent[n0].split()) == capped_sql,
       '... re-sent UNCHANGED after health (it never reached /v1/sql), unlike a 408')
    ids = list(range(70_000_000, 70_004_000))
    fb.fail_on = 'IN (70002000,'
    _r, f, m = K.pk_read(bx, ids, 'pkx', os.path.join(tmp, 'pkx'), resume=True)
    ok(f and f[0]['class'] == 'capped' and fb.n('IN \\(70000000,') == 1, 'a PK chunk 408s: recorded capped')
    bx.health()
    n0 = len(fb.sent)
    rows, f, m = K.pk_read(bx, ids, 'pkx', os.path.join(tmp, 'pkx'), resume=True)
    pks = [s for s in fb.sent[n0:] if 'FROM tenders t' in s]
    ok(not f and not m and len(rows) == 4000 and len(pks) == 2
       and all(s.split('IN (')[1].count(',') == 999 for s in pks),
       'a resumed pk_read re-sends only the failed chunk, halved (2 x 1,000); the answered chunk is reused')
    fb.fail_on = 'c.cursor > 5000 AND c.cursor <= 10000'
    crow, comp = K.enumerate_changes(bx, 0, 24000, os.path.join(tmp, 'enx'))
    bx.health()
    crow, comp = K.enumerate_changes(bx, 0, 24000, os.path.join(tmp, 'enx'))
    mn = K.jload(os.path.join(tmp, 'enx', 'manifest.json'))
    ok(comp and len(crow) == 24000 and mn['chunks']['5000_10000'].get('split') == [[5000, 7500], [7500, 10000]],
       'a capped all-kind chunk is split in two halves on resume (never re-sent as it was)')
    # one lock across boxes, no lost counts, a 408 flag survives the other box's save
    lockd = os.path.join(tmp, 'lockbox')
    spans = []

    def slow(body, dt=0.4):
        def t(sql):
            t0 = time.time()
            time.sleep(dt)
            spans.append((t0, time.time()))
            return body, ''
        return t

    okb = FakeBox.body(['1'], [[1]])
    b1 = K.Box(lockd, 10**9, transport=slow(okb), gap=0.0, probe=False, hourly=10**6, log=lambda s: None)
    b2 = K.Box(lockd, 10**9, transport=slow('{"error":"interrupted","status":408}'), gap=0.0, probe=False,
               hourly=10**6, log=lambda s: None)
    th1 = threading.Thread(target=lambda: b1.sq('SELECT 2', 'a'))
    th2 = threading.Thread(target=lambda: b2.sq('SELECT 3', 'b'))
    th1.start()
    time.sleep(0.1)
    th2.start()
    th1.join()
    th2.join()
    st = K.jload(os.path.join(lockd, 'state.json'))
    (a0, a1), (c0, c1) = sorted(spans)
    ok(a1 <= c0 and st['used'] == 2 and len(st['times']) == 2 and st['flag'],
       'two boxes on one dir: the sends never overlap, used = 2, both times kept, the 408 flag survives')
    try:
        b1.sq('SELECT 4', 'c')
        ok(False, 'the surviving flag refuses the next statement')
    except K.BoxRefused:
        ok(True, 'the surviving flag refuses the next statement (no stale save erased it)')
    spans.clear()
    b3 = K.Box(os.path.join(tmp, 'lockbox2'), 10**9, transport=slow(okb), gap=0.0, probe=False, hourly=10**6,
               log=lambda s: None)
    th1 = threading.Thread(target=lambda: b3.sq('SELECT 2', 'a'))
    th2 = threading.Thread(target=lambda: K.Box(os.path.join(tmp, 'lockbox3'), 10**9, transport=slow(okb), gap=0.0,
                                                probe=False, hourly=10**6, log=lambda s: None).sq('SELECT 3', 'b'))
    th1.start()
    time.sleep(0.1)
    th2.start()
    th1.join()
    th2.join()
    (a0, a1), (c0, c1) = sorted(spans)
    ok(a1 <= c0, 'two boxes on different dirs: still one statement in flight (one lock for every kit box)')
    cd = os.path.join(tmp, 'corrupt')
    os.makedirs(cd)
    with open(os.path.join(cd, 'state.json'), 'w') as fh:
        fh.write('{"used": 12')
    try:
        K.Box(cd, 300, transport=slow(okb, 0), gap=0.0, probe=False).sq('SELECT 1', 'health')
        ok(False, 'a corrupt state.json is refused')
    except K.BoxRefused:
        ok(open(os.path.join(cd, 'state.json')).read() == '{"used": 12', 'a corrupt state.json is refused, never reset')
    # no lifetime cap (a warning), the rolling hour over every kit box
    wd = os.path.join(tmp, 'wbig')
    K.jdump(os.path.join(wd, 'state.json'), {'used': 10000, 'last': 0.0, 'flag': None, 'times': [], 'capped': []})
    ok(W.w_box(fb, state_dir=wd).health() is not None and 'WARNING' in open(os.path.join(wd, 'queries.log')).read(),
       'a W box past 10,000 statements still answers (no lifetime cap; a warning past 560, logged for 551)')
    real_dirs = K.kit_state_dirs
    hd = [os.path.join(tmp, 'hour', x) for x in ('u6', 'u9', 'conf', 'off')]
    K.kit_state_dirs = lambda: hd
    now_t = time.time()
    K.jdump(os.path.join(hd[1], 'state.json'), {'used': 125, 'last': 0.0, 'flag': None, 'capped': [],
                                                 'times': [now_t - 60] * 125})
    try:
        W.w_box(fb, state_dir=hd[1]).health()
        ok(False, 'the readers\' 126th statement in an hour is refused')
    except K.BoxRefused:
        ok(True, 'segment readers stop at 125 statements an hour')
    ok(W.w_box(fb, state_dir=hd[3], hourly=W.W_HOURLY).health() is not None,
       '... and the switch-off box still reads (150/h over every kit box: >= 25 left)')
    K.jdump(os.path.join(hd[3], 'state.json'), {'used': 150, 'last': 0.0, 'flag': None, 'capped': [],
                                                 'times': [now_t - 60] * 150})
    try:
        W.w_box(fb, state_dir=hd[3], hourly=W.W_HOURLY).health()
        ok(False, 'the 151st statement in an hour is refused')
    except K.BoxRefused:
        ok(True, 'the 151st statement in any hour is refused (B5 150/h)')
    K.kit_state_dirs = real_dirs
    # plan readiness (0 statements)
    saved = (K.PLAN_PROBE, K.PLAN_PROBE_SRC)
    K.PLAN_PROBE, K.PLAN_PROBE_SRC = os.path.join(tmp, 'no-probe'), os.path.join(tmp, 'no-probe-src')
    ok(REG.main(['status']) == 2 and W.plan_ready(), 'status HOLDs when the plan probe is gone (every statement, the '
       'switch-off\'s too, would be refused)')
    K.PLAN_PROBE, K.PLAN_PROBE_SRC = saved
    ok(not W.plan_ready(), 'plan_ready: the probe, plan.db, plan_w.db and every statement shape plan clean')

    # ------------------------------------------------------------ the box after R0 (the real archives)
    arch = W.archived_changes()
    pk = W.archived_pk(W.ARCH_CAND)
    pk.update(W.archived_pk(W.ARCH_CHANGED))
    fb = FakeBox()
    fb.seed(arch)
    fb.tenders = {i: dict(r) for i, r in pk.items()}
    ok(fb.cursor() == W.K0 and len(fb.tenders) == 27177, f'fake box seeded: {len(arch)} change rows to K0, '
       f'{len(fb.tenders)} Tenders')

    # ------------------------------------------------------------ T3 E1 gate + replay + W12 (a)
    print('T3 W12: the E1 gate, the replay (b) and the confirmation read (a)')
    gb = W.w_box(fb, state_dir=os.path.join(tmp, 'box_gate'))
    ok(not W.e1_proven(), 'before the replay E1 is not proven')
    lo_big, hi_big = W.K0 - 60_000, W.K0
    ch = W.changed_since(gb, lo_big, hi_big, os.path.join(tmp, 'gate_auto'))
    ok(ch['route'] == 'all' and ch['e1_unproven'] and ch['complete'],
       'unproven: --route auto over a span > 50k reads all-kind (E1 is never used before W12 b)')
    try:
        W.changed_since(gb, lo_big, hi_big, os.path.join(tmp, 'gate_e1'), route='e1')
        ok(False, 'unproven: an explicit e1 is refused')
    except SystemExit as e:
        ok('not proven' in str(e), 'unproven: an explicit --route e1 is refused (exit 3)')
    ok(W.changed_since(gb, lo_big, hi_big, os.path.join(tmp, 'gate_both'), route='both')['parity'] is not None,
       'unproven: --route both is allowed (the all-kind reader stays authoritative, parity checked)')
    ok(W.changed_since(gb, lo_big, hi_big, os.path.join(tmp, 'gate_deg'), route='e1', degrade=True)['route'] == 'all',
       'the switch-off degrades an unproven e1 to all-kind (a NOTE, never a refusal)')
    n0 = len(fb.sent)
    ok(RP.main(['read'], fake=fb) == 0 and len(fb.sent) - n0 == 4,
       f'replay read: E1 over (C1, K0] in {len(fb.sent) - n0} statements (18,216 tender rows)')
    ok(RP.main(['check']) == 0 and W.e1_proven(), 'replay check: tender rows = the archive, lot-sum-awards = R0 -> '
       'PASS; E1 is proven (W12 a + b)')
    ok(W.changed_since(gb, lo_big, hi_big, os.path.join(tmp, 'gate_auto2'))['route'] == 'e1',
       'proven: --route auto over a span > 50k takes E1')
    try:
        W.changed_since(gb, W.K0 - 1000, W.K0, os.path.join(tmp, 'gate_small'), route='e1')
        ok(False, 'an explicit e1 on a small span is refused')
    except SystemExit as e:
        ok('always uses the all-kind' in str(e), 'an explicit --route e1 on a span <= 50k is refused (D5 hygiene)')
    real_rp = W.REPLAY_JSON
    W.REPLAY_JSON = jf('replay_fail.json', dict(K.jload(real_rp), **{'pass': False}))
    ok(not W.e1_proven() and W.changed_since(gb, lo_big, hi_big, os.path.join(tmp, 'gate_fail'))['route'] == 'all',
       'a FAILed replay.json keeps every reader all-kind')
    W.REPLAY_JSON = real_rp
    e1rows = []
    m = K.jload(os.path.join(W.U9, 'replay', 'e1', 'manifest.json'))
    for key in sorted(m['pages'], key=int):
        e1rows += K.rows_of(K.jload(os.path.join(W.U9, 'replay', 'e1', f'page_{key}.json')))
    r0 = [int(i) for i in K.jload(W.R0_FILE)['ids']]
    ok(not RP.check(e1rows[1:], arch, r0, [], [])['pass'], 'replay: a row missing from E1 -> FAIL')
    gone = next(r for r in e1rows if r['d'] == 'lot-sum-awards')
    alt = [dict(r, d=None) if r['entity_id'] == gone['entity_id'] else r for r in e1rows]
    ok(not RP.check(alt, arch, r0, [], [])['pass'] and RP.check(alt, arch, r0, [gone['entity_id']], [])['pass'],
       'replay: an R0 head no longer lot-sum-awards fails unless the registry recorded it LOST')
    ok(RP.main(['check', '--dir', os.path.join(tmp, 'nothing')]) == 3, 'replay check without a read: 3')

    class CF:
        def __init__(self, mode):
            self.mode, self.sent = mode, []

        def answer(self, sql):
            self.sent.append(sql)
            if sql.strip() == 'SELECT 1':
                return FakeBox.body(['1'], [[1]])
            if self.mode == 'noidx' and 'INDEXED BY' in sql:
                return '{"error":"no such index: changes_entity_cursor"}'
            if self.mode == 'nohint' and 'INDEXED BY' in sql:
                return '{"error":"near INDEXED: syntax error"}'
            return fb.answer(sql)

    for mode, want_rc, want_n in (('ok', 0, 1), ('noidx', 2, 2), ('nohint', 2, 3)):
        cf = CF(mode)
        rc = CW.main(['--out', os.path.join(tmp, f'w12a_{mode}.json'), '--state-dir', os.path.join(tmp, f'bc_{mode}')],
                     fake=cf)
        rec = K.jload(os.path.join(tmp, f'w12a_{mode}.json'))
        ok(rc == want_rc and len(cf.sent) == want_n and rec['statements'] == want_n,
           f'W12 (a) {mode}: exit {rc}, {len(cf.sent)} statement(s) (<= 3), {rec["verdict"][:60]}')

    ok(AJ.main(['build', 'days12', '--archive']) == 0
       and K.jload(os.path.join(W.AJOBS_DIR, 'days12', 'manifest.json'))['n'] == 9975
       and len(K.jload(os.path.join(W.AJOBS_DIR, 'days12', 'manifest.json'))['jobs']) == 2,
       'A for days 1-2 (H): the chains\' changed live Tenders after U7\'s drain = 9,975 in 2 dry jobs')
    ok(AJ.main(['build', 'days12-all', '--archive', '--after-cursor', str(W.C1)]) == 0
       and K.jload(os.path.join(W.AJOBS_DIR, 'days12-all', 'manifest.json'))['n'] == 13453,
       '... the decision\'s literal 13,453 (since C1, U7\'s drained Tenders included) in 3 jobs')
    ok(K.run_main(AJ.main, ['build', 'days12', '--archive']) == 3, 'an A measurement is built once')
    d12 = K.jload(os.path.join(W.AJOBS_DIR, 'days12', 'manifest.json'))
    AJ.main(['count', 'days12'] + sum((['--summary', tf(f'd12_{j["job"]}.txt', dry_summary(j['n'] - 9, j['n'],
                                                                                           list(range(1, 10))))]
                                       for j in d12['jobs']), []))

    # ------------------------------------------------------------ T4 the 10-24 chain (class chain)
    print('T4 the U9 day-3 chain (10-24) as a class-chain segment; the jobs window from the mark')
    reg = REG.load()
    tick = SR.berlin_at(datetime.date(2026, 10, 24), 9, 35)
    t0 = int(tick.timestamp())
    P_plain = [int(i) for i, e in sorted(reg['entries'].items(), key=lambda kv: int(kv[0]))
               if e['tag'] == 'P' and not e.get('pin') and int(i) not in reg['B_R']
               and e['row']['current_value_currency'] == 'EUR' and e['row']['current_value_eur_cents'] < 10**9]
    MOV, DATE1, LOSTX = P_plain[0], P_plain[1], P_plain[2]
    NEW_C, NEW_D = 8_870_001, 8_870_002
    k_before = fb.cursor()
    fb.tenders[NEW_C] = lsa_row(NEW_C, 50_000_000, nid=47500001, pub=t0 - 3600)
    fb.tenders[NEW_D] = lsa_row(NEW_D, 30_000_000, src='doe', nid=47500002, pub=t0 - 3600)
    fb.add('tender', NEW_C, t0 + 300, op='added')
    fb.add('tender', NEW_D, t0 + 301, op='added')
    r = dict(fb.tenders[MOV])
    r.update(current_value_cents=r['current_value_cents'] + 10000, current_value_eur_cents=r['current_value_eur_cents']
             + 10000, current_seq=(r['current_seq'] or 0) + 1)
    fb.tenders[MOV] = r
    fb.add('tender', MOV, t0 + 302)
    r = dict(fb.tenders[DATE1])
    r.update(current_seq=(r['current_seq'] or 0) + 1, current_published_at=(r['current_published_at'] or 0) + 86400)
    fb.tenders[DATE1] = r
    fb.add('tender', DATE1, t0 + 303)
    r = dict(fb.tenders[97901])
    r.update(current_value_eur_cents=r['current_value_eur_cents'] + 1234)
    fb.tenders[97901] = r
    fb.add('tender', 97901, t0 + 120)
    for k in range(300):
        fb.tenders[8_880_000 + k] = trow(8_880_000 + k, eur=1_000_000 + k)
        fb.add('tender', 8_880_000 + k, t0 + 400)
        fb.lots[90_000_000 + k] = 8_880_000 + k
        fb.add('lot', 90_000_000 + k, t0 + 400)
    fb.add('organization', 95_000_000, t0 + 500, op='added', cursor=k_before + 60_000)
    CA1 = fb.cursor()
    chain_jobs = [job(2799, 'project', t0 - 86400)] + [job(2800 + k, kd, t0 + 10 + k) for k, kd in enumerate(
        ['probe', 'process', 'probe', 'process', 'fetch-rates', 'rederive-eur-recent', 'project', 'reveal-recheck'])]
    clock(t0 + 3600)
    jf_chain = jf('jobs_chain1024.json', jobs_body(chain_jobs, t0 + 3600))
    ok(seg_tick('chain', 'chain-r0x', '2026-10-24', CA1,
                jf('jobs_r0x.json', jobs_body(chain_jobs + [job(2798, 'refold', W.K0_AT + 3600)], t0 + 3600))) == 2
       and SEG.load_rec('chain-r0x')['foreign'][0]['job_id'] == 2798,
       'R0: a wet job after K0\'s /health read (10-23 18:48:01Z) and before the 07:35Z tick is foreign -> STOP (B1)')
    unset_open()
    ok(seg_tick('chain', 'chain-1024', '2026-10-24', CA1,
                jf('jobs_chain_notready.json', jobs_body(chain_jobs[:-1], t0 + 3600))) == 2,
       'chain check not ready while reveal-recheck has not finished (never by the clock)')
    ok(seg_tick('chain', 'chain-1024', '2026-10-24', CA1,
                jf('jobs_chain_running.json', jobs_body(chain_jobs, t0 + 3600, current={'id': 2808, 'kind': 'x'}))) == 2,
       'chain check not ready while a job runs')
    ok(K.run_main(SEG.main, ['chain', 'chain-1024', '--date', '2026-10-24', '--ca-label', 'nosuch', '--ca-label2',
                             'nosuch2', '--jobs', jf_chain]) == 3, 'a missing /health label is a tool fault (3)')
    l1, l2 = hl_pair(CA1, t0 + 3600, CA1 + 1)
    ok(SEG.main(['chain', 'chain-1024', '--date', '2026-10-24', '--ca-label', l1, '--ca-label2', l2, '--jobs',
                 jf_chain]) == 2, 'two /health reads that differ around the jobs body: not ready (read again)')
    l1, l2 = hl_pair(CA1, t0 + 3600 + 600)
    ok(SEG.main(['chain', 'chain-1024', '--date', '2026-10-24', '--ca-label', l1, '--ca-label2', l2, '--jobs',
                 jf_chain]) == 2, 'reads that do not bracket the jobs body\'s measured_at: not ready')
    ok(K.run_main(SEG.main, ['chain', 'chain-1024', '--date', '2026-10-24', '--ca-label', 'h1a', '--ca-label2', 'h1b',
                             '--jobs', jf_chain, '--allow-missing', 'project']) == 2,
       '--allow-missing takes only fetch-rates / reveal-recheck (argparse exit 2)')
    ok(seg_tick('chain', 'chain-1024', '2026-10-24', CA1, jf_chain) == 0,
       'chain-1024 READY: the six daily kinds after the 07:35Z tick, queue empty, CA from two equal /health reads')
    ok(REG.main(['status', '--no-plan-check']) == 2, 'status HOLD while the chain\'s check is open')
    n0 = len(fb.sent)
    ok(g_read('chain-1024', W.K0, CA1, 'chain', jf('jobs_r_stale.json', jobs_body(chain_jobs, t0 + 3600 - 1200)), fb,
              now=t0 + 3600) == 3 and len(fb.sent) == n0, 'gd5 read refuses a jobs body older than 10 min (0 statements)')
    ok(g_read('chain-1024', W.K0, CA1, 'chain', jf('jobs_r_new.json', jobs_body(chain_jobs + [job(2808, 'refold',
                                                                                                   t0 + 3500)],
                                                                                t0 + 3600)), fb) == 3
       and len(fb.sent) == n0, 'gd5 read refuses when a job ran since the segment\'s CA snapshot')
    ok(g_read('chain-1024', W.K0, CA1, 'chain', jf_chain, fb) == 0, 'gd5 read chain-1024 complete')
    rdj = K.jload(os.path.join(W.SEG_DIR, 'chain-1024', 'read', 'read.json'))
    ok(rdj['changed']['route'] == 'e1' and len(fb.sent) - n0 <= 6,
       f'span > 50k (E1 proven): the E1 route; {len(fb.sent) - n0} statements (E1 + NEW PK + 2 R PK + band; ~5-6)')
    ok(AJ.main(['build', 'chain-1024', '--from-seg', 'chain-1024']) == 0, 'A_d jobs written (0 statements)')
    am = K.jload(os.path.join(W.AJOBS_DIR, 'chain-1024', 'manifest.json'))
    sums = [tf(f'a1024_{j["job"]}.txt', dry_summary(j['n'] - 5, j['n'], list(range(1, 6)))) for j in am['jobs']]
    ok(AJ.main(['count', 'chain-1024'] + sum((['--summary', s] for s in sums), [])) == 0, 'A_d counted')
    ok(AJ.main(['count', 'chain-1024', '--summary', tf('a_bad.txt', dry_summary(40, am['n'] + 1))]) == 2,
       'A count refuses a summary for a different list')
    AJ.main(['count', 'chain-1024'] + sum((['--summary', s] for s in sums), []))
    afile = os.path.join(W.AJOBS_DIR, 'chain-1024', 'A.json')
    ok(K.jload(afile)['ids_sha256'] == K.jload(os.path.join(W.SEG_DIR, 'chain-1024', 'read', 'changed_live_ids.json'))
       ['ids_sha256'], 'A.json carries the counted list\'s ids_sha256')
    ok(g_check('chain-1024', W.K0, CA1, 'chain', ['--a-from', os.path.join(W.AJOBS_DIR, 'days12', 'A.json')]) == 3
       and REG.load()['frozen']['seg'] == 'chain-1024'
       and 'not this segment' in json.dumps(K.jload(os.path.join(W.SEG_DIR, 'chain-1024', 'result.json'))),
       'chain-1024 checked with days12\'s A -> NOT A MEASUREMENT (A is not this segment\'s), K frozen')
    rc = g_check('chain-1024', W.K0, CA1, 'chain', ['--a-from', afile])
    res = K.jload(os.path.join(W.SEG_DIR, 'chain-1024', 'result.json'))
    lines = ' | '.join(ln for rr in res['rules'] for ln in rr['lines'])
    ok(rc == 0 and res['new'] == [NEW_C, NEW_D] and res['moved'] == [MOV] and res['A_ids_sha256'],
       f'chain PASS: daily fires {res["new"]} counted, {MOV} a counted chain-MOVED re-decision ({res["verdict"]})')
    ok('fresh FX' in lines and '97901' in lines and 'seq/published_at only' in lines,
       'chain NOTEs: 97901\'s eur-only move (HUF, published within 8 days, with its tender row) and a date-only move')
    adds = {x['id']: x for x in res['advance']['add']}
    ok(adds[NEW_C]['read'] == 'day7' and adds[NEW_C]['tag'] == 'chain-2026-10-24'
       and any('539' in f for f in adds[NEW_D]['flags']) and not adds[NEW_C]['flags'],
       'daily fires register chain-2026-10-24 (day-7 population); the DÖE one is flagged possibly 539-driven (A6/H)')
    reg_c = K.jload(W.REG_FILE)
    ok(REG.main(['advance', 'chain-1024']) == 0, 'registry advance chain-1024 (the PASS re-check clears its freeze)')
    reg = REG.load()
    ok(reg['K'] == CA1 and reg['n'] == 3480 and reg['entries'][str(MOV)]['moves'][-1]['kind'] == 'chain-MOVED'
       and reg['open_segment'] is None and reg['jobs_mark']['max_job_id'] == 2807
       and reg['jobs_mark']['measured_at'] == t0 + 3600,
       f'K -> {CA1}; R 3,480; the move recorded; the segment closed; the jobs mark = chain-1024\'s CA snapshot')
    ok(REG.main(['advance', 'chain-1024']) == 2, 'a segment advances once (its K is no longer the registry\'s)')
    before = open(W.REG_FILE, 'rb').read()
    ok(REG.main(['init', '--force']) == 2 and open(W.REG_FILE, 'rb').read() == before,
       'init --force refuses an advanced registry (R.json byte-identical)')

    # the jobs window (after the advance: mark 2807 @ 08:35Z 10-24); the 10-25 chain tick is 08:35Z (CET)
    t1 = int(SR.berlin_at(datetime.date(2026, 10, 25), 9, 35).timestamp())
    day25 = [job(2840 + k, kd, t1 + 10 + k) for k, kd in enumerate(
        ['probe', 'process', 'fetch-rates', 'rederive-eur-recent', 'project', 'reveal-recheck'])]
    ok(seg_tick('chain', 'ch-pre1', '2026-10-25', CA1,
                jf('j_pre1.json', jobs_body(chain_jobs + [job(2809, 'refold-sections', t0 + 7200)] + day25,
                                            t1 + 3600))) == 2
       and SEG.load_rec('ch-pre1')['foreign'][0]['pre_tick'],
       'a wet refold-sections between the mark and the tick: foreign (pre-tick) -> STOP')
    unset_open()
    ok(seg_tick('chain', 'ch-pre2', '2026-10-25', CA1,
                jf('j_pre2.json', jobs_body(chain_jobs + [dry_job(2809, t0 + 7200)] + day25, t1 + 3600))) == 0
       and SEG.load_rec('ch-pre2')['pre_tick_dry'][0]['job_id'] == 2809,
       'the previous chain\'s own dry A job (refold-lot-sum DRY RUN) between the mark and the tick: not foreign')
    unset_open()
    ok(seg_tick('chain', 'ch-pre3', '2026-10-25', CA1,
                jf('j_pre3.json', jobs_body(chain_jobs + [job(2809, 'process', t0 + 7200)] + day25, t1 + 3600))) == 2,
       'a catch-up chain kind (process) before the tick (restart catch_up_missed_tick): foreign -> STOP')
    unset_open()
    wk25 = int(SR.berlin_at(datetime.date(2026, 10, 25), 3, 10).timestamp())
    ok(seg_tick('chain', 'ch-pre4', '2026-10-25', CA1,
                jf('j_pre4.json', jobs_body(chain_jobs + [job(2830, 'data-quality', wk25 + 5)] + day25, t1 + 3600))) == 2
       and SEG.load_rec('ch-pre4')['weekly_unclosed'],
       'weekly kinds before the 10-25 chain tick with no weekly segment closed: STOP (W3), recorded as unclosed')
    unset_open()
    ok(seg_tick('chain', 'ch-pre5', '2026-10-24', CA1, jf_chain) == 0 and not SEG.load_rec('ch-pre5')['foreign'],
       'a job at or below the mark (2799, before the tick) is ignored')
    unset_open()
    ok(seg_tick('chain', 'ch-pre6', '2026-10-25', CA1,
                jf('j_pre6.json', jobs_body([job(2900 + k, kd, t1 + 10 + k) for k, kd in enumerate(
                    ['probe', 'process', 'fetch-rates', 'rederive-eur-recent', 'project', 'reveal-recheck'])],
                                            t1 + 3600))) == 3,
       'a jobs body whose recent[] no longer reaches back to the mark: NOT A MEASUREMENT (3)')
    # issue 222's weekday catch-up loop (10-26 is a Monday; tick 08:35Z)
    t2 = SR.berlin_at(datetime.date(2026, 10, 26), 9, 35)
    e2 = int(t2.timestamp())
    cu = job(2861, 'probe', e2 + 310, params='ted daily (catch-up)')

    def snap_of(recent, m):
        return {'recent': recent, 'queued': [], 'current': None, 'measured_at': m}

    ok(not SEG.catchup_state(snap_of([cu], e2 + 3 * 3600 + 360), t2)[0],
       'catch-up probe ran, no trailing daily, measured before tick + 3 h 07: not ready')
    ok(not SEG.catchup_state(snap_of([], e2 + 240), t2)[0], 'no catch-up row, measured at tick + 4 min: not ready')
    ok(SEG.catchup_state(snap_of([], e2 + 480), t2)[0], 'no catch-up row, measured at tick + 8 min: ready (a)')
    ok(SEG.catchup_state(snap_of([cu, job(2870, 'probe', e2 + 4000, params='ted daily (probe)'),
                                  job(2875, 'project', e2 + 5000)], e2 + 6000), t2)[0],
       'a trailing "ted daily (probe)" after the catch-up probe and its project: ready (b)')
    ok(SEG.catchup_state(snap_of([cu], e2 + 3 * 3600 + 480), t2)[0], 'the window is over (tick + 3 h 08): ready (c)')
    ok(not SEG.catchup_state({'recent': [], 'queued': [{'id': 1, 'kind': 'probe', 'params': 'ted daily (catch-up)'}],
                              'current': None, 'measured_at': e2 + 9000}, t2)[0], 'a queued catch-up probe: not ready')
    day26 = [job(2850 + k, kd, e2 + 10 + k) for k, kd in enumerate(
        ['probe', 'process', 'fetch-rates', 'rederive-eur-recent', 'project', 'reveal-recheck'])]
    ok(seg_tick('chain', 'ch-cu', '2026-10-26', CA1, jf('j_cu.json', jobs_body(chain_jobs + day26 + [cu], e2 + 3600)))
       == 2, 'a weekday chain whose catch-up loop may still push a daily: NOT READY')

    # chain-table negatives on the pure classifier (same reads, rows varied)
    frz = K.jload(W.FREEZE)
    rec_c = SEG.load_rec('chain-1024')
    rd_c = rdj
    rows_new_c = K.load_rows(os.path.join(W.SEG_DIR, 'chain-1024', 'read', 'new'))
    rows_r_c = K.load_rows(os.path.join(W.SEG_DIR, 'chain-1024', 'read', 'rpk'))
    band_c = K.jload(os.path.join(W.SEG_DIR, 'chain-1024', 'read', 'band.json'))
    LATE = dict(now=T('2026-10-25T01:00:00Z'), gap_end=T('2026-10-25T00:00:00Z'))

    def run(klass='chain', reg_=None, rec=None, rd=None, rn=None, rr=None, band=None, **opt):
        o = dict(klass=klass, corrected=opt.pop('corrected', None), A=opt.pop('A', 3000), explained=opt.pop('ex', None),
                 prediction=opt.pop('prediction', None), gap_end=opt.pop('gap_end', None), now=opt.pop('now', None),
                 d7=opt.pop('d7', None), own_corrected=opt.pop('own_corrected', None), A_bad=opt.pop('A_bad', None))
        outs, adv, info = G.classify(reg_ or reg_c, frz, rec or rec_c, rd or rd_c, rn if rn is not None else rows_new_c,
                                     rr if rr is not None else rows_r_c, band or band_c, o)
        code = 0
        for x in outs:
            code = W.worse(code, x.code)
        run.last = (outs, info)
        return code, ' | '.join(ln for x in outs for ln in x.lines), adv

    def esc_all():
        e = [x for o in run.last[0] for x in o.stop_esc]
        return bool(e) and all(e)

    def with_tr(rd, *ids):
        x = json.loads(json.dumps(rd))
        x['changed']['tenders'] = sorted(set(x['changed']['tenders']) | set(ids))
        return x

    code, _l, _a = run()
    ok(code == 0, 'classifier baseline (chain-1024) PASS')
    ok(run(A=None)[0] == 3, 'chain without A_d -> NOT A MEASUREMENT (the 3x rule needs it)')
    code, l, _a = run(A=10)
    ok(code == 2 and '3 x' in l and esc_all(), 'chain: 2 daily fires > 3 x A_d x 1.0974 % (A_d 10) -> STOP and read '
       '(a gap-end STOP)')
    ok(run(A=10, ex={'explanations': {'count_3x': 'read: both right'}})[0] == 0, '... explained -> PASS')
    ok(run(A=10, **LATE)[0] == 10, '... unexplained above 3x at the end of the gap -> SWITCH OFF (U9 trigger)')

    def vary(i, **kw):
        rr = dict(rows_r_c)
        rr[i] = dict(rows_r_c[i], **kw)
        return rr

    sil = P_plain[5]
    REDATE_C = next(int(i) for i, e in sorted(reg_c['entries'].items(), key=lambda kv: int(kv[0]))
                    if e['tag'] == 'P' and not e.get('pin') and int(i) not in reg_c['B_R']
                    and e['row']['current_value_currency'] not in (None, 'EUR'))
    e = rows_r_c[sil]
    det = json.loads(e['current_value_derivation_detail'])
    det['lots'] += 1
    gaprow = vary(sil, current_value_derivation_detail=json.dumps(det), current_seq=(e['current_seq'] or 0) + 1)
    code, l, _a = run(rr=gaprow)
    ok(code == 2 and 'announcement gap' in l and not esc_all(), 'chain SILENT: a detail move with a seq increase, no '
       'tender row -> STOP (announcement gap, W6; never escalates)')
    ok(run(rr=gaprow, **LATE)[0] == 2, '... still unexplained at the end of the gap: STOP, never a switch-off in a chain (W4)')
    code, l, _a = run(rr=vary(sil, current_value_cents=e['current_value_cents'] + 5))
    ok(code == 2 and 'unannounced writer' in l, 'chain SILENT: a value move with no tender row -> STOP (unannounced)')
    ok(run(rr=vary(sil, current_value_cents=e['current_value_cents'] + 5), **LATE)[0] == 10,
       '... unexplained at the end of the gap -> SWITCH OFF')
    ok(run(rr=vary(sil, current_value_cents=e['current_value_cents'] + 5),
           ex={'explanations': {str(sil): 'x'}})[0] == 0, '... explained -> PASS')
    nonEUR = next(int(i) for i, ee in sorted(reg_c['entries'].items(), key=lambda kv: int(kv[0]))
                  if ee['tag'] == 'P' and not ee.get('pin') and int(i) not in reg_c['B_R']
                  and ee['row']['current_value_currency'] not in (None, 'EUR') and int(i) != REDATE_C)
    fx = vary(nonEUR, current_value_eur_cents=rows_r_c[nonEUR]['current_value_eur_cents'] + 9,
              current_published_at=t0 - 86400)
    code, l, adv = run(rr=fx)
    ok(code == 2 and 'unannounced writer' in l and str(nonEUR) not in adv['refresh'],
       'chain: a fresh-FX eur-only move with NO tender row is SILENT (a legit EUR move writes a tender row), not a NOTE')
    ok(run(rr=fx, **LATE)[0] == 10 and run(rd=with_tr(rd_c, nonEUR), rr=fx)[0] == 0,
       '... SWITCH OFF unexplained at gap end; with its tender row it is the fresh-FX NOTE')
    br = reg_c['B_R'][3]
    rd_br = with_tr(rd_c, br)
    bv = dict(rows_r_c[br], current_value_cents=rows_r_c[br]['current_value_cents'] + 100,
              current_value_eur_cents=rows_r_c[br]['current_value_eur_cents'] + 100)
    rr = dict(rows_r_c)
    rr[br] = bv
    bnd = dict(band_c, rows=[([x[0], bv['current_value_eur_cents'], bv['current_value_cents']] + x[3:]) if x[0] == br
                             else x for x in band_c['rows']])
    code, l, _a = run(rd=rd_br, rr=rr, band=bnd)
    ok(code == 2 and 'B_R head moved' in l, 'chain: a B_R head moved with a tender row -> stop and read (W4)')
    ok(run(rd=rd_br, rr=rr, band=bnd, ex={'reads': {str(br): {'verdict': 'wrong'}}})[0] == 10, '... read wrong -> SWITCH OFF')
    ok(run(rd=rd_br, rr=rr, band=bnd, ex={'reads': {str(br): {'verdict': 'right'}}})[0] == 0, '... read right -> PASS')
    bnd2 = dict(band_c, rows=[x for x in band_c['rows'] if x[0] != br])
    code, l, _a = run(band=bnd2)
    ok(code == 2 and 'missing from the band' in l and not esc_all(), 'chain: a B_R head missing -> stop and read')
    ok(run(band=bnd2, ex={'reads': {str(br): {'verdict': 'wrong'}}})[0] == 10, '... read wrong -> SWITCH OFF')
    big = dict(rows_new_c)
    big[NEW_C] = dict(rows_new_c[NEW_C], current_value_cents=2 * 10**10, current_value_eur_cents=2 * 10**10)
    bnd3 = dict(band_c, rows=band_c['rows'] + [[NEW_C, 2 * 10**10, 2 * 10**10, 'EUR', None]])
    code, l, _a = run(rn=big, band=bnd3)
    ok(code == 2 and 'extra band row' in l, 'chain: a daily fire >= EUR 100 m -> extra band row, read at 0 wrong')
    code, _l, adv = run(rn=big, band=bnd3, ex={'reads': {str(NEW_C): {'verdict': 'right', 'ref': '551'}}})
    ok(code == 0 and NEW_C in adv['band_add'], '... read right -> PASS, joins B_R at advance')
    ok(run(rn=big, band=bnd3, ex={'reads': {str(NEW_C): {'verdict': 'wrong'}}})[0] == 10, '... read wrong -> SWITCH OFF')
    pv = 369057
    code, l, _a = run(rd=with_tr(rd_c, pv), rr=vary(pv, current_value_cents=1))
    ok(code == 2 and 'path-V' in l, 'chain: a path-V pin value move with a tender row -> stop and explain')
    ok(run(rr=vary(6465510, current_value_derivation='lot-sum-awards'))[0] == 10, '6465510 serving lot-sum-awards -> '
       'SWITCH OFF')
    for pin, what in ((8440393, 'must-stay'), (369057, 'path-V')):
        flip = lsa_row(pin, 4321, nid=47500099)
        code, l, adv = run(rd=with_tr(rd_c, pin), rr=upd(rows_r_c, pin, flip))
        ok(code == 2 and 'W5' in l and pin not in [x['id'] for x in adv['add']] and pin not in run.last[1]['new'],
           f'chain: {what} {pin} -> lot-sum-awards with a tender row: stop and explain (W5), never a counted daily fire')
    code, _l, adv = run(rd=with_tr(rd_c, 8440393), rr=upd(rows_r_c, 8440393, lsa_row(8440393, 4321)),
                        ex={'explanations': {'8440393': 'x'}})
    ok(code == 0 and 8440393 in [x['id'] for x in adv['add']] and REG.pin_role(8440393) == 'must-stay',
       '... explained -> PASS (registered read-pending; advance keeps its pin role)')
    rd_dc = with_tr(rd_c, 6908248)
    code, l, _a = run(rd=rd_dc, rr=vary(6908248, current_value_cents=rows_r_c[6908248]['current_value_cents'] * 2))
    ok(code == 2 and 'double count' in l, 'chain: 6908248 moved -> stop and read (a double count is a switch-off)')
    ok(run(rd=rd_dc, rr=vary(6908248, current_value_cents=rows_r_c[6908248]['current_value_cents'] * 2),
           ex={'reads': {'6908248': {'verdict': 'wrong'}}})[0] == 10, '... read wrong (a double count) -> SWITCH OFF')
    rd_pw = with_tr(rd_c, 585917)
    code, l, _a = run(rd=rd_pw, rr=vary(585917, current_value_cents=24_000_000, current_value_eur_cents=24_000_000))
    ok(code == 2 and 'path-W pin moved' in l, 'chain: a path-W pin (585917) moved with a tender row -> stop and read')
    code, _l, adv = run(rd=rd_pw, rr=vary(585917, current_value_cents=24_000_000, current_value_eur_cents=24_000_000),
                        ex={'reads': {'585917': {'verdict': 'right', 'ref': '551'}}})
    ok(code == 0 and 585917 in [m['id'] for m in adv['moved']], '... read right -> a counted chain-MOVED head')
    code, l, adv = run(rd=with_tr(rd_c, REDATE_C), rr=vary(REDATE_C, current_value_eur_cents=rows_r_c[REDATE_C]
                                                           ['current_value_eur_cents'] + 9,
                                                           current_published_at=1700000000))
    ok(code == 0 and REDATE_C in [x['id'] for x in adv['moved']], 'chain: an eur-only move on a non-EUR head NOT '
       'published within 8 days, with a tender row -> a counted chain-MOVED (not a fresh-FX NOTE)')
    code, _l, adv = run(rd=with_tr(rd_c, LOSTX), rr=vary(LOSTX, current_value_derivation=None))
    ok(code == 0 and [x['id'] for x in adv['lost']] == [LOSTX], 'chain: LOST with a tender row -> counted, LOST at advance')
    used = set(P_plain[:400]) | set(rd_c['changed']['tenders'])
    vb2 = next(int(i) for i, ee in sorted(reg_c['entries'].items(), key=lambda kv: int(kv[0]))
               if ee['tag'] == 'P' and not ee.get('pin') and len(ee['vouches']) > 1 and int(i) not in reg_c['B_R']
               and int(i) not in used and ee['row']['current_value_currency'] == 'EUR')
    dvb = json.loads(rows_r_c[vb2]['current_value_derivation_detail'])
    cur_vk = K.vb_key(dvb['vouched_by'])
    other = next(v for v in reg_c['entries'][str(vb2)]['vouches'] if v != cur_vk)
    dvb['vouched_by'] = {'field': other[0], 'notice_id': other[1], 'cents': other[2]}
    swap = vary(vb2, current_value_derivation_detail=json.dumps(dvb))
    code, l, adv = run(rr=swap)
    ok(code == 2 and 'unannounced writer' in l and str(vb2) not in adv['refresh'],
       'vouched_by swapped to another ACCEPTED vouch with no tender row: SILENT (the evidence is not refreshed away)')
    code, l, adv = run(rd=with_tr(rd_c, vb2), rr=swap)
    ok(code == 0 and str(vb2) in adv['refresh'] and 'W15' in l, '... with a tender row: W15 membership, refreshed')
    fj = jf('jobs_chain_foreign.json', jobs_body(chain_jobs + [job(2809, 'refold-sections', t0 + 900)], t0 + 3600))
    ok(seg_tick('chain', 'chain-x', '2026-10-24', CA1, fj) == 2
       and not SEG.load_rec('chain-x')['foreign'][0].get('pre_tick'), 'a non-chain job after the tick is foreign -> STOP')
    unset_open()

    # ------------------------------------------------------------ T5 P0 (class D)
    print('T5 P0: the W-on probe for 546 U3 (class D, E1 and the all-kind reader side by side)')
    tcb = t0 + 4 * 3600
    clock(tcb)
    ok(REG.main(['status']) == 0, 'status READY after the chain check advanced')
    ok(SEG.main(['probe-jobs', 'p0']) == 0, 'probe job files (R ∪ Rg ∪ pins, one run)')
    pj = K.jload(os.path.join(W.SEG_DIR, 'p0', 'jobs', 'p0_ids.json'))
    ok(pj['n'] == 3480 + 63 and pj['n'] <= 5000, f'p0: {pj["n"]} ids')
    sf = tf('p0_dry.txt', dry_summary(pj['n'] - 63, pj['n'], list(range(1, 64))))
    jd = os.path.join(W.SEG_DIR, 'p0', 'jobs')
    ok(MW.main(['p0', '--summary-file', sf, '--jobs-dir', jd]) == 2 and not os.path.exists(os.path.join(jd, 'p0_wet.json')),
       'make_wet: a W run (jobs under u9/) without --runs-left is a STOP (U7\'s stages.json never judges it)')
    ok(MW.main(['p0', '--summary-file', sf, '--jobs-dir', jd, '--runs-left', '1', '--minutes-per-run', '30',
                '--switchoff-gate', 'covered', '--now', '2026-10-25T07:40:00Z']) == 2
       and not os.path.exists(os.path.join(jd, 'p0_wet.json')), 'make_wet W mode: NO-GO at 10-25 07:40Z (DST) -> no wet JSON')
    ok(MW.main(['p0', '--summary-file', sf, '--jobs-dir', jd, '--runs-left', '1', '--minutes-per-run', '30',
                '--switchoff-gate', 'covered', '--now', '2026-10-24T10:00:00Z']) == 0
       and json.load(open(os.path.join(jd, 'p0_wet.json')))['expect'] == pj['n'] - 63,
       'make_wet W mode: GO at 10-24 10:00Z, wet JSON with expect = dry admitted')
    K0b = CA1
    idle = jf('jobs_idle.json', jobs_body(chain_jobs + [dry_job(2810, tcb - 600)], tcb))
    ok(seg_open('p0', 'D', 'probe-512b297', K0b + 1, idle) == 2, 'open refuses CB != K (unchecked writes)')
    ok(seg_open('p0', 'D', 'probe-512b297', K0b, jf('jobs_busy.json', jobs_body(chain_jobs, tcb, queued=[
        {'id': 2811, 'kind': 'x', 'params': ''}]))) == 2, 'open refuses a non-empty queue at CB')
    ok(seg_open('p0', 'D', 'bad tag', K0b, idle) == 3, 'a malformed tag is refused (B2 source tags)')
    ok(seg_open('p0', 'D', 'probe-512b297', K0b, idle, extra=['--salt', '551-b6-drain-546-']) == 3,
       'a salt on a probe tag is refused (551 defines none)')
    ok(seg_open('p0', 'D', 'probe-512b297', K0b, jf('jobs_wetsince.json', jobs_body(
        chain_jobs + [job(2810, 'refold-lot-sum', tcb - 600, counts='refold-lot-sum (issue 551): 3 of 3 named')], tcb)))
       == 2, 'open refuses a wet job since the mark (it belongs to no segment: gap)')
    ok(seg_open('p0', 'D', 'probe-512b297', K0b, idle, now='2026-10-24T23:30:00Z', mpr='120') == 2,
       'open refuses when the start rule says NO-GO for the segment\'s own runs (D.1)')
    ok(seg_open('p0', 'D', 'probe-512b297', K0b, idle, lock='Writer: 551 p0 until 2026-10-24T13:00Z',
                extra=['--expect-new', '0', '--expect-moves', '0 W-column moves', '--bar', '0 W-column moves']) == 0,
       'p0 OPEN at CB = K (two equal /health reads around the jobs body; start rule GO; the dry job is exempt)')
    rec_open = SEG.load_rec('p0')
    ok(rec_open['switchoff_by'] == '2026-10-25T01:40:00Z' and rec_open['prereg']['salt'] is None,
       'p0 records its latest switch-off start (01:40Z: the weekly tick binds) as its gap end; probe salt null')
    ok(REG.main(['status', '--no-plan-check']) == 2, 'status HOLD while p0 is open')
    REDATE = next(int(i) for i, e in sorted(reg['entries'].items(), key=lambda kv: int(kv[0]))
                  if e['tag'] == 'P' and not e.get('pin') and int(i) not in reg['B_R']
                  and e['row']['current_value_currency'] not in (None, 'EUR'))
    for t in P_plain[10:60]:
        r = dict(fb.tenders[t])
        r['current_seq'] = (r['current_seq'] or 0) + 1
        fb.tenders[t] = r
        fb.add('tender', t, tcb + 60)
    r = dict(fb.tenders[REDATE])
    r.update(current_published_at=(r['current_published_at'] or 0) + 86400,
             current_value_eur_cents=r['current_value_eur_cents'] + 77)
    fb.tenders[REDATE] = r
    fb.add('tender', REDATE, tcb + 61)
    for t in P_plain[60:70]:
        lot = 91_000_000 + t % 100000
        fb.lots[lot] = t
        fb.add('lot', lot, tcb + 62)
    CA_p0 = fb.cursor()
    p0_jobs = chain_jobs + [dry_job(2810, tcb - 600), job(2811, 'refold-lot-sum', tcb + 10),
                            job(2812, 'project', tcb + 40)]
    ca_jobs = jf('jobs_p0_ca.json', jobs_body(p0_jobs, tcb + 900))
    ok(seg_close('p0', CA_p0, ca_jobs, '2811,2812', ca2=CA_p0 + 1) == 2 and SEG.load_rec('p0')['status'] == 'open',
       'close: two CA reads that differ -> NOT YET, the segment stays open')
    ok(seg_close('p0', CA_p0, ca_jobs, '2811,2812') == 0, 'p0 CLOSED: the jobs in (CB, CA] are exactly its own (W10)')
    clock(tcb + 900)
    ok(g_read('p0', K0b, CA_p0, 'D', ca_jobs, fb, ['--route', 'e1']) == 3,
       'gd5 read --route e1 on a span <= 50k is refused')
    n0 = len(fb.sent)
    ok(g_read('p0', K0b, CA_p0, 'D', ca_jobs, fb, ['--route', 'both']) == 0, 'gd5 read p0 --route both')
    rdp = K.jload(os.path.join(W.SEG_DIR, 'p0', 'read', 'read.json'))
    ok(rdp['changed']['parity']['tenders_equal'] and rdp['changed']['parity']['lot_only_live'] == 10,
       'E1 = the all-kind reader on the tender rows; 10 lot-only Tenders read beside')
    ok(len(fb.sent) - n0 <= 8, f'p0 read: {len(fb.sent) - n0} statements (decision: 4-5 + the all-kind reader)')
    rc = g_check('p0', K0b, CA_p0, 'D')
    res = K.jload(os.path.join(W.SEG_DIR, 'p0', 'result.json'))
    lines = ' | '.join(ln for rr in res['rules'] for ln in rr['lines'])
    ok(rc == 0 and 'seq/published_at only' in lines and 're-dated' in lines and str(REDATE) in lines,
       'p0 PASS: 0 W-column moves; date-only NOTEs; an eur-only move on a re-dated head (its tender row) is a NOTE')
    reg_p = K.jload(W.REG_FILE)
    ok(REG.main(['advance', 'p0']) == 0 and REG.load()['K'] == CA_p0, 'registry advance p0')

    rec_p = SEG.load_rec('p0')
    rn_p = K.load_rows(os.path.join(W.SEG_DIR, 'p0', 'read', 'new'))
    rr_p = K.load_rows(os.path.join(W.SEG_DIR, 'p0', 'read', 'rpk'))
    bd_p = K.jload(os.path.join(W.SEG_DIR, 'p0', 'read', 'band.json'))

    def runD(klass='D', rd=None, rr=None, rn=None, band=None, rec=None, reg_=None, **kw):
        return run(klass=klass, reg_=reg_ or reg_p, rec=rec or rec_p, rd=rd or rdp, rn=rn if rn is not None else rn_p,
                   rr=rr if rr is not None else rr_p, band=band or bd_p, **kw)

    def varyp(i, **kw):
        rr = dict(rr_p)
        rr[i] = dict(rr_p[i], **kw)
        return rr

    tW = P_plain[12]                                        # has a tender row in p0's range
    ok(runD()[0] == 0, 'classifier baseline (p0) PASS')
    code, l, _a = runD(rr=varyp(tW, current_value_cents=rr_p[tW]['current_value_cents'] + 1))
    ok(code == 2 and 'a probe that moves a W column' in l and esc_all(), 'D: a W-column move (S) -> STOP (gap-end)')
    ok(runD(rr=varyp(tW, current_value_cents=rr_p[tW]['current_value_cents'] + 1),
            ex={'reads': {str(tW): {'verdict': 'wrong'}}})[0] == 10, 'D: a MOVED head read wrong -> SWITCH OFF')
    code, _l, adv = runD(rr=varyp(tW, current_value_cents=rr_p[tW]['current_value_cents'] + 1),
                         ex={'explanations': {str(tW): 'ported on refetched facts: right'}})
    ok(code == 0 and adv['moved'][0]['kind'] == 'explained-MOVED', 'D: an explained MOVED -> PASS, recorded in R')
    ok(runD(rr=varyp(tW, current_value_cents=rr_p[tW]['current_value_cents'] + 1), now=T('2026-10-25T00:00:00Z'),
            gap_end=T('2026-10-24T23:00:00Z'))[0] == 10, 'D: MOVED unexplained at the end of the gap -> SWITCH OFF')
    t2x = P_plain[100]
    code, l, _a = runD(rd=with_tr(rdp, t2x), rr=varyp(t2x, current_value_eur_cents=rr_p[t2x]['current_value_eur_cents']
                                                      + 1))
    ok(code == 2 and 'MOVED' in l, 'D: an eur-only move on a head that was NOT re-dated -> STOP (MOVED)')
    rd_noRE = json.loads(json.dumps(rdp))
    rd_noRE['changed']['tenders'] = [x for x in rd_noRE['changed']['tenders'] if x != REDATE]
    code, l, adv = runD(rd=rd_noRE)
    ok(code == 2 and 'unannounced writer' in l and str(REDATE) not in adv['refresh'],
       'D: the re-dated eur-only move WITHOUT its tender row -> SILENT STOP (not the re-dated NOTE)')
    ok(runD(rd=rd_noRE, **LATE)[0] == 10 and runD(rd=rd_noRE, ex={'explanations': {str(REDATE): 'x'}})[0] == 0,
       '... SWITCH OFF unexplained at gap end; explained -> PASS')
    code, l, adv = runD(rr=upd(rr_p, vb2, dict(rr_p[vb2], current_value_derivation_detail=json.dumps(dvb),
                                                 current_seq=(rr_p[vb2]['current_seq'] or 0) + 1)))
    ok(code == 2 and 'announcement gap' in l, 'D: vouched_by swapped to another accepted vouch, seq+1, no tender row '
       '-> STOP (an announcement gap, W6)')
    ok(runD(rr=upd(rr_p, vb2, dict(rr_p[vb2], current_value_derivation_detail=json.dumps(dvb),
                                      current_seq=(rr_p[vb2]['current_seq'] or 0) + 1)), **LATE)[0] == 10,
       '... in class D an announcement gap still unexplained at the end of the gap is SWITCH OFF (F)')
    ok(runD(rd=with_tr(rdp, vb2), rr=upd(rr_p, vb2, dict(rr_p[vb2], current_value_derivation_detail=json.dumps(dvb))))
       [0] == 0, '... with a tender row: W15 membership, PASS')
    for pin, mut, what in ((8426318, {'current_value_cents': 1}, 'a path-W pin off'),
                           (6679375, {'current_value_cents': 1}, 'a partial sum off its S')):
        ok(runD(rd=with_tr(rdp, pin), rr=varyp(pin, **mut))[0] == 10, f'D: {what} -> SWITCH OFF')
        ok(runD(rr=varyp(pin, **mut))[0] == 10 and pin in run.last[1]['silent'],
           f'D: {what} with NO tender row -> SWITCH OFF at once (F), recorded SILENT')
        code, _l, adv = runD(rr=varyp(pin, **mut), ex={'explanations': {str(pin): 'x'}})
        ok(code == 10 and not adv['moved'], f'... no explanation overrides it (never recorded as a move)')
    ok(runD(rr=varyp(8426318, current_value_derivation='x', current_seq=(rr_p[8426318]['current_seq'] or 0) + 1))[0]
       == 10, 'D: a path-W pin LOST with a seq increase and no tender row -> SWITCH OFF (outranks the W6 gap STOP)')
    ok(runD(rr=varyp(8426318, current_value_eur_cents=(rr_p[8426318]['current_value_eur_cents'] or 0) + 1))[0] == 2,
       'D: an eur-only move on 8426318 with no tender row stays a SILENT STOP (path-W checks no eur)')
    dc = varyp(6908248, current_value_cents=1)
    ok(runD(rd=with_tr(rdp, 6908248), rr=dc)[0] == 2, 'D: 6908248 moved with no read -> STOP (stop and read)')
    ok(runD(rd=with_tr(rdp, 6908248), rr=dc, ex={'reads': {'6908248': {'verdict': 'wrong'}}})[0] == 10,
       '... read wrong (serving a double count) -> SWITCH OFF')
    ok(runD(rd=with_tr(rdp, 6908248), rr=dc, ex={'reads': {'6908248': {'verdict': 'right'}}})[0] == 0,
       '... read right -> PASS')
    ok(runD(rd=with_tr(rdp, 6908248), rr=dc, ex={'explanations': {'6908248': 'x'}})[0] == 2,
       '... an explanation alone does not pass it (it needs the read)')
    dcl = varyp(6908248, current_value_derivation=None)
    ok(runD(rd=with_tr(rdp, 6908248), rr=dcl)[0] == 2 and runD(rd=with_tr(rdp, 6908248), rr=dcl, **LATE)[0] == 10,
       'D: 6908248 LOST -> an ordinary LOST (STOP; SWITCH OFF unexplained at gap end), R3\'s expected refusal')
    pdet = json.loads(rr_p[7153588]['current_value_derivation_detail'])
    pdet['vouched_by'] = {'field': 'TED-VAL_TOTAL', 'notice_id': 1, 'cents': 1}
    ok(runD(rd=with_tr(rdp, 7153588), rr=varyp(7153588, current_value_derivation_detail=json.dumps(pdet)))[0] == 10,
       'D: a partial sum whose vouched_by is not among its vouches -> SWITCH OFF')
    rnx = dict(rn_p)
    rdx = json.loads(json.dumps(rdp))
    for k in range(31):
        rnx[9_000_000 + k] = lsa_row(9_000_000 + k, 1000 + k)
        rdx['changed']['tenders'].append(9_000_000 + k)
    code, l, _a = runD(rd=rdx, rn=rnx)
    ok(code == 2 and 'class premise is broken' in l and 'enumeration miss' in l,
       'D: NEW (expected 0) -> STOP; > 30 -> programme STOP (U6(a): a residual is an enumeration miss)')
    ok(runD(rd=rdx, rn=rnx, ex={'explanations': {'class_premise': 'x'}})[0] == 3,
       'class_premise needs the segment\'s pre-registered salt (a probe has none): NOT A MEASUREMENT')
    rec_salt = dict(rec_p, prereg=dict(rec_p['prereg'], salt='551-b6-drain-576-'))
    drawn = K.salted([9_000_000 + k for k in range(31)], '551-b6-drain-576-')[:30]
    reads_ok = {str(i): {'verdict': 'right'} for i in drawn}
    ok(runD(rd=rdx, rn=rnx, rec=rec_salt, ex={'explanations': {'class_premise': 'x'}, 'reads': reads_ok})[0] == 0,
       'class_premise + the salted 30 read right: the NEW outside the draw is a note -> PASS')
    ok(runD(rd=rdx, rn=rnx, rec=rec_salt, ex={'explanations': {'class_premise': 'x'},
                                               'reads': dict(reads_ok, **{str(drawn[0]): {'verdict': 'wrong'}})})[0] == 10,
       '... a drawn NEW read wrong -> SWITCH OFF')
    ok(runD(rd=rdx, rn=rnx, rec=rec_salt, ex={'explanations': {'class_premise': 'x'}})[0] == 2,
       '... a drawn NEW unread -> STOP')
    rnx2 = {9_000_000: lsa_row(9_000_000, 1000)}
    rdx2 = with_tr(rdp, 9_000_000)
    ok(runD(rd=rdx2, rn=rnx2, ex={'reads': {'9000000': {'verdict': 'wrong'}}})[0] == 10, 'D: a NEW read wrong -> SWITCH OFF')
    ok(runD(rd=rdx2, rn=rnx2, ex={'explanations': {'9000000': 'a 541 residual: enumeration miss, read right'}})[0] == 0,
       'D: a NEW explained (enumeration miss) -> PASS, registered read-pending')
    ok(runD(rn={9_000_001: lsa_row(9_000_001, 5)})[0] == 2, 'a NEW with no tender row -> SILENT STOP')
    ok(runD(rn={9_000_001: lsa_row(9_000_001, 5)}, **LATE)[0] == 10, '... SWITCH OFF unexplained at the end of the gap')
    flip = lsa_row(5316712, 4321, nid=47500098)
    ok(runD(rd=with_tr(rdp, 5316712), rr=upd(rr_p, 5316712, flip))[0] == 2 and 'W5' in ' '.join(
        ln for o in run.last[0] for ln in o.lines), 'D: must-stay 5316712 -> lot-sum-awards: stop and explain (W5)')
    ok(runD(rec=dict(rec_p, foreign=[{'job_id': 2813, 'kind': 'refold'}]))[0] == 2, 'a foreign job in (CB, CA] -> STOP')
    ok(runD(rec=dict(rec_p, foreign=[{'job_id': 2813, 'kind': 'refold'}]),
            ex={'explanations': {'foreign_jobs': 'x'}})[0] == 0, '... explained -> PASS')
    ok(runD(rec=dict(rec_p, foreign=[{'job_id': 2813, 'kind': 'merge-provisional-orgs'}]),
            ex={'explanations': {'foreign_jobs': 'x'}})[0] == 10,
       'a foreign in-place W-input writer without its pre-step -> SWITCH OFF, even with foreign_jobs explained (F)')
    ok(runD(rec=dict(rec_p, missing_own=[2899]))[0] == 2
       and runD(rec=dict(rec_p, missing_own=[2899]), ex={'explanations': {'missing_own': 'a typo in --own'}})[0] == 0,
       'a missing own job -> STOP; missing_own explained -> PASS')
    rdx = json.loads(json.dumps(rdp))
    rdx['changed']['parity']['tenders_equal'] = False
    ok(runD(rd=rdx)[0] == 2, 'E1 / all-kind parity off -> STOP')
    rdx = json.loads(json.dumps(rdp))
    rdx['changed']['cls']['unknown_kinds'] = ['widget']
    ok(runD(rd=rdx)[0] == 2 and runD(rd=rdx, ex={'explanations': {'unknown_kind:widget': 'x'}})[0] == 0
       and runD(rd=rdx, ex={'explanations': {'unknown_kind:gadget': 'x'}})[0] == 2,
       'an unknown kind -> STOP; cleared per kind (unknown_kind:widget), not by another kind\'s key')
    rdx = json.loads(json.dumps(rdp))
    rdx['changed']['cls']['unmapped_lots']['bad'] = [123]
    ok(runD(rd=rdx)[0] == 2 and runD(rd=rdx, ex={'explanations': {'vanished_lots': 'cursor 123: x'}})[0] == 0
       and runD(rd=rdx, ex={'explanations': {'vanished_lots': 'cursor 124'}})[0] == 2,
       'a vanished lot -> STOP; vanished_lots clears it only when it names the cursor')
    rdx = json.loads(json.dumps(rdp))
    rdx['changed']['retired'] = [P_plain[3]]
    rrx = dict(rr_p)
    del rrx[P_plain[3]]
    code, l, _a = runD(rd=rdx, rr=rrx)
    ok(code == 2 and 'RETIRED' in l, 'a retired R member -> STOP')
    rrx = dict(rr_p)
    del rrx[P_plain[4]]
    ok(runD(rr=rrx)[0] == 3, 'an R member with no read -> NOT A MEASUREMENT')
    ok(runD(own_corrected=7, rd=dict(rdp, changed=dict(rdp['changed'], tenders=[])))[0] == 3,
       'own jobs that report corrected > 0 over a range with no tender row: NOT A MEASUREMENT (CA too small)')
    far = next(i for i in reg_p['B_R'] if i not in W.NEAR)
    ok(runD(band=dict(bd_p, rows=[x for x in bd_p['rows'] if x[0] != far]))[0] == 10, 'D: a B_R head missing -> SWITCH OFF')
    ok(runD(band=dict(bd_p, rows=[x for x in bd_p['rows'] if x[0] != 5842187]))[0] == 2,
       'D: 5842187 (within 2 % of EUR 100 m) missing -> stop and explain')
    code, l, _a = runD(band=dict(bd_p, rows=bd_p['rows'] + [[42, 5 * 10**10, 5 * 10**10, 'EUR', None]]))
    ok(code == 2 and 'unread writer' in l and not esc_all(), 'an lsa band row outside R and the segment -> STOP (find '
       'the writer; never a gap-end escalation)')
    ok(runD(band=dict(bd_p, rows=[[i, 2 * 10**10, 1, 'EUR', None] for i in range(200)]))[0] == 3,
       'a LIMIT-capped band seek -> NOT A MEASUREMENT')
    rdx = with_tr(rdp, 369057)
    ok(runD(rd=rdx, rr=varyp(369057, current_value_cents=1))[0] == 2
       and runD(rd=rdx, rr=varyp(369057, current_value_cents=1), ex={'explanations': {'369057': 'x'}})[0] == 0,
       'a path-V value move -> stop-and-explain (W5); explained -> PASS')
    ok(runD(rr=varyp(369057, current_value_cents=1), **LATE)[0] == 10,
       'D: a path-V value move with NO tender row, unexplained at the end of the gap -> SWITCH OFF (SILENT)')
    ok(runD(rr=varyp(402503, current_value_cents=12345))[0] == 2, 'an Rg value move with no tender row -> SILENT STOP')
    rg_gap = varyp(402503, current_value_derivation='lot-sum-vouched', current_seq=(rr_p[402503]['current_seq'] or 0) + 1)
    ok(runD(rr=rg_gap, **LATE)[0] == 10 and run(rr=upd(rows_r_c, 402503, dict(rows_r_c[402503], **{
        'current_value_derivation': 'lot-sum-vouched', 'current_seq': (rows_r_c[402503]['current_seq'] or 0) + 1})),
        **LATE)[0] == 2, 'an Rg announcement gap at the end of the gap: SWITCH OFF in class D, STOP in a chain (W4)')
    code, l, _a = runD(rd=with_tr(rdp, 402503), rr=varyp(402503, current_value_cents=12345))
    ok(code == 0 and 'not a path-W input' in l, 'an Rg value move WITH a tender row (not lot-sum-awards) -> NOTE')
    ok(runD(rd=dict(rdp, complete=False))[0] == 3, 'an incomplete read -> NOT A MEASUREMENT')
    ok(runD(A_bad='A is not this segment\'s')[0] == 3, 'A not this segment\'s -> NOT A MEASUREMENT')
    pred = {'9000000': {'S': 1000, 'currency': 'EUR', 'n': 2, 'vouches': [['BT-118-NoticeResult', 47500000, 1000]]}}
    ok(runD('I', rd=rdx2, rn=rnx2, prediction=pred)[0] == 0, 'I: a predicted fire at the prediction -> PASS')
    ok(runD('I', rd=rdx2, rn=rnx2, prediction={'9000000': dict(pred['9000000'], S=999)})[0] == 10,
       'I: a fire off the prediction -> SWITCH OFF (D1 rules)')
    ok(runD('I', rd=rdx2, rn=rnx2, prediction={})[0] == 10, 'I: an unpredicted fire -> SWITCH OFF')
    ok(runD('I', prediction=pred)[0] == 2 and runD('I', prediction=pred, **LATE)[0] == 10,
       'I: a predicted fire that did not happen -> STOP explain; unexplained at the end of the gap -> SWITCH OFF')
    ok(runD('I')[0] == 3, 'I without --prediction -> NOT A MEASUREMENT')
    hI = P_plain[200]
    eI = reg_p['entries'][str(hI)]
    rowI = dict(rr_p[hI], current_value_cents=eI['row']['current_value_cents'] + 500,
                current_value_eur_cents=eI['row']['current_value_eur_cents'] + 500)
    detI = K.detail_of(rowI)
    pI = {'S': rowI['current_value_cents'], 'currency': rowI['current_value_currency'], 'n': detI.get('lots'),
          'vouches': [K.vb_key(detI.get('vouched_by'))]}
    rdI = with_tr(rdp, hI)
    code, _l, adv = runD('I', rd=rdI, rr=upd(rr_p, hI, rowI), prediction={str(hI): pI})
    ok(code == 0 and adv['moved'][0]['kind'] == 'predicted-MOVED' and adv['moved'][0]['vouches'] == pI['vouches'],
       'I: an R head moved exactly as predicted -> PASS, recorded with the port\'s vouches')
    ok(runD('I', rd=rdI, rr=upd(rr_p, hI, dict(rowI, current_value_cents=rowI['current_value_cents'] + 1)),
            prediction={str(hI): pI})[0] == 10, 'I: an R head off its prediction -> SWITCH OFF (class-I per-id mismatch)')
    ok(runD('I', rd=rdI, prediction={str(hI): pI})[0] == 2 and runD('I', rd=rdI, prediction={str(hI): pI}, **LATE)[0] == 10
       and runD('I', rd=rdI, prediction={str(hI): pI}, ex={'explanations': {str(hI): 'x'}})[0] == 0,
       'I: a predicted R move that did not happen -> STOP; SWITCH OFF at gap end; explained -> PASS')
    code, _l, adv = runD('I', rd=rdI, rr=varyp(hI, current_value_derivation=None), prediction={str(hI): None})
    ok(code == 0 and [x['id'] for x in adv['lost']] == [hI], 'I: predicted LOST and lost -> PASS, LOST at advance')
    ok(runD('I', rd=rdI, prediction={str(hI): None})[0] == 2, 'I: predicted LOST but unchanged -> STOP')
    ok(runD('I', rd=rdI, rr=upd(rr_p, hI, rowI), prediction={str(hI): None})[0] == 10,
       'I: predicted LOST but still lot-sum-awards at a moved value -> SWITCH OFF')
    ok(runD('I', rd=rdI, rr=upd(rr_p, hI, rowI), prediction={})[0] == 10,
       'I: an R head in R ∩ touched that moved and is absent from the prediction -> SWITCH OFF')
    trio = lsa_row(6450467, 166066168294, nid=47500097)
    tp = {'S': 166066168294, 'currency': 'EUR', 'n': 2, 'vouches': [['BT-118-NoticeResult', 47500097, 166066168294]]}
    ok(runD('I', rd=with_tr(rdp, 6450467), rr=upd(rr_p, 6450467, trio), prediction={'6450467': tp})[0] == 2,
       'I: a trio pin flipping to lot-sum-awards, even as predicted -> STOP (W5 stop-and-explain wins)')
    code, _l, _a = runD('I', rd=rdx2, rn=rnx2, prediction=dict(pred, **{'9000005': pred['9000000']}), **LATE)
    ok(code == 10, 'I: a non-R predicted fire that never happened, at the end of the gap -> SWITCH OFF (D1)')
    runD(rr=varyp(tW, current_value_cents=rr_p[tW]['current_value_cents'] + 1))
    ok(esc_all(), 'a MOVED STOP is a gap-end STOP (escalates at the end of the gap)')

    # ------------------------------------------------------------ T6 539 step 1 (class Z)
    print('T6 539 step 1 (class Z): the zero-change shortcut, corrected, close refusals, --redo, abort')
    Kz = REG.load()['K']
    tz = tcb + 3600
    clock(tz)
    idle2 = jf('jobs_idle2.json', jobs_body(p0_jobs, tz))
    ok(seg_open('z539-1', 'Z', 'drain-539-1', Kz, idle2, lock='Writer: 551 z539-1 until 2026-10-24T13:00Z', mpr='60',
                extra=['--salt', '551-b6-drain-540-']) == 3, 'a salt that is not the tag\'s (551-b6-drain-539-) is refused')
    ok(seg_open('z539-1', 'Z', 'drain-539-1', Kz, idle2, lock='Writer: 551 z539-1 until 2026-10-24T13:00Z', mpr='60')
       == 0 and SEG.load_rec('z539-1')['prereg']['salt'] == '551-b6-drain-539-',
       'z539-1 OPEN before its dry run (it writes its mechanical rows as walked); the salt derived from the tag')
    z_jobs = p0_jobs + [job(2813, 'repair-notice-instants', tz + 10)]
    ca_z = jf('jobs_z_ca.json', jobs_body(z_jobs, tz + 600))
    ok(seg_close('z539-1', Kz, ca_z, '2813', ['--corrected', '0']) == 0,
       'z539-1 CLOSED (CA = CB, only its own job; a typed 0 for repair-notice-instants)')
    ok(seg_close('z539-1', Kz, ca_z, '2813', ['--corrected', '0']) == 2, 'a closed segment is not closed again')
    ok(seg_close('z539-1', Kz, ca_z, '2813', ['--corrected', '0', '--redo']) == 0,
       '--redo re-closes it while its read has not run')
    n0 = len(fb.sent)
    ok(g_read('z539-1', Kz, Kz, 'Z', ca_z, fb) == 0 and len(fb.sent) == n0, 'Z shortcut: K == CA, 0 statements')
    ok(seg_close('z539-1', Kz, ca_z, '2813', ['--corrected', '0', '--redo']) == 2, '--redo is refused once the read ran')
    ok(g_check('z539-1', Kz, Kz, 'Z') == 0, 'Z PASS')
    ok(REG.main(['advance', 'z539-1']) == 0 and REG.load()['K'] == Kz, 'advance: K unchanged')
    rdz = {'shortcut': True, 'complete': True, 'e1_rows': 0, 'K': 1, 'CA': 2}
    recz = SEG.load_rec('z539-1')
    ok(runD('Z', rd=rdz, rec=recz, corrected=0)[0] == 0, 'Z: one E1 statement, 0 tender rows -> PASS')
    ok(runD('Z', rd=dict(rdz, e1_rows=3), rec=recz, corrected=0)[0] == 2, 'Z: tender rows -> STOP (then gd5 --full)')
    ok(runD('Z', rd=rdz, rec=recz, corrected=4)[0] == 2, 'Z: corrected > 0 -> STOP')
    ok(runD('Z', rd=rdz, rec=recz, corrected=None)[0] == 3, 'Z without corrected -> NOT A MEASUREMENT')
    ok(runD('Z', rd=dict(rdz, complete=False), rec=recz, corrected=0)[0] == 3, 'Z: the E1 statement did not answer -> 3')
    ok(runD('Z', rd=rdz, rec=dict(recz, foreign=[{'job_id': 1, 'kind': 'x'}]), corrected=0,
            ex={'explanations': {'foreign_jobs': 'x'}})[0] == 3,
       'Z: an explained foreign job with a shortcut read -> NOT A MEASUREMENT (D.2 needs only its own jobs)')
    ok(seg_open('z539-x', 'Z', 'drain-539-x', Kz, ca_z, mpr='60') == 0, 'z539-x OPEN')
    fb.add('tender', P_plain[80], tz + 700)
    CAx = fb.cursor()
    zx_jobs = z_jobs + [job(2814, 'repair-notice-instants', tz + 650)]
    ca_x = jf('jobs_zx_ca.json', jobs_body(zx_jobs, tz + 900))
    ok(seg_close('z539-x', CAx, ca_x, '2814', ['--corrected', '0']) == 0, 'z539-x CLOSED')
    real_rp = W.REPLAY_JSON
    W.REPLAY_JSON = os.path.join(tmp, 'no_replay.json')
    n0 = len(fb.sent)
    g_read('z539-x', Kz, CAx, 'Z', ca_x, fb)
    ok(len(fb.sent) - n0 > 1 and not K.jload(os.path.join(W.SEG_DIR, 'z539-x', 'read', 'read.json')).get('shortcut'),
       'Z with E1 unproven and K != CA: no one-statement E1 shortcut -- the full all-kind read')
    W.REPLAY_JSON = real_rp
    n0 = len(fb.sent)
    g_read('z539-x', Kz, CAx, 'Z', ca_x, fb)
    ok(len(fb.sent) - n0 == 1, 'Z shortcut (E1 proven): CA != K -> ONE E1 statement')
    ok(g_check('z539-x', Kz, CAx, 'Z') == 2 and REG.load()['frozen']['seg'] == 'z539-x'
       and REG.load()['frozen']['escalate'] is None, 'Z with a tender row -> STOP; K frozen (not a gap-end STOP)')
    ok(REG.main(['status', '--no-plan-check']) == 2 and REG.main(['advance', 'z539-x']) == 2, 'status HOLD; advance refused')
    ok(seg_open('zz', 'Z', 'drain-539-y', Kz, ca_x, mpr='60') == 2, 'no writer opens while K is frozen')
    ok(seg_tick('chain', 'chain-frz', '2026-10-25', CAx, ca_x) == 2, 'the next chain check is held while K is frozen')
    ok(g_read('z539-x', Kz, CAx, 'Z', ca_x, fb, ['--full']) == 0, 'gd5 read --full over the Z range')
    exz = jf('ex_z.json', {'explanations': {'z_rows': 'a seq-only re-date of one DÖE head (539 step 1 re-stamps '
                                                      'notices; recorded in 551)'}})
    ok(g_check('z539-x', Kz, CAx, 'Z', ['--explained', exz]) == 0, 'the explained re-check -> PASS')
    ok(REG.main(['advance', 'z539-x']) == 0 and REG.load()['frozen'] is None and REG.main(['status']) == 0,
       'advance clears the freeze; READY')
    Kz2 = REG.load()['K']
    zx2_jobs = zx_jobs
    idle_z2 = jf('jobs_idle_z2.json', jobs_body(zx2_jobs, tz + 1000))
    ok(seg_open('z-ab', 'Z', 'drain-539-q', Kz2, idle_z2, mpr='60') == 0, 'z-ab OPEN')
    l1, l2 = hl_pair(Kz2, tz + 1100)
    ok(K.run_main(SEG.main, ['abort', 'z-ab', '--cb-label', l1, '--cb-label2', l2, '--reason', 'x', '--jobs',
                             jf('jobs_ab_bad.json', jobs_body(
                                 zx2_jobs + [job(2815, 'repair-notice-instants', tz + 1050, outcome='err')],
                                 tz + 1100))]) == 2 and SEG.load_rec('z-ab')['status'] == 'open',
       'abort refuses when a job ran since CB (even a failed one: close + gd5 instead)')
    ok(seg_close('z-ab', Kz2, jf('jobs_ab_ca.json', jobs_body(zx2_jobs + [job(2815, 'refold', tz + 1050)], tz + 1100)),
                 '2815', ['--corrected', '0']) == 2 and SEG.load_rec('z-ab')['status'] == 'open',
       'close refuses a typed --corrected 0 on a Z segment whose own job is not repair-notice-instants (stays open)')
    ok(seg_close('z-ab', Kz2, jf('jobs_ab_ca2.json', jobs_body(zx2_jobs + [job(2815, 'project', tz + 1050, counts=(
        '7181 notices → 497 tenders; compare (issue 495): 481 tenders verified, 8 corrected'))], tz + 1100)), '2815',
        ['--corrected', '0']) == 2, 'close refuses a typed --corrected that differs from the own counts (8)')
    l1, l2 = hl_pair(Kz2, tz + 1200)
    ok(K.run_main(SEG.main, ['abort', 'z-ab', '--cb-label', l1, '--cb-label2', l2, '--jobs',
                             jf('jobs_ab.json', jobs_body(zx2_jobs, tz + 1200)), '--reason', 'wet POST refused']) == 0
       and REG.load()['open_segment'] is None and REG.load()['K'] == Kz2 and SEG.load_rec('z-ab')['status'] == 'aborted',
       'abort: nothing written, no job since CB -> open_segment cleared, K untouched')
    ok(REG.main(['status']) == 0, '... status READY again')

    # ------------------------------------------------------------ T7 539 step 2 run a (class W)
    print('T7 539 step 2 run a (class W)')
    Kw = REG.load()['K']
    tw = tz + 2 * 3600
    clock(tw)
    idle3 = jf('jobs_idle3.json', jobs_body(zx_jobs, tw))
    ok(seg_open('w539-a', 'W', 'drain-539-a', Kw, idle3, runs='3', mpr='60') == 3,
       'class W needs --read-minutes at open (W9)')
    ok(seg_open('w539-a', 'W', 'drain-539-a', Kw, idle3, lock='Writer: 551 w539-a until 2026-10-24T17:00Z', runs='3',
                mpr='60', reads='45', extra=['--expect-new', '0', '--salt', '551-b6-drain-539-', '--bar',
                                             'U5 sequential; cap max(2Ar, Ar+10)']) == 0, 'w539-a OPEN')
    doe = [9_100_000 + k for k in range(2000)]
    for t in doe:
        fb.tenders[t] = trow(t, src='doe', seq=2, eur=2_000_000)
        fb.add('tender', t, tw + 60)
    NEWW = doe[:3]
    for k, t in enumerate(NEWW):
        fb.tenders[t] = lsa_row(t, 1_000_000 + k, src='doe', nid=47600000 + k)
    r = dict(fb.tenders[P_plain[90]])
    r['current_published_at'] = (r['current_published_at'] or 0) + 172800
    fb.tenders[P_plain[90]] = r
    fb.add('tender', P_plain[90], tw + 61)
    fb.add('organization', 96_000_000, tw + 62, cursor=fb.cursor() + 55_000)
    CAw = fb.cursor()
    w_jobs = zx_jobs + [job(2815, 'refold', tw + 10), job(2816, 'project', tw + 30)]
    ca_w = jf('jobs_w_ca.json', jobs_body(w_jobs, tw + 7200))
    ok(seg_close('w539-a', CAw, ca_w, '2815,2816') == 0, 'w539-a CLOSED')
    clock(tw + 7200)
    ok(g_read('w539-a', Kw, CAw, 'W', ca_w, fb) == 0
       and K.jload(os.path.join(W.SEG_DIR, 'w539-a', 'read', 'read.json'))['changed']['route'] == 'e1',
       'gd5 read w539-a (E1 route)')
    ok(g_check('w539-a', Kw, CAw, 'W') == 3 and not REG.load().get('frozen'),
       'class W without --runs-left is refused (a tool fault, no verdict, K not frozen)')
    ok(g_check('w539-a', Kw, CAw, 'W', ['--runs-left', '2']) == 3 and REG.load()['frozen']['seg'] == 'w539-a',
       'class W without A -> NOT A MEASUREMENT; K frozen')
    AJ.main(['build', 'w539-a', '--from-seg', 'w539-a'])
    am = K.jload(os.path.join(W.AJOBS_DIR, 'w539-a', 'manifest.json'))
    nw = am['jobs'][0]['n']
    AJ.main(['count', 'w539-a', '--summary', tf('aw.txt', dry_summary(200, nw, list(range(1, nw - 200 + 1))))])
    aw = os.path.join(W.AJOBS_DIR, 'w539-a', 'A.json')
    ok(g_check('w539-a', Kw, CAw, 'W', ['--a-from', afile, '--runs-left', '2']) == 3,
       'w539-a checked with chain-1024\'s A -> NOT A MEASUREMENT (not this segment\'s)')
    rc = g_check('w539-a', Kw, CAw, 'W', ['--a-from', aw, '--runs-left', '2', '--minutes-per-run', '60',
                                          '--read-minutes', '45', '--switchoff-gate', 'covered', '--now',
                                          '2026-10-24T12:00:00Z'])
    res = K.jload(os.path.join(W.SEG_DIR, 'w539-a', 'result.json'))
    ok(rc == 0 and res['new'] == NEWW and abs(res['cap'] - max(2 * 200 * 8 / 729, 200 * 8 / 729 + 10)) < 1e-9
       and not res['d7_hold'], f'W PASS: NEW {len(res["new"])} <= cap {res["cap"]:.2f} (A 200); D7 GO for the 2 runs left')
    popf = os.path.join(W.SEG_DIR, 'w539-a', 'new_population.json')
    pop = K.jload(popf)
    ok(pop['ids'] == NEWW and pop['ids_sha256'] == K.ids_sha(NEWW) and pop['salt'] == '551-b6-drain-539-'
       and pop['seg'] == 'w539-a', 'new_population.json in delta.py\'s shape, with seg, tag and the pre-registered salt')
    reg_w = K.jload(W.REG_FILE)
    ok(REG.main(['advance', 'w539-a']) == 0 and REG.load()['frozen'] is None,
       'advance w539-a (the PASS re-check clears its own NOT-A-MEASUREMENT freeze)')
    reg = REG.load()
    ok(all(reg['entries'][str(t)]['read'] == 'pending' and reg['entries'][str(t)]['tag'] == 'drain-539-a'
           and reg['entries'][str(t)]['read_by'] == '2026-10-25T01:40:00Z' for t in NEWW),
       'NEW enter R as drain-539-a, read pending, read by the segment\'s recorded switch-off start (01:40Z; W9)')
    ok(REG.main(['status', '--no-plan-check']) == 2, 'status HOLD: the next writer waits for the NEW reads')
    ok(REG.main(['status', '--no-plan-check', '--now', '2026-10-25T01:00:00Z', '--switchoff-by',
                 '2026-10-25T00:40:00Z']) == 10, 'an earlier --switchoff-by (00:40Z) moves the read deadline earlier -> 10')
    ok(REG.main(['status', '--no-plan-check', '--now', '2026-10-25T01:50:00Z', '--switchoff-by',
                 '2026-10-25T03:00:00Z']) == 10,
       'reads unfinished at the recorded deadline count as failed even with a later --switchoff-by (never later)')
    clock('2026-10-25T01:45:00Z')
    ok(REG.main(['status', '--no-plan-check']) == 10 and seg_open('w539-z', 'W', 'drain-539-z', CAw, ca_w, runs='1',
                                                                   mpr='60', reads='45') == 10,
       'bare status after the recorded deadline -> SWITCH OFF; segment open passes the 10 on')
    clock(tw + 7200)
    ok(seg_open('w539-b', 'W', 'drain-539-b', CAw, ca_w, runs='2', mpr='60', reads='45') == 2,
       'run b may not open while run a\'s reads are pending')
    rdirr = os.path.join(W.SEG_DIR, 'w539-a', 'reads')
    ok(DL.main(['draw', '--population', popf, '--salt', '551-b6-drain-539-', '--out', rdirr]) == 2,
       'E5: the draw refuses before the predraw record')
    ok(DL.main(['predraw', '--population', popf, '--salt', '551-b6-drain-576-', '--out', rdirr]) == 2,
       'E5: the predraw refuses a salt other than the population\'s pre-registered one')
    ok(DL.main(['predraw', '--population', popf, '--salt', '551-b6-drain-539-', '--out', rdirr]) == 0
       and DL.main(['predraw', '--population', popf, '--salt', '551-b6-drain-539-', '--out', rdirr]) == 2,
       'E5: predraw written once')
    ok(DL.main(['draw', '--population', popf, '--salt', '551-b6-other-', '--out', rdirr]) == 2,
       'E5: a salt other than the predraw\'s is refused')
    ok(DL.main(['draw', '--population', popf]) == 2, 'E5: --population/--salt/--out go together')
    ok(DL.main(['draw', '--population', popf, '--salt', '551-b6-drain-539-', '--out', rdirr]) == 0
       and K.jload(os.path.join(rdirr, 'delta_read_set.json'))['mode'] == 'all'
       and K.jload(os.path.join(rdirr, 'delta_read_set.json'))['salt'] == '551-b6-drain-539-',
       'E5: population <= 30 -> read all, salt recorded')
    bigp = jf('bigpop.json', {'ids': list(range(1, 46)), 'n': 45, 'ids_sha256': K.ids_sha(range(1, 46)),
                              'band_rows': [], 'rows': {str(i): lsa_row(i, 5) for i in range(1, 46)}})
    DL.main(['predraw', '--population', bigp, '--salt', '551-b6-drain-576-', '--out', os.path.join(tmp, 'big')])
    DL.main(['draw', '--population', bigp, '--salt', '551-b6-drain-576-', '--out', os.path.join(tmp, 'big')])
    rsb = K.jload(os.path.join(tmp, 'big', 'delta_read_set.json'))
    ok(rsb['drawn'] == K.salted(range(1, 46), '551-b6-drain-576-')[:30] and len(rsb['continuation']) == 15,
       'E5: population 45 -> the first 30 by sha256(salt + id), continuation 15')
    ok(REG.main(['reads', '--tag', 'drain-539-a', '--verdict', 'PASS', '--ref', '551 539 run a: 3 right']) == 0
       and REG.main(['status']) == 0, 'the reads PASS -> READY for run b')
    rec_w = SEG.load_rec('w539-a')
    rd_w = K.jload(os.path.join(W.SEG_DIR, 'w539-a', 'read', 'read.json'))
    rn_w = K.load_rows(os.path.join(W.SEG_DIR, 'w539-a', 'read', 'new'))
    rr_w = K.load_rows(os.path.join(W.SEG_DIR, 'w539-a', 'read', 'rpk'))
    bd_w = K.jload(os.path.join(W.SEG_DIR, 'w539-a', 'read', 'band.json'))

    def runW(**kw):
        return run(klass='W', reg_=reg_w, rec=rec_w, rd=kw.pop('rd', rd_w), rn=kw.pop('rn', rn_w),
                   rr=kw.pop('rr', rr_w), band=bd_w, **kw)

    ok(reg_w['K'] == Kw and runW(A=200)[0] == 0, 'classifier baseline (w539-a) PASS')
    ok(runW(A=0)[0] == 0, 'A 0: cap 10 holds 3 NEW')
    rnx = dict(rn_w)
    for k in range(12):
        rnx[doe[100 + k]] = lsa_row(doe[100 + k], 77 + k, src='doe')
    code, l, _a = runW(A=10, rn=rnx)
    ok(code == 2 and 'over the cap' in l, 'W: 15 NEW over the cap (A 10 -> 10.22) -> STOP before the next run')
    code, l, adv = runW(A=10, rn=rnx, ex={'explanations': {'cap': '15 NEW read: all right (551)'}})
    ok(code == 0 and len([x for x in adv['add'] if x['read'] == 'pending']) == 15,
       '... the cap explained -> PASS, every NEW still enters read-pending (W9 decides trust)')
    ok(runW(A=200, ex={'reads': {str(NEWW[0]): {'verdict': 'wrong'}}})[0] == 10, 'W: a NEW read wrong -> SWITCH OFF')
    d7 = {'runs_left': 1, 'minutes_per_run': 60, 'read_minutes': 45, 'fixed_minutes': 30, 'gate': 0}
    for at in ('2026-10-24T23:00:00Z', '2026-10-24T23:30:00Z'):
        code, l, adv = runW(A=200, d7=dict(d7, now=T(at)))
        ok(code == 0 and adv['add'] and run.last[1]['d7_hold'] and 'HOLD' in l and 'run c on 10-25' in l,
           f'D7 at {at[11:16]}Z, 1 run not yet started, NO-GO: the classification PASSes, advance populated, the '
           'advisory says HOLD -- never STOP or SWITCH OFF')
    cw = os.path.join(W.SEG_DIR, 'w539-c-jobs')
    os.makedirs(cw)
    K.jdump(os.path.join(cw, 'w539-c_ids.json'), {'run': 'w539-c', 'ids': [1, 2, 3], 'n': 3})
    ok(MW.main(['w539-c', '--summary-file', tf('wc.txt', dry_summary(3, 3)), '--jobs-dir', cw, '--runs-left', '1',
                '--minutes-per-run', '60', '--read-minutes', '45', '--switchoff-gate', 'covered', '--now',
                '2026-10-24T23:30:00Z']) == 2, 'make_wet W mode refuses run c at 23:30Z (HOLD: it moves to 10-25)')
    silw = P_plain[150]
    e = rr_w[silw]
    dw = json.loads(e['current_value_derivation_detail'])
    dw['summed'] = 'awards+'
    rrw = dict(rr_w)
    rrw[silw] = dict(e, current_value_derivation_detail=json.dumps(dw), current_seq=(e['current_seq'] or 0) + 1)
    code, l, _a = runW(A=200, rr=rrw)
    ok(code == 2 and 'announcement gap' in l, 'W: a detail move (W15) with a seq increase and no tender row -> STOP '
       '(announcement gap, W6)')
    ok(runW(A=200, rr=rrw, **LATE)[0] == 10, '... unexplained at the end of the gap in class W -> SWITCH OFF (F)')
    ok(runW(A=200, rd=with_tr(rd_w, 5316712), rr=upd(rr_w, 5316712, lsa_row(5316712, 4321)))[0] == 2,
       'W: must-stay 5316712 -> lot-sum-awards with a tender row: STOP (W5), never a read-pending NEW under the cap')
    code, l, _a = run(klass='N', reg_=reg_w, rec=rec_w, rd=rd_w, rn=rn_w, rr=rr_w, band=bd_w)
    ok(code == 2 and 'class N (expected 0)' in l, 'N: the same 3 fires in a non-W drain -> STOP (expected 0, read each)')
    # D7 in flight: an open W run that cannot finish before its latest switch-off start
    clock(tw + 7300)
    idle4 = jf('jobs_idle4.json', jobs_body(w_jobs, tw + 7300))
    ok(seg_open('w539-b', 'W', 'drain-539-b', CAw, idle4, runs='2', mpr='60', reads='45') == 0, 'w539-b OPEN')
    ok(REG.main(['status', '--no-plan-check']) == 2 and REG.main(['status', '--no-plan-check', '--remaining-minutes',
                                                                   '60']) == 2,
       'D7 in flight: the open run fits before its latest switch-off start -> HOLD (2)')
    ok(REG.main(['status', '--no-plan-check', '--remaining-minutes', '600']) == 10,
       'D7 in flight: now + 600 min + its reads is past the latest switch-off start (01:40Z) -> SWITCH OFF')
    clock('2026-10-25T01:41:00Z')
    ok(REG.main(['status', '--no-plan-check']) == 10, '... and an open run at its latest switch-off start -> SWITCH OFF')
    clock(tw + 7400)
    l1, l2 = hl_pair(CAw, tw + 7400)
    ok(K.run_main(SEG.main, ['abort', 'w539-b', '--cb-label', l1, '--cb-label2', l2, '--jobs',
                             jf('jobs_wb_ab.json', jobs_body(w_jobs, tw + 7400)), '--reason', 'test']) == 0,
       'w539-b aborted (nothing written)')
    ok(seg_open('w539-c', 'W', 'drain-539-c', CAw, idle4, runs='1', mpr='60', reads='45',
                now='2026-10-24T23:30:00Z') == 2, 'segment open refuses run c at 23:30Z (the start rule NO-GO, D7)')
    ok(REG.main(['reads', '--tag', 'drain-539-a', '--verdict', 'FAIL', '--ref', 'x']) == 2,
       'reads on a tag with nothing pending: 2')
    reg = REG.load()
    reg['entries'][str(NEWW[0])]['read'] = 'pending'
    REG.save(reg)
    ok(REG.main(['reads', '--tag', 'drain-539-a', '--verdict', 'FAIL', '--ref', 'x']) == 10
       and REG.main(['status', '--no-plan-check']) == 10, 'a failed NEW read -> SWITCH OFF; the registry is frozen with it')
    reg = REG.load()
    reg['frozen'] = None
    reg['entries'][str(NEWW[0])]['read'] = 'pass'
    REG.save(reg)
    fz = dict(REG.load(), frozen={'seg': 'p0', 'verdict': 'STOP', 'reason': 'MOVED', 'escalate': 'gap_end',
                                  'gap_end': '2026-10-25T01:40:00Z'})
    ok(REG.status(fz, T('2026-10-25T01:41:00Z'))[0] == 10 and REG.status(fz, T('2026-10-25T01:00:00Z'))[0] == 2
       and REG.status(fz, T('2026-10-25T01:00:00Z'), gap_end=T('2026-10-25T00:30:00Z'))[0] == 10,
       'a frozen gap-end STOP (MOVED) escalates to SWITCH OFF at the end of the gap (an earlier --gap-end too)')
    ok(REG.status(dict(fz, frozen=dict(fz['frozen'], escalate=None)), T('2026-10-25T03:00:00Z'))[0] == 2,
       'a frozen non-escalating STOP (an extra band row, NOT A MEASUREMENT) stays HOLD past the gap end')
    adv_seg = os.path.join(W.SEG_DIR, 'fake-adv')
    os.makedirs(adv_seg)
    K.jdump(os.path.join(adv_seg, 'segment.json'), {'seg': 'fake-adv', 'class': 'Z', 'ca_jobs': {}})
    K.jdump(os.path.join(adv_seg, 'result.json'), {'code': 0, 'K': REG.load()['K'], 'CA': REG.load()['K'], 'class': 'Z',
                                                   'tag': 'drain-1-x', 'advance': {'add': [{'id': 1, 'tag': 'drain-1-x',
                                                                                            'row': {}, 'vouches': [],
                                                                                            'vouches_from': '',
                                                                                            'read': 'pending'}],
                                                                                   'moved': [], 'lost': [], 'refresh': {}}})
    ok(K.run_main(REG.main, ['advance', 'fake-adv']) == 3, 'advance refuses pending NEW ids from a segment with no '
       'recorded switch-off instant (W9 needs the read deadline)')
    shutil.rmtree(adv_seg)
    os.makedirs(os.path.join(W.AJOBS_DIR, 'empty'))
    K.jdump(os.path.join(W.AJOBS_DIR, 'empty', 'manifest.json'), {'name': 'empty', 'n': 0, 'ids_sha256': K.ids_sha([]),
                                                                   'jobs': [], 'source': 'x'})
    ok(AJ.main(['count', 'empty']) == 0 and K.jload(os.path.join(W.AJOBS_DIR, 'empty', 'A.json'))['A'] == 0
       and K.run_main(AJ.main, ['count', 'empty', '--summary', aw]) == 3,
       'A over an empty list: count takes no summary, A = 0 complete (cap 10); a summary is refused')

    # ------------------------------------------------------------ T8 the weekly tick and gap segments
    print('T8 the weekly tick (class weekly, Sun 10-25 03:10 Berlin) and gap segments')
    wt = int(SR.berlin_at(datetime.date(2026, 10, 25), 3, 10).timestamp())
    clock(wt + 7200)
    Kwk = REG.load()['K']
    fb.add('organization', 97_000_000, wt + 600, op='added')
    CAwk = fb.cursor()
    wk_jobs = w_jobs + [job(2820 + k, kd, wt + 5 + k) for k, kd in enumerate(
        ['disk-census', 'ghost-census', 'data-quality', 'build-org-match-keys'])]
    ok(seg_tick('weekly', 'wk-x0', '2026-10-25', CAwk, jf('j_wk_early.json', jobs_body(w_jobs, wt - 600))) == 2,
       'weekly: a jobs body measured before the tick -> NOT READY (never closed early)')
    ok(seg_tick('weekly', 'wk-x1', '2026-10-25', CAwk, jf('j_wk_none.json', jobs_body(w_jobs, wt + 7200))) == 2,
       'weekly: no weekly job after the tick -> NOT READY')
    ok(seg_tick('weekly', 'wk-x2', '2026-10-25', CAwk, jf('j_wk_pre.json', jobs_body(
        wk_jobs + [job(2819, 'refold', wt - 3600)], wt + 7200))) == 2,
       'weekly: a wet job between the mark and the weekly tick -> foreign STOP')
    unset_open()
    wj = jf('jobs_weekly.json', jobs_body(wk_jobs, wt + 7200))
    ok(seg_tick('weekly', 'wk-1025', '2026-10-25', CAwk, wj) == 0, 'weekly segment READY')
    ok(K.run_main(SEG.main, ['weekly', 'wk-x', '--date', '2026-10-24', '--ca-label', 'h1a', '--ca-label2', 'h1b',
                             '--jobs', wj]) == 3, 'a weekly segment on a non-Sunday is refused')
    n0 = len(fb.sent)
    ok(g_read('wk-1025', Kwk, CAwk, 'weekly', wj, fb) == 0 and len(fb.sent) - n0 == 1,
       'weekly: one E1 statement over its range')
    ok(g_check('wk-1025', Kwk, CAwk, 'weekly') == 0, 'weekly: organization rows only, 0 tender rows -> PASS')
    ok(REG.main(['advance', 'wk-1025']) == 0, 'advance wk-1025')
    ok(runD('weekly', rd={'shortcut': True, 'complete': True, 'e1_rows': 2, 'K': 1, 'CA': 5},
            rec=SEG.load_rec('wk-1025'))[0] == 2, 'weekly with tender rows -> STOP')
    Kg = REG.load()['K']
    fb.add('organization', 97_100_000, wt + 8000, op='changed')
    CAg = fb.cursor()
    clock(wt + 8100)
    jg = jf('jobs_gap.json', jobs_body(wk_jobs + [job(2830, 'merge-provisional-orgs', wt + 7990)], wt + 8100))
    ok(seg_open('w539-b2', 'W', 'drain-539-b', CAg, jg, runs='2', mpr='60', reads='45') == 2,
       'an unattributed writer since K: open refuses (CB != K)')
    ok(seg_open('w539-b2', 'W', 'drain-539-b', Kg, jg, runs='2', mpr='60', reads='45') == 2,
       '... and with CB == K it still refuses: a wet job since the mark (B1)')
    ok(seg_gap('gap-0', CAg, jf('j_gap_cov.json', jobs_body([job(2900, 'x', wt + 7990)], wt + 8100))) == 3,
       'gap: a jobs body that no longer reaches back to the mark -> NOT A MEASUREMENT')
    ok(seg_gap('gap-1', CAg, jg, 'another agent\'s org merge 2830') == 2
       and [(j['job_id'], j['kind']) for j in SEG.load_rec('gap-1')['foreign']] == [(2830, 'merge-provisional-orgs')]
       and SEG.load_rec('gap-1')['corrected'] is None and SEG.load_rec('gap-1')['full_required'],
       'segment.py gap: STOP; the org merge 2830 recorded as foreign; corrected unknown; full read required')
    n0 = len(fb.sent)
    ok(g_read('gap-1', Kg, CAg, 'Z', jg, fb) == 0 and len(fb.sent) - n0 > 1
       and not K.jload(os.path.join(W.SEG_DIR, 'gap-1', 'read', 'read.json')).get('shortcut'),
       'gd5 read of a gap never takes the shortcut: the full read (change rows, R ∪ Rg ∪ pins, band)')
    ok(g_check('gap-1', Kg, CAg, 'Z') == 10 and REG.main(['advance', 'gap-1']) == 2,
       'gd5 check: an in-place W-input writer landed without its pre-step -> SWITCH OFF; advance refused')
    exg = jf('ex_gap.json', {'explanations': {'foreign_jobs': '2830: another agent\'s merge (551)'}})
    ok(g_check('gap-1', Kg, CAg, 'Z', ['--explained', exg]) == 10,
       '... foreign_jobs explained by free text alone: still SWITCH OFF (the C9\' pre-step is missing)')
    exg2 = jf('ex_gap2.json', {'explanations': {'foreign_jobs': '2830 (551)', 'w_input_prestep': 'C9\' prediction '
                                                'sha 1234 + paired refold (551)'}})
    ok(g_check('gap-1', Kg, CAg, 'Z', ['--explained', exg2, '--prediction', jf('pred_gap.json', {})]) == 0
       and REG.main(['advance', 'gap-1']) == 0,
       '... with the pre-step recorded and the C9\' prediction (class I rules) -> PASS; advance')
    Kg2 = REG.load()['K']
    hR = P_plain[300]
    fb.tenders[hR] = dict(fb.tenders[hR], current_value_cents=fb.tenders[hR]['current_value_cents'] + 7)
    fb.lots[98_000_000] = hR
    fb.add('lot', 98_000_000, wt + 8200)
    CAg2 = fb.cursor()
    clock(wt + 8300)
    jg2 = jf('jobs_gap2.json', jobs_body(wk_jobs + [job(2830, 'merge-provisional-orgs', wt + 7990),
                                                    job(2831, 'refold-sections', wt + 8150)], wt + 8300))
    ok(seg_gap('gap-2', CAg2, jg2) == 2, 'gap-2: a non-W-input foreign writer -> STOP')
    g_read('gap-2', Kg2, CAg2, 'Z', jg2, fb)
    ok(g_check('gap-2', Kg2, CAg2, 'Z', ['--explained', jf('ex_g2.json', {'explanations': {'foreign_jobs': 'x'}})]) == 2
       and hR in K.jload(os.path.join(W.SEG_DIR, 'gap-2', 'result.json'))['silent']
       and REG.load()['frozen']['escalate'] == 'gap_end',
       '... explained, but an R head\'s served value moved with no tender row inside the gap: SILENT STOP (a gap-end '
       'STOP: the freeze escalates)')
    g2end = SEG.load_rec('gap-2')['switchoff_by']
    ok(REG.main(['status', '--no-plan-check']) == 2 and REG.main(['status', '--no-plan-check', '--now', g2end]) == 10,
       f'registry status: HOLD before the recorded end of the gap, SWITCH OFF at it ({g2end})')
    ok(g_check('gap-2', Kg2, CAg2, 'Z', ['--explained', jf('ex_g2b.json', {'explanations': {'foreign_jobs': 'x'}}),
                                          '--now', g2end]) == 10,
       'gd5 check run at the segment\'s recorded gap end (no --gap-end given): the unexplained SILENT -> SWITCH OFF')
    ok(g_check('gap-2', Kg2, CAg2, 'Z', ['--explained', jf('ex_g3.json', {'explanations': {
        'foreign_jobs': 'x', str(hR): 'the 2831 rehome, read right'}})]) == 0 and REG.main(['advance', 'gap-2']) == 0,
       '... the SILENT move explained -> PASS; advance')
    Kg3 = REG.load()['K']
    fb.add('organization', 97_200_000, wt + 8400)
    CAg3 = fb.cursor()
    clock(wt + 8500)
    jg3 = jf('jobs_gap3.json', jobs_body(wk_jobs + [job(2830, 'merge-provisional-orgs', wt + 7990),
                                                    job(2831, 'refold-sections', wt + 8150)], wt + 8500))
    ok(seg_gap('gap-3', CAg3, jg3) == 2 and SEG.load_rec('gap-3')['foreign'][0]['kind'] == 'unattributed',
       'a gap with no job since the mark but CA > K: an unattributed writer -> STOP')
    g_read('gap-3', Kg3, CAg3, 'Z', jg3, fb)
    g_check('gap-3', Kg3, CAg3, 'Z', ['--explained', jf('ex_g4.json', {'explanations': {'foreign_jobs': 'x'}})])
    ok(REG.main(['advance', 'gap-3']) == 0, 'gap-3 explained and advanced')
    ok(seg_gap('gap-4', CAg3, jg3) == 2, 'nothing left to close')

    # ------------------------------------------------------------ T9 switchoff.py, the registry route (E4)
    print('T9 switchoff.py --registry --base-cursor (E4) on its own box')
    reg = REG.load()
    LATE_ID = 9_990_001
    fb.tenders[LATE_ID] = lsa_row(LATE_ID, 123, nid=47700000)
    fb.add('tender', LATE_ID, wt + 9000, op='added')
    N = fb.cursor()
    ok(K.run_main(SO.main, ['build', '--cursor-now', str(N)], fake=fb) == 3, 'build needs a route (registry or C1)')
    ok(K.run_main(SO.main, ['build', '--registry', W.REG_FILE, '--cursor-now', str(N)], fake=fb) == 3,
       '--registry needs --base-cursor')
    ok(K.run_main(SO.main, ['build', '--registry', W.REG_FILE, '--base-cursor', str(reg['K'] - 1), '--cursor-now',
                            str(N)], fake=fb) == 3, 'a --base-cursor that is not the registry\'s K is refused')
    K.jdump(os.path.join(W.BOX_W_DIR, 'state.json'), dict(K.jload(os.path.join(W.BOX_W_DIR, 'state.json')),
                                                          used=10_000))
    lsa_now = sorted(t for t, r in fb.tenders.items() if r['current_value_derivation'] == 'lot-sum-awards')
    fb.fail_transport = 'FROM tenders t WHERE t.id IN'
    ok(SO.main(['build', '--registry', W.REG_FILE, '--base-cursor', str(reg['K']), '--cursor-now', str(N),
                '--route', 'e1'], fake=fb) == 3, 'build: a transport failure mid-read -> 3 (a small span degrades e1 to '
       'all, never refused)')
    SO.off_box(fb).health()
    n0 = len(fb.sent)
    rc = SO.main(['build', '--registry', W.REG_FILE, '--base-cursor', str(reg['K']), '--cursor-now', str(N)], fake=fb)
    so = K.jload(os.path.join(K.U7, 'switchoff', 'switchoff.json'))
    ok(rc == 0 and so['ids'] == lsa_now and so['from_changes'] == [LATE_ID] and so['route'] == 'all'
       and fb.n('FROM changes c WHERE') == fb.n('FROM changes c WHERE') and not any(
           'FROM changes c WHERE' in s for s in fb.sent[n0:]),
       f'build resumed with the same N (the change chunks reused, 0 re-sent): every id serving lot-sum-awards '
       f'({so["n_off"]}), the late fire found through (K, N]; the W box at 10,000 used did not block it')
    vargs = ['verify', '--registry', W.REG_FILE, '--base-cursor', str(reg['K']), '--cursor-now']
    ok(SO.main(vargs + [str(N)], fake=fb, http=fb) == 2, 'verify FAILS while heads serve lot-sum-awards')
    ok(K.run_main(SO.main, vargs[:-1] + ['--cursor-now', str(N - 1)], fake=fb, http=fb) == 3,
       'verify refuses an N2 below the build\'s N')
    for t in so['ids']:
        fb.tenders[t] = dict(fb.tenders[t], current_value_derivation=None)
        fb.add('tender', t, wt + 9500)
    fb.fail_on = "current_value_derivation = 'lot-sum-awards'"
    n0 = len(fb.sent)
    rc = SO.main(vargs + [str(fb.cursor())], fake=fb, http=fb)
    ch_sent = [s for s in fb.sent[n0:] if 'FROM changes c' in s]
    ok(rc == 0 and K.jload(os.path.join(W.SWITCHOFF_BOX_DIR, 'state.json'))['flag'] and fb.api_calls
       and all(int(re.search(r'c\.cursor > (\d+)', s).group(1)) >= N for s in ch_sent),
       'verify PASSES after the drain-back, reading only (N, N2]; the band 408 is capped, the API band answers')
    W.switchoff_box(fb).health()
    st = K.jload(os.path.join(W.SWITCHOFF_BOX_DIR, 'state.json'))
    ok(st['used'] > 0 and st['capped'], 'the switch-off read through its own box (u7/switchoff/box), not u9/box')
    for t in so['ids']:                                         # restore for T10
        fb.tenders[t] = dict(fb.tenders[t], current_value_derivation='lot-sum-awards')

    # ------------------------------------------------------------ T10 day 7
    print('T10 day 7: the populations, the 539 step-2 flag, the fetched marker, the 10-28 writer hold')
    ok(D7.main(['build']) == 2, 'refused unless the last segment is the day-7 chain check')
    ok(D7.main(['build', '--allow-not-chain']) == 0, 'built (--allow-not-chain)')
    d7j = K.jload(os.path.join(W.U9, 'day7', 'day7.json'))
    ok(d7j['c8']['ids'] == sorted([97901, 8859070, 8859315, NEW_C, NEW_D]),
       f'C8 population = the chain-tagged fires {d7j["c8"]["ids"]} (no P, no delta-U8, no drain-539-a)')
    ok(d7j['chain_moved']['ids'] == [MOV] and d7j['c8']['flagged_539'] == [NEW_D] and d7j['step2_done_segment'] is None
       and d7j['step2_note'], 'chain-MOVED population = the re-decided R head; the DÖE fire flagged; step 2 incomplete')
    h1 = dict(reg_c, history=reg_c['history'] + [{'tag': 'drain-539-1', 'seg': 'z'}, {'tag': 'drain-539-a', 'seg': 'a'}])
    _c, _l, adv1 = run(reg_=h1)
    h3 = dict(reg_c, history=reg_c['history'] + [{'tag': t, 'seg': t} for t in W.STEP2_TAGS])
    _c, _l, adv3 = run(reg_=h3)
    f1 = {x['id']: x['flags'] for x in adv1['add']}
    f3 = {x['id']: x['flags'] for x in adv3['add']}
    ok(f1[NEW_D] and not f3[NEW_D] and not f1[NEW_C],
       'the 539 flag stays after step 1 and run a; it ends only when drain-539-a/b/c all advanced; non-DÖE never')
    pc8 = os.path.join(W.U9, 'day7', 'population_c8.json')
    pmv = os.path.join(W.U9, 'day7', 'population_moved.json')
    ok(DL.main(['predraw', '--population', pc8, '--salt', D7.SALT_MOVED, '--out', os.path.join(W.U9, 'day7', 'c8')])
       == 2, 'day 7: the C8 predraw refuses the chain-MOVED salt (the population records 551-b6-daily-)')
    ok(DL.main(['predraw', '--population', pc8, '--salt', D7.SALT_C8, '--out', os.path.join(W.U9, 'day7', 'c8')]) == 0
       and DL.main(['draw', '--population', pc8, '--salt', D7.SALT_C8, '--out', os.path.join(W.U9, 'day7', 'c8')]) == 0
       and K.jload(os.path.join(W.U9, 'day7', 'c8', 'delta_read_set.json'))['read'] == d7j['c8']['ids'],
       'day 7: fewer than 30 -> read all (salt 551-b6-daily-)')
    clock('2026-10-28T10:00:00Z')
    K10 = REG.load()['K']
    j28 = jf('jobs_1028.json', jobs_body(wk_jobs + [job(2830, 'merge-provisional-orgs', wt + 7990),
                                                    job(2831, 'refold-sections', wt + 8150)],
                                         int(T('2026-10-28T10:00:00Z').timestamp())))
    ok(seg_open('d7x', 'Z', 'drain-539-r', K10, j28, mpr='60', now='2026-10-28T10:00:00Z') == 2,
       '10-28 (Berlin) before the day-7 fetch is recorded: open HOLDs (B1, F)')
    fm = jf('fetch_manifest.json', {'tenders': {str(i): {'status': 200} for i in d7j['c8']['ids']}, 'tender_notices': {}})
    ok(D7.main(['fetched', '--ref', '551 day 7', '--fetch-manifest', fm]) == 2, 'fetched refuses while the chain-MOVED '
       'draw / fetch is missing')
    DL.main(['predraw', '--population', pmv, '--salt', D7.SALT_MOVED, '--out', os.path.join(W.U9, 'day7', 'moved')])
    DL.main(['draw', '--population', pmv, '--salt', D7.SALT_MOVED, '--out', os.path.join(W.U9, 'day7', 'moved')])
    ok(D7.main(['fetched', '--ref', '551 day 7', '--fetch-manifest', fm]) == 2, '... and while a drawn id is not fetched')
    fm = jf('fetch_manifest2.json', {'tenders': {str(i): {'status': 200} for i in d7j['c8']['ids'] + [MOV]},
                                     'tender_notices': {}})
    ok(D7.main(['fetched', '--ref', '551 day 7', '--fetch-manifest', fm]) == 0, 'fetched marker written')
    ok(seg_open('d7x', 'Z', 'drain-539-r', K10, j28, mpr='60', now='2026-10-27T23:30:00Z') == 0
       and SEG.load_rec('d7x')['status'] == 'open', 'after the marker the 10-28 open goes ahead (10-27 23:30Z is 10-28 '
       '00:30 in Berlin: the date is Berlin\'s)')
    l1, l2 = hl_pair(K10, int(T('2026-10-28T10:05:00Z').timestamp()))
    K.run_main(SEG.main, ['abort', 'd7x', '--cb-label', l1, '--cb-label2', l2, '--jobs',
                          jf('j28b.json', jobs_body(wk_jobs + [job(2830, 'merge-provisional-orgs', wt + 7990),
                                                               job(2831, 'refold-sections', wt + 8150)],
                                                    int(T('2026-10-28T10:05:00Z').timestamp()))), '--reason', 't'])
    with open(os.path.join(W.U9, 'day7', 'day7.json'), 'a') as fh:
        fh.write(' ')
    ok(seg_open('d7y', 'Z', 'drain-539-s', K10, j28, mpr='60', now='2026-10-28T10:00:00Z') == 2,
       'a marker that no longer matches day7.json (rebuilt): HOLD again')

    # ------------------------------------------------------------ T11 tool faults
    print('T11 tool faults')
    for args in (['check', 'nosuch', '--k', '1', '--cursor-after', '2', '--class', 'W'],
                 ['check', 'w539-a', '--k', '1', '--cursor-after', str(CAw), '--class', 'W']):
        p = subprocess.run([sys.executable, '-I', '-B', os.path.join(SB_B6, 'u9', 'gd5.py')] + args,
                           capture_output=True, text=True)
        ok(p.returncode == 3 and 'NOT A MEASUREMENT (tool fault)' in p.stdout, f'gd5 {args[1]} ... -> exit 3')
    p = subprocess.run([sys.executable, '-I', '-B', os.path.join(SB_B6, 'u9', 'registry.py'), 'advance', 'nosuch'],
                       capture_output=True, text=True)
    ok(p.returncode == 3, 'registry advance of a missing segment -> 3')
    ok(W.SWITCH_OFF == 10 and K.run_main(lambda: 10) == 10, 'SWITCH OFF is 10')

    print()
    print(f'{len(FAILS)} FAILED' if FAILS else 'ALL PASS')
    for f in FAILS:
        print('  FAILED:', f)
    return 1 if FAILS else 0


def main():
    if '--inside' in sys.argv:
        return inside()
    build_sandbox()
    p = subprocess.run([sys.executable, '-I', '-B', os.path.join(SB_B6, 'u9', 'test_w.py'), '--inside'])
    return p.returncode


if __name__ == '__main__':
    sys.exit(main())
