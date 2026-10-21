"""Issue 551 B6 U6/U7/U8 kit: shared plumbing (paths, records, the bounded box-read helper, the local plan probe).

Every box read goes through `Box.sq()`:
  - the statement is plan-probed LOCALLY first (turso's own planner: a kit COPY of target/debug/plan-probe, sha256
    recorded, so a gate's `cargo clean` cannot remove it; run on a copy of the v551 plan.db; /v1/sql refuses EXPLAIN)
    and refused if any step SCANs a base table (a CTE or generate_series may be scanned: their rows are the
    statement's own literals);
  - one bare SELECT through `ssh root@zebreus.click /root/sq.sh` (the token stays on the box), paced >= 2.5 s apart;
  - an answer counts only when the body is a JSON object with `rows` AND `columns` (prod-box-reads.md: a shed or
    408 body is never read as an empty result); anything else is recorded, sets a flag, and is NEVER retried; while the
    flag is set only `SELECT 1` (health) may run, and a good health clears it;
  - a hard budget (state file box/state.json) and a rolling one-hour count (the endpoint's 300/h per token).
Every raw body is saved; every request is logged to box/queries.log.
Also here: classify_changes (the six change-row kinds, removed lots, retired Tenders), pk_read's expected-absent ids,
the band reader (the /v1/sql seek, falling back to the public API band -- never a retry of the statement), and
run_main, which maps any tool fault to exit 3 (NOT A MEASUREMENT) so a crash can never read as a verdict.

Offline tests inject a fake transport (`Box(transport=callable)`), which never touches the network.
"""
import collections
import datetime
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True

U6 = os.path.dirname(os.path.abspath(__file__))                       # .../551-lot-sum/b6/u6
B6 = os.path.dirname(U6)
ROOT = os.path.dirname(B6)                                             # .../551-lot-sum
U7 = os.path.join(B6, 'u7')
U8 = os.path.join(B6, 'u8')
REPO = os.environ.get('KIT_REPO', '/home/user/tender-db')
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
PLAN_DB_SRC = os.path.join(SP, 'v551', 'plan.db')
PLAN_DIR = os.path.join(SP, 'b6u6kit')
PLAN_DB = os.path.join(PLAN_DIR, 'plan.db')                            # a COPY; the source is never opened
# The probe binary is COPIED out of target/ once (with its sha256) and always run from the copy: ops/check.sh's
# auto `cargo clean` (a re-gate of the switch-off twin mid-gap) would otherwise delete it under the kit.
PLAN_PROBE_SRC = os.path.join(REPO, 'target', 'debug', 'plan-probe')
PLAN_PROBE = os.environ.get('KIT_PLAN_PROBE') or os.path.join(PLAN_DIR, 'plan-probe')
PLAN_PROBE_SHA = os.path.join(PLAN_DIR, 'plan-probe.sha256')
SSH = ['ssh', '-o', 'ConnectTimeout=15', '-o', 'BatchMode=yes', 'root@zebreus.click', '/root/sq.sh']

SERVED_LSA = os.path.join(ROOT, 'b5', 'investigate', 'served_lsa_rows.json')
READ_ITEMS = os.path.join(ROOT, 'b5', 'investigate', 'read_items.json')
P_U2 = os.path.join(B6, 'u2', 'P.json')
R_SINGLE = os.path.join(B6, 'u2', 'R_single.json')
R_EXCESS = os.path.join(B6, 'u2', 'R_excess.json')
L571 = os.path.join(B6, 'l_changed_571_ids.json')
L545 = os.path.join(B6, 'l_changed_545_ids.json')
PLAN545 = os.path.join(ROOT, '..', '545-triage', 'u3', 'rehoming_plan_2650.json')
RULE = os.path.join(B6, 'rule_v2_g.py')
RULE_SHA = '23cbd8508a3afa35b195f8a2ee91a75bb3b8f08aa2e2f3f9abbd0ea80454452f'
P_U2_IDS_SHA = 'a2619fd9e380d3d5672b3417d807e504b3a85965b8e5ea319425564e3cd21872'
R_SINGLE_IDS_SHA_N = 34
EXCLUDED_179732 = 179732

# ---- cursor / time constants from issue 551's records (each a recorded fact, not a guess)
SINCE = 958787061                       # end of project 2627 (B5's corpus drain)
J2630_LO, J2630_HI = 958787062, 958809773   # switch-off drain-back 2630's rows (inclusive), version-null, one stamp
J2630_STAMP = 1792380847                # 2026-10-19T03:34:07Z
R541 = (958809778, 958809905)           # (lo, hi]: 541's project 2632 (126 corrected; 2631 stamped 753, folded 820)
R571 = (958809905, 958810037)           # (lo, hi]: 571's project 2636 (66 corrected of 67 written)
R530 = (958904203, 958919213)           # (lo, hi]: 530's stock drain 2672/2673 (+ project 2674)
ZERO_DRIFT_CURSOR = 958919213           # zero-drift re-run 2677-2679 on d617a78: cursor unchanged
CUTOFF_ISO = '2026-10-18T20:30:00Z'
CUTOFF = 1792355400                     # notices first ingested at/after this instant (the ingest cross-check)
EUR100M = 10_000_000_000                # cents
CORPUS_RATE = (3502, 251526)            # D6's projection rate: corpus fires / corpus dry count (2623)
SWITCHOFF_REV = 'd617a78'               # the deployed W=false twin = the switch-off rev (issue 551 Pre-U6 state)
FLIP_REV = '84c5118'                    # the held, gated flip rev (b6/flip-84c5118.patch)
# D7 / start rule: a switch-off redeploy of SWITCHOFF_REV is NOT pre-gated (ops/gate-marker.sh: the single marker
# names the flip rev after its gate), so a switch-off costs the gate (22-35 min) + the VPS build/deploy + the drain-back
# and its verify. Each figure is an estimate; start_rule.py takes overrides.
SWITCHOFF_MINUTES = {'gate': 35, 'deploy': 10, 'drain_back_and_verify': 20}

# ---- pins (b5/pins.py, operations.md "Verify, in order"; B6 flip expectations)
PATH_V = {  # id -> (cents, currency, rule)
    7427765: (300000000, 'EUR', 537), 5943121: (132157992, 'EUR', 537), 369057: (6237873629, 'EUR', 537),
    251619: (2433875808, 'EUR', 537), 380921: (724704000, 'PLN', 537), 392980: (3574004272, 'EUR', 537),
    425446: (154900719, 'EUR', 537), 706887: (7056079275, 'PLN', 537), 1024214: (340000000, 'EUR', 537),
    5779108: (212000000, 'EUR', 537), 6342034: (1083675445, 'EUR', 537), 6921980: (2493033611, 'EUR', 537),
    7044132: (238599200, 'EUR', 537), 7829010: (200000000, 'EUR', 537), 8514592: (17924893700, 'DKK', 537),
    928196: (4679746035, 'EUR', 492),
}
PATH_W = {8426318: (300000000000, 'GBP'), 585917: (25000000, 'EUR'), 5995030: (154951160, 'PLN')}
TRIO = {6450467: (166066168294, 'EUR'), 6450469: (166066168294, 'EUR'), 6450473: (166066168294, 'EUR')}
MUST_STAY = [8440393, 7487018, 5316712, 6885102, 4930240, 8822313, 4925871, 175096, 5625591]
NOT_LSA = [6465510]                     # D3: must not serve lot-sum-awards
U9_PINS = [6908248, 6679375, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260]   # point 12's standing-rule pins
NEAR_2PCT_WATCH = [899272, 5842187]     # U2: within 2 % of EUR 100 m (899272 outside B_P, unread)
NOT_REACHABLE = 7511890                 # not in the cohort (B3 / 2620)

SALT_STAGE1 = '551-b6-stage1-'
SALT_DELTA = '551-b6-delta-'
STAGE1_SALTED_N = 300

TENDER_COLS = ['id', 'source', 'current_seq', 'current_published_at', 'current_value_derivation',
               'current_value_cents', 'current_value_currency', 'current_value_eur_cents',
               'current_value_derivation_detail', 'projection_epoch']


# ---------------------------------------------------------------- small helpers
def now_iso():
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())


def epoch_of(iso):
    return int(datetime.datetime.fromisoformat(iso.replace('Z', '+00:00')).timestamp())


def iso_of(ep):
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(int(ep)))


def sha_bytes(b):
    return hashlib.sha256(b).hexdigest()


def sha_file(p):
    with open(p, 'rb') as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def ids_sha(ids):
    """sha256 of the ascending ids as ASCII lines 'id\\n' (the id-list digest used throughout B6)."""
    return sha_bytes(''.join(f'{i}\n' for i in sorted(set(int(x) for x in ids))).encode())


def salted(ids, salt):
    """ids ordered by sha256(salt + str(id)) ascending (the B5/B6 pre-registered draw order)."""
    return sorted(set(int(i) for i in ids), key=lambda i: hashlib.sha256((salt + str(i)).encode()).hexdigest())


def jbytes(obj):
    return (json.dumps(obj, indent=1, sort_keys=True, ensure_ascii=False, default=str) + '\n').encode('utf-8')


def jdump(path, obj):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    tmp = path + '.tmp'
    with open(tmp, 'wb') as fh:
        fh.write(jbytes(obj))
    os.replace(tmp, path)
    return sha_file(path)


def jload(path):
    if path.endswith('.gz'):
        with gzip.open(path, 'rt', encoding='utf-8') as fh:
            return json.load(fh)
    with open(path, encoding='utf-8') as fh:
        return json.load(fh)


def chunks(seq, n):
    seq = list(seq)
    return [seq[i:i + n] for i in range(0, len(seq), n)]


def rows_of(body):
    """A /v1/sql body -> list of dicts (columns zipped). Raises on a body that is not an answer."""
    if not (isinstance(body, dict) and 'rows' in body and 'columns' in body):
        raise ValueError(f'not an answer: {str(body)[:200]}')
    return [dict(zip(body['columns'], r)) for r in body['rows']]


# ---------------------------------------------------------------- records
def the_3502():
    rows = jload(SERVED_LSA)
    served = {int(r[0]): r for r in rows}
    ids = sorted(i for i in served if i != EXCLUDED_179732)
    assert len(ids) == 3502 and len(rows) == 3503, (len(ids), len(rows))
    return ids, served


def served_row_dict(r):
    """served_lsa_rows.json's column order (b5): id, source, current_published_at, current_value_derivation,
    current_value_cents, current_value_currency, current_value_eur_cents, current_value_derivation_detail."""
    keys = ['id', 'source', 'current_published_at', 'current_value_derivation', 'current_value_cents',
            'current_value_currency', 'current_value_eur_cents', 'current_value_derivation_detail']
    return dict(zip(keys, r))


def p_u2():
    P = jload(P_U2)['guard_sets']['v2+both']
    assert P['n'] == 3467 and P['ids_sha256'] == P_U2_IDS_SHA, (P['n'], P['ids_sha256'])
    ent = {int(e['id']): e for e in P['ships']}
    assert ids_sha(ent) == P_U2_IDS_SHA
    return ent


def rg_u2():
    rs = jload(R_SINGLE)
    rx = jload(R_EXCESS)
    assert rs['n'] == 34 and rx['n'] == 1, (rs['n'], rx['n'])
    return sorted(set(rs['ids']) | set(rx['ids']))


def the_61():
    items = jload(READ_ITEMS)['items']
    ids = [int(i['id']) for i in items]
    assert len(ids) == 61 and len(set(ids)) == 61, len(ids)
    return sorted(ids)


def pin_ids():
    return sorted(set(PATH_V) | set(PATH_W) | set(TRIO) | set(MUST_STAY) | set(NOT_LSA) | set(U9_PINS))


def detail_of(row):
    d = row.get('current_value_derivation_detail')
    if d is None:
        return {}
    if isinstance(d, dict):
        return d
    try:
        return json.loads(d)
    except ValueError:
        return {'_unparsed': d}


def vb_key(vb):
    if not isinstance(vb, dict):
        return None
    return [vb.get('field'), vb.get('notice_id'), vb.get('cents')]


# ---------------------------------------------------------------- local plan probe
class PlanRefused(Exception):
    pass


def ensure_plan_db():
    if not os.path.exists(PLAN_DB):
        os.makedirs(PLAN_DIR, exist_ok=True)
        shutil.copyfile(PLAN_DB_SRC, PLAN_DB)
    return PLAN_DB


def ensure_plan_probe():
    """-> the probe binary to run. The copy at PLAN_PROBE is made once from target/debug/plan-probe and its sha256
    recorded beside it; later calls verify the copy against that record (a changed copy is refused). KIT_PLAN_PROBE
    overrides the path (no copy, no record)."""
    if os.environ.get('KIT_PLAN_PROBE'):
        return PLAN_PROBE
    if not os.path.exists(PLAN_PROBE):
        if not os.path.exists(PLAN_PROBE_SRC):
            raise PlanRefused(f'no plan-probe: neither the kit copy {PLAN_PROBE} nor {PLAN_PROBE_SRC} exists (a gate\'s '
                              'cargo clean removed target/?): rebuild it with `cargo build -p tender-db --bin plan-probe`'
                              ' or point KIT_PLAN_PROBE at a copy')
        os.makedirs(PLAN_DIR, exist_ok=True)
        tmp = PLAN_PROBE + '.tmp'
        shutil.copyfile(PLAN_PROBE_SRC, tmp)
        os.chmod(tmp, 0o755)
        os.replace(tmp, PLAN_PROBE)
        with open(PLAN_PROBE_SHA, 'w') as fh:
            fh.write(f'{sha_file(PLAN_PROBE)}  plan-probe (copied {now_iso()} from {PLAN_PROBE_SRC})\n')
    if os.path.exists(PLAN_PROBE_SHA):
        with open(PLAN_PROBE_SHA) as fh:
            want = fh.read().split()[0]
        if sha_file(PLAN_PROBE) != want:
            raise PlanRefused(f'{PLAN_PROBE} does not match its recorded sha256 {want[:16]}')
    return PLAN_PROBE


SCAN_OK = re.compile(r'^SCAN (ids|ns|probe|generate_series)( AS \w+)?$')


def plan_check(sql, name, workdir):
    """EXPLAIN QUERY PLAN through turso (plan-probe) on the plan.db COPY. -> list of plan steps. Raises PlanRefused when
    a step SCANs anything but the statement's own literal rows (a VALUES CTE named ids/ns/probe, or generate_series)."""
    ensure_plan_db()
    probe = ensure_plan_probe()
    os.makedirs(workdir, exist_ok=True)
    stmts = os.path.join(workdir, f'.plan_{os.getpid()}.stmts')
    with open(stmts, 'w', encoding='utf-8') as fh:
        fh.write(f'-- name: {name}\n{sql}\n')
    try:
        p = subprocess.run([probe, 'plan', PLAN_DB, stmts], capture_output=True, text=True, timeout=120)
    finally:
        try:
            os.remove(stmts)
        except OSError:
            pass
    steps = [ln.split('\t', 2)[2] for ln in p.stdout.splitlines() if ln.startswith('PLAN\t')]
    if p.returncode != 0 or not steps:
        raise PlanRefused(f'{name}: plan-probe rc={p.returncode} {p.stderr[:300]!r} {p.stdout[:300]!r}')
    bad = [s for s in steps if s.strip().startswith('SCAN') and not SCAN_OK.match(s.strip())]
    if bad:
        raise PlanRefused(f'{name}: plan SCANs {bad} -- not sent')
    return steps


# ---------------------------------------------------------------- the box
class BoxRefused(Exception):
    pass


def ssh_transport(sql):
    """-> (stdout, stderr). One request; never retried here or anywhere."""
    try:
        p = subprocess.run(SSH, input=sql, capture_output=True, text=True, timeout=120)
        return p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return '', 'local timeout 120 s'


class Box:
    """Bounded /v1/sql reads (see the module doc). `state_dir` holds state.json, queries.log and raw/ bodies."""

    def __init__(self, state_dir, budget, transport=None, gap=2.5, hourly=280, probe=True, log=print):
        self.dir = state_dir
        self.budget = budget
        self.transport = transport or ssh_transport
        self.gap = gap
        self.hourly = hourly
        self.probe = probe
        self.log = log
        os.makedirs(os.path.join(self.dir, 'raw'), exist_ok=True)
        self.state_path = os.path.join(self.dir, 'state.json')
        self.log_path = os.path.join(self.dir, 'queries.log')

    def _state(self):
        try:
            return jload(self.state_path)
        except (OSError, ValueError):
            return {'used': 0, 'last': 0.0, 'flag': None, 'times': []}

    def _save(self, s):
        jdump(self.state_path, s)

    def sq(self, sql, name):
        """-> the parsed body (dict with rows/columns) or None (error body / 408 / non-JSON; recorded, flag set, not
        retried). Raises BoxRefused before sending when the budget, the hourly count or the flag forbids it."""
        sql = ' '.join(sql.split())
        s = self._state()
        is_health = sql.strip().upper() == 'SELECT 1'
        if s['used'] >= self.budget:
            raise BoxRefused(f'budget {s["used"]}/{self.budget} used')
        if s.get('flag') and not is_health:
            raise BoxRefused(f'flag set ({s["flag"]}); run health (SELECT 1) first')
        recent = [t for t in s.get('times', []) if time.time() - t < 3600]
        if len(recent) >= self.hourly:
            raise BoxRefused(f'{len(recent)} requests in the last hour (cap {self.hourly}; the endpoint allows 300/h)')
        if not sql.upper().startswith(('SELECT', 'WITH')):
            raise BoxRefused('only one bare SELECT is ever sent')
        plan = plan_check(sql, name, self.dir) if (self.probe and not is_health) else ['(health: no plan)']
        wait = s.get('last', 0.0) + self.gap - time.time()
        if wait > 0:
            time.sleep(wait)
        t0 = time.time()
        s['used'] += 1
        s['last'] = t0
        s['times'] = recent + [t0]
        self._save(s)
        out, err = self.transport(sql)
        dt = time.time() - t0
        s['last'] = time.time()
        n = s['used']
        raw = os.path.join(self.dir, 'raw', f'{n:04d}_{name}.json')
        with open(raw, 'w', encoding='utf-8') as fh:
            fh.write(out or '')
        with open(os.path.join(self.dir, 'raw', f'{n:04d}_{name}.sql'), 'w', encoding='utf-8') as fh:
            fh.write(sql + '\n-- plan: ' + ' | '.join(plan) + '\n')
        body, ok = None, False
        try:
            d = json.loads(out)
            if isinstance(d, dict) and 'rows' in d and 'columns' in d:
                body, ok = d, True
                status = f'rows={d.get("row_count", len(d["rows"]))} truncated={d.get("truncated")}'
            else:
                status = f'ERROR-BODY {(out or "")[:200]!r}'
        except ValueError:
            status = f'NON-JSON {(out or "")[:200]!r} stderr={(err or "")[:200]!r}'
        if not ok:
            s['flag'] = f'{name}: {status[:120]}'
        elif is_health:
            s['flag'] = None
        self._save(s)
        line = f'{now_iso()} #{n} {name} {dt:.2f}s {status} sha256={sha_bytes((out or "").encode())[:16]} raw={os.path.basename(raw)}'
        with open(self.log_path, 'a', encoding='utf-8') as fh:
            fh.write(line + '\n')
        self.log(line)
        if ok and body.get('truncated'):
            # a capped answer is not the answer: callers size statements so this never happens
            s['flag'] = f'{name}: truncated'
            self._save(s)
            return None
        return body

    def health(self):
        return self.sq('SELECT 1', 'health')


def box_from_args(state_dir, budget, fake=None, log=print):
    """A Box for a script: the real ssh transport, or (tests) a fake transport object with `.answer(sql)`."""
    if fake is not None:
        return Box(state_dir, budget, transport=lambda sql: (fake.answer(sql), ''), gap=0.0, probe=True, log=log)
    return Box(state_dir, budget, log=log)


# One state dir for every box read of U6-U8 (so the budget and the rolling hour are shared). The kit-build
# confirmation reads used their own (u6/box_confirm, 5 of 5). Tests point KIT_BOX_DIR at a scratch dir.
BOX_DIR = os.environ.get('KIT_BOX_DIR') or os.path.join(U6, 'box')
BOX_BUDGET = 300


def shared_box(fake=None, log=print):
    return box_from_args(BOX_DIR, BOX_BUDGET, fake=fake, log=log)


# ---------------------------------------------------------------- SQL builders (every shape plan-probed)
def sql_changes(lo, hi):
    """Change rows in (lo, hi] (cursor is the rowid: a PK range seek), each lot row with its Tender (lots PK seek)."""
    assert hi - lo <= 5000, (lo, hi)
    return ('SELECT c.cursor, c.entity_kind, c.entity_id, c.version_seq, c.op, c.changed_at, '
            "CASE WHEN c.entity_kind = 'lot' THEN (SELECT l.tender_id FROM lots l WHERE l.id = c.entity_id) END "
            f'AS lot_tender_id FROM changes c WHERE c.cursor > {int(lo)} AND c.cursor <= {int(hi)} ORDER BY c.cursor')


def sql_tenders_pk(ids):
    ids = sorted(set(int(i) for i in ids))
    assert 0 < len(ids) <= 2000, len(ids)
    cols = ', '.join(f't.{c}' for c in TENDER_COLS)
    return f'SELECT {cols} FROM tenders t WHERE t.id IN ({", ".join(map(str, ids))})'


def sql_notice_probe(lo, hi, step):
    assert (hi - lo) // step <= 2000
    return ('SELECT s.value AS probe, (SELECT n.id FROM notices n WHERE n.id >= s.value ORDER BY n.id LIMIT 1) AS nid, '
            '(SELECT n.ingested_at FROM notices n WHERE n.id >= s.value ORDER BY n.id LIMIT 1) AS ingested_at '
            f'FROM generate_series({int(lo)}, {int(hi)}, {int(step)}) s')


def sql_notice_bucket(lo, hi):
    assert hi - lo <= 5000
    return f'SELECT n.id, n.ingested_at FROM notices n WHERE n.id > {int(lo)} AND n.id <= {int(hi)} ORDER BY n.id'


def sql_notice_tenders(lo, hi):
    """Notices with id in [lo, hi) and the Tender versions they caused (tender_versions_notice index seek)."""
    assert hi - lo <= 5000
    return ('SELECT n.id, n.ingested_at, n.source, n.parse_state, n.projected, v.tender_id, v.seq FROM notices n '
            'LEFT JOIN tender_versions v ON v.caused_by_notice_id = n.id '
            f'WHERE n.id >= {int(lo)} AND n.id < {int(hi)} ORDER BY n.id')


def sql_notices_to_tenders(nids):
    nids = sorted(set(int(i) for i in nids))
    assert 0 < len(nids) <= 2000
    vals = ', '.join(f'({i})' for i in nids)
    return (f'WITH ns(i) AS (VALUES {vals}) SELECT ns.i AS notice_id, v.tender_id, v.seq FROM ns '
            'CROSS JOIN tender_versions v ON v.caused_by_notice_id = ns.i')


def sql_band(limit=200):
    return ('SELECT id, current_value_eur_cents, current_value_cents, current_value_currency, '
            'current_value_derivation_detail FROM tenders WHERE current_value_eur_cents >= 10000000000 '
            f"AND current_value_derivation = 'lot-sum-awards' ORDER BY current_value_eur_cents DESC LIMIT {int(limit)}")


# ---------------------------------------------------------------- PK reads (snapshot / post-run reads)
def pk_read(box, ids, label, out_dir, chunk=2000, expected_absent=()):
    """PK-read `ids` (<= 2,000 per statement). Writes out_dir/rows.json {id: row} + manifest.json. Returns
    (rows, failures, missing). A failed chunk is recorded and NOT retried; the caller decides (a run with failures > 0
    is not a measurement). `expected_absent`: ids KNOWN to have no tenders row (a retired Tender: its last change row
    in a read range is `tender removed`, retire_chunk_tx deleted the row). Such an id answering no row is recorded in
    `absent_expected` and does not make the read incomplete; any other id with no row does (`missing`)."""
    os.makedirs(out_dir, exist_ok=True)
    ids = sorted(set(int(i) for i in ids))
    exp_abs = {int(i) for i in expected_absent}
    rows, failures, chunks_rec = {}, [], []
    for k, part in enumerate(chunks(ids, chunk)):
        body = box.sq(sql_tenders_pk(part), f'{label}_pk{k:03d}')
        if body is None:
            failures.append({'chunk': k, 'first': part[0], 'last': part[-1], 'n': len(part)})
            chunks_rec.append({'chunk': k, 'n': len(part), 'ok': False})
            break                               # the flag is set: nothing else may run until health
        got = rows_of(body)
        for r in got:
            rows[int(r['id'])] = r
        chunks_rec.append({'chunk': k, 'n': len(part), 'ok': True, 'rows': len(got)})
    absent = sorted(i for i in ids if i not in rows and i in exp_abs) if not failures else None
    missing = sorted(i for i in ids if i not in rows and i not in exp_abs) if not failures else None
    man = {'label': label, 'at': now_iso(), 'n_ids': len(ids), 'ids_sha256': ids_sha(ids), 'chunks': chunks_rec,
           'failures': failures, 'n_rows': len(rows), 'missing_ids': missing, 'absent_expected': absent,
           'complete': (not failures) and not missing}
    jdump(os.path.join(out_dir, 'rows.json'), {str(k): v for k, v in sorted(rows.items())})
    jdump(os.path.join(out_dir, 'manifest.json'), man)
    return rows, failures, missing


def load_rows(d):
    """rows.json of a pk_read dir -> {int id: row}; refuses an incomplete read."""
    man = jload(os.path.join(d, 'manifest.json'))
    if not man.get('complete'):
        raise SystemExit(f'{d}: the PK read is incomplete ({man.get("failures")}, missing {man.get("missing_ids")}); '
                         'it is not a measurement')
    return {int(k): v for k, v in jload(os.path.join(d, 'rows.json')).items()}


def absent_of(d):
    """The ids a pk_read dir recorded as expected-absent (retired Tenders)."""
    return {int(i) for i in (jload(os.path.join(d, 'manifest.json')).get('absent_expected') or [])}


# ---------------------------------------------------------------- change rows: the six kinds the store writes
RESULT_KINDS = ('lot_result', 'bid', 'contract')
KNOWN_KINDS = ('tender', 'lot', 'organization') + RESULT_KINDS     # canonical.rs `changes`: entity_kind's six values


def classify_changes(rows):
    """Change rows -> what a Tender list needs from them.

    The store writes six kinds (canonical.rs's `changes` schema). Removals are ordinary: a fold's orphan sweep
    (`sweep_orphaned_entities`) and a Tender's retirement (`retire_chunk_tx`) write `removed` rows of kind lot_result /
    bid / contract / lot and then DELETE the entity rows -- retirement first writes `tender removed` and deletes the
    Tender -- so a removed lot's `lot_tender_id` reads NULL. A version diff (`append_version_changes`) writes result
    rows only when the version's rounds changed, and then always a `tender` row for the same Tender at the same
    instant, so result rows are counted, not mapped (their Tender's own row carries it).
      tenders           ids with a tender row (any op)
      retired           ids whose LAST tender row here is `removed` (no tenders row: never PK-readable)
      lot_tenders       Tenders of the mapped lot rows
      live              (tenders | lot_tenders) - retired: the Tenders to read / list
      unmapped_lots     cursors of lot rows with no Tender: 'removed' (the entity row was deleted after the row),
                        'gone_later' (added/changed, and a LATER row here removes that lot), 'bad' (neither: a lot that
                        vanished with no removed row in the range -- the only unexplained case)
      no_tender_instant cursors of unmapped lot rows and result rows whose changed_at carries no tender row here (a
                        sweep with no fact change of its Tender: its served facts did not move)
      by_kind_op        {'kind/op': n};  unknown_kinds: kinds outside the six."""
    rows = sorted(rows, key=lambda r: int(r['cursor']))
    by = collections.Counter(f'{r["entity_kind"]}/{r["op"]}' for r in rows)
    unknown = sorted({r['entity_kind'] for r in rows} - set(KNOWN_KINDS))
    tender_instants = {int(r['changed_at']) for r in rows if r['entity_kind'] == 'tender'}
    last_op, tenders, lot_tenders = {}, set(), set()
    last_removed = {}
    for r in rows:
        if r['entity_kind'] == 'tender':
            t = int(r['entity_id'])
            tenders.add(t)
            last_op[t] = r['op']
        elif r['entity_kind'] == 'lot' and r['op'] == 'removed':
            last_removed[int(r['entity_id'])] = int(r['cursor'])
    retired = {t for t, op in last_op.items() if op == 'removed'}
    unm = {'removed': [], 'gone_later': [], 'bad': []}
    no_inst = []
    for r in rows:
        c = int(r['cursor'])
        if r['entity_kind'] == 'lot':
            if r.get('lot_tender_id') is not None:
                lot_tenders.add(int(r['lot_tender_id']))
                continue
            if r['op'] == 'removed':
                unm['removed'].append(c)
            elif last_removed.get(int(r['entity_id']), -1) > c:
                unm['gone_later'].append(c)
            else:
                unm['bad'].append(c)
            if int(r['changed_at']) not in tender_instants:
                no_inst.append(c)
        elif r['entity_kind'] in RESULT_KINDS and int(r['changed_at']) not in tender_instants:
            no_inst.append(c)
    return {'by_kind_op': dict(sorted(by.items())), 'unknown_kinds': unknown, 'tenders': tenders, 'retired': retired,
            'lot_tenders': lot_tenders, 'live': (tenders | lot_tenders) - retired, 'unmapped_lots': unm,
            'no_tender_instant': no_inst}


# ---------------------------------------------------------------- the band (D4 / V4 / switch-off verify)
API_BAND_PATH = '/v1/tenders?min_value=10000000000&limit=1000'
API_BAND_MAX_PAGES = 400
BAND_COLS = ['id', 'current_value_eur_cents', 'current_value_cents', 'current_value_currency',
             'current_value_derivation_detail']


def api_http():
    """The public-API client every kit GET uses: fetch_b6's LockedHttp on cohort_parity's shared stamp (at most one
    GET per 1.5 s across fetch_b6 / confirm_draw / the kit; a 408 retried once, as B5's band route did)."""
    import importlib.util
    spec = importlib.util.spec_from_file_location('fetch_b6_kit', os.path.join(B6, 'fetch_b6.py'))
    F = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(F)
    return F.LockedHttp(stamp=F.CP.STAMP)


def band_api(http, max_pages=API_BAND_MAX_PAGES):
    """The band over the public API (B5's route, b5/confirm_draw.py band_seek: 40 pages, 35,121 rows on 10-19):
    GET /v1/tenders?min_value=10000000000&limit=1000 paged by next_cursor, keeping value_derivation.method ==
    'lot-sum-awards'. -> a body in the /v1/sql band statement's shape (columns BAND_COLS) plus 'route': 'api' and the
    page log; None when a page is not a 200 JSON answer (recorded in the body's 'error'). The EUR column is the
    cents for an EUR head and None otherwise (the server's min_value already put it at >= EUR 100 m)."""
    cursor, rows, pages, items = None, [], [], 0
    for _ in range(max_pages):
        path = API_BAND_PATH + (f'&cursor={cursor}' if cursor else '')
        code, body = http.get(path)
        if code != 200:
            return {'route': 'api', 'error': f'{path}: HTTP {code} {(body or b"")[:200]!r}', 'pages': pages}
        d = json.loads(body)
        if d.get('ignored_filters'):
            return {'route': 'api', 'error': f'{path}: ignored_filters {d["ignored_filters"]}', 'pages': pages}
        its = d.get('items') or []
        items += len(its)
        for it in its:
            vd = it.get('value_derivation') or {}
            if vd.get('method') == 'lot-sum-awards':
                v = it.get('value') or {}
                cur, cents = v.get('currency'), v.get('cents')
                det = {k: x for k, x in vd.items() if k != 'method'}
                rows.append([int(it['id']), cents if cur == 'EUR' else None, cents, cur, json.dumps(det, sort_keys=True)])
        pages.append({'cursor': cursor, 'items': len(its), 'more': bool(d.get('more')), 'next': d.get('next_cursor')})
        if not d.get('more'):
            break
        nc = d.get('next_cursor')
        if not nc or nc == cursor:
            return {'route': 'api', 'error': f'more=true with next_cursor {nc!r} after {cursor!r}', 'pages': pages}
        cursor = nc
    else:
        return {'route': 'api', 'error': f'more than {max_pages} pages', 'pages': pages}
    return {'columns': BAND_COLS, 'rows': rows, 'row_count': len(rows), 'truncated': False, 'route': 'api',
            'pages': len(pages), 'items_seen': items, 'page_log': pages, 'read_at': now_iso()}


def read_band(box, name, out_path, route='sql', http=None, log=print):
    """D4's band read. route 'sql': the plan-probed /v1/sql range seek (sql_band); a statement that does not answer is
    recorded, flags the box and is NOT retried -- the band is then read over the public API instead (band_api: a
    different path, B5's proven route), so the measurement still completes. route 'api': the API only. Writes the
    body to out_path. -> (body or None, route used)."""
    if route == 'sql':
        body = box.sq(sql_band(), name)
        if body is not None:
            body = dict(body, route='sql')
            jdump(out_path, body)
            return body, 'sql'
        log(f'{name}: the /v1/sql band seek did not answer (box flag set; run box_health before the next box read); '
            'reading the band over the public API instead')
    body = band_api(http or api_http())
    jdump(out_path, body)
    if body.get('error'):
        log(f'{name}: the API band read failed: {body["error"]}')
        return None, 'api'
    return body, 'api'


# ---------------------------------------------------------------- exit-code discipline
NOT_A_MEASUREMENT_EXIT = 3


def run_main(main, *a, **k):
    """Run a script's main() so that a tool fault can never read as a verdict: an uncaught exception, or a SystemExit
    carrying a message, prints the fault and returns 3 (NOT A MEASUREMENT); an int SystemExit (argparse) keeps its
    code. Checkers' SWITCH OFF is 10, a code Python never produces on its own."""
    import traceback
    try:
        return main(*a, **k)
    except SystemExit as e:
        if e.code is None or isinstance(e.code, int):
            return e.code or 0
        print(f'NOT A MEASUREMENT (tool fault): {e.code}')
        return NOT_A_MEASUREMENT_EXIT
    except KeyboardInterrupt:
        raise
    except BaseException as e:  # noqa: BLE001 - every fault maps to "not a measurement", never to a verdict
        traceback.print_exc()
        print(f'NOT A MEASUREMENT (tool fault): {type(e).__name__}: {e}')
        return NOT_A_MEASUREMENT_EXIT


def enumerate_changes(box, lo, hi, out_dir, width=5000):
    """Change rows in (lo, hi] in chunks of <= `width` cursor values (<= 5,000 rows each: cursor is unique). Writes
    one file per chunk (resumable: a chunk already on disk with a matching manifest entry is not re-read) and returns
    (rows, complete)."""
    os.makedirs(out_dir, exist_ok=True)
    man_p = os.path.join(out_dir, 'manifest.json')
    man = jload(man_p) if os.path.exists(man_p) else {'lo': lo, 'hi': hi, 'width': width, 'chunks': {}}
    if (man['lo'], man['hi'], man['width']) != (lo, hi, width):
        raise SystemExit(f'{out_dir}: an earlier enumeration used ({man["lo"]}, {man["hi"]}], width {man["width"]}; '
                         'use a fresh directory')
    rows, complete = [], True
    a = lo
    while a < hi:
        b = min(a + width, hi)
        key = f'{a}_{b}'
        fp = os.path.join(out_dir, f'chunk_{key}.json')
        rec = man['chunks'].get(key)
        if rec and rec.get('ok') and os.path.exists(fp) and sha_file(fp) == rec['sha256']:
            body = jload(fp)
        else:
            body = box.sq(sql_changes(a, b), f'changes_{key}')
            if body is None:
                man['chunks'][key] = {'ok': False, 'at': now_iso()}
                jdump(man_p, man)
                complete = False
                break
            sha = jdump(fp, body)
            man['chunks'][key] = {'ok': True, 'at': now_iso(), 'sha256': sha, 'rows': len(body['rows'])}
            jdump(man_p, man)
        got = rows_of(body)
        assert len(got) <= width
        for r in got:
            assert a < int(r['cursor']) <= b, (r, a, b)
        rows += got
        a = b
    man['complete'] = complete and a >= hi
    jdump(man_p, man)
    return rows, man['complete']
