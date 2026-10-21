"""Issue 551 B6 U6 step 1: enumerate L_changed (decision U6 + C1/C3/C9 + the issue's records), the 2630-excluded reading.

    python3 -I -B enum_lchanged.py fetch-changes --cursor-now N [--from 958787061] [--dir u6/lchanged/changes]
    python3 -I -B enum_lchanged.py fetch-notices [--dir u6/lchanged/notices]
    python3 -I -B enum_lchanged.py build --cursor-now N [--rederive DAY:MOVED:START_ISO:END_ISO ...]
                                         [--changes-dir DIR ...] [--notices-dir DIR] [--set541 FILE] [--out FILE]

fetch-changes  every `changes` row in (958787061, N] (N = /health's cursor read after the 10-21 chain's project and
               before anything else is enqueued), bounded: cursor is the rowid, chunks of <= 5,000 cursor values
               (so <= 5,000 rows), each lot row carrying its Tender (lots PK seek). Resumable per chunk.
fetch-notices  the ingest cross-check, bounded: (1) one generate_series probe (<= 1,001 points, two PK seeks each) over
               notice ids from 47,406,725 (the newest notice the 3,502 name, ingested 2026-10-16, before the cutoff) to
               find the bucket where ingested_at crosses 2026-10-18T20:30:00Z and the highest id; (2) the boundary bucket
               (<= 1,000 ids) and the top bucket by PK range; (3) every notice id >= N0 with the Tender versions it caused
               (tender_versions_notice seek), chunks of <= 5,000 notice ids. ingested_at is assigned at insert in id
               order (store lib.rs import_lag; asserted monotonic on every row read here).
build          offline. Writes l_changed.json (default u6/l_changed.json): the union, per-source counts, L_changed ∩ 3,502
               (the refetch list) and every anomaly. Exit 0 clean; 2 a STOP anomaly (read it before U6 goes on).

THE UNION (each source's count and its overlap with the 3,502 are recorded):
  S1 tender rows   entity 'tender' rows in (958787061, N], EXCLUDING switch-off drain-back 2630's rows (cursor
                   958787062-958809773, version_seq NULL, changed_at 2026-10-19T03:34:07Z = 1792380847);
  S2 lot rows      entity 'lot' rows in the same range, mapped to their Tender (lots.tender_id), same exclusion;
  S3 541           its project 2632's rows (cursor (958809778, 958809905], inside S1/S2) plus, if given, a file of the
                   whole refolded set (2631 stamped 753 / 2632 folded 820). No bounded read enumerates the 627 verified +
                   67 touched (the carriers are a full notice_texts walk, 40 min): recorded as a residual when no file;
  S4 571           b6/l_changed_571_ids.json (66 corrected) plus its project 2636's rows (cursor (958809905, 958810037]);
                   2636 wrote 67: the 1 verified has no row and no bounded enumeration (notice_ids walk): residual;
  S5 545           u6/l_changed_545_whole.json: the 48 Tenders of 2651's 62 moved notices (47 corrected + 1 verified);
  S6 rederive      rederive-eur-recent's moved Tenders for 10-19, 10-20, 10-21: its D5 rows are seq-less `tender changed`
                   rows (rates.rs rederive_eur_window: "each moved Tender gets a seq-less tender changed") inside the
                   range; --rederive DAY:MOVED:START:END (the job's started_at/finished_at and its moved count from
                   GET /admin/jobs) confirms the count of distinct ids with such a row in [START, END) equals MOVED --
                   or [START, END], or [START, END) plus the first of END's second in cursor order (the trailing
                   project can stamp END's second too; the rederive commits first); a mismatch prints the stamps;
  S7 530           530's drain's rows (cursor (958904203, 958919213], inside S1/S2);
  S8 ingest        the Tenders whose versions are caused by a notice FIRST INGESTED at/after 2026-10-18T20:30:00Z
                   (notice id >= N0, N0 = the first id with ingested_at >= 1792355400). "Folded since" (C3) has no
                   per-notice instant in the store: a notice ingested earlier but first parsed later (a reclaim or
                   re-parse) is not visible here; any fold of one that moved served facts wrote a change row (S1/S2).
Organization rows are counted, not mapped (an org head change is no Tender fold input; a mention move writes a
`tender changed` row, which S1 holds).
ROW KINDS (kitlib.classify_changes): the store writes six kinds (tender | lot | organization | lot_result | bid |
contract). lot_result / bid / contract rows are counted per kind and op, not mapped: a version diff writes them only
when the rounds changed, and then always a tender row for the same Tender (append_version_changes). Removals are
ordinary: an orphan sweep or a retirement writes `removed` rows and then deletes the entity rows, so a removed lot's
Tender reads NULL -- accepted and counted, as is an added/changed row of a lot a later row removes; only a lot that
vanished with NO removed row is a STOP. A Tender whose last tender row is `removed` was retired (no tenders row): it
is taken out of L_changed (it cannot be refolded or PK-read) and listed under "retired"; one of the 3,502 retired is a
STOP. Rows of an instant with no tender row (a sweep that moved no fact) are an ANOMALY, listed.
DEVIATIONS recorded in l_changed.json (RUNBOOK U6.6 records them in 551 before the flip deploy): 541's / 571's whole
refolded sets are only partly enumerable (C1/C3), and C3's "folded since" is read as "first ingested since" (U2
correction 2).

    python3 -I -B enum_lchanged.py classify --dir DIR
classify       U7.2: the rows of a fetch-changes dir by kind/op; exit 2 on any tender / lot / result row (a deploy
               writes none), 0 otherwise.
"""
import argparse
import collections
import os
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402

LCH = os.path.join(K.U6, 'lchanged')
NEWEST_NAMED_NOTICE = 47406725          # U2: the newest notice the 3,502 name, ingested 2026-10-16T07:35:45Z
NOTICE_PROBE_SPAN = 1_000_000
NOTICE_PROBE_STEP = 1000
EXPECT_2630_ROWS = 3502


# ---------------------------------------------------------------- fetch
def cmd_fetch_changes(a, fake=None):
    box = K.shared_box(fake)
    rows, complete = K.enumerate_changes(box, a.from_cursor, a.cursor_now, a.dir)
    print(f'change rows ({a.from_cursor}, {a.cursor_now}]: {len(rows)}; complete={complete}')
    return 0 if complete else 3


def cmd_fetch_notices(a, fake=None):
    box = K.shared_box(fake)
    os.makedirs(a.dir, exist_ok=True)
    lo = NEWEST_NAMED_NOTICE
    body = box.sq(K.sql_notice_probe(lo, lo + NOTICE_PROBE_SPAN, NOTICE_PROBE_STEP), 'notice_probe')
    if body is None:
        return 3
    K.jdump(os.path.join(a.dir, 'probe.json'), body)
    probes = K.rows_of(body)
    hits = [p for p in probes if p['nid'] is not None]
    if not hits or hits[0]['ingested_at'] >= K.CUTOFF:
        print(f'FAIL: the probe window does not straddle the cutoff ({hits[:1]})')
        return 2
    if probes[-1]['nid'] is not None:
        print('FAIL: the probe window ends inside the notice ids (widen NOTICE_PROBE_SPAN)')
        return 2
    for x, y in zip(hits, hits[1:]):
        assert x['ingested_at'] <= y['ingested_at'] and x['nid'] <= y['nid'], (x, y)
    first_after = next((p for p in hits if p['ingested_at'] >= K.CUTOFF), None)
    if first_after is None:
        n0 = None
        print('no notice ingested at/after the cutoff')
    else:
        prev = hits[hits.index(first_after) - 1]
        b = box.sq(K.sql_notice_bucket(prev['nid'], first_after['nid']), 'notice_boundary')
        if b is None:
            return 3
        K.jdump(os.path.join(a.dir, 'boundary.json'), b)
        rr = K.rows_of(b)
        for x, y in zip(rr, rr[1:]):
            assert x['ingested_at'] <= y['ingested_at'], (x, y)
        n0 = next(r['id'] for r in rr if r['ingested_at'] >= K.CUTOFF)
    last = hits[-1]
    t = box.sq(K.sql_notice_bucket(last['nid'] - 1, last['probe'] + NOTICE_PROBE_STEP), 'notice_top')
    if t is None:
        return 3
    K.jdump(os.path.join(a.dir, 'top.json'), t)
    nmax = max(r['id'] for r in K.rows_of(t))
    rec = {'cutoff': K.CUTOFF_ISO, 'cutoff_epoch': K.CUTOFF, 'n0': n0, 'nmax': nmax, 'chunks': []}
    if n0 is not None:
        a0 = n0
        while a0 <= nmax:
            b0 = min(a0 + 5000, nmax + 1)
            body = box.sq(K.sql_notice_tenders(a0, b0), f'notice_tenders_{a0}_{b0}')
            if body is None:
                rec['complete'] = False
                K.jdump(os.path.join(a.dir, 'manifest.json'), rec)
                return 3
            sha = K.jdump(os.path.join(a.dir, f'nt_{a0}_{b0}.json'), body)
            rec['chunks'].append({'lo': a0, 'hi': b0, 'rows': len(body['rows']), 'sha256': sha})
            a0 = b0
    rec['complete'] = True
    K.jdump(os.path.join(a.dir, 'manifest.json'), rec)
    print(f'N0={n0} NMAX={nmax} chunks={len(rec["chunks"])}')
    return 0


# ---------------------------------------------------------------- build (offline)
def load_change_dirs(dirs):
    rows = []
    ranges = []
    for d in dirs:
        man = K.jload(os.path.join(d, 'manifest.json'))
        if not man.get('complete'):
            raise SystemExit(f'{d}: the change enumeration is incomplete')
        ranges.append((man['lo'], man['hi']))
        for key, rec in sorted(man['chunks'].items(), key=lambda kv: int(kv[0].split('_')[0])):
            fp = os.path.join(d, f'chunk_{key}.json')
            assert K.sha_file(fp) == rec['sha256'], fp
            rows += K.rows_of(K.jload(fp))
    return rows, ranges


def load_notice_dir(d):
    man = K.jload(os.path.join(d, 'manifest.json'))
    if not man.get('complete'):
        raise SystemExit(f'{d}: the notice cross-check is incomplete')
    rows = []
    for c in man['chunks']:
        fp = os.path.join(d, f'nt_{c["lo"]}_{c["hi"]}.json')
        assert K.sha_file(fp) == c['sha256'], fp
        rows += K.rows_of(K.jload(fp))
    return rows, man


def parse_rederive(specs):
    out = []
    for s in specs or []:
        parts = s.split(':', 2)
        day, moved = parts[0], int(parts[1])
        start = end = None
        if len(parts) == 3:
            rest = parts[2]
            # START_ISO:END_ISO, each 'YYYY-MM-DDTHH:MM:SSZ' (colons inside), split at 'Z:'
            st, en = rest.split('Z:', 1)
            start, end = K.epoch_of(st + 'Z'), K.epoch_of(en if en.endswith('Z') else en + 'Z')
        out.append({'day': day, 'moved': moved, 'start': start, 'end': end})
    return out


def in_rng(c, rng):
    return rng[0] < c <= rng[1]


def rederive_ids(kept, rd):
    """The rederive's D5 rows (seq-less `tender changed`, rates.rs: each moved Tender's row is stamped with its
    window's `now`, taken at the window's start, so START <= stamp <= END). The trailing `project` stamps its own
    seq-less corrections with now_unix() at ITS start, which can be END's second; the rederive commits before the project
    starts, so within END's second the rederive's rows come first in cursor order. -> (ids, how, ok, stamps)."""
    seqless = [r for r in kept if r['entity_kind'] == 'tender' and r['version_seq'] is None]
    lo_ids, at_end = set(), []
    for r in sorted(seqless, key=lambda r: int(r['cursor'])):
        c, t = int(r['changed_at']), int(r['entity_id'])
        if rd['start'] <= c < rd['end']:
            lo_ids.add(t)
    for r in sorted(seqless, key=lambda r: int(r['cursor'])):
        t = int(r['entity_id'])
        if int(r['changed_at']) == rd['end'] and t not in lo_ids and t not in at_end:
            at_end.append(t)
    m = rd['moved']
    if len(lo_ids) == m:
        return lo_ids, '[START, END)', True, None
    if len(lo_ids) + len(at_end) == m:
        return lo_ids | set(at_end), '[START, END]', True, None
    if len(lo_ids) < m < len(lo_ids) + len(at_end):
        k = m - len(lo_ids)
        return (lo_ids | set(at_end[:k]), f'[START, END) + the first {k} of {len(at_end)} Tenders stamped END in cursor '
                f'order (the next job shares END\'s second; the rederive committed first)', True, None)
    stamps = collections.Counter(int(r['changed_at']) for r in seqless
                                 if rd['start'] - 120 <= int(r['changed_at']) <= rd['end'] + 120)
    return lo_ids | set(at_end), 'no reading matches', False, {K.iso_of(k): v for k, v in sorted(stamps.items())}


DEVIATIONS = [
    {'id': 'C1/C3 whole refolded sets of 541 and 571',
     'binding': 'C1, C3 and the issue\'s U2 checker correction (3): L_changed takes the WHOLE refolded sets (541: 2631 '
                'stamped 753 / 2632 folded 820; 571: 2636 wrote 67)',
     'kit': 'only the ids with a change row are enumerable (541: its 126 corrected; 571: its 66 corrected, plus the file); '
            '541\'s 627 verified + 67 touched and 571\'s 1 verified have no change row and no bounded enumeration (their '
            'carriers are a full notice_texts / notice_ids walk, ~40 / ~8 min, no on-box path)',
     'why acceptable': 'verified = the refold\'s served output compared identical; 541 and 571 change served rows only '
                       '(result rows / winners), so a verified Tender\'s stored facts did not move; none of the 3,502\'s '
                       'notices is a 541/571 carrier (U2); a missed fire only loses a fix',
     'how D5 treats them': 'as unchanged: they are outside L_changed, so a lot-sum-awards fire on one in any touched '
                           'expansion is a D5 SWITCH OFF (the premise that they compared identical would be broken)',
     'record': 'issue 551, before the flip deploy (RUNBOOK U6.6)'},
    {'id': 'C3 "folded since" read as "first ingested since"',
     'binding': 'C3: the ingest cross-check takes notices FOLDED since 2627; the U2 checker correction (2) fixed the '
                'notice criterion as "first ingested since 2026-10-18 20:30Z" (every notice was re-folded by '
                '2627/2630/2648, so "folded" read literally names every notice)',
     'kit': 'S8 = tender_versions.tender_id of notices with id >= N0 (the first ingested at/after the cutoff); a notice '
            'ingested earlier but first parsed later is not visible per notice; any fold of one that moved served facts '
            'wrote a change row (S1/S2)',
     'record': 'issue 551, before the flip deploy (RUNBOOK U6.6)'},
]


def build(change_rows, ranges, notice_rows, notice_man, cursor_now, the3502, s571_file, s545_whole, rederive,
          set541=None):
    ids3502 = set(the3502)
    anomalies, stops, notes = [], [], []
    cur = [int(r['cursor']) for r in change_rows]
    if cur != sorted(cur) or len(set(cur)) != len(cur):
        stops.append('change rows are not strictly increasing in cursor')
    lo = min(r[0] for r in ranges) if ranges else K.SINCE
    hi = max(r[1] for r in ranges) if ranges else cursor_now
    if lo != K.SINCE or hi != cursor_now:
        stops.append(f'the enumerated range ({lo}, {hi}] is not ({K.SINCE}, {cursor_now}]')
    span = sorted(ranges)
    for (a1, b1), (a2, b2) in zip(span, span[1:]):
        if b1 != a2:
            stops.append(f'enumerated ranges leave a gap or overlap: ({a1},{b1}] then ({a2},{b2}]')
    kinds = collections.Counter(r['entity_kind'] for r in change_rows)
    cls = K.classify_changes(change_rows)
    for k in cls['unknown_kinds']:
        stops.append(f'unknown entity_kind {k!r} ({kinds[k]} rows): not one of the six the store writes')
    bad_lots = set(cls['unmapped_lots']['bad'])
    retired = cls['retired']

    def is2630(r):
        return (K.J2630_LO <= int(r['cursor']) <= K.J2630_HI and r['version_seq'] is None
                and int(r['changed_at']) == K.J2630_STAMP)

    excl = [r for r in change_rows if is2630(r)]
    excl_t = [r for r in excl if r['entity_kind'] == 'tender']
    excl_l = [r for r in excl if r['entity_kind'] == 'lot']
    excl_o = [r for r in excl if r['entity_kind'] == 'organization']
    excl_x = collections.Counter(r['entity_kind'] for r in excl if r['entity_kind'] in K.RESULT_KINDS)
    if len(excl_t) != EXPECT_2630_ROWS or {int(r['entity_id']) for r in excl_t} != ids3502:
        stops.append(f'2630-excluded reading does not hold as recorded: {len(excl_t)} excluded tender rows on '
                     f'{len({int(r["entity_id"]) for r in excl_t})} ids (expected exactly the 3,502, one row each)')
    excl_lot_outside = sorted({int(r['lot_tender_id']) for r in excl_l
                               if r['lot_tender_id'] is not None and int(r['lot_tender_id']) not in ids3502})
    excl_lot_bad = [int(r['cursor']) for r in excl_l if int(r['cursor']) in bad_lots]
    excl_lot_gone = sum(1 for r in excl_l if r['lot_tender_id'] is None and int(r['cursor']) not in bad_lots)
    if excl_lot_outside:
        anomalies.append(f'{len(excl_lot_outside)} Tenders outside the 3,502 have 2630-stamped lot rows: INCLUDED in '
                         f'L_changed (conservative): {excl_lot_outside[:20]}')
    if excl_lot_bad:
        stops.append(f'{len(excl_lot_bad)} 2630-stamped lot rows map to no Tender and no later row removes their lot '
                     f'(cursors {excl_lot_bad[:10]})')
    if excl_lot_gone:
        notes.append(f'{excl_lot_gone} 2630-stamped lot rows map to no Tender because a later row in the range removed '
                     'their lot (swept or retired)')
    stamp_outside = [int(r['cursor']) for r in change_rows
                     if int(r['changed_at']) == K.J2630_STAMP and r['version_seq'] is None and not is2630(r)]
    if stamp_outside:
        anomalies.append(f'{len(stamp_outside)} version-null rows stamped 03:34:07Z OUTSIDE 958787062-958809773 are '
                         f'INCLUDED (cursors {stamp_outside[:10]})')
    in2630_kept = [int(r['cursor']) for r in change_rows
                   if K.J2630_LO <= int(r['cursor']) <= K.J2630_HI and not is2630(r)]
    excl_cursors = {int(r['cursor']) for r in excl}
    kept = [r for r in change_rows if int(r['cursor']) not in excl_cursors]
    # 2630's lot rows on the 3,502 are excluded; its lot rows mapping outside are kept (above)
    kept_extra_2630 = [r for r in excl_l if r['lot_tender_id'] is not None and int(r['lot_tender_id']) not in ids3502]

    def tid(r):
        if r['entity_kind'] == 'tender':
            return int(r['entity_id'])
        if r['entity_kind'] == 'lot':
            return None if r['lot_tender_id'] is None else int(r['lot_tender_id'])
        return None                     # organization and result rows: counted, not mapped (classify_changes)

    kept_bad = [int(r['cursor']) for r in kept if int(r['cursor']) in bad_lots]
    if kept_bad:
        stops.append(f'{len(kept_bad)} added/changed lot rows map to no Tender and no later row in the range removes '
                     f'their lot (cursors {kept_bad[:10]}): a lot vanished without a removed row')
    unm = cls['unmapped_lots']
    if unm['removed'] or unm['gone_later']:
        notes.append(f'{len(unm["removed"])} removed lot rows (the lot row was deleted: orphan sweep or retirement) and '
                     f'{len(unm["gone_later"])} added/changed lot rows of a lot removed later in the range map to no '
                     'Tender (expected: their Tender\'s own tender row carries it, or it was retired)')
    if cls['no_tender_instant']:
        anomalies.append(f'{len(cls["no_tender_instant"])} unmapped lot / result rows carry an instant with no tender row '
                         f'in the range (a sweep with no fact change of its Tender; its served facts did not move): '
                         f'cursors {cls["no_tender_instant"][:20]}')
    if retired:
        notes.append(f'{len(retired)} Tenders retired in the range (last tender row `removed`: no tenders row, never '
                     f'PK-readable): taken out of L_changed and listed under "retired": {sorted(retired)[:20]}')
    if retired & ids3502:
        stops.append(f'{len(retired & ids3502)} Tenders of the 3,502 were RETIRED in the range '
                     f'({sorted(retired & ids3502)[:10]}): the 3,502, P and stage 1/2 must be re-decided before U6 goes '
                     'on (record in 551; the kit assumes all 3,502 exist)')
    S1 = {tid(r) for r in kept if r['entity_kind'] == 'tender'}
    S2 = {tid(r) for r in kept + kept_extra_2630 if r['entity_kind'] == 'lot' and tid(r) is not None}
    rows541 = {tid(r) for r in kept if in_rng(int(r['cursor']), K.R541) and tid(r) is not None}
    rows571 = {tid(r) for r in kept if in_rng(int(r['cursor']), K.R571) and tid(r) is not None}
    rows530 = {tid(r) for r in kept if in_rng(int(r['cursor']), K.R530) and tid(r) is not None}
    S3 = set(rows541) | set(set541 or [])
    S4 = set(s571_file) | rows571
    S5 = set(s545_whole)
    # S6: rederive D5 rows, per day
    S6, rederive_rec = set(), []
    for rd in rederive:
        if rd['start'] is None:
            cands = collections.Counter(int(r['changed_at']) for r in kept if r['entity_kind'] == 'tender'
                                        and r['version_seq'] is None
                                        and K.iso_of(int(r['changed_at'])).startswith(rd['day']))
            rederive_rec.append({'day': rd['day'], 'moved': rd['moved'], 'confirmed': None,
                                 'note': 'no job window given: seq-less tender row stamps of that day listed',
                                 'seqless_stamps': {K.iso_of(k): v for k, v in sorted(cands.items())}})
            stops.append(f'rederive {rd["day"]}: give START:END from GET /admin/jobs to confirm its D5 rows')
            continue
        ids, how, ok, stamps = rederive_ids(kept, rd)
        rederive_rec.append({'day': rd['day'], 'moved': rd['moved'], 'window': [K.iso_of(rd['start']),
                                                                               K.iso_of(rd['end'])],
                             'd5_ids_in_range': len(ids), 'confirmed': ok, 'reading': how, 'ids': sorted(ids),
                             'seqless_stamps_near_window': stamps})
        if not ok:
            stops.append(f'rederive {rd["day"]}: {len(ids)} seq-less tender rows in its window, job moved '
                         f'{rd["moved"]}: its D5 rows are not all in the range (or the window is wrong); seq-less stamps '
                         f'within 2 min of the window: {stamps}')
        S6 |= ids
    days = {rd['day'] for rd in rederive}
    for d in ('2026-10-19', '2026-10-20', '2026-10-21'):
        if d not in days:
            stops.append(f'rederive {d}: not given (--rederive {d}:MOVED:START:END)')
    S7 = rows530
    # S8: the ingest cross-check
    S8, nt_stats = set(), {}
    if notice_rows is not None:
        S8 = {int(r['tender_id']) for r in notice_rows if r['tender_id'] is not None}
        nids = sorted({int(r['id']) for r in notice_rows})
        no_version = sorted({int(r['id']) for r in notice_rows if r['tender_id'] is None})
        states = collections.Counter((r['parse_state'], r['projected']) for r in notice_rows if r['tender_id'] is None)
        pend = sum(1 for r in notice_rows if r['parse_state'] == 'parsed' and r['projected'] == 0)
        nt_stats = {'n0': notice_man.get('n0'), 'nmax': notice_man.get('nmax'), 'notices': len(nids),
                    'notices_with_no_version': len(no_version),
                    'no_version_by_state': {f'{k[0]}/projected={k[1]}': v for k, v in sorted(states.items())},
                    'parsed_not_projected': pend}
        if pend:
            stops.append(f'{pend} notice rows parsed but not projected: the chain\'s project has not finished')
    elif notice_man is not None:
        nt_stats = {'n0': None, 'notices': 0}
    else:
        stops.append('the ingest cross-check was not read (fetch-notices)')
    union = S1 | S2 | S3 | S4 | S5 | S6 | S7 | S8
    union.discard(None)
    srcs = {'S1_tender_rows': S1, 'S2_lot_rows': S2, 'S3_541': S3, 'S4_571': S4, 'S5_545': S5,
            'S6_rederive': S6, 'S7_530': S7, 'S8_ingest': S8}
    retired_by_source = {k: sorted(v & retired) for k, v in srcs.items() if v & retired}
    union -= retired
    srcs = {k: v - retired for k, v in srcs.items()}       # per-source counts are of what L_changed holds
    per = {}
    for k, v in srcs.items():
        others = set().union(*[w for kk, w in srcs.items() if kk != k])
        per[k] = {'n': len(v), 'n_in_3502': len(v & ids3502), 'n_only_this_source': len(v - others)}
    residual = []
    if not set541:
        residual.append('541: 2631 stamped 753 / 2632 folded 820; only its 126 corrected Tenders have rows '
                        f'({len(rows541)} found in ({K.R541[0]}, {K.R541[1]}]); the 627 verified + 67 touched have '
                        'no row and no bounded enumeration (carriers = a full notice_texts walk). A verified Tender '
                        'compared identical (541 changes served result rows only) and none is in the 3,502 (U2).')
    residual.append(f'571: 2636 wrote 67; {len(S4)} enumerated (66 corrected); the 1 verified has no row and no bounded '
                    'enumeration (notice_ids walk); it compared identical (571 changes served winners only).')
    residual.append('ingest: a notice ingested before 2026-10-18T20:30Z but first parsed later is not visible per '
                    'notice; a fold of one that moved served facts wrote a change row (S1/S2).')
    out = {
        'what': 'issue 551 B6 U6 step 1: L_changed, the 2630-excluded reading (issue 551 U2 checker correction 2)',
        'cursor_range': [str(K.SINCE), str(cursor_now)],
        'change_rows': len(change_rows), 'rows_by_kind': dict(sorted(kinds.items())),
        'rows_by_kind_op': cls['by_kind_op'],
        'unmapped_lot_rows': {k: len(v) for k, v in unm.items()},
        'unmapped_lot_rows_head': {k: v[:20] for k, v in unm.items()},
        'no_tender_instant_rows': len(cls['no_tender_instant']),
        'retired': sorted(retired), 'n_retired': len(retired), 'retired_by_source': retired_by_source,
        'excluded_2630': {'cursor_range_inclusive': [K.J2630_LO, K.J2630_HI], 'stamp': K.iso_of(K.J2630_STAMP),
                          'tender_rows': len(excl_t), 'lot_rows': len(excl_l), 'organization_rows': len(excl_o),
                          'result_rows': dict(sorted(excl_x.items())),
                          'lot_rows_outside_3502_kept': excl_lot_outside,
                          'rows_in_range_not_excluded': len(in2630_kept),
                          'rows_in_range_not_excluded_cursors_head': in2630_kept[:20]},
        'segments': {'541_2632_rows_tenders': len(rows541), '571_2636_rows_tenders': len(rows571),
                     '530_rows_tenders': len(rows530)},
        'rederive': rederive_rec,
        'ingest_cross_check': dict(nt_stats, definition='tender_versions.tender_id where caused_by_notice_id >= N0, '
                                   'N0 = the first notice id whose ingested_at >= 1792355400 (2026-10-18T20:30:00Z); '
                                   'ingested_at is assigned at insert in id order'),
        'per_source': per,
        'l_changed': sorted(union), 'n': len(union), 'ids_sha256': K.ids_sha(union),
        'in_3502': sorted(union & ids3502), 'n_in_3502': len(union & ids3502),
        'outside_3502': len(union - ids3502),
        'residuals': residual, 'deviations': DEVIATIONS if not set541 else DEVIATIONS[1:],
        'notes': notes, 'anomalies': anomalies, 'stops': stops,
    }
    return out


def cmd_build(a):
    ids, _served = K.the_3502()
    rows, ranges = load_change_dirs(a.changes_dir or [os.path.join(LCH, 'changes')])
    nd = a.notices_dir or os.path.join(LCH, 'notices')
    nrows, nman = load_notice_dir(nd) if os.path.exists(os.path.join(nd, 'manifest.json')) else (None, None)
    s545 = K.jload(os.path.join(K.U6, 'l_changed_545_whole.json'))
    assert s545['n'] == 48 and K.ids_sha(s545['ids']) == s545['ids_sha256']
    set541 = None
    if a.set541:
        d = K.jload(a.set541)
        set541 = d['ids'] if isinstance(d, dict) else d
    out = build(rows, ranges, nrows, nman, a.cursor_now, ids, K.jload(K.L571), s545['ids'],
                parse_rederive(a.rederive), set541)
    out['inputs'] = {'changes_dirs': a.changes_dir or [os.path.join(LCH, 'changes')], 'notices_dir': nd,
                     'l_changed_571_ids.json': K.sha_file(K.L571),
                     'l_changed_545_whole.json': K.sha_file(os.path.join(K.U6, 'l_changed_545_whole.json'))}
    K.jdump(a.out, out)
    print(f'L_changed {out["n"]} (sha256 {out["ids_sha256"][:16]}), ∩ 3,502: {out["n_in_3502"]}, outside: '
          f'{out["outside_3502"]}')
    for k, v in out['per_source'].items():
        print(f'  {k}: {v}')
    print(f'  2630 excluded: {out["excluded_2630"]["tender_rows"]} tender + {out["excluded_2630"]["lot_rows"]} lot '
          f'+ {out["excluded_2630"]["organization_rows"]} org rows + result rows {out["excluded_2630"]["result_rows"]}')
    print(f'  rows by kind/op: {out["rows_by_kind_op"]}')
    print(f'  retired (out of L_changed): {out["n_retired"]}; unmapped lot rows {out["unmapped_lot_rows"]}')
    for r in out['rederive']:
        print(f'  rederive {r["day"]}: moved {r["moved"]} confirmed={r["confirmed"]} ({r.get("reading")})')
    for x in out['deviations']:
        print(f'  DEVIATION (record in 551 before the flip deploy): {x["id"]}')
    for x in out['notes']:
        print('  NOTE', x)
    for x in out['anomalies']:
        print('  ANOMALY', x)
    for x in out['stops']:
        print('  STOP', x)
    return 2 if out['stops'] else 0


def cmd_classify(a):
    """U7.2's post-deploy check: the change rows in a fetch-changes dir by kind and op. A deploy writes none: any
    tender / lot / result row is a STOP (exit 2; redo U6.2-U6.5 for that range); organization rows are listed."""
    rows, ranges = load_change_dirs([a.dir])
    cls = K.classify_changes(rows)
    by_kind = collections.Counter(r['entity_kind'] for r in rows)
    fold = {k: v for k, v in by_kind.items() if k in ('tender', 'lot') + K.RESULT_KINDS}
    print(f'{a.dir}: ranges {ranges}; {len(rows)} change rows; by kind/op {cls["by_kind_op"]}')
    if cls['unknown_kinds']:
        print(f'STOP unknown kinds {cls["unknown_kinds"]}')
        return 2
    if fold:
        print(f'STOP tender/lot/result rows in the range ({fold}; Tenders {sorted(cls["live"])[:20]}): redo U6.2-U6.5 '
              'for this range before any wet run')
        return 2
    if by_kind:
        print(f'NOTE only organization rows ({by_kind.get("organization", 0)}): no Tender fold input')
    print('PASS: no tender, lot or result row')
    return 0


def main(argv=None, fake=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    f = sub.add_parser('fetch-changes')
    f.add_argument('--cursor-now', type=int, required=True)
    f.add_argument('--from', dest='from_cursor', type=int, default=K.SINCE)
    f.add_argument('--dir', default=os.path.join(LCH, 'changes'))
    n = sub.add_parser('fetch-notices')
    n.add_argument('--dir', default=os.path.join(LCH, 'notices'))
    b = sub.add_parser('build')
    b.add_argument('--cursor-now', type=int, required=True)
    b.add_argument('--rederive', action='append')
    b.add_argument('--changes-dir', action='append')
    b.add_argument('--notices-dir')
    b.add_argument('--set541')
    b.add_argument('--out', default=os.path.join(K.U6, 'l_changed.json'))
    c = sub.add_parser('classify')
    c.add_argument('--dir', required=True)
    a = ap.parse_args(argv)
    if a.cmd == 'fetch-changes':
        return cmd_fetch_changes(a, fake)
    if a.cmd == 'fetch-notices':
        return cmd_fetch_notices(a, fake)
    if a.cmd == 'classify':
        return cmd_classify(a)
    return cmd_build(a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
