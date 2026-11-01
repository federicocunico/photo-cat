//! Esportazione e importazione delle identità in un file JSON.
//!
//! Formato (`format = "photo-cat-identities"`, `version = 1`): un oggetto con
//! l'elenco delle identità (uid, nome, nome breve, gruppo, soglia, alias) e,
//! se richiesti, i volti di riferimento con il ritaglio PNG e l'embedding in
//! base64. Un solo file leggibile, senza foto originali.

use super::identities::{self as ids, IdentityInput};
use crate::{StoreError, StoreResult};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "photo-cat-identities";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportFile {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub include_faces: bool,
    pub identities: Vec<ExportIdentity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportIdentity {
    pub uid: String,
    pub name: String,
    pub short_name: String,
    pub group: Option<String>,
    pub threshold: Option<f32>,
    pub aliases: Vec<String>,
    pub created_at: i64,
    #[serde(default)]
    pub faces: Vec<ExportFace>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportFace {
    pub crop_png: String,
    pub model_ver: String,
    pub embedding: Option<String>,
    pub source: String,
}

/// Esito dell'analisi di un file prima dell'importazione.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImportPreview {
    pub total: u32,
    /// Identità nuove (nome non presente).
    pub new: Vec<String>,
    /// Identità con un nome già presente: proposte per l'unione.
    pub conflicts: Vec<ImportConflict>,
    pub faces: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImportConflict {
    pub name: String,
    /// Identità locale con lo stesso nome.
    pub local_uid: String,
    pub incoming_faces: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImportOutcome {
    pub created: u32,
    pub merged: u32,
    pub skipped: u32,
    pub faces: u32,
}

fn f32_b64(v: &[f32]) -> String {
    B64.encode(v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
}

fn b64_f32(s: &str) -> StoreResult<Vec<f32>> {
    let b = B64
        .decode(s)
        .map_err(|e| StoreError::Invalid(format!("embedding non valido: {e}")))?;
    crate::people::vector(&b)
}

pub fn export(c: &Connection, include_faces: bool) -> StoreResult<ExportFile> {
    let all_faces = if include_faces {
        ids::all_faces(c)?
    } else {
        Vec::new()
    };
    let identities = ids::list(c)?
        .into_iter()
        .map(|i| ExportIdentity {
            faces: all_faces
                .iter()
                .filter(|f| f.uid == i.uid)
                .map(|f| ExportFace {
                    crop_png: B64.encode(&f.crop),
                    model_ver: f.model_ver.clone(),
                    embedding: f.embedding.as_deref().map(f32_b64),
                    source: f.source.clone(),
                })
                .collect(),
            uid: i.uid,
            name: i.name,
            short_name: i.short_name,
            group: i.group,
            threshold: i.threshold,
            aliases: i.aliases,
            created_at: i.created_at,
        })
        .collect();
    Ok(ExportFile {
        format: FORMAT.into(),
        version: VERSION,
        exported_at: chrono::Utc::now().to_rfc3339(),
        include_faces,
        identities,
    })
}

pub fn parse(bytes: &[u8]) -> StoreResult<ExportFile> {
    let file: ExportFile = serde_json::from_slice(bytes)?;
    if file.format != FORMAT {
        return Err(StoreError::Invalid(
            "il file non è un'esportazione di identità di photo-cat".into(),
        ));
    }
    if file.version > VERSION {
        return Err(StoreError::Invalid(format!(
            "esportazione di una versione più recente ({})",
            file.version
        )));
    }
    Ok(file)
}

/// Identità locale corrispondente: stesso uid, altrimenti stesso nome.
fn local_match(c: &Connection, i: &ExportIdentity) -> StoreResult<Option<String>> {
    if ids::get(c, &i.uid).is_ok() {
        return Ok(Some(i.uid.clone()));
    }
    ids::find_by_name(c, &i.name)
}

pub fn preview(c: &Connection, file: &ExportFile) -> StoreResult<ImportPreview> {
    let mut out = ImportPreview {
        total: file.identities.len() as u32,
        new: Vec::new(),
        conflicts: Vec::new(),
        faces: 0,
    };
    for i in &file.identities {
        out.faces += i.faces.len() as u32;
        match local_match(c, i)? {
            Some(local_uid) => out.conflicts.push(ImportConflict {
                name: i.name.clone(),
                local_uid,
                incoming_faces: i.faces.len() as u32,
            }),
            None => out.new.push(i.name.clone()),
        }
    }
    Ok(out)
}

fn add_faces(c: &Connection, uid: &str, faces: &[ExportFace]) -> StoreResult<u32> {
    let existing: Vec<Vec<u8>> = ids::all_faces(c)?
        .into_iter()
        .filter(|f| f.uid == uid)
        .map(|f| f.crop)
        .collect();
    let mut n = 0;
    for f in faces {
        let crop = B64
            .decode(&f.crop_png)
            .map_err(|e| StoreError::Invalid(format!("ritaglio non valido: {e}")))?;
        if existing.contains(&crop) {
            continue;
        }
        let emb = f.embedding.as_deref().map(b64_f32).transpose()?;
        ids::add_face(c, uid, &crop, &f.model_ver, emb.as_deref(), &f.source)?;
        n += 1;
    }
    Ok(n)
}

/// Importa il file. Le identità con nome (o uid) già presente vengono unite
/// a quella locale se il loro nome è in `merge`, altrimenti saltate: non si
/// creano mai duplicati.
pub fn apply(c: &Connection, file: &ExportFile, merge: &[String]) -> StoreResult<ImportOutcome> {
    let mut out = ImportOutcome {
        created: 0,
        merged: 0,
        skipped: 0,
        faces: 0,
    };
    for i in &file.identities {
        match local_match(c, i)? {
            Some(local) => {
                if !merge
                    .iter()
                    .any(|m| m.to_lowercase() == i.name.to_lowercase())
                {
                    out.skipped += 1;
                    continue;
                }
                let cur = ids::get(c, &local)?;
                let mut aliases = cur.aliases.clone();
                aliases.extend(i.aliases.iter().cloned());
                if i.name.to_lowercase() != cur.name.to_lowercase() {
                    aliases.push(i.name.clone());
                }
                ids::update(
                    c,
                    &local,
                    &IdentityInput {
                        name: cur.name.clone(),
                        short_name: Some(cur.short_name.clone()),
                        group: cur.group.clone().or_else(|| i.group.clone()),
                        threshold: cur.threshold.or(i.threshold),
                        aliases,
                    },
                )?;
                out.faces += add_faces(c, &local, &i.faces)?;
                out.merged += 1;
            }
            None => {
                let short = if i.short_name.trim().is_empty() {
                    ids::default_short_name(&i.name)
                } else {
                    i.short_name.clone()
                };
                ids::insert(
                    c,
                    &i.uid,
                    i.name.trim(),
                    &short,
                    &IdentityInput {
                        name: i.name.clone(),
                        short_name: Some(short.clone()),
                        group: i.group.clone(),
                        threshold: i.threshold,
                        aliases: i.aliases.clone(),
                    },
                    i.created_at,
                )?;
                out.faces += add_faces(c, &i.uid, &i.faces)?;
                out.created += 1;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppStore;

    fn seed(s: &AppStore) {
        s.with(|c| {
            let g = ids::create(
                c,
                &IdentityInput {
                    name: "Giulia Bianchi".into(),
                    aliases: vec!["Giuly".into()],
                    group: Some("Famiglia".into()),
                    threshold: Some(0.4),
                    ..Default::default()
                },
            )?;
            let mut e = vec![0.0; 128];
            e[3] = 1.0;
            ids::add_face(c, &g, b"\x89PNG-ritaglio", "sface", Some(&e), "file:g.jpg")?;
            ids::create(
                c,
                &IdentityInput {
                    name: "Rita Rossi".into(),
                    aliases: vec!["Nonna Rita".into()],
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn round_trip_into_empty_store() {
        let src = AppStore::open_in_memory().unwrap();
        seed(&src);
        let file = src.with(|c| export(c, true)).unwrap();
        let bytes = serde_json::to_vec(&file).unwrap();

        let dst = AppStore::open_in_memory().unwrap();
        let parsed = parse(&bytes).unwrap();
        let p = dst.with(|c| preview(c, &parsed)).unwrap();
        assert_eq!(p.new, vec!["Giulia Bianchi", "Rita Rossi"]);
        assert!(p.conflicts.is_empty());
        let out = dst.with(|c| apply(c, &parsed, &[])).unwrap();
        assert_eq!((out.created, out.faces), (2, 1));
        let before = src.with(ids::list).unwrap();
        let after = dst.with(ids::list).unwrap();
        assert_eq!(before, after);
        let refs = dst.with(|c| ids::references(c, "sface")).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].centroid[3], 1.0);
    }

    #[test]
    fn export_without_faces_has_no_biometrics() {
        let src = AppStore::open_in_memory().unwrap();
        seed(&src);
        let file = src.with(|c| export(c, false)).unwrap();
        let text = serde_json::to_string(&file).unwrap();
        assert!(file.identities.iter().all(|i| i.faces.is_empty()));
        assert!(!text.contains("crop_png"));
        assert!(text.contains("Nonna Rita"));
    }

    #[test]
    fn same_name_is_proposed_for_merge_never_duplicated() {
        let src = AppStore::open_in_memory().unwrap();
        seed(&src);
        let file = src.with(|c| export(c, true)).unwrap();
        let dst = AppStore::open_in_memory().unwrap();
        dst.with(|c| {
            ids::create(
                c,
                &IdentityInput {
                    name: "giulia bianchi".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();
        let p = dst.with(|c| preview(c, &file)).unwrap();
        assert_eq!(p.conflicts.len(), 1);
        assert_eq!(p.conflicts[0].name, "Giulia Bianchi");
        // Senza scegliere l'unione: saltata.
        let out = dst.with(|c| apply(c, &file, &[])).unwrap();
        assert_eq!((out.created, out.skipped), (1, 1));
        assert_eq!(dst.with(ids::list).unwrap().len(), 2);
        // Con l'unione: alias e volti passano all'identità locale, nessun duplicato.
        let out = dst
            .with(|c| apply(c, &file, &["Giulia Bianchi".into()]))
            .unwrap();
        assert_eq!(out.merged, 1);
        let list = dst.with(ids::list).unwrap();
        assert_eq!(list.len(), 2);
        let g = list.iter().find(|i| i.name == "giulia bianchi").unwrap();
        assert_eq!(g.aliases, vec!["Giuly"]);
        assert_eq!(g.faces, 1);
        assert_eq!(g.group.as_deref(), Some("Famiglia"));
        // Reimportare non duplica nemmeno i volti.
        dst.with(|c| apply(c, &file, &["Giulia Bianchi".into(), "Rita Rossi".into()]))
            .unwrap();
        assert_eq!(
            dst.with(ids::list)
                .unwrap()
                .iter()
                .find(|i| i.name == "giulia bianchi")
                .unwrap()
                .faces,
            1
        );
    }

    #[test]
    fn foreign_files_are_refused() {
        assert!(parse(b"{\"format\":\"altro\",\"version\":1,\"exported_at\":\"\",\"include_faces\":false,\"identities\":[]}").is_err());
        assert!(parse(b"non json").is_err());
    }
}
