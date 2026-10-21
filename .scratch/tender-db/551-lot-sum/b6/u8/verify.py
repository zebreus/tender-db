"""Issue 551 B6 U8: the verify read list and the whole-drain checks (decision U8 + D1-D7), after the last run.

    python3 -I -B verify.py read  [--band-route sql|api]
                                  (box) PK reads of every id of L ∪ the pins ∪ 7511890 ∪ every fire the runs saw
                                  (u8/post_all; Tenders the runs saw retired are expected-absent) and the band seek
                                  (u8/band_final.json; the public API if the /v1/sql statement does not answer)
    python3 -I -B verify.py check [--explained FILE]
                                  (offline) against u6/freeze.json, P_final, l_changed, the PRE snapshot and
                                  u7/state/registry.json:
  V1 (D1 whole)  every P id serves lot-sum-awards at P's S / currency / n with a vouched_by among P's vouches; a P id
                 not firing: STOP-explain; every Rg id identical to the PRE snapshot (else SWITCH OFF).
  V2 (D5 whole)  no id read here serves lot-sum-awards outside P ∪ L_changed (else SWITCH OFF).
  V3 (D3)        the pins.
  V4 (D4)        the band == B_P ∪ the delta band rows. EVERY delta band row (an L_changed fire >= EUR 100 m) shows
                 STOP until it is read at 0 wrong of any class -- so V4 reads STOP whenever a delta band row exists, until
                 --explained records its read; a missing B_P head: SWITCH OFF (near 100 m: STOP-explain).
  V5 (runs)      every run in u7/state/registry.json ended PASS; a run that ended STOP and was explained within the gap
                 (--explained run_explanations) is a note; a run that ended SWITCH OFF stays SWITCH OFF.
  U9 baseline    (C8, not a rule) the fire rate observed in the L_changed runs: Σ new fires / Σ dry-admitted over the
                 lcNN runs -- U9's daily count baseline, recorded in 551 before the first U9 check.
--explained FILE: {"band_reads": {"<id>": {"verdict": "right"|"neutral"|"wrong", "ref": "<where the read is recorded>"}},
                   "run_explanations": {"<run>": "<the explanation, as recorded in 551>"}} -- reads and explanations
                   already recorded in 551; right/neutral turns a band row's STOP into a note, wrong is SWITCH OFF.
Writes u8/verify_result.json and u8/delta_population.json (the new fires in L_changed not in the 3,502: delta.py's
input). Exit codes as dchecks.py (0 PASS, 10 SWITCH OFF, 2 STOP, 3 NOT A MEASUREMENT or a tool fault).
"""
import argparse
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
B6 = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(B6, 'u6'))
sys.path.insert(0, os.path.join(B6, 'u7'))
import kitlib as K  # noqa: E402
import dchecks as D  # noqa: E402


def read_ids(ctx):
    L = K.jload(os.path.join(ctx.u6, 'L_ids.json'))['ids']
    fired = [int(i) for i in ctx.reg['fired']]
    return sorted(set(L) | set(K.pin_ids()) | {K.NOT_REACHABLE} | set(fired))


def cmd_read(a, fake=None, ctx=None, out=HERE, http=None):
    ctx = ctx or D.Ctx()
    box = K.shared_box(fake)
    ids = read_ids(ctx)
    retired = set(ctx.reg.get('retired', []))
    _r, f, m = K.pk_read(box, ids, 'u8_post_all', os.path.join(out, 'post_all'), expected_absent=retired)
    if f or m:
        print(f'u8: the PK read is incomplete (failures {f}, missing {(m or [])[:10]}): box_health, then re-run')
        return D.NOT_MEASURED
    body, route = K.read_band(box, 'u8_band', os.path.join(out, 'band_final.json'), route=a.band_route, http=http)
    if body is None:
        return D.NOT_MEASURED
    print(f'u8: {len(ids)} ids read (expected-absent {len(retired & set(ids))}); band rows {len(body["rows"])} '
          f'(route {route})')
    return 0


def u9_baseline(ctx):
    """C8: Σ new fires / Σ dry-admitted over the lcNN runs (results/<run>.json new_fires, jobs/<run>_dry_result.json)."""
    per, tot_new, tot_adm = [], 0, 0
    for run in [r for r in ctx.stages['order'] if r.startswith('lc')]:
        rp, dp = ctx.d('results', f'{run}.json'), ctx.d('jobs', f'{run}_dry_result.json')
        if not (os.path.exists(rp) and os.path.exists(dp)):
            per.append({'run': run, 'missing': True})
            continue
        n_new, adm = len(K.jload(rp)['new_fires']), K.jload(dp)['found']
        per.append({'run': run, 'new_fires': n_new, 'dry_admitted': adm})
        tot_new += n_new
        tot_adm += adm
    return {'what': 'C8: the fire rate observed in the L_changed runs (daily-fold population), U9\'s count baseline',
            'new_fires': tot_new, 'dry_admitted': tot_adm, 'rate': (tot_new / tot_adm) if tot_adm else None,
            'corpus_rate_for_comparison': K.CORPUS_RATE[0] / K.CORPUS_RATE[1], 'per_run': per}


def cmd_check(a, ctx=None, out=HERE):
    ctx = ctx or D.Ctx()
    explained = K.jload(a.explained) if getattr(a, 'explained', None) else {}
    band_reads = explained.get('band_reads') or {}
    run_expl = explained.get('run_explanations') or {}
    try:
        post = K.load_rows(os.path.join(out, 'post_all'))
        ctx.absent = K.absent_of(os.path.join(out, 'post_all'))
        band = K.jload(os.path.join(out, 'band_final.json'))
    except (SystemExit, OSError) as e:
        print(f'u8: NOT A MEASUREMENT: {e}')
        return D.NOT_MEASURED
    outs = []
    o = D.d1(ctx, 'u8', sorted(ctx.ids3502), post)
    o.rule = 'V1 (D1 over the 3,502)'
    outs.append(o)
    o2 = D.Out('V2 (D5 over every id read)')
    allowed = set(ctx.P) | ctx.lc
    for i, r in sorted(post.items()):
        if D.is_lsa(r) and i not in allowed:
            o2.add(D.SWITCH_OFF, f'{i} serves lot-sum-awards outside P ∪ L_changed')
    outs.append(o2)
    o3 = D.d3(ctx, post)
    o3.rule = 'V3 (D3)'
    outs.append(o3)
    delta = sorted(i for i, r in post.items() if D.is_lsa(r) and i in ctx.lc and i not in ctx.ids3502)
    delta_band = [i for i in delta if (post[i].get('current_value_eur_cents') or 0) >= K.EUR100M]
    o4 = D.d4(ctx, band, sorted(ctx.ids3502 | ctx.lc), new_fires=delta, reads=band_reads)
    o4.rule = 'V4 (D4)'
    if delta_band:
        o4.note(f'{len(delta_band)} delta band row(s) {delta_band}: each shows STOP until read at 0 wrong of any class '
                '(record the read in 551, then pass it with --explained)')
    outs.append(o4)
    o5 = D.Out('V5 (runs)')
    for r in ctx.reg['runs']:
        if r['verdict'] == 'PASS':
            continue
        if r['verdict'] == 'SWITCH OFF':
            o5.add(D.SWITCH_OFF, f'run {r["run"]} ended SWITCH OFF')
        elif r['verdict'] == 'STOP' and run_expl.get(r['run']):
            o5.note(f'run {r["run"]} ended STOP, explained: {run_expl[r["run"]]}')
        else:
            o5.add(D.STOP, f'run {r["run"]} ended {r["verdict"]} (unexplained: record the explanation in 551 and pass it '
                           'with --explained)')
    order = ctx.stages['order']
    done = [r['run'] for r in ctx.reg['runs']]
    if [x for x in order if x not in done]:
        o5.add(D.STOP, f'runs not done: {[x for x in order if x not in done]}')
    outs.append(o5)
    base = u9_baseline(ctx)
    worst = D.PASS
    for x in outs:
        worst = D.worse(worst, x.code)
    pop = {'what': 'issue 551 B6 U8: new lot-sum-awards fires in L_changed outside the 3,502 (the delta read population)',
           'at': K.now_iso(), 'ids': delta, 'n': len(delta), 'ids_sha256': K.ids_sha(delta), 'band_rows': delta_band,
           'rows': {str(i): post[i] for i in delta}}
    K.jdump(os.path.join(out, 'delta_population.json'), pop)
    res = {'at': K.now_iso(), 'verdict': D.NAMES[worst], 'code': worst, 'rules': [x.rec() for x in outs],
           'n_read': len(post), 'n_delta': len(delta), 'delta_band_rows': delta_band, 'u9_count_baseline': base,
           'explained_file': getattr(a, 'explained', None),
           'explained_sha256': K.sha_file(a.explained) if getattr(a, 'explained', None) else None}
    K.jdump(os.path.join(out, 'verify_result.json'), res)
    for x in outs:
        print(f'{x.rule}: {D.NAMES[x.code]}')
        for ln in x.lines:
            print(f'   {ln}')
    rate = f'{base["rate"]:.4%}' if base['rate'] is not None else 'n/a'
    print(f'U9 count baseline (C8): {base["new_fires"]} new fires / {base["dry_admitted"]} dry-admitted over the lc runs '
          f'= {rate} (corpus {base["corpus_rate_for_comparison"]:.4%}) -- record in 551 before the first U9 check')
    print(f'u8 verify: {D.NAMES[worst]}; delta population {len(delta)} (band rows {delta_band})')
    return worst


def main(argv=None, fake=None, ctx=None, out=HERE, http=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    r = sub.add_parser('read')
    r.add_argument('--band-route', choices=['sql', 'api'], default='sql')
    c = sub.add_parser('check')
    c.add_argument('--explained')
    a = ap.parse_args(argv)
    if a.cmd == 'read':
        return cmd_read(a, fake, ctx, out, http)
    return cmd_check(a, ctx, out)


if __name__ == '__main__':
    sys.exit(K.run_main(main))
