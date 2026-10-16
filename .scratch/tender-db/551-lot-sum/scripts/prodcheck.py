"""Run elect.py on the stored rows of the 22 tenders 537 refused (+3 kept controls) and compare with the
served tenders.current_value_eur_cents."""
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import elect as E

def load(q):
    d = json.load(open(os.path.join(here, 'data', q + '.json')))
    return [dict(zip(d['columns'], r)) for r in d['rows']]

tenders = {r['id']: r for r in load('q1')}
amounts, lots, lotkeys, results = load('q2'), load('q3'), load('q4'), load('q5')
keys = {r['id']: r['lot_key'] for r in lotkeys}
by = lambda rows, t: [r for r in rows if r['tender_id'] == t]
order = [int(x) for x in open(os.path.join(here, 'sql', 'ids.txt')).read().strip().split(',')]
agree = 0
for t in order:
    head_seq = tenders[t]['current_seq']
    A, Lr, R = by(amounts, t), by(lots, t), by(results, t)
    seqs = sorted({r['seq'] for r in A + Lr + R} | {head_seq})
    def ver(s):
        return {"amounts": [r for r in A if r['seq'] == s], "lots": [r for r in Lr if r['seq'] == s],
                "results": [r for r in R if r['seq'] == s]}
    chain = [ver(s) for s in seqs if s < head_seq]
    h = ver(head_seq)
    eur, row, reasons = E.elect(h['amounts'], h['lots'], h['results'], chain=chain, lot_keys=keys)
    served = tenders[t]['current_value_eur_cents']
    ok = eur == served
    agree += ok
    refused = [r for r in reasons if r['status'] == 'refused']
    cur = row['currency'] if row else (refused[0]['currency'] if refused else None)
    el = f"{row['field']}@{'proc' if row['lot_id'] is None else keys.get(row['lot_id'])} {row['cents']} {row['currency']}" if row else "-"
    print(f"{'OK ' if ok else 'DIFF'} {t:>8} v{head_seq} served={served} port={eur} elected={el}")
    for r in refused:
        print(f"         refused {r['field']} {r['cents']} {r['currency']} [{r['scope']}]: {r['detail'][:110]}")
    if cur:
        ls = E.lot_sum(h['amounts'], h['lots'], h['results'], cur, keys)
        print("         lot_sum[" + cur + "]: " + "; ".join(f"{k}={v['sum']}/{v['lots']}" for k, v in sorted(ls.items())))
print(f"AGREE {agree}/{len(order)}")
