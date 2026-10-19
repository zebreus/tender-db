# usage: python3 -I dates.py <bodies/c<id>.json>... — every date-typed value (and any field id containing Date) in a notice's parsed content
import json, sys
for p in sys.argv[1:]:
    j = json.load(open(p))
    print('==', j.get('notice_id'))
    for s in j.get('sections', []):
        for v in s.get('values', []):
            if v.get('type') == 'date' or 'Date' in v.get('field_id', ''):
                print('  ', s['section_id'], v['field_id'], {k: v[k] for k in v if k not in ('field_id', 'ordinal')})
