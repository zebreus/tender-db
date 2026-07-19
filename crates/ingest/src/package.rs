//! Package walker: yields every payload file inside an archived package.
//!
//! Two container formats exist, told apart by file magic rather than by
//! source, so a renamed archive still walks correctly:
//!
//! - TED daily packages are `.tar.gz`, and their shape changed with the eras
//!   (verified against the sample ladder on the VPS): 1993–2010 (text era)
//!   the tarball holds one **ZIP per language and encoding variant**
//!   (`EN_19930102_1993001_ISO_ORG.zip`, `en_20100102_001_utf8_org.zip`, …),
//!   each containing a single large tagged-text document concatenating the
//!   whole day's notices; from 2011 one XML file per notice under a
//!   `<date>_<issue>/` directory. The walker unwraps one level of ZIP
//!   nesting and reports the nested path as `outer.zip!inner`.
//! - DÖE packages (`doe/monthly/YYYY-MM.zip`, `doe/daily/YYYY-MM-DD.zip`)
//!   are plain ZIPs holding one `<uuid|numeric>-<version>.xml` per notice
//!   version, no nesting.
//!
//! Members are visited one at a time — a 2007 daily expands to roughly a
//! gigabyte, which must never be held in memory at once.

use std::io::Read;
use std::path::Path;

/// One payload file inside a package.
pub struct Member<'a> {
    /// Package-relative path; `outer.zip!inner` for a nested member.
    pub path: String,
    pub bytes: &'a [u8],
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Zip(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Zip(e) => write!(f, "zip: {e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// The tar-level entry names of the `.tar.gz` at `archive`, without unwrapping
/// nested ZIPs — the cheap pre-scan behind package-level dispatch policy (the
/// text era's ISO-vs-UTF8 variant selection needs to know what else the day
/// ships before the first member is judged).
pub fn entry_names(archive: &Path) -> Result<Vec<String>, Error> {
    if is_zip_package(archive)? {
        let file = std::fs::File::open(archive)?;
        let zip = zip::ZipArchive::new(file)
            .map_err(|e| Error::Zip(format!("{}: {e}", archive.display())))?;
        return Ok(zip.file_names().map(str::to_owned).collect());
    }
    let file = std::fs::File::open(archive)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(std::io::BufReader::new(file)));
    let mut names = Vec::new();
    for entry in tar.entries()? {
        let entry = entry?;
        if entry.header().entry_type().is_file() {
            names.push(entry.path()?.to_string_lossy().into_owned());
        }
    }
    Ok(names)
}

/// A package is either a plain ZIP (DÖE) or a gzipped tar (TED) — decided by
/// the file's own magic bytes, never by its name.
fn is_zip_package(archive: &Path) -> Result<bool, Error> {
    use std::io::Read;
    let mut magic = [0u8; 2];
    let n = std::fs::File::open(archive)?.read(&mut magic)?;
    Ok(n == 2 && magic == *b"PK")
}

/// Visit every payload file in the package at `archive`, in archive order.
pub fn walk(archive: &Path, mut visit: impl FnMut(Member<'_>)) -> Result<(), Error> {
    if is_zip_package(archive)? {
        return walk_zip_package(archive, &mut visit);
    }
    let file = std::fs::File::open(archive)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(std::io::BufReader::new(file)));

    let mut bytes = Vec::new();
    for entry in tar.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.to_string_lossy().into_owned();
        bytes.clear();
        entry.read_to_end(&mut bytes)?;

        if is_zip(&path, &bytes) {
            walk_zip(&path, &bytes, &mut visit)?;
        } else {
            visit(Member { path, bytes: &bytes });
        }
    }
    Ok(())
}

/// A ZIP package (DÖE): one payload file per entry, streamed from disk.
fn walk_zip_package(archive: &Path, visit: &mut impl FnMut(Member<'_>)) -> Result<(), Error> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| Error::Zip(format!("{}: {e}", archive.display())))?;
    let mut bytes = Vec::new();
    for i in 0..zip.len() {
        let mut entry =
            zip.by_index(i).map_err(|e| Error::Zip(format!("{}: {e}", archive.display())))?;
        if !entry.is_file() {
            continue;
        }
        let path = entry.name().to_owned();
        bytes.clear();
        entry.read_to_end(&mut bytes)?;
        visit(Member { path, bytes: &bytes });
    }
    Ok(())
}

/// Nested ZIP: match on the local file header magic rather than the extension,
/// so a mis-named member is still unwrapped instead of quarantined as garbage.
fn is_zip(path: &str, bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || path.to_ascii_lowercase().ends_with(".zip")
}

fn walk_zip(outer: &str, bytes: &[u8], visit: &mut impl FnMut(Member<'_>)) -> Result<(), Error> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| Error::Zip(format!("{outer}: {e}")))?;
    let mut inner_bytes = Vec::new();
    for i in 0..zip.len() {
        let mut inner = zip.by_index(i).map_err(|e| Error::Zip(format!("{outer}: {e}")))?;
        if !inner.is_file() {
            continue;
        }
        let name = inner.name().to_owned();
        inner_bytes.clear();
        inner.read_to_end(&mut inner_bytes)?;
        visit(Member { path: format!("{outer}!{name}"), bytes: &inner_bytes });
    }
    Ok(())
}
