//! Apertura del catalogo, writer unico a batch e pool di connessioni in lettura.
//!
//! Tutte le scritture passano dal thread writer, che le raggruppa in
//! transazioni (soglia di operazioni o di tempo). Ogni operazione gira in un
//! savepoint proprio: un errore annulla solo quella operazione.

use crate::{migrate, StoreError};
use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use parking_lot::{Condvar, Mutex};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

type Op = Box<dyn FnOnce(&Connection) -> Result<(), StoreError> + Send>;

enum Msg {
    Op { op: Op, urgent: bool },
    Flush(Sender<bool>),
    Vacuum(Sender<Result<(), StoreError>>),
    Stop,
}

#[derive(Debug, Clone)]
pub struct StoreOptions {
    /// Numero massimo di operazioni per transazione.
    pub batch_ops: usize,
    /// Tempo massimo di attesa prima del commit di un batch.
    pub batch_window: Duration,
    pub read_connections: usize,
    pub backup_dir: Option<PathBuf>,
}

impl Default for StoreOptions {
    fn default() -> Self {
        Self {
            batch_ops: 200,
            batch_window: Duration::from_millis(250),
            read_connections: 4,
            backup_dir: None,
        }
    }
}

/// Catalogo aperto: writer + letture.
#[derive(Clone)]
pub struct Store {
    inner: Arc<StoreInner>,
}

struct StoreInner {
    path: PathBuf,
    tx: Sender<Msg>,
    writer: Mutex<Option<JoinHandle<()>>>,
    readers: Mutex<Vec<Connection>>,
    readers_cv: Condvar,
}

fn configure(conn: &Connection) -> Result<(), StoreError> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.busy_timeout(Duration::from_secs(10))?;
    Ok(())
}

impl Store {
    /// Compatta il DB sul writer, fuori da transazioni, poi tronca il WAL.
    pub fn vacuum(&self) -> Result<(), StoreError> {
        let (tx, rx) = bounded(1);
        self.inner
            .tx
            .send(Msg::Vacuum(tx))
            .map_err(|_| StoreError::Closed)?;
        rx.recv().map_err(|_| StoreError::Closed)?
    }
    pub fn open(path: &Path, opts: StoreOptions) -> Result<Self, StoreError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| StoreError::Schema(e.to_string()))?;
        }
        let mut writer_conn = Connection::open(path)?;
        configure(&writer_conn)?;
        migrate::migrate(&mut writer_conn, opts.backup_dir.as_deref())?;

        let mut readers = Vec::with_capacity(opts.read_connections.max(1));
        for _ in 0..opts.read_connections.max(1) {
            let c = Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_ONLY
                    | OpenFlags::SQLITE_OPEN_URI
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            c.busy_timeout(Duration::from_secs(10))?;
            c.pragma_update(None, "query_only", "ON")?;
            readers.push(c);
        }

        let (tx, rx) = bounded::<Msg>(10_000);
        let batch_ops = opts.batch_ops.max(1);
        let window = opts.batch_window;
        let handle = std::thread::Builder::new()
            .name("pc-store-writer".into())
            .spawn(move || writer_loop(writer_conn, rx, batch_ops, window))
            .map_err(|e| StoreError::Schema(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(StoreInner {
                path: path.to_path_buf(),
                tx,
                writer: Mutex::new(Some(handle)),
                readers: Mutex::new(readers),
                readers_cv: Condvar::new(),
            }),
        })
    }

    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// Accoda una scrittura senza attenderne l'esito (gli errori vanno nei log).
    pub fn submit<F>(&self, op: F) -> Result<(), StoreError>
    where
        F: FnOnce(&Connection) -> Result<(), StoreError> + Send + 'static,
    {
        let logged: Op = Box::new(move |conn| {
            op(conn).inspect_err(|e| {
                tracing::warn!(code = "DATABASE", detail = %e, "scrittura in coda annullata");
            })
        });
        self.inner
            .tx
            .send(Msg::Op {
                op: logged,
                urgent: false,
            })
            .map_err(|_| StoreError::Closed)
    }

    /// Esegue una scrittura e ne attende il risultato (commit incluso).
    pub fn write<T, F>(&self, op: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let slot: Arc<Mutex<Option<Result<T, StoreError>>>> = Arc::new(Mutex::new(None));
        let slot2 = slot.clone();
        let wrapped: Op = Box::new(move |conn| {
            let res = op(conn);
            let status = res.as_ref().map(|_| ()).map_err(Clone::clone);
            *slot2.lock() = Some(res);
            status
        });
        self.inner
            .tx
            .send(Msg::Op {
                op: wrapped,
                urgent: true,
            })
            .map_err(|_| StoreError::Closed)?;
        self.flush()?;
        let result = slot.lock().take().ok_or(StoreError::Closed)?;
        result
    }

    /// Attende che tutte le scritture accodate siano committate.
    pub fn flush(&self) -> Result<(), StoreError> {
        let (done_tx, done_rx) = bounded::<bool>(1);
        self.inner
            .tx
            .send(Msg::Flush(done_tx))
            .map_err(|_| StoreError::Closed)?;
        match done_rx.recv() {
            Ok(true) => Ok(()),
            Ok(false) => Err(StoreError::CommitFailed),
            Err(_) => Err(StoreError::Closed),
        }
    }

    /// Esegue una lettura con una connessione del pool.
    pub fn read<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let conn = {
            let mut pool = self.inner.readers.lock();
            loop {
                if let Some(c) = pool.pop() {
                    break c;
                }
                self.inner.readers_cv.wait(&mut pool);
            }
        };
        let result = f(&conn);
        self.inner.readers.lock().push(conn);
        self.inner.readers_cv.notify_one();
        result
    }

    /// Aggiorna le statistiche del planner (da chiamare dopo import consistenti).
    pub fn optimize(&self) -> Result<(), StoreError> {
        self.write(|c| {
            c.execute_batch("PRAGMA analysis_limit = 1000; PRAGMA optimize;")?;
            Ok(())
        })
    }

    /// Chiude il writer dopo aver committato le scritture pendenti.
    pub fn close(&self) {
        let _ = self.inner.tx.send(Msg::Stop);
        if let Some(h) = self.inner.writer.lock().take() {
            let _ = h.join();
        }
    }
}

impl Drop for StoreInner {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(h) = self.writer.lock().take() {
            let _ = h.join();
        }
    }
}

fn writer_loop(mut conn: Connection, rx: Receiver<Msg>, batch_ops: usize, window: Duration) {
    let mut pending: Vec<Op> = Vec::new();
    let mut waiters: Vec<Sender<bool>> = Vec::new();
    let mut deadline: Option<Instant> = None;
    loop {
        let msg = match deadline {
            Some(d) => match rx.recv_timeout(d.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => Some(Msg::Stop),
            },
            None => Some(rx.recv().unwrap_or(Msg::Stop)),
        };
        let mut stop = false;
        let mut commit_now = msg.is_none();
        match msg {
            Some(Msg::Vacuum(done)) => {
                let ok = commit_batch(&mut conn, &mut pending);
                deadline = None;
                for waiter in waiters.drain(..) {
                    let _ = waiter.send(ok);
                }
                let result = if ok {
                    (|| -> Result<(), StoreError> {
                        conn.execute_batch("VACUUM")?;
                        for _ in 0..5 {
                            let busy: i64 =
                                conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                                    r.get(0)
                                })?;
                            if busy == 0 {
                                return Ok(());
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        Err(StoreError::Invalid(
                            "catalogo occupato durante la pulizia del WAL".into(),
                        ))
                    })()
                } else {
                    Err(StoreError::CommitFailed)
                };
                let _ = done.send(result);
                continue;
            }
            Some(Msg::Op { op, urgent }) => {
                pending.push(op);
                deadline.get_or_insert_with(|| Instant::now() + window);
                commit_now |= urgent && rx.is_empty() || pending.len() >= batch_ops;
            }
            Some(Msg::Flush(w)) => {
                waiters.push(w);
                commit_now = true;
            }
            Some(Msg::Stop) => {
                stop = true;
                commit_now = true;
            }
            None => {}
        }
        if commit_now {
            let ok = commit_batch(&mut conn, &mut pending);
            deadline = None;
            for w in waiters.drain(..) {
                let _ = w.send(ok);
            }
        }
        if stop {
            break;
        }
    }
}

/// Esegue le operazioni in un'unica transazione, ciascuna nel proprio savepoint.
/// Restituisce `false` se il commit della transazione fallisce.
fn commit_batch(conn: &mut Connection, pending: &mut Vec<Op>) -> bool {
    if pending.is_empty() {
        return true;
    }
    let ops = std::mem::take(pending);
    let n = ops.len();
    let result = (|| -> Result<(), StoreError> {
        let mut tx = conn.transaction()?;
        for op in ops {
            let sp = tx.savepoint()?;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| op(&sp)));
            match outcome
                .unwrap_or_else(|_| Err(StoreError::Invalid("panic in una scrittura".into())))
            {
                Ok(()) => sp.commit()?,
                // Il drop del savepoint annulla solo questa operazione; l'errore
                // è già stato consegnato al chiamante o loggato da `submit`.
                Err(_) => drop(sp),
            }
        }
        tx.commit()?;
        Ok(())
    })();
    match result {
        Ok(()) => true,
        Err(e) => {
            tracing::error!(code = "DATABASE", detail = %e, ops = n, "commit del batch fallito");
            false
        }
    }
}
