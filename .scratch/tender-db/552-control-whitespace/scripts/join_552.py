import json, collections, sys
sys.argv = ['x']
src = open(__import__('os').path.join(__import__('os').path.dirname(__file__), 'guard_552.py')).read().split('c = collections.Counter()')[0]
exec(src)
d = json.load(open('rejudge2543_result.json'))
v = {}
for g in d['groups']:
    for it in g['items']: v.setdefault(it['org_id'], (it['verdict'], g['group'], it['why']))
c = collections.Counter()
for i, o in enumerate(rows):
    if o['rel'] != 'unfit': continue
    g = guard(o['old'], o['new']) or 'serve'
    vv = v.get(o['org'], ('unjudged',))[0]
    c[(g if g == 'serve' else 'withheld', vv)] += 1
    if g != 'serve' or vv not in ('right',): print(i, o['org'], g, vv, '|', v.get(o['org'], ('', '', ''))[2][:170])
print(sorted(c.items()))
print(d['tally'])
