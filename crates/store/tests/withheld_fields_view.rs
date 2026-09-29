//! Issue 239: `notice_withheld_fields` serves the same rows it did under its
//! old GROUP BY form, and no longer sorts the whole FieldsPrivacy cohort before
//! its first row — the sort that made even an unfiltered `LIMIT 1` exceed
//! /v1/sql's time limit.

/// The view as it stood before issue 239, kept verbatim as the reference.
const GROUPED: &str = "
    SELECT s.notice_id,
           s.parent_section_id AS section_id,
           MAX(CASE WHEN c.field_id LIKE 'BT-195%' THEN c.code END) AS withheld_field,
           MAX(CASE WHEN c.field_id LIKE 'BT-197%' THEN c.code END) AS reason_code,
           (SELECT t.value FROM notice_texts t
             WHERE t.notice_id = s.notice_id AND t.section_id = s.section_id
               AND t.field_id LIKE 'BT-196%' LIMIT 1) AS reason_text,
           (SELECT d.utc_seconds FROM notice_dates d
             WHERE d.notice_id = s.notice_id AND d.section_id = s.section_id
               AND d.field_id LIKE 'BT-198%' LIMIT 1) AS publish_after
      FROM notice_sections s
      LEFT JOIN notice_codes c ON c.notice_id = s.notice_id AND c.section_id = s.section_id
     WHERE s.kind = 'FieldsPrivacy'
     GROUP BY s.notice_id, s.section_id";

async fn exec(conn: &turso::Connection, sql: &str) {
    conn.execute(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

async fn rows(conn: &turso::Connection, sql: &str) -> Vec<String> {
    let mut rows = conn.query(sql, ()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        let cells: Vec<String> = (0..6).map(|i| format!("{:?}", r.get_value(i).unwrap())).collect();
        out.push(cells.join(" | "));
    }
    out.sort();
    out
}

async fn opcodes(conn: &turso::Connection, sql: &str) -> Vec<String> {
    let mut rows = conn.query(&format!("EXPLAIN {sql}"), ()).await.unwrap();
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.unwrap() {
        out.push(r.get_value(1).ok().and_then(|v| v.as_text().cloned()).unwrap_or_default());
    }
    out
}

#[tokio::test]
async fn the_view_serves_the_grouped_rows_without_sorting_the_cohort() {
    let path = format!("/tmp/tender-db-239-withheld-{}.db", std::process::id());
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
    drop(store::Db::open(&path).await.expect("open"));
    let raw = turso::Builder::new_local(&path).build().await.expect("raw open");
    let conn = raw.connect().expect("connect");

    exec(&conn, "INSERT INTO fetches (id, source, kind, period, url, sha256, bytes, fetched_at, path) VALUES (1, 'ted', 'daily', 'p', 'u', 'aa', 1, 0, 'p')").await;
    for n in [1, 2] {
        exec(&conn, &format!(
            "INSERT INTO notices (id, source, publication_id, content_hash, profile, fetch_id, member_path, ingested_at)
             VALUES ({n}, 'ted', 'pub-{n}', 'h{n}', 'eforms', 1, 'm{n}', 0)")).await;
    }
    let section = |n: i64, id: &str, parent: &str, kind: &str| {
        format!(
            "INSERT INTO notice_sections (notice_id, section_id, parent_section_id, kind)
             VALUES ({n}, '{id}', '{parent}', '{kind}')"
        )
    };
    let code = |n: i64, id: &str, field: &str, ordinal: i64, code: &str| {
        format!(
            "INSERT INTO notice_codes (notice_id, section_id, field_id, ordinal, code)
             VALUES ({n}, '{id}', '{field}', {ordinal}, '{code}')"
        )
    };
    // Every column present.
    exec(&conn, &section(1, "fp1", "RES-1", "FieldsPrivacy")).await;
    exec(&conn, &code(1, "fp1", "BT-195(BT-142)-LotResult", 0, "win-cho")).await;
    exec(&conn, &code(1, "fp1", "BT-197(BT-142)-LotResult", 0, "com-int")).await;
    exec(&conn, "INSERT INTO notice_texts (notice_id, section_id, field_id, ordinal, lang, value)
                 VALUES (1, 'fp1', 'BT-196(BT-142)-LotResult', 0, 'ENG', 'commercial interest')").await;
    exec(&conn, "INSERT INTO notice_dates (notice_id, section_id, field_id, ordinal, utc_seconds, offset_minutes, has_time)
                 VALUES (1, 'fp1', 'BT-198(BT-142)-LotResult', 0, 1700000000, 0, 0)").await;
    // Two identifier codes in one block: MAX picks, in both forms.
    exec(&conn, &section(1, "fp2", "RES-2", "FieldsPrivacy")).await;
    exec(&conn, &code(1, "fp2", "BT-195(BT-759)-LotResult", 0, "rec-sub-cou")).await;
    exec(&conn, &code(1, "fp2", "BT-195(BT-760)-LotResult", 1, "rec-sub-typ")).await;
    // A block with no codes at all still serves one row of NULLs.
    exec(&conn, &section(2, "fp1", "LOT-1", "FieldsPrivacy")).await;
    // A code that is neither BT-195 nor BT-197 does not leak into either column.
    exec(&conn, &code(2, "fp1", "BT-09-Procedure", 0, "other")).await;
    // Not a withholding block: no row.
    exec(&conn, &section(2, "lot", "root", "Lot")).await;
    exec(&conn, &code(2, "lot", "BT-195(BT-09)-Procedure", 0, "cro-bor-law")).await;

    let view = rows(&conn, "SELECT notice_id, section_id, withheld_field, reason_code, reason_text, publish_after FROM notice_withheld_fields").await;
    let grouped = rows(&conn, GROUPED).await;
    assert_eq!(view.len(), 3, "one row per FieldsPrivacy block: {view:#?}");
    assert_eq!(view, grouped, "the rewrite serves exactly the grouped form's rows");
    assert!(view.iter().any(|r| r.contains("rec-sub-typ")), "MAX over two identifier codes: {view:#?}");

    // The old form sorts the cohort before its first row; the view streams.
    let sorts = |ops: &[String]| ops.iter().filter(|op| op.starts_with("Sorter")).count();
    assert!(sorts(&opcodes(&conn, &format!("{GROUPED} LIMIT 1")).await) > 0, "the reference form sorts");
    assert_eq!(sorts(&opcodes(&conn, "SELECT * FROM notice_withheld_fields LIMIT 1").await), 0, "the view must not sort");

    drop(conn);
    for s in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{s}"));
    }
}
