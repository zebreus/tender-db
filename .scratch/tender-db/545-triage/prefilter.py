# Candidate pre-filter for the review packet: keep a stray only if its distinctive tokens share
# nothing with the org's other voting names. Measures review load and recall against the hand labels.
import json, sys, re
sys.path.insert(0, '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/r524judge')
from keys import vote_key
GENERIC = set('nhs trust foundation university council borough county city district authority services service limited ltd group uk the and of for institute company healthcare health care community national royal college school academy partnership centre center association development solutions systems management consulting consultants construction international europe holdings north south east west northern southern london wales cymru scotland england'.split())
def toks(s):
    return {t for t in re.findall(r'[a-z0-9]+', vote_key(s).lower()) if len(t) >= 3 and t not in GENERIC}
def keep(stray, head, others):
    st = toks(stray)
    pool = toks(head)
    for _, o in others: pool |= toks(o)
    return not (st & pool)
def run(path, labels):
    rows = json.load(open(path))
    kept = [r for r in rows if keep(r['stray'], r['head'], r['others'])]
    print(path, 'strays', len(rows), 'kept for review', len(kept))
    if labels:
        F = {r['stray'] for r in rows if labels.get(r['stray']) == 'F'}
        kf = {r['stray'] for r in kept if labels.get(r['stray']) == 'F'}
        print('  F recall', len(kf), 'of', len(F), 'missed', sorted(F - kf))
    return kept
lab = {r['stray']: r['class'] for r in json.load(open('q3_strays_judged.json'))}
k3 = run('q3_strays.json', lab)
cohF = {'Bedford Borough Council':'F','DDM Health Ltd':'F','Diabetes UK':'F','Autocross Euroshell Ltd':'F','LAND USE CONSULTANTS LIMITED':'F','London Cactus':'F'}
k8 = run('q8_strays.json', cohF)
for r in k3 + k8:
    print('   kept:', r['org'], repr(r['head'][:40]), '<-', repr(r['stray'][:60]), lab.get(r['stray'], cohF.get(r['stray'], '?')))
