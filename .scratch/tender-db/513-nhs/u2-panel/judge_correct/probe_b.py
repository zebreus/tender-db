import sys, os
sys.path.insert(0, '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/panel/designB')
sys.argv = [sys.argv[0], '/home/user/tender-db/.scratch/tender-db/513-nhs/u1']
import b4
import gate_b2 as G2
cases = [
 ('University of Cambridge', 'RGT'),
 ('Cambridge University', 'RGT'),
 ('Sheffield City Council', 'RHQ'),
 ('The University of Sheffield', 'RHQ'),
 ('Newcastle University', 'RTD'),
 ('Newcastle City Council', 'RTD'),
 ('Salisbury District Hospital', 'RNZ'),
 ('Northumbria University', 'RTF'),
 ('Northumberland County Council', 'RTF'),
 ('Christie Pathology Partnership', 'RBV'),
 ('Hull City Council', 'RWA'),
 ('Hull and East Yorkshire Hospitals NHS Trust', 'RWA'),
 ('University of Leicester', 'RWE'),
 ('Leicester City Council', 'RWE'),
 ('Berkshire West ICB', 'RWX'),
 ('Royal Cornwall Hospitals NHS Trust', 'REF'),
 ('Cornwall Council', 'REF'),
 ('Cornwall Partnership NHS Foundation Trust', 'REF'),
 ('Buckinghamshire Council', 'RXQ'),
 ('South Tees Hospitals', 'RTR'),
 ('Middlesbrough Council', 'RTR'),
 ('Oxford University Hospitals NHS Foundation Trust','RTH'),
 ('University of Oxford','RTH'),
 ('Oxford Health NHS Foundation Trust','RTH'),
 ('Oxfordshire County Council','RTH'),
 ('NHS Shared Business Services', 'RTH'),
 ('NHS Supply Chain', 'RTH'),
]
for name, code in cases:
    tc, why = b4.admit4(name, code)
    print(f'{code} {name!r:55} -> {"ADMIT "+tc if tc else "refuse"} ({why}) split={G2.split(name)[0]}')
print('RXQ names', G2.code_names('RXQ'))
print('RHQ names', G2.code_names('RHQ')[:3])
