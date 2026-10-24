"""guard_sim2.py (finalizer, 2026-10-24) -- INDICATIVE re-run of the 584 guard with the challenger's
D1/D2/D4 fixes, offline, no box reads:
  D1  holder = every org whose head triple is (PL, national, 5262239325): 36 and 49 (same head name);
  D2  agreement must be on a DISTINCTIVE token (generic Polish/English nouns stoplisted), or an initialism;
  D4  holder vocabulary = reviewed names only: head, ENG satellite, and the register name of the number
      (MF white list: 5262239325 = URZAD ZAMOWIEN PUBLICZNYCH), never families learned from the mentions.
Inputs: 584-u1/org49_mentions.json (all 30,567 org-49 mentions) and challenge-raw/sql/c2_org36.out
(org 36: its 142 bid-party mentions and a 6,000-mention recent sample). Org 36's other ~199k mentions
are NOT available locally, so the org-36 lines are a sample, not a census."""
import json, re, sys, unicodedata
from collections import Counter
sys.path.insert(0, sys.argv[1] + '/scripts')
U2 = sys.argv[1]
STOP = set('''w we i z ze o oo na do przy pod od dla ul al pl sp spółka spolka s a sa spk sk k j jawna akcyjna
ograniczoną ograniczona odpowiedzialnością odpowiedzialnoscia komandytowa cywilna sc the of and for
oddział oddzial siedzibą siedziba'''.split())
# D2: generic nouns that must never carry agreement on their own (5-letter stems).
GENERIC = set('''krajo izba izby izbą urząd urzęd publi państ polsk narod gmina gminn gminy powia miejs miast
komun zakła przed szpit uniwe szkoł centr ośrod bank banku spółd wojew regio depar biuro sekre preze
gospo rolni natio chamb offic publi procu'''.split())
HOLDER_NAMES = ['Krajowa Izba Odwoławcza',          # head of org 36 and of org 49
                'National Appeals Chamber',         # ENG satellite (S6)
                'Urząd Zamówień Publicznych']       # register name of the NIP (MF white list)

def tokens(n):
    return re.findall(r"[^\W\d_]+", unicodedata.normalize('NFC', n or '').replace('\xa0', ' '), re.UNICODE)
def sig(n):
    return [t.lower() for t in tokens(n) if t.lower() not in STOP and len(t) >= 3]
def stem_eq(a, b, k=5):
    if a == b: return True
    n = min(len(a), len(b))
    if n < 4: return False
    p = 0
    while p < n and a[p] == b[p]: p += 1
    return p >= min(k, n - 1)
DISTINCT = sorted({t for h in HOLDER_NAMES for t in sig(h) if t[:5] not in GENERIC})
INITS = {''.join(t[0] for t in sig(h)) for h in HOLDER_NAMES}
def fold(t):
    return ''.join(c for c in unicodedata.normalize('NFKD', t.replace('ł', 'l')) if not unicodedata.combining(c))
def lev(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]
TYPO = '--typo' in sys.argv
def tok_agree(a, b):
    if stem_eq(a, b): return True
    if not TYPO: return False
    fa, fb = fold(a), fold(b)
    return stem_eq(fa, fb) or (min(len(fa), len(fb)) >= 8 and lev(fa, fb) <= 2)
def passes(m):
    ms = [t for t in sig(m) if t[:5] not in GENERIC]
    if any(tok_agree(a, b) for a in ms for b in DISTINCT): return True
    return any(t.isupper() and 2 <= len(t) <= 6 and t.lower() in INITS for t in tokens(m))

print('variant:', 'E2 distinctive + diacritic fold + edit distance <= 2 on tokens >= 8' if TYPO else 'E1 distinctive token, stem(5)')
print('distinctive holder tokens:', DISTINCT, 'initialisms:', sorted(INITS))
rows = json.load(open(f'{U2}/../584-u1/org49_mentions.json'))
names49 = Counter(r[3] for r in rows)
ref = Counter({n: c for n, c in names49.items() if not passes(n)})
print(f'\n== org 49 (all {sum(names49.values())} mentions): refuses {sum(ref.values())} mentions / {len(ref)} names')
for n, c in ref.most_common(): print(f'   {c:6d}  {n!r}')
c2 = json.load(open(f'{U2}/challenge-raw/sql/c2_org36.out'))
bp = {(r[1], r[2]): r[4] for r in c2['rows'] if r[0] == 'bp'}
bpn = Counter(bp.values())
ref = Counter({n: c for n, c in bpn.items() if not passes(n)})
print(f'\n== org 36 bid-party mentions ({sum(bpn.values())}): refuses {sum(ref.values())} / passes {sum(bpn.values())-sum(ref.values())}')
for n, c in bpn.items():
    if passes(n): print(f'   PASS {c:4d}  {n!r}')
nm = Counter({r[4]: r[1] for r in c2['rows'] if r[0] == 'nm'})
ref = Counter({n: c for n, c in nm.items() if not passes(n)})
print(f'\n== org 36 recent sample ({sum(nm.values())} mentions, {len(nm)} names): refuses {sum(ref.values())} mentions / {len(ref)} names')
for n, c in ref.most_common(): print(f'   {c:6d}  {n!r}')
print('\n== challenger probe names (guard_probe.out):')
for n in ['KRAJOWA IZBA GOSPODARCZA', 'Krajowa Izba Kontroli', 'Urząd Miasta Szubin', 'Urząd Gminy Wojcieszków',
          'Bank Gospodarstwa Krajowego', 'Krajowy Ośrodek Wsparcia Rolnictwa', 'Publiczna Szkoła Podstawowa nr 1',
          'Krajowa Spółka Cukrowa S.A.', 'Gmina Skaryszew', 'Ministerstwo Finansów', 'Platforma eZamówienia',
          'Urząd Zamóweń Publicznych', 'Prezes Krajowej Izby Odwoławczej', 'KIO', 'KIO UZP', 'National Appeals Chamber']:
    print('  ', 'PASS (binds KIO)' if passes(n) else 'refused         ', '|', n)
