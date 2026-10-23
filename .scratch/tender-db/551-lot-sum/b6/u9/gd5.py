"""Issue 551, path W on: G-D5, the per-segment check (decision-w-on-drains.md D, E2; verdicts F and W4's chain table).

    python3 -I -B gd5.py read  <seg> --k K --cursor-after CA --class C --jobs FILE [--route auto|e1|all|both] [--full]
                                     [--band-route sql|api] [--now ISO]
    python3 -I -B gd5.py check <seg> --k K --cursor-after CA --class C [--corrected N] [--a-from FILE]
                                     [--explained FILE] [--prediction FILE] [--gap-end ISO] [--now ISO]
                                     [--runs-left N --minutes-per-run M --read-minutes R --switchoff-gate G]

C is chain | weekly | D (probe) | Z (zero corrections by design) | N (non-W, corrections) | W (W-input) | I (in-place
W-input writer). K must be the registry's checkpoint (u9/registry/R.json) and CA the segment record's CA
(segment.py close / chain / weekly / gap).

read (box, B5: through the W box, at most 125/h for readers, never during a job)
  --jobs FILE: a fresh /admin/jobs body (measured within 10 min): idle, and no job since the segment's CA snapshot
     except refold-lot-sum dry runs (writes after CA would leak into the PK reads).
  Z and weekly, without --full: the zero-change shortcut (D.2): nothing to read when K == CA, else ONE E1 statement
     over (K, CA] (a page holding any tender row is already the STOP). Never for a gap record, a record with a foreign
     or missing own job (D.2's third condition), and never with E1 unproven and K != CA: the full read, all-kind.
  every other read, and Z/weekly with --full:
   1. the change rows in (K, CA]: E1 (kitlib.tender_keyset, `changes_entity_cursor`, plan-probed on plan_w.db) for
      spans > 50k cursor values once W12 is discharged (wkit.e1_proven); the all-kind reader otherwise (D5 hygiene:
      unknown kinds, vanished lots, retirements); --route both runs the two side by side (P0) and records parity;
   2. PK reads of the NEW candidates; 3. PK reads of R ∪ Rg ∪ pins (resumable per chunk);
   4. the band seek (kitlib.read_band: the /v1/sql range seek; the public API if it does not answer -- never a retry);
   5. the changed live ids (segments/<seg>/read/changed_live_ids.json) for the dry A job (ajobs.py).
check (offline) classifies against the registry (section F for D/Z/N/W/I, W4's own table for chain/weekly):
  NEW     serves lot-sum-awards, not in R. chain: the daily fires, counted and registered chain-<date>; the 3x rule on
          A_d (unexplained above 3x at the end of the gap: switch off). W: registered read-pending; over cap = max(2 A r,
          A r + 10) -> STOP (explainable: `cap`). N/Z/D/weekly: expected 0, each a STOP until read or explained; > 30 =
          the class premise is broken (programme STOP; `class_premise` + the salted 30 by the segment's pre-registered
          salt: a NEW outside the draw is a note, a drawn NEW needs its read). I: each NEW must be the port's
          prediction (D1 rules). A NEW with no tender row in the range is SILENT. A path-V / trio / must-stay pin that
          starts serving lot-sum-awards is a value move on a W5 head: stop and explain (never a daily fire or a cap
          count).
  MOVED   in R, still lot-sum-awards, but S, currency, n, eur, detail (W15) differ, or vouched_by is not among the
          entry's vouches -- judged only when the Tender has a tender row in the range. LOST: no longer lot-sum-awards.
          D/Z/N/W/I: STOP (a probe: "moves a W column"); read wrong, or unexplained at the end of the gap: SWITCH OFF.
          chain: a counted daily re-decision -- but a B_R head, a path-W pin, a partial sum or 6908248 that moves is
          stop-and-read at 0 wrong (wrong: SWITCH OFF); a path-V / trio / must-stay head is stop-and-explain.
  SILENT  an R, Rg or pin row whose value columns (derivation, cents, currency, eur, the parsed detail with vouched_by)
          differ with NO tender row in the range (seq/published_at ignored, W6). A derivation/detail move with a seq
          increase is an announcement gap: in a chain STOP (explain, file) and never a switch-off (W4); in D/Z/N/W/I a
          SILENT like any other. Any other: an unannounced writer, STOP. Unexplained at the end of the gap: SWITCH OFF
          (every SILENT in D/Z/N/W/I; the non-gap SILENT in a chain).
  NOTE    seq/published_at only; with a tender row only: an eur-only move on a re-dated head (D/Z/N/W/I) or, in a chain,
          on a non-EUR head published within rederive-eur-recent's 8 days (fresh FX) -- a legitimate EUR move always
          writes a tender row (A2; rederive's D5 rows); 579's six order-only Tenders (W1).
  PINS    against the registry row on value columns only (W5): path-V / trio / must-stay value move = stop-and-explain;
          6465510 serving lot-sum-awards = SWITCH OFF (every class); D/Z/N/W/I: a path-W pin off its frozen value or
          vouch, or a partial sum off its S or vouch = SWITCH OFF at once, tender row or not, no read or explanation
          overrides it; 6908248 MOVED = stop and read at 0 wrong (read wrong: serving a double count, SWITCH OFF; a
          LOST is R3's expected refusal of the double count, an ordinary LOST).
  BAND    the seek against B_R plus the registered band rows: a B_R head missing = SWITCH OFF (899272 / 5842187 or
          within 2 % of EUR 100 m: stop-and-explain; chain: stop and read, switch off only if it reads wrong); an extra
          row = STOP and read at 0 wrong of any class (outside R and the segment: an unread writer, find it).
  SCOPE   a foreign job in (CB, CA] (W10) or before the tick (chain/weekly), a missing own job, an unknown kind, a
          vanished lot, a retired R member, E1/all-kind parity off, a Z run with corrected > 0 or tender rows = STOP;
          an in-place W-input writer among the foreign jobs without its C9' pre-step = SWITCH OFF; an incomplete read,
          A not this segment's, or own jobs that report corrected > 0 over a range with no tender row = NOT A
          MEASUREMENT.
  class I (or a gap whose foreign jobs hold an explained W-input writer, with --prediction): every id the prediction
          names is compared (R heads included): a matched predicted move is recorded with the port's vouches; a head
          off its prediction = SWITCH OFF; a predicted move / fire that did not happen = STOP, SWITCH OFF unexplained at
          the end of the gap; an R head with a tender row that moved and is not in the prediction = SWITCH OFF.
  D7      (W/I/D with --runs-left; class W requires it, 0 for the last run) the start rule for the runs NOT YET started:
          an ADVISORY only -- NO-GO means "do not start the next run; it moves to the next gap" (HOLD, enforced again by
          segment.py open / make_wet); it never freezes K and never changes the verdict of the run just checked. A run
          in flight is D7's switch-off case: registry.py status.
The end of the gap is the segment's recorded switch-off instant (segment.py), or an earlier --gap-end; now is the clock.
--explained FILE (reads and explanations already recorded in 551):
  {"reads": {"<id>": {"verdict": "right"|"neutral"|"wrong", "ref": "..."}},
   "explanations": {"<id>" | "foreign_jobs" | "missing_own" | "z_corrected" | "z_rows" | "count_3x" | "parity" | "cap"
                    | "class_premise" | "unknown_kind:<kind>" | "vanished_lots" | "w_input_prestep": "<text, as in 551>"}}
  a read right/neutral or an explanation turns that STOP into a note (an explained MOVED/LOST is then recorded in R);
  a read wrong is SWITCH OFF. vanished_lots must name every vanished lot cursor it explains.
Writes segments/<seg>/result.json (with the registry advance it implies) and segments/<seg>/new_population.json (the
NEW heads in u8/delta.py's population shape with seg, tag and the pre-registered salt, for E5's predraw/draw). A verdict
other than PASS freezes K (escalating at the end of the gap when it is made only of F/W4's gap-end STOPs).
Exit codes: 0 PASS, 10 SWITCH OFF, 2 STOP, 3 NOT A MEASUREMENT (a read incomplete, or any tool fault).
"""
import argparse
import datetime
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wkit as W  # noqa: E402
import registry as REG  # noqa: E402
import segment as SEG  # noqa: E402
import start_rule as SR  # noqa: E402

K = W.K
PASS, SWITCH_OFF, STOP, NOT_MEASURED = W.PASS, W.SWITCH_OFF, W.STOP, W.NOT_MEASURED
JOBS_FRESH_S = 600


def rdir(seg, *p):
    return os.path.join(SEG.seg_dir(seg), 'read', *p)


def _common(a):
    reg = REG.load()
    rec = SEG.load_rec(a.seg)
    if rec['class'] != a.klass:
        raise SystemExit(f'{a.seg} is a class-{rec["class"]} segment, not {a.klass}')
    if rec.get('status') != 'closed' or rec.get('CA') is None:
        raise SystemExit(f'{a.seg}: the segment is not closed (segment.py close / chain / weekly records CA first)')
    if int(rec['CA']) != a.cursor_after:
        raise SystemExit(f'--cursor-after {a.cursor_after} != the segment\'s CA {rec["CA"]}')
    if int(reg['K']) != a.k:
        raise SystemExit(f'--k {a.k} != the registry\'s K {reg["K"]} (gd5 runs over (K, CA])')
    if a.cursor_after < a.k:
        raise SystemExit(f'CA {a.cursor_after} < K {a.k}')
    return reg, rec


def _jobs_ok(a, rec):
    """B5 / B1: never read while a job runs, and nothing but dry runs since the segment's CA snapshot."""
    snap = SEG.jobs_snapshot(a.jobs)
    now = SR.parse_now(a.now)
    if not SEG.idle(snap):
        raise SystemExit(f'{a.jobs}: /admin/jobs is not idle -- never read while a job runs (B5)')
    m = int(snap['measured_at'])
    if abs(now.timestamp() - m) > JOBS_FRESH_S:
        raise SystemExit(f'{a.jobs}: measured {K.iso_of(m)}, more than {JOBS_FRESH_S // 60} min from now '
                         f'{now:%Y-%m-%dT%H:%M:%SZ}: save a fresh body just before the read')
    cj = rec.get('ca_jobs') or {}
    wet, _dry = SEG.since_mark(snap, {'max_job_id': cj.get('max_job_id'), 'measured_at': cj.get('measured_at')})
    if wet:
        raise SystemExit(f'job(s) since the segment\'s CA snapshot {[(j["job_id"], j["kind"]) for j in wet]}: their '
                         'writes would leak into this read; close them as their own segment first')
    return snap


# ---------------------------------------------------------------- read
def cmd_read(a, fake=None, http=None):
    reg, rec = _common(a)
    snap = _jobs_ok(a, rec)
    box = W.w_box(fake)
    u0 = W.box_used(box)
    out = rdir(a.seg)
    os.makedirs(out, exist_ok=True)
    man = {'seg': a.seg, 'class': a.klass, 'K': a.k, 'CA': a.cursor_after, 'at': K.now_iso(), 'complete': False,
           'jobs_sha256': snap['file_sha256']}
    route = a.route
    short = a.klass in ('Z', 'weekly') and not a.full
    if short and (rec.get('full_required') or rec.get('foreign') or rec.get('missing_own')):
        print(f'{a.seg}: the zero-change shortcut needs exactly the segment\'s own jobs (D.2) -- this record has '
              f'{"a gap" if rec.get("full_required") else "foreign / missing own jobs"}: the full read instead')
        short = False
    if short and a.k != a.cursor_after and not W.e1_proven():
        print(f'{a.seg}: E1 is not proven yet (W12): no one-statement E1 shortcut -- the full read, all-kind')
        short, route = False, 'all'
    if short:
        man['shortcut'] = True
        if a.k == a.cursor_after:
            man.update(complete=True, e1_rows=0, e1_ids=[], statements=0, note='K == CA: nothing was written')
        else:
            rows, _ok = K.tender_keyset(box, a.k, a.cursor_after, rdir(a.seg, 'zshort'), label=f'{a.seg}_z',
                                        max_pages=1)
            man_p = K.jload(rdir(a.seg, 'zshort', 'manifest.json'))
            answered = any(p.get('ok') for p in man_p['pages'].values())
            man.update(complete=answered, e1_rows=len(rows), e1_ids=sorted({int(r['entity_id']) for r in rows})[:200],
                       statements=W.box_used(box) - u0)
        K.jdump(os.path.join(out, 'read.json'), man)
        print(f'{a.seg}: Z shortcut over ({a.k}, {a.cursor_after}]: '
              + (f'{man["e1_rows"]} tender row(s)' if man['complete'] else 'the E1 statement did not answer')
              + (' -- a Z range with tender rows is a STOP: gd5 read --full, then check' if man.get('e1_rows') else ''))
        return 0 if man['complete'] else NOT_MEASURED
    if a.klass in ('Z', 'weekly') and route == 'auto' and not W.e1_proven():
        route = 'all'
    ch = W.changed_since(box, a.k, a.cursor_after, rdir(a.seg, 'changes'), route=route, label=a.seg)
    man['changed'] = W.jsonable_changed(ch)
    if not ch['complete']:
        man['statements'] = W.box_used(box) - u0
        K.jdump(os.path.join(out, 'read.json'), man)
        print(f'{a.seg}: the change enumeration is incomplete (box_health, then re-run this read: a transport failure '
              'or shed resumes the same page; a 408 resumes with the page halved, never the same statement)')
        return NOT_MEASURED
    entries = {int(i) for i in reg['entries']}
    rset = sorted(entries | {int(i) for i in reg['watch']} | set(K.pin_ids()))
    retired = set(ch['retired']) | {int(i) for i in reg.get('retired', [])}
    if ch['lsa'] is not None:
        cands = (set(ch['lsa']) - entries) | (set(ch['live']) - set(ch['tenders']) - set(rset))
    else:
        cands = set(ch['live']) - set(rset)
    cands = sorted(cands - set(ch['retired']))
    ok = True
    if cands:
        _r, f, m = K.pk_read(box, cands, f'{a.seg}_new', rdir(a.seg, 'new'), resume=True)
        ok = not f and not m
    else:
        K.jdump(rdir(a.seg, 'new', 'rows.json'), {})
        K.jdump(rdir(a.seg, 'new', 'manifest.json'), {'complete': True, 'n_ids': 0, 'absent_expected': []})
    if ok:
        _r, f, m = K.pk_read(box, rset, f'{a.seg}_r', rdir(a.seg, 'rpk'), expected_absent=retired, resume=True)
        ok = not f and not m
    if ok:
        body, broute = K.read_band(box, f'{a.seg}_band', rdir(a.seg, 'band.json'), route=a.band_route, http=http)
        ok = body is not None
        man['band_route'] = broute
    live = sorted(ch['live'])
    K.jdump(rdir(a.seg, 'changed_live_ids.json'), {'ids': live, 'n': len(live), 'ids_sha256': K.ids_sha(live),
                                                   'seg': a.seg})
    e1_rows = (ch.get('e1') or {}).get('n_rows')
    span_pages = -(-(e1_rows or 0) // K.E1_PAGE) + 1 if e1_rows is not None else \
        -(-(a.cursor_after - a.k) // 5000)
    man['A_jobs'] = len(K.chunks(live, 5000))
    man['switchoff_read_statements'] = 2 * (span_pages + 2) + 2   # build + verify over this span, + PK, band, health
    man.update(complete=ok, new_candidates=len(cands), rset=len(rset), statements=W.box_used(box) - u0)
    K.jdump(os.path.join(out, 'read.json'), man)
    print(f'{a.seg}: reads {"complete" if ok else "INCOMPLETE"} over ({a.k}, {a.cursor_after}] (route {ch["route"]}'
          + (', E1 unproven: all-kind' if ch.get('e1_unproven') and ch['route'] == 'all' else '') + '): '
          f'{len(ch["tenders"])} Tenders with tender rows, {len(live)} live changed, {len(ch["retired"])} retired; '
          f'{len(cands)} NEW candidates PK-read; R ∪ Rg ∪ pins {len(rset)}; band {man.get("band_route")}; '
          f'{man["statements"]} statements; changed live ids for A: {rdir(a.seg, "changed_live_ids.json")} '
          f'({man["A_jobs"]} dry job(s): ajobs.py build {a.seg} --from-seg {a.seg})')
    return PASS if ok else NOT_MEASURED


# ---------------------------------------------------------------- check (pure classification, tests call it)
def _pred_matches(r, p):
    det = K.detail_of(r)
    return (W.lsa(r) and r.get('current_value_cents') == p.get('S')
            and r.get('current_value_currency') == p.get('currency') and det.get('lots') == p.get('n')
            and K.vb_key(det.get('vouched_by')) in [list(v) for v in p.get('vouches') or []])


def _pred_equals_entry(e, p):
    reg = e['row']
    return (reg.get('current_value_cents') == p.get('S') and reg.get('current_value_currency') == p.get('currency')
            and K.detail_of(reg).get('lots') == p.get('n')
            and any(list(v) in [list(x) for x in p.get('vouches') or []] for v in e.get('vouches') or []))


def classify(reg, frz, rec, rd, rows_new, rows_r, band, opts):
    """-> (outs, adv, info). opts: klass, corrected, A, A_bad, explained, prediction, gap_end, now, d7, own_corrected."""
    klass = opts['klass']
    chain = klass == 'chain'
    zlike = klass in ('Z', 'weekly')
    ex = opts.get('explained') or {}
    reads = {str(k): v for k, v in (ex.get('reads') or {}).items()}
    expl = {str(k): v for k, v in (ex.get('explanations') or {}).items()}
    now, gap_end = opts.get('now'), opts.get('gap_end')
    late = now is not None and gap_end is not None and now >= gap_end
    tag = rec['tag']
    gap_seg = bool(rec.get('full_required')) or str(tag).startswith('gap-')
    pred = opts.get('prediction')
    winput = [j for j in rec.get('foreign') or [] if W.is_w_input_kind(j.get('kind'))]
    iclass = klass == 'I' or (pred is not None and bool(winput))
    o_scope, o_new, o_r, o_pins, o_band = W.Out('SCOPE'), W.Out('NEW'), W.Out('R (MOVED/LOST/SILENT)'), W.Out('PINS'), \
        W.Out('BAND')
    outs = [o_scope, o_new, o_r, o_pins, o_band]
    adv = {'add': [], 'moved': [], 'lost': [], 'refresh': {}, 'band_add': [], 'band_drop': [], 'retired': []}
    info = {'new': [], 'moved': [], 'lost': [], 'silent': [], 'silent_new': [], 'notes': {}, 'd7_hold': False}
    sil_code = SWITCH_OFF if late else STOP
    late_txt = ' -- unexplained at the end of the gap: switch off' if late else \
        ' (switch off if unexplained by the end of the gap)'

    def explained(key, out, msg):
        if key in expl:
            out.note(f'{msg} -- EXPLAINED: {expl[key]}')
            return True
        return False

    def read_verdict(key):
        return (reads.get(key) or {}).get('verdict')

    # -- scope: jobs (B1/W10), Z rules, completeness
    if rec.get('foreign'):
        if not explained('foreign_jobs', o_scope, f'foreign job(s) {rec["foreign"]}'):
            o_scope.add(STOP, f'a foreign job (W10/B1): {[(j.get("job_id"), j.get("kind")) for j in rec["foreign"]]}'
                              ' -- a writer that is not this segment\'s own')
    for j in winput:
        if 'w_input_prestep' in expl and pred is not None:
            o_scope.note(f'in-place W-input writer {j.get("kind")} {j.get("job_id")}: its C9\' pre-step is recorded '
                         f'({expl["w_input_prestep"]}); judged against --prediction below (class I rules)')
        else:
            o_scope.add(SWITCH_OFF, f'an in-place W-input writer landed without its pre-step (F): {j.get("kind")} '
                                    f'{j.get("job_id")} -- it needs the C9\' prediction over R ∩ touched (--prediction) '
                                    'and the paired refold, recorded as w_input_prestep')
    if rec.get('missing_own'):
        if not explained('missing_own', o_scope, f'own job(s) {rec["missing_own"]} not found'):
            o_scope.add(STOP, f'own job(s) {rec["missing_own"]} not found in (CB, CA]')
    if klass == 'Z' and not gap_seg:
        if (opts.get('corrected') or 0) > 0:
            if not explained('z_corrected', o_scope, f'corrected {opts["corrected"]} on a Z run'):
                o_scope.add(STOP, f'a Z run with corrected {opts["corrected"]} > 0 (zero corrections by design)')
        if opts.get('corrected') is None:
            o_scope.add(NOT_MEASURED, 'a Z run needs --corrected (the job\'s corrected count; D.2)')
    if rd.get('shortcut'):
        if rec.get('foreign') or rec.get('missing_own') or gap_seg:
            o_scope.add(NOT_MEASURED, 'the zero-change shortcut needs exactly the segment\'s own jobs (D.2): an '
                                      'explanation clears the W10 STOP, not D.2 -- gd5 read --full')
            return outs, adv, info
        n = rd.get('e1_rows', 0)
        if not rd.get('complete'):
            o_scope.add(NOT_MEASURED, 'the Z shortcut\'s E1 statement did not answer')
        elif n:
            if not explained('z_rows', o_scope, f'{n} tender rows in the {klass} range'):
                o_scope.add(STOP, f'{n} tender row(s) in a {klass} range (0 expected): gd5 read --full over (K, CA], '
                                  f'every NEW/MOVED read (first ids {rd.get("e1_ids", [])[:10]})')
        else:
            o_scope.note(f'zero-change shortcut: {"K == CA" if rd["K"] == rd["CA"] else "one E1 statement, 0 tender rows"}'
                         f' over ({rd["K"]}, {rd["CA"]}]; jobs in (CB, CA] = own {rec.get("own")}')
        adv['retired'] = []
        return outs, adv, info
    if not rd.get('complete'):
        o_scope.add(NOT_MEASURED, 'the read is incomplete (gd5 read again after box_health; it resumes -- a 408 halved)')
        return outs, adv, info
    chd = rd['changed']
    tenders, retired = set(chd['tenders']), set(chd['retired'])
    retired_all = retired | {int(i) for i in reg.get('retired', [])}
    adv['retired'] = sorted(retired)
    cls = chd.get('cls')
    if cls is not None:
        for kk in cls['unknown_kinds']:
            if not explained(f'unknown_kind:{kk}', o_scope, f'unknown entity_kind {kk!r}'):
                o_scope.add(STOP, f'unknown entity_kind {kk!r} in (K, CA] (explainable per kind: unknown_kind:{kk})')
        badl = cls['unmapped_lots']['bad']
        if badl:
            ev = str(expl.get('vanished_lots', ''))
            uncovered = [c for c in badl if str(c) not in ev]
            if 'vanished_lots' in expl and not uncovered:
                o_scope.note(f'{len(badl)} vanished lot row(s) {badl[:10]} -- EXPLAINED: {ev}')
            else:
                o_scope.add(STOP, f'{len(badl)} lot rows map to no Tender and no later row removes their lot (a vanished '
                                  f'lot): {badl[:10]}' + (f'; the vanished_lots explanation does not name {uncovered[:10]}'
                                                          if 'vanished_lots' in expl else ''))
        o_scope.note(f'all-kind rows {cls["by_kind_op"]}')
    else:
        o_scope.note(f'E1 route ({(chd.get("e1") or {}).get("n_rows")} tender rows {(chd.get("e1") or {}).get("by_op")}):'
                     ' the all-kind hygiene checks (unknown kinds, vanished lots) need a span <= 50k')
    if chd.get('e1_unproven') and chd.get('route') == 'all' and chd.get('route_asked') == 'auto':
        o_scope.note('E1 not proven yet (W12): this span was read all-kind')
    par = chd.get('parity')
    if par is not None:
        if not (par['tenders_equal'] and par['retired_equal']):
            if not explained('parity', o_scope, f'E1 / all-kind parity {par}'):
                o_scope.add(STOP, f'E1 and the all-kind reader disagree on (K, CA] (W12): only E1 {par["only_e1"]}, only '
                                  f'all-kind {par["only_all"]}')
        else:
            o_scope.note(f'E1 = the all-kind reader on the tender rows ({len(tenders)} Tenders, {len(retired)} retired); '
                         f'{par["lot_only_live"]} lot-only Tenders read beside')
    if zlike and tenders:
        if not explained('z_rows', o_scope, f'{len(tenders)} Tenders with tender rows in the {klass} range'):
            o_scope.add(STOP, f'{len(tenders)} Tender(s) with tender rows in a {klass} range (0 expected): every NEW / '
                              f'MOVED below is read and explained ({sorted(tenders)[:10]})')
    oc = opts.get('own_corrected')
    if klass in ('D', 'N', 'W', 'I') and oc and not tenders:
        o_scope.add(NOT_MEASURED, f'the own jobs report {oc} corrected but (K, CA] holds no tender row: CA is too small '
                                  '(read CA after the project, twice around the jobs body)')
    touched_579 = sorted(set(W.NOTE_579) & tenders)
    if touched_579:
        o_scope.note(f'579\'s order-only Tenders with tender rows here (W1 NOTE): {touched_579}')
    rows_all = dict(rows_new)
    rows_all.update(rows_r)
    entries = reg['entries']
    watch = reg['watch']
    at_epoch = K.epoch_of(rec['tick']) if rec.get('tick') else None

    # -- NEW
    new = sorted(i for i, r in rows_all.items() if W.lsa(r) and str(i) not in entries)
    pin_flip = [i for i in new if (watch.get(str(i)) or {}).get('pin') in W.STOP_EXPLAIN_ROLES
                or (int(i) in W.STOP_EXPLAIN_PINS and str(i) not in watch)]
    for i in pin_flip:
        role = (watch.get(str(i)) or {}).get('pin') or 'W5 pin'
        tr = i in tenders
        msg = (f'{role} {i}: derivation -> lot-sum-awards, a value move on a path-V / trio / must-stay head (W5)'
               + ('' if tr else '; no tender row: SILENT'))
        if read_verdict(str(i)) == 'wrong':
            o_pins.add(SWITCH_OFF, f'{msg}: READ WRONG')
        elif explained(str(i), o_pins, msg):
            adv['add'].append({'id': i, 'tag': tag, 'row': rows_all[i],
                               'vouches': [K.vb_key(K.detail_of(rows_all[i]).get('vouched_by'))],
                               'vouches_from': 'served (read pending)', 'read': 'pending',
                               'flags': [f'{role} pin flipped to lot-sum-awards, explained']})
        elif tr:
            o_pins.add(STOP, f'{msg}: stop and explain')
        else:
            info['silent'].append(i)
            o_pins.add(sil_code, f'{msg}{late_txt}', esc=True)
    rest = [i for i in new if i not in pin_flip]
    silent_new = [i for i in rest if i not in tenders]
    new_tr = [i for i in rest if i in tenders]
    for i in silent_new:
        if read_verdict(str(i)) == 'wrong':
            o_new.add(SWITCH_OFF, f'{i} serves lot-sum-awards with no tender row and READ WRONG')
        elif not explained(str(i), o_new, f'{i} serves lot-sum-awards with no tender row'):
            o_new.add(sil_code, f'{i} serves lot-sum-awards with NO tender row in (K, CA]: SILENT (an unannounced writer; '
                                f'find it){late_txt}', esc=True)
            info['silent'].append(i)
    info['silent_new'] = silent_new
    A = opts.get('A')
    cap = None
    if opts.get('A_bad'):
        o_new.add(NOT_MEASURED, opts['A_bad'])
        A = None
    if chain:
        if A is None and not opts.get('A_bad'):
            o_new.add(NOT_MEASURED, 'class chain needs A_d (u9/ajobs.py count over the chain\'s changed live Tenders; '
                                    'the 3x rule is on A_d, C8/A8)')
        elif A is not None:
            exp = A * W.RATE[0] / W.RATE[1]
            o_new.note(f'daily fires {len(new_tr)} (counted, registered {tag}); A_d {A}; expected {exp:.2f}; 3x {3 * exp:.2f}')
            if len(new_tr) > 3 * exp and not explained('count_3x', o_new, f'{len(new_tr)} > 3 x {exp:.2f}'):
                o_new.add(SWITCH_OFF if late else STOP,
                          f'{len(new_tr)} daily fires > 3 x {exp:.2f} (A_d {A} x 1.0974 %): '
                          + ('still unexplained at the end of the gap: a U9 switch-off' if late else
                             'stop and read; unexplained above 3x is a U9 switch-off'), esc=True)
    elif klass == 'W':
        if A is None and not opts.get('A_bad'):
            o_new.add(NOT_MEASURED, 'class W needs A (a dry refold-lot-sum over the segment\'s changed live Tenders: '
                                    'u9/ajobs.py count) for the cap (D.3)')
        elif A is not None:
            cap, x = W.cap_of(A)
            n_new = len(rest)
            o_new.note(f'NEW {n_new}; A {A}; A x 1.0974 % = {x:.2f}; cap max(2 A r, A r + 10) = {cap:.2f} (a tripwire)')
            if n_new > cap and not explained('cap', o_new, f'NEW {n_new} over the cap {cap:.2f} (every NEW stays '
                                                           'read-pending: W9 decides trust)'):
                o_new.add(STOP, f'NEW {n_new} over the cap {cap:.2f}: STOP before the next run (read the NEW: '
                                'explanation key cap)')
    elif iclass:
        if pred is None:
            o_new.add(NOT_MEASURED, 'class I needs --prediction (C9\': the per-id port prediction on cached facts)')
        else:
            for i in new_tr:
                e = pred.get(str(i))
                if not e:
                    o_new.add(SWITCH_OFF, f'{i}: an unpredicted fire (class I, D1 rules)')
                    continue
                if not _pred_matches(rows_all[i], e):
                    r = rows_all[i]
                    det = K.detail_of(r)
                    bad = [x for x, okk in (('S', r.get('current_value_cents') == e.get('S')),
                                            ('currency', r.get('current_value_currency') == e.get('currency')),
                                            ('n', det.get('lots') == e.get('n')),
                                            ('vouched_by', K.vb_key(det.get('vouched_by')) in
                                             [list(v) for v in e.get('vouches') or []])) if not okk]
                    o_new.add(SWITCH_OFF, f'{i}: fired off the port prediction ({bad}; class I, D1 rules)')
            for sid, e in pred.items():
                if e and int(sid) not in new_tr and sid not in entries and int(sid) not in pin_flip:
                    if not explained(sid, o_new, f'{sid} predicted to fire'):
                        o_new.add(SWITCH_OFF if late else STOP, f'{sid} predicted to fire and did not: explain within '
                                  'the gap' + (' -- unexplained at the end of the gap (D1)' if late else ''), esc=True)
    else:
        if new_tr:
            premise = len(new_tr) > W.READ_CAP
            drawn = None
            if premise and 'class_premise' in expl:
                salt = (rec.get('prereg') or {}).get('salt')
                if not salt:
                    o_new.add(NOT_MEASURED, 'class_premise needs the segment\'s pre-registered salt for the salted 30')
                else:
                    drawn = set(K.salted(new_tr, salt)[:W.READ_CAP])
            for i in new_tr:
                rv = read_verdict(str(i))
                if rv == 'wrong':
                    o_new.add(SWITCH_OFF, f'NEW {i} READ WRONG (a failed NEW read)')
                elif rv in ('right', 'neutral'):
                    o_new.note(f'NEW {i} read {rv} ({reads[str(i)].get("ref", "")})')
                elif drawn is not None and i in drawn:
                    o_new.add(STOP, f'NEW {i} is in the salted 30 (class_premise): it needs its read')
                elif drawn is not None:
                    o_new.note(f'NEW {i} outside the salted 30 (class_premise explained): not read')
                elif not explained(str(i), o_new, f'NEW {i} in class {klass}'):
                    o_new.add(STOP, f'NEW {i} in class {klass} (expected 0): read it and explain its mechanism (an '
                                    'enumeration miss or drift; a 541/571 residual is an enumeration miss, U6(a))')
            if premise and not explained('class_premise', o_new, f'{len(new_tr)} NEW > 30 in class {klass}'):
                o_new.add(STOP, f'{len(new_tr)} NEW > 30 in class {klass}: the class premise is broken -- programme STOP, '
                                'a salted 30 under U5\'s bar, no further drain until explained (class_premise)')
    if klass in ('W', 'chain'):
        for i in new_tr:
            if read_verdict(str(i)) == 'wrong':
                o_new.add(SWITCH_OFF, f'NEW {i} READ WRONG (a failed NEW read)')
    step2_done = set(W.STEP2_TAGS) <= {h['tag'] for h in reg['history']}
    for i in new_tr:
        r = rows_all[i]
        vk = K.vb_key(K.detail_of(r).get('vouched_by'))
        flags = []
        if chain and r.get('source') == 'doe' and not step2_done:
            flags.append('possibly 539-driven: a daily fire on a DÖE Tender before all three 539 step-2 runs '
                         '(drain-539-a/b/c) advanced (A6/H)')
        adv['add'].append({'id': i, 'tag': tag, 'row': r, 'vouches': [vk],
                           'vouches_from': 'served (day-7 population)' if chain else 'served (read pending)',
                           'read': 'day7' if chain else 'pending', 'flags': flags})
    for i in silent_new:
        if str(i) in expl:
            r = rows_all[i]
            adv['add'].append({'id': i, 'tag': tag, 'row': r, 'vouches': [K.vb_key(K.detail_of(r).get('vouched_by'))],
                               'vouches_from': 'served (read pending)', 'read': 'pending', 'flags': ['SILENT, explained']})
    info['new'] = new_tr

    # -- R entries
    pe_w = frz['pins_expected']['path_w']
    notes_date, notes_fx, notes_eur, notes_w15 = [], [], [], []
    for sid, e in entries.items():
        i = int(sid)
        r = rows_r.get(i)
        role = e.get('pin')
        if r is None:
            if i in retired_all:
                if explained(sid, o_r, f'R member {i} retired'):
                    adv['lost'].append({'id': i, 'tag': tag, 'row': None})
                else:
                    o_r.add(STOP, f'R member {i} RETIRED (no tenders row): explain within the gap')
            else:
                o_r.add(NOT_MEASURED, f'R member {i}: no read')
            continue
        tr = i in tenders
        if not W.lsa(r):
            kind, mv = 'LOST', ['derivation']
        else:
            mv = W.w_moves(e, r)
            kind = 'MOVED' if mv else None
        vd = W.value_diff(e['row'], r)
        dm = W.date_moved(e['row'], r)
        desc = (f'{i} ({e["tag"]}{", " + role if role else ""}) {kind or "value"} {mv if kind else vd}: '
                f'{e["row"].get("current_value_derivation")} {e["row"].get("current_value_currency")} '
                f'{e["row"].get("current_value_cents")} -> {r.get("current_value_derivation")} '
                f'{r.get("current_value_currency")} {r.get("current_value_cents")} (eur '
                f'{e["row"].get("current_value_eur_cents")} -> {r.get("current_value_eur_cents")})')
        rv = read_verdict(sid)
        # 1. D/Z/N/W/I: the pins F switches off at once -- tender row or not, no read or explanation overrides it
        if not chain and kind is not None:
            switch = None
            if role == 'path-W':
                x = pe_w.get(sid) or {}
                vk = K.vb_key(K.detail_of(r).get('vouched_by'))
                if not (W.lsa(r) and r.get('current_value_cents') == x.get('cents')
                        and r.get('current_value_currency') == x.get('currency')
                        and vk in [list(v) for v in x.get('accepted_vouches') or []]):
                    switch = f'path-W pin {i} off its path-W value or vouch (expected {x.get("currency")} {x.get("cents")})'
            elif role == 'partial-sum' and (kind == 'LOST' or 'S' in mv or 'vouched_by' in mv):
                switch = f'partial sum {i} off its S or vouch'
            if switch:
                if not tr:
                    info['silent'].append(i)
                else:
                    info['moved' if kind == 'MOVED' else 'lost'].append(i)
                o_pins.add(SWITCH_OFF, f'{switch}: ' + ('SILENT ' if not tr else '') + desc)
                continue
        # 2. class I: the port prediction names this head (or does not, though it moved with a tender row)
        if iclass and pred is not None and sid in pred:
            p = pred[sid]
            if p:
                if _pred_matches(r, p):
                    if kind is not None:
                        o_r.note(f'predicted-MOVED {desc}: at the port prediction (vouches {p.get("vouches")})')
                        info['moved'].append(i)
                        adv['moved'].append({'id': i, 'tag': tag, 'kind': 'predicted-MOVED', 'fields': mv, 'row': r,
                                             'vouches': [list(v) for v in p.get('vouches') or []]})
                    else:
                        adv['refresh'][sid] = r
                elif kind is None and not _pred_equals_entry(e, p):
                    if not explained(sid, o_r, f'{i}: a predicted move that did not happen'):
                        o_r.add(SWITCH_OFF if late else STOP, f'{i}: a predicted move that did not happen ({desc}): '
                                'explain within the gap (D1)' + (' -- unexplained at the end of the gap' if late else ''),
                                esc=True)
                else:
                    o_r.add(SWITCH_OFF, f'class-I per-id mismatch against the port prediction (F, D1): {desc}; '
                                        f'predicted {p}')
            else:
                if not W.lsa(r):
                    o_r.note(f'predicted-LOST {desc}')
                    info['lost'].append(i)
                    adv['lost'].append({'id': i, 'tag': tag, 'kind': 'predicted-LOST', 'fields': mv, 'row': r})
                elif kind is None:
                    if not explained(sid, o_r, f'{i}: a predicted loss that did not happen'):
                        o_r.add(SWITCH_OFF if late else STOP, f'{i}: predicted LOST, still serves lot-sum-awards '
                                'unchanged: explain within the gap (D1)', esc=True)
                else:
                    o_r.add(SWITCH_OFF, f'class-I per-id mismatch: {i} predicted LOST but serves lot-sum-awards at a '
                                        f'moved value ({desc})')
            continue
        eur_note = kind == 'MOVED' and mv == ['eur'] and tr and bool(dm)      # F's NOTE: eur-only on a re-dated head
        if iclass and pred is not None and tr and kind is not None and not eur_note:
            o_r.add(SWITCH_OFF, f'an unpredicted per-id move (class I: the prediction must carry every id in R ∩ '
                                f'touched): {desc}')
            continue
        # 3. no tender row: SILENT on the full value columns (vouched_by included; W6)
        if not tr:
            if not vd:
                if dm:
                    notes_date.append(i)
                adv['refresh'][sid] = r
                continue
            info['silent'].append(i)
            gap = (('current_value_derivation' in vd or 'current_value_derivation_detail' in vd)
                   and W.seq_increased(e['row'], r))
            k2 = kind or 'MOVED'
            if rv == 'wrong':
                o_r.add(SWITCH_OFF, f'SILENT {desc}: READ WRONG')
            elif explained(sid, o_r, f'SILENT {desc}'):
                (adv['lost'] if k2 == 'LOST' else adv['moved']).append(
                    {'id': i, 'tag': tag, 'kind': f'SILENT-{k2}', 'fields': mv if kind else vd, 'row': r})
            elif chain and gap:
                o_r.add(STOP, f'SILENT {desc} with a seq increase ({e["row"].get("current_seq")} -> '
                              f'{r.get("current_seq")}) and NO tender row: a fold announcement gap -- stop, explain, '
                              'file (W6; in a chain never a switch-off, W4)')
            elif gap:
                o_r.add(sil_code, f'SILENT {desc} with a seq increase and NO tender row: a fold announcement gap (W6), '
                                  f'a SILENT in class {klass}{late_txt}', esc=True)
            else:
                o_r.add(sil_code, f'SILENT {desc} with NO tender row: an unannounced writer{late_txt}', esc=True)
            continue
        # 4. a tender row: W15 membership, the NOTEs
        if kind is None:
            if dm:
                notes_date.append(i)
            if vd:
                notes_w15.append(i)
            adv['refresh'][sid] = r
            continue
        if kind == 'MOVED' and mv == ['eur']:
            if chain and W.fresh_fx(r, at_epoch or K.epoch_of(K.now_iso())):
                notes_fx.append(i)
                adv['refresh'][sid] = r
                continue
            if not chain and dm:
                notes_eur.append(i)
                adv['refresh'][sid] = r
                continue
        if chain:
            if role in W.STOP_EXPLAIN_ROLES:
                if explained(sid, o_pins, f'chain-{kind} {desc} ({role})'):
                    (adv['lost'] if kind == 'LOST' else adv['moved']).append(
                        {'id': i, 'tag': tag, 'kind': f'explained-{kind}', 'fields': mv, 'row': r})
                else:
                    o_pins.add(STOP, f'chain-{kind} {desc}: a value move on a {role} head: stop and explain (W5)')
                continue
            special = i in reg['B_R'] or role in ('path-W', 'partial-sum', 'double-count')
            if special:
                if rv == 'wrong':
                    o_r.add(SWITCH_OFF, f'chain-{kind} {desc}: READ WRONG'
                            + (' (6908248 serving a double count)' if role == 'double-count' else ''))
                    continue
                if rv in ('right', 'neutral'):
                    o_r.note(f'chain-{kind} {desc}: read {rv} at 0 wrong ({reads[sid].get("ref", "")})')
                else:
                    what = ('a B_R head moved' if i in reg['B_R'] else
                            '6908248 moved (a double count is a switch-off)' if role == 'double-count' else
                            f'a {role} pin moved')
                    o_r.add(STOP, f'chain-{kind} {desc}: {what} -- stop and read it at 0 wrong of any class')
                    continue
            else:
                o_r.note(f'chain-{kind} {desc}: a counted daily re-decision (registered {tag}; read under H)')
            info['moved' if kind == 'MOVED' else 'lost'].append(i)
            (adv['lost'] if kind == 'LOST' else adv['moved']).append(
                {'id': i, 'tag': tag, 'kind': f'chain-{kind}', 'fields': mv, 'row': r})
            continue
        # segment classes D/Z/N/W/I
        info['moved' if kind == 'MOVED' else 'lost'].append(i)
        if role == 'double-count' and kind == 'MOVED':
            if rv == 'wrong':
                o_pins.add(SWITCH_OFF, f'6908248 serving a double count (read wrong): {desc}')
            elif rv in ('right', 'neutral'):
                o_pins.note(f'6908248 moved, read {rv} at 0 wrong ({reads[sid].get("ref", "")}): {desc}')
                adv['moved'].append({'id': i, 'tag': tag, 'kind': 'read-MOVED', 'fields': mv, 'row': r})
            else:
                rise = (r.get('current_value_currency') == e['row'].get('current_value_currency')
                        and (r.get('current_value_cents') or 0) > (e['row'].get('current_value_cents') or 0))
                o_pins.add(sil_code, f'6908248 moved: read it at 0 wrong -- a double count is a switch-off ({desc})'
                           + (' -- S rose: the b5 read note 2 signature (RES-2 keyed to LOT-2, about EUR 1.68 m)'
                              if rise else '') + (' -- unread at the end of the gap' if late else ''), esc=True)
            continue
        if role in W.STOP_EXPLAIN_ROLES:
            if explained(sid, o_pins, f'{kind} {desc} ({role})'):
                (adv['lost'] if kind == 'LOST' else adv['moved']).append(
                    {'id': i, 'tag': tag, 'kind': f'explained-{kind}', 'fields': mv, 'row': r})
            else:
                o_pins.add(STOP, f'{kind} {desc}: a value move on a {role} head: stop and explain (W5)')
            continue
        lost_dc = ' (6908248 LOST: R3\'s expected refusal of the double count)' if role == 'double-count' else ''
        if rv == 'wrong':
            o_r.add(SWITCH_OFF, f'{kind} {desc}: READ WRONG')
        elif rv in ('right', 'neutral') or explained(sid, o_r, f'{kind} {desc}{lost_dc}'):
            if rv:
                o_r.note(f'{kind} {desc}: read {rv} ({reads[sid].get("ref", "")})')
            (adv['lost'] if kind == 'LOST' else adv['moved']).append(
                {'id': i, 'tag': tag, 'kind': f'explained-{kind}', 'fields': mv, 'row': r})
        else:
            what = 'a probe that moves a W column' if klass == 'D' else 'predicted 0'
            o_r.add(sil_code, f'{kind} {desc} ({what}){lost_dc}: read it, port it on refetched facts'
                    + (' -- unexplained at the end of the gap: switch off' if late else
                       '; wrong, or unexplained by the end of the gap: switch off'), esc=True)
    if notes_date:
        o_r.note(f'{len(notes_date)} R heads moved seq/published_at only (NOTE): {notes_date[:10]}')
    if notes_w15:
        o_r.note(f'{len(notes_w15)} R heads with a tender row moved vouched_by to another accepted vouch (W15 '
                 f'membership, refreshed): {notes_w15[:10]}')
    if notes_fx:
        o_r.note(f'{len(notes_fx)} eur-only moves on non-EUR heads published within 8 days, with a tender row (fresh FX, '
                 f'NOTE): {notes_fx[:10]}')
    if notes_eur:
        o_r.note(f'{len(notes_eur)} eur-only moves on re-dated heads with a tender row (explained, NOTE; D4\'s 2 % '
                 f'near-band rule in BAND): {notes_eur[:10]}')

    # -- watch rows (Rg, pins outside R, LOST heads)
    for sid, w in watch.items():
        i = int(sid)
        r = rows_r.get(i)
        if r is None:
            if i in retired_all:
                if not explained(sid, o_pins, f'watched {i} retired'):
                    o_pins.add(STOP, f'watched {w["kind"]} {i} RETIRED: explain')
            else:
                o_pins.add(NOT_MEASURED, f'watched {w["kind"]} {i}: no read')
            continue
        if i in W.NOT_LSA_PINS and W.lsa(r):
            o_pins.add(SWITCH_OFF, f'{i} serves lot-sum-awards (it must not: D3 / F)')
            continue
        if W.lsa(r):
            continue                                      # a NEW head (pin flips included): judged above
        vd = W.value_diff(w['row'], r)
        if not vd:
            adv['refresh'][sid] = r
            continue
        tr = i in tenders
        gap = ('current_value_derivation' in vd or 'current_value_derivation_detail' in vd) and \
            W.seq_increased(w['row'], r)
        desc = f'{w["kind"]}{"/" + w["pin"] if w.get("pin") else ""} {i}: {vd} moved'
        if w.get('pin') in W.STOP_EXPLAIN_ROLES:
            if explained(sid, o_pins, desc):
                adv['refresh'][sid] = r
            elif tr:
                o_pins.add(STOP, f'{desc} -- a value move on a path-V / must-stay head: stop and explain (W5)')
            else:
                info['silent'].append(i)
                if chain and gap:
                    o_pins.add(STOP, f'{desc} -- a value move on a W5 head with NO tender row and a seq increase: an '
                                     'announcement gap (stop, explain, file; W4/W6)')
                else:
                    o_pins.add(sil_code, f'{desc} -- a value move on a W5 head with NO tender row: SILENT{late_txt}',
                               esc=True)
            continue
        if not tr:
            if explained(sid, o_pins, f'SILENT {desc}'):
                adv['refresh'][sid] = r
            else:
                info['silent'].append(i)
                if chain and gap:
                    o_pins.add(STOP, f'SILENT {desc} with NO tender row: a fold announcement gap (seq increased): stop, '
                                     'explain, file (W6; in a chain never a switch-off, W4)')
                else:
                    o_pins.add(sil_code, f'SILENT {desc} with NO tender row: '
                               + ('a fold announcement gap (W6), a SILENT in class ' + klass if gap else
                                  'an unannounced writer') + late_txt, esc=True)
            continue
        o_pins.note(f'{desc} with a tender row (not lot-sum-awards; not a path-W input)')
        adv['refresh'][sid] = r

    # -- band
    try:
        brows = K.rows_of(band)
    except ValueError as e:
        o_band.add(NOT_MEASURED, f'band: {e}')
        brows = None
    if brows is not None:
        route = band.get('route', 'sql')
        if route == 'sql' and len(brows) >= 200:
            o_band.add(NOT_MEASURED, 'band seek returned its LIMIT (200): not the whole band')
        else:
            seen = {int(r['id']): r for r in brows}
            expected = set(reg['B_R'])
            for i in sorted(expected - set(seen)):
                r = rows_r.get(i) or {}
                near = W.near_100m(i, r.get('current_value_eur_cents'))
                rv = read_verdict(str(i))
                if rv == 'wrong':
                    o_band.add(SWITCH_OFF, f'B_R head {i} missing from the band and READ WRONG')
                elif chain:
                    if rv in ('right', 'neutral'):
                        o_band.note(f'B_R head {i} left the band, read {rv}')
                        adv['band_drop'].append(i)
                    else:
                        o_band.add(STOP, f'B_R head {i} missing from the band (eur {r.get("current_value_eur_cents")}): '
                                         'stop and read it at 0 wrong of any class; switch off only if it reads wrong')
                elif near:
                    if explained(str(i), o_band, f'B_R head {i} missing, within 2 % of EUR 100 m'):
                        adv['band_drop'].append(i)
                    else:
                        o_band.add(STOP, f'B_R head {i} missing, within 2 % of EUR 100 m (eur '
                                         f'{r.get("current_value_eur_cents")}): stop and explain')
                else:
                    o_band.add(SWITCH_OFF, f'a registered band head {i} is missing from the band')
            for i in sorted(set(seen) - expected):
                r = seen[i]
                eur = r.get('current_value_eur_cents')
                rv = read_verdict(str(i))
                if rv in ('right', 'neutral'):
                    o_band.note(f'extra band row {i} read {rv} at 0 wrong of any class ({reads[str(i)].get("ref", "")})')
                    adv['band_add'].append(i)
                elif rv == 'wrong':
                    o_band.add(SWITCH_OFF, f'extra band row {i} READ WRONG')
                elif W.near_100m(i, eur):
                    if explained(str(i), o_band, f'extra band row {i} within 2 % of EUR 100 m'):
                        adv['band_add'].append(i)
                    else:
                        o_band.add(STOP, f'extra band row {i} within 2 % of EUR 100 m (eur {eur}): stop and explain')
                elif i in new_tr:
                    o_band.add(STOP, f'extra band row {i} (NEW in this segment, eur {eur}): read it at 0 wrong of any '
                                     'class' + (' -- wrong means switch off (U9)' if chain else ''))
                elif str(i) in entries:
                    o_band.add(STOP, f'extra band row {i} (an R head crossed into the band, eur {eur}): read it at 0 '
                                     'wrong of any class')
                elif i not in tenders:
                    o_band.add(STOP, f'a lot-sum-awards band row {i} outside R and outside the segment (eur {eur}): an '
                                     'unread writer -- find it')
                else:
                    o_band.add(STOP, f'extra band row {i} (eur {eur}): read it at 0 wrong of any class')
            o_band.note(f'band route {route}; {len(brows)} rows; B_R {len(expected)}')

    # -- D7: an ADVISORY for the runs not yet started (never a verdict; finding: the decision defers them, D7)
    d7 = opts.get('d7')
    if d7 is not None:
        o7 = W.Out('D7 (advisory: the runs not yet started)')
        ev = SR.evaluate(d7['runs_left'], d7['now'], d7['minutes_per_run'], d7['fixed_minutes'], d7['gate'],
                         d7['read_minutes'], d7.get('switchoff_read_statements', 0))
        o7.note(SR.line(ev, d7['runs_left']))
        if d7['runs_left'] and not ev['go']:
            info['d7_hold'] = True
            o7.note(f'HOLD: start_rule NO-GO for the {d7["runs_left"]} run(s) left -- do not start the next run; it '
                    f'moves to the next gap (after the {ev["tick_kind"]} tick {ev["tick"]:%Y-%m-%dT%H:%MZ}, its segment '
                    'and the U9 check; 539 step 2: run c on 10-25). This run\'s verdict stands on its own reads.')
        outs.append(o7)
    info['cap'] = cap
    return outs, adv, info


def new_population(rows_all, ids, rec):
    ids = sorted(ids)
    return {'what': 'issue 551 W-on drains: the NEW heads of a segment (u8/delta.py population shape, E5)',
            'at': K.now_iso(), 'ids': ids, 'n': len(ids), 'ids_sha256': K.ids_sha(ids),
            'band_rows': [i for i in ids if W.eur_ge_100m(rows_all[i])],
            'rows': {str(i): rows_all[i] for i in ids}, 'seg': rec['seg'], 'tag': rec['tag'],
            'salt': (rec.get('prereg') or {}).get('salt')}


def _a_tie(a, rec):
    """-> (A, A_bad, meta): A must be this segment's -- its ids_sha256 and n equal the segment's changed live ids."""
    aj = K.jload(a.a_from)
    if not aj.get('complete'):
        raise SystemExit(f'{a.a_from}: the A count is incomplete')
    meta = {'A_file': a.a_from, 'A_sha256': K.sha_file(a.a_from)}
    ids_sha, n = aj.get('ids_sha256'), aj.get('n_named')
    if ids_sha is None:
        mp = os.path.join(os.path.dirname(a.a_from), 'manifest.json')
        if os.path.exists(mp):
            mm = K.jload(mp)
            ids_sha, n = mm.get('ids_sha256'), mm.get('n')
    meta['A_ids_sha256'] = ids_sha
    lp = rdir(a.seg, 'changed_live_ids.json')
    if not os.path.exists(lp):
        return None, f'A is not this segment\'s: {lp} is missing (gd5 read writes it)', meta
    live = K.jload(lp)
    if ids_sha != live['ids_sha256'] or (n is not None and int(n) != int(live['n'])):
        return None, (f'A is not this segment\'s: {a.a_from} counts ids_sha256 {str(ids_sha)[:16]} (n {n}), the '
                      f'segment\'s changed live ids are {live["ids_sha256"][:16]} (n {live["n"]}) -- build A from '
                      f'this read (ajobs.py build ... --from-seg {a.seg})'), meta
    return int(aj['A']), None, meta


def cmd_check(a):
    reg, rec = _common(a)
    if a.klass == 'W' and a.runs_left is None:
        raise SystemExit('class W needs --runs-left (the runs left after this one; 0 for the last run) for its D7 '
                         'advisory')
    frz = K.jload(W.FREEZE)
    rd = K.jload(rdir(a.seg, 'read.json'))
    if (rd['K'], rd['CA'], rd['class']) != (a.k, a.cursor_after, a.klass):
        raise SystemExit(f'read.json is for ({rd["K"]}, {rd["CA"]}] class {rd["class"]}: read again')
    rows_new, rows_r, band = {}, {}, None
    if not rd.get('shortcut') and rd.get('complete'):
        rows_new = K.load_rows(rdir(a.seg, 'new'))
        rows_r = K.load_rows(rdir(a.seg, 'rpk'))
        band = K.jload(rdir(a.seg, 'band.json'))
    A, A_bad, ameta = None, None, {}
    if a.a_from and not rd.get('shortcut'):
        A, A_bad, ameta = _a_tie(a, rec)
    now = SR.parse_now(a.now)
    d7 = None
    if a.runs_left is not None:
        d7 = {'runs_left': a.runs_left, 'now': now, 'minutes_per_run': a.minutes_per_run,
              'fixed_minutes': a.fixed_minutes, 'gate': SR.gate_of(a), 'read_minutes': a.read_minutes,
              'switchoff_read_statements': a.switchoff_read_statements or rd.get('switchoff_read_statements', 0)}
    recorded = SR.parse_now(rec['switchoff_by']) if rec.get('switchoff_by') else None
    given = SR.parse_now(a.gap_end) if a.gap_end else None
    gap_end = min(t for t in (recorded, given) if t is not None) if (recorded or given) else None
    own = set(int(x) for x in rec.get('own') or [])
    vals = [W.parse_corrected(s.get('counts')) for s in rec.get('started') or []
            if s.get('job_id') is not None and int(s['job_id']) in own]
    vals = [v for v in vals if v is not None]
    opts = {'klass': a.klass, 'corrected': a.corrected if a.corrected is not None else rec.get('corrected'), 'A': A,
            'A_bad': A_bad, 'explained': K.jload(a.explained) if a.explained else None,
            'prediction': K.jload(a.prediction) if a.prediction else None, 'gap_end': gap_end, 'now': now, 'd7': d7,
            'own_corrected': sum(vals) if vals else None}
    outs, adv, info = classify(reg, frz, rec, rd, rows_new, rows_r, band, opts)
    worst = PASS
    for o in outs:
        worst = W.worse(worst, o.code)
    rows_all = dict(rows_new)
    rows_all.update(rows_r)
    if info['new'] or info['silent_new']:
        pop = new_population(rows_all, sorted(set(info['new']) | set(info['silent_new'])), rec)
        K.jdump(os.path.join(SEG.seg_dir(a.seg), 'new_population.json'), pop)
    esc = [x for o in outs for x in o.stop_esc]
    escalate = 'gap_end' if (worst == STOP and esc and all(esc)) else None
    fmt = '%Y-%m-%dT%H:%M:%SZ'
    res = {'seg': a.seg, 'class': a.klass, 'tag': rec['tag'], 'K': a.k, 'CA': a.cursor_after, 'at': K.now_iso(),
           'now': now.strftime(fmt), 'gap_end': gap_end.strftime(fmt) if gap_end else None,
           'verdict': W.NAMES[worst], 'code': worst, 'rules': [o.rec() for o in outs], 'A': A, 'cap': info.get('cap'),
           'new': info['new'], 'moved': info['moved'], 'lost': info['lost'], 'silent': info['silent'],
           'advance': adv if worst == PASS else None, 'explained_file': a.explained,
           'explained_sha256': K.sha_file(a.explained) if a.explained else None,
           'prediction_sha256': K.sha_file(a.prediction) if a.prediction else None,
           'read_sha256': K.sha_file(rdir(a.seg, 'read.json')), 'd7_hold': info['d7_hold'], 'escalate': escalate}
    res.update(ameta)
    K.jdump(os.path.join(SEG.seg_dir(a.seg), 'result.json'), res)
    for o in outs:
        print(f'{o.rule}: {W.NAMES[o.code]}')
        for ln in o.lines:
            print(f'   {ln}')
    if worst != PASS:
        first = next((ln for o in outs for ln in o.lines if not ln.startswith('note:')), '')
        REG.freeze(a.seg, W.NAMES[worst], first, escalate=escalate,
                   gap_end=gap_end.strftime(fmt) if (gap_end and escalate) else None)
        print(f'{a.seg}: {W.NAMES[worst]} -- K frozen at {a.k}; the next writer is held'
              + (f' (escalates to SWITCH OFF at {gap_end:%Y-%m-%dT%H:%MZ} if still unexplained: registry.py status)'
                 if escalate and gap_end else ''))
    else:
        pend = [x['id'] for x in adv['add'] if x['read'] == 'pending']
        print(f'{a.seg}: PASS -- registry.py advance {a.seg} (K -> {a.cursor_after}; NEW {len(adv["add"])}, moved '
              f'{len(adv["moved"])}, lost {len(adv["lost"])})' + (f'; NEW reads pending {pend[:10]} (delta.py predraw/draw '
                                                                   f'--population {SEG.seg_dir(a.seg)}/new_population.json)'
                                                                   if pend else ''))
    if info['d7_hold']:
        print(f'D7 (advisory): HOLD the next run -- start_rule NO-GO for the {a.runs_left} run(s) left; they move to the '
              'next gap. This does not change the verdict above.')
    return worst


def main(argv=None, fake=None, http=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    for name in ('read', 'check'):
        p = sub.add_parser(name)
        p.add_argument('seg')
        p.add_argument('--k', type=int, required=True)
        p.add_argument('--cursor-after', type=int, required=True)
        p.add_argument('--class', dest='klass', required=True, choices=list(W.CLASSES))
        p.add_argument('--now', help='test only: the clock')
        if name == 'read':
            p.add_argument('--jobs', required=True, help='a fresh /admin/jobs body (idle; nothing since CA)')
            p.add_argument('--route', choices=['auto', 'e1', 'all', 'both'], default='auto')
            p.add_argument('--full', action='store_true')
            p.add_argument('--band-route', choices=['sql', 'api'], default='sql')
        else:
            p.add_argument('--corrected', type=int)
            p.add_argument('--a-from')
            p.add_argument('--explained')
            p.add_argument('--prediction')
            p.add_argument('--gap-end', help='an earlier end of the gap than the segment\'s recorded switch-off instant')
            p.add_argument('--runs-left', type=int)
            SR.add_args(p)
    a = ap.parse_args(argv)
    if a.cmd == 'read':
        return cmd_read(a, fake, http)
    return cmd_check(a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
