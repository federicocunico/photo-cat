//! Copertura delle analisi per fase: quante foto hanno la fase aggiornata, da
//! rifare, in coda o mai analizzata. Una sola fonte per conteggi e filtri.
//!
//! Marcatori per foto (vedi `analysis`, `similar`, `schema_v11`):
//! - classificazione: `analysis_version(stage='clip')`, modello + tassonomia;
//! - persone: `analysis_version(stage='persons')`, modello (con `|floor=`);
//! - volti: `analysis_version(stage='faces')`, modello (con `|floor=`);
//! - simili: `similar_done.params`;
//! - impronta: `media_item.phash IS NOT NULL` (sempre aggiornata se presente).
use crate::query::HIDDEN_BY_SYSTEM;
use crate::StoreResult;
use rusqlite::types::Value;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Stage {
    Classification,
    Persons,
    Faces,
    Similar,
    Phash,
}

impl Stage {
    pub const ALL: [Stage; 5] = [
        Stage::Classification,
        Stage::Persons,
        Stage::Faces,
        Stage::Similar,
        Stage::Phash,
    ];

    /// Nome dello stadio in `analysis_version` (assente per simili e impronta).
    fn analysis_stage(self) -> &'static str {
        match self {
            Stage::Classification => "clip",
            Stage::Persons => "persons",
            Stage::Faces => "faces",
            Stage::Similar | Stage::Phash => "",
        }
    }
}

/// Chiavi "correnti" di ogni fase; `None` = fase non disponibile (nessuna foto
/// è aggiornata, quelle già elaborate risultano da rifare).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StageKeys {
    /// (modello, versione tassonomia)
    pub classification: Option<(String, i64)>,
    pub persons: Option<String>,
    pub faces: Option<String>,
    /// Chiave dei parametri del calcolo simili.
    pub similar: Option<String>,
}

/// Predicato SQL su `m` (media_item) con i suoi parametri posizionali `?N`
/// numerati a partire da `base + 1`.
#[derive(Debug, Clone)]
pub struct Pred {
    pub sql: String,
    pub params: Vec<Value>,
}

impl Pred {
    fn plain(sql: &str) -> Self {
        Pred {
            sql: sql.into(),
            params: vec![],
        }
    }
}

/// Stesso criterio di `analysis::same_model`: la versione può avere il suffisso
/// `|floor=<soglia>` dopo il modello. (Replicato: lì è privato.)
fn same_model(col: &str, param: &str) -> String {
    format!("({col}={param} OR substr({col},1,length({param})+7)={param}||'|floor=')")
}

/// La foto è stata elaborata dalla fase con qualunque versione.
pub fn processed(stage: Stage) -> Pred {
    match stage {
        Stage::Similar => Pred::plain("EXISTS(SELECT 1 FROM similar_done d WHERE d.media_id=m.id)"),
        Stage::Phash => Pred::plain("m.phash IS NOT NULL"),
        s => Pred::plain(&format!(
            "EXISTS(SELECT 1 FROM analysis_version a WHERE a.media_id=m.id AND a.stage='{}')",
            s.analysis_stage()
        )),
    }
}

/// Condizione "versione corrente" sulla riga marcatore (alias `al`: `a*` o `d`).
fn row_cond(stage: Stage, keys: &StageKeys, base: usize, al: &str) -> Pred {
    let n = base + 1;
    let av = |extra: &str| {
        format!(
            "{}{extra}",
            same_model(&format!("{al}.model_ver"), &format!("?{n}"))
        )
    };
    match stage {
        Stage::Phash => Pred::plain("m.phash IS NOT NULL"),
        Stage::Classification => match &keys.classification {
            Some((model, tax)) => Pred {
                sql: av(&format!(" AND {al}.taxonomy_ver IS ?{}", n + 1)),
                params: vec![Value::Text(model.clone()), Value::Integer(*tax)],
            },
            None => Pred::plain("0"),
        },
        Stage::Persons | Stage::Faces => {
            let key = if stage == Stage::Persons {
                &keys.persons
            } else {
                &keys.faces
            };
            match key {
                // persone e volti non hanno tassonomia
                Some(k) => Pred {
                    sql: av(&format!(" AND {al}.taxonomy_ver IS NULL")),
                    params: vec![Value::Text(k.clone())],
                },
                None => Pred::plain("0"),
            }
        }
        Stage::Similar => match &keys.similar {
            Some(k) => Pred {
                sql: format!("{al}.params=?{n}"),
                params: vec![Value::Text(k.clone())],
            },
            None => Pred::plain("0"),
        },
    }
}

/// Elaborata con una versione diversa da quella corrente.
pub fn stale(stage: Stage, keys: &StageKeys, base: usize) -> Pred {
    let (p, c) = (processed(stage), current(stage, keys, base));
    Pred {
        sql: format!("({} AND NOT ({}))", p.sql, c.sql),
        params: c.params,
    }
}

/// Mai elaborata.
pub fn never(stage: Stage) -> Pred {
    Pred {
        sql: format!("NOT ({})", processed(stage).sql),
        params: vec![],
    }
}

/// Bersaglio di un job attivo per una fase.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Target {
    #[default]
    None,
    /// Tutta la libreria.
    Library,
    Ids(Vec<i64>),
}

/// Job attivi per fase, calcolati dal chiamante.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveTargets {
    pub classification: Target,
    pub persons: Target,
    pub faces: Target,
    pub similar: Target,
    pub phash: Target,
}

impl ActiveTargets {
    pub fn of(&self, stage: Stage) -> &Target {
        match stage {
            Stage::Classification => &self.classification,
            Stage::Persons => &self.persons,
            Stage::Faces => &self.faces,
            Stage::Similar => &self.similar,
            Stage::Phash => &self.phash,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StageCoverage {
    pub stage: Stage,
    pub current: i64,
    pub stale: i64,
    pub queued: i64,
    pub never: i64,
    pub total: i64,
}

/// Copertura di ogni fase sulle foto analizzabili (presenti, con miniatura, non
/// nei cataloghi di sistema). `current + stale + queued + never == total`; le
/// foto in coda sono quelle non aggiornate bersaglio di un job attivo.
pub fn coverage(
    c: &Connection,
    keys: &StageKeys,
    active: &ActiveTargets,
) -> StoreResult<Vec<StageCoverage>> {
    // Una sola scansione per tutte le fasi: un LEFT JOIN per ogni riga marcatore
    // (alias `j0..`), da cui si ricavano "elaborata" e "corrente" di ogni fase.
    let mut params: Vec<Value> = Vec::new();
    let mut joins = String::new();
    let mut cols = String::new();
    let mut sums = String::new();
    for (i, stage) in Stage::ALL.into_iter().enumerate() {
        let (cur, proc) = match source(stage) {
            Some((src, on)) => {
                let table = src.split(' ').next().unwrap_or_default();
                let al = format!("j{i}");
                joins.push_str(&format!(
                    " LEFT JOIN {table} {al} ON {al}.media_id=m.id{}",
                    on.replace("a.stage", &format!("{al}.stage"))
                ));
                let rc = row_cond(stage, keys, params.len(), &al);
                params.extend(rc.params);
                (
                    format!("coalesce({}, 0)", rc.sql),
                    format!("{al}.media_id IS NOT NULL"),
                )
            }
            None => (
                "m.phash IS NOT NULL".to_string(),
                "m.phash IS NOT NULL".to_string(),
            ),
        };
        // Gli id del bersaglio passano da json_each: niente tabella temporanea,
        // quindi funziona anche su connessioni in sola lettura.
        let tg = match active.of(stage) {
            Target::None => "0".to_string(),
            Target::Library => "1".to_string(),
            Target::Ids(v) => {
                params.push(Value::Text(serde_json::to_string(v)?));
                format!("m.id IN (SELECT value FROM json_each(?{}))", params.len())
            }
        };
        // Un solo codice per fase: 0 corrente, 1 in coda, 2 da aggiornare, 3 mai.
        cols.push_str(&format!(
            ", CASE WHEN {cur} THEN 0 WHEN {tg} THEN 1 WHEN {proc} THEN 2 ELSE 3 END AS k{i}"
        ));
        sums.push_str(&format!(
            ", coalesce(sum(k{i}=0),0), coalesce(sum(k{i}=1),0), coalesce(sum(k{i}=2),0), coalesce(sum(k{i}=3),0)"
        ));
    }
    let sql = format!(
        "SELECT count(*){sums}
         FROM (SELECT m.id{cols} FROM media_item m{joins}
           WHERE m.status='ok' AND m.thumb IS NOT NULL AND {HIDDEN_BY_SYSTEM} LIMIT -1)"
    );
    Ok(c.query_row(&sql, rusqlite::params_from_iter(params), |r| {
        let total: i64 = r.get(0)?;
        let mut out = Vec::with_capacity(Stage::ALL.len());
        for (i, stage) in Stage::ALL.into_iter().enumerate() {
            let b = 1 + i * 4;
            out.push(StageCoverage {
                stage,
                current: r.get(b)?,
                queued: r.get(b + 1)?,
                stale: r.get(b + 2)?,
                never: r.get(b + 3)?,
                total,
            });
        }
        Ok(out)
    })?)
}

/// Sorgente della riga marcatore di una fase: (tabella con alias, condizione
/// di join oltre a `media_id`).
fn source(stage: Stage) -> Option<(&'static str, String)> {
    match stage {
        Stage::Similar => Some(("similar_done d", String::new())),
        Stage::Phash => None,
        s => Some((
            "analysis_version a",
            format!(" AND a.stage='{}'", s.analysis_stage()),
        )),
    }
}

/// La foto è elaborata con la versione corrente (`base` = parametri già usati).
pub fn current(stage: Stage, keys: &StageKeys, base: usize) -> Pred {
    match (stage, source(stage)) {
        (Stage::Phash, _) | (_, None) => Pred::plain("m.phash IS NOT NULL"),
        (_, Some((src, join))) => {
            let alias = &src[src.len() - 1..];
            let c = row_cond(stage, keys, base, alias);
            Pred {
                sql: format!(
                    "EXISTS(SELECT 1 FROM {src} WHERE {alias}.media_id=m.id{join} AND {})",
                    c.sql
                ),
                params: c.params,
            }
        }
    }
}
