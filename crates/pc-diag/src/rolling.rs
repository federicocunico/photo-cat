//! Writer di log a rotazione giornaliera e per dimensione, con conservazione
//! di un numero massimo di file.
//!
//! Nomi file: `<prefisso>.<AAAA-MM-GG>.<indice>.log`.

use chrono::NaiveDate;
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type DateClock = Arc<dyn Fn() -> NaiveDate + Send + Sync>;

pub struct RollingFileWriter {
    dir: PathBuf,
    prefix: String,
    max_bytes: u64,
    max_files: usize,
    clock: DateClock,
    state: Mutex<Option<Current>>,
}

struct Current {
    file: File,
    date: NaiveDate,
    index: u32,
    size: u64,
}

impl RollingFileWriter {
    pub fn new(dir: impl AsRef<Path>, prefix: &str, max_bytes: u64, max_files: usize) -> Self {
        Self::with_clock(
            dir,
            prefix,
            max_bytes,
            max_files,
            Arc::new(|| chrono::Local::now().date_naive()),
        )
    }

    pub fn with_clock(
        dir: impl AsRef<Path>,
        prefix: &str,
        max_bytes: u64,
        max_files: usize,
        clock: DateClock,
    ) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            prefix: prefix.to_string(),
            max_bytes: max_bytes.max(1),
            max_files: max_files.max(1),
            clock,
            state: Mutex::new(None),
        }
    }

    fn file_name(&self, date: NaiveDate, index: u32) -> PathBuf {
        self.dir.join(format!(
            "{}.{}.{}.log",
            self.prefix,
            date.format("%Y-%m-%d"),
            index
        ))
    }

    /// File di log presenti, ordinati dal più vecchio al più recente.
    pub fn list_files(&self) -> io::Result<Vec<PathBuf>> {
        list_log_files(&self.dir, &self.prefix)
    }

    fn open(&self, date: NaiveDate, index: u32) -> io::Result<Current> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.file_name(date, index);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata()?.len();
        Ok(Current {
            file,
            date,
            index,
            size,
        })
    }

    /// Indice più alto già presente per la data (ripresa dopo un riavvio).
    fn last_index(&self, date: NaiveDate) -> u32 {
        let mut index = 0;
        while self.file_name(date, index + 1).exists() {
            index += 1;
        }
        index
    }

    fn prune(&self) -> io::Result<()> {
        let files = self.list_files()?;
        if files.len() > self.max_files {
            for old in &files[..files.len() - self.max_files] {
                let _ = std::fs::remove_file(old);
            }
        }
        Ok(())
    }

    fn write_locked(&self, buf: &[u8]) -> io::Result<usize> {
        let today = (self.clock)();
        let len = buf.len() as u64;
        let mut guard = self.state.lock();
        let next_index = match guard.as_ref() {
            None => {
                let cur = self.open(today, self.last_index(today))?;
                let full = cur.size > 0 && cur.size + len > self.max_bytes;
                let index = cur.index;
                *guard = Some(cur);
                full.then_some(index + 1)
            }
            Some(cur) if cur.date != today => Some(0),
            Some(cur) if cur.size > 0 && cur.size + len > self.max_bytes => Some(cur.index + 1),
            Some(_) => None,
        };
        if let Some(index) = next_index {
            *guard = Some(self.open(today, index)?);
            self.prune()?;
        }
        write_into(guard.as_mut().expect("file di log aperto"), buf)
    }
}

fn write_into(cur: &mut Current, buf: &[u8]) -> io::Result<usize> {
    cur.file.write_all(buf)?;
    cur.size += buf.len() as u64;
    Ok(buf.len())
}

/// Ordina per (data, indice) i file `<prefisso>.<data>.<indice>.log` di una cartella.
pub fn list_log_files(dir: &Path, prefix: &str) -> io::Result<Vec<PathBuf>> {
    let mut found: Vec<(NaiveDate, u32, PathBuf)> = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(rest) = name.strip_prefix(prefix).and_then(|r| r.strip_prefix('.')) else {
            continue;
        };
        let Some(rest) = rest.strip_suffix(".log") else {
            continue;
        };
        let Some((date, index)) = rest.rsplit_once('.') else {
            continue;
        };
        let (Ok(date), Ok(index)) = (NaiveDate::parse_from_str(date, "%Y-%m-%d"), index.parse())
        else {
            continue;
        };
        found.push((date, index, path));
    }
    found.sort();
    Ok(found.into_iter().map(|(_, _, p)| p).collect())
}

/// Handle clonabile da passare a `tracing_appender::non_blocking`.
#[derive(Clone)]
pub struct SharedRollingWriter(pub Arc<RollingFileWriter>);

impl Write for SharedRollingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_locked(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(cur) = self.0.state.lock().as_mut() {
            cur.file.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn clock_from(day: Arc<AtomicU32>) -> DateClock {
        Arc::new(move || NaiveDate::from_ymd_opt(2026, 10, day.load(Ordering::SeqCst)).unwrap())
    }

    #[test]
    fn rotates_by_size_and_prunes_oldest() {
        let tmp = tempfile::tempdir().unwrap();
        let day = Arc::new(AtomicU32::new(1));
        let w = Arc::new(RollingFileWriter::with_clock(
            tmp.path(),
            "pc",
            100,
            3,
            clock_from(day),
        ));
        let mut sw = SharedRollingWriter(w.clone());
        let line = [b'x'; 40];
        for _ in 0..12 {
            sw.write_all(&line).unwrap();
        }
        sw.flush().unwrap();
        let files = w.list_files().unwrap();
        assert_eq!(files.len(), 3, "{files:?}");
        for f in &files {
            assert!(std::fs::metadata(f).unwrap().len() <= 100);
        }
        // I più vecchi (indice 0) sono stati eliminati.
        let names: Vec<_> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names.last().unwrap(), "pc.2026-10-01.5.log");
    }

    #[test]
    fn rotates_daily() {
        let tmp = tempfile::tempdir().unwrap();
        let day = Arc::new(AtomicU32::new(1));
        let w = Arc::new(RollingFileWriter::with_clock(
            tmp.path(),
            "pc",
            1_000_000,
            10,
            clock_from(day.clone()),
        ));
        let mut sw = SharedRollingWriter(w.clone());
        sw.write_all(b"giorno 1\n").unwrap();
        day.store(2, Ordering::SeqCst);
        sw.write_all(b"giorno 2\n").unwrap();
        sw.flush().unwrap();
        let files = w.list_files().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[1].to_string_lossy().ends_with("pc.2026-10-02.0.log"));
        assert_eq!(std::fs::read_to_string(&files[1]).unwrap(), "giorno 2\n");
    }

    #[test]
    fn resumes_existing_file_after_restart() {
        let tmp = tempfile::tempdir().unwrap();
        let day = Arc::new(AtomicU32::new(5));
        for msg in ["a\n", "b\n"] {
            let w = Arc::new(RollingFileWriter::with_clock(
                tmp.path(),
                "pc",
                1000,
                10,
                clock_from(day.clone()),
            ));
            let mut sw = SharedRollingWriter(w);
            sw.write_all(msg.as_bytes()).unwrap();
            sw.flush().unwrap();
        }
        let files = list_log_files(tmp.path(), "pc").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(std::fs::read_to_string(&files[0]).unwrap(), "a\nb\n");
    }
}
