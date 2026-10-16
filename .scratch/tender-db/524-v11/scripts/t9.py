exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t8.py').read().split("R1=[e for e")[0])
import collections, re
PH=re.compile(r'(?i)^(keineangabe|unbekannt|nichtvorhanden)\d*$')
def ph(i): return bool(i) and (bool(PH.match(i)) or set(i)=={'0'})
BIND={'family','gb_family','consortium_removed','nhs_check'}
SERVE={'review_body','organ','code_added','address_added','caps_variant'}
def v11(st,e):
    """returns (kind, reason) under v11 given the v10 verdict"""
    r=e['rule']; reason=e.get('reason')
    v10kind='change' if st=='planned' else 'hold'
    # holds decided before serve stay
    if st=='held' and reason in ('recent','residue','undated','no_fit','initialism'): return ('hold',reason)
    if st=='held' and reason in BIND: return ('hold',reason)
    if r in ('R0n','R0e'):
        return ('hold','placeholder_id') if ph(e.get('identifier')) else (v10kind, reason if st=='held' else r)
    # reached serve: bind guard placeholder first
    if ph(e.get('identifier')): return ('hold','placeholder_id')
    if st=='held' and reason in SERVE: return ('hold',reason)
    if doubled(e['old_name'],e['new_name']): return ('hold','doubled')
    if seat_dropped(e['old_name'],e['new_name']): return ('hold','seat_dropped')
    if st=='held' and reason=='head_current': return ('hold','head_current')
    if st=='planned' and r=='R1' and not e['recent_released'] and stale(e,365) and rival(e): return ('hold','top_stale')
    return (v10kind, ('rel:'+e['recent_released']) if e['recent_released'] else r)
c=collections.Counter(); m=collections.Counter(); moves=collections.Counter()
for st,e in [('planned',e) for e in P['planned']]+[('held',e) for e in P['held']]:
    k=v11(st,e); c[k]+=1; m[k]+=e['n']
    old=('change',(('rel:'+e['recent_released']) if e['recent_released'] else e['rule'])) if st=='planned' else ('hold',e['reason'])
    if old!=k: moves[(old,k)]+=1
for k in sorted(c): print(k,c[k],m[k])
print('changes',sum(c[k] for k in c if k[0]=='change'),sum(m[k] for k in m if k[0]=='change'))
print('holds',sum(c[k] for k in c if k[0]=='hold'),sum(m[k] for k in m if k[0]=='hold'))
for k,v in sorted(moves.items(),key=lambda kv:-kv[1]): print('move',k,v)
