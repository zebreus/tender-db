"""Issue 551, path W on: the W-on drains kit's shared plumbing (b6/decision-w-on-drains.md, sections B-F, W1-W15,
W13'). Every u9/*.py imports it; it adds nothing to the box contract of u6/kitlib.py (one paced, plan-probed bare
SELECT at a time, a non-answer never retried, exit 3 = NOT A MEASUREMENT).

  - records: C1 (the flip cursor), K0 (R0's cursor), R0 and its archives (u9/20261023/), the U9 rate 8/729 (C8),
    the pins by role, 579's six order-only Tenders (W1);
  - the W box (B5): its own state dir u9/box, >= 2.5 s apart, plan-probed on plan_w.db (E1's index;
    kitlib.ensure_plan_db_w), NO lifetime cap (B5 sets a rate only; a warning past section 5's 560, recorded in 551).
    The rolling hour counts every kit box (kitlib.kit_state_dirs): segment readers stop at 125/h, so the switch-off's
    own box (u7/switchoff/box, 150/h) always keeps >= 25/h -- both inside B5's 150 (the endpoint's 300 is shared);
  - changed_since(): the change rows of a cursor range by E1 (kitlib.tender_keyset) or, for spans <= 50k cursor
    values, the all-kind reader (kitlib.enumerate_changes + classify_changes, so D5's hygiene checks stay); 'both'
    runs the two side by side and records their parity (P0, W12). E1 is used only once W12 is discharged
    (e1_proven: u9/replay/replay.json PASS and u9/confirm_w12a.json index_exists): before that 'auto' reads all-kind
    and an explicit 'e1' is refused (the switch-off degrades instead of refusing); an explicit 'e1' on a span <= 50k
    is always refused;
  - the comparators gd5 and registry share: W-column moves of an R head (E2 MOVED incl. detail, W15), value-column
    diffs of a watched row (SILENT, PINS on value columns only, W5/W6), date-only moves (NOTE);
  - the archive readers (tar.xz members read in memory; nothing is extracted).
"""
import collections
import datetime
import json
import os
import re
import sys
import tarfile

sys.dont_write_bytecode = True
U9 = os.path.dirname(os.path.abspath(__file__))
B6 = os.path.dirname(U9)
sys.path.insert(0, os.path.join(B6, 'u6'))
sys.path.insert(1, os.path.join(B6, 'u7'))
import kitlib as K  # noqa: E402

LSA = 'lot-sum-awards'
PASS, SWITCH_OFF, STOP, NOT_MEASURED = 0, 10, 2, 3
NAMES = {PASS: 'PASS', SWITCH_OFF: 'SWITCH OFF', STOP: 'STOP', NOT_MEASURED: 'NOT A MEASUREMENT'}
RANK = {PASS: 0, STOP: 1, NOT_MEASURED: 2, SWITCH_OFF: 3}

# ---------------------------------------------------------------- records (each from issue 551's U7-U9 entries)
C1 = 959004090                  # the flip cursor (U7.2)
K0 = 959180852                  # R0's cursor: /health at the 10-23 days-1-2 read (u6/health.log U9-1023)
U7_END = 959026689              # lc04's CA: U7's drain ends here; the days-1-2 chains are (U7_END, K0]
R0_DIR = os.path.join(U9, '20261023')
R0_FILE = os.path.join(R0_DIR, 'switchoff.json')
R0_IDS_SHA = '8845f0d03f91defccf46a7275a5ead4d13dc84a44a2501155202562365f3ae20'
R0_N = 3478
ARCH_STAMP = '2026-10-23T184801Z'
ARCH_CHANGES = os.path.join(R0_DIR, f'build_changes_{ARCH_STAMP}.tar.xz')
ARCH_CAND = os.path.join(R0_DIR, f'cand_{ARCH_STAMP}.tar.xz')
ARCH_CHANGED = os.path.join(R0_DIR, f'changed_{ARCH_STAMP}.tar.xz')
BAND_R0 = os.path.join(R0_DIR, 'band_20261023.json')
DAILY_FIRES_R0 = (97901, 8859070, 8859315)       # 551 U9 entry, days 1-2
RATE = (8, 729)                                  # C8: U9's count baseline, 1.0974 % (u8 verify_result.json)
NOTE_579 = (408007, 413501, 454861, 179248, 340956, 823340)   # W1: 579's six open Tenders, order-only NOTEs
PATH_W_PINS = tuple(sorted(K.PATH_W))            # 585917, 5995030, 8426318
PARTIAL_SUMS = (6679375, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260)
DOUBLE_COUNT_PIN = 6908248
STOP_EXPLAIN_PINS = tuple(sorted(set(K.PATH_V) | set(K.TRIO) | set(K.MUST_STAY)))   # W5: path-V, trio, must-stay
NOT_LSA_PINS = tuple(K.NOT_LSA)                  # 6465510
NEAR = tuple(K.NEAR_2PCT_WATCH)                  # 899272, 5842187: within 2 % of EUR 100 m
FRESH_FX_DAYS = 8                                # rederive-eur-recent's window (chain NOTE, W4)
SEG_CLASSES = ('D', 'Z', 'N', 'W', 'I')
CHAIN_CLASSES = ('chain', 'weekly')
CLASSES = CHAIN_CLASSES + SEG_CLASSES
READ_CAP = 30                                    # E: > 30 NEW in N/Z/D/weekly = the class premise is broken

U6 = os.path.join(B6, 'u6')
U8 = os.path.join(B6, 'u8')
FREEZE = os.path.join(U6, 'freeze.json')
P_FINAL = os.path.join(U6, 'P_final.json')
DELTA_READ_SET = os.path.join(U8, 'delta_read_set.json')
DELTA_READ_RESULT = os.path.join(U8, 'delta_read_result.json')
REG_DIR = os.path.join(U9, 'registry')
REG_FILE = os.path.join(REG_DIR, 'R.json')
SEG_DIR = os.path.join(U9, 'segments')
AJOBS_DIR = os.path.join(U9, 'ajobs')
BOX_W_DIR = os.path.join(U9, 'box')
SWITCHOFF_BOX_DIR = os.path.join(B6, 'u7', 'switchoff', 'box')
W_WARN = 560                    # section 5's upper estimate (270-560): a warning recorded in 551, never a refusal
W_HOURLY = 150                  # B5: the switch-off box's cap (over every kit box's statements in the hour)
W_HOURLY_READERS = 125          # segment readers (gd5, replay, day7): leaves >= 25/h to a switch-off, inside B5's 150
K0_AT = K.epoch_of('2026-10-23T18:48:01Z')   # K0's /health read (u6/health.log U9-1023): R0's jobs floor (B1)
REPLAY_JSON = os.path.join(U9, 'replay', 'replay.json')     # W12 (b)
CONFIRM_JSON = os.path.join(U9, 'confirm_w12a.json')       # W12 (a)
STOP_EXPLAIN_ROLES = ('path-V', 'trio', 'must-stay')       # W5
STEP2_TAGS = ('drain-539-a', 'drain-539-b', 'drain-539-c')  # 539 step 2 (W); step 1 (drain-539-1, class Z) refolds nothing
DAY7_DATE = '2026-10-28'                                   # B1/F: no writer on this Berlin date until the day-7 fetch
# In-place W-input writers (C9' / G; 579: repoint_org_references stamps nothing): org identity feeds R1/R6. One of these
# landing without its pre-step (a C9' prediction + a paired refold) is F's SWITCH OFF.
W_INPUT_INPLACE_KINDS = ('merge-provisional-orgs', 'match-org-identifiers', 'repair-nested-orgs', 'dissolve-guid-twins',
                         'apply-rehoming', 'fold-provisional-echoes', 'apply-case-reviews', 'unapply-case-reviews',
                         'repair-placeholder-orgs', 'repair-member-twins', 'restore-dropped-satellites')
DRY_RX = re.compile(r'^refold-lot-sum \(issue 551\) DRY RUN\s*\S\s*nothing written')
CORRECTED_RX = re.compile(r'(\d+)\s+corrected\b')


def worse(a, b):
    return a if RANK[a] >= RANK[b] else b


class Out:
    """A rule's verdict lines. add(..., esc=True) marks a STOP that F / W4 turn into SWITCH OFF when it is still
    unexplained at the end of the gap (MOVED/LOST/SILENT in D/Z/N/W/I; an unannounced SILENT or the 3x count in a
    chain): registry.py status escalates a freeze made only of such STOPs once the gap ends."""

    def __init__(self, rule):
        self.rule, self.code, self.lines, self.stop_esc = rule, PASS, [], []

    def add(self, code, msg, esc=False):
        self.lines.append(f'{NAMES[code]}: {msg}')
        self.code = worse(self.code, code)
        if code == STOP:
            self.stop_esc.append(bool(esc))

    def note(self, msg):
        self.lines.append(f'note: {msg}')

    def rec(self):
        return {'rule': self.rule, 'verdict': NAMES[self.code], 'code': self.code, 'lines': self.lines}


def w_box(fake=None, log=print, state_dir=None, budget=None, hourly=None):
    """The W-protocol box (B5): own state dir, >= 2.5 s apart, E1-aware plan db, no lifetime cap unless `budget` is
    given (confirm_w12a's 3), a warning past W_WARN. hourly: W_HOURLY_READERS for u9/box, W_HOURLY for the
    switch-off box (switchoff_box). `fake` (tests) is an object with .answer(sql) and is never the network."""
    d = state_dir or BOX_W_DIR
    b = budget if budget is not None else float('inf')
    h = hourly or (W_HOURLY if os.path.realpath(d) == os.path.realpath(SWITCHOFF_BOX_DIR) else W_HOURLY_READERS)
    plan_db = K.ensure_plan_db_w()
    if fake is not None:
        return K.Box(d, b, transport=K._fake_transport(fake), gap=0.0, hourly=h, log=log, plan_db=plan_db,
                     warn_at=W_WARN)
    return K.Box(d, b, hourly=h, log=log, plan_db=plan_db, warn_at=W_WARN)


def switchoff_box(fake=None, log=print):
    """The switch-off's own box (both routes, build and verify): u7/switchoff/box, 150/h counted over every kit box,
    no lifetime cap, plan_w.db -- never u6/box's remaining 300 and never the readers' budget."""
    return w_box(fake, log, state_dir=SWITCHOFF_BOX_DIR, hourly=W_HOURLY)


def e1_proven():
    """W12 discharged: (a) prod has changes_entity_cursor (u9/confirm_w12a.json index_exists) and (b) the replay over
    (C1, K0] PASSED (u9/replay/replay.json pass and tender_rows_equal, its E1 manifest complete)."""
    try:
        rp = K.jload(REPLAY_JSON)
        cf = K.jload(CONFIRM_JSON)
        man = K.jload(os.path.join(os.path.dirname(REPLAY_JSON), 'e1', 'manifest.json'))
    except (OSError, ValueError):
        return False
    return bool(rp.get('pass') and rp.get('tender_rows_equal') and man.get('complete') and cf.get('index_exists'))


def is_dry_job(j):
    """A refold-lot-sum dry run (the A / A_d jobs, a probe's or drain's dry post): it writes nothing. No other dry run
    is exempt (repair-notice-instants' dry run writes its mechanical rows as walked)."""
    return (j.get('kind') == 'refold-lot-sum' and j.get('outcome') in ('ok', None)
            and bool(DRY_RX.match(j.get('counts') or '')))


def is_w_input_kind(kind):
    k = kind or ''
    return k in W_INPUT_INPLACE_KINDS or k.startswith(('merge-', 'dissolve', 'rehom', 'repoint'))


def parse_corrected(counts):
    """A job's corrected count from its saved counts ('compare (issue 495): N tenders verified, M corrected'), or None."""
    m = CORRECTED_RX.search(counts or '')
    return int(m.group(1)) if m else None


def plan_ready():
    """-> [problems] (empty: ready). 0 statements: the plan-probe copy, plan.db and plan_w.db exist and every statement
    shape the W kit and the switch-off send plans clean (E1 SEARCHes changes_entity_cursor on plan_w.db; the PK read,
    the band and the all-kind chunk on both dbs). A missing probe or plan db would refuse EVERY statement, the
    switch-off's included, so it is checked before every window (registry.py status)."""
    probs = []
    try:
        K.ensure_plan_probe()
        K.ensure_plan_db()
        K.ensure_plan_db_w()
        wd = os.path.join(K.PLAN_DIR, 'ready')
        steps = K.plan_check(K.sql_tender_keyset(C1, K0), 'ready_e1', wd, K.PLAN_DB_W)
        if not any('changes_entity_cursor' in st for st in steps):
            probs.append(f'E1 does not SEARCH changes_entity_cursor on plan_w.db: {steps}')
        for db in (K.PLAN_DB_W, None):
            K.plan_check(K.sql_tenders_pk([1, 2]), 'ready_pk', wd, db)
            K.plan_check(K.sql_band(), 'ready_band', wd, db)
            K.plan_check(K.sql_changes(C1, C1 + 5000), 'ready_changes', wd, db)
    except (K.PlanRefused, OSError) as e:
        probs.append(f'{type(e).__name__}: {e}')
    return probs


def box_used(box):
    return box._state().get('used', 0)


# ---------------------------------------------------------------- archives (read in memory)
def tar_json(path, suffix):
    """-> {member name: parsed JSON} for members ending with `suffix` (e.g. '/rows.json', '.json')."""
    out = {}
    with tarfile.open(path, 'r:xz') as t:
        for m in t.getmembers():
            if m.isfile() and m.name.endswith(suffix):
                out[m.name] = json.load(t.extractfile(m))
    return out


def archived_changes(path=None):
    """The archived all-kind enumeration (C1, K0] -> rows sorted by cursor (every chunk's body is an answer)."""
    rows = []
    for name, body in tar_json(path or ARCH_CHANGES, '.json').items():
        if '/chunk_' in name:
            rows += K.rows_of(body)
    rows.sort(key=lambda r: int(r['cursor']))
    return rows


def archived_pk(path):
    """An archived pk_read dir -> {int id: row}; refuses an incomplete read."""
    js = tar_json(path, '.json')
    man = next(v for k, v in js.items() if k.endswith('/manifest.json'))
    if not man.get('complete'):
        raise SystemExit(f'{path}: the archived PK read is incomplete')
    rows = next(v for k, v in js.items() if k.endswith('/rows.json'))
    return {int(k): v for k, v in rows.items()}


# ---------------------------------------------------------------- the change rows of a range
def changed_since(box, lo, hi, out_dir, route='auto', label='seg', degrade=False):
    """The change rows in (lo, hi]. route: 'auto' (all-kind if hi - lo <= 50k, else E1), 'e1', 'all', 'both'.
    E1 preconditions (C/E1, W12): a span <= 50k always uses the all-kind reader (an explicit 'e1' there is refused);
    before W12 is discharged (e1_proven) 'auto' resolves to 'all' (recorded as e1_unproven) and an explicit 'e1' is
    refused ('both' is allowed: the all-kind reader stays authoritative and the parity is checked). degrade=True (the
    switch-off, which must never be refused) maps a refused 'e1' to 'all' with a NOTE instead.
    -> dict: route, complete, tenders (ids with a tender row), retired, live, lsa (E1: live ids whose PK subquery reads
    lot-sum-awards; None on the all-kind route), cls (the all-kind classification or None), parity (route 'both')."""
    small = hi - lo <= K.E1_SMALL_SPAN
    proven = e1_proven()
    asked, notes = route, []
    if route == 'auto':
        route = 'all' if small or not proven else 'e1'
    elif route == 'e1' and (small or not proven):
        why = (f'a span of {hi - lo} <= {K.E1_SMALL_SPAN} cursor values always uses the all-kind reader (D5 hygiene)'
               if small else 'E1 is not proven yet (W12: u9/replay/replay.json PASS and u9/confirm_w12a.json)')
        if not degrade:
            raise SystemExit(f'--route e1 refused: {why}; use --route all (or both)')
        notes.append(f'NOTE: --route e1 degraded to all: {why}')
        route = 'all'
    for n in notes:
        print(n)
    res = {'lo': lo, 'hi': hi, 'route': route, 'complete': True, 'cls': None, 'lsa': None, 'parity': None,
           'e1': None, 'route_asked': asked, 'e1_unproven': not proven, 'notes': notes}
    if lo == hi:
        res.update(tenders=set(), retired=set(), live=set(), lsa=set(), by_op={}, n_rows=0)
        return res
    if route in ('e1', 'both'):
        rows, ok = K.tender_keyset(box, lo, hi, os.path.join(out_dir, 'e1'), label=f'{label}_e1')
        if not ok:
            res['complete'] = False
            return res
        c = K.classify_tender_rows(rows)
        res['e1'] = {'n_rows': c['n_rows'], 'by_op': c['by_op']}
        res.update(tenders=c['tenders'], retired=c['retired'], live=c['live'], lsa=c['lsa'])
    if route in ('all', 'both'):
        crow, ok = K.enumerate_changes(box, lo, hi, os.path.join(out_dir, 'all'))
        if not ok:
            res['complete'] = False
            return res
        cls = K.classify_changes(crow)
        res['cls'] = cls
        if route == 'all':
            res.update(tenders=cls['tenders'], retired=cls['retired'], live=cls['live'])
        else:
            res['parity'] = {'tenders_equal': cls['tenders'] == res['tenders'],
                             'retired_equal': cls['retired'] == res['retired'],
                             'only_e1': sorted(res['tenders'] - cls['tenders'])[:20],
                             'only_all': sorted(cls['tenders'] - res['tenders'])[:20],
                             'lot_only_live': len(cls['live'] - res['tenders'])}
            res['live'] = res['live'] | cls['live']          # lot-only Tenders are read too (the all-kind superset)
    return res


def jsonable_changed(ch):
    """changed_since's dict with sets as sorted lists (for read.json)."""
    out = {}
    for k, v in ch.items():
        if k == 'cls' and v is not None:
            out[k] = {'by_kind_op': v['by_kind_op'], 'unknown_kinds': v['unknown_kinds'],
                      'unmapped_lots': v['unmapped_lots'], 'no_tender_instant': v['no_tender_instant'][:50],
                      'n_no_tender_instant': len(v['no_tender_instant'])}
        elif isinstance(v, set):
            out[k] = sorted(v)
        else:
            out[k] = v
    return out


# ---------------------------------------------------------------- comparators (E2, W5, W6, W15)
VALUE_COLS = ['current_value_derivation', 'current_value_cents', 'current_value_currency', 'current_value_eur_cents',
              'current_value_derivation_detail']
DATE_COLS = ['current_seq', 'current_published_at']


def lsa(r):
    return r is not None and r.get('current_value_derivation') == LSA


def detail_rest(r):
    d = dict(K.detail_of(r))
    d.pop('vouched_by', None)
    return d


def w_moves(entry, row):
    """An R head still serving lot-sum-awards against its registry entry -> the W columns that moved: S (cents),
    currency, n (detail.lots), eur, detail (every other detail key, e.g. summed; W15) and vouched_by (membership of the
    entry's recorded vouches, A4)."""
    reg = entry['row']
    out = []
    if row.get('current_value_cents') != reg.get('current_value_cents'):
        out.append('S')
    if row.get('current_value_currency') != reg.get('current_value_currency'):
        out.append('currency')
    dr, dn = K.detail_of(reg), K.detail_of(row)
    if dn.get('lots') != dr.get('lots'):
        out.append('n')
    if row.get('current_value_eur_cents') != reg.get('current_value_eur_cents'):
        out.append('eur')
    rr, rn = detail_rest(reg), detail_rest(row)
    rr.pop('lots', None)
    rn.pop('lots', None)
    if rr != rn:
        out.append('detail')
    vk = K.vb_key(dn.get('vouched_by'))
    if vk is None or vk not in [list(v) for v in entry.get('vouches') or []]:
        out.append('vouched_by')
    return out


def value_diff(reg_row, row):
    """Value columns (derivation, cents, currency, eur, detail -- detail compared parsed) that differ."""
    out = []
    for c in VALUE_COLS:
        a, b = reg_row.get(c), row.get(c)
        if c == 'current_value_derivation_detail':
            a, b = (K.detail_of(reg_row) if a is not None else None), (K.detail_of(row) if b is not None else None)
        if a != b:
            out.append(c)
    return out


def date_moved(reg_row, row):
    return [c for c in DATE_COLS if reg_row.get(c) != row.get(c)]


def seq_increased(reg_row, row):
    return (row.get('current_seq') or 0) > (reg_row.get('current_seq') or 0)


def eur_ge_100m(r):
    return (r.get('current_value_eur_cents') or 0) >= K.EUR100M


def near_100m(i, eur):
    return i in NEAR or (eur is not None and abs(eur - K.EUR100M) <= 0.02 * K.EUR100M)


def fresh_fx(row, at_epoch):
    """W4's chain NOTE: a non-EUR head whose current_published_at is within rederive-eur-recent's 8-day window."""
    pub = row.get('current_published_at')
    return (row.get('current_value_currency') not in (None, 'EUR') and pub is not None
            and at_epoch - pub <= FRESH_FX_DAYS * 86400)


def cap_of(a):
    """D.3: cap = max(2 * A * r, A * r + 10), r = 8/729 (a tripwire, not an expectation)."""
    x = a * RATE[0] / RATE[1]
    return max(2 * x, x + 10), x


def day_of(epoch):
    return datetime.datetime.fromtimestamp(int(epoch), datetime.timezone.utc).strftime('%Y-%m-%d')


def counts(items):
    return dict(sorted(collections.Counter(items).items()))
