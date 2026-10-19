# Builds r8_dest.sql: destination lookups. One name_norm range seek per prefix (org_name_norm is
# Unicode-lowercase + collapsed whitespace + trailing .,; stripped, canonical.rs org_name_norm),
# plus 456's holder row (Sellafield, by PK).
PREFIXES = [
 'abri', 'abbott medical', 'thomas silvey', 'castle point', 'castlepoint', 'tiaa',
 'the academy for healthcare science', 'academy for healthcare science', 'cognition education',
 'london school of economics', 'nla media', 'ppl', 'rcd lock', 'intuitive', 'fire service college',
 'red kite vet', 'cambridge university hospitals', 'hologic', 'the centre for systemic social work',
 'centre for systemic social work', 'renishaw neuro', 'xiel', 'tech against terrorism', 'ermc',
 'waterways ireland', 'collectionhq', 'collection hq', 'betsi cadwaladr', 'vohkus', 'bolton nhs',
 'hartlepool borough council', 'alder hey', 'gama healthcare', 'uttlesford district council',
 'virgin media', 'compendium', 'm group services', 'rss international', 'taylor wimpey',
 'the law society', 'law society', 'swansea bay university health board', 'schneider electric',
 # F? (parked anyway): evidence only
 'everglow', 'elv solutions', 'lifestone', 'gemini surgical', 'diagnose uk', 'mechplant',
 'owen environmental', 'johnston gilpin',
]
def upper(p):
    return p[:-1] + chr(ord(p[-1]) + 1)
arms = [f"SELECT 'id:5718658' AS q, id, country, identifier, provisional, name, name_norm FROM organizations WHERE id = 5718658"]
for p in PREFIXES:
    assert "'" not in p
    arms.append(f"SELECT 'n:{p}' AS q, id, country, identifier, provisional, name, name_norm FROM organizations "
                f"WHERE name_norm >= '{p}' AND name_norm < '{upper(p)}'")
open('r8_dest.sql', 'w').write('\nUNION ALL\n'.join(arms) + '\n')
print(len(arms), 'arms')
