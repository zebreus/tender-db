"""Human-readable dump of one tender's API detail (what v1 reads) for the fidelity check."""
import json, sys
import os
RAW = os.environ.get('M551', '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551') + '/fid/raw'
def money(v):
    return None if not v else f"{v['currency']} {v['cents']/100:,.2f}"
tid = int(sys.argv[1])
x = json.load(open(f'{RAW}/tenders/{tid}.json'))
print('TENDER', tid, x['source'], 'v', x['version'], 'published', x['published_at'], 'head value', money(x.get('value')), 'deriv', x.get('value_derivation'))
print('title:', x.get('title'))
print('versions:', [(v['seq'], v['caused_by_notice_id'], v.get('notice_subtype'), v['published_at']) for v in x['versions']])
print('lot_details:')
for l in x['lot_details']:
    print('  ', l['lot_key'], l['kind'], 'id', l['id'], '|', (l.get('title') or '')[:80], '| value', money(l.get('value')))
print('amounts:')
for a in x['amounts']:
    print('  ', a['field'], a.get('lot'), money(a.get('value')), 'q=', a.get('quality'))
print('lot_results:')
for lr in x['lot_results']:
    print('  ', lr.get('lot'), lr['key'], 'n', lr['notice_id'], money(lr.get('awarded')), lr.get('decision'),
          [(w.get('organization_id'), (w.get('organization_name') or '')[:40]) for w in lr.get('winners', [])])
print('bids:')
for b in x['bids']:
    print('  ', b.get('lot'), b['key'], 'n', b['notice_id'], money(b.get('value')),
          [(p.get('organization_id'), (p.get('organization_name') or p.get('mention_name') or '')[:40], p['role']) for p in b.get('parties', [])])
print('contracts:', [(c['key'], c['notice_id'], money(c.get('value'))) for c in x.get('contracts', [])])
