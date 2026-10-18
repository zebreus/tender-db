//! Issue 570 unit 3: the cached chains the remap's tests in `project::tests` fold, generated from
//! `.scratch/tender-db/570-lot-keys/raw-cache.tar.gz` (the measurers' and skeptics' `/v1/tenders/{id}` and
//! `/v1/notices/{id}/content` reads; 8375276's from `567-lot-mapping/readback/raw-api-cache.tar.gz`), in the
//! order `remap_port.py`'s run of 2026-10-18 searched them.
//!
//! Each notice is reduced to what the fold's lot keys read: every lot section (section id as served, kind
//! `Lot`) with its own `TED-ITEM`, `TED-LOT_NO` and `TED-LOT_NUMBER` values, plus its first `TED-TITLE`
//! where a test reads titles; every award block (`LotResult`) with the lot references of its subtree, put on
//! the block itself (the projection reads a block's subtree as one, `enclosing`). Values are as published,
//! in the store's order. The profile is `ted-export-r208` where the lot sections print only `LOT_NUMBER`,
//! else `ted-export-r209` (the fold reads neither, only that both are legacy).
//!
//! `PORT_*` are `remap_port.py`'s outcomes for the same chain (`port_result.json`'s rows; for 8375276,
//! reduced to three notices, the port re-run on the reduced chain, whose blocks agree with the full chain's).

/// One notice: `(section id, [(field, value)])` per lot section and per award block.
pub(super) struct Notice {
    pub(super) id: i64,
    pub(super) profile: &'static str,
    pub(super) lots: &'static [Section],
    pub(super) blocks: &'static [Section],
}

pub(super) type Section = (&'static str, &'static [(&'static str, &'static str)]);

/// Tender 7274985: notices 22137062, 22223285.
pub(super) const CHAIN_7274985: &[Notice] = &[
    Notice {
        id: 22137062,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "S1"), ("TED-TITLE", "Prace z zakresu rozdrabniania pozostałości roślinnych i przygotowania gleby na terenie leśnictw Sternalice, Boroszów i Siedem Źródeł oraz transport sadzonek ze szkółek LP")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "S2"), ("TED-TITLE", "Prace z zakresu rozdrabniania pozostałości roślinnych i przygotowania gleby na terenie leśnictw Radłów, Grodzisko i Osiecko.")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "S3"), ("TED-TITLE", "Prace z zakresu rozdrabniania pozostałości roślinnych i przygotowania gleby na terenie leśnictw Leśna, Chudoba i Pruszków oraz transport słupów do grodzenia upraw do leśnictw Nadleśnictwa Olesno")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "S4"), ("TED-TITLE", "Prace z zakresu rozdrabniania pozostałości roślinnych i przygotowania gleby na terenie leśnictw Szumirad, Trzebiszyn, Radawka i Ligota Turawska")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 22223285,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "S1"), ("TED-TITLE", "Prace z zakresu rozdrabniania pozostałości roślinnych i przygotowania gleby na terenie leśnictw Sternalice, Boroszów i Siedem Źródeł oraz transport sadzonek ze szkółek LP")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "S2"), ("TED-TITLE", "Prace z zakresu rozdrabniania pozostałości roślinnych i przygotowania gleby na terenie leśnictw Radłów, Grodzisko i Osiecko.")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "S4"), ("TED-TITLE", "Prace z zakresu rozdrabniania pozostałości roślinnych i przygotowania gleby na terenie leśnictw Szumirad, Trzebiszyn, Radawka i Ligota Turawska")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "S1")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "S2")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "S4")]),
        ],
    },
];
/// Tender 7274985: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_7274985: &[(i64, &str, Option<&str>)] = &[
    (22223285, "RES-1", Some("LOT-1")),
    (22223285, "RES-2", Some("LOT-2")),
    (22223285, "RES-3", Some("LOT-4")),
];
/// Tender 7274985: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_7274985: &[(i64, &str, Option<&str>, Option<&str>)] = &[
    (22223285, "RES-3", Some("LOT-3"), Some("LOT-4")),
];
/// Tender 7274985: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_7274985: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("S1")),
    ("LOT-2", Some("S2")),
    ("LOT-3", Some("S3")),
    ("LOT-4", Some("S4")),
];
/// Tender 7274985: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_7274985: &[(i64, &str)] = &[
    (22137062, "anchor"),
    (22223285, "remapped"),
];

/// Tender 7511890: notices 22690061, 22736811, 22907340, 22980479.
pub(super) const CHAIN_7511890: &[Notice] = &[
    Notice {
        id: 22690061,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1"), ("TED-TITLE", "Pakiet 1: Dostawa materiałów pomocniczych do wykonywania badań laboratoryjnych")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2"), ("TED-TITLE", "Pakiet 2: Dostawa akcesoriów i materiałów laboratoryjnych")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3"), ("TED-TITLE", "Pakiet 3: Dostawa akcesoriów i materiałów laboratoryjnych do posiadanego przez Zamawiającego drobnego sprzętu laboratoryjnego")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4"), ("TED-TITLE", "Pakiet 4: Sukcesywne dostawy zużywalnych materiałów, drobnego sprzętu laboratoryjnego, medycznego na potrzeby Centrum Symulacji Medycznych")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5"), ("TED-TITLE", "Pakiet 5: Dostawa akcesoriów i materiałów laboratoryjnych")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6"), ("TED-TITLE", "Pakiet 6: Dostawa akcesoriów i materiałów zużywalnych do posiadanych urządzeń firmy Eppendorf")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 22736811,
        profile: "ted-export-r209",
        lots: &[],
        blocks: &[],
    },
    Notice {
        id: 22907340,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1"), ("TED-TITLE", "Pakiet 1: Dostawa materiałów pomocniczych do wykonywania badań laboratoryjnych")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2"), ("TED-TITLE", "Pakiet 2: Dostawa akcesoriów i materiałów laboratoryjnych")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "5"), ("TED-TITLE", "Pakiet 5: Dostawa akcesoriów i materiałów laboratoryjnych")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "5")]),
        ],
    },
    Notice {
        id: 22980479,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "3"), ("TED-TITLE", "Pakiet 3: Dostawa akcesoriów i materiałów laboratoryjnych do posiadanego przez Zamawiającego drobnego sprzętu laboratoryjnego")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "4"), ("TED-TITLE", "Pakiet 4: Sukcesywne dostawy zużywalnych materiałów, drobnego sprzętu laboratoryjnego, medycznego na potrzeby Centrum Symulacji Medycznych")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "6"), ("TED-TITLE", "Pakiet 6: Dostawa akcesoriów i materiałów zużywalnych do posiadanych urządzeń firmy Eppendorf")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "3")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "4")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "6")]),
        ],
    },
];
/// Tender 7511890: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_7511890: &[(i64, &str, Option<&str>)] = &[
    (22907340, "RES-1", Some("LOT-1")),
    (22907340, "RES-2", Some("LOT-2")),
    (22907340, "RES-3", Some("LOT-5")),
    (22980479, "RES-1", Some("LOT-3")),
    (22980479, "RES-2", Some("LOT-4")),
    (22980479, "RES-3", Some("LOT-6")),
];
/// Tender 7511890: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_7511890: &[(i64, &str, Option<&str>, Option<&str>)] = &[
    (22907340, "RES-3", Some("LOT-3"), Some("LOT-5")),
    (22980479, "RES-1", Some("LOT-1"), Some("LOT-3")),
    (22980479, "RES-2", Some("LOT-2"), Some("LOT-4")),
    (22980479, "RES-3", Some("LOT-3"), Some("LOT-6")),
];
/// Tender 7511890: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_7511890: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-2", Some("2")),
    ("LOT-3", Some("3")),
    ("LOT-4", Some("4")),
    ("LOT-5", Some("5")),
    ("LOT-6", Some("6")),
];
/// Tender 7511890: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_7511890: &[(i64, &str)] = &[
    (22690061, "anchor"),
    (22907340, "remapped"),
    (22980479, "remapped"),
];

/// Tender 7697322: notices 23121547, 23276289, 23413021.
pub(super) const CHAIN_7697322: &[Notice] = &[
    Notice {
        id: 23121547,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "01"), ("TED-TITLE", "v.r.d")]),
            ("LOT-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "10"), ("TED-TITLE", "platrerie - peinture - faux Plafonds")]),
            ("LOT-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "11"), ("TED-TITLE", "plomberie - sanitaires - chauffage")]),
            ("LOT-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "12"), ("TED-TITLE", "electricite - courants Faibles - television")]),
            ("LOT-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "13"), ("TED-TITLE", "facades")]),
            ("LOT-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "14"), ("TED-TITLE", "serrurerie - metallerie")]),
            ("LOT-15", &[("TED-ITEM", "15"), ("TED-LOT_NO", "15"), ("TED-TITLE", "portes Sectionnables")]),
            ("LOT-16", &[("TED-ITEM", "16"), ("TED-LOT_NO", "16"), ("TED-TITLE", "carrelage - faiences - chapes")]),
            ("LOT-17", &[("TED-ITEM", "17"), ("TED-LOT_NO", "17"), ("TED-TITLE", "espaces Verts - clotures")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "02"), ("TED-TITLE", "terrassement")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "03"), ("TED-TITLE", "renforcement De Sol")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "04"), ("TED-TITLE", "gros Oeuvre - maconnerie")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "05"), ("TED-TITLE", "charpente - couverture - zinguerie - couverture Zinc")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "06"), ("TED-TITLE", "charpente Metallique - bardage - couverture")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "07"), ("TED-TITLE", "etancheite")]),
            ("LOT-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "08"), ("TED-TITLE", "menuiseries Exterieures Pvc Et Aluminium")]),
            ("LOT-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "09"), ("TED-TITLE", "menuiseries Interieures Bois")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 23276289,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "02"), ("TED-TITLE", "terrassement")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "02")]),
        ],
    },
    Notice {
        id: 23413021,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "01"), ("TED-TITLE", "v.r.d")]),
            ("LOT-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "13"), ("TED-TITLE", "facades")]),
            ("LOT-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "14"), ("TED-TITLE", "serrurerie - metallerie")]),
            ("LOT-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "15"), ("TED-TITLE", "portes Sectionnables")]),
            ("LOT-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "16"), ("TED-TITLE", "carrelage - faiences - chapes")]),
            ("LOT-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "17"), ("TED-TITLE", "espaces Verts - clotures")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "03"), ("TED-TITLE", "renforcement De Sol")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "04"), ("TED-TITLE", "gros Oeuvre - maconnerie")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "05"), ("TED-TITLE", "charpente - couverture - zinguerie - couverture Zinc")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "06"), ("TED-TITLE", "charpente Metallique - bardage - couverture")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "08"), ("TED-TITLE", "menuiseries Exterieures Pvc Et Aluminium")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "10"), ("TED-TITLE", "platrerie - peinture - faux Plafonds")]),
            ("LOT-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "11"), ("TED-TITLE", "plomberie - sanitaires - chauffage")]),
            ("LOT-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "12"), ("TED-TITLE", "electricite - courants Faibles - television")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "01")]),
            ("RES-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "13")]),
            ("RES-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "14")]),
            ("RES-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "15")]),
            ("RES-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "16")]),
            ("RES-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "17")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "03")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "04")]),
            ("RES-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "05")]),
            ("RES-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "06")]),
            ("RES-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "08")]),
            ("RES-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "10")]),
            ("RES-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "11")]),
            ("RES-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "12")]),
        ],
    },
];
/// Tender 7697322: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_7697322: &[(i64, &str, Option<&str>)] = &[
    (23276289, "RES-1", Some("LOT-2")),
    (23413021, "RES-1", Some("LOT-1")),
    (23413021, "RES-10", Some("LOT-13")),
    (23413021, "RES-11", Some("LOT-14")),
    (23413021, "RES-12", Some("LOT-15")),
    (23413021, "RES-13", Some("LOT-16")),
    (23413021, "RES-14", Some("LOT-17")),
    (23413021, "RES-2", Some("LOT-3")),
    (23413021, "RES-3", Some("LOT-4")),
    (23413021, "RES-4", Some("LOT-5")),
    (23413021, "RES-5", Some("LOT-6")),
    (23413021, "RES-6", Some("LOT-8")),
    (23413021, "RES-7", Some("LOT-10")),
    (23413021, "RES-8", Some("LOT-11")),
    (23413021, "RES-9", Some("LOT-12")),
];
/// Tender 7697322: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_7697322: &[(i64, &str, Option<&str>, Option<&str>)] = &[
    (23276289, "RES-1", Some("LOT-1"), Some("LOT-2")),
    (23413021, "RES-10", Some("LOT-10"), Some("LOT-13")),
    (23413021, "RES-11", Some("LOT-11"), Some("LOT-14")),
    (23413021, "RES-12", Some("LOT-12"), Some("LOT-15")),
    (23413021, "RES-13", Some("LOT-13"), Some("LOT-16")),
    (23413021, "RES-14", Some("LOT-14"), Some("LOT-17")),
    (23413021, "RES-2", Some("LOT-2"), Some("LOT-3")),
    (23413021, "RES-3", Some("LOT-3"), Some("LOT-4")),
    (23413021, "RES-4", Some("LOT-4"), Some("LOT-5")),
    (23413021, "RES-5", Some("LOT-5"), Some("LOT-6")),
    (23413021, "RES-6", Some("LOT-6"), Some("LOT-8")),
    (23413021, "RES-7", Some("LOT-7"), Some("LOT-10")),
    (23413021, "RES-8", Some("LOT-8"), Some("LOT-11")),
    (23413021, "RES-9", Some("LOT-9"), Some("LOT-12")),
];
/// Tender 7697322: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_7697322: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("01")),
    ("LOT-10", Some("10")),
    ("LOT-11", Some("11")),
    ("LOT-12", Some("12")),
    ("LOT-13", Some("13")),
    ("LOT-14", Some("14")),
    ("LOT-15", Some("15")),
    ("LOT-16", Some("16")),
    ("LOT-17", Some("17")),
    ("LOT-2", Some("02")),
    ("LOT-3", Some("03")),
    ("LOT-4", Some("04")),
    ("LOT-5", Some("05")),
    ("LOT-6", Some("06")),
    ("LOT-7", Some("07")),
    ("LOT-8", Some("08")),
    ("LOT-9", Some("09")),
];
/// Tender 7697322: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_7697322: &[(i64, &str)] = &[
    (23121547, "anchor"),
    (23276289, "remapped"),
    (23413021, "remapped"),
];

/// Tender 5787834: notices 19020649, 19154619, 19167246.
pub(super) const CHAIN_5787834: &[Notice] = &[
    Notice {
        id: 19020649,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1"), ("TED-TITLE", "Комплекти отстеосинтезни средства за интрамедуларна фиксация на фрактури")]),
            ("LOT-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "10"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с 36 мм керамична глава върху полиетиленов инлей")]),
            ("LOT-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "11"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с 36 мм метална глава върху полиетиленов инлей")]),
            ("LOT-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "12"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с керамична глава върху титаниева капсула с керамично покритие")]),
            ("LOT-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "13"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с 32 мм керамична глава върху полиетиленов инлей")]),
            ("LOT-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "14"), ("TED-TITLE", "Компоненти за тазобедрено ендопротезиране")]),
            ("LOT-15", &[("TED-ITEM", "15"), ("TED-LOT_NO", "15"), ("TED-TITLE", "Компоненти за първично и ревизионно тотално тазобедрено ендопротезиране, включващи циментно и безциментно закрепване")]),
            ("LOT-16", &[("TED-ITEM", "16"), ("TED-LOT_NO", "16"), ("TED-TITLE", "Ендопротези")]),
            ("LOT-17", &[("TED-ITEM", "17"), ("TED-LOT_NO", "17"), ("TED-TITLE", "Системи за тотално, първично ендопротезиране на колянна става")]),
            ("LOT-18", &[("TED-ITEM", "18"), ("TED-LOT_NO", "18"), ("TED-TITLE", "Система за уникондилно ендопротезиране на колянна става")]),
            ("LOT-19", &[("TED-ITEM", "19"), ("TED-LOT_NO", "19"), ("TED-TITLE", "Компоненти за първично тотално колянно протезиране включващи")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2"), ("TED-TITLE", "Комплекти от Заключващи плаки със заключващи винтове")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3"), ("TED-TITLE", "Комплекти от Конвенционална остеосинтеза и консумативи за артроскопски интервенции")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4"), ("TED-TITLE", "Тотална безциментна ендопротеза за тазобедрена става")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5"), ("TED-TITLE", "Тотална циментна ендопротеза за тазобедрена става")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6"), ("TED-TITLE", "Система за еднополюсно тазобедрено ендопротезиране, съставена от биполярна бедрена глава и циментно бедрено стебло")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7"), ("TED-TITLE", "Тотална хибридна система за тазобедрено ендопротезиране, съставена от циментна ацетабуларна компонента и безциментно бедрено стебло")]),
            ("LOT-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "8"), ("TED-TITLE", "Тотална хибридна система за тазобедрено ендопротезиране, съставена от безциментна ацетабуларна компонента и циментно бедрено стебло")]),
            ("LOT-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "9"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с керамични компоненти")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 19154619,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "18"), ("TED-TITLE", "Система за уникондилно ендопротезиране на колянна става")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "18")]),
        ],
    },
    Notice {
        id: 19167246,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1"), ("TED-TITLE", "Комплекти отстеосинтезни средства за интрамедуларна фиксация на фрактури")]),
            ("LOT-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "10"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с 36 мм керамична глава върху полиетиленов инлей")]),
            ("LOT-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "11"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с 36 мм метална глава върху полиетиленов инлей")]),
            ("LOT-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "12"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с керамична глава върху титаниева капсула с керамично покритие")]),
            ("LOT-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "13"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с 32 мм керамична глава върху полиетиленов инлей")]),
            ("LOT-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "14"), ("TED-TITLE", "Компоненти за тазобедрено ендопротезиране")]),
            ("LOT-15", &[("TED-ITEM", "15"), ("TED-LOT_NO", "15"), ("TED-TITLE", "Компоненти за първично и ревизионно тотално тазобедрено ендопротезиране, включващи циментно и безциментно закрепване")]),
            ("LOT-16", &[("TED-ITEM", "16"), ("TED-LOT_NO", "16"), ("TED-TITLE", "Ендопротези")]),
            ("LOT-17", &[("TED-ITEM", "17"), ("TED-LOT_NO", "17"), ("TED-TITLE", "Системи за тотално, първично ендопротезиране на колянна става")]),
            ("LOT-18", &[("TED-ITEM", "18"), ("TED-LOT_NO", "19"), ("TED-TITLE", "Компоненти за първично тотално колянно протезиране включващи:")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2"), ("TED-TITLE", "Комплекти от Заключващи плаки със заключващи винтове")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3"), ("TED-TITLE", "Комплекти от Конвенционална остеосинтеза и консумативи за артроскопски интервенции")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4"), ("TED-TITLE", "Тотална безциментна ендопротеза за тазобедрена става")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5"), ("TED-TITLE", "\"Тотална циментна ендопротеза за тазобедрена става")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6"), ("TED-TITLE", "Система за еднополюсно тазобедрено ендопротезиране, съставена от биполярна бедрена глава и циментно бедрено стебло")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7"), ("TED-TITLE", "Тотална хибридна система за тазобедрено ендопротезиране, съставена от циментна ацетабуларна компонента и безциментно бедрено стебло")]),
            ("LOT-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "8"), ("TED-TITLE", "Тотална хибридна система за тазобедрено ендопротезиране, съставена от безциментна ацетабуларна компонента и циментно бедрено стебло")]),
            ("LOT-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "9"), ("TED-TITLE", "Тотална първична безциментна ендопротеза за тазобедрена става с керамични компоненти")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "16")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "15 и 19")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "1, 2 и 3")]),
            ("RES-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4, 5, 6, 7, 8, 9, 10, 11, 12, 13 и 17")]),
            ("RES-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "14")]),
        ],
    },
];
/// Tender 5787834: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_5787834: &[(i64, &str, Option<&str>)] = &[
    (19154619, "RES-1", Some("LOT-18")),
    (19167246, "RES-1", Some("LOT-16")),
    (19167246, "RES-2", None),
    (19167246, "RES-3", None),
    (19167246, "RES-4", None),
    (19167246, "RES-5", Some("LOT-14")),
];
/// Tender 5787834: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_5787834: &[(i64, &str, Option<&str>, Option<&str>)] = &[
    (19154619, "RES-1", Some("LOT-1"), Some("LOT-18")),
];
/// Tender 5787834: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_5787834: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-10", Some("10")),
    ("LOT-11", Some("11")),
    ("LOT-12", Some("12")),
    ("LOT-13", Some("13")),
    ("LOT-14", Some("14")),
    ("LOT-15", Some("15")),
    ("LOT-16", Some("16")),
    ("LOT-17", Some("17")),
    ("LOT-18", Some("18")),
    ("LOT-19", Some("19")),
    ("LOT-2", Some("2")),
    ("LOT-3", Some("3")),
    ("LOT-4", Some("4")),
    ("LOT-5", Some("5")),
    ("LOT-6", Some("6")),
    ("LOT-7", Some("7")),
    ("LOT-8", Some("8")),
    ("LOT-9", Some("9")),
];
/// Tender 5787834: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_5787834: &[(i64, &str)] = &[
    (19020649, "anchor"),
    (19154619, "remapped"),
    (19167246, "remapped"),
];

/// Tender 8433088: notices 20329681, 20739611.
pub(super) const CHAIN_8433088: &[Notice] = &[
    Notice {
        id: 20329681,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1-4"), ("TED-TITLE", "Lot 01-04 Ultra-freezers and Accessories")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 20739611,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1"), ("TED-TITLE", "Ultra-Freezer Upright-Type")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2-3"), ("TED-TITLE", "Ultra-Freezer Chest-Type (-86 Degrees and -150 Degrees)")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "4"), ("TED-TITLE", "Accessories")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2-3")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "4")]),
        ],
    },
];
/// Tender 8433088: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_8433088: &[(i64, &str, Option<&str>)] = &[
    (20739611, "RES-1", Some("LOT-1")),
    (20739611, "RES-2", Some("LOT-2")),
    (20739611, "RES-3", Some("LOT-3")),
];
/// Tender 8433088: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_8433088: &[(i64, &str, Option<&str>, Option<&str>)] = &[];
/// Tender 8433088: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_8433088: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-2", Some("2-3")),
    ("LOT-3", Some("4")),
];
/// Tender 8433088: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_8433088: &[(i64, &str)] = &[
    (20329681, "anchor"),
    (20739611, "refused:silent-merge"),
];

/// Tender 6016286: notices 19455484, 19824266.
pub(super) const CHAIN_6016286: &[Notice] = &[
    Notice {
        id: 19455484,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 19824266,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2.")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3.")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4.")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5.")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6.")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7.")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1.")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2.")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3.")]),
            ("RES-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4.")]),
            ("RES-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5.")]),
            ("RES-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6.")]),
            ("RES-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7.")]),
        ],
    },
];
/// Tender 6016286: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_6016286: &[(i64, &str, Option<&str>)] = &[
    (19824266, "RES-1", None),
    (19824266, "RES-2", Some("LOT-2")),
    (19824266, "RES-3", Some("LOT-3")),
    (19824266, "RES-4", Some("LOT-4")),
    (19824266, "RES-5", Some("LOT-5")),
    (19824266, "RES-6", Some("LOT-6")),
    (19824266, "RES-7", Some("LOT-7")),
];
/// Tender 6016286: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_6016286: &[(i64, &str, Option<&str>, Option<&str>)] = &[];
/// Tender 6016286: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_6016286: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-2", Some("2.")),
    ("LOT-3", Some("3.")),
    ("LOT-4", Some("4.")),
    ("LOT-5", Some("5.")),
    ("LOT-6", Some("6.")),
    ("LOT-7", Some("7.")),
];
/// Tender 6016286: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_6016286: &[(i64, &str)] = &[
    (19455484, "anchor"),
    (19824266, "refused:silent-merge"),
];

/// Tender 7006083: notices 21534520, 21943329.
pub(super) const CHAIN_7006083: &[Notice] = &[
    Notice {
        id: 21534520,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("LOT-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "13")]),
            ("LOT-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "6")]),
            ("LOT-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "4")]),
            ("LOT-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "8")]),
            ("LOT-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "3")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "14")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "5")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "2")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "7")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "10")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "12")]),
            ("LOT-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "9")]),
            ("LOT-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "11")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 21943329,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("LOT-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "13")]),
            ("LOT-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "6")]),
            ("LOT-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "4")]),
            ("LOT-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "8")]),
            ("LOT-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "3")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "14")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "5")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "2")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "7")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "10")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "12")]),
            ("LOT-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "9")]),
            ("LOT-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "11")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "14")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "12")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "11")]),
            ("RES-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "2")]),
            ("RES-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "8")]),
            ("RES-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "1")]),
            ("RES-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "10")]),
            ("RES-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "4")]),
            ("RES-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "6")]),
        ],
    },
];
/// Tender 7006083: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_7006083: &[(i64, &str, Option<&str>)] = &[
    (21943329, "RES-1", Some("LOT-2")),
    (21943329, "RES-2", Some("LOT-7")),
    (21943329, "RES-3", Some("LOT-9")),
    (21943329, "RES-4", Some("LOT-4")),
    (21943329, "RES-5", Some("LOT-13")),
    (21943329, "RES-6", Some("LOT-1")),
    (21943329, "RES-7", Some("LOT-6")),
    (21943329, "RES-8", Some("LOT-12")),
    (21943329, "RES-9", Some("LOT-11")),
];
/// Tender 7006083: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_7006083: &[(i64, &str, Option<&str>, Option<&str>)] = &[];
/// Tender 7006083: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_7006083: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-10", Some("13")),
    ("LOT-11", Some("6")),
    ("LOT-12", Some("4")),
    ("LOT-13", Some("8")),
    ("LOT-14", Some("3")),
    ("LOT-2", Some("14")),
    ("LOT-3", Some("5")),
    ("LOT-4", Some("2")),
    ("LOT-5", Some("7")),
    ("LOT-6", Some("10")),
    ("LOT-7", Some("12")),
    ("LOT-8", Some("9")),
    ("LOT-9", Some("11")),
];
/// Tender 7006083: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_7006083: &[(i64, &str)] = &[
    (21534520, "anchor"),
    (21943329, "identity"),
];

/// Tender 8375276: notices 18679338, 19786640, 21337435 (reduced from the full chain of 13).
pub(super) const CHAIN_8375276: &[Notice] = &[
    Notice {
        id: 18679338,
        profile: "ted-export-r208",
        lots: &[
            ("LOT-1", &[("TED-LOT_NUMBER", "1")]),
            ("LOT-2", &[("TED-LOT_NUMBER", "2")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 19786640,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "2"), ("TED-TITLE", "Kojenecká voda")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "1"), ("TED-TITLE", "Lahve a savičky")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "2")]),
            ("RES-10", &[("TED-ITEM", "10"), ("TED-LOT_NO", "1")]),
            ("RES-11", &[("TED-ITEM", "11"), ("TED-LOT_NO", "1")]),
            ("RES-12", &[("TED-ITEM", "12"), ("TED-LOT_NO", "1")]),
            ("RES-13", &[("TED-ITEM", "13"), ("TED-LOT_NO", "1")]),
            ("RES-14", &[("TED-ITEM", "14"), ("TED-LOT_NO", "1")]),
            ("RES-15", &[("TED-ITEM", "15"), ("TED-LOT_NO", "1")]),
            ("RES-16", &[("TED-ITEM", "16"), ("TED-LOT_NO", "1")]),
            ("RES-17", &[("TED-ITEM", "17"), ("TED-LOT_NO", "1")]),
            ("RES-18", &[("TED-ITEM", "18"), ("TED-LOT_NO", "1")]),
            ("RES-19", &[("TED-ITEM", "19"), ("TED-LOT_NO", "1")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "2")]),
            ("RES-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "1")]),
            ("RES-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "1")]),
            ("RES-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "1")]),
            ("RES-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "1")]),
            ("RES-8", &[("TED-ITEM", "8"), ("TED-LOT_NO", "1")]),
            ("RES-9", &[("TED-ITEM", "9"), ("TED-LOT_NO", "1")]),
        ],
    },
    Notice {
        id: 21337435,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1"), ("TED-TITLE", "Lahve a savičky")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2"), ("TED-TITLE", "Kojenecká voda")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
        ],
    },
];
/// Tender 8375276: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_8375276: &[(i64, &str, Option<&str>)] = &[
    (19786640, "RES-1", Some("LOT-2")),
    (19786640, "RES-10", Some("LOT-1")),
    (19786640, "RES-11", Some("LOT-1")),
    (19786640, "RES-12", Some("LOT-1")),
    (19786640, "RES-13", Some("LOT-1")),
    (19786640, "RES-14", Some("LOT-1")),
    (19786640, "RES-15", Some("LOT-1")),
    (19786640, "RES-16", Some("LOT-1")),
    (19786640, "RES-17", Some("LOT-1")),
    (19786640, "RES-18", Some("LOT-1")),
    (19786640, "RES-19", Some("LOT-1")),
    (19786640, "RES-2", Some("LOT-2")),
    (19786640, "RES-3", Some("LOT-2")),
    (19786640, "RES-4", Some("LOT-1")),
    (19786640, "RES-5", Some("LOT-1")),
    (19786640, "RES-6", Some("LOT-1")),
    (19786640, "RES-7", Some("LOT-1")),
    (19786640, "RES-8", Some("LOT-1")),
    (19786640, "RES-9", Some("LOT-1")),
    (21337435, "RES-1", Some("LOT-1")),
];
/// Tender 8375276: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_8375276: &[(i64, &str, Option<&str>, Option<&str>)] = &[
    (19786640, "RES-1", Some("LOT-1"), Some("LOT-2")),
    (19786640, "RES-10", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-11", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-12", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-13", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-14", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-15", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-16", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-17", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-18", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-19", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-2", Some("LOT-1"), Some("LOT-2")),
    (19786640, "RES-3", Some("LOT-1"), Some("LOT-2")),
    (19786640, "RES-4", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-5", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-6", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-7", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-8", Some("LOT-2"), Some("LOT-1")),
    (19786640, "RES-9", Some("LOT-2"), Some("LOT-1")),
];
/// Tender 8375276: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_8375276: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-2", Some("2")),
];
/// Tender 8375276: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_8375276: &[(i64, &str)] = &[
    (18679338, "anchor"),
    (19786640, "remapped"),
    (21337435, "identity"),
];

/// Tender 7112657: notices 21736179, 21809387, 21853373, 22462819.
pub(super) const CHAIN_7112657: &[Notice] = &[
    Notice {
        id: 21736179,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 21809387,
        profile: "ted-export-r209",
        lots: &[],
        blocks: &[],
    },
    Notice {
        id: 21853373,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 22462819,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4")]),
            ("LOT-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5")]),
            ("LOT-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6")]),
            ("LOT-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3")]),
            ("RES-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4")]),
            ("RES-5", &[("TED-ITEM", "5"), ("TED-LOT_NO", "5")]),
            ("RES-6", &[("TED-ITEM", "6"), ("TED-LOT_NO", "6")]),
            ("RES-7", &[("TED-ITEM", "7"), ("TED-LOT_NO", "7")]),
        ],
    },
];
/// Tender 7112657: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_7112657: &[(i64, &str, Option<&str>)] = &[
    (22462819, "RES-1", Some("LOT-1")),
    (22462819, "RES-2", Some("LOT-2")),
    (22462819, "RES-3", Some("LOT-3")),
    (22462819, "RES-4", Some("LOT-4")),
    (22462819, "RES-5", Some("LOT-5")),
    (22462819, "RES-6", Some("LOT-6")),
    (22462819, "RES-7", Some("LOT-7")),
];
/// Tender 7112657: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_7112657: &[(i64, &str, Option<&str>, Option<&str>)] = &[];
/// Tender 7112657: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_7112657: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-2", Some("2")),
    ("LOT-3", Some("3")),
    ("LOT-4", Some("4")),
    ("LOT-5", Some("5")),
    ("LOT-6", Some("6")),
    ("LOT-7", Some("7")),
];
/// Tender 7112657: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_7112657: &[(i64, &str)] = &[
    (21736179, "anchor"),
    (21853373, "identity"),
    (22462819, "identity"),
];

/// Tender 7465548: notices 22589691, 22646614, 22615262, 22917962, 23122442, 23461598.
pub(super) const CHAIN_7465548: &[Notice] = &[
    Notice {
        id: 22589691,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 22646614,
        profile: "ted-export-r209",
        lots: &[],
        blocks: &[],
    },
    Notice {
        id: 22615262,
        profile: "ted-export-r209",
        lots: &[],
        blocks: &[],
    },
    Notice {
        id: 22917962,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1")]),
        ],
    },
    Notice {
        id: 23122442,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
        ],
    },
    Notice {
        id: 23461598,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("LOT-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("LOT-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3")]),
            ("LOT-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "1")]),
            ("RES-2", &[("TED-ITEM", "2"), ("TED-LOT_NO", "2")]),
            ("RES-3", &[("TED-ITEM", "3"), ("TED-LOT_NO", "3")]),
            ("RES-4", &[("TED-ITEM", "4"), ("TED-LOT_NO", "4")]),
        ],
    },
];
/// Tender 7465548: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_7465548: &[(i64, &str, Option<&str>)] = &[
    (22917962, "RES-1", Some("LOT-1")),
    (23122442, "RES-1", Some("LOT-1")),
    (23461598, "RES-1", Some("LOT-1")),
    (23461598, "RES-2", Some("LOT-2")),
    (23461598, "RES-3", Some("LOT-3")),
    (23461598, "RES-4", Some("LOT-4")),
];
/// Tender 7465548: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_7465548: &[(i64, &str, Option<&str>, Option<&str>)] = &[];
/// Tender 7465548: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_7465548: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("1")),
    ("LOT-2", Some("2")),
    ("LOT-3", Some("3")),
    ("LOT-4", Some("4")),
];
/// Tender 7465548: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_7465548: &[(i64, &str)] = &[
    (22589691, "anchor"),
    (22917962, "no-numbers"),
    (23122442, "no-numbers"),
    (23461598, "no-numbers"),
];

/// Tender 7025760: notices 21594444, 21687822.
pub(super) const CHAIN_7025760: &[Notice] = &[
    Notice {
        id: 21594444,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1")]),
        ],
        blocks: &[],
    },
    Notice {
        id: 21687822,
        profile: "ted-export-r209",
        lots: &[
            ("LOT-1", &[("TED-ITEM", "1"), ("TED-LOT_NO", "2")]),
        ],
        blocks: &[
            ("RES-1", &[("TED-ITEM", "1")]),
        ],
    },
];
/// Tender 7025760: every award block's lot under the port's option B, `(notice, block, lot)`.
pub(super) const PORT_B_7025760: &[(i64, &str, Option<&str>)] = &[
    (21687822, "RES-1", None),
];
/// Tender 7025760: the blocks option B moves, `(notice, block, today's lot, B's lot)`.
pub(super) const PORT_MOVED_7025760: &[(i64, &str, Option<&str>, Option<&str>)] = &[];
/// Tender 7025760: the head's lots under option B, `(key, printed number)`.
pub(super) const PORT_HEAD_B_7025760: &[(&str, Option<&str>)] = &[
    ("LOT-1", Some("2")),
];
/// Tender 7025760: each lot-publishing notice's outcome under option B.
pub(super) const PORT_OUTCOMES_7025760: &[(i64, &str)] = &[
    (21594444, "anchor"),
    (21687822, "no-numbers"),
];
