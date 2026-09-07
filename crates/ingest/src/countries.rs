//! ISO 3166-1 country tables, GENERATED from the `iso-codes` package's
//! `iso_3166-1.json` (public domain) on 2026-08-30 — not typed by hand and
//! not recalled from memory. Regenerate rather than edit: a hand-patched
//! entry is how issue 319 happened.
//!
//! Issue 319: the corpus stores alpha-2 in `organizations.country`, but TED
//! and its national feeds publish alpha-3 and, occasionally, a country NAME.
//! The previous fold covered ~50 alpha-3 codes chosen by hand — the EU-27
//! plus "common third countries" — so the other 151 passed through unchanged
//! and split their countries in two (`GL` beside `GRL`, 12 rows).
//!
//! **Never fold by truncating an alpha-3 to its first two letters.** SEN is
//! Senegal and SE is Sweden; BEN is Benin and BE is Belgium. The prefix looks
//! like a mapping and is not one.

/// ISO 3166-1 alpha-3 → alpha-2, all 249 assignments.
pub const ALPHA3_TO_ALPHA2: &[(&str, &str)] = &[
    ("ABW", "AW"), ("AFG", "AF"), ("AGO", "AO"), ("AIA", "AI"),
    ("ALA", "AX"), ("ALB", "AL"), ("AND", "AD"), ("ARE", "AE"),
    ("ARG", "AR"), ("ARM", "AM"), ("ASM", "AS"), ("ATA", "AQ"),
    ("ATF", "TF"), ("ATG", "AG"), ("AUS", "AU"), ("AUT", "AT"),
    ("AZE", "AZ"), ("BDI", "BI"), ("BEL", "BE"), ("BEN", "BJ"),
    ("BES", "BQ"), ("BFA", "BF"), ("BGD", "BD"), ("BGR", "BG"),
    ("BHR", "BH"), ("BHS", "BS"), ("BIH", "BA"), ("BLM", "BL"),
    ("BLR", "BY"), ("BLZ", "BZ"), ("BMU", "BM"), ("BOL", "BO"),
    ("BRA", "BR"), ("BRB", "BB"), ("BRN", "BN"), ("BTN", "BT"),
    ("BVT", "BV"), ("BWA", "BW"), ("CAF", "CF"), ("CAN", "CA"),
    ("CCK", "CC"), ("CHE", "CH"), ("CHL", "CL"), ("CHN", "CN"),
    ("CIV", "CI"), ("CMR", "CM"), ("COD", "CD"), ("COG", "CG"),
    ("COK", "CK"), ("COL", "CO"), ("COM", "KM"), ("CPV", "CV"),
    ("CRI", "CR"), ("CUB", "CU"), ("CUW", "CW"), ("CXR", "CX"),
    ("CYM", "KY"), ("CYP", "CY"), ("CZE", "CZ"), ("DEU", "DE"),
    ("DJI", "DJ"), ("DMA", "DM"), ("DNK", "DK"), ("DOM", "DO"),
    ("DZA", "DZ"), ("ECU", "EC"), ("EGY", "EG"), ("ERI", "ER"),
    ("ESH", "EH"), ("ESP", "ES"), ("EST", "EE"), ("ETH", "ET"),
    ("FIN", "FI"), ("FJI", "FJ"), ("FLK", "FK"), ("FRA", "FR"),
    ("FRO", "FO"), ("FSM", "FM"), ("GAB", "GA"), ("GBR", "GB"),
    ("GEO", "GE"), ("GGY", "GG"), ("GHA", "GH"), ("GIB", "GI"),
    ("GIN", "GN"), ("GLP", "GP"), ("GMB", "GM"), ("GNB", "GW"),
    ("GNQ", "GQ"), ("GRC", "GR"), ("GRD", "GD"), ("GRL", "GL"),
    ("GTM", "GT"), ("GUF", "GF"), ("GUM", "GU"), ("GUY", "GY"),
    ("HKG", "HK"), ("HMD", "HM"), ("HND", "HN"), ("HRV", "HR"),
    ("HTI", "HT"), ("HUN", "HU"), ("IDN", "ID"), ("IMN", "IM"),
    ("IND", "IN"), ("IOT", "IO"), ("IRL", "IE"), ("IRN", "IR"),
    ("IRQ", "IQ"), ("ISL", "IS"), ("ISR", "IL"), ("ITA", "IT"),
    ("JAM", "JM"), ("JEY", "JE"), ("JOR", "JO"), ("JPN", "JP"),
    ("KAZ", "KZ"), ("KEN", "KE"), ("KGZ", "KG"), ("KHM", "KH"),
    ("KIR", "KI"), ("KNA", "KN"), ("KOR", "KR"), ("KWT", "KW"),
    ("LAO", "LA"), ("LBN", "LB"), ("LBR", "LR"), ("LBY", "LY"),
    ("LCA", "LC"), ("LIE", "LI"), ("LKA", "LK"), ("LSO", "LS"),
    ("LTU", "LT"), ("LUX", "LU"), ("LVA", "LV"), ("MAC", "MO"),
    ("MAF", "MF"), ("MAR", "MA"), ("MCO", "MC"), ("MDA", "MD"),
    ("MDG", "MG"), ("MDV", "MV"), ("MEX", "MX"), ("MHL", "MH"),
    ("MKD", "MK"), ("MLI", "ML"), ("MLT", "MT"), ("MMR", "MM"),
    ("MNE", "ME"), ("MNG", "MN"), ("MNP", "MP"), ("MOZ", "MZ"),
    ("MRT", "MR"), ("MSR", "MS"), ("MTQ", "MQ"), ("MUS", "MU"),
    ("MWI", "MW"), ("MYS", "MY"), ("MYT", "YT"), ("NAM", "NA"),
    ("NCL", "NC"), ("NER", "NE"), ("NFK", "NF"), ("NGA", "NG"),
    ("NIC", "NI"), ("NIU", "NU"), ("NLD", "NL"), ("NOR", "NO"),
    ("NPL", "NP"), ("NRU", "NR"), ("NZL", "NZ"), ("OMN", "OM"),
    ("PAK", "PK"), ("PAN", "PA"), ("PCN", "PN"), ("PER", "PE"),
    ("PHL", "PH"), ("PLW", "PW"), ("PNG", "PG"), ("POL", "PL"),
    ("PRI", "PR"), ("PRK", "KP"), ("PRT", "PT"), ("PRY", "PY"),
    ("PSE", "PS"), ("PYF", "PF"), ("QAT", "QA"), ("REU", "RE"),
    ("ROU", "RO"), ("RUS", "RU"), ("RWA", "RW"), ("SAU", "SA"),
    ("SDN", "SD"), ("SEN", "SN"), ("SGP", "SG"), ("SGS", "GS"),
    ("SHN", "SH"), ("SJM", "SJ"), ("SLB", "SB"), ("SLE", "SL"),
    ("SLV", "SV"), ("SMR", "SM"), ("SOM", "SO"), ("SPM", "PM"),
    ("SRB", "RS"), ("SSD", "SS"), ("STP", "ST"), ("SUR", "SR"),
    ("SVK", "SK"), ("SVN", "SI"), ("SWE", "SE"), ("SWZ", "SZ"),
    ("SXM", "SX"), ("SYC", "SC"), ("SYR", "SY"), ("TCA", "TC"),
    ("TCD", "TD"), ("TGO", "TG"), ("THA", "TH"), ("TJK", "TJ"),
    ("TKL", "TK"), ("TKM", "TM"), ("TLS", "TL"), ("TON", "TO"),
    ("TTO", "TT"), ("TUN", "TN"), ("TUR", "TR"), ("TUV", "TV"),
    ("TWN", "TW"), ("TZA", "TZ"), ("UGA", "UG"), ("UKR", "UA"),
    ("UMI", "UM"), ("URY", "UY"), ("USA", "US"), ("UZB", "UZ"),
    ("VAT", "VA"), ("VCT", "VC"), ("VEN", "VE"), ("VGB", "VG"),
    ("VIR", "VI"), ("VNM", "VN"), ("VUT", "VU"), ("WLF", "WF"),
    ("WSM", "WS"), ("YEM", "YE"), ("ZAF", "ZA"), ("ZMB", "ZM"),
    ("ZWE", "ZW"),
];

/// ISO 3166-1 country NAMES (short, official and common forms, uppercased)
/// → alpha-2. The corpus holds one row spelled `LUXEMBOURG`; the table is
/// generated whole rather than special-cased, because the next feed to
/// publish a name will not pick that one.
pub const NAME_TO_ALPHA2: &[(&str, &str)] = &[
    ("AFGHANISTAN", "AF"), ("ALBANIA", "AL"),
    ("ALGERIA", "DZ"), ("AMERICAN SAMOA", "AS"),
    ("ANDORRA", "AD"), ("ANGOLA", "AO"),
    ("ANGUILLA", "AI"), ("ANTARCTICA", "AQ"),
    ("ANTIGUA AND BARBUDA", "AG"), ("ARAB REPUBLIC OF EGYPT", "EG"),
    ("ARGENTINA", "AR"), ("ARGENTINE REPUBLIC", "AR"),
    ("ARMENIA", "AM"), ("ARUBA", "AW"),
    ("AUSTRALIA", "AU"), ("AUSTRIA", "AT"),
    ("AZERBAIJAN", "AZ"), ("BAHAMAS", "BS"),
    ("BAHRAIN", "BH"), ("BANGLADESH", "BD"),
    ("BARBADOS", "BB"), ("BELARUS", "BY"),
    ("BELGIUM", "BE"), ("BELIZE", "BZ"),
    ("BENIN", "BJ"), ("BERMUDA", "BM"),
    ("BHUTAN", "BT"), ("BOLIVARIAN REPUBLIC OF VENEZUELA", "VE"),
    ("BOLIVIA", "BO"), ("BOLIVIA, PLURINATIONAL STATE OF", "BO"),
    ("BONAIRE, SINT EUSTATIUS AND SABA", "BQ"), ("BOSNIA AND HERZEGOVINA", "BA"),
    ("BOTSWANA", "BW"), ("BOUVET ISLAND", "BV"),
    ("BRAZIL", "BR"), ("BRITISH INDIAN OCEAN TERRITORY", "IO"),
    ("BRITISH VIRGIN ISLANDS", "VG"), ("BRUNEI DARUSSALAM", "BN"),
    ("BULGARIA", "BG"), ("BURKINA FASO", "BF"),
    ("BURUNDI", "BI"), ("CABO VERDE", "CV"),
    ("CAMBODIA", "KH"), ("CAMEROON", "CM"),
    ("CANADA", "CA"), ("CAYMAN ISLANDS", "KY"),
    ("CENTRAL AFRICAN REPUBLIC", "CF"), ("CHAD", "TD"),
    ("CHILE", "CL"), ("CHINA", "CN"),
    ("CHRISTMAS ISLAND", "CX"), ("COCOS (KEELING) ISLANDS", "CC"),
    ("COLOMBIA", "CO"), ("COMMONWEALTH OF DOMINICA", "DM"),
    ("COMMONWEALTH OF THE BAHAMAS", "BS"), ("COMMONWEALTH OF THE NORTHERN MARIANA ISLANDS", "MP"),
    ("COMOROS", "KM"), ("CONGO", "CG"),
    ("CONGO, THE DEMOCRATIC REPUBLIC OF THE", "CD"), ("COOK ISLANDS", "CK"),
    ("COSTA RICA", "CR"), ("CROATIA", "HR"),
    ("CUBA", "CU"), ("CURAÇAO", "CW"),
    ("CYPRUS", "CY"), ("CZECH REPUBLIC", "CZ"),
    ("CZECHIA", "CZ"), ("CÔTE D'IVOIRE", "CI"),
    ("DEMOCRATIC PEOPLE'S REPUBLIC OF KOREA", "KP"), ("DEMOCRATIC REPUBLIC OF SAO TOME AND PRINCIPE", "ST"),
    ("DEMOCRATIC REPUBLIC OF TIMOR-LESTE", "TL"), ("DEMOCRATIC SOCIALIST REPUBLIC OF SRI LANKA", "LK"),
    ("DENMARK", "DK"), ("DJIBOUTI", "DJ"),
    ("DOMINICA", "DM"), ("DOMINICAN REPUBLIC", "DO"),
    ("EASTERN REPUBLIC OF URUGUAY", "UY"), ("ECUADOR", "EC"),
    ("EGYPT", "EG"), ("EL SALVADOR", "SV"),
    ("EQUATORIAL GUINEA", "GQ"), ("ERITREA", "ER"),
    ("ESTONIA", "EE"), ("ESWATINI", "SZ"),
    ("ETHIOPIA", "ET"), ("FALKLAND ISLANDS (MALVINAS)", "FK"),
    ("FAROE ISLANDS", "FO"), ("FEDERAL DEMOCRATIC REPUBLIC OF ETHIOPIA", "ET"),
    ("FEDERAL DEMOCRATIC REPUBLIC OF NEPAL", "NP"), ("FEDERAL REPUBLIC OF GERMANY", "DE"),
    ("FEDERAL REPUBLIC OF NIGERIA", "NG"), ("FEDERAL REPUBLIC OF SOMALIA", "SO"),
    ("FEDERATED STATES OF MICRONESIA", "FM"), ("FEDERATIVE REPUBLIC OF BRAZIL", "BR"),
    ("FIJI", "FJ"), ("FINLAND", "FI"),
    ("FRANCE", "FR"), ("FRENCH GUIANA", "GF"),
    ("FRENCH POLYNESIA", "PF"), ("FRENCH REPUBLIC", "FR"),
    ("FRENCH SOUTHERN TERRITORIES", "TF"), ("GABON", "GA"),
    ("GABONESE REPUBLIC", "GA"), ("GAMBIA", "GM"),
    ("GEORGIA", "GE"), ("GERMANY", "DE"),
    ("GHANA", "GH"), ("GIBRALTAR", "GI"),
    ("GRAND DUCHY OF LUXEMBOURG", "LU"), ("GREECE", "GR"),
    ("GREENLAND", "GL"), ("GRENADA", "GD"),
    ("GUADELOUPE", "GP"), ("GUAM", "GU"),
    ("GUATEMALA", "GT"), ("GUERNSEY", "GG"),
    ("GUINEA", "GN"), ("GUINEA-BISSAU", "GW"),
    ("GUYANA", "GY"), ("HAITI", "HT"),
    ("HASHEMITE KINGDOM OF JORDAN", "JO"), ("HEARD ISLAND AND MCDONALD ISLANDS", "HM"),
    ("HELLENIC REPUBLIC", "GR"), ("HOLY SEE (VATICAN CITY STATE)", "VA"),
    ("HONDURAS", "HN"), ("HONG KONG", "HK"),
    ("HONG KONG SPECIAL ADMINISTRATIVE REGION OF CHINA", "HK"), ("HUNGARY", "HU"),
    ("ICELAND", "IS"), ("INDEPENDENT STATE OF PAPUA NEW GUINEA", "PG"),
    ("INDEPENDENT STATE OF SAMOA", "WS"), ("INDIA", "IN"),
    ("INDONESIA", "ID"), ("IRAN", "IR"),
    ("IRAN, ISLAMIC REPUBLIC OF", "IR"), ("IRAQ", "IQ"),
    ("IRELAND", "IE"), ("ISLAMIC REPUBLIC OF AFGHANISTAN", "AF"),
    ("ISLAMIC REPUBLIC OF IRAN", "IR"), ("ISLAMIC REPUBLIC OF MAURITANIA", "MR"),
    ("ISLAMIC REPUBLIC OF PAKISTAN", "PK"), ("ISLE OF MAN", "IM"),
    ("ISRAEL", "IL"), ("ITALIAN REPUBLIC", "IT"),
    ("ITALY", "IT"), ("JAMAICA", "JM"),
    ("JAPAN", "JP"), ("JERSEY", "JE"),
    ("JORDAN", "JO"), ("KAZAKHSTAN", "KZ"),
    ("KENYA", "KE"), ("KINGDOM OF BAHRAIN", "BH"),
    ("KINGDOM OF BELGIUM", "BE"), ("KINGDOM OF BHUTAN", "BT"),
    ("KINGDOM OF CAMBODIA", "KH"), ("KINGDOM OF DENMARK", "DK"),
    ("KINGDOM OF ESWATINI", "SZ"), ("KINGDOM OF LESOTHO", "LS"),
    ("KINGDOM OF MOROCCO", "MA"), ("KINGDOM OF NORWAY", "NO"),
    ("KINGDOM OF SAUDI ARABIA", "SA"), ("KINGDOM OF SPAIN", "ES"),
    ("KINGDOM OF SWEDEN", "SE"), ("KINGDOM OF THAILAND", "TH"),
    ("KINGDOM OF THE NETHERLANDS", "NL"), ("KINGDOM OF TONGA", "TO"),
    ("KIRIBATI", "KI"), ("KOREA, DEMOCRATIC PEOPLE'S REPUBLIC OF", "KP"),
    ("KOREA, REPUBLIC OF", "KR"), ("KUWAIT", "KW"),
    ("KYRGYZ REPUBLIC", "KG"), ("KYRGYZSTAN", "KG"),
    ("LAO PEOPLE'S DEMOCRATIC REPUBLIC", "LA"), ("LAOS", "LA"),
    ("LATVIA", "LV"), ("LEBANESE REPUBLIC", "LB"),
    ("LEBANON", "LB"), ("LESOTHO", "LS"),
    ("LIBERIA", "LR"), ("LIBYA", "LY"),
    ("LIECHTENSTEIN", "LI"), ("LITHUANIA", "LT"),
    ("LUXEMBOURG", "LU"), ("MACAO", "MO"),
    ("MACAO SPECIAL ADMINISTRATIVE REGION OF CHINA", "MO"), ("MADAGASCAR", "MG"),
    ("MALAWI", "MW"), ("MALAYSIA", "MY"),
    ("MALDIVES", "MV"), ("MALI", "ML"),
    ("MALTA", "MT"), ("MARSHALL ISLANDS", "MH"),
    ("MARTINIQUE", "MQ"), ("MAURITANIA", "MR"),
    ("MAURITIUS", "MU"), ("MAYOTTE", "YT"),
    ("MEXICO", "MX"), ("MICRONESIA, FEDERATED STATES OF", "FM"),
    ("MOLDOVA", "MD"), ("MOLDOVA, REPUBLIC OF", "MD"),
    ("MONACO", "MC"), ("MONGOLIA", "MN"),
    ("MONTENEGRO", "ME"), ("MONTSERRAT", "MS"),
    ("MOROCCO", "MA"), ("MOZAMBIQUE", "MZ"),
    ("MYANMAR", "MM"), ("NAMIBIA", "NA"),
    ("NAURU", "NR"), ("NEPAL", "NP"),
    ("NETHERLANDS", "NL"), ("NEW CALEDONIA", "NC"),
    ("NEW ZEALAND", "NZ"), ("NICARAGUA", "NI"),
    ("NIGER", "NE"), ("NIGERIA", "NG"),
    ("NIUE", "NU"), ("NORFOLK ISLAND", "NF"),
    ("NORTH KOREA", "KP"), ("NORTH MACEDONIA", "MK"),
    ("NORTHERN MARIANA ISLANDS", "MP"), ("NORWAY", "NO"),
    ("OMAN", "OM"), ("PAKISTAN", "PK"),
    ("PALAU", "PW"), ("PALESTINE, STATE OF", "PS"),
    ("PANAMA", "PA"), ("PAPUA NEW GUINEA", "PG"),
    ("PARAGUAY", "PY"), ("PEOPLE'S DEMOCRATIC REPUBLIC OF ALGERIA", "DZ"),
    ("PEOPLE'S REPUBLIC OF BANGLADESH", "BD"), ("PEOPLE'S REPUBLIC OF CHINA", "CN"),
    ("PERU", "PE"), ("PHILIPPINES", "PH"),
    ("PITCAIRN", "PN"), ("PLURINATIONAL STATE OF BOLIVIA", "BO"),
    ("POLAND", "PL"), ("PORTUGAL", "PT"),
    ("PORTUGUESE REPUBLIC", "PT"), ("PRINCIPALITY OF ANDORRA", "AD"),
    ("PRINCIPALITY OF LIECHTENSTEIN", "LI"), ("PRINCIPALITY OF MONACO", "MC"),
    ("PUERTO RICO", "PR"), ("QATAR", "QA"),
    ("REPUBLIC OF ALBANIA", "AL"), ("REPUBLIC OF ANGOLA", "AO"),
    ("REPUBLIC OF ARMENIA", "AM"), ("REPUBLIC OF AUSTRIA", "AT"),
    ("REPUBLIC OF AZERBAIJAN", "AZ"), ("REPUBLIC OF BELARUS", "BY"),
    ("REPUBLIC OF BENIN", "BJ"), ("REPUBLIC OF BOSNIA AND HERZEGOVINA", "BA"),
    ("REPUBLIC OF BOTSWANA", "BW"), ("REPUBLIC OF BULGARIA", "BG"),
    ("REPUBLIC OF BURUNDI", "BI"), ("REPUBLIC OF CABO VERDE", "CV"),
    ("REPUBLIC OF CAMEROON", "CM"), ("REPUBLIC OF CHAD", "TD"),
    ("REPUBLIC OF CHILE", "CL"), ("REPUBLIC OF COLOMBIA", "CO"),
    ("REPUBLIC OF COSTA RICA", "CR"), ("REPUBLIC OF CROATIA", "HR"),
    ("REPUBLIC OF CUBA", "CU"), ("REPUBLIC OF CYPRUS", "CY"),
    ("REPUBLIC OF CÔTE D'IVOIRE", "CI"), ("REPUBLIC OF DJIBOUTI", "DJ"),
    ("REPUBLIC OF ECUADOR", "EC"), ("REPUBLIC OF EL SALVADOR", "SV"),
    ("REPUBLIC OF EQUATORIAL GUINEA", "GQ"), ("REPUBLIC OF ESTONIA", "EE"),
    ("REPUBLIC OF FIJI", "FJ"), ("REPUBLIC OF FINLAND", "FI"),
    ("REPUBLIC OF GHANA", "GH"), ("REPUBLIC OF GUATEMALA", "GT"),
    ("REPUBLIC OF GUINEA", "GN"), ("REPUBLIC OF GUINEA-BISSAU", "GW"),
    ("REPUBLIC OF GUYANA", "GY"), ("REPUBLIC OF HAITI", "HT"),
    ("REPUBLIC OF HONDURAS", "HN"), ("REPUBLIC OF ICELAND", "IS"),
    ("REPUBLIC OF INDIA", "IN"), ("REPUBLIC OF INDONESIA", "ID"),
    ("REPUBLIC OF IRAQ", "IQ"), ("REPUBLIC OF KAZAKHSTAN", "KZ"),
    ("REPUBLIC OF KENYA", "KE"), ("REPUBLIC OF KIRIBATI", "KI"),
    ("REPUBLIC OF LATVIA", "LV"), ("REPUBLIC OF LIBERIA", "LR"),
    ("REPUBLIC OF LITHUANIA", "LT"), ("REPUBLIC OF MADAGASCAR", "MG"),
    ("REPUBLIC OF MALAWI", "MW"), ("REPUBLIC OF MALDIVES", "MV"),
    ("REPUBLIC OF MALI", "ML"), ("REPUBLIC OF MALTA", "MT"),
    ("REPUBLIC OF MAURITIUS", "MU"), ("REPUBLIC OF MOLDOVA", "MD"),
    ("REPUBLIC OF MOZAMBIQUE", "MZ"), ("REPUBLIC OF MYANMAR", "MM"),
    ("REPUBLIC OF NAMIBIA", "NA"), ("REPUBLIC OF NAURU", "NR"),
    ("REPUBLIC OF NICARAGUA", "NI"), ("REPUBLIC OF NORTH MACEDONIA", "MK"),
    ("REPUBLIC OF PALAU", "PW"), ("REPUBLIC OF PANAMA", "PA"),
    ("REPUBLIC OF PARAGUAY", "PY"), ("REPUBLIC OF PERU", "PE"),
    ("REPUBLIC OF POLAND", "PL"), ("REPUBLIC OF SAN MARINO", "SM"),
    ("REPUBLIC OF SENEGAL", "SN"), ("REPUBLIC OF SERBIA", "RS"),
    ("REPUBLIC OF SEYCHELLES", "SC"), ("REPUBLIC OF SIERRA LEONE", "SL"),
    ("REPUBLIC OF SINGAPORE", "SG"), ("REPUBLIC OF SLOVENIA", "SI"),
    ("REPUBLIC OF SOUTH AFRICA", "ZA"), ("REPUBLIC OF SOUTH SUDAN", "SS"),
    ("REPUBLIC OF SURINAME", "SR"), ("REPUBLIC OF TAJIKISTAN", "TJ"),
    ("REPUBLIC OF THE CONGO", "CG"), ("REPUBLIC OF THE GAMBIA", "GM"),
    ("REPUBLIC OF THE MARSHALL ISLANDS", "MH"), ("REPUBLIC OF THE NIGER", "NE"),
    ("REPUBLIC OF THE PHILIPPINES", "PH"), ("REPUBLIC OF THE SUDAN", "SD"),
    ("REPUBLIC OF TRINIDAD AND TOBAGO", "TT"), ("REPUBLIC OF TUNISIA", "TN"),
    ("REPUBLIC OF TÜRKIYE", "TR"), ("REPUBLIC OF UGANDA", "UG"),
    ("REPUBLIC OF UZBEKISTAN", "UZ"), ("REPUBLIC OF VANUATU", "VU"),
    ("REPUBLIC OF YEMEN", "YE"), ("REPUBLIC OF ZAMBIA", "ZM"),
    ("REPUBLIC OF ZIMBABWE", "ZW"), ("ROMANIA", "RO"),
    ("RUSSIAN FEDERATION", "RU"), ("RWANDA", "RW"),
    ("RWANDESE REPUBLIC", "RW"), ("RÉUNION", "RE"),
    ("SAINT BARTHÉLEMY", "BL"), ("SAINT HELENA, ASCENSION AND TRISTAN DA CUNHA", "SH"),
    ("SAINT KITTS AND NEVIS", "KN"), ("SAINT LUCIA", "LC"),
    ("SAINT MARTIN (FRENCH PART)", "MF"), ("SAINT PIERRE AND MIQUELON", "PM"),
    ("SAINT VINCENT AND THE GRENADINES", "VC"), ("SAMOA", "WS"),
    ("SAN MARINO", "SM"), ("SAO TOME AND PRINCIPE", "ST"),
    ("SAUDI ARABIA", "SA"), ("SENEGAL", "SN"),
    ("SERBIA", "RS"), ("SEYCHELLES", "SC"),
    ("SIERRA LEONE", "SL"), ("SINGAPORE", "SG"),
    ("SINT MAARTEN (DUTCH PART)", "SX"), ("SLOVAK REPUBLIC", "SK"),
    ("SLOVAKIA", "SK"), ("SLOVENIA", "SI"),
    ("SOCIALIST REPUBLIC OF VIET NAM", "VN"), ("SOLOMON ISLANDS", "SB"),
    ("SOMALIA", "SO"), ("SOUTH AFRICA", "ZA"),
    ("SOUTH GEORGIA AND THE SOUTH SANDWICH ISLANDS", "GS"), ("SOUTH KOREA", "KR"),
    ("SOUTH SUDAN", "SS"), ("SPAIN", "ES"),
    ("SRI LANKA", "LK"), ("STATE OF ISRAEL", "IL"),
    ("STATE OF KUWAIT", "KW"), ("STATE OF QATAR", "QA"),
    ("SUDAN", "SD"), ("SULTANATE OF OMAN", "OM"),
    ("SURINAME", "SR"), ("SVALBARD AND JAN MAYEN", "SJ"),
    ("SWEDEN", "SE"), ("SWISS CONFEDERATION", "CH"),
    ("SWITZERLAND", "CH"), ("SYRIA", "SY"),
    ("SYRIAN ARAB REPUBLIC", "SY"), ("TAIWAN", "TW"),
    ("TAIWAN, PROVINCE OF CHINA", "TW"), ("TAJIKISTAN", "TJ"),
    ("TANZANIA", "TZ"), ("TANZANIA, UNITED REPUBLIC OF", "TZ"),
    ("THAILAND", "TH"), ("THE STATE OF ERITREA", "ER"),
    ("THE STATE OF PALESTINE", "PS"), ("TIMOR-LESTE", "TL"),
    ("TOGO", "TG"), ("TOGOLESE REPUBLIC", "TG"),
    ("TOKELAU", "TK"), ("TONGA", "TO"),
    ("TRINIDAD AND TOBAGO", "TT"), ("TUNISIA", "TN"),
    ("TURKMENISTAN", "TM"), ("TURKS AND CAICOS ISLANDS", "TC"),
    ("TUVALU", "TV"), ("TÜRKIYE", "TR"),
    ("UGANDA", "UG"), ("UKRAINE", "UA"),
    ("UNION OF THE COMOROS", "KM"), ("UNITED ARAB EMIRATES", "AE"),
    ("UNITED KINGDOM", "GB"), ("UNITED KINGDOM OF GREAT BRITAIN AND NORTHERN IRELAND", "GB"),
    ("UNITED MEXICAN STATES", "MX"), ("UNITED REPUBLIC OF TANZANIA", "TZ"),
    ("UNITED STATES", "US"), ("UNITED STATES MINOR OUTLYING ISLANDS", "UM"),
    ("UNITED STATES OF AMERICA", "US"), ("URUGUAY", "UY"),
    ("UZBEKISTAN", "UZ"), ("VANUATU", "VU"),
    ("VENEZUELA", "VE"), ("VENEZUELA, BOLIVARIAN REPUBLIC OF", "VE"),
    ("VIET NAM", "VN"), ("VIETNAM", "VN"),
    ("VIRGIN ISLANDS OF THE UNITED STATES", "VI"), ("VIRGIN ISLANDS, BRITISH", "VG"),
    ("VIRGIN ISLANDS, U.S.", "VI"), ("WALLIS AND FUTUNA", "WF"),
    ("WESTERN SAHARA", "EH"), ("YEMEN", "YE"),
    ("ZAMBIA", "ZM"), ("ZIMBABWE", "ZW"),
    ("ÅLAND ISLANDS", "AX"),
];

/// Every ISO 3166-1 alpha-2 code, plus the user-assigned `XK` (Kosovo) the
/// corpus and the fold both use. Generated with the tables above.
///
/// This exists so a report can say which stored country values are not codes
/// at all. The length test that reads like the same question is not: `VU` is
/// two characters and a real code, and it is still wrong for the Bulgarian
/// bodies filed under it (issue 319's per-case half) — while `ZZ` is two
/// characters and no country whatsoever, which only a list can tell you.
pub const ALPHA2: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT",
    "AU", "AW", "AX", "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI",
    "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS", "BT", "BV", "BW", "BY",
    "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM",
    "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK",
    "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL",
    "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR",
    "IS", "IT", "JE", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN",
    "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS",
    "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW",
    "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP",
    "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM",
    "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM",
    "SN", "SO", "SR", "SS", "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF",
    "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO", "TR", "TT", "TV", "TW",
    "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

/// Is this a real alpha-2 country code (or Kosovo's user-assigned `XK`)?
pub fn is_alpha2(code: &str) -> bool {
    let up = code.trim().to_ascii_uppercase();
    up == "XK" || ALPHA2.binary_search(&up.as_str()).is_ok()
}

/// Two country codes exactly one letter apart — issue 326's corruption filter.
///
/// `SK`/`SG` (Slovakia → Singapore), `CZ`/`CR` (Czechia → Costa Rica),
/// `BG`/`BF` (Bulgaria → Burkina Faso). A single-character slip in a two-letter
/// field, which is what makes a shared identifier across the pair read as
/// transcription rather than as geography.
///
/// It is a FILTER and not a verdict. `LT`/`LV` and `SK`/`SI` are real neighbour
/// pairs where both countries plausibly hold the same registrant, so a cluster
/// passing this test is a candidate, not a finding.
pub fn one_letter_apart(a: &str, b: &str) -> bool {
    if a.chars().count() != 2 || b.chars().count() != 2 || a == b {
        return false;
    }
    a.chars().zip(b.chars()).filter(|(x, y)| x != y).count() == 1
}

/// Names that mark an organization as an operational footprint rather than a
/// transcription error (issue 326).
///
/// **This is the class a same-identifier rule would destroy.** An embassy or a
/// development agency is ONE legal entity with ONE register number, filing
/// procurement from every country it operates in. Sweden's Regeringskansliet
/// (`2021003831`) stands under `1A DE KE MD MZ SE UA UG`; Denmark's royal
/// embassy under `BD BF DE KE UA UG US`; Belgium's Enabel under
/// `BE BF BI ML MR NE`; the Swiss development directorate under
/// `BA BF CH CO JO RO TD TJ`. None is a corrupted home code — the country field
/// is recording where the procurement happened.
///
/// Neither of the obvious discriminators separates them: the checksum is silent,
/// and the mention spread shows the SAME asymmetry the real typos do (Enabel is
/// 2 mentions under `NE` against 249 under `BE`). Only the name does, which is
/// why this list exists.
///
/// Matched case-insensitively against the substring, because these appear inside
/// longer strings ("Ambassade Royale du Danemark à Nairobi").
pub fn is_operational_footprint(name: &str) -> bool {
    let n = name.to_lowercase();
    FOOTPRINT_MARKERS.iter().any(|m| n.contains(m))
}

/// The markers, read off the four widest clusters the pair census carried plus
/// their language variants. Deliberately narrow: a marker that also matches an
/// ordinary company would suppress a real typo, and a missed embassy stays in
/// the abstain bucket where it is harmless.
const FOOTPRINT_MARKERS: &[&str] = &[
    // Diplomatic missions.
    "embassy",
    "ambassade",
    "ambasada",
    "ambasciata",
    "botschaft",
    "embajada",
    "consulate",
    "consulat",
    "konsulat",
    "permanent mission",
    "delegation of the european union",
    // Development agencies, which file from the countries they work in.
    "développement et de la coopération",
    "agence belge de développement",
    "development cooperation",
    "entwicklungszusammenarbeit",
    "regeringskansliet",
];

#[cfg(test)]
mod cluster_filters {
    use super::*;

    #[test]
    fn one_letter_apart_is_exactly_one_substitution() {
        // The pairs issue 326 opened with.
        for (a, b) in [
            ("SK", "SG"),
            ("SK", "SO"),
            ("SK", "SR"),
            ("SK", "SI"),
            ("CZ", "CR"),
            ("BG", "BF"),
            ("BG", "BT"),
            ("LT", "LV"),
            ("NL", "NO"),
        ] {
            assert!(one_letter_apart(a, b), "{a}/{b}");
            assert!(one_letter_apart(b, a), "symmetric: {b}/{a}");
        }
        // Two substitutions is not one, and identity is not a slip.
        for (a, b) in [("SK", "GB"), ("BG", "VU"), ("SK", "SK"), ("PL", "IT")] {
            assert!(!one_letter_apart(a, b), "{a}/{b}");
        }
        // `VA`/`VE` IS one letter apart, and the first draft of this test
        // asserted otherwise. Worth keeping as a fixture because of what it
        // exposed: in the Bulgarian cluster `BG BW VA VE VG VU`, the SPRAY codes
        // are one letter from EACH OTHER, so a cluster-wide "some two codes are
        // one letter apart" test can fire on the spray alone and say nothing
        // about the heavy code. That is why the census reports
        // `heavy_one_letter` separately.
        assert!(one_letter_apart("VA", "VE"));
        assert!(one_letter_apart("VA", "VU"));
        assert!(one_letter_apart("BG", "BW"));
        // A transposition is two substitutions. Recorded because "one letter
        // apart" could plausibly have meant edit distance 1, which admits
        // transpositions and insertions; this is substitution only, and the
        // fixed two-character width is why that is the right reading.
        assert!(!one_letter_apart("SK", "KS"));
        // Not two letters at all.
        assert!(!one_letter_apart("DEU", "DE"));
        assert!(!one_letter_apart("D", "DE"));
        assert!(!one_letter_apart("", "DE"));
    }

    #[test]
    fn the_footprint_markers_catch_the_clusters_they_were_read_from() {
        for name in [
            "Embassy of Sweden",
            "Regeringskansliet",
            "Ambassade Royale du Danemark",
            "Enabel — Agence belge de développement",
            "Direction du développement et de la coopération",
            "AMBASSADE DE FRANCE",           // caps
            "Botschaft der Bundesrepublik Deutschland",
        ] {
            assert!(is_operational_footprint(name), "{name}");
        }
        // And do NOT catch ordinary registrants. A marker that fires here would
        // suppress a real typo, which is the costlier mistake: a missed embassy
        // merely stays in the abstain bucket.
        for name in [
            "„Петрол“ АД",
            "Philips AB Healthcare",
            "Inmac WStore SAS",
            "Krajowa Izba Odwoławcza",
            "ÅF-Infrastructure AB",
            "Development Bank of Austria",   // 'development' alone must not fire
            "Consultancy Services Ltd",      // must not trip on 'consulat'
        ] {
            assert!(!is_operational_footprint(name), "{name}");
        }
    }
}

/// Label text some publishers write in FRONT of the identifier (issue 328).
///
/// `USTIDDE329214156` is `USt-IdNr. DE329214156` — a perfectly good German VAT
/// number with its own field name glued on. 5,766 rows carry one of these, and
/// **3,253 of them have a partner row already standing under the bare value**,
/// so the class is not cosmetic: it splits an organization from its own
/// correctly-formed twin.
///
/// **Read off the corpus, longest first, and the order is load-bearing** —
/// `USTIDNR` must be tried before `USTID`, and
/// `UMSATZSTEUERIDENTIFIKATIONSNUMMER` before `UMSATZSTEUERID`, or the longer
/// label leaves a fragment behind. [`label_prefixes_are_longest_first`] asserts
/// the ordering rather than trusting it.
///
/// **`HRB` and `HRA` are deliberately absent.** `HANDELSREGISTERHRB…` strips to
/// `HRB…` and stops there: `HANDELSREGISTER` is a field name, `HRB` is the
/// register division and carries meaning. The census separates them; a guess
/// would have taken both.
///
/// **The non-German labels (issue 359).** Issue 328 read the class off the
/// German VAT rows and stopped there; the 357 campaign's last slices then met
/// the same shape in Polish, Italian and Spanish — `NIPA41015322` (a Spanish CIF
/// with the Polish field name), `NUMERNIPDE312308370`, `CFEPIVA10548370963`,
/// `CIFA48283964` — and the corpus carries 18,318 distinct `NIP…` values,
/// ~1,000 `PIVA…`, ~280 `CFEPIVA…`, ~260 `CIF…`, ~290 `NUMERNIP…` (2026-09-06
/// prefix reads, idle box). `NIP`, `KRS`, `REGON`, `CIF` and `NIF` are ALSO
/// register tags in [`crate::project`]'s `REGISTER_PREFIXES`: unlike `HRB` they
/// name the very scheme the bare value classifies as by shape (a 10-digit PL
/// national IS a NIP, a 9-digit one a REGON, a 0-led 10-digit a KRS serial, a
/// letter-and-eight under ES a CIF), so the tag is a field name here, not a
/// division — the strip runs first and the register arm keeps the value only when
/// the remainder is refused. `VAT` alone is NOT listed: `VATNO…` is as often
/// the Norwegian country prefix as the English word.
///
/// **The Finnish labels (issue 363).** Read off the `FI` national rows on
/// 2026-09-07 (the issue-358 move had just folded Åland into them): `YTUNNUS…`
/// 149 rows, `FONR…` 12 and `FONUMMER…` 3 (Swedish *FO-nummer*, how Åland and
/// the Swedish-speaking municipalities write the same id), `BUSINESSID…` 2 —
/// and 11 of the first 12 `YTUNNUS` rows read had a bare twin standing
/// (Ramboll, Siemens, Kerava, Rauma…). The bare `Y` abbreviation (`Y 0123456-7`
/// → `Y01234567`, 266 rows) is NOT a table entry: a one-letter prefix would
/// match every Y-led word, so it is a shape rule in [`label_prefix_stripped`].
const LABEL_PREFIXES: &[&str] = &[
    "UMSATZSTEUERIDENTIFIKATIONSNUMMERGEM27AUMSATZSTEUERGESETZ",
    "UMSATZSTEUERIDENTIFIKATIONSNUMMERGEM27AUSTG",
    "UMSATZSTEUERIDENTIFIKATIONSNUMMER",
    "UMSATZSTEUERIDENTIFIKATIONSNR",
    "USTIDENTIFIKATIONSNUMMER",
    "UMSATZSTEUERIDENTNUMMER",
    "HANDELSREGISTERNUMMER",
    "UMSATZSTEUERIDENTNR",
    "UMSATZSTEUERGESETZ",
    "HANDELSREGISTERNR",
    "UMSATZSTEUERIDNR",
    "HANDELSREGISTER",
    "UMSATZSTEUERID",
    "UMSATZSTEUERNR",
    "USTIDENTNUMMER",
    "CODICEFISCALE",
    "STEUERNUMMER",
    "USTIDNUMMER",
    "PARTITAIVA",
    "USTIDENTNR",
    "USTIDNRUID",
    "BUSINESSID",
    "NUMERNIP",
    "NIPNUMER",
    "FONUMMER",
    "USTIDNR",
    "CFEPIVA",
    "YTUNNUS",
    "USTID",
    "VATID",
    "REGON",
    "IDNR",
    "STNR",
    "PIVA",
    "FONR",
    "NIP",
    "KRS",
    "CIF",
    "NIF",
    "CF",
];

/// `value` with one leading publisher label removed, or `None` when it carries
/// none.
///
/// **Removing the label is only half the job — the caller MUST re-validate.**
/// Three rows carry `UMSATZSTEUERIDENTIFIKATIONSNUMMER` and nothing else: the
/// field name alone, no number anywhere. Stripping that leaves the empty string,
/// and a strip that does not check its own remainder would go on to invent an
/// identifier out of whatever fragment survived. `USTIDNRUIDDE…` is the same
/// hazard one level up — strip the wrong entry and `UIDDE…` looks plausible
/// without being anything.
///
/// So this returns the REMAINDER and makes no claim about it. Issue 325's
/// suffix work needed no such guard because a scheme label at the back sits
/// behind a value that already parsed; a label at the front hides the value
/// entirely until it is gone.
pub fn label_prefix_stripped(value: &str) -> Option<&str> {
    // The LONGEST matching label is the right reading of the string, and it is
    // chosen BEFORE the remainder is judged. Doing it the other way — taking the
    // first entry that leaves something non-empty — falls through from a longer
    // label to a shorter one and manufactures a fragment: the first draft of
    // this function turned the bare field name
    // `UMSATZSTEUERIDENTIFIKATIONSNUMMER` into the identifier
    // `ENTIFIKATIONSNUMMER`, by matching `UMSATZSTEUERID` after the exact entry
    // left nothing. Its own test caught it.
    let Some(label) = LABEL_PREFIXES.iter().find(|p| value.starts_with(**p)) else {
        // Issue 363: the Finnish `Y` (Y-tunnus) abbreviation — shape-bound
        // rather than a table entry. `Y` followed by seven or eight digits and
        // NOTHING ELSE: a Spanish NIE (`Y1234567X`) ends in its check letter,
        // a word (`YMPARISTO…`) has letters after the Y, and neither matches.
        // The caller's re-validation still decides: under `FI` the remainder
        // meets the HARD Y-tunnus checksum, so a mistyped one stays as
        // published rather than being mangled.
        let rest = value.strip_prefix('Y')?;
        return ((7..=8).contains(&rest.len()) && rest.bytes().all(|b| b.is_ascii_digit()))
            .then_some(rest);
    };
    let rest = &value[label.len()..];
    // A label with nothing after it is a field name, not an identifier.
    (!rest.is_empty()).then_some(rest)
}

#[cfg(test)]
mod label_prefixes {
    use super::*;

    /// The ordering is the whole correctness argument for a longest-match table,
    /// so it is asserted rather than maintained by care.
    #[test]
    fn label_prefixes_are_longest_first() {
        for pair in LABEL_PREFIXES.windows(2) {
            assert!(
                pair[0].len() >= pair[1].len(),
                "{} must not precede the longer {}",
                pair[0],
                pair[1]
            );
        }
    }

    /// Real prod values, and what each must leave behind.
    #[test]
    fn the_label_comes_off_and_the_identifier_survives() {
        for (raw, want) in [
            ("USTIDDE329214156", "DE329214156"),     // Die Autobahn GmbH des Bundes
            ("USTIDNRDE811335517", "DE811335517"),   // Regierung von Oberbayern
            ("UMSATZSTEUERIDDE188369991", "DE188369991"), // TU Dresden
            ("UMSATZSTEUERIDENTIFIKATIONSNUMMERDE198235088", "DE198235088"),
            ("UMSATZSTEUERIDENTNRDE111111111", "DE111111111"),
            // NOT German: the label is country-agnostic and `ATU` is Austria's
            // own VAT prefix, which must survive intact.
            ("USTIDNRATU37675002", "ATU37675002"),
            ("USTIDATU37675002", "ATU37675002"),
            // The field name is the label; the register division is not.
            ("HANDELSREGISTERHRB12345", "HRB12345"),
            ("HANDELSREGISTERNUMMERHRB12345", "HRB12345"),
            ("STEUERNUMMER12345678", "12345678"),
            ("STNRDE123456789", "DE123456789"),
            // Issue 359 — the Polish, Italian and Spanish field names, all real
            // prod values from the 357 campaign's packets.
            ("NIP1070000916", "1070000916"),          // SAFEGE's Polish branch
            ("NIPA41015322", "A41015322"),            // Ayesa: a Spanish CIF under a Polish label
            ("NUMERNIPDE312308370", "DE312308370"),   // Acandis GmbH, "Numer NIP: DE…"
            ("NIPNUMER5260001234", "5260001234"),
            ("NIPPL5260001234", "PL5260001234"),      // the label AND the VAT prefix
            ("REGON123456789", "123456789"),
            ("KRS0000123456", "0000123456"),
            ("PIVA10548370963", "10548370963"),       // Lloyd's Insurance Company, Italian branch
            ("CFEPIVA10548370963", "10548370963"),    // "CF e P.IVA"
            ("PARTITAIVA12525420159", "12525420159"),
            ("CODICEFISCALE97819940152", "97819940152"),
            ("CF97819940152", "97819940152"),
            ("CIFA48283964", "A48283964"),            // IDOM
            ("NIF501234567", "501234567"),            // a Portuguese NIF
            ("VATIDGB287249363", "GB287249363"),      // Therakos EMEA
            // Issue 363 — the Finnish field names, prod values from the FI
            // national rows (Kerava, an Åland company, Ramboll).
            ("YTUNNUS01274855", "01274855"),
            ("FONR01446821", "01446821"),
            ("FONUMMER01446821", "01446821"),
            ("BUSINESSID010111975", "010111975"),
            // …and the bare `Y` abbreviation, by shape.
            ("Y01274855", "01274855"),
            ("Y1274855", "1274855"),
        ] {
            assert_eq!(label_prefix_stripped(raw), Some(want), "{raw}");
        }
    }

    /// A LABEL WITH NO NUMBER IS NOT AN IDENTIFIER. Three prod rows carry the
    /// field name alone.
    #[test]
    fn a_bare_label_strips_to_nothing_and_is_refused() {
        assert_eq!(label_prefix_stripped("UMSATZSTEUERIDENTIFIKATIONSNUMMER"), None);
        assert_eq!(label_prefix_stripped("USTID"), None);
        assert_eq!(label_prefix_stripped("STNR"), None);
        assert_eq!(label_prefix_stripped("NIP"), None);
        assert_eq!(label_prefix_stripped("CIF"), None);
        assert_eq!(label_prefix_stripped("YTUNNUS"), None);
        assert_eq!(label_prefix_stripped("Y"), None);
    }

    /// Issue 363: the bare `Y` is a shape, not a word — only `Y` + 7–8 digits
    /// and nothing else. A Spanish NIE keeps its `Y`, and so does any Y-led
    /// word or a Y-led value of another length.
    #[test]
    fn the_bare_y_strips_only_by_shape() {
        for keep in ["Y1234567X", "YMPARISTOMINISTERIO123", "Y12345", "Y123456789", "Y0127485A", "YT22493"] {
            assert_eq!(label_prefix_stripped(keep), None, "{keep} keeps its Y");
        }
    }

    /// And the remainder is returned WITHOUT a claim, which is the contract the
    /// caller's re-validation rests on. `USTIDNRDEDE…` (three prod rows, a
    /// doubled country code) and `USTIDNRUIDDE…` both strip to something
    /// malformed, and it is the classifier downstream — not this function —
    /// that must refuse them.
    #[test]
    fn a_malformed_remainder_is_returned_not_judged() {
        assert_eq!(label_prefix_stripped("USTIDNRDEDE123456789"), Some("DEDE123456789"));
        // The longest match wins, so the nested-label row resolves cleanly…
        assert_eq!(label_prefix_stripped("USTIDNRUIDDE123456789"), Some("DE123456789"));
        // …and something carrying no label at all is left entirely alone.
        assert_eq!(label_prefix_stripped("DE329214156"), None);
        assert_eq!(label_prefix_stripped("HRB12345"), None);
    }
}
