//! Comandi della schermata Identità e del collegamento "Chi è?" in Persone.
//! Gli embedding non arrivano mai alla UI: solo nomi, conteggi e ritagli.
use crate::state::AppState;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use pc_core::{AppError, AppResult};
use pc_store::app_store::identities::{self as ids, Identity, IdentityInput, RefFaceView};
use pc_store::app_store::transfer::{self, ImportOutcome, ImportPreview};
use pc_store::{people, AppStore, Store};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentityRow {
    #[serde(flatten)]
    pub identity: Identity,
    /// Persona collegata nella libreria aperta (`None` = non collegata).
    pub person: Option<i64>,
    /// Foto della persona collegata nella libreria aperta.
    pub photos: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentitiesView {
    pub identities: Vec<IdentityRow>,
    pub groups: Vec<String>,
    /// Dove si trova `app.db` (contiene dati biometrici).
    pub store_path: String,
    pub default_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FaceChoice {
    pub index: u32,
    /// Ritaglio allineato come data URL PNG.
    pub crop: String,
    pub bbox: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum AddReferenceOutcome {
    Added {
        id: i64,
    },
    /// Più volti nell'immagine: va indicato quale usare.
    Choose {
        faces: Vec<FaceChoice>,
    },
    NoFace,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentitySuggestion {
    pub uid: String,
    pub name: String,
    pub short_name: String,
    /// Somiglianza (coseno) con i volti del gruppo, se calcolabile.
    pub score: Option<f32>,
    /// Persona già collegata nella libreria.
    pub person: Option<i64>,
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", B64.encode(png))
}

/// All'apertura della libreria: persone con nome → identità (una volta sola)
/// e nomi delle persone collegate riallineati alle identità.
pub fn on_library_open(store: &Store, app: &AppStore) {
    let a = app.clone();
    let res = store.write(move |c| {
        let linked =
            people::migrate_to_identities(c, |name| a.with(|ac| ids::ensure_by_name(ac, name)))?;
        let names: HashMap<String, String> = a
            .with(ids::list)?
            .into_iter()
            .map(|i| (i.uid, i.name))
            .collect();
        let synced = people::sync_identity_names(c, &names)?;
        Ok((linked, synced))
    });
    match res {
        Ok((linked, synced)) if linked + synced > 0 => {
            tracing::info!(count = linked, synced, "persone collegate alle identità")
        }
        Err(e) => {
            tracing::error!(code = "DATABASE", detail = %e, "collegamento persone e identità")
        }
        _ => {}
    }
}

pub fn view(db: &Store, app: &AppStore, query: Option<&str>) -> AppResult<IdentitiesView> {
    let (list, groups) =
        app.with(|c| Ok((ids::search(c, query.unwrap_or(""))?, ids::groups(c)?)))?;
    let photos = db.read(people::identity_photos)?;
    let persons: HashMap<String, i64> = db.read(|c| {
        let mut q =
            c.prepare("SELECT identity_uid, MIN(id) FROM person WHERE identity_uid IS NOT NULL GROUP BY identity_uid")?;
        let rows = q
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    })?;
    Ok(IdentitiesView {
        identities: list
            .into_iter()
            .map(|i| IdentityRow {
                person: persons.get(&i.uid).copied(),
                photos: photos.get(&i.uid).copied().unwrap_or(0),
                identity: i,
            })
            .collect(),
        groups,
        store_path: app
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        default_threshold: ids::DEFAULT_THRESHOLD,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn list_identities(
    state: State<'_, AppState>,
    query: Option<String>,
) -> AppResult<IdentitiesView> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || view(&db, &app, query.as_deref())).await
}

/// Riallinea il nome delle persone collegate dopo una modifica alle identità.
fn sync_names(db: &Store, app: &AppStore) -> AppResult<()> {
    let names: HashMap<String, String> = app
        .with(ids::list)?
        .into_iter()
        .map(|i| (i.uid, i.name))
        .collect();
    db.write(move |c| people::sync_identity_names(c, &names).map(|_| ()))?;
    Ok(())
}

/// Crea (senza `uid`) o modifica un'identità. Un nome già usato è un errore.
#[tauri::command]
#[specta::specta]
pub async fn save_identity(
    state: State<'_, AppState>,
    uid: Option<String>,
    input: IdentityInput,
) -> AppResult<String> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let uid = app.tx(|c| match &uid {
            Some(u) => ids::update(c, u, &input).map(|_| u.clone()),
            None => ids::create(c, &input),
        })?;
        sync_names(&db, &app)?;
        Ok(uid)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_identity(state: State<'_, AppState>, uid: String) -> AppResult<()> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        app.tx(|c| ids::delete(c, &uid))?;
        sync_names(&db, &app)
    })
    .await
}

/// Unisce `from` in `into`: alias, volti di riferimento e collegamenti.
#[tauri::command]
#[specta::specta]
pub async fn merge_identities(
    state: State<'_, AppState>,
    from: String,
    into: String,
) -> AppResult<()> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let target = app.with(|c| ids::get(c, &into))?;
        app.tx(|c| ids::merge(c, &from, &into))?;
        db.write(move |c| people::relink_identity(c, &from, &target.uid, &target.name))?;
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub fn identity_faces(state: State<'_, AppState>, uid: String) -> AppResult<Vec<RefFaceView>> {
    Ok(state.app_store.with(|c| ids::faces(c, &uid))?)
}

/// Ritaglio di un volto di riferimento (data URL PNG).
#[tauri::command]
#[specta::specta]
pub fn identity_face_image(state: State<'_, AppState>, id: i64) -> AppResult<String> {
    Ok(data_url(&state.app_store.with(|c| ids::face_crop(c, id))?))
}

#[tauri::command]
#[specta::specta]
pub fn remove_identity_face(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    Ok(state.app_store.with(|c| ids::remove_face(c, id))?)
}

fn faces_ready(state: &AppState) -> AppResult<()> {
    if !state.config.read().ai.faces_enabled {
        return Err(AppError::invalid_input(
            "il riconoscimento volti è disattivato nelle impostazioni",
        ));
    }
    Ok(())
}

fn resources(state: &AppState) -> pc_ai::stages::AiResources {
    pc_ai::stages::AiResources {
        models: state.model_store.clone(),
        pool: state.model_pool.clone(),
        governor: state.governor.clone(),
    }
}

/// Core di "aggiungi riferimento da file": con un solo volto lo salva, con più
/// volti chiede quale usare (`choice` = indice scelto). Il file non viene copiato.
pub fn add_reference_from_path(
    resources: &pc_ai::stages::AiResources,
    app: &AppStore,
    uid: &str,
    path: &Path,
    choice: Option<u32>,
    min_pixels: u32,
) -> AppResult<AddReferenceOutcome> {
    app.with(|c| ids::get(c, uid))?;
    let found = pc_ai::face_analysis::candidates_from_file(
        resources,
        &resources.governor,
        path,
        min_pixels,
    )?;
    let pick = match (found.len(), choice) {
        (0, _) => return Ok(AddReferenceOutcome::NoFace),
        (1, None) => 0,
        (_, Some(i)) if (i as usize) < found.len() => i as usize,
        (_, Some(_)) => return Err(AppError::invalid_input("volto scelto non valido")),
        (_, None) => {
            return Ok(AddReferenceOutcome::Choose {
                faces: found
                    .iter()
                    .enumerate()
                    .map(|(i, c)| FaceChoice {
                        index: i as u32,
                        crop: data_url(&c.crop_png),
                        bbox: c.bbox,
                    })
                    .collect(),
            })
        }
    };
    let c = &found[pick];
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let id = app.with(|a| {
        ids::add_face(
            a,
            uid,
            &c.crop_png,
            &c.model_ver,
            Some(&c.embedding),
            &format!("file:{name}"),
        )
    })?;
    Ok(AddReferenceOutcome::Added { id })
}

#[tauri::command]
#[specta::specta]
pub async fn add_identity_reference_file(
    state: State<'_, AppState>,
    uid: String,
    path: String,
    choice: Option<u32>,
) -> AppResult<AddReferenceOutcome> {
    faces_ready(&state)?;
    let res = resources(&state);
    let app = state.app_store.clone();
    let min = state.config.read().ai.face_min_size_px;
    blocking(move || add_reference_from_path(&res, &app, &uid, Path::new(&path), choice, min)).await
}

/// Volto della libreria come riferimento (ritaglio allineato dall'originale).
#[tauri::command]
#[specta::specta]
pub async fn add_identity_reference_face(
    state: State<'_, AppState>,
    uid: String,
    face: i64,
) -> AppResult<i64> {
    faces_ready(&state)?;
    let res = resources(&state);
    let (db, app, gov) = (
        state.store.clone(),
        state.app_store.clone(),
        state.governor.clone(),
    );
    blocking(move || {
        app.with(|c| ids::get(c, &uid))?;
        let c = pc_ai::face_analysis::candidate_from_library_face(&res, &db, &gov, face)?;
        Ok(app.with(|a| {
            ids::add_face(
                a,
                &uid,
                &c.crop_png,
                &c.model_ver,
                Some(&c.embedding),
                &format!("library:{face}"),
            )
        })?)
    })
    .await
}

/// Collega un gruppo di volti a un'identità (o lo scollega con `None`).
/// Restituisce la persona risultante (diversa se le persone sono state unite).
#[tauri::command]
#[specta::specta]
pub async fn link_person_identity(
    state: State<'_, AppState>,
    person: i64,
    uid: Option<String>,
) -> AppResult<i64> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let target = uid.map(|u| app.with(|c| ids::get(c, &u))).transpose()?;
        Ok(db.write(move |c| {
            people::link_identity(
                c,
                person,
                target.as_ref().map(|t| (t.uid.as_str(), t.name.as_str())),
            )
        })?)
    })
    .await
}

/// Proposte per "Chi è?": identità con nome o alias che contengono `query`,
/// ordinate per somiglianza con i volti del gruppo.
pub fn suggestions(
    db: &Store,
    app: &AppStore,
    person: i64,
    query: &str,
) -> AppResult<Vec<IdentitySuggestion>> {
    let found = app.with(|c| ids::search(c, query))?;
    // Centroide della persona per modello.
    let centroids: Vec<(String, Vec<f32>)> = db.read(move |c| {
        let mut q =
            c.prepare("SELECT model_ver, vector FROM person_centroid WHERE person_id = ?1")?;
        let rows = q
            .query_map([person], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(m, b)| Ok((m, people::vector(&b)?)))
            .collect()
    })?;
    let mut refs: HashMap<String, Vec<ids::Reference>> = HashMap::new();
    for (model, _) in &centroids {
        refs.insert(model.clone(), app.with(|c| ids::references(c, model))?);
    }
    let mut out: Vec<IdentitySuggestion> = Vec::new();
    for i in found {
        let mut score: Option<f32> = None;
        for (model, v) in &centroids {
            if let Some(r) = refs
                .get(model)
                .and_then(|l| l.iter().find(|r| r.uid == i.uid))
            {
                let s = people::cosine(v, &r.centroid);
                score = Some(score.map_or(s, |x: f32| x.max(s)));
            }
        }
        let uid = i.uid.clone();
        let linked = db.read(move |c| people::person_for_identity(c, &uid))?;
        if score.is_none() {
            if let Some(other) = linked.filter(|p| *p != person) {
                // Somiglianza con la persona già collegata in questa libreria.
                let theirs: Vec<(String, Vec<u8>)> = db.read(move |c| {
                    let mut q = c.prepare(
                        "SELECT model_ver, vector FROM person_centroid WHERE person_id = ?1",
                    )?;
                    let rows = q
                        .query_map([other], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(rows)
                })?;
                for (model, b) in theirs {
                    if let Some((_, v)) = centroids.iter().find(|(m, _)| *m == model) {
                        let s = people::cosine(v, &people::vector(&b)?);
                        score = Some(score.map_or(s, |x: f32| x.max(s)));
                    }
                }
            }
        }
        out.push(IdentitySuggestion {
            uid: i.uid,
            name: i.name,
            short_name: i.short_name,
            score,
            person: linked,
        });
    }
    out.sort_by(|a, b| {
        b.score
            .unwrap_or(f32::MIN)
            .total_cmp(&a.score.unwrap_or(f32::MIN))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

#[tauri::command]
#[specta::specta]
pub async fn suggest_identities(
    state: State<'_, AppState>,
    person: i64,
    query: String,
) -> AppResult<Vec<IdentitySuggestion>> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || suggestions(&db, &app, person, &query)).await
}

#[tauri::command]
#[specta::specta]
pub async fn export_identities(
    state: State<'_, AppState>,
    destination: String,
    include_faces: bool,
) -> AppResult<()> {
    let app = state.app_store.clone();
    blocking(move || {
        let file = app.with(|c| transfer::export(c, include_faces))?;
        let dest = Path::new(&destination);
        let parent = dest
            .parent()
            .ok_or_else(|| AppError::invalid_input("destinazione non valida"))?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(tmp.as_file_mut(), &file)?;
        tmp.as_file().sync_all()?;
        tmp.persist(dest).map_err(|e| AppError::from(e.error))?;
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn preview_identity_import(
    state: State<'_, AppState>,
    path: String,
) -> AppResult<ImportPreview> {
    let app = state.app_store.clone();
    blocking(move || {
        let file = transfer::parse(&std::fs::read(&path)?)?;
        Ok(app.with(|c| transfer::preview(c, &file))?)
    })
    .await
}

/// Importa le identità; quelle con nome già presente vengono unite solo se
/// elencate in `merge`, altrimenti saltate.
#[tauri::command]
#[specta::specta]
pub async fn import_identities(
    state: State<'_, AppState>,
    path: String,
    merge: Vec<String>,
) -> AppResult<ImportOutcome> {
    let (db, app) = (state.store.clone(), state.app_store.clone());
    blocking(move || {
        let file = transfer::parse(&std::fs::read(&path)?)?;
        let out = app.tx(|c| transfer::apply(c, &file, &merge))?;
        sync_names(&db, &app)?;
        Ok(out)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_store::model::NewMedia;

    fn face(axis: usize) -> people::DetectedFace {
        let mut embedding = vec![0.0; 128];
        embedding[axis] = 1.0;
        people::DetectedFace {
            bbox: [0.1, 0.1, 0.2, 0.2],
            quality: 0.99,
            embedding,
        }
    }

    #[test]
    fn view_and_suggestions_rank_by_similarity() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let app = AppStore::open(&tmp.path().join("app.db")).unwrap();
        let (giulia, giuseppe) = app
            .with(|c| {
                let g = ids::create(
                    c,
                    &IdentityInput {
                        name: "Giulia Bianchi".into(),
                        ..Default::default()
                    },
                )?;
                let p = ids::create(
                    c,
                    &IdentityInput {
                        name: "Giuseppe Verdi".into(),
                        aliases: vec!["Peppe".into()],
                        ..Default::default()
                    },
                )?;
                ids::create(
                    c,
                    &IdentityInput {
                        name: "Marco".into(),
                        ..Default::default()
                    },
                )?;
                ids::add_face(
                    c,
                    &g,
                    b"png",
                    "sface",
                    Some(&face(3).embedding),
                    "file:g.jpg",
                )?;
                ids::add_face(
                    c,
                    &p,
                    b"png",
                    "sface",
                    Some(&face(9).embedding),
                    "file:p.jpg",
                )?;
                Ok((g, p))
            })
            .unwrap();
        let person = db
            .write(|c| {
                let id = pc_store::repo::media::upsert(
                    c,
                    &NewMedia {
                        path: "a.jpg".into(),
                        name: "a.jpg".into(),
                        ext: "jpg".into(),
                        ..Default::default()
                    },
                )?;
                people::save_faces(c, id, &[face(3)], "sface", "det+sface", 0.363)?;
                let p = people::create_person(c, None)?;
                people::assign(c, &[people::records(c, Some(id))?[0].id], p)?;
                Ok(p)
            })
            .unwrap();
        let s = suggestions(&db, &app, person, "Giu").unwrap();
        assert_eq!(
            s.iter().map(|x| x.uid.clone()).collect::<Vec<_>>(),
            vec![giulia.clone(), giuseppe]
        );
        assert!(s[0].score.unwrap() > 0.99);
        assert_eq!(suggestions(&db, &app, person, "peppe").unwrap().len(), 1);

        let v = view(&db, &app, None).unwrap();
        assert_eq!(v.identities.len(), 3);
        assert!(v.identities.iter().all(|r| r.person.is_none()));
        db.write(move |c| {
            people::link_identity(c, person, Some((&giulia, "Giulia Bianchi"))).map(|_| ())
        })
        .unwrap();
        let v = view(&db, &app, Some("giulia")).unwrap();
        assert_eq!(v.identities[0].person, Some(person));
        assert_eq!(v.identities[0].photos, 1);
        assert!(v.store_path.ends_with("app.db"));
    }

    #[test]
    fn startup_migration_links_named_people_once() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let app = AppStore::open(&tmp.path().join("app.db")).unwrap();
        db.write(|c| {
            c.execute(
                "DELETE FROM settings_kv WHERE key = ?1",
                [people::IDENTITIES_MIGRATED],
            )?;
            people::create_person(c, Some("Giulia"))?;
            Ok(())
        })
        .unwrap();
        on_library_open(&db, &app);
        on_library_open(&db, &app);
        assert_eq!(app.with(ids::list).unwrap().len(), 1);
    }
}

/// `cargo test -p photo-cat --features model-tests`: modelli reali di `assets/models`.
#[cfg(all(test, feature = "model-tests"))]
mod model_tests {
    use super::*;
    use pc_models::{pool::ModelPool, runtime, store::ModelStore, Manifest};
    use std::sync::Arc;

    #[test]
    fn image_with_three_faces_asks_which_one() {
        let assets = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        runtime::init(&runtime::dev_runtime_path()).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let mut manifest = Manifest::builtin();
        manifest
            .entries
            .retain(|e| e.id == "yunet" || e.id == "sface");
        let models = Arc::new(ModelStore::new(
            manifest,
            tmp.path().join("m"),
            Some(assets.join("models")),
        ));
        models.restore(
            &models.check_all(),
            None,
            &std::sync::atomic::AtomicBool::new(false),
        );
        let gov = pc_governor::MemoryGovernor::new(512);
        let pool = Arc::new(ModelPool::new(
            models.clone(),
            gov.clone(),
            Arc::new(pc_governor::FakeProbe::new(8192, 6000, 8)),
            runtime::SessionOptions {
                intra_threads: 4,
                ..Default::default()
            },
        ));
        let res = pc_ai::stages::AiResources {
            models,
            pool,
            governor: gov,
        };
        let app = AppStore::open(&tmp.path().join("app.db")).unwrap();
        let uid = app
            .with(|c| {
                ids::create(
                    c,
                    &IdentityInput {
                        name: "Giulia Bianchi".into(),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        let file = assets.join("selftest/group_3_faces.jpg");
        let out = add_reference_from_path(&res, &app, &uid, &file, None, 40).unwrap();
        let AddReferenceOutcome::Choose { faces } = out else {
            panic!("attesa la scelta del volto: {out:?}");
        };
        assert_eq!(faces.len(), 3);
        assert!(faces
            .iter()
            .all(|f| f.crop.starts_with("data:image/png;base64,")));
        assert_eq!(app.with(|c| ids::get(c, &uid)).unwrap().faces, 0);
        let out = add_reference_from_path(&res, &app, &uid, &file, Some(1), 40).unwrap();
        assert!(matches!(out, AddReferenceOutcome::Added { .. }));
        assert_eq!(app.with(|c| ids::get(c, &uid)).unwrap().faces, 1);
        let land = assets.join("selftest/landscape_mountain.jpg");
        assert!(matches!(
            add_reference_from_path(&res, &app, &uid, &land, None, 40).unwrap(),
            AddReferenceOutcome::NoFace
        ));
    }
}
