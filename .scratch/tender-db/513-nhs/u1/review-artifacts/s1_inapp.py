import json, re, sys, collections
rows = json.load(open(sys.argv[1]))
STOP = set("spolka ograniczona odpowiedzialnoscia akcyjna jawna komandytowa cywilna oddzial lider konsorcjum uczestnik partner societa cooperativa sociale onlus consorzio stabile capogruppo mandataria mandatario mandante rti rtp ati sigla srl spa snc sas sapa scarl scrl soc coop ltd limited gmbh mbh inc the and und von der des sp zoo".split())
def cores(n):
    s = (n or '').lower().replace("'", '')
    return {t for t in re.findall(r'[a-z0-9]+', s) if len(t) > 2 and t not in STOP}
by = collections.defaultdict(list)
for r in rows:
    if r['side'] == 'registry': by[r['code']].append(r)
moved = flagged = 0; fc = collections.Counter(); mc = collections.Counter()
for code, rs in by.items():
    top = cores(max(rs, key=lambda r: r['coded'])['name'])
    for r in rs:
        moved += r['coded']; mc[r['final']] += r['coded']
        a = cores(r['name'])
        if a and top and a.isdisjoint(top):
            flagged += r['coded']; fc[r['final']] += r['coded']
print('in-app core tokens: moved', moved, 'flagged', flagged, f'{100*flagged/moved:.2f}%', dict(fc), 'stray coded', mc['DIFFERENT'] + mc['NOT_ORG'])
