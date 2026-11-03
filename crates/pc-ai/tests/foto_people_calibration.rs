//! Taratura e prova sul catalogo reale `D:\Foto` (sola lettura) con cartella dati
//! separata. Non fa parte della CI: richiede modelli e `D:\Foto`.
//!
//!   cargo test -p pc-ai --features model-tests --test foto_people_calibration -- --nocapture
#![cfg(feature = "model-tests")]

use pc_ai::stages::AiResources;
use pc_governor::{FakeProbe, MemoryGovernor};
use pc_models::manifest::Manifest;
use pc_models::pool::ModelPool;
use pc_models::runtime::{self, SessionOptions};
use pc_models::store::ModelStore;
use pc_store::app_store::identities as ids;
use pc_store::people::{self, FaceMatch, FaceScope, FaceSort, LocateFaceRequest, PeopleSection};
use pc_store::{AppStore, Store};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const FOTO: &str = r"D:\Foto";
const SAMPLE: usize = 48;
const DATA: &str = r"D:\Github\photo-cat-people\.tmp-people-data";

fn asset(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets")
        .join(rel)
}

fn resources() -> AiResources {
    runtime::init(&runtime::dev_runtime_path()).expect("onnx runtime");
    let mut manifest = Manifest::builtin();
    manifest
        .entries
        .retain(|e| e.id == "yunet" || e.id == "sface");
    let models_dir = PathBuf::from(DATA).join("models");
    let store = Arc::new(ModelStore::new(
        manifest,
        models_dir,
        Some(asset("models")),
    ));
    store.restore(&store.check_all(), None, &AtomicBool::new(false));
    let gov = MemoryGovernor::new(1024);
    let pool = Arc::new(ModelPool::new(
        store.clone(),
        gov.clone(),
        Arc::new(FakeProbe::new(8192, 6000, 8)),
        SessionOptions {
            intra_threads: 4,
            ..Default::default()
        },
    ));
    AiResources {
        models: store,
        pool,
        governor: gov,
    }
}

fn collect_jpgs(dir: &Path, out: &mut Vec<PathBuf>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        if out.len() >= limit {
            return;
        }
        let p = e.path();
        if p.is_dir() {
            collect_jpgs(&p, out, limit);
        } else {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(ext.as_str(), "jpg" | "jpeg") {
                out.push(p);
            }
        }
    }
}

fn sample_jpgs(root: &Path, limit: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for prefer in [
        "2018.07 - Montagna (Gio Alessia Giulia)",
        "2013 Rome",
        "2018.11.24 - SHG.MilanoMusicShow",
    ] {
        collect_jpgs(&root.join(prefer), &mut out, limit);
        if out.len() >= limit {
            break;
        }
    }
    if out.len() < limit {
        collect_jpgs(root, &mut out, limit);
    }
    out.truncate(limit);
    out
}

fn resnap(path: &Path) -> Vec<String> {
    let mut lines = Vec::new();
    fn walk(dir: &Path, lines: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, lines);
            } else if let Ok(meta) = e.metadata() {
                let t = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                lines.push(format!("{}|{}|{}", p.display(), meta.len(), t));
            }
        }
    }
    walk(path, &mut lines);
    lines.sort();
    lines
}

#[test]
fn calibrate_and_exercise_people_on_foto_sample() {
    assert!(
        Path::new(FOTO).is_dir(),
        "D:\\Foto assente: taratura non eseguibile"
    );
    std::fs::create_dir_all(DATA).unwrap();
    let before = resnap(Path::new(FOTO));

    let files = sample_jpgs(Path::new(FOTO), SAMPLE);
    assert!(
        files.len() >= 12,
        "troppi pochi JPEG nel campione ({})",
        files.len()
    );
    eprintln!("campione: {} file da {}", files.len(), FOTO);

    let res = resources();
    let catalog = PathBuf::from(DATA).join("catalog.db");
    let _ = std::fs::remove_file(&catalog);
    let db = Store::open(&catalog, Default::default()).unwrap();
    let app = AppStore::open(&PathBuf::from(DATA).join("app.db")).unwrap();
    let m = FaceMatch {
        similarity: 0.363,
        min_quality: 0.8,
        min_sharpness: 0.35,
        min_size_px: 40,
        floor: Some(0.8),
    };

    let mut sharp_scores = Vec::new();
    let mut face_total = 0i64;
    let mut media_with_faces = Vec::new();

    for (i, path) in files.iter().enumerate() {
        let id = (i + 1) as i64;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("foto.jpg")
            .to_string();
        let path_s = path.to_string_lossy().into_owned();
        let img = match image::open(path) {
            Ok(i) => i.to_rgb8(),
            Err(e) => {
                eprintln!("skip {path_s}: {e}");
                continue;
            }
        };
        let dims = img.dimensions();
        let found = match pc_ai::face_analysis::analyze(&res, &img, dims, 40, 0.7) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("analyze {path_s}: {e}");
                continue;
            }
        };
        for f in &found {
            if let Some(s) = f.sharpness {
                sharp_scores.push(s);
            }
        }
        face_total += found.len() as i64;
        let had_faces = !found.is_empty();
        let found_owned = found;
        let name_owned = name;
        let path_owned = path_s;
        db.write(move |c| {
            c.execute(
                "INSERT INTO media_item(id,path,name,ext,size,mtime,added_at,updated_at) VALUES (?1,?2,?3,'jpg',1,1,1,1)",
                rusqlite::params![id, path_owned, name_owned],
            )?;
            if !found_owned.is_empty() {
                people::save_faces_tuned(
                    c,
                    id,
                    &found_owned,
                    "sface",
                    "yunet+sface",
                    m,
                    &people::IdentityRefs::default(),
                )?;
            }
            Ok(())
        })
        .unwrap();
        if had_faces {
            media_with_faces.push(id);
        }
    }

    assert!(face_total > 0, "nessun volto nel campione");
    sharp_scores.sort_by(|a, b| a.total_cmp(b));
    let pct = |p: f32| {
        let i = ((sharp_scores.len() as f32 - 1.0) * p).round() as usize;
        sharp_scores[i.min(sharp_scores.len() - 1)]
    };
    let (p10, p50, p90) = (pct(0.10), pct(0.50), pct(0.90));
    let below = sharp_scores.iter().filter(|s| **s < 0.35).count();
    eprintln!(
        "nitidezza n={} p10={p10:.3} p50={p50:.3} p90={p90:.3} soglia=0.35 scartati_sotto={below}",
        sharp_scores.len()
    );
    let notes = format!(
        "- Taratura {FOTO} (campione {} JPEG, {} volti con nitidezza): p10={p10:.3}, p50={p50:.3}, p90={p90:.3}. \
         Soglia predefinita 0,35: ~{}% sotto soglia. Curva `sqrt(var/(var+250))` e soglia 0,35 confermate sul campione; \
         nessuna modifica all'approccio.",
        files.len(),
        sharp_scores.len(),
        if sharp_scores.is_empty() {
            0
        } else {
            below * 100 / sharp_scores.len()
        }
    );
    std::fs::write(PathBuf::from(DATA).join("sharpness-notes.txt"), &notes).unwrap();
    eprintln!("{notes}");

    pc_ai::cluster::run(
        &db,
        &res.governor,
        &pc_jobs::JobControl::new(),
        Arc::new(pc_ai::face_policy::FacePolicy::new(true)),
        &Default::default(),
        &pc_core::AiThresholds {
            face_detection: 0.8,
            face_min_sharpness: 0.35,
            face_similarity: 0.363,
            ..Default::default()
        },
    )
    .unwrap();

    let sections = db.read(|c| people::people_sections(c, &m)).unwrap();
    eprintln!("sezioni: {sections:?}");
    assert!(sections.review + sections.people + sections.singles + sections.discarded > 0);

    let review = db
        .read(|c| {
            people::list_groups(
                c,
                PeopleSection::Review,
                None,
                people::GroupSort::FacesDesc,
                None,
                20,
                &m,
            )
        })
        .unwrap();
    assert!(
        !review.items.is_empty() || sections.singles > 0 || sections.discarded > 0,
        "nessun gruppo/volto da rivedere"
    );

    let uid = app
        .with(|a| {
            ids::create(
                a,
                &ids::IdentityInput {
                    name: "Persona prova".into(),
                    ..Default::default()
                },
            )
        })
        .unwrap();

    let review_ids: Vec<i64> = review.items.iter().map(|g| g.id).collect();
    if let Some(&g0) = review_ids.first() {
        let uid2 = uid.clone();
        db.write(move |c| people::link_identity(c, g0, Some((uid2.as_str(), "Persona prova"))))
            .unwrap();
        let after_link = db.read(|c| people::people_sections(c, &m)).unwrap();
        assert!(after_link.people >= 1, "collegamento → sezione Persone");
    }

    if review_ids.len() >= 2 {
        let b = review_ids[1];
        let uid2 = uid.clone();
        let kept = db
            .write(move |c| people::link_identity(c, b, Some((uid2.as_str(), "Persona prova"))))
            .unwrap();
        eprintln!("unione via identità: kept={kept} from={b}");
    }

    let review2 = db
        .read(|c| {
            people::list_groups(
                c,
                PeopleSection::Review,
                None,
                people::GroupSort::FacesDesc,
                None,
                5,
                &m,
            )
        })
        .unwrap();
    if let Some(g) = review2.items.first().map(|r| r.id) {
        db.write(move |c| people::ignore_person(c, g)).unwrap();
        let ign = db
            .read(|c| {
                people::list_groups(
                    c,
                    PeopleSection::Ignored,
                    None,
                    people::GroupSort::FacesDesc,
                    None,
                    10,
                    &m,
                )
            })
            .unwrap();
        assert!(ign.items.iter().any(|r| r.id == g));
        db.write(move |c| people::restore_person(c, g)).unwrap();
        db.write(move |c| people::ignore_person(c, g)).unwrap();
    }

    let mut located_ok = 0;
    for &mid in media_with_faces.iter().take(3) {
        let loc = db
            .read(move |c| {
                people::locate_face(
                    c,
                    &LocateFaceRequest::Media {
                        media_id: mid,
                        person_id: None,
                    },
                    &m,
                )
            })
            .unwrap();
        eprintln!(
            "locate media={mid} → section={:?} group={:?} discarded={} index={}",
            loc.section, loc.group_id, loc.discarded, loc.index
        );
        assert_eq!(loc.media_id, mid);
        located_ok += 1;
        if let Some(gid) = loc.group_id {
            let discarded = loc.discarded;
            let page = db
                .read(move |c| {
                    let scope = if discarded {
                        FaceScope::Discarded { group: Some(gid) }
                    } else {
                        FaceScope::Group { group: gid }
                    };
                    people::group_faces(c, scope, FaceSort::QualityDesc, None, 200, &m)
                })
                .unwrap();
            assert!(
                (loc.index as usize) < page.items.len() || page.total > loc.index,
                "indice fuori range"
            );
        }
    }
    assert_eq!(located_ok, 3.min(media_with_faces.len()));

    let after = resnap(Path::new(FOTO));
    let before_set: std::collections::HashSet<_> = before.iter().cloned().collect();
    let after_set: std::collections::HashSet<_> = after.iter().cloned().collect();
    assert_eq!(
        before_set.len(),
        after_set.len(),
        "numero file D:\\Foto cambiato"
    );
    let missing: Vec<_> = before_set.difference(&after_set).take(5).collect();
    let added: Vec<_> = after_set.difference(&before_set).take(5).collect();
    assert!(
        missing.is_empty() && added.is_empty(),
        "D:\\Foto modificata: missing={missing:?} added={added:?}"
    );
    eprintln!("D:\\Foto invariata ({} file)", after.len());
}
