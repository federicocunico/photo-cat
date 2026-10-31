use pc_core::{AppError, AppResult};
use pc_store::{repo, Store};
use std::path::{Path, PathBuf};
/// Risolve anche antenati esistenti e junction; non crea directory.
pub fn resolved(path: &Path) -> AppResult<PathBuf> {
    if !path.is_absolute() {
        return Err(AppError::invalid_input("percorso assoluto richiesto"));
    }
    if path.exists() {
        return Ok(std::fs::canonicalize(path)?);
    }
    let parent = path
        .parent()
        .ok_or_else(|| AppError::invalid_input("percorso senza genitore"))?;
    let name = path
        .file_name()
        .ok_or_else(|| AppError::invalid_input("nome destinazione mancante"))?;
    if name == ".." || name == "." {
        return Err(AppError::invalid_input("percorso non normalizzato"));
    }
    Ok(resolved(parent)?.join(name))
}
fn key(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        let s = s.to_lowercase();
        if let Some(rest) = s.strip_prefix("//?/unc/") {
            format!("//{rest}")
        } else {
            s.strip_prefix("//?/").unwrap_or(&s).to_owned()
        }
    }
    #[cfg(not(windows))]
    {
        s
    }
}
pub fn same(a: &Path, b: &Path) -> bool {
    key(a) == key(b)
}
pub fn within(path: &Path, root: &Path) -> bool {
    let p = key(path);
    let r = key(root);
    p == r || p.starts_with(&(r.trim_end_matches('/').to_owned() + "/"))
}
/// Tutte le sorgenti, inclusi file selezionati, sono protette da qualsiasi output.
pub fn writable_destination(store: &Store, path: &Path) -> AppResult<PathBuf> {
    let dest = resolved(path)?;
    let protected = store.read(|c| {
        let mut paths: Vec<PathBuf> = repo::source::list(c)?
            .into_iter()
            .map(|s| PathBuf::from(s.path))
            .collect();
        let mut q = c.prepare("SELECT path FROM media_item")?;
        paths.extend(
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(PathBuf::from),
        );
        Ok(paths)
    })?;
    for p in protected {
        let p = resolved(&p).unwrap_or(p);
        if within(&dest, &p) {
            return Err(AppError::invalid_input(
                "la destinazione è dentro una sorgente protetta",
            ));
        }
    }
    Ok(dest)
}
#[cfg(windows)]
pub fn long(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\?\") {
        path.to_path_buf()
    } else if let Some(s) = s.strip_prefix(r"\\") {
        PathBuf::from(format!(r"\\?\UNC\{s}"))
    } else {
        PathBuf::from(format!(r"\\?\{s}"))
    }
}
#[cfg(not(windows))]
pub fn long(path: &Path) -> PathBuf {
    path.to_path_buf()
}
