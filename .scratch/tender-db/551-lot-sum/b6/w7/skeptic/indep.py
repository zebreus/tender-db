import re, sys, collections
P = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/project_head.rs'
src = open(P).read().split('\n')
T = next(i for i,l in enumerate(src) if l.startswith('mod tests {'))
prod = src[:T]
lits = collections.defaultdict(list)
def add_lits(lines, name):
    for i, l in enumerate(lines, 1):
        s = l.strip()
        if s.startswith('//'): continue
        for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', l):
            lits[m.group(1)].append(f"{name}:{i}")
add_lits(prod, 'project.rs')
for f in ['guid_twin','key_census','role_census','escapes','title_key_nfkd']:
    lines = open(f + '.rs').read().split('\n')
    # cut at #[cfg(test)] mod tests
    cut = next((i for i,l in enumerate(lines) if l.startswith('mod tests') ), len(lines))
    add_lits(lines[:cut], f + '.rs')
# DE1 aliases
blk = '\n'.join(src[1375:1525])
de1 = dict(re.findall(r'\(\s*"(DE1-[^"]+)",\s*(?:"([^"]+)"|([A-Z_]+))\s*,?\s*\)', blk) and [(a, b or c) for a,b,c in re.findall(r'\(\s*"(DE1-[^"]+)",\s*(?:"([^"]+)"|([A-Z_]+))\s*,?\s*\)', blk)])
consts = {'LOGICAL_NOTICE_FIELD':'BT-701-notice','SUBTYPE_FIELD':'OPP-070-notice','ORG_NAME_FIELD':'BT-500-Organization-Company','ORG_IDENTIFIER_FIELD':'BT-501-Organization-Company','ORG_COUNTRY_FIELD':'BT-514-Organization-Company'}
de1 = {k: consts.get(v, v) for k, v in de1.items()}
print("DE1 aliases:", len(de1), file=sys.stderr)
def stem(fid):
    idx = [m.start() for m in re.finditer('-', fid)]
    return fid[:idx[1]] if len(idx) >= 2 else fid
# A ids: every token in shadow.out / shadow_extra.out / moves that looks like a field id
A = set()
ID = re.compile(r'\b((?:BT|OPT|OPP|OPA|DE1|UBL|SDK01|TED|TXT)-[A-Za-z0-9()_.\-]+)')
for f in ['/home/user/tender-db/.scratch/tender-db/577-triage/shadow.out','/home/user/tender-db/.scratch/tender-db/577-triage/shadow_extra.out',
          '/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/moves_bind.tsv','/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/moves_extra.tsv']:
    for line in open(f):
        if line.startswith('    versions') or line.startswith('    alias'): continue
        for m in ID.finditer(line):
            tok = m.group(1).rstrip('.-')
            # strip trailing "(declared)" glued etc
            A.add(tok)
# drop path fragments like 'BT-...' inside xpaths? xpaths have no BT- tokens normally
A = {a for a in A if not a.endswith('(declared') and len(a) > 4}
print("A ids:", len(A), file=sys.stderr)
analyst_A = set(l.split('\t')[0] for l in open('/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/set_A.tsv').read().splitlines()[1:])
print("mine-not-analyst:", sorted(A - analyst_A)[:50], file=sys.stderr)
print("analyst-not-mine:", sorted(analyst_A - A)[:50], file=sys.stderr)
prefix_lits = [l for l in lits if re.match(r'^(BT|OPT|OPP|DE1|UBL|SDK01|TED|TXT)-', l)]
hits = collections.defaultdict(set)
for a in sorted(A):
    forms = {a: 'id', stem(a): 'stem'}
    n = de1.get(a)
    if n:
        forms[n] = 'norm'; forms[stem(n)] = 'norm-stem'
    for f, how in forms.items():
        if f in lits:
            hits[a].add(f"{how}={f} @ {lits[f][0]}(+{len(lits[f])-1})")
    for l in prefix_lits:
        if l.endswith('-') or l.endswith('(') :
            for f in forms:
                if f.startswith(l) and l != f:
                    hits[a].add(f"prefix={l} @ {lits[l][0]}")
inter = set(l.split('\t')[0] for l in open('/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/intersection.tsv').read().splitlines()[1:])
nonw = set(l.split('\t')[0] for l in open('/home/user/tender-db/.scratch/tender-db/551-lot-sum/b6/w7/nonW_reads_in_A.tsv').read().splitlines()[1:])
for a in sorted(hits):
    tag = 'ANALYST-HIT' if a in inter else ('ANALYST-NONW' if a in nonw else 'UNCLASSIFIED')
    print(tag, a, ' | '.join(sorted(hits[a]))[:300])
