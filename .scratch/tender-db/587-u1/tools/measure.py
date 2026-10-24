"""measure.py -- 587 U1 measure.json from census 2781, the 19 name_prefix listings, s01-s04 and g20."""
import json, os, sys, collections, re
sys.path.insert(0, os.path.dirname(__file__))
from classify import classify, fold, PHRASES
HERE = '/home/user/tender-db/.scratch/tender-db/587-u1'
R = lambda p: json.load(open(os.path.join(HERE, p)))
def rows(name):
    d = R(f'raw/sql/{name}.out')
    assert 'rows' in d and 'columns' in d and not d.get('truncated'), name
    return [dict(zip(d['columns'], r)) for r in d['rows']]

names = R('raw/names.json')
census_body = json.loads(R('census_2781.json')['body'])
crow = names['census_rows']
L = names['listing_orgs']
orgsets = R('sql/orgs.json')
NEXT15 = R('sql/orgs_next15.json')['next15']
ORDER = orgsets['top15'] + orgsets['extra'] + orgsets['control'] + NEXT15
by_id = {o['id']: o for o in L}
for r in crow:
    by_id.setdefault(r['org_id'], {'id': r['org_id'], 'name': r['name'], 'mentions': r['mentions'], 'country': r['country'],
                                   'identifier': r['identifier'], 'cls': r['cls']})

def bucket(o):
    if o.get('identifier'):
        return 'identified'
    if not o.get('country'):
        return 'no_identifier_null_country'
    return 'no_identifier_with_country'

# (1) classification -------------------------------------------------------------------------------------------
def tally(items, key, mkey='mentions'):
    t = collections.defaultdict(lambda: collections.defaultdict(lambda: {'orgs': 0, 'mentions': 0}))
    for o in items:
        for k in key(o):
            t[k][o['cls']]['orgs'] += 1
            t[k][o['cls']]['mentions'] += o[mkey]
    return {k: dict(sorted(v.items())) for k, v in sorted(t.items())}

census_cls = tally(crow, lambda r: ['all'])['all']
census_by_phrase = tally(crow, lambda r: [r['phrases'][0]])
census_winner = [{'org_id': r['org_id'], 'name': r['name'], 'winner_tenders': r['winner_tenders'], 'cls': r['cls']}
                 for r in crow if r['winner_tenders']]
census_bare = [{'org_id': r['org_id'], 'name': r['name'], 'mentions': r['mentions'], 'identifier': r['identifier'],
                'country': r['country'], 'roles': r['roles']} for r in crow if r['cls'].startswith('BARE')]
# Prefix coverage check: every bare census row must appear in a listing.
coverage = {r['org_id']: (r['org_id'] in {o['id'] for o in L}) for r in crow if r['cls'].startswith('BARE')}

lst_cls = tally(L, lambda o: ['all'])['all']
lst_bucket = tally(L, lambda o: [bucket(o)])
lst_by_phrase = tally(L, lambda o: o['phrases'] or ['(no census phrase)'])
bare = [o for o in L if o['cls'].startswith('BARE')]
bare_hist = collections.Counter('1' if o['mentions'] <= 1 else '2-9' if o['mentions'] < 10 else '10-99' if o['mentions'] < 100 else '100+'
                                for o in bare)
bare_hist_m = collections.Counter()
for o in bare:
    bare_hist_m['1' if o['mentions'] <= 1 else '2-9' if o['mentions'] < 10 else '10-99' if o['mentions'] < 100 else '100+'] += o['mentions']
# Estimate vs by_phrase: share of each phrase's census orgs that are bare (listing count / census count).
est = {}
for p in PHRASES:
    tot = census_body['by_phrase'].get(p, 0)
    b = [o for o in bare if p in o['phrases']]
    est[p] = {'census_orgs': tot, 'bare_orgs': len(b), 'bare_mentions': sum(o['mentions'] for o in b),
              'bare_share_of_orgs': round(len(b) / tot, 3) if tot else None,
              'bare_name_keyed_with_country': sum(1 for o in b if bucket(o) == 'no_identifier_with_country'),
              'bare_name_keyed_mentions': sum(o['mentions'] for o in b if bucket(o) == 'no_identifier_with_country')}
bare_census_phrase = [o for o in bare if o['phrases']]
bare_no_phrase = [o for o in bare if not o['phrases']]
top_bare_names = collections.Counter(); top_bare_n = collections.Counter()
for o in bare:
    top_bare_names[o['folded']] += o['mentions']; top_bare_n[o['folded']] += 1
hubs_by_name_country = sorted([o for o in bare if bucket(o) == 'no_identifier_with_country'], key=lambda o: -o['mentions'])

# (2) buyers beside --------------------------------------------------------------------------------------------
s02 = rows('s02_buyers') + rows('s05_buyers_next15'); s01 = rows('s01a_roles') + rows('s01b_roles') + rows('s06_roles_next15'); s03 = rows('s03_mentions'); s04 = rows('s04_selfbuyer')
per = {}
for x in ORDER:
    o = by_id[x]
    b = [r for r in s02 if r['org'] == x and r['brole'] != 'SAMPLED']
    sampled = [r['tenders'] for r in s02 if r['org'] == x and r['brole'] == 'SAMPLED'][0]
    self_t = sum(r['tenders'] for r in b if r['buyer'] == x)
    other = [r for r in b if r['buyer'] != x]
    by_name = collections.Counter()
    for r in other:
        by_name[fold(r['bname'] or '')] += r['tenders']
    mass = sum(by_name.values())
    top = by_name.most_common(1)[0] if by_name else (None, 0)
    ment = {r['sec']: r['n'] for r in s03 if r['org'] == x}
    roles = collections.defaultdict(lambda: collections.Counter())
    for r in s01:
        if r['org'] == x:
            roles[r['sec']][r['role']] += r['tenders']
    group = 'control' if x in orgsets['control'] else 'extra' if x in orgsets['extra'] else 'next15' if x in NEXT15 else 'top15'
    per[x] = {'org_id': x, 'group': group, 'name': o['name'], 'cls': o['cls'], 'country': o.get('country'),
              'identifier': o.get('identifier'), 'mentions': o['mentions'], 'mentions_by_section_prefix': ment,
              'sampled_served_tenders': sampled, 'self_buyer_tenders': self_t, 'buyer_org_ids': len({r['buyer'] for r in other}),
              'buyer_names_folded': len(by_name), 'buyer_tender_mass': mass, 'top_buyer': top[0], 'top_buyer_tenders': top[1],
              'top_buyer_share': round(top[1] / mass, 3) if mass else None,
              'buyers_top8': by_name.most_common(8),
              'roles_tenders_by_path': {k: dict(v.most_common()) for k, v in roles.items()}}

def verdict(p):
    if p['buyer_names_folded'] == 0:
        return 'SELF-BUYER (label is the buyer of record; buyer ids cannot show pooling)'
    if p['buyer_names_folded'] == 1 or (p['top_buyer_share'] or 0) >= 0.97:
        return 'N (one buyer)'
    if (p['top_buyer_share'] or 0) >= 0.8:
        return 'DOMINANT+TAIL (one buyer >= 80%, others pooled)'
    return 'W HUB (many unrelated buyers)'
for p in per.values():
    p['verdict'] = verdict(p)
# Hand reads where the folded-name rule over-counts one administration (read from buyers_top8).
OVERRIDE = {
    1165758: 'N (one administration: Ville de Strasbourg + Communaute urbaine de Strasbourg, 100/100 tenders each - its shared service)',
    22506338: 'N-ish (Ardenne Metropole 77 + Commune de Charleville-Mezieres 24 + 3 small: the agglomeration shared purchasing service; identified FRF2108000)',
    1606217: 'N (control: four spellings of Hospital Universitario Ramon y Cajal / SERMAS)',
    1182792: 'N (control: ADIF and ADIF Alta Velocidad, one group)',
    1244992: 'DOMINANT+TAIL (ISPPC Charleroi 188/200 in two spellings; Humani 11; Tauron Wytwarzanie 1)',
    19553: 'W HUB as buyer of record (DOE notices: g20 titles name Wuppertal, Leverkusen, Schwerte, Aulendorf, Rheinzabern, Bad Segeberg, Klein Nordende-Lieth, Reepsholt)',
    1214290: 'W HUB (Generalitat Valenciana consellerias x5 + Universitat Jaume I 35 + others)',
    1193769: 'DOMINANT+TAIL (TARC Andalucia 117/135 as buyer, 4 Andalusian bodies)',
}
for k, v in OVERRIDE.items():
    per[k]['verdict_hand'] = v

# DOE titles (g20): places named.
g20 = R('raw/get/g20_t19553.json')['items']
places = collections.Counter()
for t in g20:
    ti = t.get('title') or ''
    m = None
    for pl in ['Wuppertal', 'Leverkusen', 'Schwerte', 'Aulendorf', 'Rheinzabern', 'Bad Segeberg', 'Klein Nordende', 'Reepsholt',
               'Priemelsfehn']:
        if pl.lower() in ti.lower():
            m = pl; break
    if m is None and re.match(r'^[BLFUZ]-0\d{3}-2\d', ti.strip()):
        m = 'Wuppertal (B/L/F/U-nnnn-yy file numbers)'
    if m is None and re.match(r'^2023-0\d{3}', ti.strip()):
        m = 'Leverkusen (2023-0nnn file numbers)'
    places[m or '(no place in title)'] += 1
s04_sum = {x: {'sources': [(r['oname'], r['ocountry'], r['tenders']) for r in s04 if r['org'] == x and r['role'] == 'SOURCE'],
               'other_parties': [(r['role'], r['other'], r['oname'], r['tenders']) for r in s04 if r['org'] == x and r['role'] != 'SOURCE']}
           for x in (19553, 2997642, 1416875)}

# (3) roles: touchpoint vs ORG path over the 22 bare orgs measured ----------------------------------------------
bare_measured = [x for x in ORDER if x not in orgsets['control']]
path_ment = collections.Counter(); path_roles = collections.defaultdict(collections.Counter)
for r in s03:
    if r['org'] in bare_measured:
        path_ment[r['sec']] += r['n']
for r in s01:
    if r['org'] in bare_measured:
        path_roles[r['sec']][r['role']] += r['tenders']
roles_by_org_path = collections.defaultdict(collections.Counter)
for r in s01:
    if r['org'] in ORDER:
        roles_by_org_path[r['org']][r['sec']] += r['tenders']
LEGACY = {'appeal-information', 'further-information', 'tender-receipt', 'specifications-provider', 'review-body', 'mediation-body',
          'buyer', 'purchasing-body', 'winner'}
role_family = collections.Counter()
for sec, c in path_roles.items():
    for role, n in c.items():
        fam = 'TED R2 address/role block (ORG path)' if role in LEGACY else (
            'eForms touchpoint (TPO path)' if sec == 'TPO' else 'eForms ORG section (ORG path)')
        role_family[fam] += n
census_roles = census_body['roles']
census_role_family = collections.Counter()
for role, n in census_roles.items():
    census_role_family['TED R2 legacy roles' if role in LEGACY or role == 'bid:tenderer' else 'eForms roles'] += n
tpo_orgs = sorted([(x, per[x]['name'], per[x]['mentions_by_section_prefix'].get('TPO', 0), per[x]['mentions'])
                   for x in bare_measured if per[x]['mentions_by_section_prefix'].get('TPO')], key=lambda t: -t[2])

def vclass(p):
    v = p.get('verdict_hand') or p['verdict']
    return 'W' if v.startswith('W') else 'DOMINANT+TAIL' if v.startswith('DOMINANT') else 'N' if v.startswith('N') else 'SELF'
vt = collections.defaultdict(lambda: collections.defaultdict(lambda: {'orgs': 0, 'mentions': 0, 'ids': []}))
for x in ORDER:
    p = per[x]; c = vclass(p); p['verdict_class'] = c
    for g in ([p['group']] if p['group'] == 'control' else [p['group'], 'all_bare']):
        vt[g][c]['orgs'] += 1; vt[g][c]['mentions'] += p['mentions']; vt[g][c]['ids'].append(x)
verdict_tally = {g: dict(v) for g, v in vt.items()}
bare_total_m = sum(o['mentions'] for o in L if o['cls'].startswith('BARE'))
measured_m = sum(per[x]['mentions'] for x in ORDER if per[x]['group'] != 'control')
verdict_tally['coverage'] = {'bare_orgs_measured': sum(1 for x in ORDER if per[x]['group'] != 'control'),
                             'bare_mentions_measured': measured_m, 'bare_mentions_in_listings': bare_total_m,
                             'share': round(measured_m / bare_total_m, 3)}
out = {
    'issue': 587, 'unit': 1, 'generated_from': ['census_2781.json', 'raw/get/g01..g19 (name_prefix listings)', 'raw/get/g20_t19553.json',
                                               'raw/sql/s01a_roles, s01b_roles, s02_buyers, s03_mentions, s04_selfbuyer'],
    'budget_used': {'sql_statements': 7, 'sql_budget': 12, 'gets': 20, 'get_budget': 20},
    'census_listing_order': 'rows = hits.into_values().take(cap) over a BTreeMap keyed by org_id (canonical.rs name_phrase_census): '
                            'the 500 LOWEST org ids (884..79376), not the largest; 491 of 500 are vergabestelle rows',
    'census_rows': {'n': len(crow), 'by_class': census_cls, 'by_first_phrase': census_by_phrase, 'bare_rows': census_bare,
                    'winner_rows': census_winner, 'bare_rows_found_in_prefix_listings': coverage},
    'prefix_listings': {'orgs': len(L), 'truncated_listings': ['g01_dcp (1000 rows, more=true; the bare exact name sorts first)'],
                        'by_class': lst_cls, 'by_key_bucket': lst_bucket, 'by_census_phrase': lst_by_phrase,
                        'bare_mention_histogram_orgs': dict(bare_hist), 'bare_mention_histogram_mentions': dict(bare_hist_m),
                        'bare_with_census_phrase': {'orgs': len(bare_census_phrase), 'mentions': sum(o['mentions'] for o in bare_census_phrase)},
                        'bare_without_census_phrase': {'orgs': len(bare_no_phrase), 'mentions': sum(o['mentions'] for o in bare_no_phrase),
                                                       'names': collections.Counter(o['folded'] for o in bare_no_phrase).most_common(8)},
                        'bare_estimate_per_phrase': est,
                        'top_bare_folded_names': [(k, v, top_bare_n[k]) for k, v in top_bare_names.most_common(30)],
                        'bare_name_keyed_rows_with_country_top30': [(o['id'], o['name'], o['country'], o['mentions']) for o in hubs_by_name_country[:30]],
                        'bare_name_keyed_rows_with_country_total': {'orgs': len(hubs_by_name_country), 'mentions': sum(o['mentions'] for o in hubs_by_name_country)}},
    'buyers_beside': [per[x] for x in ORDER],
    'self_buyer_lens_s04': s04_sum, 'doe_19553_titles_g20': {'tenders_read': len(g20), 'places': places.most_common()},
    'verdict_tally': verdict_tally,
    'roles': {'bare_measured_orgs': len(bare_measured), 'mentions_by_section_prefix_22_orgs_s03': dict(path_ment),
              'party_tenders_by_path_per_org': {str(k): dict(v) for k, v in roles_by_org_path.items()},
              'tenders_by_path_and_role': {k: dict(v.most_common()) for k, v in path_roles.items()},
              'tenders_by_role_family': dict(role_family), 'tpo_path_orgs': tpo_orgs,
              'census_all_phrase_orgs_roles_family': dict(census_role_family)},
}
json.dump(out, open(os.path.join(HERE, 'measure.json'), 'w'), indent=1, ensure_ascii=False)
print(json.dumps({k: out[k] for k in ('census_rows',)}, ensure_ascii=False)[:1500])
print(json.dumps(out['prefix_listings']['by_class'])); print(json.dumps(out['prefix_listings']['by_key_bucket']))
print(out['prefix_listings']['bare_mention_histogram_orgs'], out['prefix_listings']['bare_mention_histogram_mentions'])
print(out['prefix_listings']['bare_with_census_phrase'], out['prefix_listings']['bare_without_census_phrase'])
print(json.dumps(out['prefix_listings']['bare_name_keyed_rows_with_country_total']))
for p, v in est.items(): print('  ', p, v)
for x in ORDER:
    p = per[x]
    print(x, p['group'], p['cls'], p['mentions'], p['sampled_served_tenders'], p['buyer_names_folded'], p['top_buyer_share'], p['self_buyer_tenders'], '|', p.get('verdict_hand') or p['verdict'], '|', p['name'][:50])
print(json.dumps(out['roles'], ensure_ascii=False, indent=0)[:3000])
print(places.most_common())
