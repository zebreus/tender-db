"""Issue 551 B6 U7: the drain's stop rules D1-D7 (decision "Drain stop rules" + C4/C5/C6, A4), per run.

    python3 -I -B dchecks.py read  <run> --cursor-before N --cursor-after M [--band-route sql|api]
    python3 -I -B dchecks.py check <run> --stamped S --verified V --corrected C [--drift 0] [--now ISO]

read   (box, bounded) after the run's project finished: the project's change rows in (N, M] (N = /health cursor read
       right BEFORE the wet POST, M = right after the project finished; <= 5,000 per chunk) into u7/d5/<run>/changes,
       classified (kitlib.classify_changes: six kinds; removed lots; Tenders retired in the range); PK reads of the
       run's named ids (u7/post/<run>/named), of every pin (u7/post/<run>/pins) and of every live Tender those rows
       name (u7/d5/<run>/rows) -- a Tender retired in the range is expected-absent, not a missing row; the band seek
       (>= EUR 100 m, lot-sum-awards, plan-probed range seek on tenders_current_value_eur) into u7/band/<run>.json. If
       that statement does not answer, it is NOT retried: the band is read over the public API instead (B5's route,
       min_value paging through fetch_b6's stamp), and the box flag stays set until box_health. --band-route api skips
       the /v1/sql statement.
check  (offline) every rule that applies to the run, against u6/freeze.json, u6/P_final.json, u6/l_changed.json and
       the PRE snapshot u6/snapshot/pre (taken before the flip deploy):
  D1 (stage runs)  the named ids of the 3,502 that serve lot-sum-awards == P ∩ run exactly; each with S
                   (current_value_cents), currency and n (detail.lots) equal to P's, vouched_by a MEMBER of P's G-filtered
                   vouches (A4); every Rg id compares identical to the snapshot. Unpredicted fire / wrong S, currency, n,
                   vouched_by / an Rg id moved: SWITCH OFF. A predicted id that does not fire: STOP, explain within the
                   gap, switch off if unexplained by its end.
  D2 (stage runs)  the snapshot diff of the named ids == the P fires exactly (tolerance 0); the job's corrected ==
                   |run ∩ P| + drift (C2's probe drift, 0); a surplus is accepted only when it equals the touched
                   expansion's changed Tenders (D5/D6 judge them, C4); verified + corrected == stamped (D7 complete).
                   A predicted id that did not fire is D1's STOP (explain within the gap): D2 takes it out of its set and
                   its count (want = |exp ∩ fired| + the non-fires whose row moved otherwise + drift) and reports it as
                   STOP, so the run reads STOP, not SWITCH OFF. Any surplus or extra diff stays SWITCH OFF.
  D3 (every run)   the pins (freeze.json pins_expected): 585917 / 5995030 / 8426318 lot-sum-awards at their cents and
                   currency with a vouched_by among the port's vouches; the trio EUR 1,660,661,682.94 no derivation; the
                   16 path-V pins byte-identical to the snapshot and at their recorded sums; the 9 must-stay heads with
                   no derivation and their snapshot EUR value; 6465510 not lot-sum-awards. Any off: SWITCH OFF.
  D4 (every run)   the band seek == B_P ∩ the ids drained so far. A missing head: SWITCH OFF (STOP-explain if it is
                   within 2 % of EUR 100 m). An extra row: STOP and READ it (0 wrong of any class; switch off only if it
                   reads wrong); STOP-explain if within 2 %; an early B_P fire through the touched expansion is a note.
  D5 (every run)   every live Tender with a change row in the project's range is PK-read: any serving lot-sum-awards
                   outside P ∪ L_changed: SWITCH OFF. A named P fire with no tender change row in the range: STOP-explain.
                   A lot that vanished with no removed row, or an unknown kind: STOP. A named id retired: STOP-explain.
  D6 (lcNN runs)   projection = dry admitted × 3,502/251,526; new fires (named or touched, not in P, not fired before)
                   above max(2 × projection, projection + 10): STOP and read before the next run. A new fire at
                   >= EUR 100 m is a delta band row: STOP and READ (0 wrong of any class). A named Tender whose served
                   VALUE columns moved without a new fire is unexplained non-W drift: STOP-explain; a move in seq or
                   published_at only (539's DÖE re-date) is a note. corrected is set against the Tenders with change rows
                   (a note).
  D7 (every run)   the start rule for the runs still to do (start_rule.evaluate, the switch-off gate budgeted): GO ->
                   PASS line; the 2 h margin gone -> STOP (do not start the next run unless re-estimated to fit);
                   the rest could not finish before the latest switch-off start -> SWITCH OFF (before the chain).
Verdict codes (exit): 0 PASS, 10 SWITCH OFF, 2 STOP, 3 NOT A MEASUREMENT (a read incomplete, or a tool fault: any
uncaught exception maps to 3, never to a verdict). Results go to u7/results/<run>.json; u7/state/registry.json keeps
the drained ids, every fire seen and every Tender retired, across runs.
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), 'u6'))
sys.path.insert(1, HERE)
import kitlib as K  # noqa: E402
import start_rule as SR  # noqa: E402

PASS, SWITCH_OFF, STOP, NOT_MEASURED = 0, 10, 2, 3
NAMES = {PASS: 'PASS', SWITCH_OFF: 'SWITCH OFF', STOP: 'STOP', NOT_MEASURED: 'NOT A MEASUREMENT'}
LSA = 'lot-sum-awards'
VAL = ['current_seq', 'current_published_at', 'current_value_derivation', 'current_value_cents',
       'current_value_currency', 'current_value_eur_cents', 'current_value_derivation_detail']


class Ctx:
    def __init__(self, base=HERE, u6=K.U6):
        self.base = base
        self.u6 = u6
        self.freeze = K.jload(os.path.join(u6, 'freeze.json'))
        pf_path = os.path.join(u6, 'P_final.json')
        if K.sha_file(pf_path) != self.freeze['P']['P_final_sha256']:
            raise SystemExit('P_final.json is not the file freeze.json froze')
        self.pf = K.jload(pf_path)
        self.P = {int(e['id']): e for e in self.pf['ships']}
        self.Rg = set(self.freeze['Rg'])
        self.lc = set(K.jload(os.path.join(u6, 'l_changed.json'))['l_changed'])
        self.ids3502 = set(K.the_3502()[0])
        self.pre = K.load_rows(os.path.join(u6, 'snapshot', 'pre'))
        self.stages = K.jload(os.path.join(base, 'stages.json'))
        self.reg_path = os.path.join(base, 'state', 'registry.json')
        self.reg = K.jload(self.reg_path) if os.path.exists(self.reg_path) else {'runs': [], 'drained': [], 'fired': {}}
        self.reg.setdefault('retired', [])
        self.absent = set()                     # ids a post read recorded as expected-absent (retired in the run)

    def run_ids(self, run):
        return K.jload(os.path.join(self.base, 'jobs', f'{run}_ids.json'))['ids']

    def d(self, *p):
        return os.path.join(self.base, *p)


RANK = {PASS: 0, STOP: 1, NOT_MEASURED: 2, SWITCH_OFF: 3}


def worse(a, b):
    """SWITCH OFF > NOT A MEASUREMENT > STOP > PASS."""
    return a if RANK[a] >= RANK[b] else b


def vals(r):
    return None if r is None else [r.get(c) for c in VAL]


def is_lsa(r):
    return r is not None and r.get('current_value_derivation') == LSA


class Out:
    def __init__(self, rule):
        self.rule, self.code, self.lines = rule, PASS, []

    def add(self, code, msg):
        self.lines.append(f'{NAMES[code]}: {msg}')
        self.code = worse(self.code, code)

    def note(self, msg):
        self.lines.append(f'note: {msg}')

    def rec(self):
        return {'rule': self.rule, 'verdict': NAMES[self.code], 'code': self.code, 'lines': self.lines}


# ---------------------------------------------------------------- the rules (pure; tests call them directly)
def d1(ctx, run, named, post):
    o = Out('D1')
    named = set(named)
    exp = named & set(ctx.P)
    fired = {i for i in named if is_lsa(post.get(i))}
    pre_lsa = sorted(i for i in named if is_lsa(ctx.pre.get(i)))
    if pre_lsa:
        o.add(STOP, f'the PRE snapshot already shows lot-sum-awards on {pre_lsa[:10]}: it is not the W=false state')
    for i in sorted(exp):
        r, e = post.get(i), ctx.P[i]
        if r is None:
            if i in getattr(ctx, 'absent', ()):
                o.add(STOP, f'{i} predicted, but RETIRED in the run\'s range (no tenders row): explain within the gap')
            else:
                o.add(NOT_MEASURED, f'{i}: no post read')
            continue
        if i not in fired:
            o.add(STOP, f'{i} predicted (S {e["currency"]} {e["S"]}, n {e["n"]}) did not fire: serves '
                        f'{r.get("current_value_derivation")} eur {r.get("current_value_eur_cents")} -- explain within '
                        'the gap, switch off if unexplained by its end')
            continue
        det = K.detail_of(r)
        bad = []
        if r.get('current_value_cents') != e['S']:
            bad.append(f'S {r.get("current_value_cents")} != {e["S"]}')
        if r.get('current_value_currency') != e['currency']:
            bad.append(f'currency {r.get("current_value_currency")} != {e["currency"]}')
        if det.get('lots') != e['n']:
            bad.append(f'n {det.get("lots")} != {e["n"]}')
        vk = K.vb_key(det.get('vouched_by'))
        if vk is None or vk not in [list(v) for v in e['vouches']]:
            bad.append(f'vouched_by {det.get("vouched_by")} not among the port\'s vouches {e["vouches"]}')
        if bad:
            o.add(SWITCH_OFF, f'{i} fired with ' + '; '.join(bad))
    for i in sorted(fired - exp):
        o.add(SWITCH_OFF, f'{i} serves lot-sum-awards and is not in P (unpredicted fire): {post[i]}')
    for i in sorted(named & ctx.Rg):
        if i in getattr(ctx, 'absent', ()) and post.get(i) is None:
            o.add(STOP, f'Rg id {i} RETIRED in the run\'s range: explain within the gap')
        elif vals(post.get(i)) != vals(ctx.pre.get(i)):
            o.add(SWITCH_OFF, f'Rg id {i} does not compare identical: pre {vals(ctx.pre.get(i))} post {vals(post.get(i))}')
    eq_first = sum(1 for i in exp & fired if K.detail_of(post[i]).get('vouched_by') == ctx.P[i]['vouched_by'])
    o.note(f'{len(exp)} predicted, {len(fired & exp)} fired as predicted; {len(named & ctx.Rg)} Rg ids identical-checked; '
           f'vouched_by equal to the port\'s first on {eq_first} of {len(exp & fired)}')
    return o


def d2(ctx, run, named, post, corrected, verified, stamped, drift, dry_found, changed_by_rows):
    o = Out('D2')
    named = set(named)
    exp_all = named & set(ctx.P)
    nonfire = {i for i in exp_all if not is_lsa(post.get(i))}
    exp = exp_all - nonfire                                  # the P fires D2 counts
    diff = {i for i in named if vals(post.get(i)) != vals(ctx.pre.get(i))}
    nonfire_moved = nonfire & diff                           # a non-fire whose row moved otherwise: the job counts it
    if nonfire:
        o.add(STOP, f'{len(nonfire)} predicted id(s) did not fire -- D1\'s STOP (explain within the gap; switch off if '
                    f'unexplained by its end): {sorted(nonfire)[:10]}; D2 takes them out of its set and count '
                    f'({len(nonfire_moved)} of them moved otherwise and are counted as corrected)')
    # a named id OUTSIDE the 3,502 (a pin) whose value columns are identical and only its date/seq moved is non-W drift
    # (C2's probe covered the 3,502 only; e.g. 539's DÖE re-date): STOP and explain, not a path-W correction
    datedrift = {i for i in diff - exp_all if i not in ctx.ids3502 and post.get(i) is not None
                 and ctx.pre.get(i) is not None and vals(post[i])[2:] == vals(ctx.pre[i])[2:]}
    if datedrift:
        o.add(STOP, f'named pins outside the 3,502 changed only seq/published_at (non-W drift; explain): '
                    f'{sorted(datedrift)[:10]}')
    got = diff - datedrift - nonfire
    if got != exp:
        o.add(SWITCH_OFF, f'snapshot diff of the named ids ({len(got)}, D1\'s non-fires and date-only pins aside) != the '
                          f'P fires ({len(exp)}): extra {sorted(got - exp)[:10]}, missing {sorted(exp - got)[:10]}')
    named_rows = set(changed_by_rows) & named
    if named_rows - exp_all - datedrift:
        o.add(SWITCH_OFF, f'named ids outside P with change rows in the project range (drift, e.g. a lot-only change): '
                          f'{sorted(named_rows - exp_all - datedrift)[:10]}')
    expansion = sorted(set(changed_by_rows) - named)
    want = len(exp) + len(nonfire_moved) + drift
    if corrected != want:
        surplus = corrected - want
        if surplus > 0 and surplus == len(expansion) + len(datedrift & named_rows):
            o.note(f'corrected {corrected} = |fired ∩ P| {len(exp)} + {len(nonfire_moved)} moved non-fires + drift '
                   f'{drift} + {len(expansion)} touched-expansion Tenders (judged by D5/D6, C4) + '
                   f'{len(datedrift & named_rows)} date-drift pins (STOP above): {expansion[:10]}')
        elif surplus < 0:
            o.add(SWITCH_OFF, f'corrected {corrected} < {want} (|fired ∩ P| {len(exp)} + {len(nonfire_moved)} moved '
                              f'non-fires + drift {drift}): fewer corrections than the fires the PK reads show')
        else:
            o.add(SWITCH_OFF, f'corrected {corrected} != |fired ∩ P| {len(exp)} + {len(nonfire_moved)} moved non-fires + '
                              f'drift {drift} (touched expansion with change rows: {len(expansion)})')
    if stamped is not None and dry_found is not None and stamped != dry_found:
        o.add(STOP, f'wet stamped {stamped} != dry admitted {dry_found}')
    if stamped is not None and verified is not None and verified + corrected != stamped:
        o.add(STOP, f'verified {verified} + corrected {corrected} != stamped {stamped} (D7 incomplete)')
    o.note(f'corrected {corrected}, expected {want}; named snapshot diff {len(diff)}; named ids with rows {len(named_rows)}')
    return o


def d3(ctx, pins_post):
    o = Out('D3')
    pe = ctx.freeze['pins_expected']

    def missing(t, what):
        if int(t) in getattr(ctx, 'absent', ()):
            o.add(STOP, f'{what} {t}: RETIRED in the run\'s range (no tenders row): explain')
        else:
            o.add(NOT_MEASURED, f'{what} {t}: no read')

    for t, e in pe['path_w'].items():
        r = pins_post.get(int(t))
        if r is None:
            missing(t, 'pin')
            continue
        det = K.detail_of(r)
        vk = K.vb_key(det.get('vouched_by'))
        ok = (is_lsa(r) and r.get('current_value_cents') == e['cents'] and r.get('current_value_currency') == e['currency']
              and vk is not None and vk in [list(v) for v in (e['accepted_vouches'] or [])])
        if not ok:
            o.add(SWITCH_OFF, f'path-W pin {t} off: {r.get("current_value_derivation")} {r.get("current_value_currency")} '
                              f'{r.get("current_value_cents")} vouched_by {det.get("vouched_by")} (expected '
                              f'{e["currency"]} {e["cents"]} lot-sum-awards, vouched_by among {e["accepted_vouches"]})')
    for t, e in pe['trio'].items():
        r = pins_post.get(int(t))
        if r is None:
            missing(t, 'trio')
        elif r.get('current_value_eur_cents') != e['eur_cents'] or r.get('current_value_derivation') is not None:
            o.add(SWITCH_OFF, f'Lyon-Turin {t} off: eur {r.get("current_value_eur_cents")} derivation '
                              f'{r.get("current_value_derivation")}')
    for t, e in pe['path_v'].items():
        r, p = pins_post.get(int(t)), ctx.pre.get(int(t))
        if r is None or p is None:
            if r is None and int(t) in getattr(ctx, 'absent', ()):
                missing(t, 'path-V')
            else:
                o.add(NOT_MEASURED, f'path-V {t}: no read (post {r is not None}, pre {p is not None})')
            continue
        det = K.detail_of(r)
        if vals(r) != vals(p):
            o.add(SWITCH_OFF, f'path-V {t} not byte-identical: pre {vals(p)} post {vals(r)}')
        elif (r.get('current_value_derivation') != 'lot-sum-vouched' or r.get('current_value_cents') != e['cents']
              or r.get('current_value_currency') != e['currency'] or str(det.get('rule')) != str(e['rule'])):
            o.add(SWITCH_OFF, f'path-V {t} identical to the snapshot but not at its recorded sum '
                              f'({e["currency"]} {e["cents"]} rule {e["rule"]}): {vals(r)}')
    for t in pe['must_stay']:
        r, p = pins_post.get(int(t)), ctx.pre.get(int(t))
        if r is None or p is None:
            missing(t, 'must-stay')
        elif r.get('current_value_derivation') is not None or r.get('current_value_eur_cents') != p.get('current_value_eur_cents'):
            o.add(SWITCH_OFF, f'must-stay {t} off: derivation {r.get("current_value_derivation")} eur '
                              f'{r.get("current_value_eur_cents")} (pre {p.get("current_value_eur_cents")})')
    for t in pe['not_lsa']:
        r = pins_post.get(int(t))
        if r is None:
            missing(t, 'not-lot-sum-awards pin')
        elif is_lsa(r):
            o.add(SWITCH_OFF, f'{t} serves lot-sum-awards (D3: it must not)')
    return o


def d4(ctx, band_body, drained, new_fires=(), reads=None):
    """`reads` (U8 verify --explained): {id: {'verdict': 'right'|'neutral'|'wrong', ...}} for extra band rows read under
    0 wrong of any class -- right/neutral turns that row's STOP into a note, wrong is SWITCH OFF."""
    o = Out('D4')
    reads = reads or {}
    try:
        rows = K.rows_of(band_body)
    except ValueError as e:
        o.add(NOT_MEASURED, f'band seek: {e}')
        return o
    route = band_body.get('route', 'sql')
    if route == 'sql' and len(rows) >= 200:
        o.add(NOT_MEASURED, 'band seek returned its LIMIT (200): not the whole band')
        return o
    seen = {int(r['id']): r for r in rows}
    bp = set(ctx.freeze['B_P'])
    near = set(ctx.freeze['near_eur100m_2pct']) | set(K.NEAR_2PCT_WATCH)
    expected = bp & set(drained)
    for i in sorted(expected - set(seen)):
        if i in near:
            o.add(STOP, f'B_P head {i} missing from the band, within 2 % of EUR 100 m: stop and explain')
        else:
            o.add(SWITCH_OFF, f'B_P head {i} missing from the band')
    for i in sorted(set(seen) - expected):
        r = seen[i]
        eur = r.get('current_value_eur_cents')
        if i in bp:
            o.note(f'B_P head {i} fired before its run (touched expansion): predicted, eur {eur}')
            continue
        rd = reads.get(str(i)) or reads.get(i)
        if rd and rd.get('verdict') in ('right', 'neutral'):
            o.note(f'extra band row {i} read {rd["verdict"]} under 0 wrong of any class ({rd.get("ref", "")})')
            continue
        if rd and rd.get('verdict') == 'wrong':
            o.add(SWITCH_OFF, f'extra band row {i} READ WRONG ({rd.get("ref", "")}): switch off')
            continue
        if i in near or (eur is not None and abs(eur - K.EUR100M) <= 0.02 * K.EUR100M):
            o.add(STOP, f'extra band row {i} within 2 % of EUR 100 m (eur {eur}): stop and explain')
        else:
            tag = 'delta band row (an L_changed fire)' if i in set(new_fires) else 'extra band row'
            eur_s = eur if eur is not None else 'not converted on the API route (>= EUR 100 m by min_value)'
            o.add(STOP, f'{tag} {i} eur {eur_s} {r.get("current_value_currency")} {r.get("current_value_cents")}: '
                        'stop and READ it (0 wrong of any class; switch off only if it reads wrong)')
    o.note(f'band route {route}; band rows {len(rows)}; expected so far {len(expected)} of B_P {len(bp)}')
    return o


def d5(ctx, change_rows, d5_rows, named, run_is_stage):
    o = Out('D5')
    cls = K.classify_changes(change_rows)
    for k in cls['unknown_kinds']:
        o.add(STOP, f'unknown entity_kind {k!r} in the project\'s range')
    if cls['unmapped_lots']['bad']:
        o.add(STOP, f'{len(cls["unmapped_lots"]["bad"])} lot change rows map to no Tender and no later row removes '
                    f'their lot: {cls["unmapped_lots"]["bad"][:10]}')
    changed, tender_rows, retired = cls['live'], cls['tenders'], cls['retired']
    if retired & set(named):
        o.add(STOP, f'named id(s) RETIRED in the project\'s range: {sorted(retired & set(named))[:10]} -- explain')
    if retired - set(named):
        o.note(f'{len(retired - set(named))} Tender(s) retired in the range (not PK-readable; no fire possible): '
               f'{sorted(retired - set(named))[:10]}')
    if cls['no_tender_instant']:
        o.note(f'{len(cls["no_tender_instant"])} lot/result rows at an instant with no tender row (a sweep that moved no '
               f'fact): {cls["no_tender_instant"][:10]}')
    missing = sorted(i for i in changed if i not in d5_rows)
    if missing:
        o.add(NOT_MEASURED, f'{len(missing)} changed Tenders not PK-read: {missing[:10]}')
    allowed = set(ctx.P) | ctx.lc
    for i in sorted(changed):
        r = d5_rows.get(i)
        if is_lsa(r) and i not in allowed:
            o.add(SWITCH_OFF, f'{i} serves lot-sum-awards outside P ∪ L_changed: {r}')
    if run_is_stage:
        for i in sorted(set(named) & set(ctx.P)):
            if i not in tender_rows:
                o.add(STOP, f'P fire {i} has no tender change row in the project\'s range (announcement missing?)')
    expansion = sorted(changed - set(named))
    o.note(f'{len(change_rows)} change rows {cls["by_kind_op"]}, {len(changed)} live Tenders changed, '
           f'{len(expansion)} outside the named list (touched expansion): {expansion[:10]}')
    return o, changed, expansion, retired


def d6(ctx, run, named, post, d5_rows, expansion, dry_admitted, prior_fired):
    o = Out('D6')
    proj = dry_admitted * K.CORPUS_RATE[0] / K.CORPUS_RATE[1]
    cap = max(2 * proj, proj + 10)
    cand = {}
    for i in set(named):
        if is_lsa(post.get(i)):
            cand[i] = post[i]
    for i in expansion:
        if is_lsa(d5_rows.get(i)):
            cand[i] = d5_rows[i]
    new = {i: r for i, r in cand.items() if i not in ctx.P and str(i) not in prior_fired
           and not is_lsa(ctx.pre.get(i))}
    if len(new) > cap:
        o.add(STOP, f'{len(new)} new fires > max(2 × {proj:.1f}, {proj:.1f} + 10) = {cap:.1f}: stop and read before '
                    'the next run')
    band = sorted(i for i, r in new.items() if (r.get('current_value_eur_cents') or 0) >= K.EUR100M)
    for i in band:
        o.add(STOP, f'delta band row {i} (eur {new[i].get("current_value_eur_cents")}): read at 0 wrong of any class')
    o.note(f'dry admitted {dry_admitted}, projection {proj:.2f}, cap {cap:.1f}, new fires {len(new)}: '
           f'{sorted(new)[:20]}')
    return o, new


def lc_other_changes(o, ctx, named, post, new):
    """lc runs: named Tenders whose served row moved WITHOUT a new fire. seq / published_at only (539's DÖE re-date) ->
    note; any VALUE column (derivation, cents, currency, eur, detail) -> STOP-explain: unexplained non-W drift during a
    W-on refold (C2's zero-drift probe covered the 3,502 only)."""
    other = sorted(i for i in named if post.get(i) is not None and vals(post.get(i)) != vals(ctx.pre.get(i))
                   and i not in new)
    date_only = [i for i in other if ctx.pre.get(i) is not None and vals(post[i])[2:] == vals(ctx.pre[i])[2:]]
    value_moved = [i for i in other if i not in date_only]
    if date_only:
        o.note(f'{len(date_only)} named Tenders changed only seq/published_at (non-W re-date, e.g. 539\'s DÖE): '
               f'{date_only[:10]}')
    if value_moved:
        o.add(STOP, f'{len(value_moved)} named Tenders whose served VALUE columns moved without a new lot-sum-awards fire '
                    f'(unexplained non-W drift; explain before the next run): {value_moved[:10]}')
    return other


def d7(ctx, run, now):
    """D7 / start rule after this run: the runs still to do, with the switch-off gate budgeted (start_rule)."""
    o = Out('D7')
    done = {r['run'] for r in ctx.reg['runs']} | {run}
    left = [r for r in ctx.stages['order'] if r not in done]
    ev = SR.evaluate(len(left), now)
    o.note(SR.line(ev, len(left)))
    if left and not ev['go']:
        if ev['finish_rest'] > ev['switchoff_by']:
            o.add(SWITCH_OFF, f'the {len(left)} run(s) left {left} cannot finish before the latest switch-off start '
                              f'{ev["switchoff_by"]:%Y-%m-%dT%H:%MZ}: switch off now (§S) so it completes before the '
                              f'{ev["chain"]:%Y-%m-%dT%H:%MZ} chain (D7)')
        else:
            o.add(STOP, f'the 2 h margin is gone for the {len(left)} run(s) left {left}: do not start the next run unless '
                        f're-estimated to finish by {ev["deadline"]:%Y-%m-%dT%H:%MZ}; otherwise switch off (§S) before '
                        f'{ev["switchoff_by"]:%Y-%m-%dT%H:%MZ}')
    return o


# ---------------------------------------------------------------- read + check drivers
def cmd_read(a, fake=None, ctx=None, http=None):
    ctx = ctx or Ctx()
    box = K.shared_box(fake)
    ids = ctx.run_ids(a.run)
    rows, complete = K.enumerate_changes(box, a.cursor_before, a.cursor_after, ctx.d('d5', a.run, 'changes'))
    if not complete:
        print(f'{a.run}: the change enumeration is incomplete (box_health, then re-run this read)')
        return NOT_MEASURED
    cls = K.classify_changes(rows)
    retired = cls['retired'] | set(ctx.reg.get('retired', []))   # this range's retirements and earlier runs'
    _r, f, m = K.pk_read(box, ids, f'{a.run}_named', ctx.d('post', a.run, 'named'), expected_absent=retired)
    worst = NOT_MEASURED if (f or m) else PASS
    if not f:
        _r, f, m = K.pk_read(box, K.pin_ids(), f'{a.run}_pins', ctx.d('post', a.run, 'pins'), expected_absent=retired)
        worst = NOT_MEASURED if (f or m) else worst
    if not f:
        changed = sorted(cls['live'])
        if changed:
            _r, f, m = K.pk_read(box, changed, f'{a.run}_d5', ctx.d('d5', a.run, 'rows'))
            worst = NOT_MEASURED if (f or m) else worst
        else:
            K.jdump(ctx.d('d5', a.run, 'rows', 'rows.json'), {})
            K.jdump(ctx.d('d5', a.run, 'rows', 'manifest.json'), {'complete': True, 'n_ids': 0})
    if not f:
        body, route = K.read_band(box, f'{a.run}_band', ctx.d('band', f'{a.run}.json'), route=a.band_route, http=http)
        if body is None:
            return NOT_MEASURED
    K.jdump(ctx.d('post', a.run, 'cursors.json'), {'cursor_before': a.cursor_before, 'cursor_after': a.cursor_after,
                                                   'at': K.now_iso(), 'retired_in_range': sorted(cls['retired'])})
    print(f'{a.run}: reads done (exit {worst}); change rows {len(rows)} {cls["by_kind_op"]}; retired in the range '
          f'{sorted(cls["retired"])[:10]}')
    return worst


def cmd_check(a, ctx=None):
    ctx = ctx or Ctx()
    run = a.run
    named = ctx.run_ids(run)
    is_stage = run.startswith('stage')
    outs = []
    try:
        post = K.load_rows(ctx.d('post', run, 'named'))
        pins_post = K.load_rows(ctx.d('post', run, 'pins'))
        ctx.absent = K.absent_of(ctx.d('post', run, 'named')) | K.absent_of(ctx.d('post', run, 'pins'))
        d5_rows = K.load_rows(ctx.d('d5', run, 'rows'))
        cur = K.jload(ctx.d('post', run, 'cursors.json'))
        man = K.jload(ctx.d('d5', run, 'changes', 'manifest.json'))
        if not man.get('complete'):
            raise SystemExit('change enumeration incomplete')
        change_rows = []
        for key, rec in sorted(man['chunks'].items(), key=lambda kv: int(kv[0].split('_')[0])):
            change_rows += K.rows_of(K.jload(ctx.d('d5', run, 'changes', f'chunk_{key}.json')))
        band = K.jload(ctx.d('band', f'{run}.json'))
    except (SystemExit, OSError) as e:
        print(f'{run}: NOT A MEASUREMENT: {e}')
        return NOT_MEASURED
    dry = K.jload(ctx.d('jobs', f'{run}_dry_result.json')) if os.path.exists(ctx.d('jobs', f'{run}_dry_result.json')) else {}
    o5, changed, expansion, retired = d5(ctx, change_rows, d5_rows, named, is_stage)
    if is_stage:
        outs.append(d1(ctx, run, named, post))
        outs.append(d2(ctx, run, named, post, a.corrected, a.verified, a.stamped, a.drift, dry.get('found'),
                       changed))
    outs.append(d3(ctx, pins_post))
    new = {}
    if is_stage:
        exp_fires = {i: d5_rows[i] for i in expansion if is_lsa(d5_rows.get(i)) and i not in ctx.P
                     and str(i) not in ctx.reg['fired']}
        if exp_fires:
            o6 = Out('D6 (touched expansion of a stage run)')
            o6.note(f'{len(exp_fires)} new fires in the touched expansion (each in L_changed, else D5 switched off): '
                    f'{sorted(exp_fires)[:20]}; they join the U8 delta population')
            for i, r in sorted(exp_fires.items()):
                if (r.get('current_value_eur_cents') or 0) >= K.EUR100M:
                    o6.add(STOP, f'delta band row {i} (eur {r.get("current_value_eur_cents")}): read at 0 wrong of any class')
            outs.append(o6)
            new = exp_fires
    if not is_stage:
        if not dry:
            print(f'{run}: NOT A MEASUREMENT: no dry result (make_wet.py records the admitted count)')
            return NOT_MEASURED
        o6, new = d6(ctx, run, named, post, d5_rows, expansion, dry['found'], ctx.reg['fired'])
        lc_other_changes(o6, ctx, named, post, new)
        o6.note(f'corrected {a.corrected} vs {len(changed)} live Tenders with change rows in the range '
                f'({len(set(changed) & set(named))} named, {len(expansion)} touched expansion)')
        outs.append(o6)
    drained = sorted(set(ctx.reg['drained']) | set(named))
    outs.append(d4(ctx, band, drained, new))
    outs.append(o5)
    outs.append(d7(ctx, run, SR.parse_now(a.now)))
    worst = PASS
    for o in outs:
        worst = worse(worst, o.code)
    res = {'run': run, 'at': K.now_iso(), 'cursors': cur, 'job': {'stamped': a.stamped, 'verified': a.verified,
                                                                  'corrected': a.corrected, 'drift': a.drift},
           'verdict': NAMES[worst], 'code': worst, 'rules': [o.rec() for o in outs],
           'new_fires': sorted(new), 'expansion': expansion, 'retired': sorted(retired),
           'dry_admitted': dry.get('found')}
    K.jdump(ctx.d('results', f'{run}.json'), res)
    fired = dict(ctx.reg['fired'])
    for i, r in list(post.items()) + list(d5_rows.items()):
        if is_lsa(r) and str(i) not in fired:
            fired[str(i)] = {'run': run, 'eur': r.get('current_value_eur_cents'), 'in_P': i in ctx.P,
                             'in_l_changed': i in ctx.lc, 'in_3502': i in ctx.ids3502}
    ctx.reg = {'runs': ctx.reg['runs'] + [{'run': run, 'verdict': NAMES[worst]}], 'drained': drained, 'fired': fired,
               'retired': sorted(set(ctx.reg.get('retired', [])) | retired)}
    K.jdump(ctx.reg_path, ctx.reg)
    for o in outs:
        print(f'{o.rule}: {NAMES[o.code]}')
        for ln in o.lines:
            print(f'   {ln}')
    print(f'{run}: {NAMES[worst]}')
    return worst


def main(argv=None, fake=None, ctx=None, http=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    r = sub.add_parser('read')
    r.add_argument('run')
    r.add_argument('--cursor-before', type=int, required=True)
    r.add_argument('--cursor-after', type=int, required=True)
    r.add_argument('--band-route', choices=['sql', 'api'], default='sql')
    c = sub.add_parser('check')
    c.add_argument('run')
    c.add_argument('--stamped', type=int)
    c.add_argument('--verified', type=int)
    c.add_argument('--corrected', type=int, required=True)
    c.add_argument('--drift', type=int, default=0)
    c.add_argument('--now', help='test only: the clock D7 reads')
    a = ap.parse_args(argv)
    if a.cmd == 'read':
        return cmd_read(a, fake, ctx, http)
    return cmd_check(a, ctx)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
