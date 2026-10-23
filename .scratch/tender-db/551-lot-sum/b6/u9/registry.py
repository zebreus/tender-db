"""Issue 551, path W on: the served registry R and its checkpoint K (decision-w-on-drains.md B2, E3, W9).

    python3 -I -B registry.py init    [--force [--discard-version N]]
    python3 -I -B registry.py advance <seg>
    python3 -I -B registry.py reads   --tag TAG --verdict PASS|FAIL --ref "<551 entry>"
    python3 -I -B registry.py status  [--now ISO] [--switchoff-by ISO] [--gap-end ISO] [--remaining-minutes M]
                                      [--no-plan-check]
    python3 -I -B registry.py show    [--id ID]

init     (offline, 0 statements) R0 from u9/20261023/switchoff.json (3,478 ids, ids_sha256 8845f0d0...; cursor_now =
         K0 = 959180852) and the archived PK reads beside it (cand_/changed_ tar.xz, read in memory): each entry holds
         its served row (cents, currency, eur, detail, vouched_by; seq and published_at for NOTEs), its source tag and
         the vouches its vouched_by is judged against (A4 membership):
           P (3,467)          vouches = the port's (u6/P_final.json, sha256 checked against freeze.json); the served
                              S, currency, n must equal P's and vouched_by must be a member -- else init refuses;
           delta-U8 (8)       u8/delta_read_set.json's ids, read 8 right (u8/delta_read_result.json); vouches = served;
           chain-<date> (3)   the remainder, which must be exactly 97901, 8859070, 8859315 (551 U9 days 1-2); <date> =
                              the day of its first tender row after U7's drain (cursor > 959026689) in the archived
                              all-kind enumeration; read status 'day7' (the day-7 population).
         Watch rows: Rg (freeze.json, 35) and every pin not in R, at their archived served rows (SILENT / PINS compare
         value columns against them, W5/W6). B_R = B_P (freeze.json, 29), checked equal to the 10-23 band seek and to R0's
         >= EUR 100 m entries. The jobs mark (B1: the jobs window every later segment checks from) starts at K0's
         /health read, 2026-10-23T18:48:01Z (no /admin/jobs body was saved on 10-23). Writes u9/registry/R.json;
         prints ids_sha256 and entries_sha256 (a digest of the content, stable across re-inits: record it in 551).
         --force re-writes R.json ONLY when it is still exactly an unadvanced R0 (version 0, not frozen, no segment
         open, nothing lost, no history, the same entries digest); anything else is refused (exit 2) unless
         --discard-version names the file's version. A forced re-write first copies the old file to
         registry/history/R_forced_v<version>_<stamp>.json.
advance  (offline) applies a segment's gd5 result (u9/segments/<seg>/result.json) when its verdict is PASS and its K
         is the registry's K: NEW ids enter as `<tag>` (class W/N/Z/D/I: read 'pending' -- the next writer waits for
         their reads, W9; class chain: read 'day7'), chain MOVED/LOST are recorded as counted re-decisions (tag
         chain-<date>), every read row refreshes its entry (so seq/published_at NOTEs do not repeat), explained band rows
         join B_R, and K = the segment's CA. The segment's CA jobs snapshot becomes the jobs mark (its newest job id and
         measured_at). Pending NEW ids carry the segment's recorded switch-off instant as their read deadline
         (read_by, W9); a segment with pending NEW ids and no recorded instant is refused (exit 3). Added entries keep
         their pin role; a class-I predicted move takes the port's vouches. The previous R.json is kept under
         registry/history/.
reads    marks the pending NEW entries of TAG as read: PASS -> 'pass'; FAIL -> 'fail' and the registry is frozen with
         SWITCH OFF (a failed NEW read; exit 10).
status   exit 0 READY for the next writer; 2 HOLD (K frozen by a STOP / NOT A MEASUREMENT, NEW reads pending, a
         segment open, or the plan probe / plan dbs not ready -- 0 statements); 10 SWITCH OFF when:
           - K is frozen by a SWITCH OFF;
           - K is frozen by a STOP made only of gap-end-escalating lines (an unexplained MOVED/LOST/SILENT in D/Z/N/W/I,
             an unannounced SILENT or the 3x count in a chain; F, W4) and now >= the end of the gap (the frozen
             segment's recorded switch-off instant, or an earlier --gap-end);
           - NEW reads are still pending at now >= their recorded read_by (or an earlier --switchoff-by): W9 counts
             reads unfinished at the gap's latest switch-off start as failed;
           - D7 in flight: an open class-D/W/I segment cannot finish (the run's planned minutes from its open, or now +
             --remaining-minutes for an overrun, plus its read minutes) before its latest switch-off start. Never
             cancel the project: §S starts at that instant and the switch-off queues behind it (B6/W11).
         now is the wall clock (--now is a test override).
A gd5 check that ends STOP, NOT A MEASUREMENT or SWITCH OFF freezes K (gd5 writes `frozen`); re-running the check with
the explanation recorded (gd5 check --explained) and advancing the same segment clears it.
Exit 3 on a tool fault (an input that is not what 551 recorded is a refusal with a message: also 3).
"""
import argparse
import datetime
import os
import shutil
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wkit as W  # noqa: E402
import start_rule as SR  # noqa: E402

K = W.K
ROW_COLS = K.TENDER_COLS


def pin_role(i):
    if i in K.PATH_W:
        return 'path-W'
    if i in K.PATH_V:
        return 'path-V'
    if i in K.TRIO:
        return 'trio'
    if i in K.MUST_STAY:
        return 'must-stay'
    if i in K.NOT_LSA:
        return 'not-lsa'
    if i == W.DOUBLE_COUNT_PIN:
        return 'double-count'
    if i in W.PARTIAL_SUMS:
        return 'partial-sum'
    return None


def slim(r):
    return {c: r.get(c) for c in ROW_COLS}


def content_sha(reg):
    body = {'K': reg['K'], 'B_R': reg['B_R'], 'entries': reg['entries'], 'watch': reg['watch'], 'lost': reg['lost']}
    return K.sha_bytes(K.jbytes(body))


def load(path=None):
    p = path or W.REG_FILE
    if not os.path.exists(p):
        raise SystemExit(f'{p}: no registry (run registry.py init first)')
    return K.jload(p)


def save(reg, path=None, keep_history=True):
    p = path or W.REG_FILE
    if keep_history and os.path.exists(p):
        old = K.jload(p)
        hd = os.path.join(os.path.dirname(p), 'history')
        os.makedirs(hd, exist_ok=True)
        shutil.copyfile(p, os.path.join(hd, f'R_v{int(old["version"]):04d}.json'))
    reg['n'] = len(reg['entries'])
    reg['ids_sha256'] = K.ids_sha([int(i) for i in reg['entries']])
    reg['entries_sha256'] = content_sha(reg)
    return K.jdump(p, reg)


# ---------------------------------------------------------------- init (R0)
def build_r0():
    so = K.jload(W.R0_FILE)
    ids = [int(i) for i in so['ids']]
    if so.get('ids_sha256') != W.R0_IDS_SHA or K.ids_sha(ids) != W.R0_IDS_SHA or len(ids) != W.R0_N:
        raise SystemExit(f'{W.R0_FILE}: not the R0 551 recorded ({len(ids)} ids, {so.get("ids_sha256", "")[:16]})')
    if so.get('cursor_now') != W.K0 or so.get('flip_cursor') != W.C1:
        raise SystemExit(f'{W.R0_FILE}: cursors {so.get("flip_cursor")} / {so.get("cursor_now")} != C1 {W.C1} / K0 {W.K0}')
    rows = W.archived_pk(W.ARCH_CAND)
    ch = W.archived_pk(W.ARCH_CHANGED)
    if set(rows) & set(ch):
        raise SystemExit('the archived cand and changed reads overlap: not the 10-23 build')
    rows.update(ch)
    lsa_ids = sorted(i for i, r in rows.items() if W.lsa(r))
    if lsa_ids != sorted(ids):
        raise SystemExit(f'the archived rows serving lot-sum-awards ({len(lsa_ids)}) != R0 ({len(ids)})')
    frz = K.jload(W.FREEZE)
    if K.sha_file(W.P_FINAL) != frz['P']['P_final_sha256']:
        raise SystemExit('u6/P_final.json is not the file freeze.json froze')
    P = {int(e['id']): e for e in K.jload(W.P_FINAL)['ships']}
    drs = K.jload(W.DELTA_READ_SET)
    delta = sorted(int(i) for i in drs['read'])
    dres = K.jload(W.DELTA_READ_RESULT)['result']
    if not dres.get('pass') or dres['tally'].get('wrong') != 0 or dres['tally'].get('n') != len(delta):
        raise SystemExit(f'u8 delta read result is not the recorded 8 right: {dres.get("tally")}')
    rest = sorted(set(ids) - set(P) - set(delta))
    if not set(P) <= set(ids) or not set(delta) <= set(ids) or tuple(rest) != tuple(sorted(W.DAILY_FIRES_R0)):
        raise SystemExit(f'R0 != P ∪ delta ∪ the 3 daily fires: P missing {sorted(set(P) - set(ids))[:5]}, delta '
                         f'missing {sorted(set(delta) - set(ids))}, remainder {rest[:10]}')
    arch = W.archived_changes()
    first = {}
    for r in arch:
        if r['entity_kind'] == 'tender' and int(r['cursor']) > W.U7_END and int(r['entity_id']) in W.DAILY_FIRES_R0:
            first.setdefault(int(r['entity_id']), r)
    entries, bad = {}, []
    for i in ids:
        r = slim(rows[i])
        det = K.detail_of(r)
        vk = K.vb_key(det.get('vouched_by'))
        if i in P:
            e = P[i]
            if (r['current_value_cents'] != e['S'] or r['current_value_currency'] != e['currency']
                    or det.get('lots') != e['n'] or vk not in [list(v) for v in e['vouches']]):
                bad.append(i)
            entries[str(i)] = {'tag': 'P', 'row': r, 'vouches': [list(v) for v in e['vouches']],
                               'vouches_from': 'port P_final (23cbd850)', 'read': 'pass',
                               'read_ref': 'U5 B-fresh + U7 D1 (fired as predicted) + U9 days 1-2 (0 off)'}
        elif i in delta:
            entries[str(i)] = {'tag': 'delta-U8', 'row': r, 'vouches': [vk], 'vouches_from': 'served, read right',
                               'read': 'pass', 'read_ref': 'U8 delta read 8 right / 0 wrong (wf_60e69470-890)'}
        else:
            fr = first.get(i)
            if fr is None:
                raise SystemExit(f'daily fire {i}: no tender row after U7\'s drain in the archive')
            entries[str(i)] = {'tag': f'chain-{W.day_of(fr["changed_at"])}', 'row': r, 'vouches': [vk],
                               'vouches_from': 'served (unread: day-7 population)', 'read': 'day7',
                               'read_ref': 'U9 day-7 read (10-28)',
                               'tag_basis': f'first tender row after U7 (cursor {fr["cursor"]}, {fr["op"]})'}
            if r.get('source') == 'doe':
                entries[str(i)]['flags'] = ['possibly 539-driven: a daily fire on a DÖE Tender before 539\'s '
                                            'segment (A6/H)']
        for k in ('seg', 'K_registered'):
            entries[str(i)][k] = 'R0' if k == 'seg' else W.K0
        entries[str(i)]['moves'] = []
        role = pin_role(i)
        if role:
            entries[str(i)]['pin'] = role
    if bad:
        raise SystemExit(f'{len(bad)} P ids not at P\'s S / currency / n / vouches in R0: {bad[:10]}')
    watch = {}
    for i in sorted((set(frz['Rg']) | set(K.pin_ids())) - set(ids)):
        if i not in rows:
            raise SystemExit(f'watch row {i} (Rg or pin) is not in the archived reads')
        watch[str(i)] = {'kind': 'Rg' if i in frz['Rg'] else 'pin', 'pin': pin_role(i), 'row': slim(rows[i])}
    band = K.jload(W.BAND_R0)
    band_ids = sorted(int(r[0]) for r in band['rows'])
    big = sorted(i for i in ids if W.eur_ge_100m(rows[i]))
    if sorted(frz['B_P']) != band_ids or band_ids != big:
        raise SystemExit(f'B_P {len(frz["B_P"])} / band 10-23 {len(band_ids)} / R0 >= EUR 100 m {len(big)} disagree')
    inputs = {os.path.relpath(p, W.B6): K.sha_file(p) for p in
              (W.R0_FILE, W.ARCH_CAND, W.ARCH_CHANGED, W.ARCH_CHANGES, W.BAND_R0, W.FREEZE, W.P_FINAL,
               W.DELTA_READ_SET, W.DELTA_READ_RESULT)}
    reg = {'what': 'issue 551 W-on drains: the served registry R and checkpoint K (decision B2/E3)', 'version': 0,
           'K': W.K0, 'C1': W.C1, 'B_R': band_ids, 'entries': entries, 'watch': watch, 'lost': {},
           'retired': sorted(int(i) for i in so.get('retired', [])), 'frozen': None, 'open_segment': None,
           'history': [], 'inputs': inputs, 'at': K.now_iso(),
           'jobs_mark': {'seg': 'R0', 'max_job_id': None, 'measured_at': W.K0_AT,
                         'basis': 'K0 /health read 2026-10-23T18:48:01Z (u6/health.log U9-1023); no jobs body saved'},
           'tags': W.counts(e['tag'] for e in entries.values())}
    return reg


def pristine_r0(old, fresh):
    return (int(old.get('version', -1)) == 0 and old.get('frozen') is None and old.get('open_segment') is None
            and not old.get('lost') and not old.get('history') and old.get('entries_sha256') == content_sha(fresh))


def cmd_init(a):
    if os.path.exists(W.REG_FILE) and not a.force:
        print(f'{W.REG_FILE} exists; init writes R0 once (a fresh checkout has no R.json and needs no --force)')
        return 2
    reg = build_r0()
    if os.path.exists(W.REG_FILE):
        old = load()
        if not pristine_r0(old, reg) and a.discard_version != old.get('version'):
            print(f'REFUSED: {W.REG_FILE} is not an unadvanced R0 (version {old.get("version")}, K {old.get("K")}, '
                  f'frozen {old.get("frozen")}, open segment {old.get("open_segment")}, lost {len(old.get("lost") or {})}, '
                  f'history {len(old.get("history") or [])}): --force would discard it. Only --discard-version '
                  f'{old.get("version")} does that, and only when 551 records why')
            return 2
        hd = os.path.join(os.path.dirname(W.REG_FILE), 'history')
        os.makedirs(hd, exist_ok=True)
        shutil.copyfile(W.REG_FILE, os.path.join(hd, f'R_forced_v{int(old.get("version", 0)):04d}_'
                                                     f'{K.now_iso().replace(":", "")}_{os.getpid()}.json'))
    sha = save(reg, keep_history=False)
    print(f'R0: {reg["n"]} entries {reg["tags"]}; ids_sha256 {reg["ids_sha256"]}; K0 {reg["K"]}; B_R {len(reg["B_R"])}; '
          f'watch {len(reg["watch"])} (Rg + pins outside R); entries_sha256 {reg["entries_sha256"]}; file sha256 '
          f'{sha} -- record ids_sha256, entries_sha256 and K0 in 551')
    return 0


# ---------------------------------------------------------------- freeze / advance / reads / status
def freeze(seg, verdict, reason, path=None, escalate=None, gap_end=None):
    """gd5 check calls this on a non-PASS verdict: K stays, the next writer is held (B2). escalate='gap_end': the STOP
    is made only of lines F / W4 turn into SWITCH OFF when still unexplained at the end of the gap (gap_end, ISO)."""
    reg = load(path)
    reg['frozen'] = {'seg': seg, 'verdict': verdict, 'reason': reason[:500], 'at': K.now_iso(), 'escalate': escalate,
                     'gap_end': gap_end}
    save(reg, path)


def cmd_advance(a):
    reg = load()
    rp = os.path.join(W.SEG_DIR, a.seg, 'result.json')
    res = K.jload(rp)
    if res.get('code') != W.PASS:
        print(f'REFUSED: {a.seg} ended {res.get("verdict")}: K stays frozen at {reg["K"]} (explain, re-run gd5 check '
              'with --explained, then advance)')
        return 2
    if int(res['K']) != int(reg['K']):
        print(f'REFUSED: {a.seg} was checked from K {res["K"]}, the registry is at K {reg["K"]}')
        return 2
    if reg.get('frozen') and reg['frozen'].get('seg') != a.seg:
        print(f'REFUSED: the registry is frozen by {reg["frozen"]["seg"]} ({reg["frozen"]["verdict"]}); that segment '
              'must pass first')
        return 2
    adv = res['advance']
    rec = K.jload(os.path.join(W.SEG_DIR, a.seg, 'segment.json'))
    if any(x['read'] == 'pending' for x in adv['add']) and not rec.get('switchoff_by'):
        raise SystemExit(f'{a.seg} registers NEW ids read-pending but its record has no switch-off instant (W9 needs '
                         'the read deadline): re-record the segment with this kit')
    added, moved, lost = [], [], []
    for x in adv['add']:
        i = str(x['id'])
        if i in reg['entries']:
            raise SystemExit(f'{i} is already in R')
        entry = {'tag': x['tag'], 'row': x['row'], 'vouches': x['vouches'], 'vouches_from': x['vouches_from'],
                 'read': x['read'], 'read_ref': x.get('read_ref', ''), 'seg': a.seg, 'K_registered': res['CA'],
                 'moves': []}
        if x['read'] == 'pending':
            entry['read_by'] = rec['switchoff_by']
        role = pin_role(int(i))
        if role:
            entry['pin'] = role
        if x.get('flags'):
            entry['flags'] = x['flags']
        reg['entries'][i] = entry
        reg['watch'].pop(i, None)
        added.append(int(i))
    for x in adv['moved']:
        e = reg['entries'][str(x['id'])]
        e['moves'].append({'seg': a.seg, 'tag': x['tag'], 'kind': x['kind'], 'fields': x['fields'],
                           'from': {c: e['row'].get(c) for c in W.VALUE_COLS},
                           'to': {c: x['row'].get(c) for c in W.VALUE_COLS}})
        e['row'] = x['row']
        if x.get('vouches'):
            e['vouches'] = [list(v) for v in x['vouches']]          # a class-I predicted move: the port's vouches
            e['vouches_from'] = f'port prediction ({a.seg})'
        else:
            vk = K.vb_key(K.detail_of(x['row']).get('vouched_by'))
            if vk is not None and vk not in e['vouches']:
                e['vouches'] = e['vouches'] + [vk]
        moved.append(x['id'])
    for x in adv['lost']:
        i = str(x['id'])
        e = reg['entries'].pop(i)
        e.update(lost_seg=a.seg, lost_tag=x['tag'], lost_row=x['row'])
        reg['lost'][i] = e
        if x['row'] is not None:
            reg['watch'][i] = {'kind': 'lost', 'pin': pin_role(int(i)), 'row': x['row']}
        lost.append(x['id'])
    for i, r in adv['refresh'].items():
        if i in reg['entries']:
            reg['entries'][i]['row'] = r
        elif i in reg['watch']:
            reg['watch'][i]['row'] = r
    for i in adv.get('band_add', []):
        if int(i) not in reg['B_R']:
            reg['B_R'] = sorted(reg['B_R'] + [int(i)])
    for i in adv.get('band_drop', []):
        reg['B_R'] = [x for x in reg['B_R'] if x != int(i)]
    reg['retired'] = sorted(set(reg.get('retired', [])) | {int(i) for i in adv.get('retired', [])})
    k_from = reg['K']
    reg['K'] = int(res['CA'])
    reg['version'] = int(reg['version']) + 1
    cj = rec.get('ca_jobs') or {}
    reg['jobs_mark'] = {'seg': a.seg, 'max_job_id': cj.get('max_job_id'), 'measured_at': cj.get('measured_at')}
    if reg.get('frozen') and reg['frozen'].get('seg') == a.seg:
        reg['frozen'] = None
    if reg.get('open_segment') == a.seg:
        reg['open_segment'] = None
    reg['history'].append({'version': reg['version'], 'seg': a.seg, 'class': res['class'], 'tag': res['tag'],
                           'K_from': k_from, 'K_to': reg['K'], 'added': added, 'moved': moved, 'lost': lost,
                           'ca_max_job_id': cj.get('max_job_id'), 'ca_measured_at': cj.get('measured_at'),
                           'result_sha256': K.sha_file(rp), 'at': K.now_iso()})
    sha = save(reg)
    pend = sorted(i for i in added if reg['entries'][str(i)]['read'] == 'pending')
    print(f'advanced by {a.seg} ({res["class"]}): K {k_from} -> {reg["K"]}; +{len(added)} NEW {added[:10]}, '
          f'{len(moved)} moved, {len(lost)} lost; R now {reg["n"]} (ids_sha256 {reg["ids_sha256"][:16]}, entries_sha256 '
          f'{reg["entries_sha256"][:16]}, file {sha[:16]})' + (f'; NEW reads PENDING {pend[:10]}: the next writer waits '
                                                               'for them (W9)' if pend else ''))
    return 0


def cmd_reads(a):
    reg = load()
    hit = [i for i, e in reg['entries'].items() if e['tag'] == a.tag and e['read'] == 'pending']
    if not hit:
        print(f'no pending entries tagged {a.tag}')
        return 2
    for i in hit:
        reg['entries'][i]['read'] = 'pass' if a.verdict == 'PASS' else 'fail'
        reg['entries'][i]['read_ref'] = a.ref
    if a.verdict == 'FAIL':
        reg['frozen'] = {'seg': a.tag, 'verdict': 'SWITCH OFF', 'reason': f'NEW read failed ({a.ref})',
                         'at': K.now_iso()}
    save(reg)
    print(f'{len(hit)} NEW entries of {a.tag} marked read {a.verdict} ({a.ref})')
    return W.SWITCH_OFF if a.verdict == 'FAIL' else 0


def _iso(t):
    return t.strftime('%Y-%m-%dT%H:%M:%SZ') if t is not None else None


def _min_t(*ts):
    ts = [t for t in ts if t is not None]
    return min(ts) if ts else None


def open_record(reg):
    seg = reg.get('open_segment')
    if not seg:
        return None
    p = os.path.join(W.SEG_DIR, seg, 'segment.json')
    return K.jload(p) if os.path.exists(p) else None


def status(reg, now=None, switchoff_by=None, gap_end=None, remaining_minutes=None):
    """-> (code, lines). now defaults to the clock (start_rule.parse_now: the wall clock, or a test's CLOCK)."""
    now = now or SR.parse_now(None)
    lines = [f'K {reg["K"]}; version {reg["version"]}; R {len(reg["entries"])} '
             f'{W.counts(e["tag"] for e in reg["entries"].values())}; B_R {len(reg["B_R"])}; now {_iso(now)}']
    pend = sorted(int(i) for i, e in reg['entries'].items() if e['read'] == 'pending')
    code = W.PASS
    fz = reg.get('frozen')
    if fz:
        c = W.SWITCH_OFF if fz.get('verdict') == 'SWITCH OFF' else W.STOP
        end = _min_t(SR.parse_now(fz['gap_end']) if fz.get('gap_end') else None, gap_end)
        if c == W.STOP and fz.get('escalate') == 'gap_end' and end is not None and now >= end:
            c = W.SWITCH_OFF
            lines.append(f'SWITCH OFF: {fz["seg"]} is still unexplained at the end of the gap ({_iso(end)}; F/W4): '
                         f'{fz["reason"]}')
        else:
            lines.append(f'{W.NAMES[c] if c == W.SWITCH_OFF else "HOLD"}: K frozen by {fz["seg"]} ({fz["verdict"]}'
                         + (f'; escalates to SWITCH OFF at {_iso(end)} if still unexplained'
                            if fz.get('escalate') == 'gap_end' and end is not None else '') + f'): {fz["reason"]}')
        code = W.worse(code, c)
    if pend:
        by = [SR.parse_now(reg['entries'][str(i)]['read_by']) for i in pend if reg['entries'][str(i)].get('read_by')]
        deadline = _min_t(min(by) if by else None, switchoff_by)
        late = deadline is not None and now >= deadline
        c = W.SWITCH_OFF if late else W.STOP
        lines.append(f'{"SWITCH OFF" if late else "HOLD"}: {len(pend)} NEW read(s) pending {pend[:10]}'
                     + (f' -- unfinished at the latest switch-off start {_iso(deadline)}: they count as FAILED (W9)'
                        if late else f' -- the next writer waits for them (W9; read by {_iso(deadline)})'))
        code = W.worse(code, c)
    if reg.get('open_segment'):
        rec = open_record(reg)
        c = W.STOP
        msg = f'HOLD: segment {reg["open_segment"]} is open (its check has not closed)'
        if rec and rec.get('status') == 'open' and rec.get('class') in ('D', 'W', 'I') and rec.get('opened_at'):
            sb = _min_t(SR.parse_now(rec['switchoff_by']) if rec.get('switchoff_by') else None, switchoff_by)
            planned = SR.parse_now(rec['opened_at']) + datetime.timedelta(minutes=int(rec.get('minutes_per_run') or 0))
            reads = datetime.timedelta(minutes=int(rec.get('read_minutes') or 0))
            if remaining_minutes is not None:
                est = max(planned, now + datetime.timedelta(minutes=remaining_minutes)) + reads
            else:
                est = max(planned, now) + reads
            if sb is not None and est > sb:
                c = W.SWITCH_OFF
                msg = (f'SWITCH OFF: the open {rec["class"]} run {rec["seg"]} cannot finish (estimate {_iso(est)}, its '
                       f'reads included) before its latest switch-off start {_iso(sb)} (D7, in flight): start §S at '
                       f'{_iso(sb)}; never cancel its project -- the switch-off queues behind it (B6/W11)')
            elif sb is not None:
                msg += f'; D7 in flight: estimate {_iso(est)} fits before the latest switch-off start {_iso(sb)}'
        lines.append(msg)
        code = W.worse(code, c)
    if code == W.PASS:
        lines.append('READY: no segment open, no NEW read pending, K not frozen')
    return code, lines


def cmd_status(a):
    reg = load()
    code, lines = status(reg, SR.parse_now(a.now) if a.now else None,
                         SR.parse_now(a.switchoff_by) if a.switchoff_by else None,
                         SR.parse_now(a.gap_end) if a.gap_end else None, a.remaining_minutes)
    if not a.no_plan_check:
        probs = W.plan_ready()
        for pr in probs:
            lines.append(f'HOLD: the plan probe is not ready (every box statement, the switch-off\'s too, would be '
                         f'refused): {pr}')
        if probs:
            code = W.worse(code, W.STOP)
        else:
            lines.append('plan probe ready: plan-probe copy, plan.db, plan_w.db; E1 / PK / band / all-kind shapes clean')
    for ln in lines:
        print(ln)
    return code


def cmd_show(a):
    reg = load()
    if a.id:
        i = str(a.id)
        print(K.jbytes(reg['entries'].get(i) or reg['watch'].get(i) or reg['lost'].get(i) or {'absent': i}).decode())
        return 0
    for ln in status(reg)[1]:
        print(ln)
    for h in reg['history'][-10:]:
        print(f'  v{h["version"]} {h["seg"]} ({h["class"]}, {h["tag"]}): K {h["K_from"]} -> {h["K_to"]}; '
              f'+{len(h["added"])} moved {len(h["moved"])} lost {len(h["lost"])}')
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    i = sub.add_parser('init')
    i.add_argument('--force', action='store_true')
    i.add_argument('--discard-version', type=int)
    v = sub.add_parser('advance')
    v.add_argument('seg')
    r = sub.add_parser('reads')
    r.add_argument('--tag', required=True)
    r.add_argument('--verdict', choices=['PASS', 'FAIL'], required=True)
    r.add_argument('--ref', required=True)
    s = sub.add_parser('status')
    s.add_argument('--now', help='test only: the clock')
    s.add_argument('--switchoff-by', help='an earlier read deadline than the recorded one (never later)')
    s.add_argument('--gap-end', help='an earlier end of the gap than the frozen segment\'s recorded one')
    s.add_argument('--remaining-minutes', type=int, help='D7 in flight: the open run\'s remaining minutes (overrun)')
    s.add_argument('--no-plan-check', action='store_true', help='skip the 0-statement plan readiness check')
    sh = sub.add_parser('show')
    sh.add_argument('--id', type=int)
    a = ap.parse_args(argv)
    return {'init': cmd_init, 'advance': cmd_advance, 'reads': cmd_reads, 'status': cmd_status,
            'show': cmd_show}[a.cmd](a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
