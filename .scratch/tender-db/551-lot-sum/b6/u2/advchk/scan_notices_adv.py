"""Scan every notice the 3,502 fresh tenders name for 541's drain predicate (TED-COST_PARAMETERS carriers, TED-FORM T01/T02)
and for any TED-AWARD_CONTRACT_MOVE field. Layers in the port's order (b6 notices first, then the older read-only layers).

python3 -I -B scan_notices_adv.py OUT.json
"""
import collections
import gzip
import json
import os
import sys

SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'
ROOT = '/home/user/tender-db/.scratch/tender-db/551-lot-sum'
LAYERS = [(SP + '/b6/notices', '{}.json.gz'), (SP + '/b3/raw/notices', '{}.json'), (SP + '/m551/cache/notices', '{}.json'),
          (SP + '/b3/bundle/cache/notices', '{}.json'), (SP + '/b3/verify/raw/notices', '{}.json'),
          (SP + '/b5/raw/notices', '{}.json')]


def load(nid):
    for base, pat in LAYERS:
        p = os.path.join(base, pat.format(nid))
        if os.path.exists(p):
            if p.endswith('.gz'):
                with gzip.open(p, 'rt', encoding='utf-8') as fh:
                    return base, json.load(fh)
            with open(p, encoding='utf-8') as fh:
                return base, json.load(fh)
    return None, None


def main(out):
    rows = json.load(open(os.path.join(ROOT, 'b5/investigate/served_lsa_rows.json')))
    ids = sorted(int(r[0]) for r in rows if int(r[0]) != 179732)
    tn = {}
    for t in ids:
        with gzip.open(f'{SP}/b6/tenders/{t}.json.gz', 'rt', encoding='utf-8') as fh:
            x = json.load(fh)
        s = set()
        for v in x.get('versions', []):
            if v.get('caused_by_notice_id'):
                s.add(int(v['caused_by_notice_id']))
        for k in ('lot_results', 'bids', 'contracts'):
            for r in x.get(k) or []:
                if r.get('notice_id'):
                    s.add(int(r['notice_id']))
        tn[t] = s
    allnid = sorted(set().union(*tn.values()))
    owner = collections.defaultdict(list)
    for t, s in tn.items():
        for n in s:
            owner[n].append(t)
    forms = collections.Counter()
    layer = collections.Counter()
    hits_cost, hits_move, t0x, missing = [], [], [], []
    for n in allnid:
        base, c = load(n)
        if c is None:
            missing.append(n)
            continue
        layer[base] += 1
        fids = set()
        form = None
        for sec in c.get('sections', []):
            for v in sec.get('values', []):
                fids.add(v['field_id'])
                if v['field_id'] == 'TED-FORM' and form is None:
                    form = str(v.get('value') if 'value' in v else v)
        forms[form] += 1
        if 'TED-COST_PARAMETERS' in fids:
            hits_cost.append({'notice': n, 'tenders': owner[n], 'form': form})
        if any('MOVE' in f for f in fids):
            hits_move.append({'notice': n, 'tenders': owner[n], 'form': form, 'fields': sorted(f for f in fids if 'MOVE' in f)})
        if form and form.strip().upper() in ('T01', 'T02'):
            t0x.append({'notice': n, 'tenders': owner[n], 'form': form})
    res = {'n_tenders': len(ids), 'n_notices': len(allnid), 'missing': missing, 'layers': dict(layer),
           'cost_parameters_carriers': hits_cost, 'move_fields': hits_move, 'T01_T02': t0x,
           'forms_top': forms.most_common(40)}
    json.dump(res, open(out, 'w'), indent=1, default=str)
    print(json.dumps({k: v for k, v in res.items() if k != 'forms_top'}, indent=1, default=str))
    print(forms.most_common(40))


if __name__ == '__main__':
    main(sys.argv[1])
