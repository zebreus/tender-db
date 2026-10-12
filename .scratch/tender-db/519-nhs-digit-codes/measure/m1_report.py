#!/usr/bin/env python3
"""Issue 519 M1/M5 report: m1.json (from m1_score.py) -> m1.md, and the M5 holders -> m5.json + m5.md.

    python3 -I -B m1_report.py
"""
import collections
import datetime as dt
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import child_check_sim as cc  # noqa: E402
from m1_score import body_of, armed, org_name_norm  # noqa: E402  (re-runs the scorer's loads; cheap)

RAW = os.path.join(HERE, 'm1-raw')
d = json.load(open(os.path.join(HERE, 'm1.json')))
H = d['holders']


def day(t):
    return dt.datetime.fromtimestamp(t, dt.timezone.utc).date().isoformat() if t else '-'


def target_org(body):
    """A stray's target: the 513 survivor of the code it names, when one stands."""
    if body in cc.table and cc.table[body]['allow']:
        b = body_of(body) or {}
        s = b.get('survivor')
        if s:
            return f"{s['org_id']} ({s['kind']})"
        return f"no applied 513 body ({b.get('status')})"
    if body in cc.table:
        return 'off the allowlist (' + ('Inactive ' if cc.table[body]['status'] == 'I' else '') + cc.table[body]['role'] + ')'
    return 'to be read (M7)'


L = []
w = L.append
w('# 519 M1 — every child holder\'s mention names under `child_check`, and M5 — the zero-only tails\' holders')
w('')
w(f'Computed {d["computed"]} from `m1_score.py` (table `{d["tsv_version"]}`). Per-name detail: `m1.json`; '
  'M5: `m5.json`, `m5.md`.')
w('')
w('## Method')
w('')
w('- **Holders (85).** The 52 RO98 sub-ICB orgs (`../sub-icb.json`), the 25 RO198/RC2 trust-site orgs '
  '(`../gbnhs-orgs-classified.json`), and the 8 provisional holders of the zero-only tails 00P, 00L and 00X found for '
  'M5. The QHM survivor 31556600 also holds 00P and 00N; it is the body, so it is listed in M5 only.')
w('- **Mentions and dates.** Every mention of every holder, and each notice\'s `published_at`. The 77 sub-ICB and site '
  'holders come from M4\'s bounded reads (`m4-reads.json`: 1,880 mentions, 1,861 notices), reused so that the box was '
  'read only once. The zero-only holders were read for this unit (`m1-raw/m5_*`). All reads were indexed: '
  '`organizations` PK, a `name_norm` range seek, `organization_mentions_org`, `notices` PK, `tender_versions_notice`, '
  'and the `tender_versions` PK. IN lists had at most 25 ids; every read had a LIMIT. There were no 408s and no '
  'retries.')
w('- **Parents.** The dated parents come from each child\'s ODS record: `../ods-table-codes/`, else M2\'s refetch '
  '`ods-ro98/` (03K: HTTP 200 now), else `../ods/`. The relations are RE5 → RO261 for RO98 children and RE6 → RO197 for '
  'sites, with Operational dates. Adjacent intervals with one parent are merged. RE5 targets of other roles (RO210, RO173) '
  'are dropped and listed in `m1.json`. The map matches M2\'s `child-map.json` row for row for all 85 holders\' codes.')
w('- **The gate.** `child_check_sim.py` ports decision §1.3. It is 513\'s `check` (from `check_sim.py`, updated to the '
  'nine-field TSV and to the Rust `charity` markers) with two changes. First, Active RO98 rows are dropped from the '
  'rivals. Second, an `nhse` refusal is added, placed after NonNhs.')
w('  - Self-test: the 513 tally still reads 233/235 SAME registry and 116/116 SAME PPON, with 0 DIFFERENT/NOT_ORG admitted.')
w('  - Every row of §1.3\'s expected table matches except one. "College Road Surgery" under QHL answers `non-nhs`, not '
  'the `zero` the decision expects: `college` without `hospital` trips the Rust `non_nhs` first. It is refused either way.')
w('- **The chosen parent** is the dated parent a holder would join. Allowlisted parents come first. Among those, the one '
  'admitting the most mentions wins, then the one admitting the head name, then the earlier period.')
w('- **Name calls** under the chosen parent:')
w('  - `admitted`;')
w('  - `stray`: another body\'s name, i.e. a rehome candidate when the holder also carries admitted names. A name is '
  'called stray when it is admitted under another dated parent, when it is `nhse`, or when `child_check` admits it under '
  'the table row it fits best;')
w('  - `noise`: the body\'s own name refused by the gate, e.g. `better` against the child\'s own predecessor CCG;')
w('  - `site`: a trust site\'s own site or service name.')
w('  - 22 names were called by hand (`m1_overrides.json`, each with its reason).')
w('')

# ---- class counts
w('## Counts per class')
w('')
w('Holder classes:')
w('- `all-admitted`: every name, the head name included, is admitted under the chosen parent.')
w('- `other-body-strays`: admitted names plus other bodies\' names. These are the rehome candidates.')
w('- `own-name-noise`: admitted names plus refused own names.')
w('- `none-admitted:*`: no name is admitted. The holder is wholly another body\'s, wholly site-named, or (W2U3Z) a mix.')
w('- `no-parent`: no RO261 parent; ODS gives RO210 regions only.')
w('')
w('| kind | class | holders | mentions |')
w('|---|---|---:|---:|')
for kind in ('sub-icb', 'site', 'zero-only'):
    hs = [h for h in H if h['kind'] == kind]
    c = collections.Counter(h['class'] for h in hs)
    m = collections.Counter()
    for h in hs:
        m[h['class']] += h['mentions']
    for k in sorted(c):
        w(f'| {kind} | {k} | {c[k]} | {m[k]:,} |')
    w(f'| **{kind}** | **all** | **{len(hs)}** | **{sum(h["mentions"] for h in hs):,}** |')
w('')
w('By the decision\'s partition (§1.5), using the chosen parent:')
w('- `A-ppon-survivor`: phase A.')
w('- `A2-anchor-survivor`: held under ruling 5.')
w('- `A3-off-allowlist`: a digit or five-character parent (U6, or nothing).')
w('')
w('| kind | phase | class | holders | mentions | admitted | stray | noise | site |')
w('|---|---|---|---:|---:|---:|---:|---:|---:|')
ct = collections.defaultdict(lambda: [0, 0, 0, 0, 0, 0])
for h in H:
    k = (h['kind'], h['phase'], h['class'])
    v = ct[k]
    v[0] += 1; v[1] += h['mentions']; v[2] += h['admitted_mentions']; v[3] += h['stray_mentions']
    v[4] += h['noise_mentions']; v[5] += h['site_mentions']
for k in sorted(ct):
    v = ct[k]
    w(f'| {k[0]} | {k[1]} | {k[2]} | {v[0]} | {v[1]:,} | {v[2]:,} | {v[3]} | {v[4]} | {v[5]} |')
w('')

# ---- ruling 2 / S1
pa = [h for h in H if h['phase'] == 'A-ppon-survivor']
pa_m = sum(h['mentions'] for h in pa)
only_noise = [h for h in pa if h['noise_mentions'] and not h['stray_mentions'] and not h['site_mentions']]
any_noise = [h for h in pa if h['noise_mentions']]
w('## Ruling 2 and S1: what the strict all-names gate costs')
w('')
w(f'Phase A has {len(pa)} holders and {pa_m:,} mentions in all: sub-ICB, sites and zero-only clusters.')
w('')
w(f'**Held only by refusals of the body\'s own names: {len(only_noise)} holder(s), '
  f'{sum(h["mentions"] for h in only_noise)} mention(s) '
  f'({100 * sum(h["mentions"] for h in only_noise) / pa_m:.2f}% of phase-A mentions).**')
for h in only_noise:
    names = '; '.join(f'"{s["name"]}" ({s["n"]})' for s in h['name_detail'] if s.get('call') == 'noise')
    w(f'- {h["child"]} {h["org_id"]}: {names}')
w('')
w(f'Phase-A holders carrying any own-name noise: {len(any_noise)}. S1\'s halt (more than 3 holders, or more than 5% of '
  'phase-A mentions) does **not** trip.')
w('')
w('Apart from that one typo cluster, own-name noise exists only under anchor-survivor parents, which ruling 5 holds anyway:')
w('- 92G (2): "Bath, Swindon & Wiltshire ICB" `better{13G}`; "NHS Bath and North East Somerset ICB" `better{11E}`.')
w('- 72Q (2): "SEL ICB (Lewisham & Greenwich)" `zero`.')
w('- 36L (2): "NHS South West London" `tie{RVM}`.')
w('- 70F (2): "NHS Sussex" `tie{RDR}`.')
w('- W2U3Z (2): QRV\'s own name ties with its successor Z9B2Z.')
w('')
w('In phase A the only own-name refusal is that typo; every other phase-A refusal is another body\'s name. So M1 gives '
  'no case for the attestation alternative to ruling 2.')
w('')

# ---- rehome candidates
w('## Other-body strays: the rehome candidates (§1.4)')
w('')
w('Mixed holders, admitted names plus other bodies\' names. Target is the 513 survivor of the named code where one stands.')
w('')
w('| holder | org | phase | admitted | stray name | n | published | body | target |')
w('|---|---:|---|---:|---|---:|---|---|---|')
for h in sorted(H, key=lambda h: (h['phase'], -h['mentions'])):
    if h['class'] not in ('other-body-strays', 'strays-and-noise'):
        continue
    for s in h['name_detail']:
        if s.get('call') != 'stray':
            continue
        w(f'| {h["child"]} | {h["org_id"]} | {h["phase"]} | {h["admitted_mentions"]}/{h["mentions"]} | {s["name"]} '
          f'{"(head)" if s["head"] else ""} | {s["n"]} | {day(s["first"])}{"" if day(s["first"]) == day(s["last"]) else " → " + day(s["last"])} '
          f'| {s["body"]} | {target_org(s["body"])} |')
w('')
st_pa = [h for h in pa if h['class'] == 'other-body-strays']
w(f'**Phase A:** {len(st_pa)} mixed holders and {sum(h["mentions"] for h in st_pa):,} mentions; '
  f'{sum(h["stray_mentions"] for h in st_pa)} of those mentions are strays.')
w('- 13T: 15 CSU-named (to 0AR\'s survivor 30929531) and 1 NHS England.')
w('- 26A: James Paget (RGP) and Norfolk & Norwich (RM1), 1 each.')
w('- 15E: 4 — Black Country to QUA, plus three GP practice names (Wake Green, College Road, Small Heath).')
w('- 15N: 2 BNSSG, to QUY.')
w('- 15C: 1 "NHS Devon ICB", to QJK.')
w('- 93C: 1 "N E L Commissioning Support Unit", which is 0AQ, Inactive.')
w('')
w('**13T\'s head name is the CSU\'s** ("NORTH OF ENGLAND COMMISSIONING SUPPORT", the 15 CSU mentions). E5 gates the '
  'head name too. Rehoming the 15 mentions therefore leaves 13T held until its head re-designates to a NENC name. '
  'U3 must check the head after `apply-rehoming`.')
w('')
w('The GP-practice strays (15E ×3) and 11X\'s "Rural Practice Network" are not ODS-table rows: M7 must read their notices '
  'to fix a target org.')
w('')

# ---- whole-holder refusals
w('## Holders with no admitted name')
w('')
w('| holder | org | kind | phase | class | names (n) → body |')
w('|---|---:|---|---|---|---|')
for h in H:
    if not h['class'].startswith('none-admitted'):
        continue
    names = '; '.join(f'"{s["name"]}" ({s["n"]}) → {s.get("body")}' for s in h['name_detail'])
    w(f'| {h["child"]} | {h["org_id"]} | {h["kind"]} | {h["phase"]} | {h["class"]} | {names} |')
w('')
w('These never join under the child rule:')
w('- the site-named sites, which keep 513\'s "hosted sites stay separate";')
w('- the wholly other-body holders, the real mis-coding: RH8C3 → QJK, RTR54 and X7V4Q → QHM, RJ128 → RJZ, '
  'RW5NC → NHS England, 03Q → NHS England, 04Y → QOC. They are 452 or 317 material, not 519\'s.')
w('')

# ---- moved children
w('## Moved children: dated counts (ruling 4, no date fence)')
w('')
w('| child | name | n | before P | in P | after P | under the later parent |')
w('|---|---|---:|---:|---:|---:|---|')
for h in H:
    ps = h['parents'] or []
    if len([p for p in ps if p['parent'] in cc.table]) < 2:
        continue
    P = h['chosen_parent']
    for s in h['name_detail']:
        per = s['periods'].get(P) or {}
        later = [(p['parent'], s['check'][p['parent']]['verdict']) for p in ps if p['parent'] in s['check'] and p['parent'] != P]
        w(f'| {h["child"]} (P={P}) | {s["name"]} | {s["n"]} | {per.get("before_start", 0)} | {per.get("in_period", 0)} | '
          f'{per.get("after_end", 0)} | {", ".join(f"{a} {b}" for a, b in later)} |')
w('')
w('The names published after the old parent\'s end are still the old body\'s own names: 26A 2, D4U1Y 8, 93C 1, 10Q 1. '
  'Under ruling 4 they fold with the rest. The new body\'s name appears under a moved child only here:')
w('- 10Q: 1, "NHS Thames Valley ICB" (2026-06-18), a stray under QU9;')
w('- W2U3Z: 1, "NHS West and North London ICB" (2026-10-02), a stray under QRV;')
w('- 15A: 22 Thames Valley names. S0E4D is chosen because both of 15A\'s parents are off the allowlist, so 15A\'s one '
  'BOB name (2024-12-13) is the stray, under QU9.')
w('')
w('No "Norfolk and Suffolk" name was published under 26A, and no "Hampshire and Isle of Wight" name under D4U1Y.')
w('')

# ---- sites
w('## Trust sites (ruling 7)')
w('')
sites_a = [h for h in H if h['kind'] == 'site' and h['phase'] == 'A-ppon-survivor' and h['class'] == 'all-admitted']
w(f'Ruling 7 admits the trust-named sites under PPON-survivor trusts that the gate passes: **{len(sites_a)} sites / '
  f'{sum(h["mentions"] for h in sites_a)} mentions**.')
for h in sites_a:
    w(f'- {h["child"]} {h["org_id"]} "{h["head_name"]}" under {h["chosen_parent"]}; survivor {target_org(h["chosen_parent"])}')
w('')
w('The other 19 site orgs fall into three groups:')
w('- trust-named, but under anchor or digit trusts (3 + 3);')
w('- site-named (8);')
w('- wholly another body\'s (5).')
w('')
w('B4Z5Z, "The Royal Marsden Hospital", is admitted under RPY. It is the site\'s name and also the trust\'s common name, '
  'so the reviewer decides.')
w('')

# ---- corrections
w('## Corrections to the decision\'s counts')
w('')
w('- **15C** is not clean: 1 "NHS Devon Integrated Care Board" mention (a stray to QJK) sits among 30 BNSSG names. It '
  'moves from A1 to the mixed group.')
w('- **03K** is clean phase A: 12 × "NHS Humber and North Yorkshire ICB" under QOQ (PPON survivor 31573365). A1 is '
  'therefore still 19 holders / 742 mentions: +03K (12), −15C (31).')
w('- **13T**: 488 admitted, 15 CSU, 1 NHS England (all names, not only the top five).')
w('- **The phase-A mixed group** is 6 holders / 750 mentions, with 26 stray mentions in all. The decision had 5 / 719 and '
  '"about 22" strays.')
w('- **92G**\'s `better{13G}` is a false refusal of BSW\'s own name. 92G is held anyway (anchor).')
w('- **93C**\'s CSU name is 0AQ (North and East London CSU, Inactive). 93C has no "North Central London ICS" stray: '
  'that name is admitted (coverage 3/4).')
w('- **W2U3Z**: 2 of QRV\'s own names tie with Z9B2Z, and 1 is Z9B2Z\'s own name (2026-10-02).')
w('')

# ---- per-holder table
w('## Per holder')
w('')
w('| kind | child | org | head name | parents (start→end) | chosen | phase | class | mentions | adm | stray | noise | site |')
w('|---|---|---:|---|---|---|---|---|---:|---:|---:|---:|---:|')
for h in H:
    ps = ', '.join(f'{p["parent"]} {p["start"]}→{p["end"] or ""}' for p in (h['parents'] or [])) or '—'
    w(f'| {h["kind"]} | {h["child"]} | {h["org_id"]} | {(h["head_name"] or "")[:60]} | {ps} | {h["chosen_parent"] or "—"} | '
      f'{h["phase"]} | {h["class"]} | {h["mentions"]} | {h["admitted_mentions"]} | {h["stray_mentions"]} | '
      f'{h["noise_mentions"]} | {h["site_mentions"]} |')
w('')
open(os.path.join(HERE, 'm1.md'), 'w').write('\n'.join(L) + '\n')

# ================================ M5 ================================
qhm_zero = json.load(open(os.path.join(RAW, 'm5_qhm_zero.json')))['rows']
surv_names = collections.Counter((r[4], r[2]) for r in qhm_zero)
m5 = {'tails': {}, 'holders': [], 'survivor': {'org_id': 31556600, 'code': 'QHM', 'identifier': 'GBPPONPMYZ6944BYLH',
                                              'provisional': 0, 'zero_coded_mentions': [
                                                  {'raw': k[0], 'name': k[1], 'n': v} for k, v in sorted(surv_names.items())]}}
plan_off = {}
plan = json.loads(json.load(open(os.path.join(os.path.dirname(os.path.dirname(HERE)), '513-nhs', 'u5',
                                              'plan-s4-after-daily.json')))['body'])
for o in plan['off_allowlist']:
    if o['tail'] in ('00P', '00N', '00L', '00X'):
        plan_off[o['tail']] = {'holders': o['holders'], 'parties': o['parties'], 'later_parties': o['later_parties'],
                               'top_names': o['top_names'], 'ods_name': (o['ods'] or {}).get('name')}
m5['tails'] = plan_off
pre = json.loads(json.load(open(os.path.join(os.path.dirname(os.path.dirname(HERE)), '513-nhs', 'u5',
                                             'plan-2274-dry.json')))['body'])
m5['tails_before_513_wet'] = {o['tail']: {'holders': o['holders'], 'parties': o['parties']}
                              for o in pre['off_allowlist'] if o['tail'] in ('00P', '00N', '00L', '00X')}
qhm_rev = set((plan_codes := {c['code']: c for c in plan['codes']})['QHM']['verdict']['reviewed_holders'])
for h in H:
    if h['kind'] != 'zero-only':
        continue
    raws = collections.Counter()
    for s in h['name_detail']:
        for k, v in s['raws'].items():
            raws[k] += v
    m5['holders'].append({
        'org_id': h['org_id'], 'tail': h['child'], 'standing': True, 'provisional': h['provisional'],
        'identifier': h['identifier'], 'head_name': h['head_name'], 'mentions': h['mentions'], 'raws': dict(raws),
        'names': [{'name': s['name'], 'n': s['n'], 'first': day(s['first']), 'last': day(s['last']),
                   'check': s['check'], 'armed_513_key': s['armed_513_key']} for s in h['name_detail']],
        'armed_513_key_any': any(s['armed_513_key'] for s in h['name_detail']),
        'in_qhm_513_reviewed_holders': h['org_id'] in qhm_rev,
        'parent': h['chosen_parent'], 'phase': h['phase'], 'm1_class': h['class']})
json.dump(m5, open(os.path.join(HERE, 'm5.json'), 'w'), indent=1)

L = []
w = L.append
w('# 519 M5 — the zero-only tails\' holders (00P, 00N, 00L, 00X)')
w('')
w('Read 2026-10-12, bounded and indexed. The raw reads are in `m1-raw/m5_*`, the detail in `m5.json`, and the '
  'gate scores come from `m1.json`.')
w('')
w('## How they were found')
w('')
w('The census lists holder ids only for allowlisted codes. A zero-only tail\'s holders show up only through `other_codes` '
  'on a QHM holder, which gives 3 of the 10. Neither `section_id` nor `raw_identifier` has an index. The other 7 were '
  'found in three steps:')
w('1. `name_norm` range seeks on the tails\' published names and their spellings (32 prefixes);')
w('2. each candidate\'s mentions read by `organization_mentions_org`, filtered to `raw_identifier LIKE \'GB-NHS-00%\'`;')
w('3. a sibling-notice pass (`tender_versions_notice`, then the `tender_versions` PK), which confirmed no 00P party sat '
  'on another holder of the same tenders.')
w('')
w('Every census count is accounted for: 00P has 6 holders / 25 parties, 00N 1 / 5, 00L 1 / 1, 00X 2 / 2.')
w('')
w('## The holders')
w('')
w('| tail | org | standing | provisional | head name | mentions (raws) | 513 key armed | in QHM 513 `reviewed_holders` | parent → M1 |')
w('|---|---:|---|---|---|---|---|---|---|')
w('| 00P ×20, 00N ×5 | 31556600 | yes | no (PPON `GBPPONPMYZ6944BYLH`) | NHS North East and North Cumbria Integrated Care Board '
  '| 2,395 (25 zero-coded, names all armed QHM keys) | — (it is the survivor) | yes (member) | QHM survivor |')
for x in m5['holders']:
    raws = ', '.join(f'{k} ×{v}' for k, v in sorted(x['raws'].items(), key=lambda kv: -kv[1]))
    w(f'| {x["tail"]} | {x["org_id"]} | yes | {"yes" if x["provisional"] else "no"} | {x["head_name"]} | {x["mentions"]} ({raws}) | '
      f'{"yes" if x["armed_513_key_any"] else "no"} | {"yes" if x["in_qhm_513_reviewed_holders"] else "no"} | '
      f'{x["parent"]} ({x["phase"]}) → {x["m1_class"]} |')
w('')
w('## What this means for the decision')
w('')
w('- **00N is already folded.** Its only holder is the QHM survivor itself: 5 parties, which arrived through 513\'s fold '
  'of 30915134. **25 of the 31 "zero-only parties under QHM" are already on the survivor**: 00P ×20 (through the folds '
  'of 30915134, 31538425, 31593327, 31595235 and 31604957) and 00N ×5. Before 513\'s wet run, 00P had 10 holders; now it '
  'has 6.')
w('- **Six provisional holders are left under QHM: 6 mentions coded 00P/00L, 12 mentions in all.**')
w('  - Already 513 holders: 31603816 ("… - County Durham Place", 5 mentions, 4 of them coded QHM) and 31605903 ("… - Tees '
  'Valley Place", 3, 2 coded QHM). Both carry QHM raws, so they are QHM\'s own census holders, and 513 U3 adjudicated them '
  '`HOSTED_OR_SITE` and left them out (in `reviewed_holders`, not in `members`). `child_check` **admits** both names: the '
  'place suffix still clears coverage. A 519 verdict naming them would reverse a reviewed 513 call. Default: leave them out.')
w('  - Single-mention 00P/00L clusters admitted by the gate: 31610859 ("The North East and North Cumbria Integrated Care '
  'System (ICS)"), 31615979 ("The North East and North Cumbria Integrated Care Board (ICB)") and 31632651 (00L, "NHS North '
  'East and North Cumbria Integrated Care Boards"). These are the real zero-only name members.')
w('  - 31615636, "NHS North East and North Cumbira ICB" (1, typo): `tie{QMF}`, so the strict gate holds it. This is the '
  'only own-name-noise hold in phase A.')
w('- **No zero-only holder\'s name is an armed 513 alias key.** 513 armed QHM\'s keys only for the 9 folded spellings. So '
  'a new mention with any of these 7 names still binds its own provisional row by name, whatever its code. The decision\'s '
  'fold of these holders would arm their names as new keys (`loser_name_norms`), which 513 did not.')
w('- **00X is not a NENC cluster and not under an allowlisted parent.** Its parent is QE1, a digit code off the '
  'allowlist (U6).')
w('  - 30915461 is the Lancashire and South Cumbria ICB name cluster: 252 mentions, 251 uncoded, 1 coded 00X. It is a body '
  'name cluster, not a child cluster.')
w('  - 31624168 ("Lancashire and Cumbria integrated Care Board", 1) is admitted under QE1.')
w('  - **For U6:** the LSC body has a standing PPON org, 31569664 (`GBPPONPBWM8336DJRG`), beside the identity org 30915463 '
  '(`GBNHSQE1`) and a typo identity 31549058 (`GBNHSQE1000`). U6\'s "the one standing GBNHS<P> org becomes the survivor" '
  'would leave the PPON org apart, so U6 needs the PPON tier first.')
w('')
open(os.path.join(HERE, 'm5.md'), 'w').write('\n'.join(L) + '\n')
print('wrote m1.md, m5.json, m5.md')
