"""Issue 551 B6 U6 step 2: refetch L_changed ∩ 3,502 and re-run the port of record (v2+both) -> the final P.

    python3 -I -B predict.py backup [--lchanged u6/l_changed.json]
        copies the fresh-layer body + sidecar of every id in L_changed ∩ 3,502 into u6/pre_refresh/ (fetch_b6 --refresh
        keeps the old sidecar but overwrites the body) and writes u6/refresh_ids.json for fetch_b6.py --ids.
    (runbook) python3 -I -B ../fetch_b6.py --ids u6/refresh_ids.json --refresh
    python3 -I -B predict.py run --refreshed-after ISO [--lchanged ...] [--rates-json u6/rates.json] [--ids-limit K]
        the port (sha256 23cbd850... asserted, never edited) over ALL 3,502 under the refreshed rates
        (RULE_G_RATES_DIR from rates.json), guards {single, cover} (P = v2+both, U4):
          - an id in L_changed ∩ 3,502 must have been re-read (its sidecar fetched_at >= --refreshed-after); its new
            decision REPLACES its P entry (changes listed);
          - every other id must decide exactly as U2 did (u2/decisions.jsonl.gz: ship, first, S, currency, n,
            vouched_by, vouches); S_eur may move only for a head dated after the OLD set's newest fixing (2026-10-16:
            from 2026-10-17, read from the old zip; the port's rate() carries the last fixing on or before the head
            date, so an earlier head reads a fixing both sets hold); any other difference is a STOP (the facts, the
            port or the ECB's history moved without a change row);
        writes u6/P_final.json (ids_sha256, per id S / currency / n / S_eur_cents / vouched_by / vouches, Rg = the
        3,502 minus P, the replaced entries, the U2 comparison) and u6/decisions_u6.jsonl.gz.
Exit 0 clean; 2 STOP; 3 refused (missing input).
"""
import argparse
import gzip
import importlib.util
import io
import json
import os
import shutil
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kitlib as K  # noqa: E402

BOTH = frozenset({'single', 'cover'})
U2_DECISIONS = os.path.join(K.B6, 'u2', 'decisions.jsonl.gz')
PRE = os.path.join(K.U6, 'pre_refresh')
FIELDS = ('ship', 'first', 'S', 'currency', 'n', 'vouched_by', 'vouches')
RATES_MOVE_FROM_DEFAULT = '2026-10-17'  # the old (10-17) set's newest fixing 2026-10-16 + 1 day


def rates_move_from(rates):
    """The first head date whose S_eur the refreshed rates may move: the OLD set's newest fixing + 1 day. The port's
    rate() takes the last fixing on or before the head date (7-day carry), so a head dated on or before the old set's
    newest fixing reads a fixing BOTH sets hold; only a later head can see a new rate. A move on an earlier head means the
    ECB revised its history: a STOP. Read from the old zip itself (rates.json's old dir); the default only when no
    rates.json exists (an offline test with --old-rates-ok)."""
    if not rates:
        return RATES_MOVE_FROM_DEFAULT
    import datetime
    import rates_refresh as RR
    newest, _n = RR.check_zip(os.path.join(rates['old']['dir'], 'eurofxref-hist.zip'))
    return (datetime.date.fromisoformat(newest) + datetime.timedelta(days=1)).isoformat()


def load_port(rates_dir):
    with open(os.path.join(K.B6, 'SHA256SUMS')) as fh:
        sums = dict(reversed(line.split()) for line in fh if line.strip())
    if sums.get('rule_v2_g.py') != K.RULE_SHA or K.sha_file(K.RULE) != K.RULE_SHA:
        raise SystemExit('predict: rule_v2_g.py is not the port of record')
    spec = importlib.util.spec_from_file_location('rule_v2_g_u6', K.RULE)
    R = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(R)
    if rates_dir:
        R.set_rates_dir(rates_dir)
    return R


def u2_decisions():
    out = {}
    with gzip.open(U2_DECISIONS, 'rt', encoding='utf-8') as fh:
        for line in fh:
            r = json.loads(line)
            out[int(r['id'])] = r
    assert len(out) == 3502, len(out)
    return out


def cmd_backup(a):
    lc = K.jload(a.lchanged)
    if lc.get('stops'):
        print(f'REFUSED: l_changed.json carries STOPs: {lc["stops"]}')
        return 3
    ids = sorted(lc['in_3502'])
    os.makedirs(PRE, exist_ok=True)
    R = load_port(None)
    rec = []
    for t in ids:
        for name in (f'{t}.json.gz', f'{t}.meta.json'):
            src = os.path.join(R.B6_TENDERS, name)
            dst = os.path.join(PRE, name)
            if not os.path.exists(src):
                print(f'REFUSED: {src} missing (the U2 layer should hold every id of the 3,502)')
                return 3
            if not os.path.exists(dst):
                shutil.copyfile(src, dst)
            rec.append({'file': name, 'sha256': K.sha_file(dst)})
    K.jdump(os.path.join(PRE, 'manifest.json'), {'at': K.now_iso(), 'n_ids': len(ids), 'ids_sha256': K.ids_sha(ids),
                                                 'files': rec})
    K.jdump(os.path.join(K.U6, 'refresh_ids.json'), {'ids': ids, 'n': len(ids), 'ids_sha256': K.ids_sha(ids)})
    print(f'{len(ids)} ids backed up to {PRE}; fetch list u6/refresh_ids.json')
    return 0


def decide_one(R, t):
    f = R.facts(t)
    d = R.decide_facts(f, BOTH)
    return f, d


def same_u2(d, u2rec):
    u = u2rec['decisions']['v2+both']
    diff = [k for k in FIELDS if (d[k] if k != 'vouches' else [list(x) for x in d[k]]) !=
            (u[k] if k != 'vouches' else [list(x) for x in u[k]])]
    return diff


def cmd_run(a):
    lc = K.jload(a.lchanged)
    if lc.get('stops'):
        print(f'REFUSED: l_changed.json carries STOPs: {lc["stops"]}')
        return 3
    rates = K.jload(a.rates_json) if a.rates_json else None
    if rates is None and not a.old_rates_ok:
        print('REFUSED: no rates.json (A5: refresh the rates first); --old-rates-ok only for an offline test')
        return 3
    R = load_port(rates['RULE_G_RATES_DIR'] if rates else None)
    if rates and R.rates_sha256()['files'] != rates['new']['sha256']:
        print('STOP: the rates dir does not hold the files rates.json recorded')
        return 2
    ids, served = K.the_3502()
    if a.ids_limit:
        ids = ids[:a.ids_limit]
    refetch = set(lc['in_3502'])
    u2 = u2_decisions()
    P_u2 = K.p_u2()
    after = K.epoch_of(a.refreshed_after) if a.refreshed_after else None
    move_from = rates_move_from(rates)
    stops, notes = [], []
    recs, replaced, same, rates_moves, errors = {}, [], 0, [], {}
    for t in ids:
        if t in refetch:
            meta = R.tender_read_meta(t) or {}
            fa = meta.get('fetched_at')
            if after is None or fa is None or K.epoch_of(fa.rstrip('Z').split('.')[0] + 'Z') < after:
                stops.append(f'{t} is in L_changed ∩ 3,502 but its tender read ({fa}) is not after --refreshed-after')
                continue
        try:
            f, d = decide_one(R, t)
        except Exception as e:                                    # noqa: BLE001 - a miss is a STOP, recorded
            errors[t] = f'{type(e).__name__}: {e}'
            continue
        recs[t] = {'id': t, 'fetched_at': (R.tender_read_meta(t) or {}).get('fetched_at'),
                   'published_at': f.get('published_at') or (R.tender_detail(t).get('published_at')),
                   'S_eur': f.get('S_eur'), 'decision': {k: d[k] for k in FIELDS + ('refusals', 'g_removed')},
                   'v1_why': f['v1']['why']}
        u = u2[t]
        diff = same_u2(d, u)
        if t in refetch:
            if diff or u['S_eur'] != f.get('S_eur'):
                replaced.append({'id': t, 'fields': diff, 'u2': {k: u['decisions']['v2+both'][k] for k in FIELDS},
                                 'u6': {k: d[k] for k in FIELDS}, 'u2_S_eur': u['S_eur'], 'u6_S_eur': f.get('S_eur')})
            continue
        if diff:
            stops.append(f'{t} (not in L_changed) decides differently from U2 on {diff}')
        elif u['S_eur'] != f.get('S_eur'):
            pub = R.head_date(recs[t]['published_at']) or ''
            rates_moves.append({'id': t, 'published': pub, 'u2': u['S_eur'], 'u6': f.get('S_eur')})
            if pub < move_from:
                stops.append(f'{t}: S_eur moved {u["S_eur"]} -> {f.get("S_eur")} on a head dated {pub} (before '
                             f'{move_from}, the first date the refreshed rates may move): the ECB revised its history')
        else:
            same += 1
    if errors:
        stops.append(f'{len(errors)} port errors (a notice missing from every layer? run fetch_b6 --notices-only)')
    ships = {t: r for t, r in recs.items() if r['decision']['ship']}
    P = []
    for t in sorted(ships):
        r = ships[t]
        d = r['decision']
        P.append({'id': t, 'S': d['S'], 'currency': d['currency'], 'n': d['n'], 'S_eur_cents': r['S_eur'],
                  'vouched_by': d['vouched_by'], 'vouches': d['vouches']})
    Rg = sorted(set(recs) - set(ships))
    rg_u2 = K.rg_u2()
    out = {
        'what': 'issue 551 B6 U6: the final P (port v2+both on the U2 layer, L_changed ∩ 3,502 refetched), frozen '
                'before the flip deploy',
        'port_sha256': K.RULE_SHA, 'rates': R.rates_sha256(), 'refreshed_after': a.refreshed_after,
        'rates_move_from': move_from,
        'l_changed_sha256': lc['ids_sha256'], 'n_l_changed_in_3502': len(refetch),
        'n_decided': len(recs), 'n': len(P), 'ids_sha256': K.ids_sha(ships), 'ships': P,
        'Rg': Rg, 'n_Rg': len(Rg), 'Rg_equals_u2': Rg == rg_u2 if not a.ids_limit else None,
        'Rg_vs_u2': {'added': sorted(set(Rg) - set(rg_u2)), 'removed': sorted(set(rg_u2) - set(Rg))},
        'P_vs_u2': {'added': sorted(set(ships) - set(P_u2)), 'removed': sorted(set(P_u2) & set(recs) - set(ships)),
                    'u2_n': len(P_u2), 'u2_ids_sha256': K.P_U2_IDS_SHA},
        'replaced': replaced, 'same_as_u2': same, 'rates_only_S_eur_moves': rates_moves,
        'errors': {str(k): v for k, v in errors.items()}, 'stops': stops, 'notes': notes,
        'partial': bool(a.ids_limit),
    }
    raw = ''.join(json.dumps(recs[t], sort_keys=True, default=str) + '\n' for t in sorted(recs)).encode()
    buf = io.BytesIO()
    with gzip.GzipFile(filename='', mode='wb', fileobj=buf, mtime=0) as fh:
        fh.write(raw)
    with open(a.decisions_out, 'wb') as fh:
        fh.write(buf.getvalue())
    out['decisions_sha256'] = K.sha_file(a.decisions_out)
    K.jdump(a.out, out)
    print(f'P_final {len(P)} (ids_sha256 {out["ids_sha256"][:16]}); Rg {len(Rg)} (equals U2: {out["Rg_equals_u2"]}); '
          f'replaced {len(replaced)}; same as U2 {same}; S_eur-only moves {len(rates_moves)}; errors {len(errors)}')
    for s in stops[:30]:
        print('  STOP', s)
    return 2 if stops else 0


def main(argv=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    b = sub.add_parser('backup')
    b.add_argument('--lchanged', default=os.path.join(K.U6, 'l_changed.json'))
    r = sub.add_parser('run')
    r.add_argument('--lchanged', default=os.path.join(K.U6, 'l_changed.json'))
    r.add_argument('--rates-json', default=os.path.join(K.U6, 'rates.json'))
    r.add_argument('--old-rates-ok', action='store_true')
    r.add_argument('--refreshed-after')
    r.add_argument('--ids-limit', type=int)
    r.add_argument('--out', default=os.path.join(K.U6, 'P_final.json'))
    r.add_argument('--decisions-out', default=os.path.join(K.U6, 'decisions_u6.jsonl.gz'))
    a = ap.parse_args(argv)
    if a.cmd == 'backup':
        return cmd_backup(a)
    if a.rates_json and not os.path.exists(a.rates_json):
        a.rates_json = None
    return cmd_run(a)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
