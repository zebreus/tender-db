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
        "published_at": instant(t.published_at),
        "dispatched_at": t.dispatched_at.map(instant).unwrap_or(Value::Null),
        "publication_id": t.publication_id,
        "notice_subtype": t.notice_subtype,
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
        // A provisional profile is one mention with no usable official
        // identifier — deliberately never merged with another (CONTEXT.md).
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
        "published_at": n.published_at.map(instant).unwrap_or(Value::Null),
        "dispatched_at": n.dispatched_at.map(instant).unwrap_or(Value::Null),
        "parse_state": n.parse_state,
    })
}

/// The single-notice detail: the identity [`notice`] returns, plus `quarantine`
/// (issue 218). For a held notice this is its ONLY content — a quarantined notice
/// has no parsed satellites and no canonical tender — so a consumer learns why it
/// is absent from the data instead of receiving a bare `parse_state` stub. `null`
/// when the notice parsed. The list endpoint keeps the lean [`notice`] shape; only
/// this by-id path pays the extra `(notice_id)` lookup.
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
        map.insert("value".into(), money(f.cents, f.currency.as_deref()));
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
        "winners": r.winners.iter().map(result_org).collect::<Vec<_>>(),
        "statistics": r.statistics.iter()
            .map(|(kind, count)| (kind.clone(), json!(count)))
            .collect::<serde_json::Map<_, _>>(),
    })
}

fn bid(b: &BidRow) -> Value {
    json!({
        "notice_id": b.notice_id,
        "key": b.key,
        "lot": b.lot_key,
        "value": money(b.cents, b.currency.as_deref()),
        "parties": b.parties.iter().map(result_org).collect::<Vec<_>>(),
    })
}

fn contract(c: &ContractRow) -> Value {
    json!({
        "notice_id": c.notice_id,
        "key": c.key,
        "buyer_contract_id": c.buyer_contract_id,
        "concluded": stamp(c.concluded),
        "value": money(c.cents, c.currency.as_deref()),
    })
}

fn version(v: &VersionRow) -> Value {
    json!({
        "seq": v.seq,
        "published_at": instant(v.published_at),
        "dispatched_at": v.dispatched_at.map(instant).unwrap_or(Value::Null),
        "publication_id": v.publication_id,
        "notice_subtype": v.notice_subtype,
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
