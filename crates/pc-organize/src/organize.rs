use crate::{paths, Scope};
use pc_core::{AppError, AppResult};
use pc_jobs::{JobPlan, PhaseSpec, RunSummary};
use pc_store::{detail::MediaDetail, Store};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub const JOB_KIND: &str = "organize";
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Symlink,
    Hardlink,
    Copy,
}
impl Method {
    fn name(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Hardlink => "hardlink",
            Self::Copy => "copy",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OrganizeOptions {
    pub scope: Scope,
    pub destination: PathBuf,
    pub levels: Vec<String>,
    pub method: Method,
}
pub type Options = OrganizeOptions;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OrganizeEntry {
    pub media_id: i64,
    pub source: PathBuf,
    pub relative: PathBuf,
    pub size: u64,
}
pub type Entry = OrganizeEntry;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub entries: Vec<Entry>,
    pub folders: BTreeMap<String, u64>,
    pub files: u64,
    pub bytes: u64,
    pub conflicts: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    pub available: bool,
    pub alternative: Method,
    pub developer_mode: bool,
    /// Codice del motivo di indisponibilità, tradotto dalla UI.
    pub reason: Option<String>,
    pub message: String,
}
pub const LEVELS: &[&str] = &[
    "place",
    "year",
    "month",
    "person",
    "subject",
    "location",
    "environment",
    "tag",
    "catalog",
];
pub fn sanitize(value: &str) -> String {
    let mut out = String::new();
    let mut units = 0;
    for c in value.trim().chars() {
        let c = if c.is_control() || "<>:\"/\\|?*".contains(c) {
            '_'
        } else {
            c
        };
        if units + c.len_utf16() > 100 {
            break;
        }
        units += c.len_utf16();
        out.push(c);
    }
    out = out.trim_matches(['.', ' ']).to_string();
    if out.is_empty() {
        out = "Senza valore".into();
    }
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    if reserved {
        out.insert(0, '_');
    }
    out
}
fn values(d: &MediaDetail, level: &str) -> Vec<String> {
    let mut v = match level {
        "place" => d
            .location
            .as_ref()
            .and_then(|l| l.name.clone())
            .into_iter()
            .collect(),
        "year" | "month" => d
            .dates
            .as_ref()
            .and_then(|d| d.captured_at.as_ref())
            .and_then(|s| s.get(..if level == "year" { 4 } else { 7 }))
            .map(str::to_owned)
            .into_iter()
            .collect(),
        "person" => d.people.clone(),
        "tag" => d.tags.clone(),
        "catalog" => d.catalogs.clone(),
        group => d
            .labels
            .iter()
            .filter(|l| l.group == group)
            .map(|l| l.value.clone())
            .collect(),
    };
    v.retain(|s| !s.trim().is_empty());
    v.sort();
    v.dedup();
    if v.is_empty() {
        v.push(format!(
            "Senza {}",
            match level {
                "place" => "luogo",
                "year" => "anno",
                "month" => "mese",
                "person" => "persona",
                "subject" => "soggetto",
                "location" => "locazione",
                "environment" => "ambiente",
                "tag" => "tag",
                _ => "catalogo",
            }
        ));
    }
    v
}
fn suffixed(path: &Path, n: u32) -> PathBuf {
    if n == 1 {
        return path.to_path_buf();
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    path.with_file_name(format!("{stem} ({n}){ext}"))
}
/// Puro: non legge né scrive il file system della destinazione.
pub fn tree(rows: impl IntoIterator<Item = MediaDetail>, levels: &[String]) -> AppResult<Preview> {
    try_tree(rows.into_iter().map(Ok), levels)
}
fn try_tree(
    rows: impl IntoIterator<Item = AppResult<MediaDetail>>,
    levels: &[String],
) -> AppResult<Preview> {
    if levels.is_empty() || levels.len() > 6 || levels.iter().any(|l| !LEVELS.contains(&l.as_str()))
    {
        return Err(AppError::invalid_input("gerarchia non valida"));
    }
    let mut preview = Preview {
        entries: Vec::new(),
        folders: BTreeMap::new(),
        files: 0,
        bytes: 0,
        conflicts: 0,
    };
    let mut used = HashSet::new();
    for d in rows {
        let d = d?;
        let mut folders = vec![PathBuf::new()];
        for level in levels {
            let vals = values(&d, level);
            if folders.len().saturating_mul(vals.len()) > 256 {
                return Err(AppError::invalid_input(
                    "troppi raggruppamenti per una foto (massimo 256)",
                ));
            }
            folders = folders
                .iter()
                .flat_map(|p| vals.iter().map(move |v| p.join(sanitize(v))))
                .collect();
        }
        for folder in folders {
            let base = folder.join(sanitize(&d.item.name));
            let mut n = 1;
            let mut relative = base.clone();
            while !used.insert(relative.to_string_lossy().to_lowercase()) {
                n += 1;
                relative = suffixed(&base, n);
            }
            if n > 1 {
                preview.conflicts += 1;
            }
            *preview
                .folders
                .entry(folder.to_string_lossy().into())
                .or_default() += 1;
            preview.files += 1;
            let size = d.item.size.max(0) as u64;
            preview.bytes = preview.bytes.saturating_add(size);
            preview.entries.push(Entry {
                media_id: d.item.id,
                source: PathBuf::from(&d.item.path),
                relative,
                size,
            });
        }
    }
    Ok(preview)
}
pub fn preview(store: &Store, opts: &Options) -> AppResult<Preview> {
    let ids = crate::ids(store, &opts.scope)?;
    try_tree(
        ids.into_iter().map(|id| {
            store
                .read(|c| pc_store::detail::media_detail(c, id))
                .map_err(Into::into)
        }),
        &opts.levels,
    )
}

/// Modalità sviluppatore di Windows (`None` altrove: i symlink non dipendono da privilegi).
#[cfg(windows)]
fn developer_mode() -> Option<bool> {
    use std::os::windows::process::CommandExt;
    let enabled = std::process::Command::new("reg.exe")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock",
            "/v",
            "AllowDevelopmentWithoutDevLicense",
        ])
        .creation_flags(0x08000000)
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("0x1"))
        .unwrap_or(false);
    Some(enabled)
}
#[cfg(not(windows))]
fn developer_mode() -> Option<bool> {
    None
}
#[cfg(windows)]
fn symlink(source: &Path, dest: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source, dest)
}
#[cfg(unix)]
fn symlink(source: &Path, dest: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}
pub fn preflight(store: &Store, opts: &Options) -> AppResult<Preflight> {
    let dest = paths::writable_destination(store, &opts.destination)?;
    let mut parent = dest.as_path();
    while !parent.is_dir() {
        parent = parent
            .parent()
            .ok_or_else(|| AppError::invalid_input("destinazione non disponibile"))?;
    }
    let test = tempfile::tempdir_in(parent)?;
    let own = test.path().join("probe");
    std::fs::write(&own, b"photo-cat link probe")?;
    let mut sources = Vec::new();
    for id in crate::ids(store, &opts.scope)? {
        sources.push(PathBuf::from(
            store.read(|c| pc_store::repo::media::get(c, id))?.path,
        ));
    }
    let links = |method: Method| -> bool {
        for (i, source) in sources.iter().enumerate() {
            let link = test.path().join(format!("link-{i}"));
            let res = if method == Method::Symlink {
                symlink(&paths::long(source), &link)
            } else {
                std::fs::hard_link(paths::long(source), &link)
            };
            if res.is_err() {
                return false;
            }
            if std::fs::remove_file(&link).is_err() {
                return false;
            }
        }
        true
    };
    let probe = LinkProbe {
        symlink: (opts.method == Method::Symlink).then(|| links(Method::Symlink)),
        // Gli hard link servono come metodo scelto o come alternativa a un symlink non consentito.
        hardlink: (opts.method != Method::Copy).then(|| links(Method::Hardlink)),
        cross_volume: sources.iter().any(|s| !same_volume(s, parent)),
        developer_mode: developer_mode(),
    };
    Ok(decide(opts.method, &probe))
}

/// Esito delle prove di collegamento sulla destinazione (`None` = non provato).
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkProbe {
    pub symlink: Option<bool>,
    pub hardlink: Option<bool>,
    pub cross_volume: bool,
    /// Solo su Windows: `Some(false)` spiega un symlink negato con i privilegi mancanti.
    pub developer_mode: Option<bool>,
}

/// Decide disponibilità, alternativa e motivo leggibile a partire dalle prove.
pub fn decide(method: Method, p: &LinkProbe) -> Preflight {
    let hardlink_ok = p.hardlink.unwrap_or(false);
    let (available,reason)=match method{
        Method::Copy=>(true,None),
        Method::Symlink=>match p.symlink{
            Some(true)=>(true,None),
            _ if p.developer_mode == Some(false)=>(false,Some(("symlink_privilege","I collegamenti simbolici richiedono la Modalità sviluppatore di Windows o privilegi di amministratore."))),
            _=>(false,Some(("symlink_denied","Collegamenti simbolici non consentiti su questa destinazione."))),
        },
        Method::Hardlink=>if hardlink_ok{(true,None)}else if p.cross_volume{(false,Some(("cross_volume","Gli hard link richiedono che sorgenti e destinazione siano sullo stesso volume.")))}else{(false,Some(("hardlink_unsupported","Hard link non supportati su questa destinazione.")))},
    };
    let alternative = if available {
        method
    } else if method == Method::Symlink && hardlink_ok {
        Method::Hardlink
    } else {
        Method::Copy
    };
    let message = match reason {
        None => "Metodo disponibile".to_string(),
        Some((_, r)) => format!(
            "{r} Alternativa proposta: {}.",
            match alternative {
                Method::Hardlink => "hard link",
                _ => "copia",
            }
        ),
    };
    Preflight {
        available,
        alternative,
        developer_mode: p.developer_mode.unwrap_or(false),
        reason: reason.map(|(code, _)| code.to_string()),
        message,
    }
}

/// Stesso volume: su Windows confronta la radice (lettera di unità o condivisione UNC).
#[cfg(windows)]
fn same_volume(a: &Path, b: &Path) -> bool {
    let root = |p: &Path| {
        p.components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
    };
    root(a) == root(b)
}
#[cfg(unix)]
fn same_volume(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev(),
        _ => true,
    }
}
fn hash(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}
#[cfg(windows)]
fn identity(path: &Path) -> AppResult<String> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[repr(C)]
    struct Info {
        attributes: u32,
        creation: FileTime,
        access: FileTime,
        write: FileTime,
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(handle: *mut std::ffi::c_void, info: *mut Info) -> i32;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(7)
        .custom_flags(0x00200000 | 0x02000000)
        .open(path)?;
    let mut info = std::mem::MaybeUninit::<Info>::uninit();
    // Il buffer ha esattamente il layout Win32 BY_HANDLE_FILE_INFORMATION.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let info = unsafe { info.assume_init() };
    Ok(format!(
        "{}:{}:{}:{}:{}",
        info.volume, info.index_high, info.index_low, info.creation.high, info.creation.low
    ))
}
#[cfg(unix)]
fn identity(path: &Path) -> AppResult<String> {
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::symlink_metadata(path)?;
    Ok(format!("{}:{}", m.dev(), m.ino()))
}
fn register(
    store: &Store,
    op: i64,
    path: &Path,
    target: Option<&Path>,
    kind: &str,
) -> AppResult<()> {
    let id = identity(path)?;
    let fingerprint = if kind == "copy" || kind == "hardlink" {
        hash(path)?
    } else {
        String::new()
    };
    let (created, target, kind) = (
        path.to_string_lossy().into_owned(),
        target.map(|p| p.to_string_lossy().into_owned()),
        kind.to_owned(),
    );
    store.write(move|c|{c.execute("INSERT INTO organize_entry(op_id,created,target,kind,identity,fingerprint) VALUES (?1,?2,?3,?4,?5,?6)",rusqlite::params![op,created,target,kind,id,fingerprint])?;Ok(())})?;
    Ok(())
}
fn ensure_dir(store: &Store, op: i64, root: &Path, dir: &Path) -> AppResult<()> {
    let mut missing = Vec::new();
    let mut p = dir;
    while !p.exists() {
        if !paths::within(p, root) {
            return Err(AppError::invalid_input(
                "la cartella genitore della destinazione deve esistere",
            ));
        }
        missing.push(p.to_path_buf());
        p = p
            .parent()
            .ok_or_else(|| AppError::invalid_input("cartella non valida"))?;
    }
    if !paths::within(&paths::resolved(dir)?, root) {
        return Err(AppError::invalid_input("cartella oltre la destinazione"));
    }
    for p in missing.into_iter().rev() {
        match std::fs::create_dir(&p) {
            Ok(()) => register(store, op, &p, None, "dir")?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
/// Copia a blocchi: create_new impedisce sovrascritture anche in presenza di race.
fn copy(source: &Path, dest: &Path, control: &pc_jobs::JobControl) -> AppResult<()> {
    let mut input = std::fs::File::open(source)?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)?;
    let result = (|| {
        let mut buf = [0u8; 256 * 1024];
        loop {
            if !control.checkpoint() {
                return Err(AppError::cancelled());
            }
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            output.write_all(&buf[..n])?;
        }
        output.sync_all()?;
        Ok(())
    })();
    drop(output);
    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
}
pub fn plan(store: Store, params: &serde_json::Value) -> AppResult<JobPlan> {
    let opts: Options = serde_json::from_value(params.clone())?;
    Ok(JobPlan {
        phases: vec![PhaseSpec::new("organizzazione", 1.0)],
        min_memory_mb: 32,
        run: Box::new(move |ctx| {
            let check = preflight(&store, &opts)?;
            if !check.available {
                return Err(AppError::invalid_input(check.message));
            }
            let tree = preview(&store, &opts)?;
            let root = paths::writable_destination(&store, &opts.destination)?;
            let (dest, method, params) = (
                root.to_string_lossy().into_owned(),
                opts.method.name().to_owned(),
                serde_json::to_string(&opts)?,
            );
            let op=store.write(move|c|{c.execute("INSERT INTO organize_op(created_at,dest,method,params,state) VALUES (?1,?2,?3,?4,'running')",rusqlite::params![pc_store::now_ms(),dest,method,params])?;Ok(c.last_insert_rowid())})?;
            let phase = ctx.progress.phase("organizzazione");
            phase.set_total(tree.files as i64);
            phase.start();
            let mut failed = 0;
            for entry in tree.entries {
                if !ctx.control.checkpoint() {
                    break;
                }
                let outcome = (|| -> AppResult<()> {
                    let base = root.join(entry.relative);
                    let parent = base
                        .parent()
                        .ok_or_else(|| AppError::invalid_input("percorso destinazione"))?;
                    ensure_dir(&store, op, &root, parent)?;
                    // Verifica di nuovo le junction prima di creare il file.
                    if !paths::within(&paths::resolved(parent)?, &root) {
                        return Err(AppError::invalid_input("destinazione non più sicura"));
                    }
                    let mut n = 1;
                    loop {
                        let dest = paths::long(&suffixed(&base, n));
                        let src = paths::long(&entry.source);
                        let result = match opts.method {
                            Method::Symlink => symlink(&src, &dest).map_err(Into::into),
                            Method::Hardlink => std::fs::hard_link(&src, &dest).map_err(Into::into),
                            Method::Copy => copy(&src, &dest, &ctx.control),
                        };
                        match result {
                            Ok(()) => {
                                register(&store, op, &dest, Some(&src), opts.method.name())?;
                                break;
                            }
                            Err(_) if std::fs::symlink_metadata(&dest).is_ok() => {
                                n += 1;
                                if n > 100000 {
                                    return Err(AppError::invalid_input("troppi conflitti"));
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(())
                })();
                match outcome {
                    Ok(()) => phase.inc_done(1),
                    Err(e) if e.code == pc_core::ErrorCode::Cancelled => break,
                    Err(e) => {
                        failed += 1;
                        phase.inc_failed(1);
                        tracing::error!(job_id=ctx.job_id,phase="organizzazione",path=%entry.source.display(),code=e.code.as_str(),detail=e.detail,"organizzazione del file fallita");
                    }
                }
            }
            store.write(move |c| {
                c.execute("UPDATE organize_op SET state='done' WHERE id=?1", [op])?;
                Ok(())
            })?;
            phase.finish();
            Ok(RunSummary {
                cancelled: ctx.control.is_cancelled(),
                failed,
            })
        }),
    })
}
/// L'identità del file e l'impronta impediscono di rimuovere sostituzioni/modifiche dell'utente.
pub fn undo(store: &Store, op: i64) -> AppResult<(u64, u64)> {
    let (root,mut entries)=store.read(|c|{
        let root:String=c.query_row("SELECT dest FROM organize_op WHERE id=?1 AND state!='undone'",[op],|r|r.get(0))?;
        let mut q=c.prepare("SELECT rowid,created,target,kind,identity,fingerprint FROM organize_entry WHERE op_id=?1 ORDER BY rowid DESC")?;
        let entries=q.query_map([op],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?)))?.collect::<Result<Vec<_>,_>>()?;Ok((PathBuf::from(root),entries))
    })?;
    let safe_root = paths::writable_destination(store, &root)?;
    let mut removed = 0;
    let mut kept = 0;
    // File prima delle directory, più profonde prima delle genitrici.
    entries.sort_by_key(|e| (e.3 == "dir", std::cmp::Reverse(e.1.len())));
    for (row, path, target, kind, id, fingerprint) in entries {
        let path = PathBuf::from(path);
        let safe = (|| -> AppResult<bool> {
            if !paths::within(&path, &safe_root) {
                return Ok(false);
            }
            let parent = path
                .parent()
                .ok_or_else(|| AppError::invalid_input("genitore"))?;
            if path != safe_root && !paths::within(&paths::resolved(parent)?, &safe_root) {
                return Ok(false);
            }
            if std::fs::symlink_metadata(&path).is_err() {
                return Ok(true);
            }
            if id.as_deref() != Some(identity(&path)?.as_str()) {
                return Ok(false);
            }
            match kind.as_str() {
                "dir" => match std::fs::remove_dir(&path) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {
                        return Ok(false)
                    }
                    Err(e) => return Err(e.into()),
                },
                "symlink" => {
                    if !paths::same(
                        &std::fs::read_link(&path)?,
                        Path::new(target.as_deref().unwrap_or("")),
                    ) {
                        return Ok(false);
                    }
                    std::fs::remove_file(&path)?;
                }
                "copy" | "hardlink" => {
                    if fingerprint.as_deref() != Some(hash(&path)?.as_str()) {
                        return Ok(false);
                    }
                    std::fs::remove_file(&path)?;
                }
                _ => return Ok(false),
            }
            Ok(true)
        })();
        if matches!(safe, Ok(true)) {
            store.write(move |c| {
                c.execute("DELETE FROM organize_entry WHERE rowid=?1", [row])?;
                Ok(())
            })?;
            removed += 1;
        } else {
            kept += 1;
        }
    }
    if kept == 0 {
        store.write(move |c| {
            c.execute("UPDATE organize_op SET state='undone' WHERE id=?1", [op])?;
            Ok(())
        })?;
    }
    Ok((removed, kept))
}
