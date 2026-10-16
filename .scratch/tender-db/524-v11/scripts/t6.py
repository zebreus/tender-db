exec(open('/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/v11/judge/t1.py').read().split("def A_hold")[0])
import textwrap
for x,e,f in sorted(ship,key=lambda s:s[2]['ta']):
    if f['ta']>=365 and not f['sh']:
        print('==',x['plan'],x['org_id'],x['final'],'ta',f['ta'],'ha',f['ha'],'H',e['head'],'2nd',e['second'],'T',e['top'],'C',e['clusters'])
        print('  ',e['old_name'][:90],'->',e['new_name'][:90])
        print('  ',e['spellings'])
        print(textwrap.indent(textwrap.fill(x['why'][:700],160),'   '))
