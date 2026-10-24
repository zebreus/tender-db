# usage: python3 -I write_md.py — renders r1.md and r2.md from r1.json, r2.json, r1_sample.json and q8_class.json.
import json, os
from collections import Counter
D = os.path.dirname(os.path.abspath(__file__))
r1 = json.load(open(D + '/r1.json')); r2 = json.load(open(D + '/r2.json')); S = json.load(open(D + '/r1_sample.json'))
cls = json.load(open(D + '/q8_class.json'))
def esc(s): return str(s).replace('|', '\\|')
L = []
L += ['# 572 unit 2 — R1, served state (read 2026-10-24)', '',
      'Bar (decision.md "Read bars", R1): **40 R, 0 W, at most 5 replaced N.**',
      'Result: **40 R (10 exemplars + 30 salted), 0 N, 0 W, 0 replacements.** The 3 v1-only exemplars read N as expected (5 class rows, none on the current version). **PASS.**', '',
      '## Reads', '',
      '- 1 `/v1/sql` request, `q8_rows` (rc=0, 0.99 s, 263 rows, not truncated), logged "(572 u2 R1)" in `../requests.log`. 0 retries, no 408.',
      '  - SQL: `q8_rows.sql`: q6\'s 22-arm UNION ALL (`../q6_current.sql`), ungrouped, selecting `p.tender_id`, `p.seq = t.current_seq`,',
      '    `p.organization_id`, `p.role`, `m.notice_id`, `m.section_id`, `m.name`; same FROM / CROSS JOIN order / `+m.name` filters as q6.',
      '  - Plan: `q8_rows.plan.txt`, probed before the run on a private copy of `b6u6kit/plan.db` with python sqlite3 and turso',
      '    `plan-probe`: per arm `SEARCH p USING INDEX tender_version_parties_org`, `SEARCH t USING INTEGER PRIMARY KEY`,',
      '    `SEARCH m USING INDEX sqlite_autoindex_organization_mentions_1`; 66 SEARCH steps, 0 SCAN, no temp B-tree.',
      '  - Body: `q8_rows.out.json`.',
      '- 43 GETs `/v1/tenders/{id}` (13 exemplars, 30 salted), all 200, at least 2.1 s apart, one line each in `../requests.log`',
      '  ("(572 u2 R1ex)" / "(572 u2 R1s)"); bodies in `bodies/`. The job queue read idle (current null, queued []) before the',
      '  statement and before each GET batch.',
      '- Scripts: `classify.py` (class rows, analyze_q6.py\'s exclusions), `sample.py` (the sample, written to `r1_sample.json`',
      '  before any GET), `score.py` (rubric, writes `r1.json`), `get.sh` (the GET runner), `write_md.py` (this file).', '']
dc = Counter((c['direction'], c['cur']) for c in cls)
newrows = [c for c in cls if not c['in_q4']]
L += ['## The class today against 2026-10-19', '',
      '| | 2026-10-19 (q6) | 2026-10-24 (q8) |', '|---|---|---|',
      '| class party rows | 191 | %d |' % len(cls),
      '| P@A current / older | 97 / 56 | %d / %d |' % (dc[('P@A', 1)], dc[('P@A', 0)]),
      '| A@P current / older | 13 / 25 | %d / %d |' % (dc[('A@P', 1)], dc[('A@P', 0)]),
      '| non-class rows q6\'s arms also return (V / U / T / agent pattern) | 10 / 47 / 6 / 4 | 11 / 49 / 6 / 4 |', '',
      'All 191 class mentions of 2026-10-19 are still present. Two class rows are new since then, both current P@A buyer rows:']
for c in newrows:
    L.append('- tender %d, notice %d `%s`: "%s" on %d (%s)%s.' % (c['tender_id'], c['notice_id'], c['section_id'], c['name'], c['org'], c['agent'],
             ' — a **new key**, not on frozen list (a): monitor M\'s "unreviewed key" bucket' if c['name'] == 'COMMUNITY GATEWAY ASSOCIATION LIMITED' else ' — an existing reviewed key'))
L += ['', 'The 5 transferred mentions are all still bound as on 2026-10-19 (Riverside and Hywel Dda Tenderer current, Swansea University',
      'older-version, Pagabo ×2 current); they were excluded from the sample.', '']
L += ['## Sample', '',
      '- (a) the 10 exemplar tenders whose class party is current, scored on their current class row; the 3 v1-only exemplars',
      '  (8654651, 8621643, 8613154) read, expected N.',
      '- (b) eligible: current class rows, not one of the 5 transferred mentions, not on any of the 13 exemplar tenders: %d rows'
      % S['eligible_rows'] + ' (P@A %d, A@P %d), on %d P@A and %d A@P tenders.' % (S['eligible_by_stratum']['P@A'], S['eligible_by_stratum']['A@P'], S['eligible_tenders_by_stratum']['P@A'], S['eligible_tenders_by_stratum']['A@P']),
      '  Each stratum ordered by `sha256(\'572-u2-\' + tender_id + \'/\' + notice_id + \'/\' + section_id)`, first row per tender kept;',
      '  A@P 8 of 9 tenders, then P@A 22. Tender 8626810 (the ABUHB/NWSSP swap on 46819472) is in both strata; its A@P row was',
      '  picked and its P@A row sits at P@A position 64, so the overlap changes no pick. Replacement queues (A@P 1, P@A 66) were',
      '  not needed.', '']
L += ['## Scored table', '',
      '| # | set | tender | v | org (served head) | role | notice / section | printed name = served `mention_name` | score |',
      '|---|---|---|---|---|---|---|---|---|']
i = 0
for x in r1['exemplars']:
    i += 1; p = x['detail'].get('party') or {}
    L.append('| %d | exemplar %s | %d | %s | %d (%s) | %s | %d `%s` | %s | **%s** |' % (i, x['direction'], x['tender_id'], x['detail']['version'], x['org'], esc(p.get('organization_name')), x['role'], x['notice_id'], x['section_id'], esc(x['name']), x['score']))
for x in r1['salted']:
    i += 1; p = x['detail'].get('party') or {}
    L.append('| %d | salted %s (%s) | %d | %s | %d (%s) | %s | %d `%s` | %s%s | **%s** |' % (i, x['stratum'], x['salt'][:8], x['tender_id'], x['detail']['version'], x['org'], esc(p.get('organization_name')), x['role'], x['notice_id'], x['section_id'], esc(x['name']), ' (new since 10-19)' if x['new_since_q4'] else '', x['score']))
L += ['', 'v = the tender\'s current version in the GET body. No scored tender has a version published on or after 2026-10-19 except',
      'those whose class row is itself that version\'s notice (tender 8858460, notice 47428982).' if any(x['detail']['versions_published_since_cutoff'] for x in r1['salted'] + r1['exemplars']) else 'No scored tender has a version published on or after 2026-10-19.', '',
      '### v1-only exemplars (expected N)', '',
      '| tender | class row (older version) | current v | current buyer parties (org ← `mention_name`) | score |', '|---|---|---|---|---|']
for x in r1['v1_only']:
    L.append('| %d | %d on %d ← "%s" | %s | %s | %s (expected N) |' % (x['tender_id'], x['notice_id'], x['org'], esc(x['name']), x['current_version'],
             '; '.join('%d ← "%s"' % (p['organization_id'], esc(p['mention_name'])) for p in x['current_buyer_parties']), x['score']))
L += ['', '## Verdict', '',
      '40 R / 0 W / 0 replaced N: the bar is met. Every sampled class party is served with `organization_id` = the identifier\'s',
      'holder and `mention_name` byte-equal to the printed body, in both directions (P@A 8 exemplars + 22 salted = 30, A@P 2 exemplars + 8 salted',
      '= 10) and for the host case (Star on Trafford, 8655231 and 8655040). 456 design 3 holds; the caveat\'s served-state',
      'sentences are true today. Stop rule not triggered.', '']
open(D + '/r1.md', 'w').write('\n'.join(L) + '\n')

M = ['# 572 unit 2 — R2, `?buyer=` semantics (read 2026-10-24)', '',
     'Bar (decision.md "Read bars", R2): **6 of 6 rows exactly as stated, 0 W.** Result: **6 of 6, 0 W. PASS.**', '',
     '## Reads', '',
     '- 11 GETs `/v1/tenders?buyer=<org>&limit=1000`, all 200, logged "(572 u2 R2)" in `../requests.log`, at least 2.1 s apart;',
     '  bodies in `bodies/v1_tenders_buyer_<org>_limit_1000.json`. Every list fit one page (`more` false, `next_cursor` null,',
     '  `ignored_filters` []), so no cursor page was needed. The job queue read idle before the batch.',
     '- Script: `r2_score.py` (writes `r2.json`).', '',
     '## Lists', '', '| org | list | tenders | pages | complete | on 2026-10-19 |', '|---|---|---|---|---|---|']
prev = {'30953692': '74', '30914707': '198', '30914393': '172', '30917293': '31', '30917275': '63 (judges/user-value)'}
names = {'30953692': 'NHC', '30914707': 'Birmingham', '30914393': 'Darlington', '30913798': 'Welland', '30915335': 'Rutland', '30913738': 'NWSSP', '30917293': 'HEIW', '30913722': 'Gibbs Laidler', '2291108': 'English Rural HA (provisional)', '30917275': 'Trafford', '30916230': 'Rochdale'}
for o, mm in r2['lists'].items():
    M.append('| %s | %s | %d | %d | %s | %s |' % (o, names.get(o, ''), mm['tenders'], len(mm['pages']), 'yes' if mm['complete'] else 'NO', prev.get(o, '—')))
M += ['', '## Scored table', '', '| tender | must include (present?) | must exclude (absent?) | score |', '|---|---|---|---|']
for r in r2['rows']:
    inc = '; '.join('%d %s: %s' % (c['org'], c['label'], 'present' if c['present'] else 'ABSENT') for c in r['checks'] if c['must'] == 'include')
    exc = '; '.join('%d %s: %s' % (c['org'], c['label'], 'absent' if not c['present'] else 'PRESENT') for c in r['checks'] if c['must'] == 'exclude') or '—'
    M.append('| %d | %s | %s | **%s** |' % (r['tender'], inc, exc, r['score']))
M += ['', '## Verdict', '',
      '6 of 6. `?buyer=` with the agent\'s id returns the routed Tender, and with the member\'s own organization it does not',
      '(Birmingham, Darlington, Rutland, HEIW, English Rural HA\'s provisional row); the host case lists the Tender under both the',
      'host (Trafford) and the real buyer (Rochdale). The caveat\'s `?buyer=` sentence is true today. Stop rule not triggered.', '',
      'List sizes moved slightly since 2026-10-19 (NHC 74 → 75, with the new Community Gateway Association call-off 8853907;',
      'Birmingham 198 → 199; HEIW 31 → 32; Darlington unchanged at 172).', '']
open(D + '/r2.md', 'w').write('\n'.join(M) + '\n')
print('ok')
