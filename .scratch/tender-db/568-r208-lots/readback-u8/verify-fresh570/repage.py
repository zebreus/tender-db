import json, os, subprocess, time, random
S = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/fresh570'
OUT = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/verify-fresh570/pages'
A = 957854566
def since_of(ent, i):
    if i == 0:
        return A
    return int(json.load(open(f'{S}/changes_{ent}/p{i-1:04d}.json'))['last_cursor'])
rnd = random.Random('verify-fresh570-pages')
picks = [('tender', 0), ('tender', 46), ('lot', 0), ('lot', rnd.randrange(1, 862)), ('lot', rnd.randrange(1, 862)), ('lot', 862)]
last = 0
for ent, i in picks:
    s = since_of(ent, i)
    out = f'{OUT}/{ent}_p{i:04d}.json'
    for attempt in (1, 2):
        w = last + 1.6 - time.time()
        if w > 0: time.sleep(w)
        last = time.time()
        r = subprocess.run(['curl', '-sS', '--max-time', '60', '-o', out, '-w', '%{http_code}',
                            f'https://tenders.zebreus.click/v1/changes?entity={ent}&since={s}&limit=1000'], capture_output=True, text=True)
        if r.stdout.strip() == '408' and attempt == 1: continue
        break
    code = r.stdout.strip()
    if code != '200':
        print(ent, i, 'HTTP', code); continue
    a = json.load(open(out)); b = json.load(open(f'{S}/changes_{ent}/p{i:04d}.json'))
    print(ent, i, 'since', s, 'events equal', a['events'] == b['events'], 'n', len(a['events']), 'last_cursor', a['last_cursor'], b['last_cursor'], 'more', a.get('more'), b.get('more'), 'gen', a.get('generation'))
