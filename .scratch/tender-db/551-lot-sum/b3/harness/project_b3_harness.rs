//! Issue 551 B3: TEMPORARY projection-parity harness (removed after the run; the record copy lives in
//! `.scratch/tender-db/551-lot-sum/b3/harness/`). Included from project.rs by three lines before its
//! `#[cfg(test)] mod tests`:
//!
//!     #[cfg(test)]
//!     #[path = "project_b3_harness.rs"]
//!     mod b3_harness;
//!
//! B1's REAL projection fed the parsed notices: for each Tender of `B3_PROJ_IN` (one JSON object per line, from
//! `b3/export.py`: the chain's notices in fold order, each with an inferred profile and the path of its
//! `/v1/notices/{id}/content`), every notice's content is turned back into the parse layer (`Parsed`: sections in
//! published order, values sorted as the store returns them, each JSON value to its `NoticeValue`), the chunk goes
//! through `normalise_de1`, each notice through `NoticeState::read` and `bind_organizations` (every Organization
//! section bound to a synthetic organization, so a block's winners are present), and the chain through `fold`,
//! exactly as `apply_plan_batch` does. `B3_PROJ_OUT` gets one JSON object per Tender: per version, its lots
//! (key, kind, published_lot_no); at the head, every round's lot results (notice, key, lot_key, published_lot_no,
//! group, award) and the stated totals (notice, logical id, field, kind, cents, currency, lots_awarded).
use super::{NoticeState, ORGANIZATION_KIND, fold, normalise_de1};
use std::collections::HashMap;
use store::{NoticeValue, Parsed, StatedTotalKind};

/// Days since 1970-01-01 of a civil date (Howard Hinnant's days_from_civil).
fn days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The content endpoint's date (`stamp`): `YYYY-MM-DD` (no time) or RFC 3339 with seconds and an offset.
fn date_value(s: &str) -> NoticeValue {
    let n = |a: usize, b: usize| s.get(a..b).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    let day = days(n(0, 4), n(5, 7), n(8, 10));
    if s.len() <= 10 {
        return NoticeValue::Date { utc_seconds: day * 86_400, offset_minutes: 0, has_time: false };
    }
    let local = day * 86_400 + n(11, 13) * 3600 + n(14, 16) * 60 + n(17, 19);
    let tz = &s[19..];
    let offset = if tz == "Z" || tz.is_empty() {
        0
    } else {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        sign * (tz[1..3].parse::<i64>().unwrap_or(0) * 60 + tz[4..6].parse::<i64>().unwrap_or(0))
    };
    NoticeValue::Date { utc_seconds: local - offset * 60, offset_minutes: offset, has_time: true }
}

fn value_of(v: &serde_json::Value) -> NoticeValue {
    let st = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_owned);
    match v["type"].as_str().expect("type") {
        "text" => NoticeValue::Text { lang: st("lang"), value: st("value").unwrap_or_default() },
        "code" => NoticeValue::Code { list: st("list"), code: st("code").unwrap_or_default() },
        "classification" => NoticeValue::Classification { scheme: st("scheme").unwrap_or_default(), code: st("code").unwrap_or_default() },
        "amount" => NoticeValue::Amount { cents: v["cents"].as_i64().expect("cents"), currency: st("currency").unwrap_or_default() },
        "date" => date_value(v["value"].as_str().unwrap_or("1970-01-01")),
        "integer" => NoticeValue::Integer(v["value"].as_i64().expect("integer")),
        "number" => NoticeValue::Number { value: v["value"].as_f64().expect("number"), unit: st("unit") },
        "id" => NoticeValue::Id { scheme: st("scheme"), value: st("value").unwrap_or_default(), is_ref: v["is_ref"].as_bool().unwrap_or(false) },
        other => panic!("value type {other}"),
    }
}

fn parsed_of(content: &serde_json::Value) -> Parsed {
    let mut sections = Vec::new();
    let mut values = Vec::new();
    for s in content["sections"].as_array().expect("sections") {
        let id = s["section_id"].as_str().expect("section_id").to_owned();
        sections.push(store::Section {
            id: id.clone(),
            kind: s["kind"].as_str().expect("kind").to_owned(),
            parent: s["parent_section_id"].as_str().map(str::to_owned),
        });
        for v in s["values"].as_array().expect("values") {
            values.push(store::ValueRow {
                section_id: id.clone(),
                field_id: v["field_id"].as_str().expect("field_id").to_owned(),
                ordinal: v["ordinal"].as_i64().unwrap_or(0),
                value: value_of(v),
            });
        }
    }
    // The store's order (`parsed_by_ids`): section id as text, field, ordinal.
    values.sort_by(|a, b| (&a.section_id, &a.field_id, a.ordinal).cmp(&(&b.section_id, &b.field_id, b.ordinal)));
    Parsed { sections, values }
}

fn kind_name(k: StatedTotalKind) -> &'static str {
    match k {
        StatedTotalKind::Award => "Award",
        StatedTotalKind::Estimate => "Estimate",
        StatedTotalKind::Range => "Range",
    }
}

#[test]
#[ignore = "issue 551 B3 projection-parity harness: reads B3_PROJ_IN, writes B3_PROJ_OUT"]
fn b3_projection() {
    let input = std::env::var("B3_PROJ_IN").expect("B3_PROJ_IN");
    let output = std::env::var("B3_PROJ_OUT").expect("B3_PROJ_OUT");
    let mut out = String::new();
    let mut tenders = 0usize;
    for line in std::fs::read_to_string(&input).expect("read B3_PROJ_IN").lines() {
        let t: serde_json::Value = serde_json::from_str(line).expect("json");
        let source = t["source"].as_str().unwrap_or("ted").to_owned();
        let mut chunk: Vec<(store::NoticeRef, Parsed)> = Vec::new();
        for n in t["notices"].as_array().expect("notices") {
            let content: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(n["path"].as_str().expect("path")).expect("content")).expect("content json");
            chunk.push((
                store::NoticeRef {
                    id: n["id"].as_i64().expect("id"),
                    // the notice's own source (export.py: `ted` for a TED/text notice, else the Tender's)
                    source: n["source"].as_str().map_or_else(|| source.clone(), str::to_owned),
                    publication_id: n["publication_id"].as_str().unwrap_or("").to_owned(),
                    profile: n["profile"].as_str().expect("profile").to_owned(),
                },
                parsed_of(&content),
            ));
        }
        normalise_de1(&mut chunk);
        let mut next_org = 1_000_000i64;
        let states: Vec<NoticeState> = chunk
            .iter()
            .map(|(notice, parsed)| {
                let mut state = NoticeState::read(notice, parsed);
                let orgs: HashMap<String, i64> = parsed
                    .sections
                    .iter()
                    .filter(|s| s.kind == ORGANIZATION_KIND)
                    .map(|s| {
                        next_org += 1;
                        (s.id.clone(), next_org)
                    })
                    .collect();
                state.bind_organizations(&orgs);
                state
            })
            .collect();
        let refs: Vec<&NoticeState> = states.iter().collect();
        let versions = fold(&refs);
        let lots_by_version: serde_json::Map<String, serde_json::Value> = versions
            .iter()
            .map(|v| {
                (
                    v.caused_by_notice_id.to_string(),
                    serde_json::Value::Array(
                        v.lots.iter().map(|l| serde_json::json!([l.key, l.kind, l.published_lot_no])).collect(),
                    ),
                )
            })
            .collect();
        let head = versions.last();
        let results: Vec<serde_json::Value> = head
            .map(|h| {
                h.rounds
                    .iter()
                    .flat_map(|r| {
                        r.lot_results.iter().map(move |x| {
                            serde_json::json!([r.notice_id, x.key, x.lot_key, x.published_lot_no, x.group, x.awarded_cents, x.awarded_currency, x.winners.len()])
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let totals: Vec<serde_json::Value> = head
            .map(|h| {
                h.stated_totals
                    .iter()
                    .map(|s| serde_json::json!([s.notice_id, s.logical_notice_id, s.field, kind_name(s.kind), s.cents, s.currency, s.lots_awarded]))
                    .collect()
            })
            .unwrap_or_default();
        out.push_str(
            &serde_json::json!({
                "id": t["id"],
                "versions": versions.iter().map(|v| v.caused_by_notice_id).collect::<Vec<_>>(),
                "lots_by_version": lots_by_version,
                "results": results,
                "totals": totals,
            })
            .to_string(),
        );
        out.push('\n');
        tenders += 1;
    }
    std::fs::write(&output, out).expect("write B3_PROJ_OUT");
    eprintln!("B3 PROJECTION HARNESS: {tenders} Tenders folded from {input} -> {output}");
}
