import json, gzip, re, sys, collections, unicodedata
d = json.load(gzip.open(sys.argv[1]))['body']
if isinstance(d, str): d = json.loads(d)
LEGAL = set('''gmbh ag kg ohg ug mbh co ev e v eg gbr srl s r l spa s p a sa s a sas sarl sàrl ltd limited plc llp lp inc
corp bv b v nv n v oy ab as a s aps sro s r o spol zoo z o o sp ooo oao zao kft bt zrt nyrt doo d o o dd d d ad
ae ee epe ike ope oe sl s l slu slp scrl s c r l sc s c scarl coop soc cooperativa società societa srls'''.split())
def toks(s):
    s = unicodedata.normalize('NFKC', s or '').casefold()
    s = re.sub(r"[^\w]+", ' ', s)
    return [t for t in s.split() if t]
def core(s):
    return [t for t in toks(s) if t not in LEGAL]
CW = re.compile(r'[\n\r\t]')
ADDR = re.compile(r'(\b\d{4,5}\b|\bD-\d{5}\b|str\.|straße|strasse|\bul\.|\bvia\b|\brue\b|\bstreet\b|\broad\b|\bplatz\b|\bweg\b|\bpostfach\b|\btel\b|@|\bcalle\b|\bavenida\b|\bodos\b|οδός|\bnr\.?\s*\d)', re.I)
out = []
for lst in ('planned', 'held'):
    for r in d[lst]:
        old = r['old_name'] or ''
        if not CW.search(old): continue
        new = r['new_name'] or ''
        lines = [l.strip() for l in CW.split(old) if l.strip()]
        collapsed = ' '.join(old.split())
        first = lines[0] if lines else ''
        rest = ' '.join(lines[1:])
        co, cn = core(collapsed), core(new)
        cls = []
        if co == cn: cls.append('equal_core')
        elif set(cn) < set(co) and len(cn) < len(co): cls.append('loss_subset')
        elif cn and co[:len(cn)] == cn: cls.append('loss_prefix')
        elif set(co) < set(cn): cls.append('gain_superset')
        else: cls.append('other')
        if core(first) == cn: cls.append('new_eq_first_line')
        legal_lines = sum(1 for l in lines if set(toks(l)) & LEGAL)
        if len(lines) >= 2 and legal_lines >= 2: cls.append('multi_entity')
        if rest and ADDR.search(rest): cls.append('addr_tail')
        out.append(dict(list=lst, org=r['org_id'], reason=r['reason'], rel=r.get('recent_released'), lines=len(lines),
                        cls=cls, old=old, new=new, spellings=r.get('spellings', [])[:5]))
json.dump(out, open('cw_rows.json', 'w'), ensure_ascii=False, indent=1)
c = collections.Counter()
for o in out:
    c[(o['list'], o['cls'][0])] += 1
    for k in o['cls'][1:]: c[(o['list'], k)] += 1
for k, v in sorted(c.items()): print(k, v)
print('total', len(out))
