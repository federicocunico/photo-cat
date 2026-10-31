//! Download robusto: ripresa con `Range` su file `.part`, verifica SHA-256 prima
//! di rendere definitivo il file, tentativi con attesa crescente e mirror.

use pc_core::{AppError, AppResult, ErrorCode};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub attempts_per_url: u32,
    pub first_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            attempts_per_url: 3,
            first_backoff: Duration::from_millis(800),
        }
    }
}

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

pub fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// Esito di un singolo tentativo.
enum Attempt {
    Done,
    /// Errore temporaneo (rete): si può riprovare riprendendo dal punto raggiunto.
    Retry(AppError),
}

fn try_once(
    client: &reqwest::blocking::Client,
    url: &str,
    part: &Path,
    expected_size: u64,
    on_progress: &dyn Fn(u64),
    checkpoint: &dyn Fn() -> bool,
) -> AppResult<Attempt> {
    if !checkpoint() {
        return Err(AppError::cancelled());
    }
    let have = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    if have > expected_size {
        std::fs::remove_file(part)?;
        return Ok(Attempt::Retry(AppError::new(
            ErrorCode::ChecksumMismatch,
            "file parziale troppo grande",
        )));
    }
    if have == expected_size {
        return Ok(Attempt::Done);
    }
    let mut req = client.get(url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let mut resp = match req.send() {
        Ok(r) => r,
        Err(e) => {
            return Ok(Attempt::Retry(AppError::new(
                ErrorCode::Network,
                e.to_string(),
            )))
        }
    };
    let status = resp.status();
    let append = match status.as_u16() {
        206 => {
            let valid_range = resp
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("bytes "))
                .and_then(|v| v.split_once('/'))
                .and_then(|(range, total)| {
                    let (start, end) = range.split_once('-')?;
                    Some((
                        start.parse::<u64>().ok()?,
                        end.parse::<u64>().ok()?,
                        total.parse::<u64>().ok()?,
                    ))
                })
                .is_some_and(|(start, end, total)| {
                    start == have && start <= end && end < total && total == expected_size
                });
            if !valid_range {
                let _ = std::fs::remove_file(part);
                return Ok(Attempt::Retry(AppError::new(
                    ErrorCode::Network,
                    "Content-Range non corrispondente",
                )));
            }
            have > 0
        }
        200 => false, // il server ignora il Range: si riparte da zero
        s if (500..600).contains(&s) || s == 429 => {
            return Ok(Attempt::Retry(AppError::new(
                ErrorCode::Network,
                format!("HTTP {s} da {url}"),
            )))
        }
        s => {
            return Err(
                AppError::new(ErrorCode::Network, format!("HTTP {s} da {url}"))
                    .with_retriable(false),
            )
        }
    };
    let mut file = if append {
        OpenOptions::new().append(true).open(part)?
    } else {
        File::create(part)?
    };
    let mut written = if append { have } else { 0 };
    on_progress(written);
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        if !checkpoint() {
            file.flush()?;
            return Err(AppError::cancelled());
        }
        let n = match resp.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                file.flush()?;
                return Ok(Attempt::Retry(AppError::new(
                    ErrorCode::Network,
                    format!("connessione interrotta: {e}"),
                )));
            }
        };
        if written.saturating_add(n as u64) > expected_size {
            drop(file);
            std::fs::remove_file(part)?;
            return Ok(Attempt::Retry(AppError::new(
                ErrorCode::ChecksumMismatch,
                "risposta più grande del modello atteso",
            )));
        }
        file.write_all(&buf[..n])?;
        written += n as u64;
        on_progress(written);
    }
    file.flush()?;
    if written < expected_size {
        return Ok(Attempt::Retry(AppError::new(
            ErrorCode::Network,
            format!("download incompleto: {written}/{expected_size} byte"),
        )));
    }
    Ok(Attempt::Done)
}

/// Scarica `dest` provando gli URL in ordine. Il file definitivo compare solo
/// dopo che lo SHA-256 corrisponde. `on_progress` riceve i byte scaricati.
pub fn download(
    urls: &[String],
    dest: &Path,
    expected_size: u64,
    sha256: &str,
    policy: &RetryPolicy,
    on_progress: &dyn Fn(u64),
    cancel: &AtomicBool,
) -> AppResult<()> {
    download_with_control(
        urls,
        dest,
        expected_size,
        sha256,
        policy,
        on_progress,
        &|| !cancel.load(Ordering::Relaxed),
    )
}

/// Come `download`, con pausa/annullamento cooperativi anche durante il trasferimento
/// e il backoff. `checkpoint` attende la ripresa e restituisce false se annullato.
pub fn download_with_control(
    urls: &[String],
    dest: &Path,
    expected_size: u64,
    sha256: &str,
    policy: &RetryPolicy,
    on_progress: &dyn Fn(u64),
    checkpoint: &dyn Fn() -> bool,
) -> AppResult<()> {
    if expected_size == 0 {
        return Err(AppError::invalid_input("dimensione modello mancante"));
    }
    if urls.is_empty() {
        return Err(AppError::new(
            ErrorCode::ModelUnavailable,
            "nessun URL di download disponibile",
        ));
    }
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("photo-cat/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::new(ErrorCode::Network, e.to_string()))?;
    let part = part_path(dest);
    let mut last_err = AppError::new(ErrorCode::Network, "download non riuscito");
    for (url_index, url) in urls.iter().enumerate() {
        let mut backoff = policy.first_backoff;
        for attempt in 1..=policy.attempts_per_url.max(1) {
            match try_once(&client, url, &part, expected_size, on_progress, checkpoint) {
                Ok(Attempt::Done) => {
                    let got = sha256_file(&part)?;
                    if got.eq_ignore_ascii_case(sha256) {
                        std::fs::rename(&part, dest)?;
                        tracing::info!(url = url.as_str(), path = %dest.display(), "download completato e verificato");
                        return Ok(());
                    }
                    let _ = std::fs::remove_file(&part);
                    last_err = AppError::new(
                        ErrorCode::ChecksumMismatch,
                        format!(
                            "SHA-256 non corrispondente da {url}: atteso {sha256}, ottenuto {got}"
                        ),
                    );
                    tracing::warn!(
                        code = "CHECKSUM_MISMATCH",
                        detail = last_err.detail.as_str(),
                        "file scaricato corrotto"
                    );
                }
                Ok(Attempt::Retry(e)) => {
                    tracing::warn!(
                        code = e.code.as_str(),
                        detail = e.detail.as_str(),
                        attempt,
                        url = url.as_str(),
                        "nuovo tentativo di download"
                    );
                    last_err = e;
                }
                Err(e) if e.code == ErrorCode::Cancelled => return Err(e),
                Err(e) => {
                    last_err = e;
                    break; // errore definitivo su questo URL: si passa al mirror
                }
            }
            if attempt < policy.attempts_per_url.max(1) {
                let start = std::time::Instant::now();
                while start.elapsed() < backoff {
                    if !checkpoint() {
                        return Err(AppError::cancelled());
                    }
                    std::thread::sleep(
                        (backoff - start.elapsed().min(backoff)).min(Duration::from_millis(50)),
                    );
                }
                backoff = backoff.saturating_mul(2);
            }
        }
        // Il mirror riparte da zero: i byte parziali potrebbero non coincidere.
        if url_index + 1 < urls.len() {
            let _ = std::fs::remove_file(&part);
        }
    }
    Err(last_err)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    pub enum Mode {
        Ok,
        /// La prima richiesta si interrompe dopo metà file.
        InterruptOnce,
        WrongRangeOnce,
        Corrupt,
        NotFound,
    }

    /// Server HTTP minimale (supporta `Range`) per i test. Restituisce l'URL base e il contatore richieste.
    pub fn serve(body: Vec<u8>, mode: Mode) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let h2 = hits.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let n = h2.fetch_add(1, Ordering::SeqCst);
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut range_start = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        range_start = v.trim().trim_end_matches('-').parse().unwrap_or(0);
                    }
                }
                let mut data = body.clone();
                if matches!(mode, Mode::Corrupt) {
                    let mid = data.len() / 2;
                    data[mid] ^= 0xFF;
                }
                if matches!(mode, Mode::NotFound) {
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                let slice = &data[range_start..];
                let status = if range_start > 0 {
                    "206 Partial Content"
                } else {
                    "200 OK"
                };
                let reported_start = if matches!(mode, Mode::WrongRangeOnce) && n == 0 {
                    0
                } else {
                    range_start
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Range: bytes {reported_start}-{}/{}\r\nConnection: close\r\n\r\n",
                    slice.len(), data.len() - 1, data.len()
                );
                let _ = stream.write_all(head.as_bytes());
                if matches!(mode, Mode::InterruptOnce) && n == 0 {
                    let _ = stream.write_all(&slice[..slice.len() / 2]);
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    continue;
                }
                let _ = stream.write_all(slice);
            }
        });
        (format!("http://{addr}/file.onnx"), hits)
    }

    pub fn sha(data: &[u8]) -> String {
        hex::encode(Sha256::digest(data))
    }

    fn body() -> Vec<u8> {
        (0..2_000_000u32).map(|i| (i % 253) as u8).collect()
    }

    fn fast() -> RetryPolicy {
        RetryPolicy {
            attempts_per_url: 3,
            first_backoff: Duration::from_millis(10),
        }
    }

    #[test]
    fn resumes_after_interruption_with_range() {
        let data = body();
        let (url, hits) = serve(data.clone(), Mode::InterruptOnce);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("m.onnx");
        let max = AtomicUsize::new(0);
        download(
            &[url],
            &dest,
            data.len() as u64,
            &sha(&data),
            &fast(),
            &|b| {
                max.fetch_max(b as usize, Ordering::SeqCst);
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), data);
        assert_eq!(hits.load(Ordering::SeqCst), 2, "una ripresa con Range");
        assert_eq!(max.load(Ordering::SeqCst), data.len());
        assert!(!part_path(&dest).exists());
    }

    #[test]
    fn corrupt_download_is_discarded_then_mirror_succeeds() {
        let data = body();
        let (bad, _) = serve(data.clone(), Mode::Corrupt);
        let (good, _) = serve(data.clone(), Mode::Ok);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("m.onnx");
        download(
            &[bad.clone(), good],
            &dest,
            data.len() as u64,
            &sha(&data),
            &fast(),
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), data);

        let dest2 = tmp.path().join("m2.onnx");
        let err = download(
            &[bad],
            &dest2,
            data.len() as u64,
            &sha(&data),
            &fast(),
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::ChecksumMismatch);
        assert!(
            !dest2.exists() && !part_path(&dest2).exists(),
            "nessun file corrotto lasciato"
        );
    }

    #[test]
    fn permanent_http_error_moves_to_mirror_and_reports_failure() {
        let data = body();
        let (missing, hits) = serve(data.clone(), Mode::NotFound);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("m.onnx");
        let err = download(
            &[missing],
            &dest,
            data.len() as u64,
            &sha(&data),
            &fast(),
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert_eq!(hits.load(Ordering::SeqCst), 1, "404 non si ritenta");
        let unreachable = "http://127.0.0.1:9/x".to_string();
        let err = download(
            &[unreachable],
            &dest,
            10,
            &sha(b"x"),
            &fast(),
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
    }

    #[test]
    fn failed_transfer_can_resume_in_a_later_job() {
        let data = body();
        let (url, hits) = serve(data.clone(), Mode::InterruptOnce);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("m.onnx");
        let once = RetryPolicy {
            attempts_per_url: 1,
            first_backoff: Duration::ZERO,
        };
        let result = download(
            std::slice::from_ref(&url),
            &dest,
            data.len() as u64,
            &sha(&data),
            &once,
            &|_| {},
            &AtomicBool::new(false),
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::Network);
        assert!(!dest.exists());
        assert_eq!(
            std::fs::metadata(part_path(&dest)).unwrap().len(),
            data.len() as u64 / 2
        );
        download(
            &[url],
            &dest,
            data.len() as u64,
            &sha(&data),
            &once,
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), data);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn cancellation_keeps_partial_file_and_never_publishes_it() {
        let data = body();
        let (url, _) = serve(data.clone(), Mode::Ok);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("m.onnx");
        let control = pc_jobs::JobControl::new();
        let result = download_with_control(
            &[url],
            &dest,
            data.len() as u64,
            &sha(&data),
            &fast(),
            &|bytes| {
                if bytes > 0 {
                    control.cancel();
                }
            },
            &|| control.checkpoint(),
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::Cancelled);
        assert!(part_path(&dest).is_file());
        assert!(!dest.exists());
    }

    #[test]
    fn incorrect_range_is_discarded_before_restarting() {
        let data = body();
        let (url, hits) = serve(data.clone(), Mode::WrongRangeOnce);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("m.onnx");
        std::fs::write(part_path(&dest), &data[..data.len() / 4]).unwrap();
        download(
            &[url],
            &dest,
            data.len() as u64,
            &sha(&data),
            &fast(),
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), data);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }
}
