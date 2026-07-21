//! Package walker: yields every payload file inside an archived package.
//!
//! Container formats are told apart by file magic rather than by source or
//! name, so a renamed archive still walks correctly:
//!
//! - TED daily packages are `.tar.gz`, and their shape changed with the eras
//!   (verified against the sample ladder on the VPS): 1993–2010 (text era)
//!   the tarball holds one **ZIP per language and encoding variant**
//!   (`EN_19930102_1993001_ISO_ORG.zip`, `en_20100102_001_utf8_org.zip`, …),
//!   each containing a single large tagged-text document concatenating the
//!   whole day's notices; from 2011 one XML file per notice under a
//!   `<date>_<issue>/` directory. The walker unwraps one level of ZIP
//!   nesting and reports the nested path as `outer.zip!inner`.
//! - TED monthly packages are **plain tars whose members are the month's
//!   daily `.tar.gz` files**. A gzip-magic member is unpacked in-stream and
//!   its payloads carry the nested path `<daily>.tar.gz/<file>`, so a
//!   monthly-first backfill ingests the same notices under the monthly's
//!   fetch row and identity dedup makes re-processing the standalone daily a
//!   no-op (and vice versa).
//! - DÖE packages (`doe/monthly/YYYY-MM.zip`, `doe/daily/YYYY-MM-DD.zip`)
//!   are plain ZIPs holding one `<uuid|numeric>-<version>.xml` per notice
//!   version, no nesting.
//!
//! Members are visited one at a time, and nested tars are decompressed as
//! streams — a 2007 daily expands to roughly a gigabyte, which must never be
//! held in memory at once.

use std::io::Read;
use std::path::Path;

/// One payload file inside a package.
pub struct Member<'a> {
    /// Package-relative path; `outer.zip!inner` for a member of a nested ZIP,
    /// `daily.tar.gz/inner` for a member of a nested tar.
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

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// The payload-level entry names of the package at `archive`, without opening
/// nested ZIPs — the cheap pre-scan behind package-level dispatch policy (the
/// text era's ISO-vs-UTF8 variant selection needs to know what else the
/// package ships before the first member is judged). Nested tars *are*
/// descended, names only, so the policy sees a monthly's dailies too.
pub fn entry_names(archive: &Path) -> Result<Vec<String>, Error> {
    let mut names = Vec::new();
    match open(archive)? {
        Container::Zip(file) => {
            let zip = zip::ZipArchive::new(file)
                .map_err(|e| Error::Zip(format!("{}: {e}", archive.display())))?;
            names.extend(zip.file_names().map(str::to_owned));
        }
        Container::TarGz(file) => {
            let mut reader = flate2::read::GzDecoder::new(std::io::BufReader::new(file));
            tar_names(&mut reader, "", &mut names)?;
        }
        Container::Tar(file) => {
            let mut reader = std::io::BufReader::new(file);
            tar_names(&mut reader, "", &mut names)?;
        }
    }
    Ok(names)
}

/// Visit every payload file in the package at `archive`, in archive order.
pub fn walk(archive: &Path, mut visit: impl FnMut(Member<'_>)) -> Result<(), Error> {
    match open(archive)? {
        Container::Zip(file) => {
            let mut zip = zip::ZipArchive::new(file)
                .map_err(|e| Error::Zip(format!("{}: {e}", archive.display())))?;
            let mut bytes = Vec::new();
            for i in 0..zip.len() {
                let mut entry = zip
                    .by_index(i)
                    .map_err(|e| Error::Zip(format!("{}: {e}", archive.display())))?;
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
        Container::TarGz(file) => {
            let mut reader = flate2::read::GzDecoder::new(std::io::BufReader::new(file));
            walk_tar(&mut reader, "", &mut visit)
        }
        Container::Tar(file) => {
            let mut reader = std::io::BufReader::new(file);
            walk_tar(&mut reader, "", &mut visit)
        }
    }
}

/// What the file's own magic says the package is: ZIP (DÖE), gzipped tar
/// (TED dailies), or plain tar (TED monthlies).
enum Container {
    Zip(std::fs::File),
    TarGz(std::fs::File),
    Tar(std::fs::File),
}

fn open(archive: &Path) -> Result<Container, Error> {
    use std::io::Seek;
    let mut file = std::fs::File::open(archive)?;
    let mut magic = [0u8; 2];
    let n = read_up_to(&mut file, &mut magic)?;
    file.rewind()?;
    Ok(match &magic[..n] {
        m if m == b"PK" => Container::Zip(file),
        m if m == GZIP_MAGIC => Container::TarGz(file),
        _ => Container::Tar(file),
    })
}

/// Walk one tar stream. A gzip-magic member is itself a tar.gz (a monthly's
/// nested daily) and is descended in-stream; a ZIP member (text-era language
/// bundle) is unwrapped one level; everything else is a payload.
///
/// `dyn Read` is deliberate: recursing generically would instantiate an ever
/// deeper reader type per nesting level and never finish compiling.
fn walk_tar(
    reader: &mut dyn Read,
    prefix: &str,
    visit: &mut impl FnMut(Member<'_>),
) -> Result<(), Error> {
    let mut tar = tar::Archive::new(reader);
    let mut bytes = Vec::new();
    for entry in tar.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = nested(prefix, &entry.path()?.to_string_lossy());
        let mut head = [0u8; 2];
        let n = read_up_to(&mut entry, &mut head)?;
        if head[..n] == GZIP_MAGIC {
            let mut chained = flate2::read::GzDecoder::new((&head[..]).chain(entry));
            walk_tar(&mut chained, &path, visit)?;
            continue;
        }
        bytes.clear();
        bytes.extend_from_slice(&head[..n]);
        entry.read_to_end(&mut bytes)?;
        if is_zip(&path, &bytes) {
            walk_zip(&path, &bytes, visit);
        } else {
            visit(Member { path, bytes: &bytes });
        }
    }
    Ok(())
}

/// Names pass of [`walk_tar`]: same descent, no payload reads.
fn tar_names(reader: &mut dyn Read, prefix: &str, names: &mut Vec<String>) -> Result<(), Error> {
    let mut tar = tar::Archive::new(reader);
    for entry in tar.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = nested(prefix, &entry.path()?.to_string_lossy());
        let mut head = [0u8; 2];
        let n = read_up_to(&mut entry, &mut head)?;
        if head[..n] == GZIP_MAGIC {
            let mut chained = flate2::read::GzDecoder::new((&head[..]).chain(entry));
            tar_names(&mut chained, &path, names)?;
        } else {
            names.push(path);
        }
    }
    Ok(())
}

fn nested(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_owned() } else { format!("{prefix}/{name}") }
}

/// Read up to `buf.len()` bytes, tolerating shorter files.
fn read_up_to(reader: &mut impl Read, buf: &mut [u8]) -> Result<usize, Error> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = reader.read(&mut buf[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(filled)
}

/// Nested ZIP: match on the local file header magic rather than the extension,
/// so a mis-named member is still unwrapped instead of quarantined as garbage.
fn is_zip(path: &str, bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || path.to_ascii_lowercase().ends_with(".zip")
}

/// Unwrap one nested ZIP, visiting each inner file. Corruption is **never
/// fatal**: a truncated or otherwise unreadable bundle (e.g. TED's upstream
/// 1996 archive ships a `SV_..._ISO_ORG.zip` truncated on a 384 KB block
/// boundary, no End-Of-Central-Directory) must not abort the package or the
/// whole multi-year job. Per ADR-0004 we keep going and surface the bad member
/// for triage rather than dropping the run: an unopenable bundle is handed on
/// as a single payload so the parser quarantines it, and an unreadable entry
/// inside an otherwise-good bundle is skipped. The raw archive stays intact, so
/// a future fix can always reprocess.
fn walk_zip(outer: &str, bytes: &[u8], visit: &mut impl FnMut(Member<'_>)) {
    let mut zip = match zip::ZipArchive::new(std::io::Cursor::new(bytes)) {
        Ok(zip) => zip,
        Err(e) => {
            eprintln!("walk_zip {outer}: {e}; quarantining unreadable bundle");
            visit(Member { path: outer.to_owned(), bytes });
            return;
        }
    };
    let mut inner_bytes = Vec::new();
    for i in 0..zip.len() {
        let mut inner = match zip.by_index(i) {
            Ok(inner) => inner,
            Err(e) => {
                eprintln!("walk_zip {outer}[{i}]: {e}; skipping entry");
                continue;
            }
        };
        if !inner.is_file() {
            continue;
        }
        let name = inner.name().to_owned();
        inner_bytes.clear();
        if let Err(e) = inner.read_to_end(&mut inner_bytes) {
            eprintln!("walk_zip {outer}!{name}: {e}; skipping entry");
            continue;
        }
        visit(Member { path: format!("{outer}!{name}"), bytes: &inner_bytes });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a plain tar (a TED-monthly-shaped container) from named members.
    fn tar_with(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, data) in members {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).expect("append");
        }
        builder.into_inner().expect("tar")
    }

    fn scratch_tar(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("pkgtest-{}-{name}.tar", std::process::id()));
        std::fs::write(&path, bytes).expect("write scratch tar");
        path
    }

    /// A truncated inner ZIP (a valid local-file-header magic but no
    /// End-Of-Central-Directory) must be quarantined as a single bundle rather
    /// than aborting the walk — the exact shape that killed the 1996 TED
    /// monthly (`SV_..._ISO_ORG.zip`, truncated on a 384 KB boundary).
    #[test]
    fn corrupt_inner_zip_is_surfaced_not_fatal() {
        let good = b"<TED_EXPORT>ok</TED_EXPORT>".as_slice();
        let truncated_zip = b"PK\x03\x04\x14\x00\x00\x00\x08\x00truncated-no-eocd".as_slice();
        let after = b"<TED_EXPORT>after</TED_EXPORT>".as_slice();
        let tar = tar_with(&[
            ("day/good.xml", good),
            ("day/bad.zip", truncated_zip),
            ("day/after.xml", after),
        ]);
        let path = scratch_tar("corrupt-zip", &tar);

        let mut seen = Vec::new();
        let result = walk(&path, |m| seen.push(m.path.clone()));
        std::fs::remove_file(&path).ok();

        assert!(result.is_ok(), "a corrupt inner zip must not abort the walk: {result:?}");
        assert!(seen.iter().any(|p| p == "day/good.xml"), "member before the bad zip: {seen:?}");
        assert!(
            seen.iter().any(|p| p == "day/bad.zip"),
            "the unreadable bundle is surfaced for quarantine: {seen:?}"
        );
        assert!(
            seen.iter().any(|p| p == "day/after.xml"),
            "walking continues past the bad zip: {seen:?}"
        );
    }

    /// A well-formed nested ZIP still unwraps one level, reported as `outer!inner`.
    #[test]
    fn good_inner_zip_unwraps_one_level() {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        zip.start_file("EN.xml", opts).expect("start");
        std::io::Write::write_all(&mut zip, b"<TED_EXPORT>hi</TED_EXPORT>").expect("write");
        let zip_bytes = zip.finish().expect("finish").into_inner();

        let tar = tar_with(&[("day/bundle.zip", zip_bytes.as_slice())]);
        let path = scratch_tar("good-zip", &tar);

        let mut seen = Vec::new();
        let result = walk(&path, |m| seen.push(m.path.clone()));
        std::fs::remove_file(&path).ok();

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(seen, ["day/bundle.zip!EN.xml"], "one level unwrapped: {seen:?}");
    }
}
