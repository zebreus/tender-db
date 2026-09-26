//! `plan-probe` — the instrument for issue 428: what does `ANALYZE` change?
//!
//! It opens a database file DIRECTLY with turso (no `Db::open`: no migrations, no
//! schema batch, nothing written except what a subcommand says), so it can be
//! pointed at a reflinked copy of a weekly snapshot and never at the serving DB.
//!
//! ```text
//! plan-probe analyze <db> [--resume] [TABLE...]   ANALYZE table by table, timed;
//!                                                 --resume skips tables already in sqlite_stat1
//! plan-probe stats   <db>                         dump sqlite_stat1
//! plan-probe plan    <db> <statements>            EXPLAIN QUERY PLAN of every statement
//! plan-probe run     <db> <statements> <name>     execute ONE statement, drain it, time it
//! ```
//!
//! The statements file is blocks headed `-- name: <name>`, optionally followed by
//! `-- params: <json array>` (integers, strings, null — bound exactly as the app
//! binds them, so the plan is the app's plan), then the SQL.
//!
//! `run` executes a single statement per process on purpose: turso 0.7.2's SDK
//! exposes no interrupt (issue 425), so the only bound on a runaway plan is the
//! caller's `timeout` killing the process. Every line is tab-separated and
//! flushed as it is produced, so a killed run still leaves what it measured.

use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;
use turso::Value;

struct Stmt {
    name: String,
    params: Vec<Value>,
    sql: String,
}

fn parse_statements(text: &str) -> Result<Vec<Stmt>, String> {
    let mut out: Vec<Stmt> = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("-- name:") {
            out.push(Stmt { name: name.trim().to_owned(), params: Vec::new(), sql: String::new() });
        } else if let Some(json) = line.strip_prefix("-- params:") {
            let cur = out.last_mut().ok_or("`-- params:` before any `-- name:`")?;
            let parsed: serde_json::Value =
                serde_json::from_str(json.trim()).map_err(|e| format!("{}: params: {e}", cur.name))?;
            let arr = parsed.as_array().ok_or_else(|| format!("{}: params is not an array", cur.name))?;
            cur.params = arr
                .iter()
                .map(|v| match v {
                    serde_json::Value::Null => Ok(Value::Null),
                    serde_json::Value::String(s) => Ok(Value::Text(s.clone())),
                    serde_json::Value::Number(n) => n
                        .as_i64()
                        .map(Value::Integer)
                        .or_else(|| n.as_f64().map(Value::Real))
                        .ok_or_else(|| format!("{}: unrepresentable number {n}", cur.name)),
                    other => Err(format!("{}: unsupported param {other}", cur.name)),
                })
                .collect::<Result<_, _>>()?;
        } else if let Some(cur) = out.last_mut() {
            cur.sql.push_str(line);
            cur.sql.push('\n');
        }
    }
    out.retain(|s| !s.sql.trim().is_empty());
    Ok(out)
}

fn emit(line: String) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

async fn open(path: &str) -> Result<turso::Connection, String> {
    if !std::path::Path::new(path).exists() {
        // turso would create an empty database and every plan would read "fine".
        return Err(format!("{path}: no such file (refusing to create one)"));
    }
    let db = turso::Builder::new_local(path).build().await.map_err(|e| format!("open {path}: {e}"))?;
    db.connect().map_err(|e| format!("connect {path}: {e}"))
}

async fn strings(conn: &turso::Connection, sql: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = conn.query(sql, ()).await.map_err(|e| format!("{sql}: {e}"))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| format!("{sql}: {e}"))? {
        let mut cols = Vec::new();
        for i in 0..row.column_count() {
            cols.push(match row.get_value(i).map_err(|e| e.to_string())? {
                Value::Null => "NULL".to_owned(),
                Value::Integer(i) => i.to_string(),
                Value::Real(r) => r.to_string(),
                Value::Text(t) => t,
                Value::Blob(b) => format!("<blob {}>", b.len()),
            });
        }
        out.push(cols);
    }
    Ok(out)
}

async fn analyze(db: &str, args: &[String]) -> Result<(), String> {
    let conn = open(db).await?;
    let resume = args.iter().any(|a| a == "--resume");
    let named: Vec<String> = args.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    let tables: Vec<String> = if named.is_empty() {
        strings(
            &conn,
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .await?
        .into_iter()
        .map(|r| r[0].clone())
        .collect()
    } else {
        named
    };
    let done: Vec<String> = if resume {
        strings(&conn, "SELECT DISTINCT tbl FROM sqlite_stat1").await.unwrap_or_default().into_iter().map(|r| r[0].clone()).collect()
    } else {
        Vec::new()
    };
    let total = Instant::now();
    for table in tables {
        if done.contains(&table) {
            emit(format!("SKIP\t{table}\talready in sqlite_stat1"));
            continue;
        }
        let started = Instant::now();
        let sql = format!("ANALYZE \"{}\"", table.replace('"', "\"\""));
        match conn.execute(&sql, ()).await {
            Ok(_) => emit(format!("ANALYZE\t{table}\t{:.1}s", started.elapsed().as_secs_f64())),
            Err(e) => emit(format!("ANALYZE-ERR\t{table}\t{:.1}s\t{e}", started.elapsed().as_secs_f64())),
        }
    }
    emit(format!("ANALYZE-TOTAL\t{:.1}s", total.elapsed().as_secs_f64()));
    Ok(())
}

async fn stats(db: &str) -> Result<(), String> {
    let conn = open(db).await?;
    for r in strings(&conn, "SELECT tbl, COALESCE(idx, ''), stat FROM sqlite_stat1 ORDER BY tbl, idx").await? {
        emit(format!("STAT\t{}\t{}\t{}", r[0], r[1], r[2]));
    }
    Ok(())
}

async fn plan(db: &str, file: &str) -> Result<(), String> {
    // No `query_only` here: turso refuses to even PREPARE a write under it, and the
    // fold's INSERT … SELECT / UPDATE … WHERE plans are part of what stats could
    // move. `EXPLAIN QUERY PLAN` compiles and never executes.
    let conn = open(db).await?;
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    for s in parse_statements(&text)? {
        let sql = format!("EXPLAIN QUERY PLAN {}", s.sql.trim().trim_end_matches(';'));
        let mut rows = match conn.query(&sql, s.params.clone()).await {
            Ok(rows) => rows,
            Err(e) => {
                emit(format!("PLAN-ERR\t{}\t{}", s.name, e.to_string().replace('\n', " ")));
                continue;
            }
        };
        // (id, parent, detail) → depth by following parents, so the rendering is a
        // tree a diff can read.
        let mut nodes: Vec<(i64, i64, String)> = Vec::new();
        loop {
            match rows.next().await {
                Ok(Some(row)) => {
                    let id = match row.get_value(0) { Ok(Value::Integer(i)) => i, _ => -1 };
                    let parent = match row.get_value(1) { Ok(Value::Integer(i)) => i, _ => -1 };
                    let detail = match row.get_value(3) { Ok(Value::Text(t)) => t, other => format!("{other:?}") };
                    nodes.push((id, parent, detail));
                }
                Ok(None) => break,
                Err(e) => {
                    emit(format!("PLAN-ERR\t{}\t{}", s.name, e.to_string().replace('\n', " ")));
                    break;
                }
            }
        }
        for (id, parent, detail) in &nodes {
            let mut depth = 0;
            let mut p = *parent;
            while p != 0 && depth < 32 {
                depth += 1;
                p = nodes.iter().find(|n| n.0 == p).map(|n| n.1).unwrap_or(0);
            }
            let _ = id;
            emit(format!("PLAN\t{}\t{}{}", s.name, "  ".repeat(depth), detail));
        }
    }
    Ok(())
}

async fn run(db: &str, file: &str, name: &str) -> Result<(), String> {
    let conn = open(db).await?;
    conn.execute("PRAGMA query_only = 1", ()).await.map_err(|e| format!("query_only: {e}"))?;
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let s = parse_statements(&text)?
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("{file}: no statement named {name}"))?;
    let started = Instant::now();
    let mut rows = conn.query(s.sql.trim(), s.params.clone()).await.map_err(|e| format!("{name}: {e}"))?;
    let mut n = 0u64;
    let mut first = None;
    while rows.next().await.map_err(|e| format!("{name}: {e}"))?.is_some() {
        if n == 0 {
            first = Some(started.elapsed().as_secs_f64());
        }
        n += 1;
    }
    emit(format!(
        "RUN\t{name}\trows={n}\tsecs={:.3}\tfirst={}",
        started.elapsed().as_secs_f64(),
        first.map(|f| format!("{f:.3}")).unwrap_or_else(|| "-".into())
    ));
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["analyze", db, rest @ ..] => analyze(db, &rest.iter().map(|s| s.to_string()).collect::<Vec<_>>()).await,
        ["stats", db] => stats(db).await,
        ["plan", db, file] => plan(db, file).await,
        ["run", db, file, name] => run(db, file, name).await,
        _ => Err("usage: plan-probe analyze <db> [--resume] [TABLE...] | stats <db> | plan <db> <file> | run <db> <file> <name>".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("plan-probe: {e}");
            ExitCode::FAILURE
        }
    }
}
