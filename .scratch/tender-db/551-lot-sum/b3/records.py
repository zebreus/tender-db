"""Issue 551 B3: the 4,745 unit-1 records (unit3/data/ctx.json.gz, the stored rows `../results-slim.jsonl` describes)
fed to BOTH sides as recorded: parity of the decision logic on identical inputs.

    python3 -I records.py OUT_DIR

Writes OUT_DIR/rec_in.tsv (the Rust harness input: every recorded version of the chain, as unit 3's port check
exported it, with the B1 facts absent: no published lot number, no group flag, no stated total, no bid, no winner, no
decision, none of which the stored rows of unit 1's pull carry) and OUT_DIR/rec_port.jsonl (rule_v2_b3's decision).

The port side: v1 is unit 3's merge_rule.serve on the same entry and rates (the whole recorded chain, so the scale
rules see every version, as the Rust does); v2's facts are rule_v2_b3.facts_from on an API-shaped detail built from the
SAME head version (amounts with their lot keys, a withheld amount without a value as the API serves it, lots with kind
and no title, every round's lot results with key, lot, award; no bids) and no notice content. The port's
`W/G0-served` check (an API reader's) is not applied: there is no served head here, and the Rust has no such check.
"""
import importlib.util, json, os, sys
from urllib.parse import quote

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('rule_v2_b3', os.path.join(HERE, 'rule_v2_b3.py'))
R = importlib.util.module_from_spec(spec)
spec.loader.exec_module(R)
MR, E = R.MR, R.E


def s(x):
    if x is None:
        return '-'
    x = quote(str(x), safe='')
    return '%2D' if x == '-' else x


def v1_of(entry, rates):
    out = MR.serve(entry, rates)
    versions, head, partners = MR.build(entry)

    def conv(f):
        return MR.to_eur(rates, f.currency, f.cents) if (f.currency == 'EUR' or f.currency in rates) else None
    today = MR.elect_today(head, partners, conv)
    by = MR.lot_sums_by_currency(head)
    if today is not None and today[2] == E.LOT(None):
        cur = today[1].currency
    elif today is None:
        cands = [(len(ls.awards), c) for c, ls in by.items() if ls.awards]
        cur = max(cands)[1] if cands else None
    else:
        cur = today[1].currency
    per = {lot: nc for lot, nc in by[cur].awards.items()} if (cur is not None and cur in by) else {}
    t_fact = None if today is None else {'cents': today[1].cents, 'currency': today[1].currency, 'field': today[1].field,
                                         'scope': list(today[2]), 'eur': today[0]}
    return {'why': out['why'], 'fires': out['why'] == 'W', 'derived': out['derived'], 'eur': out['eur'],
            'today_eur': out['today'], 'today': t_fact, 'served': None, 'served_derivation': None,
            'served_matches_election': True, 'currency': cur, 'per_lot': {k: list(v) for k, v in per.items()},
            'rates': rates, 'date': None, 'flags': []}, versions, head


def detail_of(tid, head):
    amounts = []
    for f in head.facts:
        amounts.append({'field': f.field, 'lot': None, 'quality': f.quality,
                        'value': None if f.quality is not None else {'cents': f.cents, 'currency': f.currency}})
    lots = []
    for l in head.lots:
        lots.append({'id': l.lot_id, 'lot_key': l.key, 'kind': l.kind, 'title': None})
        for f in l.facts:
            amounts.append({'field': f.field, 'lot': l.key, 'quality': f.quality,
                            'value': None if f.quality is not None else {'cents': f.cents, 'currency': f.currency}})
    results = []
    for rnd in head.rounds:
        for r in rnd.lot_results:
            aw = {'cents': r.cents, 'currency': r.currency} if (r.cents is not None and r.currency is not None) else None
            results.append({'key': r.key, 'lot': r.lot_key, 'notice_id': rnd.notice_id, 'awarded': aw, 'decision': None,
                            'winners': []})
    return {'id': tid, 'source': None, 'published_at': None, 'version': None, 'versions': [], 'amounts': amounts,
            'lot_details': lots, 'lot_results': results, 'bids': [], 'contracts': []}


def main():
    od = sys.argv[1]
    os.makedirs(od, exist_ok=True)
    ctx = MR.load_ctx()
    recs = {}
    for line in open(os.path.join(HERE, '..', 'results-slim.jsonl')):
        r = json.loads(line)
        recs[r['id']] = r
    tsv = open(os.path.join(od, 'rec_in.tsv'), 'w')
    pj = open(os.path.join(od, 'rec_port.jsonl'), 'w')
    for tid in sorted(ctx):
        e = ctx[tid]
        rates = MR.rates_of(e)
        v1, versions, head = v1_of(e, rates)
        x = detail_of(tid, head)
        contents = {nid: {'notice_id': nid, 'sections': []} for nid in R.version_notice_ids(x)}
        f = R.facts_from(x, contents, v1)
        ship, first, S, cur, n = R.v2_decision(f)
        pj.write(json.dumps({'id': tid, 'pop': recs[tid]['pop'], 'ship': ship, 'first': first, 'S': S, 'cur': cur, 'n': n,
                             'vouched_by': f.get('vouched_by'), 'v1_fires': R.v1_fires(f), 'v1_why': v1['why'],
                             'refusals': R.v2_refusals(f), 'today_eur': v1['today_eur'], 'v1_eur': v1['eur'],
                             'stored': recs[tid]['elected_eur_cents']}, default=str) + '\n')
        tsv.write(f'T\t{tid}\n')
        for c, r in sorted(rates.items()):
            tsv.write(f'R\t{s(c)}\t{r!r}\n')
        for i, v in enumerate(versions):
            tsv.write(f'V\t{i + 1}\n')
            for a in v.facts:
                tsv.write(f'F\t{s(a.field)}\t{a.cents}\t{s(a.currency)}\t{s(a.quality)}\n')
            for l in v.lots:
                tsv.write(f'L\t{s(l.key)}\t{s(l.kind)}\t-\t-\n')
                for a in l.facts:
                    tsv.write(f'LF\t{s(a.field)}\t{a.cents}\t{s(a.currency)}\t{s(a.quality)}\n')
            for rnd in v.rounds:
                tsv.write(f'N\t{rnd.notice_id}\n')
                for r in rnd.lot_results:
                    tsv.write('A\t' + '\t'.join([s(r.key if r.key is not None else 'RES'), s(r.lot_key), s(r.cents), s(r.currency),
                                                 '-', '-', '-', '0']) + '\n')
        tsv.write('E\n')
    print('records', len(ctx))


if __name__ == '__main__':
    main()
