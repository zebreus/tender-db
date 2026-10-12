#!/usr/bin/env python3
"""519 M3, offline: purity and flags per child holder (52 sub-ICB + 25 RO198 sites). Writes m3.json.

Inputs (all already read; nothing here touches the box):
- m3-raw/holders.json: the 77 holders (unit 1, `../gbnhs-orgs-classified.json`, primary role RO98 / RO198).
- m3-raw/api/<id>.json: the public org detail (identifier, mentions, merged_identifiers, identifier_status).
- m3-raw/admin_<t>.json: the verdict-store readbacks (identifier, merge, rehoming, case, country, name).
- m4-reads.json: every mention of the 77 holders (organization_mentions_org, 2026-10-12 10:21).
- m4-results.json: the §1.5 class per child code.
- ../../513-nhs/u5/plan-s4-after-daily.json: the newest 513 census (off_allowlist ppons/holders per tail,
  countryless_parties, non_gb_parties).
- ../../469-registry-pairs/: the altid plan 2163 (registry_sample), U2 read, U6 plan 2208 and posted verdicts.
"""
import json, os, re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, 'm3-raw')
SCR = os.path.normpath(os.path.join(HERE, '..', '..'))


def norm_ident(raw):
    # project::normalise_identifier's first step: ASCII alphanumerics, upper-cased.
    return ''.join(c for c in raw if c.isascii() and c.isalnum()).upper() if raw else None


def load(p):
    return json.load(open(p))


def main():
    holders = load(os.path.join(RAW, 'holders.json'))
    reads = load(os.path.join(HERE, 'm4-reads.json'))
    classes = {c: v['cls'] for c, v in load(os.path.join(HERE, 'm4-results.json'))['children'].items()}
    admin = {t: load(os.path.join(RAW, f'admin_{t}.json')) for t in
             ['identifier', 'merge', 'rehoming', 'case', 'country', 'name']}
    plan513 = json.loads(load(os.path.join(SCR, '513-nhs/u5/plan-s4-after-daily.json'))['body'])
    off = {e['tail']: e for e in plan513['off_allowlist']}

    ments_by = {}
    for m in reads['mentions']:
        ments_by.setdefault(m[0], []).append(m)

    # 469: every org id / GB-NHS literal the register-pair reports name.
    f469 = {
        'altid-merge-plan-2163 registry_sample': json.loads(load(os.path.join(
            SCR, '469-registry-pairs/u1/altid-merge-plan-2163.json'))['body']).get('registry_sample', []),
        'u2-read (469 U2 reviewer+challenger)': load(os.path.join(SCR, '469-registry-pairs/u2/u2-read.json')),
        'registry-plan-2208 (469 U6)': load(os.path.join(SCR, '469-registry-pairs/u6/registry-plan-2208.json')),
        'verdicts-confirmed (469 U6 posted)': load(os.path.join(SCR, '469-registry-pairs/u6/verdicts-confirmed.json')),
    }
    s469 = {k: json.dumps(v) for k, v in f469.items()}

    merge_rows = admin['merge']['rows']
    for r in merge_rows:
        r['_members'] = set(json.loads(r['members'] or '[]'))
        r['_holders'] = set(json.loads(r['reviewed_holders'] or '[]'))

    out = []
    for h in holders:
        oid, code, ident = h['org_id'], h['code'], h['identifier']
        api = load(os.path.join(RAW, 'api', f'{oid}.json'))
        ms = ments_by.get(oid, [])
        kinds = Counter()
        odd = []
        for m in ms:
            _, nid, sid, name, country, raw, scheme = m
            n = norm_ident(raw)
            if raw is None:
                k = 'no-identifier'
            elif country is None:
                k = 'country-less'
            elif country != 'GB':
                k = 'foreign'
            elif n != ident:
                k = 'other-value'
            elif raw != f'GB-NHS-{code}':
                k = 'own-respelt'
            else:
                k = 'own'
            kinds[k] += 1
            if k not in ('own',):
                odd.append({'notice_id': nid, 'section_id': sid, 'name': name, 'country': country,
                            'raw_identifier': raw, 'scheme': scheme, 'normalised': n, 'kind': k})
        impure = sum(v for k, v in kinds.items() if k not in ('own', 'own-respelt'))
        merged = api.get('merged_identifiers') or []
        merged_ppon = [x for x in merged if (x.get('identifier') or '').startswith(('GBPPON', 'PPON'))]
        own_ppon = (api.get('identifier') or '').startswith(('GBPPON', 'PPON'))
        names = {(m[3] or '').lower() for m in ms} | {h['name'].lower()}
        flags = {
            'identifier_verdicts': [r for r in admin['identifier']['rows']
                                    if r['org_id'] == oid or r['identifier'] == ident],
            'merge_verdicts': [{k: r[k] for k in ('scheme', 'key', 'cohort', 'action', 'confidence')}
                               for r in merge_rows
                               if oid in r['_members'] or oid in r['_holders']
                               or re.search(r'(^|[:~])(GBNHS|GB-NHS-)?' + re.escape(code) + r'($|[~])', r['key'])],
            'rehoming': [r for r in admin['rehoming']['rows']
                         if r['case_org_id'] == oid or r['target_org_id'] == oid],
            'case_reviews': [r for r in admin['case']['rows'] if r['case_org_id'] == oid],
            'country_verdicts': [r for r in admin['country']['rows'] if r['org_id'] == oid],
            'name_verdicts': [r for r in admin['name']['rows'] if r['name_norm'] in names],
        }
        in469 = [k for k, s in s469.items()
                 if re.search(r'(?<!\d)' + str(oid) + r'(?!\d)', s)
                 or re.search(r'(GBNHS|GB-NHS-)' + re.escape(code) + r'(?![A-Z0-9])', s)]
        oa = off.get(code)
        out.append({
            'org_id': oid, 'code': code, 'kind': h['kind'], 'class': classes.get(code),
            'name': h['name'], 'identifier': ident,
            'api': {k: api.get(k) for k in ('country', 'identifier_kind', 'identifier', 'provisional',
                                            'mentions', 'identifier_status', 'merged_identifiers')},
            'mentions_read': len(ms),
            'mention_identifier_kinds': dict(kinds),
            'odd_mentions': odd,
            'child_impure': impure > 0,
            'impure_mentions': impure,
            'own_ppon': own_ppon,
            'merged_ppon': merged_ppon,
            'child_carries_ppon': own_ppon or bool(merged_ppon),
            'child_withheld': any(r['verdict'] in ('wrong', 'related') for r in flags['identifier_verdicts'])
            or api.get('identifier_status') is not None,
            'flags': flags,
            'in_469_reports': in469,
            'census513_off_allowlist': None if oa is None else {
                'parties': oa['parties'], 'holders': oa['holders'], 'ppons': oa['ppons']},
        })

    tot = lambda f: sum(1 for o in out if f(o))
    summary = {
        'holders': len(out),
        'sub_icb': tot(lambda o: o['kind'] == 'sub-icb'),
        'sites': tot(lambda o: o['kind'] == 'site'),
        'mentions_read': sum(o['mentions_read'] for o in out),
        'api_mentions': sum(o['api']['mentions'] for o in out),
        'mention_identifier_kinds': dict(sum((Counter(o['mention_identifier_kinds']) for o in out), Counter())),
        'child_impure': tot(lambda o: o['child_impure']),
        'child_carries_ppon': tot(lambda o: o['child_carries_ppon']),
        'child_withheld': tot(lambda o: o['child_withheld']),
        'merged_identifiers_any': tot(lambda o: o['api']['merged_identifiers']),
        'identifier_status_any': tot(lambda o: o['api']['identifier_status'] is not None),
        'flag_hits': {k: tot(lambda o, k=k: o['flags'][k]) for k in out[0]['flags']},
        'in_469_reports': {o['code']: o['in_469_reports'] for o in out if o['in_469_reports']},
        'party_ppon_copublished_513': {o['code']: o['census513_off_allowlist']['ppons'] for o in out
                                       if o['census513_off_allowlist'] and o['census513_off_allowlist']['ppons']},
        'off_allowlist_multi_holder': {o['code']: o['census513_off_allowlist']['holders'] for o in out
                                       if o['census513_off_allowlist'] and o['census513_off_allowlist']['holders'] != 1},
        'off_allowlist_listed': tot(lambda o: o['census513_off_allowlist'] is not None),
        'census513': {'countryless_parties': plan513['countryless_parties'],
                      'non_gb_parties': plan513['non_gb_parties'],
                      'computed_at': load(os.path.join(SCR, '513-nhs/u5/plan-s4-after-daily.json'))['computed_at']},
        'admin_rows_read': {t: admin[t]['count'] for t in admin},
        'admin_truncated': [t for t in admin if admin[t]['count'] >= admin[t]['limit']],
        'gb_nhs_identifier_verdicts': sum(1 for r in admin['identifier']['rows']
                                          if r['identifier'].startswith('GBNHS')),
        'by_class': {},
        # E3's capped-read denial (`child-names-unread`): NhsBodyCaps::PRODUCTION reads 4,096 mentions and keeps
        # 64 distinct name_norms per holder (canonical.rs:5930-5939).
        'caps': {'mentions_cap': 4096, 'names_cap': 64,
                 'max_mentions': max(o['mentions_read'] for o in out),
                 'max_distinct_names': max(len({(m[3] or '').lower() for m in ments_by.get(o['org_id'], [])})
                                           for o in out)},
    }
    summary['child_names_unread'] = tot(lambda o: o['mentions_read'] >= 4096 or len(
        {(m[3] or '').lower() for m in ments_by.get(o['org_id'], [])}) > 64)
    for o in out:
        c = summary['by_class'].setdefault(o['class'], {'holders': 0, 'mentions': 0, 'impure': 0, 'ppon': 0,
                                                        'withheld': 0})
        c['holders'] += 1
        c['mentions'] += o['mentions_read']
        c['impure'] += o['child_impure']
        c['ppon'] += o['child_carries_ppon']
        c['withheld'] += o['child_withheld']
    json.dump({'computed': '2026-10-12', 'summary': summary, 'holders': out},
              open(os.path.join(HERE, 'm3.json'), 'w'), indent=1, default=list)
    print(json.dumps(summary, indent=1, default=list))


if __name__ == '__main__':
    main()
