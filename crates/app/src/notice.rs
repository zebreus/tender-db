//! The operator notice: one plain-text file on the box whose text every page
//! shows as a banner and `GET /v1` and `GET /health` serve as `notice` — how a
//! known issue or maintenance is announced without a deploy (owner ruling
//! 2026-10-24: there are no active users yet, and prod may carry known issues
//! while a banner says so).
//!
//! The file is the whole interface. Write text into it and the notice shows
//! within [`TTL`]; delete it, or leave only whitespace in it, and the notice is
//! gone. Its path is `TENDER_DB_NOTICE_FILE`, else [`FILE_NAME`] beside the
//! database (`TENDER_DB`'s directory — `/data/db/notice.txt` on the box, a
//! directory the service already reads and root already writes, so neither a
//! unit edit nor a restart is needed).
//!
//! A request never reads the file: the text is cached for [`TTL`] and re-read by
//! the first request after it lapses, so under any load the file is read once a
//! TTL. A request never fails on it either — an unreadable file is logged once
//! and serves as no notice.

use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

/// The longest notice served, in bytes. A banner is a sentence or two; the cap
/// keeps a mistaken `cp` of a log file from becoming megabytes on every page.
pub const MAX_BYTES: usize = 2_000;

/// How long one read is served before the file is read again — how soon a
/// write on the box shows.
pub const TTL: Duration = Duration::from_secs(10);

/// The file's name beside the database when `TENDER_DB_NOTICE_FILE` is unset.
pub const FILE_NAME: &str = "notice.txt";

/// One notice file and its cached text.
pub struct Notice {
    path: PathBuf,
    ttl: Duration,
    cache: Mutex<Cache>,
}

#[derive(Default)]
struct Cache {
    /// When `text` was read; `None` until the first request asks.
    read_at: Option<Instant>,
    text: Option<String>,
    /// The last read failed (other than "no file") and the log said so — the
    /// next failure stays quiet until a read succeeds again.
    warned: bool,
}

impl Notice {
    /// The notice at `path`, cached for [`TTL`].
    pub fn at(path: impl Into<PathBuf>) -> Notice {
        Notice::with_ttl(path, TTL)
    }

    /// As [`at`](Notice::at), with an explicit cache lifetime — a test passes
    /// zero to watch a write show on the very next request.
    pub fn with_ttl(path: impl Into<PathBuf>, ttl: Duration) -> Notice {
        Notice { path: path.into(), ttl, cache: Mutex::new(Cache::default()) }
    }

    /// The notice the environment names: `TENDER_DB_NOTICE_FILE`, else
    /// [`FILE_NAME`] beside `TENDER_DB` ([`path_from`]).
    pub fn from_env() -> Notice {
        Notice::at(path_from(
            std::env::var("TENDER_DB_NOTICE_FILE").ok().as_deref(),
            std::env::var("TENDER_DB").ok().as_deref(),
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The notice's text, or `None` when there is none to show. Served from the
    /// cache while it is younger than the TTL; the first call after that reads
    /// the file (a stat and at most a few KiB) and refreshes it.
    pub fn current(&self) -> Option<String> {
        // A poisoned lock only means a panic elsewhere while it was held; the
        // cache it guards is always whole, so keep serving it.
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(at) = cache.read_at
            && at.elapsed() < self.ttl
        {
            return cache.text.clone();
        }
        cache.text = match read(&self.path) {
            Ok(text) => {
                cache.warned = false;
                text
            }
            Err(e) => {
                if !cache.warned {
                    eprintln!(
                        "notice: cannot read {}: {e} — serving no notice until it reads",
                        self.path.display()
                    );
                    cache.warned = true;
                }
                None
            }
        };
        cache.read_at = Some(Instant::now());
        cache.text.clone()
    }
}

/// The process's notice, built from the environment once — what `main`'s
/// [`crate::v1::AppState`] and the dashboard's server function share, so every
/// surface serves the same read.
pub fn global() -> Arc<Notice> {
    static GLOBAL: OnceLock<Arc<Notice>> = OnceLock::new();
    GLOBAL.get_or_init(|| Arc::new(Notice::from_env())).clone()
}

/// Where the notice lives: `explicit` (`TENDER_DB_NOTICE_FILE`) when set, else
/// [`FILE_NAME`] in the database's directory. `db` is `TENDER_DB`, defaulted the
/// way `store::state` defaults it (`tender-db.db` in the working directory), so
/// the notice always sits beside whatever file the server opened.
pub fn path_from(explicit: Option<&str>, db: Option<&str>) -> PathBuf {
    match explicit.map(str::trim).filter(|p| !p.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => Path::new(db.filter(|p| !p.is_empty()).unwrap_or("tender-db.db")).with_file_name(FILE_NAME),
    }
}

/// Read the file at `path` into a notice. No file is no notice, not an error.
fn read(path: &Path) -> std::io::Result<Option<String>> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    // A FIFO would block the request that opened it; a directory fails late.
    if !meta.is_file() {
        return Err(std::io::Error::other("not a regular file"));
    }
    // A few times the cap: room for surrounding whitespace, and never the whole
    // of a large file pointed here by mistake.
    let mut raw = Vec::new();
    std::fs::File::open(path)?.take(4 * MAX_BYTES as u64).read_to_end(&mut raw)?;
    Ok(clean(&String::from_utf8_lossy(&raw)))
}

/// The text a file's contents serve as: trimmed, `None` when nothing is left,
/// and cut to [`MAX_BYTES`] on a character boundary, with an ellipsis saying so.
pub fn clean(raw: &str) -> Option<String> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if text.len() <= MAX_BYTES {
        return Some(text.to_owned());
    }
    let mut end = MAX_BYTES - '…'.len_utf8();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Some(format!("{}…", text[..end].trim_end()))
}

/// `text` safe to place in HTML element content or a quoted attribute — for the
/// hand-written pages (`/docs`); the dashboard's rsx escapes its own text.
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch file path unique to this test process and `name`.
    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("tender-db-notice-{name}-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_missing_file_is_no_notice() {
        let notice = Notice::with_ttl(scratch("missing"), Duration::ZERO);
        assert_eq!(notice.current(), None);
    }

    #[test]
    fn an_empty_or_whitespace_file_is_no_notice() {
        let path = scratch("blank");
        let notice = Notice::with_ttl(&path, Duration::ZERO);
        for contents in ["", "   ", "\n\n", " \t\r\n "] {
            std::fs::write(&path, contents).unwrap();
            assert_eq!(notice.current(), None, "{contents:?} is no notice");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_text_is_trimmed_and_inner_lines_kept() {
        let path = scratch("trim");
        std::fs::write(&path, "\n  Known issue: lot values are being re-derived.\nBack by 18:00.  \n\n").unwrap();
        let notice = Notice::with_ttl(&path, Duration::ZERO);
        assert_eq!(
            notice.current().as_deref(),
            Some("Known issue: lot values are being re-derived.\nBack by 18:00.")
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_oversize_file_is_cut_on_a_char_boundary_with_an_ellipsis() {
        // Two-byte characters, so the cut lands mid-character unless it looks.
        let text = clean(&"ä".repeat(MAX_BYTES)).expect("a notice");
        assert!(text.len() <= MAX_BYTES, "{} bytes", text.len());
        assert!(text.ends_with('…'));
        assert!(text.trim_end_matches('…').chars().all(|c| c == 'ä'));
        // At the cap exactly, nothing is cut.
        assert_eq!(clean(&"a".repeat(MAX_BYTES)).map(|t| t.len()), Some(MAX_BYTES));

        // Through the file, too, including one far larger than the read bound.
        let path = scratch("oversize");
        std::fs::write(&path, "x".repeat(1 << 20)).unwrap();
        let read = Notice::with_ttl(&path, Duration::ZERO).current().expect("a notice");
        assert!(read.len() <= MAX_BYTES && read.ends_with('…'), "{} bytes", read.len());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_unreadable_path_is_no_notice_not_a_failure() {
        // A directory where the file should be: the read fails, the notice is none.
        let path = scratch("unreadable");
        let _ = std::fs::remove_dir(&path);
        std::fs::create_dir(&path).unwrap();
        let notice = Notice::with_ttl(&path, Duration::ZERO);
        let warned = || notice.cache.lock().unwrap().warned;
        assert_eq!(notice.current(), None);
        assert!(warned(), "the first failure is logged and marked");
        // …and a second request is answered the same way, the mark that gates the
        // log still set, so it is logged once, not twice.
        assert_eq!(notice.current(), None);
        assert!(warned(), "the second failure stays quiet");
        // A good read clears the mark, so a later failure is logged afresh.
        std::fs::remove_dir(&path).unwrap();
        std::fs::write(&path, "Back to normal").unwrap();
        assert_eq!(notice.current().as_deref(), Some("Back to normal"));
        assert!(!warned(), "a good read resets the mark");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_fifo_is_no_notice_and_never_blocks() {
        // Opening a FIFO to read waits for a writer, and it would wait holding the
        // cache lock every page, `/v1` and `/health` queue on: it is refused unopened.
        let path = scratch("fifo");
        let made = std::process::Command::new("mkfifo").arg(&path).status().expect("mkfifo runs");
        assert!(made.success(), "mkfifo {}", path.display());
        let (tx, rx) = std::sync::mpsc::channel();
        let reader = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Notice::with_ttl(reader, Duration::ZERO).current());
        });
        // A read blocked in `open` never sends: the deadline fails the test rather
        // than hanging it (the stuck thread is left to the process's exit).
        let served = rx.recv_timeout(Duration::from_secs(10)).expect("the read returns, not blocks on the FIFO");
        assert_eq!(served, None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_write_shows_after_the_ttl_and_not_before() {
        let path = scratch("ttl");
        let cached = Notice::with_ttl(&path, Duration::from_secs(3600));
        let live = Notice::with_ttl(&path, Duration::ZERO);
        assert_eq!(cached.current(), None, "read once: no file");
        std::fs::write(&path, "Maintenance tonight").unwrap();
        assert_eq!(cached.current(), None, "inside the TTL the cached read is served");
        assert_eq!(live.current().as_deref(), Some("Maintenance tonight"), "a lapsed TTL re-reads");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(live.current(), None, "and a removed file clears it");
    }

    #[test]
    fn the_path_defaults_to_beside_the_database() {
        assert_eq!(path_from(None, Some("/data/db/tender-db.db")), PathBuf::from("/data/db/notice.txt"));
        assert_eq!(path_from(None, None), PathBuf::from("notice.txt"), "beside the workdir default");
        assert_eq!(path_from(Some(""), Some("/x/y.db")), PathBuf::from("/x/notice.txt"), "empty is unset");
        assert_eq!(path_from(Some("/etc/tender-db/banner"), Some("/x/y.db")), PathBuf::from("/etc/tender-db/banner"));
    }

    #[test]
    fn html_escaping_leaves_no_markup() {
        assert_eq!(
            escape_html(r#"<script>alert("x")</script> & 'y'"#),
            "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; &#39;y&#39;"
        );
        assert_eq!(escape_html("Wartung: Ä–Ö"), "Wartung: Ä–Ö");
    }
}
