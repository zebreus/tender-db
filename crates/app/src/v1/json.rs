//! The JSON contract: how canonical rows appear to an external client.
//!
//! Three rules, applied everywhere (CONTEXT.md, docs/architecture.md):
//! timestamps are ISO 8601 with the offset the source published (never
//! normalised away — the buyer's local wall-clock deadline is the meaningful
//! one); money is `{cents, currency}`, never a float; and the change cursor is
//! an opaque *string*, so no client is tempted to do arithmetic on it
//! (CouchDB's lesson, docs/research/api-layer.md §3).

use chrono::{DateTime, FixedOffset, SecondsFormat, TimeZone};
use serde_json::{Value, json};
use store::read::{
    BidRow, ContractRow, FactRow, LotResultRow, LotRow, NoticeRow, OrganizationRow, PartyRow,
    QuarantineRow, ResultOrgRow, Stamp, TenderDetail, TenderRow, VersionRow,
};

/// A UTC instant as ISO 8601, e.g. `2026-07-19T09:30:00Z`.
pub fn instant(utc_seconds: i64) -> Value {
    match DateTime::from_timestamp(utc_seconds, 0) {
        Some(dt) => json!(dt.to_rfc3339_opts(SecondsFormat::Secs, true)),
        None => Value::Null,
    }
}

/// A stored timestamp rendered in its published offset: `has_time = false`
/// means the source gave a date only, and inventing a time would be a lie, so
/// the response carries the date alone.
pub fn stamp(s: Option<Stamp>) -> Value {
    let Some(s) = s else { return Value::Null };
    let Some(offset) = FixedOffset::east_opt((s.offset_minutes * 60) as i32) else {
        return instant(s.utc_seconds);
    };
    let Some(local) = offset.timestamp_opt(s.utc_seconds, 0).single() else {
        return instant(s.utc_seconds);
    };
    if s.has_time {
        json!(local.to_rfc3339_opts(SecondsFormat::Secs, false))
    } else {
        json!(local.date_naive().to_string())
    }
}

/// A publication or dispatch instant (issue 367 unit 3): in the offset and
/// precision the source published it with once the notice row carries them —
/// a date-only publication is then the date it stated, `2026-09-05` rather
/// than `2026-09-04T22:00:00Z` — and the bare UTC instant on a row stamped
/// before that pair existed (`repair-notice-instants` fills those in).
fn published(stored: Option<Stamp>, utc_seconds: i64) -> Value {
    match stored {
        Some(s) => stamp(Some(s)),
        None => instant(utc_seconds),
    }
}

fn published_opt(stored: Option<Stamp>, utc_seconds: Option<i64>) -> Value {
    match (stored, utc_seconds) {
        (Some(s), _) => stamp(Some(s)),
        (None, Some(utc)) => instant(utc),
        (None, None) => Value::Null,
    }
}

/// Money is always `{cents, currency}` — integer minor units plus the code, the
/// representation the canonical layer stores and the only one that survives
/// round-tripping.
pub fn money(cents: Option<i64>, currency: Option<&str>) -> Value {
    match (cents, currency) {
        (Some(cents), Some(currency)) => json!({ "cents": cents, "currency": currency }),
        _ => Value::Null,
    }
}

/// A cursor as clients see it: an opaque string.
pub fn cursor(value: i64) -> Value {
    json!(value.to_string())
}

pub fn tender(t: &TenderRow) -> Value {
    json!({
        "id": t.id,
        "source": t.source,
        "procedure_key": t.procedure_key,
        "kind": t.kind,
        "title": t.title,
        "version": t.seq,
        "published_at": published(t.published, t.published_at),
        "dispatched_at": published_opt(t.dispatched, t.dispatched_at),
        "publication_id": t.publication_id,
        "notice_subtype": t.notice_subtype,
        "original_lang": t.original_lang,
        "value": money(t.value_cents, t.currency.as_deref()),
        "submission_deadline": stamp(t.deadline),
        "lots": t.lots,
        // Echo the fields a client can filter on, so a list row shows why it
        // matched (issue 49): CPV codes and NUTS place codes of this version.
        "cpv": t.cpv,
        "country": t.country,
    })
}

pub fn lot(l: &LotRow) -> Value {
    json!({
        "id": l.id,
        "tender_id": l.tender_id,
        "lot_key": l.lot_key,
        "kind": l.kind,
        "title": l.title,
        "version": l.seq,
        "value": money(l.value_cents, l.currency.as_deref()),
        "submission_deadline": stamp(l.deadline),
    })
}

pub fn organization(o: &OrganizationRow) -> Value {
    json!({
        "id": o.id,
        "name": o.name,
        "country": o.country,
        "identifier_kind": o.identifier_kind,
        "identifier": o.identifier,
        // A provisional profile has NO official identifier, and nothing more is
        // implied: its identity is name-scoped, so mentions sharing a normalised
        // name and country land on one row and it may hold many (234, 351, 370).
        "provisional": o.provisional,
        "mentions": o.mentions,
    })
}

pub fn notice(n: &NoticeRow) -> Value {
    json!({
        "id": n.id,
        "source": n.source,
        "publication_id": n.publication_id,
        "content_hash": n.content_hash,
        "profile": n.profile,
        "declared_version": n.declared_version,
        "member_path": n.member_path,
        "ingested_at": instant(n.ingested_at),
        "published_at": published_opt(n.published, n.published_at),
        "dispatched_at": published_opt(n.dispatched, n.dispatched_at),
        "parse_state": n.parse_state,
    })
}

/// The single-notice detail: the identity [`notice`] returns, plus `quarantine`
/// (issue 218). For a notice held TODAY this is its only content — a still-held
/// notice has no parsed satellites and no canonical tender — so a consumer learns
/// why it is absent instead of receiving a bare `parse_state` stub.
///
/// But the field is the hold HISTORY, not a held-today flag (issue 398): the
/// ledger keeps the row after a reclaim, and reclaimed is the majority outcome, so
/// most notices carrying a non-null `quarantine` parsed and are fully served.
/// `null` means the notice was never held; `parse_state` answers "is it held now".
/// See [`store::read::notice_quarantine`] for the measurements.
///
/// The list endpoint keeps the lean [`notice`] shape; only this by-id path pays
/// the extra `(notice_id)` lookup.
pub fn notice_detail(n: &NoticeRow, q: Option<&QuarantineRow>) -> Value {
    let mut base = notice(n);
    base["quarantine"] = q.map(quarantine).unwrap_or(Value::Null);
    base
}

/// A notice's whole parsed payload (issue 218-B): the section tree and every
/// typed value, grouped by section in published-section order, values in
/// `(field_id, ordinal)` order within their type. Field ids stay in the SOURCE's
/// own vocabulary (eForms BT/OPT ids, TED export field ids…) — this is the parse
/// layer verbatim, not the canonical projection; `profile` on the notice detail
/// names which vocabulary to read them in.
pub fn notice_content(notice_id: i64, parsed: &store::Parsed) -> Value {
    let value_json = |v: &store::NoticeValue| -> Value {
        match v {
            store::NoticeValue::Text { lang, value } => {
                json!({ "type": "text", "lang": lang, "value": value })
            }
            store::NoticeValue::Code { list, code } => {
                json!({ "type": "code", "list": list, "code": code })
            }
            store::NoticeValue::Classification { scheme, code } => {
                json!({ "type": "classification", "scheme": scheme, "code": code })
            }
            store::NoticeValue::Amount { cents, currency } => {
                json!({ "type": "amount", "cents": cents, "currency": currency })
            }
            store::NoticeValue::Date { utc_seconds, offset_minutes, has_time } => {
                json!({
                    "type": "date",
                    "value": stamp(Some(store::read::Stamp {
                        utc_seconds: *utc_seconds,
                        offset_minutes: *offset_minutes,
                        has_time: *has_time,
                    })),
                })
            }
            store::NoticeValue::Integer(value) => json!({ "type": "integer", "value": value }),
            store::NoticeValue::Number { value, unit } => {
                json!({ "type": "number", "value": value, "unit": unit })
            }
            store::NoticeValue::Id { scheme, value, is_ref } => {
                json!({ "type": "id", "scheme": scheme, "value": value, "is_ref": is_ref })
            }
        }
    };
    let sections: Vec<Value> = parsed
        .sections
        .iter()
        .map(|s| {
            let mut values: Vec<&store::ValueRow> =
                parsed.values.iter().filter(|v| v.section_id == s.id).collect();
            values.sort_by(|a, b| (&a.field_id, a.ordinal).cmp(&(&b.field_id, b.ordinal)));
            json!({
                "section_id": s.id,
                "kind": s.kind,
                "parent_section_id": s.parent,
                "values": values
                    .iter()
                    .map(|v| {
                        let mut o = value_json(&v.value);
                        let map = o.as_object_mut().expect("value objects");
                        map.insert("field_id".into(), json!(v.field_id));
                        map.insert("ordinal".into(), json!(v.ordinal));
                        o
                    })
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "notice_id": notice_id, "sections": sections })
}

/// The quarantine hold, as a client sees it. Timestamps are ISO 8601 like every
/// other instant in the contract; `first_reason`/`first_detail` appear only when a
/// re-attempt overwrote the original cause (issue 87), and the terminal stamps say
/// which of the three outcomes the member reached (outstanding / reclaimed /
/// skipped-by-policy, issue 84).
pub fn quarantine(q: &QuarantineRow) -> Value {
    json!({
        "reason": q.reason,
        "detail": q.detail,
        "profile": q.profile,
        "first_seen": instant(q.first_seen),
        "attempts": q.attempts,
        "last_attempt_at": q.last_attempt_at.map(instant).unwrap_or(Value::Null),
        "reprocessed_at": q.reprocessed_at.map(instant).unwrap_or(Value::Null),
        "skipped_at": q.skipped_at.map(instant).unwrap_or(Value::Null),
        "skipped_reason": q.skipped_reason,
        "first_reason": q.first_reason,
        "first_detail": q.first_detail,
    })
}

fn fact(f: &FactRow) -> Value {
    let mut object = json!({ "lot": f.lot_key, "field": f.field });
    let map = object.as_object_mut().expect("just built as an object");
    if let Some(text) = &f.text {
        map.insert("lang".into(), json!(f.lang));
        map.insert("value".into(), json!(text));
    }
    if f.cents.is_some() {
        // Issue 372: a withheld figure is NOT published as a value. `cents` holds
        // the eForms SDK's -1 placeholder, so emitting it would keep asserting a
        // -0.01 contract where the notice said the value is suppressed -- which is
        // the whole defect. A consumer that ignores `quality` now reads "no
        // value", which is true; one that reads it learns why.
        match f.quality.as_deref() {
            Some(quality) => {
                map.insert("value".into(), Value::Null);
                map.insert("quality".into(), json!(quality));
            }
            None => {
                map.insert("value".into(), money(f.cents, f.currency.as_deref()));
            }
        }
    }
    if let Some(code) = &f.code {
        map.insert("scheme".into(), json!(f.scheme));
        map.insert("code".into(), json!(code));
    }
    if f.stamp.is_some() {
        map.insert("value".into(), stamp(f.stamp));
    }
    object
}

fn party(p: &PartyRow) -> Value {
    json!({
        "lot": p.lot_key,
        "role": p.role,
        "organization_id": p.organization_id,
        "organization_name": p.organization_name,
    })
}

fn result_org(o: &ResultOrgRow) -> Value {
    json!({
        "role": o.role,
        "organization_id": o.organization_id,
        "organization_name": o.organization_name,
    })
}

/// An award decision. `notice_id` + `key` name the origin evidence: results
/// accumulate across framework/DPS rounds, each round keyed by its notice.
fn lot_result(r: &LotResultRow) -> Value {
    json!({
        "notice_id": r.notice_id,
        "key": r.key,
        "lot": r.lot_key,
        "decision": r.decision,
        "reason": r.reason,
        "awarded": money(r.awarded_cents, r.awarded_currency.as_deref()),
        // When the buyer decided — the legacy eras' award-block date (issue 255).
        "decided": stamp(r.decided),
        "winners": r.winners.iter().map(result_org).collect::<Vec<_>>(),
        // Issue 372 unit 4: a withheld statistic is not a statistic. Its `kind` is
        // the literal `unpublished` and its count -1, so publishing the pair would
        // put a fake submission type in a map keyed BY type and assert -1 of them.
        // The map therefore holds only real readings, and the withheld ones are
        // reported as a count of what is missing — which is what the notice
        // actually says: "there were statistics here and they are suppressed".
        "statistics": r.statistics.iter()
            .filter(|(_, _, quality)| quality.is_none())
            .map(|(kind, count, _)| (kind.clone(), json!(count)))
            .collect::<serde_json::Map<_, _>>(),
        "statistics_withheld": r.statistics.iter().filter(|(_, _, q)| q.is_some()).count(),
    })
}

fn bid(b: &BidRow) -> Value {
    // Issue 372, same rule as [`fact`]: a bid whose BT-720 the notice withheld
    // reports no value rather than the SDK's -1 placeholder, and says why.
    let withheld = b.quality.is_some();
    json!({
        "notice_id": b.notice_id,
        "key": b.key,
        "lot": b.lot_key,
        "value": if withheld { Value::Null } else { money(b.cents, b.currency.as_deref()) },
        "quality": b.quality,
        "parties": b.parties.iter().map(result_org).collect::<Vec<_>>(),
    })
}

fn contract(c: &ContractRow) -> Value {
    json!({
        "notice_id": c.notice_id,
        "key": c.key,
        "buyer_contract_id": c.buyer_contract_id,
        "concluded": stamp(c.concluded),
        // BT-1451: when the buyer decided, as distinct from when the contract was
        // signed (issue 255).
        "decided": stamp(c.decided),
        "value": money(c.cents, c.currency.as_deref()),
    })
}

fn version(v: &VersionRow) -> Value {
    json!({
        "seq": v.seq,
        "published_at": published(v.published, v.published_at),
        "dispatched_at": published_opt(v.dispatched, v.dispatched_at),
        "publication_id": v.publication_id,
        "notice_subtype": v.notice_subtype,
        // ADR-0013 D3: the notice's own original language (ISO 639-2/T), the
        // third leg of the ?lang= fallback; null where the era never said.
        "original_lang": v.original_lang,
        // Every version names the Notice that caused it: the ADR-0001
        // traceability chain reaches the API surface.
        "caused_by_notice_id": v.caused_by_notice_id,
    })
}

pub fn detail(d: &TenderDetail) -> Value {
    let mut object = tender(&d.tender);
    let map = object.as_object_mut().expect("just built as an object");
    map.insert("texts".into(), json!(d.texts.iter().map(fact).collect::<Vec<_>>()));
    map.insert("amounts".into(), json!(d.amounts.iter().map(fact).collect::<Vec<_>>()));
    map.insert("dates".into(), json!(d.dates.iter().map(fact).collect::<Vec<_>>()));
    map.insert(
        "classifications".into(),
        json!(d.classifications.iter().map(fact).collect::<Vec<_>>()),
    );
    map.insert("parties".into(), json!(d.parties.iter().map(party).collect::<Vec<_>>()));
    map.insert("lot_details".into(), json!(d.lots.iter().map(lot).collect::<Vec<_>>()));
    map.insert("lot_results".into(), json!(d.lot_results.iter().map(lot_result).collect::<Vec<_>>()));
    map.insert("bids".into(), json!(d.bids.iter().map(bid).collect::<Vec<_>>()));
    map.insert("contracts".into(), json!(d.contracts.iter().map(contract).collect::<Vec<_>>()));
    map.insert("versions".into(), json!(d.versions.iter().map(version).collect::<Vec<_>>()));
    object
}

/// The list envelope. `next_cursor` is null exactly when `more` is false, so a
/// client can loop on either one.
///
/// `ignored_filters` names any query parameter the client sent that this collection
/// does not apply — the shared filter vocabulary is accepted on every path, but each
/// collection honours only the subset meaningful to it, and a dropped filter would
/// otherwise return an unfiltered page that looks filtered (issue 118). The field is
/// always present: an empty array is the honest "every filter you sent applied".
pub fn page(items: Vec<Value>, next: Option<String>, ignored: &[&str]) -> Value {
    json!({
        "items": items,
        "next_cursor": next,
        "more": next.is_some(),
        "ignored_filters": ignored,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount_fact(cents: i64, quality: Option<&str>) -> FactRow {
        FactRow {
            lot_key: None,
            field: "result_value".into(),
            lang: None,
            text: None,
            cents: Some(cents),
            currency: Some("EUR".into()),
            scheme: None,
            code: None,
            stamp: None,
            quality: quality.map(str::to_owned),
        }
    }

    /// Issue 372: the API must stop asserting a value the notice said it was not
    /// publishing. A withheld amount reports `value: null` and says why, so a
    /// consumer that never heard of `quality` reads "no value" — which is true —
    /// instead of a -0.01 contract. An ordinary amount is untouched.
    #[test]
    fn a_withheld_amount_reports_no_value_and_says_why() {
        let ordinary = fact(&amount_fact(1_234_500, None));
        assert_eq!(ordinary["value"]["cents"], json!(1_234_500));
        assert_eq!(ordinary["value"]["currency"], json!("EUR"));
        assert!(ordinary.get("quality").is_none(), "no marker, no key: {ordinary}");

        let withheld = fact(&amount_fact(-100, Some("withheld")));
        assert_eq!(withheld["value"], Value::Null, "the placeholder must not be published");
        assert_eq!(withheld["quality"], json!("withheld"));
        // The field is still named, so the reader learns WHICH value is missing
        // rather than the fact vanishing — the distinction option (b) exists for.
        assert_eq!(withheld["field"], json!("result_value"));
    }

    /// Issue 372 unit 4: a withheld statistic leaves the `statistics` map rather
    /// than sitting in it as `{"unpublished": -1}`.
    ///
    /// The map is keyed BY submission type, so publishing a withheld entry puts a
    /// fake type in the key position and −1 in the value — junk on both halves.
    /// Dropping it silently would be its own lie though (a reader could not tell a
    /// suppressed statistic from a notice that published none), so the count of
    /// what is missing is reported beside the map.
    #[test]
    fn a_withheld_statistic_leaves_the_map_and_is_counted_instead() {
        let result = LotResultRow {
            notice_id: 7,
            key: "RES-0001".into(),
            lot_key: Some("LOT-0001".into()),
            decision: Some("selec-w".into()),
            reason: None,
            awarded_cents: None,
            awarded_currency: None,
            decided: None,
            winners: Vec::new(),
            statistics: vec![
                ("tenders".into(), 4, None),
                ("unpublished".into(), -1, Some("withheld".into())),
            ],
        };
        let out = lot_result(&result);
        assert_eq!(out["statistics"]["tenders"], json!(4), "the real reading survives");
        assert!(
            out["statistics"].get("unpublished").is_none(),
            "the placeholder must not appear as a submission type: {out}",
        );
        assert_eq!(out["statistics_withheld"], json!(1), "but the reader is told one is missing");

        // No withholding, no phantom: the count is 0 rather than absent, so a
        // consumer can read the field unconditionally.
        let plain = LotResultRow { statistics: vec![("tenders".into(), 4, None)], ..result };
        assert_eq!(lot_result(&plain)["statistics_withheld"], json!(0));
    }

    /// The same rule on the bids satellite, where BT-720 lands (issue 372's
    /// second surface). A withheld bid must not read as a -0.01 offer.
    #[test]
    fn a_withheld_bid_reports_no_value_and_says_why() {
        let row = |cents: i64, quality: Option<&str>| BidRow {
            notice_id: 7,
            key: "TEN-0001".into(),
            lot_key: Some("LOT-0001".into()),
            cents: Some(cents),
            currency: Some("EUR".into()),
            quality: quality.map(str::to_owned),
            parties: Vec::new(),
        };

        let ordinary = bid(&row(500_000, None));
        assert_eq!(ordinary["value"]["cents"], json!(500_000));
        assert_eq!(ordinary["quality"], Value::Null);

        let withheld = bid(&row(-100, Some("withheld")));
        assert_eq!(withheld["value"], Value::Null);
        assert_eq!(withheld["quality"], json!("withheld"));
        assert_eq!(withheld["key"], json!("TEN-0001"), "the bid is still identified");
    }
}
