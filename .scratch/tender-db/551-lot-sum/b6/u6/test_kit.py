"""Issue 551 B6 U6/U7/U8 kit: offline tests (no network, no box, no write outside the scratchpad sandbox).

    python3 -I -B test_kit.py [--quick]

Builds a sandbox copy of the kit under scratchpad b6u6kit/sandbox (inputs symlinked read-only: b5, b6/u2, the port, its
pinned imports, the 545 plan), re-runs itself inside it, and drives every script end to end against a FAKE box that
answers the kit's own statement shapes from synthetic tables:
  - synthetic change rows over (958787061, cursor_now]: switch-off 2630's 3,502 version-null tender rows + 500 lot rows
    (stamp 03:34:07Z, cursor 958787062-...), 541 (126), 571 (the 66 on file), the 10-19/10-20/10-21 chains with
    rederive D5 seq-less rows (0 / 3 / 2 moved), 545's 47, 530's drain rows ending at cursor 958919213; two of the
    3,502 touched by the 10-19 chain (so L_changed ∩ 3,502 = 2);
  - synthetic notices crossing 2026-10-18T20:30Z (the ingest cross-check, 6,000 Tenders);
  - the REAL U2 fresh layer and the port of record for predict.py (full mode: all 3,502, the rates "refreshed" to a
    byte copy of the 10-17 set, so P_final must equal U2's P: 3,467, ids_sha256 a2619fd9...); --quick builds P_final
    from U2's P without the port (for iterating on the later stages);
  - a simulated drain (stage1, stage2, lc01, lc02) that the checkers must PASS, then negative cases on the pure rule
    functions (each must return its rule's verdict), make_wet's parser, the start rule and the delta draw.
"""
import bisect
import importlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
REAL_B6 = os.path.dirname(HERE)
REAL_ROOT = os.path.dirname(REAL_B6)
REAL_SCR = os.path.dirname(REAL_ROOT)                       # .scratch/tender-db
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
SB = os.path.join(SP, 'b6u6kit', 'sandbox')
SB_SCR = os.path.join(SB, 'tender-db-scratch')
SB_ROOT = os.path.join(SB_SCR, '551-lot-sum')
SB_B6 = os.path.join(SB_ROOT, 'b6')

FAILS = []


def ok(cond, what):
    print(('  ok   ' if cond else '  FAIL ') + what, flush=True)
    if not cond:
        FAILS.append(what)


# ---------------------------------------------------------------- sandbox
def build_sandbox():
    if os.path.exists(SB):
        shutil.rmtree(SB)
    os.makedirs(SB_B6)
    for d in ('b5', 'unit3', 'scripts', 'postdrain', 'b3'):
        os.symlink(os.path.join(REAL_ROOT, d), os.path.join(SB_ROOT, d))
    for d in ('545-triage', '568-r208-lots', '570-lot-keys'):
        os.symlink(os.path.join(REAL_SCR, d), os.path.join(SB_SCR, d))
    for f in ('rule_v2_g.py', 'SHA256SUMS', 'fetch_b6.py', 'l_changed_571_ids.json', 'l_changed_545_ids.json'):
        os.symlink(os.path.join(REAL_B6, f), os.path.join(SB_B6, f))
    os.symlink(os.path.join(REAL_B6, 'u2'), os.path.join(SB_B6, 'u2'))
    for u in ('u6', 'u7', 'u8'):
        os.makedirs(os.path.join(SB_B6, u))
        for f in os.listdir(os.path.join(REAL_B6, u)):
            if f.endswith('.py') or f == 'l_changed_545_whole.json':
                shutil.copyfile(os.path.join(REAL_B6, u, f), os.path.join(SB_B6, u, f))


# ---------------------------------------------------------------- the fake box
class FakeBox:
    def __init__(self):
        self.changes = []          # dicts
        self.lots = {}             # lot id -> tender id
        self.tenders = {}          # id -> row dict
        self.notice_ids = []       # sorted
        self.notices = {}          # id -> dict(ingested_at, source, parse_state, projected)
        self.tv = {}               # notice id -> [(tender_id, seq)]
        self.fail_next = False
        self.fail_on = None        # a substring: the next statement containing it gets a 408 body
        self.sent = []
        self.api_calls = []

    @staticmethod
    def body(cols, rows):
        return json.dumps({'columns': cols, 'rows': rows, 'row_count': len(rows), 'truncated': False})

    def cursor(self):
        return self.changes[-1]['cursor'] if self.changes else 958787061

    def add_change(self, kind, eid, at, vs=None, op='changed', cursor=None):
        c = cursor if cursor is not None else self.cursor() + 1
        assert c > self.cursor()
        self.changes.append({'cursor': c, 'entity_kind': kind, 'entity_id': eid, 'version_seq': vs, 'op': op,
                             'changed_at': at})
        return c

    def answer(self, sql):
        self.sent.append(sql)
        if self.fail_next or (self.fail_on and self.fail_on in sql):
            self.fail_next, self.fail_on = False, None
            return '{"error":"statement interrupted at the 15 s cap","status":408}'
        if sql.strip().upper() == 'SELECT 1':
            return self.body(['1'], [[1]])
        m = re.search(r'FROM changes c WHERE c\.cursor > (\d+) AND c\.cursor <= (\d+)', sql)
        if m:
            lo, hi = int(m.group(1)), int(m.group(2))
            cs = [r['cursor'] for r in self.changes]
            i, j = bisect.bisect_right(cs, lo), bisect.bisect_right(cs, hi)
            rows = [[r['cursor'], r['entity_kind'], r['entity_id'], r['version_seq'], r['op'], r['changed_at'],
                     self.lots.get(r['entity_id']) if r['entity_kind'] == 'lot' else None] for r in self.changes[i:j]]
            return self.body(['cursor', 'entity_kind', 'entity_id', 'version_seq', 'op', 'changed_at', 'lot_tender_id'],
                             rows)
        m = re.search(r'FROM tenders t WHERE t\.id IN \(([\d, ]+)\)', sql)
        if m:
            ids = [int(x) for x in m.group(1).split(',')]
            cols = ['id', 'source', 'current_seq', 'current_published_at', 'current_value_derivation',
                    'current_value_cents', 'current_value_currency', 'current_value_eur_cents',
                    'current_value_derivation_detail', 'projection_epoch']
            return self.body(cols, [[self.tenders[i][c] for c in cols] for i in ids if i in self.tenders])
        m = re.search(r'generate_series\((\d+), (\d+), (\d+)\)', sql)
        if m:
            lo, hi, st = (int(x) for x in m.groups())
            rows = []
            for v in range(lo, hi + 1, st):
                k = bisect.bisect_left(self.notice_ids, v)
                if k < len(self.notice_ids):
                    n = self.notice_ids[k]
                    rows.append([v, n, self.notices[n]['ingested_at']])
                else:
                    rows.append([v, None, None])
            return self.body(['probe', 'nid', 'ingested_at'], rows)
        m = re.search(r'FROM notices n WHERE n\.id > (\d+) AND n\.id <= (\d+) ORDER BY n\.id', sql)
        if m:
            lo, hi = int(m.group(1)), int(m.group(2))
            i, j = bisect.bisect_right(self.notice_ids, lo), bisect.bisect_right(self.notice_ids, hi)
            return self.body(['id', 'ingested_at'], [[n, self.notices[n]['ingested_at']] for n in self.notice_ids[i:j]])
        m = re.search(r'LEFT JOIN tender_versions v .* WHERE n\.id >= (\d+) AND n\.id < (\d+)', sql)
        if m:
            lo, hi = int(m.group(1)), int(m.group(2))
            i, j = bisect.bisect_left(self.notice_ids, lo), bisect.bisect_left(self.notice_ids, hi)
            rows = []
            for n in self.notice_ids[i:j]:
                x = self.notices[n]
                vs = self.tv.get(n) or [(None, None)]
                for t, s in vs:
                    rows.append([n, x['ingested_at'], x['source'], x['parse_state'], x['projected'], t, s])
            return self.body(['id', 'ingested_at', 'source', 'parse_state', 'projected', 'tender_id', 'seq'], rows)
        if "current_value_derivation = 'lot-sum-awards'" in sql and 'ORDER BY current_value_eur_cents DESC' in sql:
            rows = sorted(([t['id'], t['current_value_eur_cents'], t['current_value_cents'], t['current_value_currency'],
                            t['current_value_derivation_detail']] for t in self.tenders.values()
                           if t['current_value_derivation'] == 'lot-sum-awards'
                           and (t['current_value_eur_cents'] or 0) >= 10_000_000_000), key=lambda r: -r[1])[:200]
            return self.body(['id', 'current_value_eur_cents', 'current_value_cents', 'current_value_currency',
                              'current_value_derivation_detail'], rows)
        raise AssertionError(f'fake box: unknown statement shape: {sql[:200]}')

    # ---- the public API (the kit's band fallback and /health), as a CP.Http-style client
    def get(self, path):
        self.api_calls.append(path)
        if path == '/health':
            return 200, json.dumps({'rev': 'd617a78', 'checks': {'database': {'cursor': self.cursor()},
                                                                 'last_job': {'kind': 'project', 'outcome': 'ok',
                                                                              'started_at': 1, 'finished_at': 2}}}).encode()
        m = re.match(r'^/v1/tenders\?min_value=10000000000&limit=1000(?:&cursor=(\d+))?$', path)
        if not m:
            return 404, b'{"error":"not found"}'
        after = int(m.group(1) or 0)
        band = [t for t in self.tenders.values() if (t['current_value_eur_cents'] or 0) >= 10_000_000_000
                and t['id'] > after]
        band.sort(key=lambda t: t['id'])
        page = band[:7]                                   # small pages: the paging is exercised
        items = []
        for t in page:
            det = json.loads(t['current_value_derivation_detail']) if t['current_value_derivation_detail'] else {}
            vd = dict(det, method=t['current_value_derivation']) if t['current_value_derivation'] else None
            items.append({'id': t['id'], 'value': {'cents': t['current_value_cents'] or t['current_value_eur_cents'],
                                                   'currency': t['current_value_currency'] or 'EUR'},
                          'value_derivation': vd, 'published_at': '2026-01-01'})
        more = len(band) > len(page)
        return 200, json.dumps({'items': items, 'more': more, 'next_cursor': str(page[-1]['id']) if more else None,
                                'ignored_filters': []}).encode()


def trow(i, pub=1700000000, eur=None, deriv=None, cents=None, cur=None, detail=None):
    return {'id': i, 'source': 'ted', 'current_seq': 1, 'current_published_at': pub, 'current_value_derivation': deriv,
            'current_value_cents': cents, 'current_value_currency': cur, 'current_value_eur_cents': eur,
            'current_value_derivation_detail': detail, 'projection_epoch': 4}


# ---------------------------------------------------------------- inside the sandbox
def inside(quick):
    os.environ['KIT_BOX_DIR'] = os.path.join(SB, 'box')
    sys.path.insert(0, os.path.join(SB_B6, 'u6'))
    sys.path.insert(1, os.path.join(SB_B6, 'u7'))
    sys.path.insert(2, os.path.join(SB_B6, 'u8'))
    import kitlib as K
    assert K.U6.startswith(SB), K.U6
    E = importlib.import_module('enum_lchanged')
    PR = importlib.import_module('predict')
    FZ = importlib.import_module('freeze')
    SN = importlib.import_module('snapshot')
    RR = importlib.import_module('rates_refresh')
    BS = importlib.import_module('build_stages')
    MW = importlib.import_module('make_wet')
    SR = importlib.import_module('start_rule')
    D = importlib.import_module('dchecks')
    V = importlib.import_module('verify')
    DL = importlib.import_module('delta')

    print('T0 helpers')
    ok(K.ids_sha([3, 1, 2]) == K.sha_bytes(b'1\n2\n3\n'), 'ids_sha is the ascending-lines digest')
    ok(K.salted([1, 2, 3], 'x') == K.salted([3, 2, 1], 'x'), 'salted order is deterministic')
    try:
        K.plan_check("SELECT cursor FROM changes WHERE op = 'x'", 'scan', os.path.join(SB, 'probe'))
        ok(False, 'plan_check refuses a SCAN')
    except K.PlanRefused:
        ok(True, 'plan_check refuses a SCAN')
    ids3502, served = K.the_3502()
    P_u2 = K.p_u2()
    rg = K.rg_u2()
    ok(len(rg) == 35 and set(rg) | set(P_u2) == set(ids3502) and not set(rg) & set(P_u2), 'U2: P ∪ Rg = the 3,502')

    print('T1 rates refresh (A5) on a byte copy of the 10-17 zip')
    newest, _n = RR.check_zip(os.path.join(SP, 'm551', 'rates', 'eurofxref-hist.zip'))
    rc = RR.main(['--zip', os.path.join(SP, 'm551', 'rates', 'eurofxref-hist.zip'), '--expect-latest', newest,
                  '--new-root', os.path.join(SB, 'm551')])
    ok(rc == 3, 'refuses without --after-chain-fetch-rates-done')
    rc = RR.main(['--after-chain-fetch-rates-done', '--zip', os.path.join(SP, 'm551', 'rates', 'eurofxref-hist.zip'),
                  '--expect-latest', newest, '--new-root', os.path.join(SB, 'm551')])
    rates = K.jload(os.path.join(K.U6, 'rates.json'))
    ok(rc == 0 and rates['new']['sha256'] == rates['old']['sha256'], 'new set = byte copy, three sha256 recorded')
    time.sleep(1.1)         # rates dirs are named to the second
    rc2 = RR.main(['--after-chain-fetch-rates-done', '--zip', os.path.join(SP, 'm551', 'rates', 'eurofxref-hist.zip'),
                   '--expect-latest', '2026-10-15', '--new-root', os.path.join(SB, 'm551'),
                   '--out', os.path.join(SB, 'rates_dev.json')])
    ok(rc2 == 2 and os.path.exists(os.path.join(SB, 'rates_dev.json.deviation.json')),
       'a fixing newer than the store\'s is a recorded deviation (exit 2), rates.json not written')

    print('T2 synthetic box: change rows, tenders, notices')
    fb = FakeBox()
    j2630 = K.J2630_STAMP
    for k, t in enumerate(ids3502):
        fb.add_change('tender', t, j2630, cursor=958787062 + k if k == 0 else None)
        if k < 500:
            lot = 50_000_000 + k
            fb.lots[lot] = t
            fb.add_change('lot', lot, j2630)
    for k in range(10):
        fb.add_change('organization', 70_000_000 + k, j2630)
    for k in range(3):                                      # 2630 result rows (excluded with the reading, counted)
        fb.add_change('lot_result', 69_000_000 + k, j2630)
    assert fb.cursor() < K.J2630_HI
    for k in range(5):
        fb.add_change('organization', 71_000_000 + k, j2630 + 400, cursor=958809774 + k)
    s541 = [9_100_001 + k for k in range(126)]
    for k, t in enumerate(s541):
        fb.add_change('tender', t, 1792383000, cursor=958809779 + k if k == 0 else None)
    fb.lots[60_000_001] = s541[0]
    fb.add_change('lot', 60_000_001, 1792383000)
    assert fb.cursor() <= K.R541[1]
    s571 = K.jload(K.L571)
    for k, t in enumerate(s571):
        fb.add_change('tender', t, 1792386000, cursor=958809906 if k == 0 else None)
    assert fb.cursor() <= K.R571[1]
    chain19 = [9_200_001 + k for k in range(200)]
    touched = [sorted(P_u2)[10], sorted(P_u2)[20]]
    at19 = K.epoch_of('2026-10-19T07:50:00Z')
    for k, t in enumerate(chain19 + touched):
        fb.add_change('tender', t, at19, vs=2, cursor=958810038 if k == 0 else None)
    for k in range(30):
        fb.lots[61_000_000 + k] = chain19[k]
        fb.add_change('lot', 61_000_000 + k, at19, vs=2)
    # result-graph rows of the chain's appended rounds (each with its Tender's row at the same instant)
    for kind, eid in (('lot_result', 63_000_001), ('bid', 63_000_002), ('contract', 63_000_003)):
        fb.add_change(kind, eid, at19, vs=2, op='added')
    RETIRED = 9_250_001                                     # folded on 10-19, retired (merged away) on 10-20
    fb.add_change('tender', RETIRED, at19, vs=2)
    fb.lots[61_500_000] = RETIRED
    fb.add_change('lot', 61_500_000, at19, vs=2, op='added')
    fb.lots[61_600_000] = chain19[40]                       # a lot added on 10-19, swept on 10-20
    fb.add_change('lot', 61_600_000, at19, vs=2, op='added')
    s545 = K.jload(K.L545)
    at545 = K.epoch_of('2026-10-19T15:00:00Z')
    for k, t in enumerate(s545):
        fb.add_change('tender', t, at545, cursor=958848500 if k == 0 else None)
    chain20 = [9_300_001 + k for k in range(300)]
    at20 = K.epoch_of('2026-10-20T07:50:00Z')
    rd20 = K.epoch_of('2026-10-20T07:40:10Z')
    for k, t in enumerate(chain20[:3]):
        fb.add_change('tender', t, rd20, cursor=958850000 if k == 0 else None)
    for t in chain20:
        fb.add_change('tender', t, at20, vs=3)
    # 10-20: the retirement (tender removed, then its lot / results removed, rows deleted) and two orphan sweeps
    fb.add_change('tender', RETIRED, at20, op='removed')
    fb.add_change('lot', 61_500_000, at20, op='removed')
    fb.add_change('lot_result', 63_000_009, at20, op='removed')
    del fb.lots[61_500_000]
    fb.add_change('lot', 61_600_000, at20, op='removed')
    del fb.lots[61_600_000]
    fb.add_change('lot', 50_000_000, at20, op='removed')     # a lot of one of the 3,502 (2630 wrote a row for it)
    del fb.lots[50_000_000]
    s530 = [9_400_001 + k for k in range(400)]
    at530 = 1792546459
    fb.add_change('tender', s530[0], at530, cursor=958904204)
    for t in s530[1:]:
        fb.add_change('tender', t, at530)
    n_org = K.R530[1] - fb.cursor()
    for k in range(n_org - 1):
        fb.add_change('organization', 72_000_000 + k, at530)
    fb.add_change('organization', 72_999_999, at530, cursor=K.ZERO_DRIFT_CURSOR)
    chain21 = [9_500_001 + k for k in range(250)]
    rd21 = K.epoch_of('2026-10-21T07:39:30Z')
    at21 = K.epoch_of('2026-10-21T07:55:00Z')
    for t in chain21[:2]:
        fb.add_change('tender', t, rd21)
    for t in chain21:
        fb.add_change('tender', t, at21, vs=4)
    fb.lots[62_000_000] = chain21[5]
    fb.add_change('lot', 62_000_000, at21, vs=4)
    for kind, eid in (('bid', 63_100_001), ('contract', 63_100_002)):
        fb.add_change(kind, eid, at21, vs=4, op='changed')
    SWEEP_ONLY_AT = at21 + 500                              # a sweep with no fact change: no tender row at its instant
    fb.add_change('lot_result', 63_100_009, SWEEP_ONLY_AT, op='removed')
    cursor_now = fb.cursor()
    # notices: 47,406,000.. ingested 10-17 (before the cutoff); 47,410,000..47,427,999 after, 3 per Tender
    for n in range(47_406_000, 47_410_000):
        fb.notices[n] = {'ingested_at': K.epoch_of('2026-10-17T07:35:00Z') + (n - 47_406_000), 'source': 'ted',
                         'parse_state': 'parsed', 'projected': 1}
    s8 = set()
    for n in range(47_410_000, 47_428_000):
        fb.notices[n] = {'ingested_at': K.epoch_of('2026-10-19T07:35:00Z') + (n - 47_410_000), 'source': 'ted',
                         'parse_state': 'parsed', 'projected': 1}
        if n % 500 == 7:
            fb.notices[n]['parse_state'] = 'quarantined'
            continue
        t = 9_600_000 + (n - 47_410_000) // 3
        fb.tv[n] = [(t, 1)]
        s8.add(t)
    fb.notice_ids = sorted(fb.notices)
    # tenders
    for t in ids3502:
        fb.tenders[t] = trow(t, pub=served[t][2], eur=(served[t][6] or 0) // 3 or None)
    for t, (c, cur, rule) in K.PATH_V.items():
        fb.tenders[t] = trow(t, eur=c, deriv='lot-sum-vouched', cents=c, cur=cur,
                             detail=json.dumps({'summed': 'vouched', 'lots': 3, 'rule': str(rule)}))
    for t, (c, cur) in K.TRIO.items():
        fb.tenders[t] = trow(t, eur=c)
    for t in K.MUST_STAY + [K.NOT_REACHABLE]:
        fb.tenders.setdefault(t, trow(t, eur=123456))
    extra = set(s541) | set(s571) | set(chain19) | set(s545) | {8714759} | set(chain20) | set(s530) | set(chain21) | s8
    for t in extra:
        fb.tenders.setdefault(t, trow(t, eur=5_000_000))
    for t in K.pin_ids():
        fb.tenders.setdefault(t, trow(t, eur=777))
    ok(True, f'{len(fb.changes)} change rows up to cursor {cursor_now}; {len(fb.notices)} notices; {len(fb.tenders)} tenders')

    print('T3 enum_lchanged fetch + build')
    rc = E.main(['fetch-changes', '--cursor-now', str(cursor_now)], fake=fb)
    ok(rc == 0, 'fetch-changes complete')
    nchunks = len(K.jload(os.path.join(K.U6, 'lchanged', 'changes', 'manifest.json'))['chunks'])
    ok(nchunks == -(-(cursor_now - K.SINCE) // 5000), f'{nchunks} chunks of <= 5,000 cursor values')
    fb.fail_next = True
    rc = E.main(['fetch-notices', '--dir', os.path.join(SB, 'notices_fail')], fake=fb)
    st = K.jload(os.path.join(SB, 'box', 'state.json'))
    ok(rc == 3 and st['flag'], 'a 408 body is recorded, flags the box, is not retried (exit 3: not a measurement)')
    try:
        E.main(['fetch-notices'], fake=fb)
        ok(False, 'the flag refuses the next read')
    except K.BoxRefused:
        ok(True, 'the flag refuses the next read')
    ok(K.shared_box(fb).health() is not None, 'a good health clears the flag')
    rc = E.main(['fetch-notices'], fake=fb)
    nman = K.jload(os.path.join(K.U6, 'lchanged', 'notices', 'manifest.json'))
    ok(rc == 0 and nman['n0'] == 47_410_000 and nman['nmax'] == 47_427_999, f'N0 {nman["n0"]}, NMAX {nman["nmax"]}')
    rd = ['--rederive', '2026-10-19:0:2026-10-19T07:36:00Z:2026-10-19T07:40:00Z',
          '--rederive', '2026-10-20:3:2026-10-20T07:40:00Z:2026-10-20T07:41:00Z',
          '--rederive', '2026-10-21:2:2026-10-21T07:39:00Z:2026-10-21T07:40:00Z']
    rc = E.main(['build', '--cursor-now', str(cursor_now)] + rd)
    lc = K.jload(os.path.join(K.U6, 'l_changed.json'))
    want = set(s541) | set(s571) | set(chain19) | set(touched) | set(s545) | {8714759} | set(chain20) | set(s530) \
        | set(chain21) | s8
    ok(rc == 0 and not lc['stops'], f'build clean (stops {lc["stops"]})')
    ok(set(lc['l_changed']) == want, f'L_changed = the expected union ({lc["n"]} vs {len(want)})')
    ok(lc['retired'] == [RETIRED] and RETIRED not in lc['l_changed'] and lc['retired_by_source'].get('S1_tender_rows')
       == [RETIRED], f'a retired Tender is listed and kept out of L_changed ({lc["retired"]})')
    kop = lc['rows_by_kind_op']
    ok(kop.get('lot_result/added') == 1 and kop.get('bid/added') == 1 and kop.get('contract/added') == 1
       and kop.get('bid/changed') == 1 and kop.get('tender/removed') == 1 and kop.get('lot/removed') == 3,
       f'all six kinds counted by op ({kop})')
    ok(lc['unmapped_lot_rows'] == {'removed': 3, 'gone_later': 3, 'bad': 0},       # 2 added on 10-19 + 2630's row of 50000000
       f'removed lots and lots removed later are accepted ({lc["unmapped_lot_rows"]})')
    ok(lc['excluded_2630']['result_rows'] == {'lot_result': 3}, '2630\'s result rows are excluded with the reading')
    ok(any('no tender row' in x for x in lc['anomalies']) and lc['no_tender_instant_rows'] == 1,
       'a sweep-only instant is an ANOMALY, not a STOP')
    ok([d['id'] for d in lc['deviations']] == ['C1/C3 whole refolded sets of 541 and 571',
                                               'C3 "folded since" read as "first ingested since"'],
       'the 541/571 residual and the ingest reading are recorded as deviations')
    ok(lc['in_3502'] == sorted(touched), f'L_changed ∩ 3,502 = the two touched ids {lc["in_3502"]}')
    ok(lc['excluded_2630']['tender_rows'] == 3502 and lc['excluded_2630']['lot_rows'] == 500,
       '2630 exclusion: 3,502 tender + 500 lot rows')
    ok(lc['per_source']['S8_ingest']['n'] == len(s8) and lc['per_source']['S5_545']['n'] == 48
       and lc['per_source']['S3_541']['n'] == 126 and lc['per_source']['S4_571']['n'] == 66,
       f'per-source counts {lc["per_source"]}')
    ok([r['confirmed'] for r in lc['rederive']] == [True, True, True], 'rederive D5 rows confirmed on all three days')
    # negatives on the pure builder
    rows, ranges = E.load_change_dirs([os.path.join(K.U6, 'lchanged', 'changes')])
    nrows, nm = E.load_notice_dir(os.path.join(K.U6, 'lchanged', 'notices'))
    s545w = K.jload(os.path.join(K.U6, 'l_changed_545_whole.json'))['ids']
    rdp = E.parse_rederive([x for x in rd if x != '--rederive'])
    bad = [r for r in rows if r['cursor'] != 958787062]
    out = E.build(bad, ranges, nrows, nm, cursor_now, ids3502, s571, s545w, rdp)
    ok(any('2630-excluded reading' in s for s in out['stops']), 'a missing 2630 row breaks the reading: STOP')
    lot_c = next(r['cursor'] for r in rows if r['entity_kind'] == 'lot' and r['entity_id'] == 62_000_000)
    bad = [dict(r, lot_tender_id=None) if r['cursor'] == lot_c else r for r in rows]
    out = E.build(bad, ranges, nrows, nm, cursor_now, ids3502, s571, s545w, rdp)
    ok(any('map to no Tender' in s for s in out['stops']), 'a changed lot row whose lot vanished with no removed row: STOP')
    bad = rows + [{'cursor': cursor_now + 1, 'entity_kind': 'widget', 'entity_id': 1, 'version_seq': None,
                   'op': 'changed', 'changed_at': at21, 'lot_tender_id': None}]
    out = E.build(bad, [(K.SINCE, cursor_now + 1)], nrows, nm, cursor_now + 1, ids3502, s571, s545w, rdp)
    ok(any("unknown entity_kind 'widget'" in s for s in out['stops']), 'a kind outside the six: STOP')
    gone = ids3502[7]
    bad = rows + [{'cursor': cursor_now + 1, 'entity_kind': 'tender', 'entity_id': gone, 'version_seq': None,
                   'op': 'removed', 'changed_at': at21, 'lot_tender_id': None}]
    out = E.build(bad, [(K.SINCE, cursor_now + 1)], nrows, nm, cursor_now + 1, ids3502, s571, s545w, rdp)
    ok(any('of the 3,502 were RETIRED' in s for s in out['stops']), 'one of the 3,502 retired: STOP')
    # the rederive window: the trailing project shares END's second; the rederive's rows come first in cursor order
    st, en = 1000, 1060
    kr = [{'cursor': 1, 'entity_kind': 'tender', 'entity_id': 1, 'version_seq': None, 'changed_at': 1010},
          {'cursor': 2, 'entity_kind': 'tender', 'entity_id': 2, 'version_seq': None, 'changed_at': en},
          {'cursor': 3, 'entity_kind': 'tender', 'entity_id': 3, 'version_seq': None, 'changed_at': en},
          {'cursor': 4, 'entity_kind': 'tender', 'entity_id': 4, 'version_seq': 9, 'changed_at': en}]
    ids_, how, okk, _st = E.rederive_ids(kr, {'start': st, 'end': en, 'moved': 2})
    ok(okk and ids_ == {1, 2} and 'cursor order' in how, f'rederive sharing END\'s second with the project ({how})')
    ids_, how, okk, _st = E.rederive_ids(kr, {'start': st, 'end': en, 'moved': 1})
    ok(okk and ids_ == {1} and how == '[START, END)', 'rederive confirmed on [START, END)')
    ids_, how, okk, stamps = E.rederive_ids(kr, {'start': st, 'end': en, 'moved': 5})
    ok(not okk and stamps, f'a rederive count no reading matches: STOP with the stamps ({stamps})')
    out = E.build(rows, ranges, nrows, nm, cursor_now, ids3502, s571, s545w, rdp[:2])
    ok(any('2026-10-21: not given' in s for s in out['stops']), 'a missing rederive day: STOP')
    out = E.build(rows, ranges, nrows, nm, cursor_now, ids3502, s571, s545w,
                  E.parse_rederive(['2026-10-20:4:2026-10-20T07:40:00Z:2026-10-20T07:41:00Z']) + rdp[0:1] + rdp[2:3])
    ok(any('2026-10-20: 3 seq-less' in s for s in out['stops']), 'a rederive count mismatch: STOP')
    np_ = [dict(r, projected=0) if r['id'] == 47_420_000 else r for r in nrows]
    out = E.build(rows, ranges, np_, nm, cursor_now, ids3502, s571, s545w, rdp)
    ok(any('not projected' in s for s in out['stops']), 'a parsed-not-projected notice (chain not finished): STOP')
    stray = rows + [{'cursor': cursor_now + 1, 'entity_kind': 'tender', 'entity_id': 1, 'version_seq': None,
                     'op': 'changed', 'changed_at': K.J2630_STAMP, 'lot_tender_id': None}]
    out = E.build(stray, [(K.SINCE, cursor_now + 1)], nrows, nm, cursor_now + 1, ids3502, s571, s545w, rdp)
    ok(any('OUTSIDE' in s for s in out['anomalies']) and 1 in out['l_changed'],
       'a 2630-stamped row outside the recorded range is INCLUDED and flagged')

    print('T4 predict (backup, port re-run) -> P_final')
    rc = PR.main(['backup'])
    ok(rc == 0 and len(K.jload(os.path.join(K.U6, 'refresh_ids.json'))['ids']) == 2, 'backup of the 2 refetch ids')
    import datetime as _dt
    mf = PR.rates_move_from(K.jload(os.path.join(K.U6, 'rates.json')))
    ok(mf == (_dt.date.fromisoformat(newest) + _dt.timedelta(days=1)).isoformat() and mf == '2026-10-17',
       f'S_eur may move only from the old set\'s newest fixing + 1 day, read from the old zip ({mf})')
    if quick:
        ships = [dict(e, id=int(t)) for t, e in sorted(P_u2.items())]
        pf = {'what': 'TEST quick: U2 P', 'n': len(ships), 'ids_sha256': K.ids_sha(P_u2),
              'ships': [{k: e[k] for k in ('id', 'S', 'currency', 'n', 'S_eur_cents', 'vouched_by', 'vouches')}
                        for e in ships], 'Rg': rg, 'stops': [], 'partial': False, 'errors': {}}
        K.jdump(os.path.join(K.U6, 'P_final.json'), pf)
        ok(True, 'quick mode: P_final = U2 P (port not run)')
    else:
        rc = PR.main(['run', '--refreshed-after', '2026-10-19T00:00:00Z'])
        pf = K.jload(os.path.join(K.U6, 'P_final.json'))
        ok(rc == 0 and pf['n'] == 3467 and pf['ids_sha256'] == K.P_U2_IDS_SHA,
           f'port re-run on the U2 layer reproduces U2 P exactly ({pf["n"]}, {pf["ids_sha256"][:16]})')
        ok(pf['Rg_equals_u2'] and pf['same_as_u2'] == 3500 and not pf['replaced'],
           f'Rg = U2 (35); 3,500 identical, 2 refetch ids decided again with no change')
        rc = PR.main(['run', '--refreshed-after', '2026-10-21T00:00:00Z', '--ids-limit', '50',
                      '--out', os.path.join(SB, 'pf_stale.json'), '--decisions-out', os.path.join(SB, 'd.gz')])
        ok(rc == 2 or not set(lc['in_3502']) & set(ids3502[:50]),
           'a refetch id read before --refreshed-after is a STOP (or not in the first 50)')

    print('T5 freeze')
    rc = FZ.main([])
    fz = K.jload(os.path.join(K.U6, 'freeze.json'))
    u2bp = K.jload(os.path.join(K.B6, 'u2', 'u2-result.json'))
    ok(rc == 0 and not fz['stops'], f'freeze clean ({fz["stops"]})')
    ok(fz['n_B_P'] == 29 and fz['B_P_equals_u2'], f'B_P = the 29 ({fz["n_B_P"]})')
    ok(fz['Rg_equals_u2'] and fz['n_Rg'] == 35, 'Rg = U2 R_single ∪ R_excess (35)')
    pw = fz['pins_expected']['path_w']
    ok(all(pw[str(t)]['vouched_by'] for t in K.PATH_W), f'path-W pins carry the port vouched_by: '
       f'{[(t, pw[str(t)]["vouched_by"]) for t in K.PATH_W]}')
    ok(fz['L']['n'] == len(set(ids3502) | want), 'L = 3,502 ∪ L_changed')
    nb_id = next(e['id'] for e in pf['ships'] if (e['S_eur_cents'] or 0) < K.EUR100M and e['id'] not in K.PATH_W)
    nb_pf = dict(pf, ships=[dict(e, S_eur_cents=2 * K.EUR100M) if e['id'] == nb_id else e for e in pf['ships']])
    out, _L, _s = FZ.build(nb_pf, lc, ids3502)
    ok([r['id'] for r in out['band_reread']] == [nb_id] and 'new B_P head' in out['band_reread'][0]['why'],
       f'a head new to B_P (not among the 29 read) is listed for a re-read before the deploy ({nb_id})')
    bad_pf = dict(pf, ships=[e for e in pf['ships'] if e['id'] != 585917])
    bad_pf['ids_sha256'] = K.ids_sha([e['id'] for e in bad_pf['ships']])
    bad_pf['Rg'] = sorted(set(pf['Rg']) | {585917})
    out, _L, _s = FZ.build(bad_pf, lc, ids3502)
    ok(any('585917' in s for s in out['stops']), 'a path-W pin the port does not predict: STOP before deploy')

    print('T6 snapshot (pre)')
    rc = SN.main(['--ids', os.path.join(K.U6, 'snapshot_ids.json'), '--label', 'pre'], fake=fb)
    man = K.jload(os.path.join(K.U6, 'snapshot', 'pre', 'manifest.json'))
    ok(rc == 0 and man['complete'] and man['n_rows'] == fz['snapshot_ids']['n'], f'pre snapshot complete '
       f'({man["n_rows"]} rows, {len(man["chunks"])} statements of <= 2,000)')
    ok(SN.main(['--ids', os.path.join(K.U6, 'snapshot_ids.json'), '--label', 'pre'], fake=fb) == 2,
       'a complete snapshot is never overwritten')

    print('T7 stages + job JSON')
    rc = BS.main([])
    stg = K.jload(os.path.join(K.U7, 'stages.json'))
    s1 = K.jload(os.path.join(K.U7, 'jobs', 'stage1_ids.json'))['ids']
    s2 = K.jload(os.path.join(K.U7, 'jobs', 'stage2_ids.json'))['ids']
    ok(rc == 0 and stg['order'][:2] == ['stage1', 'stage2'] and all(r['n'] <= 5000 for r in stg['runs']),
       f'runs {stg["order"]} each <= 5,000')
    ok(set(K.the_61()) | set(K.pin_ids()) | {K.NOT_REACHABLE} | set(rg) <= set(s1), 'stage1 holds the 61, pins, 7511890, Rg')
    drawn = stg['stage1_salted_drawn']
    pop = sorted(set(P_u2) - (set(K.the_61()) | set(K.pin_ids()) | {K.NOT_REACHABLE} | set(rg)))
    ok(len(drawn) == 300 and drawn == K.salted(pop, '551-b6-stage1-')[:300], 'salted 300 of P (salt 551-b6-stage1-)')
    ok(set(s1) | set(s2) >= set(ids3502) and not set(s1) & set(s2), 'stage1 + stage2 cover the 3,502, disjoint')
    lcruns = [r for r in stg['order'] if r.startswith('lc')]
    lcids = set()
    for r in lcruns:
        lcids |= set(K.jload(os.path.join(K.U7, 'jobs', f'{r}_ids.json'))['ids'])
    ok(lcids == want - set(s1) - set(s2), f'lc runs {lcruns} = L_changed minus the named')
    dj = json.load(open(os.path.join(K.U7, 'jobs', 'stage1_dry.json')))
    ok(dj == {'kind': 'refold-lot-sum', 'tender_ids': s1}, 'dry JSON: kind + tender_ids (dry by default)')

    print('T8 simulated drain: every run must PASS')
    P = {e['id']: e for e in pf['ships']}

    def fire(t, use_second=False):
        e = P[t]
        vb = e['vouched_by']
        if use_second and len(e['vouches']) > 1:
            f, n, c = e['vouches'][1]
            vb = {'field': f, 'notice_id': n, 'cents': c}
        fb.tenders[t] = dict(fb.tenders[t], current_value_derivation='lot-sum-awards', current_value_cents=e['S'],
                             current_value_currency=e['currency'], current_value_eur_cents=e['S_eur_cents'],
                             current_value_derivation_detail=json.dumps({'summed': 'awards', 'lots': e['n'],
                                                                         'vouched_by': vb}))

    def summary(found, named, left=()):
        tail = ('; every named Tender is in the cohort' if not left else
                f'; {len(left)} named Tender(s) not in the cohort (no such Tender, or a head no lot sum can reach): '
                + ','.join(map(str, list(left)[:20])) + (',…' if len(left) > 20 else ''))
        return (f'refold-lot-sum (issue 551) DRY RUN — nothing written: {found} of {named} named Tender(s) whose head '
                f'could serve a derived lot sum, {found * 4} notice(s){tail}; run it with "dry_run":false,"expect":'
                f'{found} with the same tender_ids to stamp and re-queue them (a `project` is queued behind the wet run)')

    new_fire_ids = {}
    at = K.epoch_of('2026-10-21T10:30:00Z')
    NOW = '2026-10-21T10:30:00Z'
    flip_c1 = fb.cursor()                                   # C1: the cursor right after the flip deploy
    retire_lc = next(t for t in chain19 if t not in s1 and t not in s2)   # an L_changed id retired during stage 2
    s1ids = K.jload(os.path.join(K.U7, 'jobs', 'stage1_ids.json'))['ids']
    sf = os.path.join(SB, 'stage1_summary.txt')
    open(sf, 'w').write(summary(len(s1ids) - 1, len(s1ids), [K.NOT_REACHABLE]))
    rc = MW.main(['stage1', '--summary-file', sf, '--now', '2026-10-22T05:00:00Z'])
    ok(rc == 2 and not os.path.exists(os.path.join(K.U7, 'jobs', 'stage1_wet.json')),
       'make_wet enforces the start rule: NO-GO at 05:00Z -> no wet JSON (exit 2)')
    for run in stg['order']:
        ids = K.jload(os.path.join(K.U7, 'jobs', f'{run}_ids.json'))['ids']
        left = [K.NOT_REACHABLE] if run == 'stage1' else ([i for i in ids[:7]] if run.startswith('lc') else [])
        sf = os.path.join(SB, f'{run}_summary.txt')
        open(sf, 'w').write(summary(len(ids) - len(left), len(ids), left))
        rc = MW.main([run, '--summary-file', sf, '--now', NOW])
        wet = json.load(open(os.path.join(K.U7, 'jobs', f'{run}_wet.json')))
        ok(rc == 0 and wet['expect'] == len(ids) - len(left) and wet['dry_run'] is False and wet['tender_ids'] == ids,
           f'{run}: wet JSON expect {wet["expect"]}')
        before = fb.cursor()
        fires = [t for t in ids if t in P]
        newf = []
        if run.startswith('lc'):
            newf = [t for t in ids[7:] if t not in P][:5]
            for t in newf:
                fb.tenders[t] = dict(fb.tenders[t], current_value_derivation='lot-sum-awards',
                                     current_value_cents=4_000_000, current_value_currency='EUR',
                                     current_value_eur_cents=4_000_000,
                                     current_value_derivation_detail=json.dumps({'summed': 'awards', 'lots': 2,
                                                                                 'vouched_by': {'field': 'BT-161-NoticeResult',
                                                                                                'notice_id': 1, 'cents': 4_000_000}}))
            new_fire_ids[run] = newf
        for k, t in enumerate(fires):
            fire(t, use_second=(k % 7 == 3))
        for t in fires + newf:
            fb.add_change('tender', t, at)
        # result-graph rows beside the corrections, and a sweep-only row at an instant with no tender row
        fb.add_change('bid', 64_000_000 + fb.cursor() % 100000, at, op='changed')
        fb.add_change('lot_result', 64_500_000 + fb.cursor() % 100000, at + 7, op='removed')
        if run == 'stage2':                                  # a retirement inside a drain run's range
            fb.add_change('tender', retire_lc, at, op='removed')
            for lot, t in list(fb.lots.items()):
                if t == retire_lc:
                    fb.add_change('lot', lot, at, op='removed')
                    del fb.lots[lot]
            del fb.tenders[retire_lc]
        after = fb.cursor()
        rc = D.main(['read', run, '--cursor-before', str(before), '--cursor-after', str(after)], fake=fb)
        ok(rc == 0, f'{run}: reads complete')
        stamped = len(ids) - len(left)
        rc = D.main(['check', run, '--stamped', str(stamped), '--verified', str(stamped - len(fires) - len(newf)),
                     '--corrected', str(len(fires) + len(newf)), '--now', NOW])
        res = K.jload(os.path.join(K.U7, 'results', f'{run}.json'))
        ok(rc == 0 and res['verdict'] == 'PASS', f'{run}: PASS ({[(r["rule"], r["verdict"]) for r in res["rules"]]})')
        if run == 'stage2':
            ok(res['retired'] == [retire_lc], f'stage2: the Tender retired in its range is recorded ({res["retired"]})')
        if retire_lc in ids:
            man = K.jload(os.path.join(K.U7, 'post', run, 'named', 'manifest.json'))
            ok(man['complete'] and man['absent_expected'] == [retire_lc],
               f'{run}: a named id retired in an earlier run reads expected-absent, the read stays complete')

    print('T9 U8 verify + delta')
    rc = V.main(['read'], fake=fb)
    ok(rc == 0, 'u8 reads complete')
    man = K.jload(os.path.join(K.U8, 'post_all', 'manifest.json'))
    ok(man['complete'] and man['absent_expected'] == [retire_lc], 'u8: the retired L_changed id reads expected-absent')
    rc = V.main(['check'])
    vr = K.jload(os.path.join(K.U8, 'verify_result.json'))
    allnew = sorted(t for v in new_fire_ids.values() for t in v)
    popj = K.jload(os.path.join(K.U8, 'delta_population.json'))
    ok(rc == 0 and vr['verdict'] == 'PASS', f'u8 verify PASS ({[(r["rule"], r["verdict"]) for r in vr["rules"]]})')
    ok(popj['ids'] == allnew, f'delta population = the simulated new fires ({popj["n"]})')
    lcr = [r for r in stg['order'] if r.startswith('lc')]
    adm = sum(K.jload(os.path.join(K.U7, 'jobs', f'{r}_dry_result.json'))['found'] for r in lcr)
    ub = vr['u9_count_baseline']
    ok(ub['new_fires'] == len(allnew) and ub['dry_admitted'] == adm and abs(ub['rate'] - len(allnew) / adm) < 1e-12,
       f'U9 baseline (C8) = Σ new fires / Σ dry-admitted over the lc runs ({ub["new_fires"]}/{ub["dry_admitted"]})')
    ok(DL.main(['draw']) == 2, 'draw refuses before the predraw record')
    ok(DL.main(['predraw']) == 0 and DL.main(['predraw']) == 2, 'predraw written once')
    pdr = K.jload(os.path.join(K.U8, 'delta_predraw.json'))
    ok(pdr['bar_applies'] == 'population_30_or_fewer' and set(pdr['bar']) >= {
        'population_over_30', 'population_31_to_59', 'population_30_or_fewer', 'band_rows', 'neutral'},
       'the predraw record fixes the full bar (all population cases + the band rule) before the draw')
    ok(DL.main(['draw']) == 0, 'draw')
    rs = K.jload(os.path.join(K.U8, 'delta_read_set.json'))
    ok(rs['mode'] == 'all' and rs['read'] == allnew, f'population <= 30: read all ({len(rs["read"])})')
    big = os.path.join(SB, 'bigdelta')
    os.makedirs(big)
    ids_big = list(range(9_700_001, 9_700_046))
    K.jdump(os.path.join(big, 'delta_population.json'),
            {'ids': ids_big, 'n': 45, 'ids_sha256': K.ids_sha(ids_big), 'band_rows': [9_700_045],
             'rows': {str(i): trow(i, eur=(20_000_000_000 if i == 9_700_045 else 1), deriv='lot-sum-awards',
                                   cents=5, cur='EUR', detail=json.dumps({'lots': 2})) for i in ids_big}})
    DL.cmd_predraw(big)
    DL.cmd_draw(big)
    rsb = K.jload(os.path.join(big, 'delta_read_set.json'))
    ok(rsb['mode'] == 'salted-30' and rsb['drawn'] == K.salted(ids_big, '551-b6-delta-')[:30]
       and len(rsb['continuation']) == 15 and 9_700_045 in rsb['read'],
       'population 45: first 30 by sha256(551-b6-delta-+id), continuation 15, the band row read in full')

    # V4 / V5 with reads and explanations already recorded in 551 (--explained), on a copy of the U8 reads
    import types
    u8x = os.path.join(SB, 'u8x')
    shutil.copytree(os.path.join(K.U8, 'post_all'), os.path.join(u8x, 'post_all'))
    bf = K.jload(os.path.join(K.U8, 'band_final.json'))
    xid = 42_000_042
    K.jdump(os.path.join(u8x, 'band_final.json'), dict(bf, rows=bf['rows'] + [[xid, 5 * 10**10, 5 * 10**10, 'EUR', None]]))
    ctxx = D.Ctx()
    ctxx.reg = dict(ctxx.reg, runs=ctxx.reg['runs'] + [{'run': 'lc99', 'verdict': 'STOP'}])
    rc = V.cmd_check(types.SimpleNamespace(explained=None), ctx=ctxx, out=u8x)
    ok(rc == D.STOP, 'V4: an extra band row shows STOP until read; V5: an unexplained STOP run -> STOP')
    ex = os.path.join(SB, 'explained.json')
    K.jdump(ex, {'band_reads': {str(xid): {'verdict': 'right', 'ref': 'issue 551 U8 read'}},
                 'run_explanations': {'lc99': 'explained in 551'}})
    ctxx = D.Ctx()
    ctxx.reg = dict(ctxx.reg, runs=ctxx.reg['runs'] + [{'run': 'lc99', 'verdict': 'STOP'}])
    ok(V.cmd_check(types.SimpleNamespace(explained=ex), ctx=ctxx, out=u8x) == D.PASS,
       '--explained: a band row read right and an explained STOP run turn into notes -> PASS')
    K.jdump(ex, {'band_reads': {str(xid): {'verdict': 'wrong', 'ref': 'issue 551 U8 read'}}})
    ctxx = D.Ctx()
    ok(V.cmd_check(types.SimpleNamespace(explained=ex), ctx=ctxx, out=u8x) == D.SWITCH_OFF,
       '--explained: a band row read wrong -> SWITCH OFF')

    print('T10 negative cases on the rules')
    ctx = D.Ctx()
    s1set = set(s1)
    post = K.load_rows(os.path.join(K.U7, 'post', 'stage1', 'named'))
    pins_post = K.load_rows(os.path.join(K.U7, 'post', 'stage1', 'pins'))
    p1 = sorted(s1set & set(P))
    ok(D.d1(ctx, 'stage1', s1, post).code == D.PASS, 'D1 baseline PASS')
    t = next(i for i in s1 if i in ids3502 and i not in P)
    x = dict(post)
    x[t] = dict(post[t], current_value_derivation='lot-sum-awards')
    ok(D.d1(ctx, 'stage1', s1, x).code == D.SWITCH_OFF, 'D1 unpredicted fire -> SWITCH OFF (and Rg moved)')
    x = dict(post)
    x[p1[0]] = dict(post[p1[0]], current_value_cents=post[p1[0]]['current_value_cents'] + 1)
    ok(D.d1(ctx, 'stage1', s1, x).code == D.SWITCH_OFF, 'D1 wrong S -> SWITCH OFF')
    x = dict(post)
    det = json.loads(post[p1[0]]['current_value_derivation_detail'])
    det['vouched_by'] = {'field': 'TED-VAL_TOTAL', 'notice_id': 1, 'cents': 1}
    x[p1[0]] = dict(post[p1[0]], current_value_derivation_detail=json.dumps(det))
    ok(D.d1(ctx, 'stage1', s1, x).code == D.SWITCH_OFF, 'D1 vouched_by not among the port\'s vouches -> SWITCH OFF')
    x = dict(post)
    x[p1[1]] = dict(ctx.pre[p1[1]])
    ok(D.d1(ctx, 'stage1', s1, x).code == D.STOP, 'D1 predicted id not firing -> STOP (explain within the gap)')
    rgi = sorted(s1set & set(rg))[0]
    x = dict(post)
    x[rgi] = dict(post[rgi], current_value_eur_cents=(post[rgi]['current_value_eur_cents'] or 0) + 1)
    ok(D.d1(ctx, 'stage1', s1, x).code == D.SWITCH_OFF, 'D1 an Rg id moved -> SWITCH OFF')
    ch = set(p1)
    ok(D.d2(ctx, 'stage1', s1, post, len(p1), len(s1) - 1 - len(p1), len(s1) - 1, 0, len(s1) - 1, ch).code == D.PASS,
       'D2 baseline PASS')
    ok(D.d2(ctx, 'stage1', s1, post, len(p1) + 1, None, None, 0, None, ch).code == D.SWITCH_OFF,
       'D2 corrected off by one, no expansion -> SWITCH OFF')
    ok(D.d2(ctx, 'stage1', s1, post, len(p1) + 1, None, None, 0, None, ch | {123}).code == D.PASS,
       'D2 surplus = the touched expansion -> PASS (judged by D5/D6)')
    # D1 + D2 together on a predicted id that does not fire (a 179732-type parity miss): the run reads STOP, not
    # SWITCH OFF -- D2 defers the non-fire to D1's explain-within-the-gap
    x = dict(post)
    x[p1[1]] = dict(ctx.pre[p1[1]])
    o1 = D.d1(ctx, 'stage1', s1, x)
    o2 = D.d2(ctx, 'stage1', s1, x, len(p1) - 1, len(s1) - 1 - (len(p1) - 1), len(s1) - 1, 0, len(s1) - 1,
              ch - {p1[1]})
    ok(o1.code == D.STOP and o2.code == D.STOP and D.worse(o1.code, o2.code) == D.STOP,
       f'D1 + D2 on a non-fire: STOP + STOP -> run STOP ({o2.lines[:2]})')
    o2 = D.d2(ctx, 'stage1', s1, x, len(p1), None, None, 0, None, ch - {p1[1]})
    ok(o2.code == D.SWITCH_OFF, 'D2: a non-fire plus a correction nothing explains -> SWITCH OFF (a surplus)')
    ok(D.d2(ctx, 'stage1', s1, post, len(p1), 5, len(s1) - 1, 0, len(s1) - 1, ch).code == D.STOP,
       'D2 verified + corrected != stamped -> STOP')
    ok(D.d2(ctx, 'stage1', s1, post, len(p1), None, None, 0, None, ch | {rgi}).code == D.SWITCH_OFF,
       'D2 a named non-P id with a change row (lot-only drift) -> SWITCH OFF')
    pin_out = next(i for i in s1 if i not in ids3502 and i in K.MUST_STAY)
    x = dict(post)
    x[pin_out] = dict(post[pin_out], current_published_at=(post[pin_out]['current_published_at'] or 0) + 86400)
    ok(D.d2(ctx, 'stage1', s1, x, len(p1) + 1, None, None, 0, None, ch | {pin_out}).code == D.STOP,
       'D2 a pin outside the 3,502 re-dated only (non-W drift) -> STOP-explain, not SWITCH OFF')
    x[pin_out] = dict(post[pin_out], current_value_eur_cents=(post[pin_out]['current_value_eur_cents'] or 0) + 1)
    ok(D.d2(ctx, 'stage1', s1, x, len(p1) + 1, None, None, 0, None, ch | {pin_out}).code == D.SWITCH_OFF,
       'D2 a pin outside the 3,502 whose value moved -> SWITCH OFF')
    ok(D.d3(ctx, pins_post).code == D.PASS, 'D3 baseline PASS')
    for t, mut, what in [(8426318, {'current_value_cents': 200000000000}, 'path-W pin cents'),
                         (369057, {'current_value_cents': 1}, 'path-V pin moved'),
                         (6450467, {'current_value_derivation': 'lot-sum-awards'}, 'trio derived'),
                         (K.MUST_STAY[0], {'current_value_eur_cents': 1}, 'must-stay moved'),
                         (6465510, {'current_value_derivation': 'lot-sum-awards'}, '6465510 lot-sum-awards')]:
        x = dict(pins_post)
        x[t] = dict(pins_post[t], **mut)
        ok(D.d3(ctx, x).code == D.SWITCH_OFF, f'D3 {what} -> SWITCH OFF')
    band = K.jload(os.path.join(K.U8, 'band_final.json'))
    drained = sorted(ctx.ids3502)
    ok(D.d4(ctx, band, drained).code == D.PASS, 'D4 baseline PASS (band == B_P)')
    bp = fz['B_P']
    farb = [i for i in bp if i not in fz['near_eur100m_2pct']][0]
    b2 = dict(band, rows=[r for r in band['rows'] if r[0] != farb])
    ok(D.d4(ctx, b2, drained).code == D.SWITCH_OFF, 'D4 a B_P head missing -> SWITCH OFF')
    nbs = [i for i in fz['near_eur100m_2pct'] if i in bp]
    if nbs:
        nb = nbs[0]
        b2 = dict(band, rows=[r for r in band['rows'] if r[0] != nb])
        ok(D.d4(ctx, b2, drained).code == D.STOP, f'D4 a near-100m head ({nb}) missing -> STOP-explain')
    b2 = dict(band, rows=band['rows'] + [[42, 50_000_000_000, 50_000_000_000, 'EUR', None]])
    ok(D.d4(ctx, b2, drained).code == D.STOP, 'D4 an extra row -> STOP and READ')
    b2 = dict(band, rows=band['rows'] + [[43, 10_100_000_000, 10_100_000_000, 'EUR', None]])
    ok(D.d4(ctx, b2, drained).code == D.STOP, 'D4 an extra row within 2 % -> STOP-explain')
    b2 = dict(band, rows=[[i, 2 * 10**10, 1, 'EUR', None] for i in range(200)])
    ok(D.d4(ctx, b2, drained).code == D.NOT_MEASURED, 'D4 a LIMIT-capped seek -> NOT A MEASUREMENT')
    apib = K.band_api(fb)
    ok(apib['route'] == 'api' and sorted(r[0] for r in apib['rows']) == sorted(r[0] for r in band['rows'])
       and apib['pages'] > 1, f'the API band route finds the same rows ({len(apib["rows"])}, {apib["pages"]} pages)')
    ok(D.d4(ctx, apib, drained).code == D.PASS, 'D4 on the API route: PASS (no LIMIT cap)')
    b2 = dict(apib, rows=apib['rows'] + [[44, None, 9 * 10**10, 'PLN', None]])
    ok(D.d4(ctx, b2, drained).code == D.STOP, 'D4 on the API route: an extra non-EUR row (eur unknown) -> STOP and READ')
    b2 = dict(band, rows=band['rows'] + [[42, 50_000_000_000, 50_000_000_000, 'EUR', None]])
    ok(D.d4(ctx, b2, drained, reads={'42': {'verdict': 'neutral'}}).code == D.PASS
       and D.d4(ctx, b2, drained, reads={'42': {'verdict': 'wrong'}}).code == D.SWITCH_OFF,
       'D4 with a recorded read: neutral -> note, wrong -> SWITCH OFF')
    crow2 = [{'cursor': 1, 'entity_kind': 'tender', 'entity_id': 77, 'version_seq': None, 'op': 'removed',
              'changed_at': 5, 'lot_tender_id': None},
             {'cursor': 2, 'entity_kind': 'lot', 'entity_id': 78, 'version_seq': None, 'op': 'removed',
              'changed_at': 5, 'lot_tender_id': None},
             {'cursor': 3, 'entity_kind': 'bid', 'entity_id': 79, 'version_seq': 2, 'op': 'added',
              'changed_at': 6, 'lot_tender_id': None},
             {'cursor': 4, 'entity_kind': 'tender', 'entity_id': 80, 'version_seq': 2, 'op': 'changed',
              'changed_at': 6, 'lot_tender_id': None}]
    o, _c, _e, ret = D.d5(ctx, crow2, {80: trow(80)}, [], False)
    ok(o.code == D.PASS and ret == {77} and _c == {80}, 'D5: a retirement, a removed lot and a bid row read cleanly')
    o, _c, _e, ret = D.d5(ctx, crow2, {80: trow(80)}, [77], False)
    ok(o.code == D.STOP, 'D5: a named id retired in the range -> STOP-explain')
    o, *_ = D.d5(ctx, crow2 + [{'cursor': 5, 'entity_kind': 'lot', 'entity_id': 81, 'version_seq': 2, 'op': 'changed',
                                'changed_at': 6, 'lot_tender_id': None}], {80: trow(80)}, [], False)
    ok(o.code == D.STOP, 'D5: a changed lot whose lot vanished with no removed row -> STOP')
    lc1x = K.jload(os.path.join(K.U7, 'jobs', 'lc01_ids.json'))['ids']
    pl = K.load_rows(os.path.join(K.U7, 'post', 'lc01', 'named'))
    tv = next(i for i in lc1x if i in pl and not D.is_lsa(pl[i]))
    o = D.Out('D6')
    xx = dict(pl)
    xx[tv] = dict(pl[tv], current_published_at=(pl[tv]['current_published_at'] or 0) + 86400)
    lcnew = {i: r for i, r in pl.items() if D.is_lsa(r)}      # lc01's new fires (W's own moves)
    D.lc_other_changes(o, ctx, lc1x, xx, lcnew)
    ok(o.code == D.PASS and any('seq/published_at' in ln for ln in o.lines), 'lc run: a date-only move is a note')
    xx[tv] = dict(pl[tv], current_value_eur_cents=(pl[tv]['current_value_eur_cents'] or 1) * 50)
    o = D.Out('D6')
    D.lc_other_changes(o, ctx, lc1x, xx, lcnew)
    ok(o.code == D.STOP, 'lc run: a value-column move without a new fire -> STOP-explain (non-W drift)')
    ctx7 = D.Ctx()
    ctx7.reg = {'runs': [], 'drained': [], 'fired': {}, 'retired': []}
    import datetime as _dt2
    utc = _dt2.timezone.utc
    ok(D.d7(ctx7, 'stage1', _dt2.datetime(2026, 10, 21, 10, 30, tzinfo=utc)).code == D.PASS, 'D7 at 10:30Z: PASS')
    ok(D.d7(ctx7, 'stage1', _dt2.datetime(2026, 10, 22, 4, 0, tzinfo=utc)).code == D.STOP,
       'D7: the 2 h margin gone but a switch-off still fits -> STOP')
    ok(D.d7(ctx7, 'stage1', _dt2.datetime(2026, 10, 22, 5, 20, tzinfo=utc)).code == D.SWITCH_OFF,
       'D7: the rest cannot finish before the latest switch-off start -> SWITCH OFF')
    crow = [{'cursor': 1, 'entity_kind': 'tender', 'entity_id': 5, 'version_seq': None, 'op': 'changed',
             'changed_at': 0, 'lot_tender_id': None}]
    o, _c, _e, _r = D.d5(ctx, crow, {5: trow(5, deriv='lot-sum-awards')}, [], False)
    ok(o.code == D.SWITCH_OFF, 'D5 lot-sum-awards outside P ∪ L_changed -> SWITCH OFF')
    o, _c, _e, _r = D.d5(ctx, crow, {}, [], False)
    ok(o.code == D.NOT_MEASURED, 'D5 a changed Tender not read -> NOT A MEASUREMENT')
    lc1 = K.jload(os.path.join(K.U7, 'jobs', 'lc01_ids.json'))['ids']
    many = [i for i in lc1 if i not in P][:200]
    postlc = {i: trow(i, deriv='lot-sum-awards', eur=1) for i in many}
    o, _n = D.d6(ctx, 'lc01', lc1, postlc, {}, [], 5000, {})
    ok(o.code == D.STOP, f'D6 200 new fires on 5,000 admitted (cap {max(2 * 5000 * 3502 / 251526, 5000 * 3502 / 251526 + 10):.1f}) -> STOP')
    o, _n = D.d6(ctx, 'lc01', lc1, {many[0]: trow(many[0], deriv='lot-sum-awards', eur=2 * 10**10)}, {}, [], 5000, {})
    ok(o.code == D.STOP, 'D6 a delta band row -> STOP and READ')

    print('T11 make_wet parser, start rule')
    p = MW.parse(summary(3500, 3502, [K.NOT_REACHABLE, 5]))
    ok(p['found'] == 3500 and p['not_admitted_shown'] == [K.NOT_REACHABLE, 5], 'parses the not-admitted clause')
    p = MW.parse(summary(10, 30, list(range(1, 21)) + [99]))
    ok(p['not_admitted_truncated'] and len(p['not_admitted_shown']) == 20, 'parses a truncated (20 shown) list')
    sf = os.path.join(SB, 'bad_summary.txt')
    open(sf, 'w').write(summary(len(s1) - 2, len(s1), [K.NOT_REACHABLE, sorted(set(s1) & set(ids3502))[0]]))
    ok(MW.main(['stage1', '--summary-file', sf]) == 2, 'a 3,502 id not admitted -> STOP')
    open(sf, 'w').write(summary(len(s1), len(s1) + 1))
    ok(MW.main(['stage1', '--summary-file', sf]) == 2, 'a dry that named a different list -> STOP')
    ok(SR.main(['--runs-left', '4', '--now', '2026-10-21T10:00:00Z']) == 0, 'start rule: 10:00Z, 4 runs -> GO')
    ok(SR.main(['--runs-left', '4', '--now', '2026-10-22T04:00:00Z']) == 2, 'start rule: 04:00Z -> NO-GO')

    print('T12 switch-off route: id list, drain-back, verify')
    SO = importlib.import_module('switchoff')
    ok(K.run_main(SO.main, ['build'], fake=fb) == 2, 'build refuses without --flip-cursor / --cursor-now (point 11 / C7)')
    lsa_now = sorted(t for t, r in fb.tenders.items() if r['current_value_derivation'] == 'lot-sum-awards')
    rgx = sorted(rg)[0]                                       # an unpredicted Rg fire no completed check recorded
    fb.tenders[rgx] = dict(fb.tenders[rgx], current_value_derivation='lot-sum-awards')
    late = 9_800_001                                          # a fire after the flip outside every list (in flight)
    fb.tenders[late] = trow(late, deriv='lot-sum-awards', cents=1, cur='EUR', eur=1, detail='{"lots":2}')
    fb.add_change('tender', late, K.epoch_of('2026-10-22T07:50:00Z'), vs=2)
    rc = SO.main(['build', '--flip-cursor', str(flip_c1), '--cursor-now', str(fb.cursor())], fake=fb)
    so = K.jload(os.path.join(K.U7, 'switchoff', 'switchoff.json'))
    ok(rc == 0 and so['ids'] == sorted(lsa_now + [rgx, late]) and so['from_changes'] == [late],
       f'switch-off list = every id serving lot-sum-awards: the 3,502 (an Rg fire too), L_changed, fires, pins, and the '
       f'change rows since C1 (the late fire) ({so["n_off"]})')
    ok(retire_lc in so['retired'], 'the switch-off reads treat the retired L_changed id as expected-absent')
    vargs = ['verify', '--flip-cursor', str(flip_c1), '--cursor-now', str(fb.cursor())]
    ok(SO.main(vargs, fake=fb) == 2, 'verify FAILS while heads still serve lot-sum-awards')
    ok(K.run_main(SO.main, ['verify'], fake=fb) == 2, 'verify refuses without --flip-cursor / --cursor-now')
    for t in so['ids']:
        fb.tenders[t] = dict(ctx.pre.get(t) or trow(t, eur=1))
        fb.add_change('tender', t, K.epoch_of('2026-10-22T08:10:00Z'))      # the drain-back's correction rows
    fb.fail_on = "current_value_derivation = 'lot-sum-awards'"              # the band statement 408s
    rc = SO.main(['verify', '--flip-cursor', str(flip_c1), '--cursor-now', str(fb.cursor())], fake=fb, http=fb)
    ok(rc == 0 and K.jload(os.path.join(K.BOX_DIR, 'state.json'))['flag'],
       'verify PASSES after the drain-back; the /v1/sql band 408 is not retried, the API band answers instead')
    ok(K.shared_box(fb).health() is not None, 'box_health clears the flag the band statement set')
    ok(SO.main(['verify', '--flip-cursor', str(flip_c1), '--cursor-now', str(fb.cursor()), '--band-route', 'api'],
               fake=fb, http=fb) == 0, 'verify --band-route api: PASS')

    print('T13 tool faults, the post-deploy classifier, /health, the probe copy')
    ok(K.run_main(lambda: (_ for _ in ()).throw(SystemExit('P_final.json is not the file freeze.json froze'))) == 3,
       'a SystemExit carrying a message maps to 3 (NOT A MEASUREMENT), never to a verdict')
    ok(K.run_main(lambda: (_ for _ in ()).throw(K.BoxRefused('budget'))) == 3, 'an uncaught exception maps to 3')
    ok(D.SWITCH_OFF == 10 and K.run_main(lambda: 10) == 10, 'SWITCH OFF is 10, a code Python never produces itself')
    p = subprocess.run([sys.executable, '-I', '-B', os.path.join(SB_B6, 'u7', 'dchecks.py'), 'check', 'nosuchrun',
                        '--corrected', '0'], capture_output=True, text=True, env=dict(os.environ))
    ok(p.returncode == 3 and 'NOT A MEASUREMENT (tool fault)' in p.stdout,
       f'a checker crash (missing run file) exits 3, not 1 (rc {p.returncode})')
    c0 = fb.cursor()
    fb.add_change('organization', 79_000_001, K.epoch_of('2026-10-22T09:00:00Z'))
    rc = E.main(['fetch-changes', '--from', str(c0), '--cursor-now', str(fb.cursor()), '--dir',
                 os.path.join(SB, 'postdeploy_ok')], fake=fb)
    ok(rc == 0 and E.main(['classify', '--dir', os.path.join(SB, 'postdeploy_ok')]) == 0,
       'classify: organization rows only -> PASS (exit 0)')
    c0 = fb.cursor()
    fb.add_change('tender', 1, K.epoch_of('2026-10-22T09:01:00Z'))
    E.main(['fetch-changes', '--from', str(c0), '--cursor-now', str(fb.cursor()), '--dir',
            os.path.join(SB, 'postdeploy_bad')], fake=fb)
    ok(E.main(['classify', '--dir', os.path.join(SB, 'postdeploy_bad')]) == 2, 'classify: a tender row -> STOP (exit 2)')
    H = importlib.import_module('health')
    ok(H.main(['--label', 'C0'], http=fb) == 0 and 'cursor=' in open(H.LOG).read().splitlines()[-1],
       '/health read through the shared GET client and logged')
    ok(K.ensure_plan_probe() == K.PLAN_PROBE and os.path.exists(K.PLAN_PROBE_SHA)
       and open(K.PLAN_PROBE_SHA).read().split()[0] == K.sha_file(K.PLAN_PROBE),
       'plan-probe runs from the kit copy, sha256 recorded (a gate\'s cargo clean cannot remove it)')

    print()
    print(f'{len(FAILS)} FAILED' if FAILS else 'ALL PASS')
    for f in FAILS:
        print('  FAILED:', f)
    return 1 if FAILS else 0


def main():
    quick = '--quick' in sys.argv
    if '--inside' in sys.argv:
        return inside(quick)
    build_sandbox()
    me = os.path.join(SB_B6, 'u6', 'test_kit.py')
    p = subprocess.run([sys.executable, '-I', '-B', me, '--inside'] + (['--quick'] if quick else []))
    return p.returncode


if __name__ == '__main__':
    sys.exit(main())
