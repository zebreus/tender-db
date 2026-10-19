"""Issue 551 B6 U2: the measurement of decision points 4 and 7 (b6/decision-b6.md, amendments adopted), offline.

    python3 -I -B run_u2.py                     every stage; writes the outputs below (byte-for-byte reproducible)
    python3 -I -B run_u2.py --stage lchanged    stage 1 only (the provisional L_changed, fixed BEFORE any comparison)
    python3 -I -B run_u2.py --check             recompute everything and compare with the files on disk; writes nothing

Reads (never writes): the port of record b6/rule_v2_g.py (sha256 checked against b6/SHA256SUMS; never edited), its fresh
tender layer (scratchpad b6/tenders, gzip + sidecar fetch time) and notice layers, the rates m551/rates (10-17 set,
sha256 recorded, not refreshed), b5/investigate/served_lsa_rows.json, b6/l_changed_571_ids.json,
b6/l_changed_545_ids.json, and the public-API reads fetch_u2_api.py recorded under u2/api/ (the tender change feed from
cursor 958787061 to the head, and the ingested_at probes of the named notices). The network is disabled for the run.

Writes (u2/): l_changed_provisional.json, decisions.jsonl.gz, parity_v2.json, P.json, R_single.json, R_excess.json,
estimate_flags.json, u2-result.json, u2-result.md.
"""
import argparse
import collections
import datetime
import gzip
import hashlib
import importlib.util
import io
import json
import os
import socket
import sys
import urllib.request

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))                 # .../551-lot-sum/b6/u2
B6 = os.path.dirname(HERE)
ROOT = os.path.dirname(B6)
RULE = os.path.join(B6, 'rule_v2_g.py')
RULE_SHA = '23cbd8508a3afa35b195f8a2ee91a75bb3b8f08aa2e2f3f9abbd0ea80454452f'
SUMS = os.path.join(B6, 'SHA256SUMS')
SERVED_LSA = os.path.join(ROOT, 'b5', 'investigate', 'served_lsa_rows.json')
L571 = os.path.join(B6, 'l_changed_571_ids.json')
L545 = os.path.join(B6, 'l_changed_545_ids.json')
API = os.path.join(HERE, 'api')
API_MANIFEST = os.path.join(API, 'manifest.json')
RATES_U1 = {'eurofxref-hist.zip': '8e925cef', 'ecu-ert_h_eur_d.csv': '47e94127', 'ecu-ert_bil_eur_d.csv': '48e2c9f1'}

SINCE = 958787061                      # end of project 2627 (B5's corpus drain)
J2630 = (958787061, 958809778)         # (lo, hi]: switch-off drain-back 2630's rows (the feed shows them; asserted)
R541 = (958809778, 958809905)          # (lo, hi]: 541's 2632 corrections (issue 551 B6 ledger)
CUTOFF = '2026-10-18T20:30:00Z'
EXCLUDED = 179732                      # served_lsa_rows minus 179732 = the 3,502
EUR100M_CENTS = 10_000_000_000
NAMES = ('v2', 'v2+single', 'v2+cover', 'v2+both')
GUARDED = ('v2+single', 'v2+cover', 'v2+both')

OUT = {k: os.path.join(HERE, v) for k, v in (
    ('lchanged', 'l_changed_provisional.json'), ('decisions', 'decisions.jsonl.gz'), ('parity', 'parity_v2.json'),
    ('P', 'P.json'), ('R_single', 'R_single.json'), ('R_excess', 'R_excess.json'),
    ('estimate_flags', 'estimate_flags.json'), ('result_json', 'u2-result.json'), ('result_md', 'u2-result.md'))}

# Disagreement causes, filled in by hand after reading each one (input / port / served). Empty: none found.
CAUSES = {}


# ---------------------------------------------------------------- plumbing
def no_network():
    def refuse(*a, **k):
        raise RuntimeError('run_u2: network disabled (U2 decides from the recorded layers only)')
    socket.socket.connect = refuse
    socket.create_connection = refuse
    urllib.request.urlopen = refuse


def sha_file(p):
    with open(p, 'rb') as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def sha_bytes(b):
    return hashlib.sha256(b).hexdigest()


def ids_sha(ids):
    """sha256 of the ascending ids as ASCII lines 'id\\n' (the id-list digest used throughout)."""
    return sha_bytes(''.join(f'{i}\n' for i in sorted(ids)).encode())


def jbytes(obj):
    return (json.dumps(obj, indent=1, sort_keys=True, ensure_ascii=False, default=str) + '\n').encode('utf-8')


def gz_bytes(raw):
    buf = io.BytesIO()
    with gzip.GzipFile(filename='', mode='wb', fileobj=buf, mtime=0, compresslevel=6) as fh:
        fh.write(raw)
    return buf.getvalue()


def epoch(s):
    return datetime.datetime.fromisoformat(s.replace('Z', '+00:00')).timestamp()


def fetch_eff(s):
    """A fetch instant as an epoch, for 'a change row up to that id's own fetch time'. A second-precision sidecar
    (the U1 small fetch) is taken as the END of its second (the wider reading); a millisecond one as written."""
    t = epoch(s)
    return t + 0.999 if '.' not in s else t


def load_rule():
    with open(SUMS) as fh:
        sums = dict(reversed(line.split()) for line in fh if line.strip())
    if sums.get('rule_v2_g.py') != RULE_SHA or sha_file(RULE) != RULE_SHA:
        raise SystemExit(f'run_u2: the port is not the port of record ({sha_file(RULE)} / SHA256SUMS '
                         f'{sums.get("rule_v2_g.py")}; expected {RULE_SHA})')
    spec = importlib.util.spec_from_file_location('rule_v2_g_u2', RULE)
    R = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(R)
    rs = R.rates_sha256()
    for f, pre in RATES_U1.items():
        if not rs['files'].get(f, '').startswith(pre):
            raise SystemExit(f'run_u2: rates file {f} is not the 10-17 set U1 recorded ({rs["files"].get(f)})')
    return R


def the_ids():
    with open(SERVED_LSA, encoding='utf-8') as fh:
        rows = json.load(fh)
    served = {int(r[0]): r for r in rows}
    ids = sorted(i for i in served if i != EXCLUDED)
    assert len(ids) == 3502, len(ids)
    return ids, served


def meta_of(R, t):
    with open(os.path.join(R.B6_TENDERS, f'{t}.meta.json'), encoding='utf-8') as fh:
        return json.load(fh)


# ---------------------------------------------------------------- stage 1: the provisional L_changed
def load_feed():
    with open(API_MANIFEST, encoding='utf-8') as fh:
        m = json.load(fh)
    events, pages, since = [], [], SINCE
    for rec in sorted(m['changes'], key=lambda r: r['i']):
        assert rec['status'] == 200, rec
        assert rec['path'] == f'/v1/changes?since={since}&entity=tender&limit=1000', (rec['path'], since)
        p = os.path.join(API, rec['file'])
        assert sha_file(p) == rec['sha256'], p
        with open(p, encoding='utf-8') as fh:
            d = json.load(fh)
        assert not d.get('reset'), rec
        events += d['events']
        pages.append({'i': rec['i'], 'path': rec['path'], 'sent_at': rec['sent_at'], 'sha256': rec['sha256'],
                      'events': len(d['events']), 'last_cursor': d['last_cursor'], 'more': d['more'],
                      'generation': d.get('generation')})
        since = int(d['last_cursor'])
    assert pages and pages[-1]['more'] is False, 'the feed was not read to its head'
    assert len({p['generation'] for p in pages}) == 1, 'the feed generation moved while paging'
    cur = [int(e['cursor']) for e in events]
    assert cur == sorted(cur) and len(set(cur)) == len(cur) and cur[0] > SINCE
    assert all(e['entity'] == 'tender' for e in events)
    return m, events, pages


def stage_lchanged(R, ids):
    ids_set = set(ids)
    m, events, pages = load_feed()
    # the feed by job stamp (changed_at is the writing job's own instant, not each row's commit time)
    stamps = collections.OrderedDict()
    for e in events:
        stamps.setdefault(e['changed_at'], []).append(e)
    by_stamp = [{'changed_at': k, 'rows': len(v), 'distinct_ids': len({e['id'] for e in v}),
                 'distinct_ids_in_3502': len({e['id'] for e in v} & ids_set), 'first_cursor': v[0]['cursor'],
                 'last_cursor': v[-1]['cursor'],
                 'ops': {f'{op}/{"version-null" if vn else "versioned"}': n for (op, vn), n in sorted(
                     collections.Counter((e['op'], e['version'] is None) for e in v).items())}}
                for k, v in stamps.items()]
    # 2630's own rows: every row in (958787061, 958809778] -- asserted to be one version-null `changed` row per id of
    # exactly the 3,502, all stamped one instant (the switch-off drain-back "3,502 written / 3,502 corrected")
    j2630 = [e for e in events if J2630[0] < int(e['cursor']) <= J2630[1]]
    assert len(j2630) == 3502 and {e['id'] for e in j2630} == ids_set, len(j2630)
    assert all(e['op'] == 'changed' and e['version'] is None for e in j2630)
    assert len({e['changed_at'] for e in j2630}) == 1
    j2630_cursors = {e['cursor'] for e in j2630}
    ids541 = sorted({e['id'] for e in events if R541[0] < int(e['cursor']) <= R541[1]})
    with open(L571) as fh:
        s571 = sorted(set(json.load(fh)))
    with open(L545) as fh:
        s545 = sorted(set(json.load(fh)))
    rows_of = collections.defaultdict(list)
    for e in events:
        rows_of[e['id']].append(e)
    fetched, crit_a, crit_a_literal, any_non2630 = {}, {}, {}, {}
    for t in ids:
        fa = meta_of(R, t)['fetched_at']
        fetched[str(t)] = fa
        lim = fetch_eff(fa)
        mine = [e for e in rows_of.get(t, []) if int(e['cursor']) > SINCE]
        lit = [e['cursor'] for e in mine if epoch(e['changed_at']) <= lim]
        if lit:
            crit_a_literal[str(t)] = lit
        op = [c for c in lit if c not in j2630_cursors]
        if op:
            crit_a[str(t)] = op
        rest = [e['cursor'] for e in mine if e['cursor'] not in j2630_cursors]
        if rest:
            any_non2630[str(t)] = rest
    # the notice criterion: a notice the fetched Tender names whose ingested_at is at or after the cutoff. ingested_at
    # is assigned at insert in id order (store lib.rs import_lag), so it is decided by the boundary the probes found;
    # the probes are re-checked for monotonicity here.
    probes = sorted((int(k), v['ingested_at']) for k, v in m['notices'].items())
    assert all(v['status'] == 200 for v in m['notices'].values())
    for (_a, ta), (_b, tb) in zip(probes, probes[1:]):
        assert ta <= tb, 'ingested_at is not monotonic in notice id on the probes'
    for k, v in m['notices'].items():
        p = os.path.join(API, v['file'])
        assert sha_file(p) == v['sha256'], p
        with open(p, encoding='utf-8') as fh:
            assert json.load(fh)['ingested_at'] == v['ingested_at']
    b = m['boundary']
    named = {}
    for t in ids:
        named[t] = R.version_notice_ids(R.tender_detail(t))
    all_named = sorted({n for v in named.values() for n in v})
    assert b['n_named'] == len(all_named) and b['cutoff'] == CUTOFF
    first = b['first_id_at_or_after']
    if first is None:
        assert str(all_named[-1]) in m['notices'] and m['notices'][str(all_named[-1])]['ingested_at'] < CUTOFF
    crit_b = {str(t): [n for n in v if first is not None and n >= first] for t, v in named.items()}
    crit_b = {k: v for k, v in crit_b.items() if v}
    union = sorted({int(k) for k in crit_a} | {int(k) for k in crit_b} | set(s571) | set(s545) | set(ids541))
    lit_union = sorted({int(k) for k in crit_a_literal} | {int(k) for k in crit_b} | set(s571) | set(s545) | set(ids541))
    newest = all_named[-1]
    out = {
        'what': 'issue 551 B6 U2: the PROVISIONAL L_changed (decision point 4, amendment B5), fixed per id at its '
                'fetch time and written BEFORE any v2 comparison',
        'definition': {
            'operative': [
                f'(a) an id of the 3,502 with a `tender` change row in GET /v1/changes after cursor {SINCE} (the end of '
                f'project 2627) whose changed_at is at or before that id\'s own fetch time (its b6 sidecar fetched_at; '
                f'a second-precision sidecar is read as the end of its second), EXCLUDING the 3,502 rows of the '
                f'switch-off drain-back 2630 (cursor ({J2630[0]}, {J2630[1]}]);',
                f'(b) OR an id whose fetched Tender names (version caused_by, lot_results/bids/contracts notice_id; the '
                f'port\'s version_notice_ids) a notice folded since {CUTOFF}, read as: its ingested_at is at or after '
                f'{CUTOFF} (the wider reading, see notice_criterion);',
                f'(c) unioned with 571\'s set (b6/l_changed_571_ids.json), 545\'s set (b6/l_changed_545_ids.json) and '
                f'541\'s 2632 corrections (the tender rows in cursor ({R541[0]}, {R541[1]}]).'],
            'why_2630_is_excluded':
                'Read literally, criterion (a) puts every one of the 3,502 in L_changed: project 2630 (2026-10-19 '
                '03:34-03:36Z, refold-lot-sum by the 3,502 at W=false) wrote one version-null `changed` row for each of '
                'exactly the 3,502, after 958787061 and before every fetch, so the parity would compare nothing. Those '
                'rows are path W\'s own drain-back (each served head going from lot-sum-awards back to its max '
                'election; issue 551 "B5 -- switched off and drained back"), not a change of any fact the port reads: '
                'the decision excludes them itself where it builds the complement ("minus 2630\'s own correction rows") '
                'and lists the post-2627 fact changes as 541, the daily chains and 571 only. The one code change 2630 '
                'carried besides W=false is 541 unit 2 (923172d: T02 MOVE awards write a RES-T02), inert on the 3,502: '
                'none of their 15,537 notices is a T02 or carries TED-COST_PARAMETERS (one T01 prior-information notice, '
                '18002514 on 5278856, which 541 does not read). The literal reading is recorded in literal_reading.',
            'notice_criterion':
                f'The decision words this three ways: "a notice folded since 2026-10-18 20:30Z" (point 4, amendment B5), '
                f'"the tender ids of every notice ingested since 2026-10-18 20:30Z" (synthesis) and "notices folded since '
                f'2627, not ingested_at (notices fetched before the cutoff but parsed later fall through)" (C3). Wider '
                f'reading taken: first folded (parsed and folded for the first time) since the EARLIER cutoff {CUTOFF}. '
                f'Re-folds of already-folded notices are not counted (2627, 2630 and the zero-drift probe re-folded every '
                f'notice of the 3,502, which would make the criterion vacuous). On the public API the only per-notice '
                f'instant is ingested_at; it is assigned at insert in id order, so the probes binary-searched the '
                f'boundary over the 15,537 named notices: the newest named notice, {newest}, was ingested '
                f'{m["notices"][str(newest)]["ingested_at"]}, before the cutoff, so no named notice was ingested since '
                f'it. A notice ingested earlier but first parsed later (a quarantine reclaim or a re-parse keeps its '
                f'ingested_at) is not visible per notice without a detail GET of each; no reclaim or re-parse job is on '
                f'record in the window (the daily chain does not reclaim), and such a fold that moved facts, rounds, '
                f'groups, closure or value writes a tender change row (canonical.rs append_version_changes), which (a) '
                f'reads; only a silent (projection-only) version would escape. Recorded as a residual.'},
        'inputs': {'served_lsa_rows.json': sha_file(SERVED_LSA), 'l_changed_571_ids.json': sha_file(L571),
                   'l_changed_545_ids.json': sha_file(L545), 'api/manifest.json': sha_file(API_MANIFEST)},
        'feed': {'since': str(SINCE), 'head_last_cursor': pages[-1]['last_cursor'], 'pages': pages,
                 'events': len(events), 'by_job_stamp': by_stamp},
        'job_2630_rows': {'cursor_range': [str(J2630[0]), str(J2630[1])], 'rows': len(j2630),
                          'changed_at': j2630[0]['changed_at'], 'ids_equal_the_3502': True},
        'literal_reading': {'n_ids_of_3502': len(crit_a_literal),
                            'n_union': len(lit_union),
                            'note': 'criterion (a) without the 2630 exclusion: every id of the 3,502 (via its 2630 row)'},
        'criterion_a_ids': sorted(int(k) for k in crit_a),
        'criterion_a_rows': crit_a,
        'ids_of_3502_with_any_non2630_row_up_to_head': sorted(int(k) for k in any_non2630),
        'notice_boundary': {'cutoff': CUTOFF, 'n_named': len(all_named), 'probes': [[n, t] for n, t in probes],
                            'first_named_id_at_or_after_cutoff': first, 'newest_named_id': newest,
                            'newest_named_ingested_at': m['notices'][str(newest)]['ingested_at']},
        'criterion_b_ids': sorted(int(k) for k in crit_b),
        'set_541_2632': ids541, 'set_571': s571, 'set_545': s545,
        'l_changed_provisional': union, 'n': len(union), 'ids_sha256': ids_sha(union),
        'in_3502': sorted(set(union) & ids_set), 'n_in_3502': len(set(union) & ids_set),
        'compared_ids_sha256': ids_sha(set(ids) - set(union)), 'n_compared': len(set(ids) - set(union)),
        'fetch_time_per_id': fetched,
        'fetch_time_range': [min(fetched.values()), max(fetched.values())],
    }
    return out


# ---------------------------------------------------------------- stage 2: the port over the 3,502
def stage_decide(R, ids):
    recs, errors, layer_use = {}, {}, collections.Counter()
    tdig, ndig, seen_n = [], [], set()
    for t in ids:
        try:
            x = R.tender_detail(t)
            tp = R.find_cached('tenders', t)
            with open(tp, 'rb') as fh:
                tdig.append(f'{t}\t{sha_bytes(fh.read())}\n')
            for nid in R.version_notice_ids(x):
                if nid in seen_n:
                    continue
                seen_n.add(nid)
                p = R.find_cached('notices', nid)
                if p is None:
                    raise R.FetchError(f'/v1/notices/{nid}/content', 'in no layer (rb8 deleted?)')
                layer_use[os.path.dirname(p)] += 1
                with open(p, 'rb') as fh:
                    ndig.append((nid, f'{nid}\t{os.path.dirname(p)}\t{sha_bytes(fh.read())}\n'))
            f = R.facts(t)
            dec = R.decide_all(f)
        except Exception as e:                       # noqa: BLE001 - recorded as a miss
            errors[t] = f'{type(e).__name__}: {e}'
            continue
        recs[t] = {'id': t, 'fetched_at': meta_of(R, t)['fetched_at'], 'version': x.get('version'),
                   'published_at': x.get('published_at'), 'v1_why': f['v1']['why'], 'S_eur': f.get('S_eur'),
                   'flags': f['flags'], 'summed': [[s['key'], s['notice_id'], s['cents']] for s in f.get('summed') or []],
                   'vouch_candidates': f.get('vouch_candidates'), 'decisions': dec}
    ndig.sort()
    digests = {'tender_layer': sha_bytes(''.join(tdig).encode()), 'n_tenders': len(tdig),
               'notice_layers': sha_bytes(''.join(s for _n, s in ndig).encode()), 'n_notices': len(ndig),
               'notice_layer_use': dict(sorted(layer_use.items()))}
    return recs, errors, digests


def decisions_bytes(recs):
    raw = ''.join(json.dumps(recs[t], sort_keys=True, ensure_ascii=False, default=str) + '\n' for t in sorted(recs))
    return gz_bytes(raw.encode('utf-8'))


# ---------------------------------------------------------------- stage 3: v2 parity
def parity(row, v2):
    """The U1 self-test's T4 comparison (selftest_g.parity), unchanged: ship/refuse, S, currency, n, and the served
    vouched_by by MEMBERSHIP in the port's v2 vouch list (amendment A4)."""
    detail = json.loads(row[7]) if row[7] else {}
    vb = detail.get('vouched_by')
    served_ship = row[3] == 'lot-sum-awards'
    kinds = []
    if served_ship != v2['ship']:
        kinds.append('ship/refuse')
    if served_ship:
        if row[4] != v2['S']:
            kinds.append('S')
        if row[5] != v2['currency']:
            kinds.append('currency')
        if detail.get('lots') != v2['n']:
            kinds.append('n')
        key = None if not isinstance(vb, dict) else [vb.get('field'), vb.get('notice_id'), vb.get('cents')]
        member = key is not None and key in v2['vouches']
        if not member:
            kinds.append('vouched_by')
    else:
        member = None
    return {'ok': not kinds, 'kinds': kinds,
            'served': {'ship': served_ship, 'S': row[4], 'currency': row[5], 'n': detail.get('lots'), 'vouched_by': vb},
            'vouched_by_member': member, 'vouched_by_equal_first': vb == v2['vouched_by']}


def stage_parity(ids, served, recs, errors, lchanged):
    exempt = set(lchanged['l_changed_provisional'])
    compared = [t for t in ids if t not in exempt]
    dis, not_first, unread = {}, [], []
    kinds = collections.Counter()
    for t in compared:
        if t not in recs:
            unread.append(t)
            continue
        p = parity(served[t], recs[t]['decisions']['v2'])
        if not p['ok']:
            v2 = recs[t]['decisions']['v2']
            dis[str(t)] = dict(p, port_v2={k: v2[k] for k in ('ship', 'first', 'S', 'currency', 'n', 'vouched_by',
                                                              'vouches', 'refusals')},
                               v1_why=recs[t]['v1_why'], flags=recs[t]['flags'],
                               cause=CAUSES.get(t, 'UNEXPLAINED'))
            kinds.update(p['kinds'])
        elif not p['vouched_by_equal_first']:
            not_first.append(t)
    return {'what': 'issue 551 B6 U2: port v2 (guards=frozenset()) vs served_lsa_rows.json on every id of the 3,502 '
                    'outside the provisional L_changed (ship/refuse, S, currency, n; vouched_by by membership)',
            'n_ids': len(ids), 'n_exempt': len(set(ids) & exempt), 'n_compared': len(compared),
            'n_agree': len(compared) - len(dis) - len(unread), 'n_disagree': len(dis), 'disagreement_kinds': dict(kinds),
            'disagreements': dis, 'unread': unread, 'port_errors': {str(k): v for k, v in errors.items()},
            'vouched_by_member_not_first': sorted(not_first), 'n_vouched_by_member_not_first': len(not_first),
            'compared_ids_sha256': ids_sha(compared)}


# ---------------------------------------------------------------- stage 4: P and the guard sets
def year_of(published_at):
    return str(published_at or '')[:4] or None


def ship_entry(t, rec, name):
    d = rec['decisions'][name]
    return {'id': t, 'S': d['S'], 'currency': d['currency'], 'n': d['n'], 'S_eur_cents': rec['S_eur'],
            'vouched_by': d['vouched_by'], 'vouches': d['vouches'],
            'vouched_by_moved_from_v2': (d['vouched_by'] != rec['decisions']['v2']['vouched_by']) if name != 'v2' else None}


def superseded_shape(R, t, rec, name):
    """5816379's shape (recorded only, not a bar; issue 551 M2 fallback: "a vouch from an earlier award notice while a
    later award notice in the chain states its own different total"), as tested here: the first kept vouch is of kind
    award or range, from version notice N; a LATER version notice N' (higher seq) holds at least one served lot_results
    row and states its own award-kind total (the port's frozen table, r3_kind 'award') more than 1 % away from the
    vouching total. `lot_null`: every lot_results row of such an N' is stored lot = null (5816379's exact pattern,
    which the port's summed set cannot see)."""
    d = rec['decisions'][name]
    vb = d['vouched_by']
    if not vb:
        return None
    cand = next((c for c in rec['vouch_candidates'] if [c['field'], c['notice_id'], c['cents']] ==
                 [vb['field'], vb['notice_id'], vb['cents']]), None)
    if cand is None or cand['kind'] not in ('award', 'range'):
        return None
    x = R.tender_detail(t)
    seq = {}
    for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0):
        seq.setdefault(v.get('caused_by_notice_id'), v.get('seq') or 0)
    s0 = seq.get(vb['notice_id'])
    if s0 is None:
        return None
    rows = collections.defaultdict(list)
    for lr in x.get('lot_results', []):
        rows[lr.get('notice_id')].append(lr)
    hits = []
    for nid, s in sorted(seq.items(), key=lambda kv: kv[1]):
        if s <= s0 or nid == vb['notice_id'] or not rows.get(nid):
            continue
        own = [tt['cents'] for tt in R.notice_facts(R.notice_content(nid))['totals']
               if tt['r3_kind'] == 'award' and tt['cents'] is not None and tt['cents'] > 0]
        diff = [c for c in own if abs(c - vb['cents']) > 0.01 * vb['cents']]
        if diff:
            hits.append({'notice_id': nid, 'seq': s, 'award_totals': sorted(set(diff)),
                         'lot_null': all(lr.get('lot') in (None, '') for lr in rows[nid])})
    if not hits:
        return None
    return {'vouch_notice': vb['notice_id'], 'vouch_seq': s0, 'later': hits,
            'lot_null': any(h['lot_null'] for h in hits)}


def stage_sets(R, ids, recs):
    ships = {nm: sorted(t for t in ids if t in recs and recs[t]['decisions'][nm]['ship']) for nm in NAMES}
    for nm in GUARDED:
        assert set(ships[nm]) <= set(ships['v2']), nm
    v2s = set(ships['v2'])
    refused = {nm: sorted(v2s - set(ships[nm])) for nm in GUARDED}
    R_single = refused['v2+single']
    R_cover = refused['v2+cover']
    R_both = refused['v2+both']
    R_excess = sorted(set(R_cover) - set(R_single))
    R_excess_both = sorted(set(R_both) - set(R_single))
    interaction = sorted(set(R_both) - set(R_single) - set(R_cover))

    def refusal_entry(t, nm):
        rec = recs[t]
        d, v2 = rec['decisions'][nm], rec['decisions']['v2']
        cand = {(c['field'], c['notice_id'], c['cents']): c for c in rec['vouch_candidates'] or []}
        removed = [dict(cand[tuple(k)], key=k) for k in d['g_removed']]
        return {'id': t, 'first': d['first'], 'refusals': d['refusals'], 'S': v2['S'], 'currency': v2['currency'],
                'n': v2['n'], 'S_eur_cents': rec['S_eur'], 'year': year_of(rec['published_at']),
                'v2_vouched_by': v2['vouched_by'], 'v2_vouches': v2['vouches'],
                'removed': [{k: c[k] for k in ('field', 'notice_id', 'cents', 'kind', 'form', 'round_results',
                                               'content_lot_results', 'names_summed_keys', 'g_single', 'g_cover')}
                            for c in removed],
                'summed': rec['summed']}

    def shape_mix(entries):
        return {'removed_field': dict(sorted(collections.Counter(c['field'] for e in entries for c in e['removed']).items())),
                'removed_kind': dict(sorted(collections.Counter(c['kind'] for e in entries for c in e['removed']).items())),
                'v2_vouched_by_field': dict(sorted(collections.Counter((e['v2_vouched_by'] or {}).get('field')
                                                                      for e in entries).items())),
                'year': dict(sorted(collections.Counter(e['year'] for e in entries).items())),
                'eur_band': dict(sorted(collections.Counter(band(e['S_eur_cents']) for e in entries).items()))}

    P = {}
    extras = {}
    for nm in NAMES:
        ent = [ship_entry(t, recs[t], nm) for t in ships[nm]]
        bp = sorted(e['id'] for e in ent if e['S_eur_cents'] is not None and e['S_eur_cents'] >= EUR100M_CENTS)
        near = sorted(t for t in ships[nm] if 'within_2pct_of_eur100m' in recs[t]['flags'])
        sup = {t: superseded_shape(R, t, recs[t], nm) for t in ships[nm]}
        sup = {t: v for t, v in sup.items() if v}
        moved = sorted(e['id'] for e in ent if e['vouched_by_moved_from_v2'])
        P[nm] = {'n': len(ent), 'ids_sha256': ids_sha(ships[nm]), 'ships': ent}
        extras[nm] = {'n_ships': len(ent), 'g_refused_of_3502': len(ids) - len(ent),
                      'B_P_band_eur100m': bp, 'n_B_P': len(bp),
                      'within_2pct_of_eur100m': near,
                      'shape_5816379': {'n': len(sup), 'n_lot_null': sum(1 for v in sup.values() if v['lot_null']),
                                        'ids': sorted(sup), 'ids_lot_null': sorted(t for t, v in sup.items() if v['lot_null']),
                                        'detail': {str(t): v for t, v in sorted(sup.items())},
                                        'definition': superseded_shape.__doc__.strip()},
                      'vouched_by_moved_from_v2_still_ships': moved, 'n_vouched_by_moved': len(moved),
                      'eur_band': dict(sorted(collections.Counter(band(e['S_eur_cents']) for e in ent).items())),
                      'currency': dict(sorted(collections.Counter(e['currency'] for e in ent).items()))}
    rs_entries = [refusal_entry(t, 'v2+single') for t in R_single]
    rx_entries = [refusal_entry(t, 'v2+cover') for t in R_excess]
    sets = {
        'R_single': {'what': 'v2 ships G-single refuses (v2 ships minus v2+single ships)', 'n': len(R_single),
                     'ids_sha256': ids_sha(R_single), 'ids': R_single, 'shape_mix': shape_mix(rs_entries),
                     'entries': rs_entries},
        'R_excess': {'what': 'v2 ships G-cover refuses beyond G-single (amendment A3: G-cover \\ G-single = '
                             '(v2 ships refused under {cover}) minus R_single)', 'n': len(R_excess),
                     'ids_sha256': ids_sha(R_excess), 'ids': R_excess, 'shape_mix': shape_mix(rx_entries),
                     'entries': rx_entries,
                     'R_cover_all': R_cover, 'n_R_cover_all': len(R_cover),
                     'R_cover_and_single': sorted(set(R_cover) & set(R_single)),
                     'alt_R_both_minus_R_single': R_excess_both, 'n_alt': len(R_excess_both),
                     'alt_equals_primary': R_excess_both == R_excess,
                     'interaction_refused_only_with_both_guards': interaction},
    }
    return ships, refused, P, extras, sets


def band(c):
    if c is None:
        return 'no-eur'
    for lim, name in ((1_000_000, '<10k'), (10_000_000, '<100k'), (100_000_000, '<1m'), (1_000_000_000, '<10m'),
                      (10_000_000_000, '<100m'), (100_000_000_000, '<1bn')):
        if c < lim:
            return name
    return '>=1bn'


# ---------------------------------------------------------------- stage 5: the estimate-vouch flags (point 7, D4)
CN_EFORMS_SUBTYPES = {str(i) for i in range(1, 25)} | {'E1', 'E2', 'E3'}
CN_BT02_PREFIX = ('cn-', 'pin-', 'qu-', 'subco')
SINGLE_LOT_KINDS = ('Lot', 'Part', 'ProcurementProjectLot')        # lot sections; a LotsGroup is not a lot


def notice_cn(R, content):
    """-> (is_cn, reason). CN-type when the notice's form says so: a TED-FORM in the port's frozen CN set
    (cn_type), an eForms subtype (OPP-070-notice / DE1-NoticeSubType-SubTypeCode) 1..24 or E1..E3, or a BT-02-notice
    cn-*/pin-*/qu-*/subco*; with no form field at all, a notice holding no LotResult section."""
    vals = collections.defaultdict(list)
    has_result = False
    for s in content.get('sections', []):
        if s.get('kind') == 'LotResult':
            has_result = True
        for v in s.get('values', []):
            if v['field_id'] in ('TED-FORM', 'OPP-070-notice', 'DE1-NoticeSubType-SubTypeCode', 'BT-02-notice'):
                vals[v['field_id']].append(str(R._val(v)).strip())
    if vals.get('TED-FORM'):
        f = vals['TED-FORM'][0]
        return R.cn_type(f), f'TED-FORM {f}'
    for k in ('OPP-070-notice', 'DE1-NoticeSubType-SubTypeCode'):
        if vals.get(k):
            f = vals[k][0].upper()
            return f in CN_EFORMS_SUBTYPES, f'{k} {f}'
    if vals.get('BT-02-notice'):
        f = vals['BT-02-notice'][0]
        return f.lower().startswith(CN_BT02_PREFIX), f'BT-02-notice {f}'
    return (not has_result), 'no form field; ' + ('has' if has_result else 'no') + ' LotResult section'


def stage_estimate(R, ids, recs, ships):
    """Every estimate-kind R10 candidate of every v2 ship, tested: from a CN-type notice with exactly ONE lot section
    whose served key (the 570 remap target of that section) is not a summed key. Then, per guard set, the ships whose
    KEPT vouch list holds such a candidate (any), whose vouched_by is one (first), or whose every kept vouch is one
    (only: S has no other support)."""
    flagged = {}
    examined = 0
    for t in ships['v2']:
        rec = recs[t]
        ests = [c for c in rec['vouch_candidates'] or [] if c['kind'] == 'estimate']
        if not ests:
            continue
        x = R.tender_detail(t)
        contents = {nid: R.notice_content(nid) for nid in R.version_notice_ids(x)}
        inv, _how = R.remap_inverse(x, contents)
        keys = {k for k, _n, _c in rec['summed']}
        out = []
        for c in ests:
            examined += 1
            content = contents[c['notice_id']]
            is_cn, why = notice_cn(R, content)
            lots = [s['section_id'] for s in content.get('sections', []) if s.get('kind') in SINGLE_LOT_KINDS]
            served_key = None
            if len(lots) == 1:
                own = lots[0]
                back = [k for k, o in (inv.get(c['notice_id']) or {}).items() if o == own]
                served_key = back[0] if back else own
            flag = bool(is_cn and len(lots) == 1 and served_key not in keys)
            out.append({'field': c['field'], 'notice_id': c['notice_id'], 'cents': c['cents'], 'form': why,
                        'is_cn': bool(is_cn), 'n_lot_sections': len(lots), 'lot_section': lots[0] if len(lots) == 1 else None,
                        'served_key': served_key, 'key_in_S': (served_key in keys) if len(lots) == 1 else None,
                        'flag': flag})
        if any(o['flag'] for o in out):
            flagged[str(t)] = {'summed_keys': sorted(keys), 'candidates': out}
    per = {}
    for nm in NAMES:
        any_, first_, only_ = [], [], []
        for t in ships[nm]:
            fl = flagged.get(str(t))
            if not fl:
                continue
            fk = {(o['field'], o['notice_id'], o['cents']) for o in fl['candidates'] if o['flag']}
            d = recs[t]['decisions'][nm]
            kept = [tuple(k) for k in d['vouches']]
            hit = [k for k in kept if k in fk]
            if hit:
                any_.append(t)
                vb = d['vouched_by']
                if vb and (vb['field'], vb['notice_id'], vb['cents']) in fk:
                    first_.append(t)
                if len(hit) == len(kept):
                    only_.append(t)
        per[nm] = {'n_any': len(any_), 'n_first': len(first_), 'n_only': len(only_), 'any': any_, 'first': first_,
                   'only': only_}
    return {'what': 'issue 551 B6 U2: estimate vouches from a single-lot-section CN whose lot is not in S (decision '
                    'point 7 / amendment D4; the estimate twin of 6465510). Recorded only; flagged to B-fresh readers.',
            'definition': stage_estimate.__doc__.strip() + ' ' + notice_cn.__doc__.strip(),
            'n_estimate_candidates_examined': examined, 'per_guard_set': per, 'flagged': flagged}


# ---------------------------------------------------------------- driver
def build(stage=None):
    no_network()
    R = load_rule()
    ids, served = the_ids()
    files = {}
    lch = stage_lchanged(R, ids)
    files['lchanged'] = jbytes(lch)
    if stage == 'lchanged':
        return files
    recs, errors, digests = stage_decide(R, ids)
    files['decisions'] = decisions_bytes(recs)
    par = stage_parity(ids, served, recs, errors, lch)
    files['parity'] = jbytes(par)
    ships, refused, P, extras, sets = stage_sets(R, ids, recs)
    files['P'] = jbytes({'what': 'issue 551 B6 U2: P, the port\'s v2+G ships among the 3,502, under each candidate '
                                 'guard set (exact ids with S, currency, n, vouched_by = the first G-kept vouch, and '
                                 'the kept vouch list); v2 for reference. The final P is the adopted set (U4).',
                         'guard_sets': P})
    files['R_single'] = jbytes(sets['R_single'])
    files['R_excess'] = jbytes(sets['R_excess'])
    est = stage_estimate(R, ids, recs, ships)
    files['estimate_flags'] = jbytes(est)
    out_sha = {os.path.basename(OUT[k]): sha_bytes(v) for k, v in files.items()}
    rates = R.rates_sha256()
    v2_refuse = sorted(set(ids) - set(ships['v2']))
    res = {
        'what': 'issue 551 B6 U2 result: decision points 4 and 7 (b6/decision-b6.md, amendments adopted)',
        'port': {'path': 'b6/rule_v2_g.py', 'sha256': RULE_SHA, 'SHA256SUMS_match': True},
        'rates': rates,
        'run_u2.py_sha256': sha_file(os.path.abspath(__file__)),
        'fetch_u2_api.py_sha256': sha_file(os.path.join(HERE, 'fetch_u2_api.py')),
        'api_requests_log_lines': sum(1 for _ in open(os.path.join(HERE, 'api_requests.log'))),
        'inputs': dict(lch['inputs'], fresh_layer=digests),
        'misses': {'port_errors': len(errors), 'notices_only_in_rb8': 0 if not errors else 'see port_errors'},
        'l_changed_provisional': {'n': lch['n'], 'n_in_3502': lch['n_in_3502'], 'ids_sha256': lch['ids_sha256'],
                                  'criterion_a': len(lch['criterion_a_ids']), 'criterion_b': len(lch['criterion_b_ids']),
                                  'set_541_2632': len(lch['set_541_2632']), 'set_571': len(lch['set_571']),
                                  'set_545': len(lch['set_545']),
                                  'literal_reading_n_in_3502': lch['literal_reading']['n_ids_of_3502'],
                                  'feed_head_last_cursor': lch['feed']['head_last_cursor'],
                                  'fetch_time_range': lch['fetch_time_range']},
        'parity_v2': {k: par[k] for k in ('n_compared', 'n_agree', 'n_disagree', 'disagreement_kinds', 'unread',
                                          'n_vouched_by_member_not_first', 'compared_ids_sha256')},
        'parity_disagreements': {k: {'kinds': v['kinds'], 'cause': v['cause']} for k, v in par['disagreements'].items()},
        'counts': {
            'v2_ships': len(ships['v2']), 'v2_refuses': len(v2_refuse), 'v2_refused_ids': v2_refuse,
            'P_v2+single': len(ships['v2+single']), 'P_v2+cover': len(ships['v2+cover']),
            'P_v2+both': len(ships['v2+both']),
            'R_single': sets['R_single']['n'], 'R_excess': sets['R_excess']['n'],
            'R_cover_all': sets['R_excess']['n_R_cover_all'],
            'R_excess_alt_both_minus_single': sets['R_excess']['n_alt'],
            'R_excess_alt_equals_primary': sets['R_excess']['alt_equals_primary'],
            'interaction_refused_only_with_both': sets['R_excess']['interaction_refused_only_with_both_guards'],
            'estimate_flags': {nm: {k: est['per_guard_set'][nm][k] for k in ('n_any', 'n_first', 'n_only')}
                               for nm in NAMES},
        },
        'ids': {'R_single': sets['R_single']['ids'], 'R_excess': sets['R_excess']['ids'],
                'P_ids_sha256': {nm: P[nm]['ids_sha256'] for nm in NAMES}},
        'shape_mix': {'R_single': sets['R_single']['shape_mix'], 'R_excess': sets['R_excess']['shape_mix']},
        'extras_per_guard_set': extras,
        'outputs_sha256': out_sha,
    }
    files['result_json'] = jbytes(res)
    files['result_md'] = render_md(res, lch, par, sets, est, extras, sha_bytes(files['result_json'])).encode('utf-8')
    return files


def fmt_ids(ids, limit=None):
    ids = list(ids)
    s = ', '.join(str(i) for i in (ids if limit is None else ids[:limit]))
    return s + (f', ... ({len(ids)} in all)' if limit is not None and len(ids) > limit else '')


def render_md(res, lch, par, sets, est, extras, json_sha):
    c = res['counts']
    L = []
    w = L.append
    w('# Issue 551 B6 U2 -- result (decision points 4 and 7)\n')
    w('Produced by `run_u2.py` (offline; network disabled; reproduces every output byte for byte; `--check` verifies). '
      'Public-API reads by `fetch_u2_api.py` (15 GETs, `api_requests.log`).\n')
    w('## Inputs\n')
    w(f'- Port of record `b6/rule_v2_g.py` **sha256 {RULE_SHA}** (matches `b6/SHA256SUMS`; not edited).')
    for f, h in res['rates']['files'].items():
        w(f'- Rates `m551/rates/{f}` sha256 {h} (the 10-17 set U1 recorded; not refreshed -- refresh is before U6).')
    fl = res['inputs']['fresh_layer']
    w(f'- Fresh U2 layer: {fl["n_tenders"]} tenders (digest {fl["tender_layer"]}), {fl["n_notices"]} notices (digest '
      f'{fl["notice_layers"]}); notice layers used: ' +
      '; '.join(f'`{os.path.basename(os.path.dirname(k))}/{os.path.basename(k)}` {v}' for k, v in fl['notice_layer_use'].items()) +
      f'. Port errors / misses: {res["misses"]["port_errors"]} (no notice needed the deleted rb8 layers).')
    w(f'- Tender fetch times {lch["fetch_time_range"][0]} .. {lch["fetch_time_range"][1]}.')
    for k, v in res['inputs'].items():
        if k != 'fresh_layer':
            w(f'- `{k}` sha256 {v}')
    w('')
    w('## 1. The provisional L_changed (fixed per id at fetch time, before comparing)\n')
    w(f'- Feed: `GET /v1/changes?since={SINCE}&entity=tender&limit=1000`, {len(lch["feed"]["pages"])} pages to the head '
      f'(last_cursor {lch["feed"]["head_last_cursor"]}, more=false), {lch["feed"]["events"]} tender rows. By job stamp:')
    for s in lch['feed']['by_job_stamp']:
        w(f'  - {s["changed_at"]}: {s["rows"]} rows, {s["distinct_ids"]} ids ({s["distinct_ids_in_3502"]} of the 3,502), '
          f'cursor {s["first_cursor"]}..{s["last_cursor"]}, {s["ops"]}')
    w(f'- **Literal reading** (a change row after {SINCE} up to the fetch time): **{lch["literal_reading"]["n_ids_of_3502"]} '
      f'of the 3,502** -- every id, through its row from the switch-off drain-back 2630 (cursor ({J2630[0]}, '
      f'{J2630[1]}], {lch["job_2630_rows"]["rows"]} rows stamped {lch["job_2630_rows"]["changed_at"]}, exactly the '
      f'3,502). The parity would compare nothing.')
    w('- **Operative reading** (2630 excluded): ' + lch['definition']['why_2630_is_excluded'])
    w(f'- Criterion (a), 2630 excluded: **{len(lch["criterion_a_ids"])}** ids. No id of the 3,502 has any non-2630 tender '
      f'row even up to the head ({len(lch["ids_of_3502_with_any_non2630_row_up_to_head"])}).')
    nb = lch['notice_boundary']
    w(f'- Criterion (b), notices: **{len(lch["criterion_b_ids"])}** ids. ' + lch['definition']['notice_criterion'] +
      f' Probes (id, ingested_at): {nb["probes"]}.')
    w(f'- Sets: 541 2632 corrections {len(lch["set_541_2632"])} (feed range ({R541[0]}, {R541[1]}]), 571 '
      f'{len(lch["set_571"])} (= the feed\'s 05:53:39Z rows), 545 {len(lch["set_545"])} (= the feed\'s 11:06:21Z rows).')
    w(f'- **Provisional L_changed: {lch["n"]} ids** (sha256 {lch["ids_sha256"]}), **{lch["n_in_3502"]} of them among '
      f'the 3,502**, so all {lch["n_compared"]} are compared.\n')
    w('## 2. v2 parity (port guards=frozenset() vs served_lsa_rows.json)\n')
    w(f'- Compared **{par["n_compared"]}** (outside L_changed), **agree {par["n_agree"]}, disagree {par["n_disagree"]}**, '
      f'unread {len(par["unread"])}, port errors {len(par["port_errors"])}. Kinds: {par["disagreement_kinds"] or "none"}.')
    for k, v in par['disagreements'].items():
        w(f'  - {k}: {v["kinds"]}; served {v["served"]}; port {v["port_v2"]}; cause: {v["cause"]}')
    w(f'- vouched_by a member of the port\'s list but not its first: {par["n_vouched_by_member_not_first"]} (A4\'s known '
      f'naming difference; not a disagreement).\n')
    w('## 3. P and the guard sets\n')
    w(f'- v2 ships {c["v2_ships"]} of 3,502 (refuses {c["v2_refuses"]}{": " + fmt_ids(c["v2_refused_ids"]) if c["v2_refused_ids"] else ""}).')
    for nm in GUARDED:
        e = extras[nm]
        w(f'- **{nm}: P = {e["n_ships"]}** (ids sha256 {res["ids"]["P_ids_sha256"][nm]}), g = {e["g_refused_of_3502"]}; '
          f'vouched_by moved but still ships {e["n_vouched_by_moved"]}; B_P (>= EUR 100 m) {e["n_B_P"]}: '
          f'{fmt_ids(e["B_P_band_eur100m"])}; within 2 % of EUR 100 m: {fmt_ids(e["within_2pct_of_eur100m"]) or "none"}; '
          f'5816379 shape {e["shape_5816379"]["n"]} (lot-less later award {e["shape_5816379"]["n_lot_null"]}: '
          f'{fmt_ids(e["shape_5816379"]["ids_lot_null"], 40) or "none"}).')
    rs, rx = sets['R_single'], sets['R_excess']
    w(f'- **R_single = {rs["n"]}** (sha256 {rs["ids_sha256"]}): {fmt_ids(rs["ids"], 60)}. Shape: {rs["shape_mix"]}.')
    w(f'- **R_excess = {rx["n"]}** (G-cover refuses beyond G-single; sha256 {rx["ids_sha256"]}): {fmt_ids(rx["ids"], 60)}. '
      f'Shape: {rx["shape_mix"]}. All G-cover refusals {rx["n_R_cover_all"]}, of which in R_single '
      f'{len(rx["R_cover_and_single"])}; R_both minus R_single = {rx["n_alt"]} (equal to R_excess: '
      f'{rx["alt_equals_primary"]}); refused only with both guards: {rx["interaction_refused_only_with_both_guards"] or "none"}.')
    w(f'- Adoption reads (U4, not done here): min(|R|, 30) of each set in `sha256("551-b6-gcost-"+id)` order, M2 protocol; '
      f'adopt iff >= 10 % non-right.\n')
    w('## 4. Estimate-vouch flags (point 7 / D4; recorded only, flagged to B-fresh readers)\n')
    w(f'- Definition: {est["definition"]}')
    w(f'- Estimate candidates examined (v2 ships): {est["n_estimate_candidates_examined"]}; ids with a flagged candidate: '
      f'{len(est["flagged"])}.')
    for nm in NAMES:
        p = est['per_guard_set'][nm]
        w(f'- {nm}: any {p["n_any"]} ({fmt_ids(p["any"], 40) or "none"}); first (vouched_by) {p["n_first"]}; only '
          f'(no other kept vouch) {p["n_only"]} ({fmt_ids(p["only"]) or "none"}).')
    w('')
    w('## Outputs (sha256)\n')
    for k, v in res['outputs_sha256'].items():
        w(f'- `u2/{k}` {v}')
    w(f'- `u2/u2-result.json` {json_sha}')
    w(f'- `u2/run_u2.py` {res["run_u2.py_sha256"]}; `u2/fetch_u2_api.py` {res["fetch_u2_api.py_sha256"]}')
    return '\n'.join(L) + '\n'


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--stage', choices=['lchanged'])
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args(argv)
    files = build(a.stage)
    if a.check:
        bad = []
        for k, b in files.items():
            p = OUT[k]
            if not os.path.exists(p) or sha_file(p) != sha_bytes(b):
                bad.append(os.path.basename(p))
        print('CHECK', 'MATCH' if not bad else 'DIFFER: ' + ', '.join(bad))
        return 0 if not bad else 1
    for k, b in files.items():
        tmp = OUT[k] + '.tmp'
        with open(tmp, 'wb') as fh:
            fh.write(b)
        os.replace(tmp, OUT[k])
        print(os.path.basename(OUT[k]), sha_bytes(b))
    return 0


if __name__ == '__main__':
    sys.exit(main())
