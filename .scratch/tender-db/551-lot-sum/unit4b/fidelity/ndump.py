"""Compact dump of a notice's /content: every section's kind/id/parent and its non-text values (texts truncated)."""
import json, sys, re
import os
RAW = os.environ.get('M551', '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551') + '/fid/raw'
full = '--full' in sys.argv
skip_kinds = {'Organization', 'CompanyLegalEntity', 'ContractingParty', 'ServiceProviderParty', 'LotAwardCriterion',
              'LotPlacePerformance', 'LotAdditionalClassification', 'ReviewBody', 'Address', 'Contact'} if not full else set()
for nid in [a for a in sys.argv[1:] if not a.startswith('--')]:
    n = json.load(open(f'{RAW}/notices/{nid}.json'))
    print('######## NOTICE', nid)
    for s in n['sections']:
        if s.get('kind') in skip_kinds:
            continue
        vals = []
        KEEP = re.compile(r'^(TED-(LOT_NO|LOT_NUMBER|ITEM|AWARDED_TO_GROUP|NO_AWARDED_TO_GROUP|ADDRESS_CONTRACTOR|FORM|AWARDED_CONTRACT|NO_AWARDED_CONTRACT|PROCUREMENT_UNSUCCESSFUL|PROCUREMENT_DISCONTINUED|CONTRACT_NO|NB_TENDERS_RECEIVED|DATE_PUB|LEGAL_BASIS|NOTICE_TYPE|TYPE_CONTRACT|ADDRESS_CONTRACTING_BODY|INFO_ADD|CATEGORY|DIRECTIVE|OFFICIALNAME|NATIONALID)$|BT-|OPT-3|OPP-070|DE1-)')
        for v in s['values']:
            if v.get('type') != 'amount' and not KEEP.match(v['field_id']) and not full:
                continue
            t = v.get('type')
            if t == 'amount':
                vals.append(f"{v['field_id']}={v.get('currency')} {v['cents']/100:,.2f}" if v.get('cents') is not None else f"{v['field_id']}=AMT?{v}")
            elif t == 'text':
                if v['field_id'] in ('TED-TITLE', 'BT-21-Lot', 'BT-21-Procedure', 'TED-OFFICIALNAME', 'BT-500-Organization-Company') or full:
                    vals.append(f"{v['field_id']}='{(v.get('value') or '')[:60]}'")
            elif t == 'code':
                vals.append(f"{v['field_id']}:{v.get('code')}")
            else:
                vals.append(f"{v['field_id']}={v.get('value')}")
        print(f"-- {s.get('kind')} [{s.get('section_id')}] <{s.get('parent_section_id')}>: " + ' | '.join(vals))
