    // PORT-CHECK-551-BEGIN (temporary harness, removed after the run)
    #[test]
    #[ignore = "issue 551 port check: reads PORT_CHECK_551_IN, writes PORT_CHECK_551_OUT"]
    fn port_check_551() {
        use super::{DerivationPath, LotResultState, Round, head_value};
        let input = std::env::var("PORT_CHECK_551_IN").expect("PORT_CHECK_551_IN");
        let output = std::env::var("PORT_CHECK_551_OUT").expect("PORT_CHECK_551_OUT");
        fn dec(s: &str) -> Option<String> {
            if s == "-" {
                return None;
            }
            let b = s.as_bytes();
            let mut out = Vec::new();
            let mut i = 0;
            while i < b.len() {
                if b[i] == b'%' {
                    out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
                    i += 3;
                } else {
                    out.push(b[i]);
                    i += 1;
                }
            }
            Some(String::from_utf8(out).unwrap())
        }
        let amount = |p: &[&str]| Fact::Amount {
            field: dec(p[1]).unwrap(),
            cents: p[2].parse().unwrap(),
            currency: dec(p[3]).unwrap(),
            tax_basis: None,
            quality: dec(p[4]),
        };
        let text = std::fs::read_to_string(input).unwrap();
        let date = crate::rates::civil_date(0);
        let mut out = String::new();
        let mut id = 0i64;
        let mut rates = crate::rates::RatesLookup::default();
        let mut chain: Vec<TenderVersion> = Vec::new();
        for line in text.lines() {
            let p: Vec<&str> = line.split('\t').collect();
            match p[0] {
                "T" => {
                    id = p[1].parse().unwrap();
                    rates = crate::rates::RatesLookup::default();
                    chain.clear();
                }
                "R" => rates.insert_for_test(crate::rates::canonical_currency(p[1]), &date, p[2].parse().unwrap()),
                "V" => {
                    let mut v = head(Vec::new(), Vec::new());
                    v.lots.clear();
                    v.caused_by_notice_id = chain.len() as i64 + 1;
                    chain.push(v);
                }
                "F" => {
                    chain.last_mut().unwrap().facts.insert(amount(&p));
                }
                "L" => chain.last_mut().unwrap().lots.push(LotState { key: dec(p[1]).unwrap(), kind: dec(p[2]).unwrap(), facts: Default::default() }),
                "LF" => {
                    chain.last_mut().unwrap().lots.last_mut().unwrap().facts.insert(amount(&p));
                }
                "N" => chain.last_mut().unwrap().rounds.push(Round {
                    notice_id: p[1].parse().unwrap(),
                    logical_notice_id: None,
                    lot_results: Vec::new(),
                    bids: Vec::new(),
                    contracts: Vec::new(),
                }),
                "A" => chain.last_mut().unwrap().rounds.last_mut().unwrap().lot_results.push(LotResultState {
                    key: dec(p[1]).unwrap(),
                    lot_key: dec(p[2]),
                    decision: None,
                    reason: None,
                    awarded_cents: if p[3] == "-" { None } else { Some(p[3].parse().unwrap()) },
                    awarded_currency: dec(p[4]),
                    decided: None,
                    winners: Vec::new(),
                    buyer_winners: Vec::new(),
                    statistics: Vec::new(),
                }),
                "E" => {
                    let v = head_value(&chain, &rates);
                    let today = super::max_election_eur_cents(&chain, &rates);
                    let eur = v.eur_cents.map_or("-".to_owned(), |e| e.to_string());
                    let today = today.map_or("-".to_owned(), |e| e.to_string());
                    let derived = match &v.derived {
                        None => "-".to_owned(),
                        Some(d) => {
                            let path = match &d.path {
                                DerivationPath::Vouched { rule, field, cents, currency, exponent } => {
                                    format!("lot-sum-vouched\t{}\t{exponent}\t{field}\t{cents}\t{currency}", rule.issue())
                                }
                                DerivationPath::Awards => "lot-sum-awards\t-\t-\t-\t-\t-".to_owned(),
                            };
                            format!("{path}\t{}\t{}\t{}\t{}", d.kind.label(), d.cents, d.currency, d.lots)
                        }
                    };
                    out.push_str(&format!("{id}\t{today}\t{eur}\t{derived}\n"));
                }
                other => panic!("unknown line kind {other}"),
            }
        }
        std::fs::write(output, out).unwrap();
    }
    // PORT-CHECK-551-END
