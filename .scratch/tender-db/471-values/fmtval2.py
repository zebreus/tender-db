import sys, tarfile, re, io, collections
pat = re.compile(rb'<([A-Z_]+)\b[^>]*?FMTVAL="([0-9.]+)"[^>]*>([^<]*)<')
def num(t):
    t = t.decode('utf-8','replace').strip().replace(' ',' ').replace(' ','')
    if not t: return None
    # european decimal: last separator , with <=2 digits after
    m = re.fullmatch(r'([0-9.,]+)', t)
    if not m: return None
    if re.search(r',\d{1,2}$', t): t = t.replace('.','').replace(',','.')
    elif re.search(r'\.\d{1,2}$', t) and t.count('.')==1: t = t.replace(',','')
    else: t = t.replace(',','').replace('.','')
    try: return float(t)
    except: return None
for path in sys.argv[1:]:
    byday = collections.Counter(); c = collections.Counter(); ratios = collections.Counter(); ex = []
    with tarfile.open(path) as outer:
        for om in outer:
            if not om.isfile() or not om.name.endswith('.tar.gz'): continue
            with tarfile.open(fileobj=outer.extractfile(om), mode='r:gz') as inner:
                for im in inner:
                    if not im.isfile() or not im.name.endswith('.xml'): continue
                    data = inner.extractfile(im).read()
                    for el, f, txt in pat.findall(data):
                        v = num(txt); fv = float(f)
                        if v is None: c['text_unparsed'] += 1; continue
                        c['elements'] += 1
                        if abs(fv - v) <= 0.011 * max(1, abs(v)) * 0 + 0.011 or (v and abs(fv/v - 1) < 1e-9): c['agree'] += 1
                        else:
                            c['disagree'] += 1
                            r = fv / v if v else float('inf')
                            k = round(__import__('math').log10(r)) if r > 0 and r != float('inf') else None
                            exact = k is not None and abs(r - 10**k) < 1e-6 * 10**k
                            ratios[f'10^{k}' if exact else 'other'] += 1
                            if len(ex) < 6: ex.append((im.name, el.decode(), f.decode(), txt.decode('utf-8','replace').strip()))
                            if exact and k >= 2: byday[om.name] += 1
    print(path, dict(c), dict(ratios.most_common(8)))
    print('  big-scale by daily package:', sorted(byday.items()))
