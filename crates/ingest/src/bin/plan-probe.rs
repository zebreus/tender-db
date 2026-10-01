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
//! plan-probe exec    <db> <sql>                   one statement, writes allowed (a COPY only)
//! plan-probe mem     <db> <deadline_ms> <sql>     issue 426: peak memory of ONE statement
//! ```
//!
//! The statements file is blocks headed `-- name: <name>`, optionally followed by
//! `-- params: <json array>` (integers, strings, null — bound exactly as the app
//! binds them, so the plan is the app's plan), then the SQL.
//!
//! `run` executes a single statement per process on purpose: this tool wires no
//! interrupt (the vendored SDK has had `Connection::interrupt` since issues 425/438,
//! and `/v1/sql` uses it; this probe never did), so the only bound on a runaway plan
//! is the caller's `timeout` killing the process. Every line is tab-separated and
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
    // `PLAN_PROBE_PRINT=<n>` echoes the first n rows — for picking a realistic
    // parameter (a skewed value) from the copy itself.
    let print: u64 = std::env::var("PLAN_PROBE_PRINT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut n = 0u64;
    let mut first = None;
    while let Some(row) = rows.next().await.map_err(|e| format!("{name}: {e}"))? {
        if n == 0 {
            first = Some(started.elapsed().as_secs_f64());
        }
        if n < print {
            let cols: Vec<String> = (0..row.column_count()).map(|i| format!("{:?}", row.get_value(i).ok())).collect();
            emit(format!("ROW\t{name}\t{}", cols.join("\t")));
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

/// Issue 426: how much memory can ONE `/v1/sql` statement take before the engine's
/// deadline stops it? Opens the file the way a `/v1/sql` reader is set up — `query_only`,
/// a page cache of `PLAN_PROBE_CACHE_KIB` (the endpoint's readers get the store's
/// `cache_size`, 128 MiB unless `TENDER_CACHE_KIB` says otherwise), the per-statement
/// deadline (issue 425) — drains the result the way the endpoint does (it stops reading
/// at its 10,000-row cap), and reports the process's peak RSS. One statement per
/// process, so the high-water mark (`VmHWM`) is this statement's. A sampler thread
/// records the RSS timeline and the bytes held in temporary files (a spilled sorter or
/// a temp table lives in an unlinked file the fd table still shows), so memory the
/// engine moved to disk is counted too — issue 337 is what a temp file can cost.
async fn mem(db: &str, deadline_ms: &str, sql: &str) -> Result<(), String> {
    let deadline = std::time::Duration::from_millis(deadline_ms.parse().map_err(|e| format!("deadline_ms: {e}"))?);
    let conn = open(db).await?;
    let cache_kib: u64 = std::env::var("PLAN_PROBE_CACHE_KIB").ok().and_then(|v| v.parse().ok()).unwrap_or(131_072);
    strings(&conn, &format!("PRAGMA cache_size = -{cache_kib}")).await?;
    conn.execute("PRAGMA query_only = 1", ()).await.map_err(|e| format!("query_only: {e}"))?;
    conn.set_query_timeout(deadline).map_err(|e| format!("set_query_timeout: {e}"))?;

    let db_path = std::fs::canonicalize(db).map_err(|e| format!("{db}: {e}"))?;
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sampler = {
        let stop = stop.clone();
        std::thread::spawn(move || {
            let (mut peak_temp, mut timeline) = (0u64, Vec::new());
            let started = Instant::now();
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                peak_temp = peak_temp.max(temp_bytes(&db_path));
                if timeline.last().is_none_or(|(t, _): &(f64, u64)| started.elapsed().as_secs_f64() - t >= 1.0) {
                    timeline.push((started.elapsed().as_secs_f64(), status_kib("VmRSS")));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            (peak_temp, timeline)
        })
    };
    let hwm_before = status_kib("VmHWM");
    let started = Instant::now();
    let outcome: Result<u64, turso::Error> = async {
        let mut rows = conn.query(sql, ()).await?;
        let mut n = 0u64;
        while rows.next().await?.is_some() {
            n += 1;
            if n >= 10_000 {
                break;
            }
        }
        Ok(n)
    }
    .await;
    let secs = started.elapsed().as_secs_f64();
    let hwm_after = status_kib("VmHWM");
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let (peak_temp, timeline) = sampler.join().map_err(|_| "sampler panicked".to_owned())?;
    let outcome = match outcome {
        Ok(n) => format!("rows={n}"),
        Err(turso::Error::Interrupt(_)) => "STOPPED".to_owned(),
        Err(e) => format!("error={e}"),
    };
    let timeline: Vec<String> = timeline.iter().map(|(t, kib)| format!("{t:.0}s:{}M", kib / 1024)).collect();
    emit(format!(
        "MEM\t{outcome}\tsecs={secs:.3}\tpeak_rss_mib={}\tbaseline_mib={}\tgrowth_mib={}\tpeak_temp_mib={}\trss={}",
        hwm_after / 1024,
        hwm_before / 1024,
        hwm_after.saturating_sub(hwm_before) / 1024,
        peak_temp / (1024 * 1024),
        timeline.join(",")
    ));
    Ok(())
}

/// A `/proc/self/status` field in KiB (0 where it cannot be read — non-Linux).
fn status_kib(field: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix(field)?.strip_prefix(':').map(str::trim).map(str::to_owned))
        })
        .and_then(|v| v.split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

/// Bytes in the regular files this process holds open other than the database, its
/// WAL and its shm — i.e. the engine's temporary files, unlinked or not.
fn temp_bytes(db: &std::path::Path) -> u64 {
    let Ok(fds) = std::fs::read_dir("/proc/self/fd") else { return 0 };
    let db = db.to_string_lossy().into_owned();
    fds.flatten()
        .filter_map(|fd| {
            let target = std::fs::read_link(fd.path()).ok()?.to_string_lossy().into_owned();
            if !target.starts_with('/') || target.starts_with(&db) || target.starts_with("/dev/") || target.starts_with("/proc/") {
                return None;
            }
            std::fs::metadata(fd.path()).ok().filter(|m| m.is_file()).map(|m| m.len())
        })
        .sum()
}

/// Execute one statement, writes allowed — for shaping a COPY's `sqlite_stat1`
/// (e.g. keeping only a subset's rows) before re-planning. Never the serving DB.
async fn exec(db: &str, sql: &str) -> Result<(), String> {
    let conn = open(db).await?;
    let started = Instant::now();
    let n = conn.execute(sql, ()).await.map_err(|e| format!("{sql}: {e}"))?;
    emit(format!("EXEC\tchanged={n}\tsecs={:.3}", started.elapsed().as_secs_f64()));
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
        ["exec", db, sql] => exec(db, sql).await,
        ["mem", db, deadline_ms, sql] => mem(db, deadline_ms, sql).await,
        _ => Err("usage: plan-probe analyze <db> [--resume] [TABLE...] | stats <db> | plan <db> <file> | run <db> <file> <name> | exec <db> <sql> | mem <db> <deadline_ms> <sql>".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("plan-probe: {e}");
            ExitCode::FAILURE
        }
    }
}
