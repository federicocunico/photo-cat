//! Scansione di una sorgente: file supportati, ricorsione, pattern da ignorare.

use crate::formats::{support, DEFAULT_EXTENSIONS};
use globset::{Glob, GlobSet, GlobSetBuilder};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_store::model::Source;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundFile {
    pub path: PathBuf,
    pub size: u64,
    /// Ultima modifica, ms dall'epoca.
    pub mtime: i64,
}

/// Regole di selezione derivate dalla configurazione della sorgente.
pub struct ScanRules {
    root: PathBuf,
    recursive: bool,
    extensions: Vec<String>,
    ignore: GlobSet,
}

impl ScanRules {
    pub fn from_source(src: &Source) -> AppResult<Self> {
        let extensions: Vec<String> = if src.include_formats.is_empty() {
            DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect()
        } else {
            src.include_formats
                .iter()
                .map(|s| s.trim_start_matches('.').to_ascii_lowercase())
                .collect()
        };
        let mut b = GlobSetBuilder::new();
        for p in &src.ignore_patterns {
            let g = Glob::new(p).map_err(|e| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    format!("pattern non valido \"{p}\": {e}"),
                )
            })?;
            b.add(g);
        }
        let ignore = b
            .build()
            .map_err(|e| AppError::invalid_input(e.to_string()))?;
        Ok(Self {
            root: PathBuf::from(&src.path),
            recursive: src.recursive,
            extensions,
            ignore,
        })
    }

    fn accepts_ext(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                let e = e.to_ascii_lowercase();
                support(&e).is_some() && self.extensions.contains(&e)
            })
            .unwrap_or(false)
    }

    /// Il percorso (relativo alla radice, con `/`) corrisponde a un pattern da ignorare.
    fn ignored(&self, path: &Path) -> bool {
        if self.ignore.is_empty() {
            return false;
        }
        let rel = path.strip_prefix(&self.root).unwrap_or(path);
        let rel = rel.to_string_lossy().replace('\\', "/");
        self.ignore.is_match(&rel) || self.ignore.is_match(path)
    }

    /// Percorre la sorgente chiamando `on_file` per ogni file accettato
    /// (`false` interrompe) e `on_error` per cartelle o file illeggibili.
    pub fn walk(
        &self,
        mut on_file: impl FnMut(FoundFile) -> bool,
        mut on_error: impl FnMut(&Path, AppError),
    ) -> AppResult<()> {
        if !self.root.is_dir() {
            return Err(AppError::new(
                ErrorCode::NotFound,
                format!("cartella sorgente non trovata: {}", self.root.display()),
            ));
        }
        let mut walker = walkdir::WalkDir::new(&self.root).follow_links(false);
        if !self.recursive {
            walker = walker.max_depth(1);
        }
        let mut it = walker.into_iter();
        while let Some(entry) = it.next() {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    let path = e
                        .path()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| self.root.clone());
                    let err = match e.into_io_error() {
                        Some(io) => AppError::from(io),
                        None => AppError::new(ErrorCode::Io, "ciclo di collegamenti"),
                    };
                    on_error(&path, err);
                    continue;
                }
            };
            if entry.depth() > 0 && self.ignored(entry.path()) {
                if entry.file_type().is_dir() {
                    it.skip_current_dir();
                }
                continue;
            }
            if !entry.file_type().is_file() || !self.accepts_ext(entry.path()) {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    let err = e
                        .into_io_error()
                        .map(AppError::from)
                        .unwrap_or_else(|| AppError::internal("metadati"));
                    on_error(entry.path(), err);
                    continue;
                }
            };
            let found = FoundFile {
                path: entry.into_path(),
                size: meta.len(),
                mtime: mtime_ms(&meta),
            };
            if !on_file(found) {
                break;
            }
        }
        Ok(())
    }
}

pub fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(root: &Path, recursive: bool, formats: &[&str], ignore: &[&str]) -> Source {
        Source {
            id: 1,
            path: root.display().to_string(),
            recursive,
            include_formats: formats.iter().map(|s| s.to_string()).collect(),
            ignore_patterns: ignore.iter().map(|s| s.to_string()).collect(),
            watch: false,
            created_at: 0,
        }
    }

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    fn collect(rules: &ScanRules) -> Vec<String> {
        let mut v = Vec::new();
        rules
            .walk(
                |f| {
                    v.push(f.path.file_name().unwrap().to_string_lossy().to_string());
                    true
                },
                |_, _| {},
            )
            .unwrap();
        v.sort();
        v
    }

    #[test]
    fn recursion_formats_and_ignore_patterns() {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        for f in [
            "a.JPG",
            "b.png",
            "note.txt",
            "2023/c.jpeg",
            "2023/Toscana/d.heic",
            "2023/.thumbs/e.jpg",
            "Privato/f.jpg",
            "g.nef",
        ] {
            touch(r, f);
        }
        let all =
            ScanRules::from_source(&source(r, true, &[], &["**/.thumbs/**", "Privato"])).unwrap();
        assert_eq!(
            collect(&all),
            vec!["a.JPG", "b.png", "c.jpeg", "d.heic", "g.nef"]
        );

        let flat = ScanRules::from_source(&source(r, false, &[], &[])).unwrap();
        assert_eq!(collect(&flat), vec!["a.JPG", "b.png", "g.nef"]);

        let only_jpg = ScanRules::from_source(&source(r, true, &["jpg", ".JPEG"], &[])).unwrap();
        assert_eq!(
            collect(&only_jpg),
            vec!["a.JPG", "c.jpeg", "e.jpg", "f.jpg"]
        );
    }

    #[test]
    fn invalid_pattern_and_missing_root() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(ScanRules::from_source(&source(tmp.path(), true, &[], &["a[b"])).is_err());
        let gone =
            ScanRules::from_source(&source(&tmp.path().join("nope"), true, &[], &[])).unwrap();
        assert_eq!(
            gone.walk(|_| true, |_, _| {}).unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}
