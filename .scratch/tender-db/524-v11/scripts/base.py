import sys; sys.path.insert(0,'/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge')
from load import *
P=plan()
byorg={}
for e in P['planned']: byorg[e['org_id']]=('planned',e)
for e in P['held']:
    assert e['org_id'] not in byorg
    byorg[e['org_id']]=('held',e)
L=labels()
def status(x):
    s=byorg.get(x['org_id'])
    if s is None: return ('absent',None)
    return s
