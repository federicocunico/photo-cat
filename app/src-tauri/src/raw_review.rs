//! Varianti RAW (Fujifilm X-T5): stato del modulo, protocollo `review://` e precaricamento.
//!
//! Le anteprime non passano da `protocol-asset` (che ampliarebbe lo scope dei file leggibili dalla
//! WebView): la UI chiede `review://<chiave>` e il backend serve soltanto file che il *suo stesso*
//! indice di una sessione aperta ammette.
//!
//! # Formato della chiave
//!
//! `<sessione>-<lato>-<scatto>-<preset>`, quattro campi esadecimali minuscoli separati da `-`:
//!
//! * `sessione`: identificatore opaco assegnato dal backend all'apertura della sessione;
//! * `lato`: lato lungo massimo dell'anteprima in pixel, in esadecimale;
//! * `scatto` e `preset`: chiave dello scatto e identificatore del preset, in UTF-8 esadecimale.
//!
//! La chiave non contiene mai un percorso: il file si ricava solo dall'indice della sessione, e
//! l'alfabeto esadecimale la rende immune a codifiche percentuali, separatori e `..`. Sul sistema
//! operativo la UI la trasforma con `convertFileSrc(chiave, "review")`.
//!
//! # Precaricamento
//!
//! Tre classi di lavoro, come in FujiView: le anteprime *interattive* (richieste del protocollo,
//! con un limite di decodifiche contemporanee), il *precaricamento* degli scatti vicini
//! (+1, −1, +2, +3, −2, +4) e le *miniature* della lista. Ogni invio di una classe sostituisce la
//! coda precedente e aumenta la sua generazione: i lavori di generazioni superate vengono scartati
//! prima di decodificare. Le decodifiche già iniziate non si interrompono.

use crate::state::AppState;
use parking_lot::{Condvar, Mutex, RwLock};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_raw_review::preview::{PreviewCache, MAX_EDGE, MIN_EDGE};
use pc_raw_review::scan::{Layout, ReviewIndex};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::http::{header, HeaderValue, Response, StatusCode};
use tauri::Manager;

/// Lato lungo delle anteprime di confronto precaricate.
pub const PREFETCH_EDGE: u32 = 2048;
/// Lato lungo delle miniature della lista degli scatti.
pub const THUMBNAIL_EDGE: u32 = MIN_EDGE;
/// Scostamenti degli scatti vicini da precaricare, in ordine di priorità (+4/−2 come FujiView).
const PREFETCH_OFFSETS: [isize; 6] = [1, -1, 2, 3, -2, 4];
/// Memoria stimata per una decodifica di anteprima (MB), per dimensionare i worker.
const DECODE_COST_MB: u64 = 160;
/// Cache della WebView per le anteprime: breve, perché una variante può essere rigenerata.
const REVIEW_CACHE_CONTROL: &str = "max-age=60";

// ---------------------------------------------------------------------------------------------
// Tipi esposti alla UI
// ---------------------------------------------------------------------------------------------

/// Chiave `review://` di una variante di uno scatto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReviewImageKey {
    pub preset: String,
    /// Da passare a `convertFileSrc(key, "review")`.
    pub key: String,
}

/// Esito di `raw_review_prefetch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchReport {
    /// Anteprime messe in coda per gli scatti vicini.
    pub queued: u32,
    /// Lavori della generazione precedente ancora in coda, annullati da questa richiesta.
    pub cancelled: u32,
}

/// Miniatura di uno scatto della lista.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailKey {
    pub shot_key: String,
    /// `None` se lo scatto non ha varianti da mostrare.
    pub key: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// Sessione aperta
// ---------------------------------------------------------------------------------------------

/// Sessione aperta: tutto ciò che il protocollo può servire per questa sessione.
#[derive(Debug)]
pub struct OpenSession {
    id: String,
    root: PathBuf,
    /// Cartelle in cui devono stare i file serviti: la radice e, se esterna, la cartella delle varianti.
    allowed_roots: Vec<PathBuf>,
    /// Chiavi degli scatti nell'ordine della lista.
    order: Vec<String>,
    position: HashMap<String, usize>,
    /// Per scatto: preset -> file ammesso (dall'indice, mai dall'URL).
    files: HashMap<String, HashMap<String, PathBuf>>,
    guide_preset: Option<String>,
}

impl OpenSession {
    fn from_index(id: String, index: &ReviewIndex) -> AppResult<Self> {
        let root = std::fs::canonicalize(&index.root)?;
        let mut allowed_roots = vec![root.clone()];
        if let Layout::RafWithVariants { output_dir } = &index.layout {
            // La cartella delle varianti può stare fuori dalla radice: va ammessa esplicitamente.
            if let Ok(dir) = std::fs::canonicalize(output_dir) {
                if !allowed_roots.contains(&dir) {
                    allowed_roots.push(dir);
                }
            }
        }
        let mut order = Vec::with_capacity(index.shots.len());
        let mut position = HashMap::new();
        let mut files = HashMap::new();
        for shot in &index.shots {
            position.insert(shot.key.clone(), order.len());
            order.push(shot.key.clone());
            let variants = shot
                .variants
                .iter()
                .map(|v| (v.preset.clone(), PathBuf::from(&v.path)))
                .collect();
            files.insert(shot.key.clone(), variants);
        }
        Ok(Self {
            id,
            root,
            allowed_roots,
            order,
            position,
            files,
            guide_preset: index.guide_preset.clone(),
        })
    }

    /// Identificatore opaco della sessione.
    #[allow(dead_code)] // usato da `raw_review_open` (task 3.2)
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Radice canonica della cartella aperta.
    #[allow(dead_code)] // usato da `raw_review_open` (task 3.2)
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn file(&self, shot: &str, preset: &str) -> Option<&PathBuf> {
        self.files.get(shot)?.get(preset)
    }

    /// Il file, una volta risolti i collegamenti simbolici, sta dentro una cartella ammessa.
    fn is_inside_allowed(&self, file: &Path) -> bool {
        std::fs::canonicalize(file)
            .map(|real| self.allowed_roots.iter().any(|root| real.starts_with(root)))
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------------------------
// Chiave
// ---------------------------------------------------------------------------------------------

fn hex_encode(text: &str) -> String {
    use std::fmt::Write;
    text.bytes().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn hex_decode(text: &str) -> Option<String> {
    if text.is_empty() || !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes = text
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            u8::try_from(hi * 16 + lo).ok()
        })
        .collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

fn is_lower_hex(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Costruisce la chiave `review://` di una variante.
pub fn review_key(session_id: &str, shot: &str, preset: &str, edge: u32) -> String {
    format!(
        "{session_id}-{edge:x}-{}-{}",
        hex_encode(shot),
        hex_encode(preset)
    )
}

/// Chiave scomposta (ancora non verificata contro una sessione).
struct ParsedKey {
    session: String,
    edge: u32,
    shot: String,
    preset: String,
}

fn parse_key(raw: &str) -> Option<ParsedKey> {
    let key = raw.strip_prefix('/').unwrap_or(raw);
    let mut parts = key.split('-');
    let (session, edge, shot, preset) =
        (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || !(is_lower_hex(session) && is_lower_hex(edge)) {
        return None;
    }
    if !(is_lower_hex(shot) && is_lower_hex(preset)) {
        return None;
    }
    Some(ParsedKey {
        session: session.to_string(),
        edge: u32::from_str_radix(edge, 16).ok()?,
        shot: hex_decode(shot)?,
        preset: hex_decode(preset)?,
    })
}

// ---------------------------------------------------------------------------------------------
// Pianificazione dei lavori
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lane {
    Prefetch,
    Thumbnails,
}

struct Job {
    path: PathBuf,
    edge: u32,
    generation: u64,
}

#[derive(Default)]
struct Lanes {
    prefetch: VecDeque<Job>,
    thumbnails: VecDeque<Job>,
    shutdown: bool,
}

impl Lanes {
    fn queue(&mut self, lane: Lane) -> &mut VecDeque<Job> {
        match lane {
            Lane::Prefetch => &mut self.prefetch,
            Lane::Thumbnails => &mut self.thumbnails,
        }
    }
}

/// Contatori osservabili (test).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub struct SchedulerStats {
    pub completed: u64,
    pub skipped_stale: u64,
    pub failed: u64,
}

struct Shared {
    cache: Arc<PreviewCache>,
    lanes: Mutex<Lanes>,
    wake_prefetch: Condvar,
    wake_thumbnails: Condvar,
    prefetch_generation: AtomicU64,
    thumbnail_generation: AtomicU64,
    completed: AtomicU64,
    skipped_stale: AtomicU64,
    failed: AtomicU64,
}

impl Shared {
    fn generation(&self, lane: Lane) -> &AtomicU64 {
        match lane {
            Lane::Prefetch => &self.prefetch_generation,
            Lane::Thumbnails => &self.thumbnail_generation,
        }
    }

    fn wake(&self, lane: Lane) -> &Condvar {
        match lane {
            Lane::Prefetch => &self.wake_prefetch,
            Lane::Thumbnails => &self.wake_thumbnails,
        }
    }

    /// Sostituisce la coda della classe con `jobs` e ne aumenta la generazione.
    /// Restituisce quanti lavori ancora in coda sono stati annullati.
    fn submit(&self, lane: Lane, jobs: Vec<(PathBuf, u32)>) -> usize {
        let generation = self.generation(lane).fetch_add(1, Ordering::AcqRel) + 1;
        let mut lanes = self.lanes.lock();
        let queue = lanes.queue(lane);
        let cancelled = queue.len();
        queue.clear();
        queue.extend(jobs.into_iter().map(|(path, edge)| Job {
            path,
            edge,
            generation,
        }));
        self.wake(lane).notify_all();
        cancelled
    }

    #[cfg(test)]
    fn pop(&self, lane: Lane) -> Option<Job> {
        self.lanes.lock().queue(lane).pop_front()
    }

    /// Esegue un lavoro, a meno che la sua generazione sia stata superata.
    fn run(&self, lane: Lane, job: Job) {
        if job.generation != self.generation(lane).load(Ordering::Acquire) {
            self.skipped_stale.fetch_add(1, Ordering::Relaxed);
            return;
        }
        match self.cache.get(&job.path, job.edge) {
            Ok(_) => self.completed.fetch_add(1, Ordering::Relaxed),
            Err(e) => {
                // Un'anteprima non riuscita al precaricamento non è un errore per l'utente:
                // verrà richiesta di nuovo, in modo interattivo, se serve.
                tracing::debug!(
                    code = e.code.as_str(),
                    detail = e.detail.as_str(),
                    "precaricamento anteprima non riuscito"
                );
                self.failed.fetch_add(1, Ordering::Relaxed)
            }
        };
    }

    fn worker(self: Arc<Self>, lane: Lane) {
        loop {
            let job = {
                let mut lanes = self.lanes.lock();
                loop {
                    if lanes.shutdown {
                        return;
                    }
                    if let Some(job) = lanes.queue(lane).pop_front() {
                        break job;
                    }
                    self.wake(lane).wait(&mut lanes);
                }
            };
            self.run(lane, job);
        }
    }
}

/// Code di precaricamento con i loro worker.
pub struct Scheduler {
    shared: Arc<Shared>,
}

impl Scheduler {
    fn new(cache: Arc<PreviewCache>, prefetch_workers: usize, thumbnail_workers: usize) -> Self {
        let shared = Arc::new(Shared {
            cache,
            lanes: Mutex::new(Lanes::default()),
            wake_prefetch: Condvar::new(),
            wake_thumbnails: Condvar::new(),
            prefetch_generation: AtomicU64::new(0),
            thumbnail_generation: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            skipped_stale: AtomicU64::new(0),
            failed: AtomicU64::new(0),
        });
        for (lane, count, name) in [
            (Lane::Prefetch, prefetch_workers, "raw-review-prefetch"),
            (Lane::Thumbnails, thumbnail_workers, "raw-review-thumbs"),
        ] {
            for n in 0..count {
                let shared = shared.clone();
                let spawned = std::thread::Builder::new()
                    .name(format!("{name}-{n}"))
                    .spawn(move || shared.worker(lane));
                if let Err(e) = spawned {
                    tracing::warn!(detail = %e, "avvio worker anteprime non riuscito");
                }
            }
        }
        Self { shared }
    }

    #[cfg(test)]
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            completed: self.shared.completed.load(Ordering::Relaxed),
            skipped_stale: self.shared.skipped_stale.load(Ordering::Relaxed),
            failed: self.shared.failed.load(Ordering::Relaxed),
        }
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.shared.lanes.lock().shutdown = true;
        self.shared.wake_prefetch.notify_all();
        self.shared.wake_thumbnails.notify_all();
    }
}

/// Limite di decodifiche interattive contemporanee.
struct Slots {
    free: Mutex<usize>,
    freed: Condvar,
}

struct SlotGuard<'a>(&'a Slots);

impl Slots {
    fn new(count: usize) -> Self {
        Self {
            free: Mutex::new(count.max(1)),
            freed: Condvar::new(),
        }
    }

    fn enter(&self) -> SlotGuard<'_> {
        let mut free = self.free.lock();
        while *free == 0 {
            self.freed.wait(&mut free);
        }
        *free -= 1;
        SlotGuard(self)
    }
}

impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        *self.0.free.lock() += 1;
        self.0.freed.notify_one();
    }
}

/// Numero di worker per classe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Workers {
    pub interactive: usize,
    pub prefetch: usize,
    pub thumbnails: usize,
}

impl Workers {
    /// Come FujiView (4 interattivi, 2 di precaricamento, 1 per le miniature) ma limitati dai core
    /// e dalla memoria del governor di photo-cat.
    pub fn for_system(capacity_mb: u64, logical_cores: usize) -> Self {
        let total =
            pc_governor::budget::recommended_workers(capacity_mb, DECODE_COST_MB, logical_cores);
        Self {
            interactive: total.clamp(1, 4),
            prefetch: (total / 2).clamp(1, 2),
            thumbnails: 1,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Stato del modulo
// ---------------------------------------------------------------------------------------------

/// Stato del modulo Varianti RAW, gestito da `AppState`.
pub struct ReviewState {
    cache: Option<Arc<PreviewCache>>,
    scheduler: Option<Scheduler>,
    interactive: Slots,
    sessions: RwLock<HashMap<String, Arc<OpenSession>>>,
    counter: AtomicU64,
}

fn unavailable() -> AppError {
    AppError::new(
        ErrorCode::Io,
        "la cache delle anteprime delle Varianti RAW non è disponibile",
    )
}

fn unknown_session(id: &str) -> AppError {
    AppError::new(
        ErrorCode::NotFound,
        format!("sessione di revisione «{id}» non aperta"),
    )
}

impl ReviewState {
    /// Apre la cache in `<data_dir>/raw-review/previews/` con il limite in byte. Se la cache non si
    /// apre, il modulo resta senza anteprime ma l'app parte comunque.
    pub fn new(data_dir: &Path, limit_bytes: u64, workers: Workers) -> Self {
        match PreviewCache::open(data_dir, limit_bytes) {
            Ok(cache) => Self::with_cache(Arc::new(cache), workers),
            Err(e) => {
                tracing::error!(
                    code = e.code.as_str(),
                    detail = e.detail.as_str(),
                    "cache anteprime Varianti RAW non apribile"
                );
                Self::without_cache(workers)
            }
        }
    }

    fn with_cache(cache: Arc<PreviewCache>, workers: Workers) -> Self {
        let scheduler = Scheduler::new(cache.clone(), workers.prefetch, workers.thumbnails);
        Self {
            cache: Some(cache),
            scheduler: Some(scheduler),
            ..Self::without_cache(workers)
        }
    }

    fn without_cache(workers: Workers) -> Self {
        Self {
            cache: None,
            scheduler: None,
            interactive: Slots::new(workers.interactive),
            sessions: RwLock::new(HashMap::new()),
            counter: AtomicU64::new(0),
        }
    }

    /// Rende una sessione servibile dal protocollo. Una sessione già aperta sulla stessa radice
    /// viene sostituita (le sue chiavi smettono di valere). Restituisce l'identificatore.
    #[allow(dead_code)] // usato da `raw_review_open` (task 3.2)
    pub fn register(&self, index: &ReviewIndex) -> AppResult<Arc<OpenSession>> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let session = Arc::new(OpenSession::from_index(format!("{nanos:x}{n:x}"), index)?);
        let mut sessions = self.sessions.write();
        sessions.retain(|_, s| s.root != session.root);
        sessions.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    /// Chiude la sessione: da qui in poi le sue chiavi vengono rifiutate. Falso se non era aperta.
    pub fn close(&self, session_id: &str) -> bool {
        self.sessions.write().remove(session_id).is_some()
    }

    fn session(&self, id: &str) -> AppResult<Arc<OpenSession>> {
        self.sessions
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| unknown_session(id))
    }

    /// Chiavi `review://` di tutte le varianti di uno scatto, al lato richiesto.
    pub fn image_keys(
        &self,
        session_id: &str,
        shot_key: &str,
        max_edge: u32,
    ) -> AppResult<Vec<ReviewImageKey>> {
        let session = self.session(session_id)?;
        let variants = session.files.get(shot_key).ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                format!("scatto «{shot_key}» sconosciuto"),
            )
        })?;
        let edge = max_edge.clamp(MIN_EDGE, MAX_EDGE);
        let mut keys: Vec<ReviewImageKey> = variants
            .keys()
            .map(|preset| ReviewImageKey {
                preset: preset.clone(),
                key: review_key(&session.id, shot_key, preset, edge),
            })
            .collect();
        keys.sort_by(|a, b| natord::compare(&a.preset, &b.preset));
        Ok(keys)
    }

    /// Precarica gli scatti vicini a `center_shot_key` per i preset dei pannelli. Annulla le
    /// anteprime ancora in coda dell'invio precedente (l'utente ha cambiato scatto).
    pub fn prefetch(
        &self,
        session_id: &str,
        center_shot_key: &str,
        presets: &[String],
    ) -> AppResult<PrefetchReport> {
        let session = self.session(session_id)?;
        let scheduler = self.scheduler.as_ref().ok_or_else(unavailable)?;
        let center = *session.position.get(center_shot_key).ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                format!("scatto «{center_shot_key}» sconosciuto"),
            )
        })?;
        let mut jobs = Vec::new();
        for offset in PREFETCH_OFFSETS {
            let Some(shot) = center
                .checked_add_signed(offset)
                .and_then(|i| session.order.get(i))
            else {
                continue;
            };
            for preset in presets {
                if let Some(path) = session.file(shot, preset) {
                    jobs.push((path.clone(), PREFETCH_EDGE));
                }
            }
        }
        let queued = jobs.len();
        let cancelled = scheduler.shared.submit(Lane::Prefetch, jobs);
        Ok(PrefetchReport {
            queued: u32::try_from(queued).unwrap_or(u32::MAX),
            cancelled: u32::try_from(cancelled).unwrap_or(u32::MAX),
        })
    }

    /// Chiavi delle miniature degli scatti indicati (preset guida, altrimenti la prima variante)
    /// e accodamento della loro generazione al posto di quella precedente.
    pub fn thumbnails(
        &self,
        session_id: &str,
        shot_keys: &[String],
    ) -> AppResult<Vec<ThumbnailKey>> {
        let session = self.session(session_id)?;
        let scheduler = self.scheduler.as_ref().ok_or_else(unavailable)?;
        let mut jobs = Vec::new();
        let mut out = Vec::with_capacity(shot_keys.len());
        for shot in shot_keys {
            let variants = session.files.get(shot);
            let chosen = variants.and_then(|v| {
                session
                    .guide_preset
                    .as_ref()
                    .and_then(|g| v.get_key_value(g))
                    .or_else(|| v.iter().min_by(|a, b| natord::compare(a.0, b.0)))
            });
            if let Some((preset, path)) = chosen {
                jobs.push((path.clone(), THUMBNAIL_EDGE));
                out.push(ThumbnailKey {
                    shot_key: shot.clone(),
                    key: Some(review_key(&session.id, shot, preset, THUMBNAIL_EDGE)),
                });
            } else {
                out.push(ThumbnailKey {
                    shot_key: shot.clone(),
                    key: None,
                });
            }
        }
        scheduler.shared.submit(Lane::Thumbnails, jobs);
        Ok(out)
    }

    /// Contatori dei worker di precaricamento.
    #[cfg(test)]
    pub fn stats(&self) -> Option<SchedulerStats> {
        self.scheduler.as_ref().map(Scheduler::stats)
    }

    /// Risolve la chiave in un file ammesso. Il percorso del file non arriva mai dall'URL.
    fn resolve(&self, raw_path: &str) -> Result<(PathBuf, u32), StatusCode> {
        let key = parse_key(raw_path).ok_or(StatusCode::BAD_REQUEST)?;
        if !(MIN_EDGE..=MAX_EDGE).contains(&key.edge) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let session = self
            .sessions
            .read()
            .get(&key.session)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)?;
        let file = session
            .file(&key.shot, &key.preset)
            .ok_or(StatusCode::NOT_FOUND)?;
        if !session.is_inside_allowed(file) {
            // Il file indicizzato è sparito o, tramite un collegamento, esce dalle cartelle ammesse.
            return Err(if file.exists() {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::NOT_FOUND
            });
        }
        Ok((file.clone(), key.edge))
    }

    /// Risposta di `review://<chiave>`. Bloccante: chiamare fuori dal thread principale.
    pub fn serve(&self, raw_path: &str) -> Response<Vec<u8>> {
        let (file, edge) = match self.resolve(raw_path) {
            Ok(found) => found,
            Err(status) => {
                if status != StatusCode::NOT_FOUND {
                    tracing::warn!(%status, "richiesta review:// rifiutata");
                }
                return reply(status, Vec::new());
            }
        };
        let Some(cache) = &self.cache else {
            return reply(StatusCode::SERVICE_UNAVAILABLE, Vec::new());
        };
        let result = {
            let _slot = self.interactive.enter();
            cache.get(&file, edge)
        };
        match result.and_then(|p| std::fs::read(p.cache_path).map_err(AppError::from)) {
            Ok(bytes) => reply(StatusCode::OK, bytes),
            Err(e) => {
                tracing::warn!(
                    code = e.code.as_str(),
                    detail = e.detail.as_str(),
                    "anteprima review:// non generata"
                );
                reply(
                    if e.code == ErrorCode::NotFound {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::INTERNAL_SERVER_ERROR
                    },
                    Vec::new(),
                )
            }
        }
    }
}

/// Risposta con gli header del protocollo; senza corpo per gli errori.
fn reply(status: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    let ok = status == StatusCode::OK;
    let mut response = Response::new(body);
    *response.status_mut() = status;
    let headers = response.headers_mut();
    if ok {
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(REVIEW_CACHE_CONTROL),
        );
    } else {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

/// `review://<chiave>`: anteprime delle sole varianti di sessioni aperte, fuori dal thread della UI.
pub fn review_protocol(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let handle = ctx.app_handle().clone();
    let Some(state) = handle.try_state::<AppState>() else {
        let mut response = reply(StatusCode::SERVICE_UNAVAILABLE, Vec::new());
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
        responder.respond(response);
        return;
    };
    let review = state.raw_review.clone();
    let path = request.uri().path().to_string();
    tauri::async_runtime::spawn_blocking(move || responder.respond(review.serve(&path)));
}

// ---------------------------------------------------------------------------------------------
// Comandi
// ---------------------------------------------------------------------------------------------

/// Chiavi `review://` di tutte le varianti di uno scatto (il lato è limitato a 384–8192 px).
#[tauri::command]
#[specta::specta]
pub async fn raw_review_image_keys(
    app: tauri::AppHandle,
    session_id: String,
    shot_key: String,
    max_edge: u32,
) -> AppResult<Vec<ReviewImageKey>> {
    crate::state::blocking(app, move |_app, state| {
        state
            .raw_review
            .image_keys(&session_id, &shot_key, max_edge)
    })
    .await
}

/// Precarica gli scatti vicini allo scatto corrente per i preset dei pannelli, annullando il
/// precaricamento precedente non ancora eseguito.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_prefetch(
    app: tauri::AppHandle,
    session_id: String,
    center_shot_key: String,
    presets: Vec<String>,
) -> AppResult<PrefetchReport> {
    crate::state::blocking(app, move |_app, state| {
        state
            .raw_review
            .prefetch(&session_id, &center_shot_key, &presets)
    })
    .await
}

/// Chiavi delle miniature degli scatti visibili e relativa generazione in background.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_thumbnails(
    app: tauri::AppHandle,
    session_id: String,
    shot_keys: Vec<String>,
) -> AppResult<Vec<ThumbnailKey>> {
    crate::state::blocking(app, move |_app, state| {
        state.raw_review.thumbnails(&session_id, &shot_keys)
    })
    .await
}

/// Chiude una sessione di revisione: le sue chiavi `review://` non valgono più.
#[tauri::command]
#[specta::specta]
pub async fn raw_review_close(app: tauri::AppHandle, session_id: String) -> AppResult<bool> {
    crate::state::blocking(app, move |_app, state| {
        Ok(state.raw_review.close(&session_id))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_raw_review::scan::scan;
    use std::fs;

    fn jpeg(path: &Path, w: u32, h: u32) {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 251) as u8, (y % 241) as u8, 90])
        });
        img.save_with_format(path, image::ImageFormat::Jpeg)
            .unwrap();
    }

    struct Fixture {
        _tmp: tempfile::TempDir,
        photos: PathBuf,
        state: ReviewState,
        index: ReviewIndex,
    }

    /// Cartella di foto nella forma "una sottocartella per preset", cache in una cartella a parte.
    fn fixture(shots: usize, workers: Workers) -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let photos = tmp.path().join("foto");
        for preset in ["Velvia", "Acros"] {
            fs::create_dir_all(photos.join(preset)).unwrap();
            for n in 1..=shots {
                jpeg(
                    &photos.join(preset).join(format!("DSCF{n:04}.jpg")),
                    800,
                    600,
                );
            }
        }
        let index = scan(&photos, None).unwrap();
        let state = ReviewState::new(&tmp.path().join("dati"), 1 << 30, workers);
        Fixture {
            _tmp: tmp,
            photos,
            state,
            index,
        }
    }

    const NO_WORKERS: Workers = Workers {
        interactive: 2,
        prefetch: 0,
        thumbnails: 0,
    };

    fn tree(dir: &Path) -> Vec<(PathBuf, u64)> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(d).unwrap().flatten() {
                let meta = e.metadata().unwrap();
                if meta.is_dir() {
                    stack.push(e.path());
                } else {
                    out.push((e.path(), meta.len()));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn key_roundtrip_and_alphabet() {
        let key = review_key("ab12", "dscf 0001 è", "raw-preview", 2048);
        assert!(key.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'));
        let parsed = parse_key(&format!("/{key}")).unwrap();
        assert_eq!(parsed.session, "ab12");
        assert_eq!(parsed.edge, 2048);
        assert_eq!(parsed.shot, "dscf 0001 è");
        assert_eq!(parsed.preset, "raw-preview");
    }

    #[test]
    fn open_session_key_is_served_as_jpeg() {
        let f = fixture(2, NO_WORKERS);
        let session = f.state.register(&f.index).unwrap();
        let keys = f.state.image_keys(session.id(), "dscf0001", 1024).unwrap();
        assert_eq!(keys.len(), 2);
        let resp = f.state.serve(&format!("/{}", keys[0].key));
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.headers()["Content-Type"], "image/jpeg");
        assert_eq!(resp.headers()["Cache-Control"], REVIEW_CACHE_CONTROL);
        assert_eq!(&resp.body()[..2], &[0xFF, 0xD8], "inizio JPEG");
    }

    #[test]
    fn foreign_malformed_and_traversal_keys_are_rejected() {
        let f = fixture(1, NO_WORKERS);
        let session = f.state.register(&f.index).unwrap();
        let sid = session.id().to_string();
        let status = |p: &str| f.state.serve(p).status();
        // Sessione mai aperta.
        assert_eq!(
            status(&review_key("deadbeef", "dscf0001", "Acros", 1024)),
            404
        );
        // Scatto o preset che la sessione non conosce.
        assert_eq!(status(&review_key(&sid, "altro", "Acros", 1024)), 404);
        assert_eq!(status(&review_key(&sid, "dscf0001", "Sepia", 1024)), 404);
        // Lato fuori dai limiti.
        assert_eq!(status(&review_key(&sid, "dscf0001", "Acros", 100_000)), 400);
        // Percorsi, traversal, codifiche e chiavi malformate.
        for bad in [
            "../../etc/passwd",
            "/..%2F..%2Fsegreto.jpg",
            "C:/Windows/win.ini",
            "/",
            "",
            "/zz-400-aa-bb",
            "/ab-400-aa",
            "/ab-400-aa-bb-cc",
            "/ab-400-a-bb",
        ] {
            assert_eq!(status(bad), 400, "{bad}");
        }
        // Un preset di traversal codificato in esadecimale non è un percorso: è solo un preset ignoto.
        assert_eq!(
            status(&review_key(&sid, "dscf0001", "../../segreto", 1024)),
            404
        );
        assert_eq!(status(&review_key(&sid, "../x", "Acros", 1024)), 404);
    }

    #[test]
    fn closed_and_replaced_sessions_are_rejected() {
        let f = fixture(1, NO_WORKERS);
        let first = f.state.register(&f.index).unwrap();
        let key = f.state.image_keys(first.id(), "dscf0001", 1024).unwrap()[0]
            .key
            .clone();
        assert_eq!(f.state.serve(&key).status(), 200);
        // Riaprire la stessa radice sostituisce la sessione: la vecchia chiave smette di valere.
        let second = f.state.register(&f.index).unwrap();
        assert_ne!(first.id(), second.id());
        assert_eq!(f.state.serve(&key).status(), 404);
        let key2 = f.state.image_keys(second.id(), "dscf0001", 1024).unwrap()[0]
            .key
            .clone();
        assert_eq!(f.state.serve(&key2).status(), 200);
        assert!(f.state.close(second.id()));
        assert!(!f.state.close(second.id()));
        assert_eq!(f.state.serve(&key2).status(), 404);
        assert!(f.state.image_keys(second.id(), "dscf0001", 1024).is_err());
    }

    #[test]
    fn file_outside_allowed_roots_is_forbidden() {
        let f = fixture(1, NO_WORKERS);
        // Indice manomesso: una variante che punta fuori dalla radice (simula un collegamento).
        let outside = f._tmp.path().join("fuori.jpg");
        jpeg(&outside, 800, 600);
        let mut index = f.index.clone();
        index.shots[0].variants[0].path = outside.to_string_lossy().into_owned();
        let preset = index.shots[0].variants[0].preset.clone();
        let session = f.state.register(&index).unwrap();
        let key = review_key(session.id(), "dscf0001", &preset, 1024);
        assert_eq!(f.state.serve(&key).status(), 403);
    }

    #[test]
    fn nothing_is_written_in_the_photo_folder() {
        let f = fixture(
            3,
            Workers {
                interactive: 2,
                prefetch: 1,
                thumbnails: 1,
            },
        );
        let before = tree(&f.photos);
        let session = f.state.register(&f.index).unwrap();
        for shot in ["dscf0001", "dscf0002"] {
            for k in f.state.image_keys(session.id(), shot, 1024).unwrap() {
                assert_eq!(f.state.serve(&k.key).status(), 200);
            }
        }
        f.state
            .prefetch(session.id(), "dscf0002", &["Acros".into(), "Velvia".into()])
            .unwrap();
        f.state
            .thumbnails(session.id(), &["dscf0001".into(), "dscf0003".into()])
            .unwrap();
        // Attende la fine dei lavori in background.
        for _ in 0..200 {
            if f.state.stats().unwrap().completed >= 1 {
                std::thread::sleep(std::time::Duration::from_millis(50));
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert_eq!(tree(&f.photos), before);
    }

    #[test]
    fn prefetch_orders_neighbours_and_cancels_the_stale_generation() {
        let f = fixture(8, NO_WORKERS);
        let session = f.state.register(&f.index).unwrap();
        let presets = vec!["Acros".to_string(), "Nessuno".to_string()];
        // Centro sul terzo scatto: +1, -1, +2, +3, -2, +4 -> 6 scatti x 1 preset esistente.
        let first = f
            .state
            .prefetch(session.id(), "dscf0003", &presets)
            .unwrap();
        assert_eq!(
            first,
            PrefetchReport {
                queued: 6,
                cancelled: 0
            }
        );
        // L'utente cambia scatto prima che i worker abbiano fatto qualcosa: la coda si sostituisce.
        let second = f
            .state
            .prefetch(session.id(), "dscf0008", &presets)
            .unwrap();
        assert_eq!(
            second,
            PrefetchReport {
                queued: 2,
                cancelled: 6
            }
        );
        let shared = &f.state.scheduler.as_ref().unwrap().shared;
        // Un lavoro già estratto dalla coda ma di una generazione superata viene scartato.
        let stale = Job {
            path: f.photos.join("Acros").join("DSCF0001.jpg"),
            edge: PREFETCH_EDGE,
            generation: 1,
        };
        shared.run(Lane::Prefetch, stale);
        assert_eq!(
            f.state.stats().unwrap(),
            SchedulerStats {
                completed: 0,
                skipped_stale: 1,
                failed: 0
            }
        );
        // I lavori correnti si eseguono (scatti 7 e... solo -1 (7) e -2 (6)).
        while let Some(job) = shared.pop(Lane::Prefetch) {
            shared.run(Lane::Prefetch, job);
        }
        let stats = f.state.stats().unwrap();
        assert_eq!(stats.completed, 2);
        assert_eq!(stats.skipped_stale, 1);
    }

    #[test]
    fn prefetch_rejects_unknown_session_and_shot() {
        let f = fixture(2, NO_WORKERS);
        let session = f.state.register(&f.index).unwrap();
        let e = f.state.prefetch("nope", "dscf0001", &[]).unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
        let e = f.state.prefetch(session.id(), "assente", &[]).unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
    }

    #[test]
    fn thumbnails_use_the_guide_preset_and_queue_work() {
        let f = fixture(2, NO_WORKERS);
        let session = f.state.register(&f.index).unwrap();
        let out = f
            .state
            .thumbnails(session.id(), &["dscf0002".into(), "assente".into()])
            .unwrap();
        assert_eq!(out.len(), 2);
        let guide = f.index.guide_preset.clone().unwrap();
        assert_eq!(
            out[0].key.as_deref(),
            Some(review_key(session.id(), "dscf0002", &guide, THUMBNAIL_EDGE).as_str())
        );
        assert_eq!(out[1].key, None);
        let shared = &f.state.scheduler.as_ref().unwrap().shared;
        let job = shared.pop(Lane::Thumbnails).unwrap();
        shared.run(Lane::Thumbnails, job);
        assert_eq!(f.state.stats().unwrap().completed, 1);
    }

    #[test]
    fn workers_follow_cores_and_memory() {
        assert_eq!(
            Workers::for_system(64_000, 32),
            Workers {
                interactive: 4,
                prefetch: 2,
                thumbnails: 1
            }
        );
        assert_eq!(
            Workers::for_system(200, 2),
            Workers {
                interactive: 1,
                prefetch: 1,
                thumbnails: 1
            }
        );
    }
}
