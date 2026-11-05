//! Aggregati per i grafici dell'area AI (istogramma, copertura mensile, gruppi di simili).
use crate::query::{compile_filter, LibraryFilter};
use crate::StoreResult;
use rusqlite::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};

/// Stessa base di `query::FROM` (alias `m`, `d`, `t`).
const FROM: &str = "FROM media_item m
    LEFT JOIN media_dates d ON d.media_id = m.id
    LEFT JOIN media_tech t ON t.media_id = m.id";

/// Classe dell'istogramma: `[from, to)` (l'ultima include 1,0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Bin {
    pub from: f64,
    pub to: f64,
    pub count: i64,
}

/// Istogramma delle confidenze delle etichette automatiche `(group, value)` delle foto
/// che soddisfano il filtro: `bins` classi uguali su [0, 1] (default 20 se `bins == 0`).
pub fn label_histogram(
    c: &Connection,
    filter: &LibraryFilter,
    group: &str,
    value: &str,
    bins: usize,
) -> StoreResult<Vec<Bin>> {
    let n = if bins == 0 { 20 } else { bins };
    let w = compile_filter(c, filter)?;
    let sql = format!(
        "SELECT MAX(0, MIN(?1 - 1, CAST(l.score * ?1 AS INTEGER))) AS b, count(*)
         FROM label l JOIN media_item m ON m.id = l.media_id
         LEFT JOIN media_dates d ON d.media_id = m.id
         LEFT JOIN media_tech t ON t.media_id = m.id
         WHERE l.grp = ?2 AND l.value = ?3 AND l.origin = 'auto' AND {}
         GROUP BY b",
        w.sql
    );
    let mut args: Vec<Value> = vec![
        Value::Integer(n as i64),
        Value::Text(group.into()),
        Value::Text(value.into()),
    ];
    args.extend(w.args.iter().cloned());
    let mut out: Vec<Bin> = (0..n)
        .map(|i| Bin {
            from: i as f64 / n as f64,
            to: (i + 1) as f64 / n as f64,
            count: 0,
        })
        .collect();
    let mut st = c.prepare(&sql)?;
    let rows = st.query_map(params_from_iter(args.iter()), |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (b, k) = row?;
        if let Some(bin) = out.get_mut(b as usize) {
            bin.count = k;
        }
    }
    Ok(out)
}

/// Foto di un mese (`YYYY-MM`, `None` = senza data).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MonthCount {
    pub month: Option<String>,
    pub total: i64,
    /// Foto con la classificazione CLIP eseguita (`analysis_version.stage = 'clip'`, qualsiasi versione).
    pub analyzed: i64,
}

/// Foto totali e analizzate per mese di scatto, in ordine cronologico; "senza data" in coda.
pub fn month_coverage(c: &Connection, filter: &LibraryFilter) -> StoreResult<Vec<MonthCount>> {
    let w = compile_filter(c, filter)?;
    let sql = format!(
        "SELECT substr(d.captured_at, 1, 7) AS mo, count(*),
                sum(EXISTS(SELECT 1 FROM analysis_version a WHERE a.media_id = m.id AND a.stage = 'clip'))
         {FROM} WHERE {} GROUP BY mo ORDER BY mo IS NULL, mo",
        w.sql
    );
    let mut st = c.prepare(&sql)?;
    let rows = st
        .query_map(params_from_iter(w.args.iter()), |r| {
            Ok(MonthCount {
                month: r.get(0)?,
                total: r.get(1)?,
                analyzed: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Classe di dimensione dei gruppi di simili.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum SizeBucket {
    /// 2 foto.
    Two,
    /// 3-5 foto.
    ThreeFive,
    /// 6-10 foto.
    SixTen,
    /// Più di 10 foto.
    Over10,
}

impl SizeBucket {
    pub const ALL: [SizeBucket; 4] = [Self::Two, Self::ThreeFive, Self::SixTen, Self::Over10];
    pub fn label(self) -> &'static str {
        match self {
            Self::Two => "2",
            Self::ThreeFive => "3-5",
            Self::SixTen => "6-10",
            Self::Over10 => ">10",
        }
    }
    /// Intervallo `[min, max]` di dimensioni.
    pub fn range(self) -> (i64, i64) {
        match self {
            Self::Two => (2, 2),
            Self::ThreeFive => (3, 5),
            Self::SixTen => (6, 10),
            Self::Over10 => (11, i64::MAX),
        }
    }
    pub fn of(size: i64) -> Option<Self> {
        Self::ALL.into_iter().find(|b| {
            let (lo, hi) = b.range();
            size >= lo && size <= hi
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SizeStat {
    pub bucket: SizeBucket,
    pub label: String,
    pub groups: i64,
    pub photos: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SimilarStats {
    pub groups: i64,
    pub grouped_photos: i64,
    pub size_buckets: Vec<SizeStat>,
    /// Foto ancora da elaborare: la compila il chiamante (`pc_ai::similar::pending_count`).
    pub pending: i64,
}

/// Gruppi di simili e dimensioni; con `filter` si contano i membri che lo soddisfano
/// (un gruppo conta se ha almeno 2 membri nel filtro).
pub fn similar_stats(c: &Connection, filter: Option<&LibraryFilter>) -> StoreResult<SimilarStats> {
    let sizes: Vec<i64> = match filter {
        None => {
            let mut st = c.prepare("SELECT count(*) FROM similar_member GROUP BY group_id")?;
            let v = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
            v
        }
        Some(f) => {
            let w = compile_filter(c, f)?;
            let mut st = c.prepare(&format!(
                "SELECT count(*) FROM similar_member s JOIN media_item m ON m.id = s.media_id
                 LEFT JOIN media_dates d ON d.media_id = m.id
                 LEFT JOIN media_tech t ON t.media_id = m.id
                 WHERE {} GROUP BY s.group_id",
                w.sql
            ))?;
            let v = st
                .query_map(params_from_iter(w.args.iter()), |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            v
        }
    };
    let mut buckets: Vec<SizeStat> = SizeBucket::ALL
        .iter()
        .map(|b| SizeStat {
            bucket: *b,
            label: b.label().into(),
            groups: 0,
            photos: 0,
        })
        .collect();
    let (mut groups, mut photos) = (0, 0);
    for s in sizes {
        let Some(b) = SizeBucket::of(s) else { continue };
        let e = buckets.iter_mut().find(|e| e.bucket == b).unwrap();
        e.groups += 1;
        e.photos += s;
        groups += 1;
        photos += s;
    }
    Ok(SimilarStats {
        groups,
        grouped_photos: photos,
        size_buckets: buckets,
        pending: 0,
    })
}
