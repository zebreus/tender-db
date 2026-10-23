"""Issue 551 B6 U6/U7/U8 kit: shared plumbing (paths, records, the bounded box-read helper, the local plan probe).

Every box read goes through `Box.sq()`:
  - the statement is plan-probed LOCALLY first (turso's own planner: a kit COPY of target/debug/plan-probe, sha256
    recorded, so a gate's `cargo clean` cannot remove it; run on a copy of the v551 plan.db; /v1/sql refuses EXPLAIN)
    and refused if any step SCANs a base table (a CTE or generate_series may be scanned: their rows are the
    statement's own literals);
  - one bare SELECT through `ssh root@zebreus.click /root/sq.sh` (the token stays on the box), paced >= 2.5 s apart;
  - an answer counts only when the body is a JSON object with `rows` AND `columns` (prod-box-reads.md: a shed or
    408 body is never read as an empty result); anything else is recorded with its failure class, sets a flag, and is
    NEVER retried; while the flag is set only `SELECT 1` (health) may run, and a good health clears it;
  - failure classes: 'capped' (a 408 / interrupted / timeout / truncated / unknown body: the statement may have run to
    the cap) puts the statement's sha256 on the box's capped list and the IDENTICAL statement is never sent again --
    the resumable readers (tender_keyset, enumerate_changes, pk_read resume=True) resume with the page / chunk HALVED,
    down to FLOOR; 'shed' (503/429) and 'transport' (ssh, nothing reached /v1/sql) may be re-sent after health;
  - one lock (BOX_LOCK) for every Box, held across the state read, the pacing, the send and the save: one statement in
    flight across all the kit's state dirs, and no lost budget counts;
  - a hard budget (state file box/state.json) and a rolling one-hour count over every kit state dir (kit_state_dirs:
    the endpoint's 300/h is per token).
Every raw body is saved; every request is logged to box/queries.log.
Also here: classify_changes (the six change-row kinds, removed lots, retired Tenders), pk_read's expected-absent ids,
the band reader (the /v1/sql seek, falling back to the public API band -- never a retry of the statement), and
run_main, which maps any tool fault to exit 3 (NOT A MEASUREMENT) so a crash can never read as a verdict.

Offline tests inject a fake transport (`Box(transport=callable)`), which never touches the network.
"""
import collections
import contextlib
import datetime
import fcntl
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time

sys.dont_write_bytecode = True

U6 =os.path.dirname(os.path.abspath(__file__))                       # .../551-lot-sum/b6/u6
B6 = os.path.dirname(U6)
ROOT = os.path.dirname(B6)                                             # .../551-lot-sum
U7 = os.path.join(B6, 'u7')
U8 = os.path.join(B6, 'u8')
REPO = os.environ.get('KIT_REPO', '/home/user/tender-db')
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
# The plan db's source: the session scratchpad's v551 copy, or (if the scratchpad was pruned) the byte copy kept under
# b6/u9/plan/ (gitignored as *.db; its sha256 is in u9/KIT_W_SHA256SUMS).
PLAN_DB_SRC_SP = os.path.join(SP, 'v551', 'plan.db')
PLAN_DB_SRC_B6 = os.path.join(B6, 'u9', 'plan', 'plan.db')
PLAN_DB_SRC = PLAN_DB_SRC_SP if os.path.exists(PLAN_DB_SRC_SP) else PLAN_DB_SRC_B6
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
    tmp = f'{path}.{os.getpid()}.{threading.get_ident()}.tmp'       # unique: two writers never share a tmp name
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
        src = PLAN_DB_SRC if os.path.exists(PLAN_DB_SRC) else PLAN_DB_SRC_B6
        if not os.path.exists(src):
            raise PlanRefused(f'no plan db: neither {PLAN_DB_SRC_SP} nor {PLAN_DB_SRC_B6} exists (restore the byte copy '
                              'recorded in u9/KIT_W_SHA256SUMS)')
        os.makedirs(PLAN_DIR, exist_ok=True)
        shutil.copyfile(src, PLAN_DB)
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

# W-on drains (decision-w-on-drains.md C/E1, A3): prod builds `changes_entity_cursor` lazily at the END of a projection
# (canonical.rs ensure_changes_entity_cursor_index), and the kit's plan.db predates it. E1 is plan-probed on plan_w.db:
# a copy of plan.db with exactly that DDL applied THROUGH turso (the probe binary's `exec`), so the planner sees prod's
# index set. Without the index turso walks the cursor's rowid range instead (all kinds), which this kit refuses to call E1.
CHANGES_ENTITY_CURSOR_DDL = 'CREATE INDEX IF NOT EXISTS changes_entity_cursor ON changes(entity_kind, cursor)'
PLAN_DB_W = os.path.join(PLAN_DIR, 'plan_w.db')


def ensure_plan_db_w():
    """-> PLAN_DB_W, built once from the plan.db copy plus CHANGES_ENTITY_CURSOR_DDL (turso writes it; its -wal file
    stays beside the db, which turso reads)."""
    if not os.path.exists(PLAN_DB_W):
        ensure_plan_db()
        probe = ensure_plan_probe()
        shutil.copyfile(PLAN_DB, PLAN_DB_W)
        p = subprocess.run([probe, 'exec', PLAN_DB_W, CHANGES_ENTITY_CURSOR_DDL], capture_output=True, text=True,
                           timeout=120)
        if p.returncode != 0:
            for suffix in ('', '-wal', '-shm'):
                try:
                    os.remove(PLAN_DB_W + suffix)
                except OSError:
                    pass
            raise PlanRefused(f'plan_w.db: the index DDL failed: rc={p.returncode} {p.stderr[:300]!r}')
    return PLAN_DB_W


def plan_check(sql, name, workdir, plan_db=None):
    """EXPLAIN QUERY PLAN through turso (plan-probe) on the plan.db COPY (or `plan_db`, e.g. PLAN_DB_W). -> list of plan
    steps. Raises PlanRefused when a step SCANs anything but the statement's own literal rows (a VALUES CTE named
    ids/ns/probe, or generate_series), or when turso cannot plan it at all (PLAN-ERR, e.g. an INDEXED BY naming an index
    the plan db lacks)."""
    db = plan_db or ensure_plan_db()
    probe = ensure_plan_probe()
    os.makedirs(workdir, exist_ok=True)
    stmts = os.path.join(workdir, f'.plan_{os.getpid()}.stmts')
    with open(stmts, 'w', encoding='utf-8') as fh:
        fh.write(f'-- name: {name}\n{sql}\n')
    try:
        p = subprocess.run([probe, 'plan', db, stmts], capture_output=True, text=True, timeout=120)
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


# One lock for EVERY Box, whatever its state dir (u6/box, u9/box, u9/box_confirm, u7/switchoff/box, a test's dir): the
# kit's boxes share one token, one 2-concurrent cap and one 300/h, so one statement is in flight at a time across all
# of them (B5: one at a time, >= 2.5 s apart). Held across the state read, the pacing sleep, the send and the final save.
BOX_LOCK = os.environ.get('KIT_BOX_LOCK') or os.path.join(B6, '.box.lock')
# Failure classes (recorded per statement): 'capped' -- a 408 / interrupted / cap / local timeout / truncated / any body
# the kit does not recognise (fail closed): the statement may have run to the cap, so the IDENTICAL statement is never
# sent again (prod-box-reads: "never retry a 408"); a resume uses a smaller statement (halved), down to FLOOR.
# 'shed' -- a 503/429/saturated body: the statement did not run. 'transport' -- empty stdout and an ssh error: it never
# reached /v1/sql. shed and transport may go out again unchanged after a good health read.
FLOOR = 500
_SHED_RX = re.compile(r'saturat|\bshed\b|too many requests|busy|overloaded', re.I)


def classify_failure(out, err):
    o, e = (out or '').strip(), (err or '').strip()
    if not o:
        if 'local timeout' in e or not e:
            return 'capped'
        return 'transport'
    try:
        d = json.loads(o)
    except ValueError:
        d = None
    status = d.get('status') if isinstance(d, dict) else None
    if status in (429, 503) or (status is None and _SHED_RX.search(o)):
        return 'shed'
    return 'capped'


def stmt_sha(sql):
    return sha_bytes(' '.join(sql.split()).encode())


@contextlib.contextmanager
def box_lock():
    path = BOX_LOCK
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, 'a') as fh:
        fcntl.flock(fh, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(fh, fcntl.LOCK_UN)


def kit_state_dirs():
    """Every kit box state dir: the rolling-hour cap of each Box counts the statements of all of them (the endpoint's
    300/h is one budget per token)."""
    return [BOX_DIR, os.path.join(B6, 'u9', 'box'), os.path.join(B6, 'u9', 'box_confirm'),
            os.path.join(B6, 'u7', 'switchoff', 'box')]


class Box:
    """Bounded /v1/sql reads (see the module doc). `state_dir` holds state.json, queries.log and raw/ bodies.
    `hourly` caps the statements in any hour over this dir AND every other kit state dir (kit_state_dirs);
    `warn_at` prints and logs a warning once `used` passes it (never a refusal)."""

    def __init__(self, state_dir, budget, transport=None, gap=2.5, hourly=280, probe=True, log=print, plan_db=None,
                 warn_at=None):
        self.dir = state_dir
        self.budget = budget
        self.transport = transport or ssh_transport
        self.gap = gap
        self.hourly = hourly
        self.probe = probe
        self.log = log
        self.plan_db = plan_db            # None: plan.db; the W kit passes PLAN_DB_W (E1's index)
        self.warn_at = warn_at
        self.last_failure = None
        os.makedirs(os.path.join(self.dir, 'raw'), exist_ok=True)
        self.state_path = os.path.join(self.dir, 'state.json')
        self.log_path = os.path.join(self.dir, 'queries.log')

    def _state(self):
        """Only a MISSING state file is a fresh state; an unreadable one is refused (the budget, the flag and the
        capped list are never reset silently)."""
        try:
            s = jload(self.state_path)
        except FileNotFoundError:
            return {'used': 0, 'last': 0.0, 'flag': None, 'times': [], 'capped': []}
        except (OSError, ValueError) as e:
            raise BoxRefused(f'{self.state_path}: unreadable box state ({e}); repair it by hand, it is never reset')
        s.setdefault('capped', [])
        s.setdefault('times', [])
        return s

    def _save(self, s):
        jdump(self.state_path, s)

    def _peer_recent(self, now):
        mine = os.path.realpath(self.dir)
        n = 0
        for d in kit_state_dirs():
            if os.path.realpath(d) == mine:
                continue
            try:
                t = jload(os.path.join(d, 'state.json')).get('times', [])
            except (OSError, ValueError):
                continue
            n += sum(1 for x in t if now - x < 3600)
        return n

    def is_capped(self, sql):
        return stmt_sha(sql) in self._state().get('capped', [])

    def _refusals(self, s, sql, sha, is_health, name):
        if s['used'] >= self.budget:
            raise BoxRefused(f'budget {s["used"]}/{self.budget} used')
        if s.get('flag') and not is_health:
            raise BoxRefused(f'flag set ({s["flag"]}); run health (SELECT 1) first')
        if not is_health and sha in s.get('capped', []):
            raise BoxRefused(f'{name}: this exact statement was capped (a 408 / timeout / unknown body, sha256 '
                             f'{sha[:16]}): it is never sent again; resume with a smaller statement (the kit halves it)')

    def sq(self, sql, name):
        """-> the parsed body (dict with rows/columns) or None (error body / 408 / non-JSON; recorded with its failure
        class in self.last_failure, flag set, not retried). Raises BoxRefused before sending when the budget, the
        rolling hour, the flag or the capped list forbids it."""
        sql = ' '.join(sql.split())
        is_health = sql.strip().upper() == 'SELECT 1'
        if not sql.upper().startswith(('SELECT', 'WITH')):
            raise BoxRefused('only one bare SELECT is ever sent')
        sha = sha_bytes(sql.encode())
        self._refusals(self._state(), sql, sha, is_health, name)
        plan = plan_check(sql, name, self.dir, self.plan_db) if (self.probe and not is_health) else ['(health: no plan)']
        with box_lock():
            s = self._state()
            self._refusals(s, sql, sha, is_health, name)
            now = time.time()
            recent = [t for t in s.get('times', []) if now - t < 3600]
            peers = self._peer_recent(now)
            if len(recent) + peers >= self.hourly:
                raise BoxRefused(f'{len(recent)} + {peers} (other kit boxes) requests in the last hour (cap '
                                 f'{self.hourly}; B5 150/h for the W protocol, the endpoint allows 300/h per token)')
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
            body, ok, cls = None, False, None
            try:
                d = json.loads(out)
                if isinstance(d, dict) and 'rows' in d and 'columns' in d:
                    body, ok = d, True
                    status = f'rows={d.get("row_count", len(d["rows"]))} truncated={d.get("truncated")}'
                else:
                    status = f'ERROR-BODY {(out or "")[:200]!r}'
            except ValueError:
                status = f'NON-JSON {(out or "")[:200]!r} stderr={(err or "")[:200]!r}'
            if ok and body.get('truncated'):
                ok, cls = False, 'capped'                 # a capped answer is not the answer
                status += ' (a truncated answer: capped)'
            if not ok:
                cls = cls or classify_failure(out, err)
                s['flag'] = f'{name}: {cls}: {status[:120]}'
                if cls == 'capped' and not is_health and sha not in s['capped']:
                    s['capped'].append(sha)
                self.last_failure = {'name': name, 'class': cls, 'sha256': sha, 'status': status[:200]}
                body = None
            elif is_health:
                s['flag'] = None
            warn = None
            if self.warn_at is not None and s['used'] > self.warn_at and not s.get('warned'):
                s['warned'] = True
                warn = (f'{now_iso()} WARNING: {s["used"]} statements through {self.dir} (past {self.warn_at}, the '
                        'decision\'s section-5 upper estimate): record it in 551 (a warning, never a refusal)')
            self._save(s)
            line = (f'{now_iso()} #{n} {name} {dt:.2f}s {status}' + (f' class={cls}' if cls else '')
                    + f' sha256={sha_bytes((out or "").encode())[:16]} raw={os.path.basename(raw)}')
            with open(self.log_path, 'a', encoding='utf-8') as fh:
                fh.write(line + '\n')
                if warn:
                    fh.write(warn + '\n')
        self.log(line)
        if warn:
            self.log(warn)
        return body

    def health(self):
        return self.sq('SELECT 1', 'health')


def _fake_transport(fake):
    def t(sql):
        r = fake.answer(sql)
        return r if isinstance(r, tuple) else (r, '')
    return t


def box_from_args(state_dir, budget, fake=None, log=print):
    """A Box for a script: the real ssh transport, or (tests) a fake transport object with `.answer(sql)` (a str body,
    or an (stdout, stderr) pair for a transport failure)."""
    if fake is not None:
        return Box(state_dir, budget, transport=_fake_transport(fake), gap=0.0, probe=True, log=log)
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


E1_PAGE = 5000                          # LIMIT per E1 statement (decision C/E1)
E1_SMALL_SPAN = 50_000                  # spans of <= 50k cursor values always use the all-kind reader (D5 hygiene)
# E1 names its index (INDEXED BY): the plan is the same as without the hint (plan-probed both ways on plan_w.db), but
# if prod ever lacks `changes_entity_cursor` (it is built lazily at a projection's end; issue 82's rebuild once dropped
# a deferred index) turso refuses at PREPARE ("no such index") instead of walking the cursor's rowid range for 15 s.
# The kit-build confirmation read (u9/confirm_w12a.py) sends this exact shape once; E1_HINT=False is the fallback if
# /v1/sql ever refuses the clause itself.
E1_HINT = True


def sql_tender_keyset(after, hi, limit=E1_PAGE, hint=None):
    """E1: the tender rows in (after, hi], cursor order, each with its Tender's CURRENT derivation by PK subquery.
    Plan (plan_w.db): SEARCH c USING INDEX changes_entity_cursor (entity_kind=? AND cursor>? AND cursor<=?) +
    CORRELATED SCALAR SUBQUERY: SEARCH t USING INTEGER PRIMARY KEY. Paged by the last cursor (tender_keyset)."""
    hint = E1_HINT if hint is None else hint
    idx = ' INDEXED BY changes_entity_cursor' if hint else ''
    assert 0 < int(limit) <= E1_PAGE and int(hi) > int(after), (after, hi, limit)
    return ('SELECT c.cursor, c.entity_id, c.op, (SELECT t.current_value_derivation FROM tenders t WHERE t.id = '
            f'c.entity_id) AS d FROM changes c{idx} WHERE c.entity_kind = \'tender\' AND c.cursor > {int(after)} '
            f'AND c.cursor <= {int(hi)} ORDER BY c.cursor LIMIT {int(limit)}')


def _failed(box, **extra):
    lf = getattr(box, 'last_failure', None) or {}
    return dict({'ok': False, 'at': now_iso(), 'class': lf.get('class', 'capped'), 'sql_sha256': lf.get('sha256')},
                **extra)


def tender_keyset(box, after, hi, out_dir, label='e1', max_pages=None):
    """E1 over (after, hi]: pages of <= E1_PAGE tender rows, each page starting after the previous page's last cursor;
    a page shorter than its own LIMIT ends the walk. One file per page (resumable: a page already on disk with a
    matching manifest entry is not re-read). A page that did not answer is recorded with its failure class and never
    sent again as it was: after a transport failure or a shed it is re-sent unchanged (after health); after a 408 /
    timeout ('capped') the resume sends the same page with its LIMIT halved (recorded per page), down to FLOOR -- at the
    floor the walk stops, NOT A MEASUREMENT. `max_pages` stops early (the Z shortcut sends exactly one). -> (rows,
    complete): complete is False when a page did not answer or max_pages stopped the walk."""
    os.makedirs(out_dir, exist_ok=True)
    man_p = os.path.join(out_dir, 'manifest.json')
    man = jload(man_p) if os.path.exists(man_p) else {'after': after, 'hi': hi, 'page': E1_PAGE, 'pages': {}}
    if (man['after'], man['hi']) != (after, hi):
        raise SystemExit(f'{out_dir}: an earlier E1 walk used ({man["after"]}, {man["hi"]}]; use a fresh directory')
    rows, complete, a, n = [], False, after, 0
    while True:
        if max_pages is not None and n >= max_pages:
            break
        key = str(a)
        fp = os.path.join(out_dir, f'page_{a}.json')
        rec = man['pages'].get(key)
        if rec and rec.get('ok') and os.path.exists(fp) and sha_file(fp) == rec['sha256']:
            body = jload(fp)
            limit = int(rec.get('limit', E1_PAGE))
        else:
            limit = E1_PAGE
            if rec and not rec.get('ok'):
                limit = int(rec.get('limit', E1_PAGE))
                if rec.get('class', 'capped') == 'capped':
                    limit //= 2
                    if limit < FLOOR:
                        man['pages'][key] = dict(rec, floor=True)
                        man['floor'] = key
                        jdump(man_p, man)
                        print(f'{out_dir}: the E1 page after cursor {a} was capped down to the floor ({FLOOR} rows): '
                              'NOT A MEASUREMENT (hand the range to the owner; never re-send it)')
                        break
            body = box.sq(sql_tender_keyset(a, hi, limit), f'{label}_{a}' + (f'_l{limit}' if limit != E1_PAGE else ''))
            if body is None:
                man['pages'][key] = _failed(box, limit=limit)
                jdump(man_p, man)
                break
            sha = jdump(fp, body)
            man['pages'][key] = {'ok': True, 'at': now_iso(), 'sha256': sha, 'rows': len(body['rows']), 'limit': limit}
            jdump(man_p, man)
        got = rows_of(body)
        n += 1
        prev = a
        for r in got:
            c = int(r['cursor'])
            assert prev < c <= hi, (r, prev, hi)
            prev = c
        rows += got
        if len(got) < limit:
            complete = True
            break
        a = int(got[-1]['cursor'])
        if a >= hi:
            complete = True
            break
    man['complete'] = complete
    man['n_rows'] = len(rows)
    jdump(man_p, man)
    return rows, complete


def classify_tender_rows(rows):
    """E1 rows -> tenders (ids with a tender row), retired (last op `removed`), live, d (each live id's derivation from
    its LAST row's PK subquery) and lsa (live ids whose d is lot-sum-awards)."""
    rows = sorted(rows, key=lambda r: int(r['cursor']))
    last, d = {}, {}
    for r in rows:
        t = int(r['entity_id'])
        last[t] = r['op']
        d[t] = r.get('d')
    tenders = set(last)
    retired = {t for t, op in last.items() if op == 'removed'}
    live = tenders - retired
    by_op = collections.Counter(r['op'] for r in rows)
    return {'tenders': tenders, 'retired': retired, 'live': live, 'd': {t: d[t] for t in live},
            'lsa': {t for t in live if d[t] == 'lot-sum-awards'}, 'by_op': dict(sorted(by_op.items())),
            'n_rows': len(rows)}


def sql_band(limit=200):
    return ('SELECT id, current_value_eur_cents, current_value_cents, current_value_currency, '
            'current_value_derivation_detail FROM tenders WHERE current_value_eur_cents >= 10000000000 '
            f"AND current_value_derivation = 'lot-sum-awards' ORDER BY current_value_eur_cents DESC LIMIT {int(limit)}")


# ---------------------------------------------------------------- PK reads (snapshot / post-run reads)
def pk_read(box, ids, label, out_dir, chunk=2000, expected_absent=(), resume=False):
    """PK-read `ids` (<= 2,000 per statement). Writes out_dir/rows.json {id: row} + manifest.json. Returns
    (rows, failures, missing). A failed chunk is recorded (with its failure class) and NOT retried; the caller decides
    (a run with failures > 0 is not a measurement). `expected_absent`: ids KNOWN to have no tenders row (a retired
    Tender: its last change row in a read range is `tender removed`, retire_chunk_tx deleted the row). Such an id
    answering no row is recorded in `absent_expected` and does not make the read incomplete; any other id with no row
    does (`missing`).
    resume=True (the W kit, the switch-off): each answered chunk is kept on disk (part_<first>_<last>_<n>.json) and a
    re-run over the SAME id list skips it; a chunk that failed 'capped' (408 / timeout) is re-sent HALVED, never as it
    was (down to FLOOR ids; at the floor: NOT A MEASUREMENT); a transport / shed failure is re-sent unchanged."""
    os.makedirs(out_dir, exist_ok=True)
    ids = sorted(set(int(i) for i in ids))
    exp_abs = {int(i) for i in expected_absent}
    man_p = os.path.join(out_dir, 'manifest.json')
    prev = jload(man_p) if (resume and os.path.exists(man_p)) else None
    parts = dict(prev.get('parts') or {}) if (prev and prev.get('ids_sha256') == ids_sha(ids)) else {}
    rows, failures, chunks_rec = {}, [], []
    queue = chunks(ids, chunk)
    k = 0
    while queue:
        part = queue.pop(0)
        key = f'{part[0]}_{part[-1]}_{len(part)}'
        fp = os.path.join(out_dir, f'part_{key}.json')
        rec = parts.get(key)
        if resume and rec and rec.get('ok') and os.path.exists(fp) and sha_file(fp) == rec.get('sha256'):
            got = rows_of(jload(fp))
        else:
            if rec and not rec.get('ok') and rec.get('class', 'capped') == 'capped':
                half = len(part) // 2
                if half < FLOOR:
                    failures.append({'chunk': k, 'first': part[0], 'last': part[-1], 'n': len(part), 'floor': True,
                                     'class': 'capped'})
                    chunks_rec.append({'chunk': k, 'n': len(part), 'ok': False, 'floor': True})
                    print(f'{out_dir}: PK chunk {key} was capped down to the floor ({FLOOR} ids): NOT A MEASUREMENT')
                    break
                parts[key] = dict(rec, split=True)
                queue[0:0] = [part[:half], part[half:]]
                continue
            body = box.sq(sql_tenders_pk(part), f'{label}_pk{k:03d}' + (f'_n{len(part)}' if len(part) != chunk else ''))
            if body is None:
                parts[key] = _failed(box, n=len(part))
                failures.append({'chunk': k, 'first': part[0], 'last': part[-1], 'n': len(part),
                                 'class': parts[key]['class']})
                chunks_rec.append({'chunk': k, 'n': len(part), 'ok': False})
                break                               # the flag is set: nothing else may run until health
            got = rows_of(body)
            if resume:
                parts[key] = {'ok': True, 'at': now_iso(), 'sha256': jdump(fp, body), 'n': len(part),
                              'rows': len(got)}
        for r in got:
            rows[int(r['id'])] = r
        chunks_rec.append({'chunk': k, 'n': len(part), 'ok': True, 'rows': len(got)})
        k += 1
    absent = sorted(i for i in ids if i not in rows and i in exp_abs) if not failures else None
    missing = sorted(i for i in ids if i not in rows and i not in exp_abs) if not failures else None
    man = {'label': label, 'at': now_iso(), 'n_ids': len(ids), 'ids_sha256': ids_sha(ids), 'chunks': chunks_rec,
           'failures': failures, 'n_rows': len(rows), 'missing_ids': missing, 'absent_expected': absent,
           'complete': (not failures) and not missing, 'parts': parts if resume else {}}
    jdump(os.path.join(out_dir, 'rows.json'), {str(k): v for k, v in sorted(rows.items())})
    jdump(man_p, man)
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
    different path, B5's proven route), so the measurement still completes. A band statement this box already saw
    capped is never sent again: the API directly. route 'api': the API only. Writes the body to out_path. -> (body or
    None, route used)."""
    if route == 'sql':
        if box.is_capped(sql_band()):
            log(f'{name}: this box saw the band statement capped before: it is never re-sent; the public API instead')
        else:
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
    (rows, complete). A chunk that failed 'capped' (408 / timeout) is never re-sent as it was: the resume splits it in
    two halves recorded under its key (down to FLOOR cursor values; at the floor: NOT A MEASUREMENT); a transport /
    shed failure is re-sent unchanged after health."""
    os.makedirs(out_dir, exist_ok=True)
    man_p = os.path.join(out_dir, 'manifest.json')
    man = jload(man_p) if os.path.exists(man_p) else {'lo': lo, 'hi': hi, 'width': width, 'chunks': {}}
    if (man['lo'], man['hi'], man['width']) != (lo, hi, width):
        raise SystemExit(f'{out_dir}: an earlier enumeration used ({man["lo"]}, {man["hi"]}], width {man["width"]}; '
                         'use a fresh directory')

    def read(a, b):
        key = f'{a}_{b}'
        fp = os.path.join(out_dir, f'chunk_{key}.json')
        rec = man['chunks'].get(key)
        if rec and rec.get('ok') and os.path.exists(fp) and sha_file(fp) == rec['sha256']:
            return rows_of(jload(fp))
        if rec and not rec.get('ok') and rec.get('class', 'capped') == 'capped':
            m = a + (b - a) // 2
            if (b - a) // 2 < FLOOR:
                man['chunks'][key] = dict(rec, floor=True)
                jdump(man_p, man)
                print(f'{out_dir}: chunk ({a}, {b}] was capped down to the floor ({FLOOR} cursor values): NOT A '
                      'MEASUREMENT')
                return None
            man['chunks'][key] = dict(rec, split=[[a, m], [m, b]])
            jdump(man_p, man)
            r1 = read(a, m)
            if r1 is None:
                return None
            r2 = read(m, b)
            return None if r2 is None else r1 + r2
        body = box.sq(sql_changes(a, b), f'changes_{key}')
        if body is None:
            man['chunks'][key] = _failed(box)
            jdump(man_p, man)
            return None
        sha = jdump(fp, body)
        man['chunks'][key] = {'ok': True, 'at': now_iso(), 'sha256': sha, 'rows': len(body['rows'])}
        jdump(man_p, man)
        return rows_of(body)

    rows, complete = [], True
    a = lo
    while a < hi:
        b = min(a + width, hi)
        got = read(a, b)
        if got is None:
            complete = False
            break
        assert len(got) <= width
        for r in got:
            assert a < int(r['cursor']) <= b, (r, a, b)
        rows += got
        a = b
    man['complete'] = complete and a >= hi
    jdump(man_p, man)
    return rows, man['complete']
