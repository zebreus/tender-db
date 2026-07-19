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
    FactRow, LotRow, NoticeRow, OrganizationRow, PartyRow, Stamp, TenderDetail, TenderRow,
    VersionRow,
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
        "publication_id": t.publication_id,
        "notice_subtype": t.notice_subtype,
        "value": money(t.value_cents, t.currency.as_deref()),
        "submission_deadline": stamp(t.deadline),
        "lots": t.lots,
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
        "parse_state": n.parse_state,
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

fn version(v: &VersionRow) -> Value {
    json!({
        "seq": v.seq,
        "published_at": instant(v.published_at),
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
    map.insert("versions".into(), json!(d.versions.iter().map(version).collect::<Vec<_>>()));
    object
}

/// The list envelope. `next_cursor` is null exactly when `more` is false, so a
/// client can loop on either one.
pub fn page(items: Vec<Value>, next: Option<i64>) -> Value {
    json!({
        "items": items,
        "next_cursor": next.map(|n| n.to_string()),
        "more": next.is_some(),
    })
}
