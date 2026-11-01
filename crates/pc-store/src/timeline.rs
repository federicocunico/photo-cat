//! Istante reale di scatto, gruppi di sorgente e correzioni orarie.
//!
//! `media_dates.captured_at` resta l'ora letta dal file. L'istante UTC
//! corretto (`instant_utc`, ms) è materializzato e indicizzato, e si
//! ricalcola in blocco quando cambia una correzione di gruppo:
//!
//! `instant_utc = captured_at − fuso + shift_s`, dove il fuso è quello
//! dichiarato nel file, altrimenti quello dell'ora locale del gruppo alla data
//! dello scatto, altrimenti il fuso di sistema. Senza fuso dichiarato l'istante
//! è "presunto". L'ora mostrata è `instant_utc + local_offset_min`.

use crate::model::MediaId;
use crate::{StoreError, StoreResult};
use chrono::{Duration, LocalResult, NaiveDateTime, Offset, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const LOCAL_FMT: &str = "%Y-%m-%d %H:%M:%S";

/// Fuso di sistema all'ora locale indicata (iniettabile nei test).
pub type SystemOffset = dyn Fn(NaiveDateTime) -> i32;

pub fn parse_local(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s.trim(), LOCAL_FMT).ok()
}

fn minutes(o: impl Offset) -> i32 {
    o.fix().local_minus_utc() / 60
}

/// Fuso di sistema reale.
pub fn system_offset(local: NaiveDateTime) -> i32 {
    match chrono::Local.offset_from_local_datetime(&local) {
        LocalResult::Single(o) | LocalResult::Ambiguous(o, _) => minutes(o),
        LocalResult::None => chrono::Local
            .offset_from_local_datetime(&(local - Duration::hours(1)))
            .earliest()
            .map(minutes)
            .unwrap_or(0),
    }
}

/// Scostamento (minuti) del fuso IANA all'ora locale indicata. Ora ambigua
/// (ritorno all'ora solare): la prima occorrenza; ora inesistente (salto in
/// avanti): il fuso in vigore prima del salto.
pub fn tz_offset_at_local(tz: &str, local: NaiveDateTime) -> Option<i32> {
    let tz: chrono_tz::Tz = tz.parse().ok()?;
    match tz.offset_from_local_datetime(&local) {
        LocalResult::Single(o) | LocalResult::Ambiguous(o, _) => Some(minutes(o)),
        LocalResult::None => tz
            .offset_from_local_datetime(&(local - Duration::hours(1)))
            .earliest()
            .map(minutes),
    }
}

/// Scostamento (minuti) del fuso IANA nell'istante UTC indicato.
pub fn tz_offset_at_utc(tz: &str, utc_ms: i64) -> Option<i32> {
    let tz: chrono_tz::Tz = tz.parse().ok()?;
    let utc = chrono::DateTime::from_timestamp_millis(utc_ms)?.naive_utc();
    Some(minutes(tz.offset_from_utc_datetime(&utc)))
}

pub fn valid_tz(tz: &str) -> bool {
    tz.parse::<chrono_tz::Tz>().is_ok()
}

/// Correzioni di un gruppo di sorgente.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupClock {
    pub display_tz: Option<String>,
    pub display_offset_min: Option<i32>,
    pub shift_s: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instant {
    pub utc_ms: i64,
    pub presumed: bool,
    pub local_offset_min: i32,
}

/// Istante corretto di uno scatto.
pub fn compute(
    captured_at: &str,
    declared: Option<i32>,
    g: &GroupClock,
    system: &SystemOffset,
) -> Option<Instant> {
    let local = parse_local(captured_at)?;
    let (offset, presumed) = match declared {
        Some(o) => (o, false),
        None => {
            let group = g
                .display_tz
                .as_deref()
                .and_then(|tz| tz_offset_at_local(tz, local))
                .or(g.display_offset_min);
            (group.unwrap_or_else(|| system(local)), true)
        }
    };
    let utc_ms = local.and_utc().timestamp_millis() - offset as i64 * 60_000 + g.shift_s * 1000;
    let local_offset_min = g
        .display_tz
        .as_deref()
        .and_then(|tz| tz_offset_at_utc(tz, utc_ms))
        .or(g.display_offset_min)
        .unwrap_or(offset);
    Some(Instant {
        utc_ms,
        presumed,
        local_offset_min,
    })
}

/// Ora locale mostrata: `AAAA-MM-GG HH:MM:SS`.
pub fn local_text(utc_ms: i64, offset_min: i32) -> Option<String> {
    let t = chrono::DateTime::from_timestamp_millis(utc_ms + offset_min as i64 * 60_000)?;
    Some(t.naive_utc().format(LOCAL_FMT).to_string())
}

/// `+02:00`, `-06:00`.
pub fn offset_text(min: i32) -> String {
    let sign = if min < 0 { '-' } else { '+' };
    let m = min.abs();
    format!("{sign}{:02}:{:02}", m / 60, m % 60)
}

// ---------------------------------------------------------------- gruppi

/// Gruppo della sorgente (dispositivo + fuso dichiarato), creato se manca. Le
/// foto senza dispositivo formano un unico gruppo "Altre sorgenti".
pub fn ensure_group(
    c: &Connection,
    device: Option<&str>,
    declared: Option<i32>,
) -> StoreResult<i64> {
    let device = device.map(str::trim).filter(|d| !d.is_empty());
    let declared = if device.is_some() { declared } else { None };
    c.execute(
        "INSERT OR IGNORE INTO source_group(device, declared_offset_min) VALUES (?1, ?2)",
        params![device, declared],
    )?;
    Ok(c.query_row(
        "SELECT id FROM source_group
         WHERE COALESCE(device, '') = COALESCE(?1, '')
           AND COALESCE(declared_offset_min, -100000) = COALESCE(?2, -100000)",
        params![device, declared],
        |r| r.get(0),
    )?)
}

pub fn group_clock(c: &Connection, id: i64) -> StoreResult<GroupClock> {
    c.query_row(
        "SELECT display_tz, display_offset_min, shift_s FROM source_group WHERE id = ?1",
        [id],
        |r| {
            Ok(GroupClock {
                display_tz: r.get(0)?,
                display_offset_min: r.get(1)?,
                shift_s: r.get(2)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound(format!("gruppo di sorgente {id}")))
}

/// Ricalcola gruppo e istante di un file dopo che le sue date sono cambiate.
pub fn refresh_media(c: &Connection, id: MediaId, system: &SystemOffset) -> StoreResult<()> {
    let row: Option<(Option<String>, Option<i32>, Option<String>)> = c
        .query_row(
            "SELECT captured_at, declared_offset_min, device FROM media_dates WHERE media_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((captured, declared, device)) = row else {
        return Ok(());
    };
    let group = ensure_group(c, device.as_deref(), declared)?;
    let clock = group_clock(c, group)?;
    write_instant(c, id, group, captured.as_deref(), declared, &clock, system)
}

fn write_instant(
    c: &Connection,
    id: MediaId,
    group: i64,
    captured: Option<&str>,
    declared: Option<i32>,
    clock: &GroupClock,
    system: &SystemOffset,
) -> StoreResult<()> {
    let placed: Option<(i64, Option<i32>)> = c
        .query_row(
            "SELECT instant_utc, local_offset_min FROM media_time_placement WHERE media_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (utc, presumed, local) = match placed {
        Some((utc, local)) => (Some(utc), true, local),
        None => match captured.and_then(|s| compute(s, declared, clock, system)) {
            Some(i) => (Some(i.utc_ms), i.presumed, Some(i.local_offset_min)),
            None => (None, true, None),
        },
    };
    c.execute(
        "UPDATE media_dates SET group_id = ?2, instant_utc = ?3, instant_presumed = ?4, local_offset_min = ?5
         WHERE media_id = ?1",
        params![id, group, utc, presumed, local],
    )?;
    Ok(())
}

/// Ricalcola in blocco tutti i membri di un gruppo (cambio di fuso). Restituisce
/// il numero di file aggiornati.
pub fn recompute_group(c: &Connection, group: i64, system: &SystemOffset) -> StoreResult<usize> {
    let clock = group_clock(c, group)?;
    let mut q = c.prepare(
        "SELECT media_id, captured_at, declared_offset_min FROM media_dates WHERE group_id = ?1",
    )?;
    let rows = q
        .query_map([group], |r| {
            Ok((
                r.get::<_, MediaId>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<i32>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, captured, declared) in &rows {
        write_instant(
            c,
            *id,
            group,
            captured.as_deref(),
            *declared,
            &clock,
            system,
        )?;
    }
    Ok(rows.len())
}

// ------------------------------------------------------- correzioni dei gruppi

/// Gruppo di sorgente per la UI ("Sorgenti e orari").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SourceGroupView {
    pub id: i64,
    /// `None` = "Altre sorgenti".
    pub device: Option<String>,
    pub declared_offset_min: Option<i32>,
    pub display_tz: Option<String>,
    pub display_offset_min: Option<i32>,
    pub shift_s: i64,
    /// Foto del gruppo (nell'ambito richiesto).
    pub count: i64,
}

/// Gruppi con almeno una foto, eventualmente limitati ai file indicati.
pub fn list_groups(c: &Connection, ids: Option<&[MediaId]>) -> StoreResult<Vec<SourceGroupView>> {
    let scope = ids.map(serde_json::to_string).transpose()?;
    let mut q = c.prepare(
        "SELECT g.id, g.device, g.declared_offset_min, g.display_tz, g.display_offset_min, g.shift_s, count(d.media_id)
         FROM source_group g JOIN media_dates d ON d.group_id = g.id
         WHERE ?1 IS NULL OR d.media_id IN (SELECT value FROM json_each(?1))
         GROUP BY g.id ORDER BY g.device IS NULL, count(d.media_id) DESC, g.id",
    )?;
    let rows = q
        .query_map([scope], |r| {
            Ok(SourceGroupView {
                id: r.get(0)?,
                device: r.get(1)?,
                declared_offset_min: r.get(2)?,
                display_tz: r.get(3)?,
                display_offset_min: r.get(4)?,
                shift_s: r.get(5)?,
                count: r.get(6)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Imposta le correzioni di un gruppo e ricalcola i membri, senza registrare
/// l'annullamento. Se cambia solo lo scostamento basta un `UPDATE` in blocco.
pub fn set_clock(
    c: &Connection,
    group: i64,
    clock: &GroupClock,
    system: &SystemOffset,
) -> StoreResult<usize> {
    if let Some(tz) = &clock.display_tz {
        if !valid_tz(tz) {
            return Err(StoreError::Invalid(format!(
                "fuso orario sconosciuto: {tz}"
            )));
        }
    }
    if let Some(o) = clock.display_offset_min {
        if !(-14 * 60..=14 * 60).contains(&o) {
            return Err(StoreError::Invalid(
                "scostamento del fuso fuori dai limiti".into(),
            ));
        }
    }
    let old = group_clock(c, group)?;
    c.execute(
        "UPDATE source_group SET display_tz = ?2, display_offset_min = ?3, shift_s = ?4 WHERE id = ?1",
        params![group, clock.display_tz, clock.display_offset_min, clock.shift_s],
    )?;
    let only_shift = old.display_tz == clock.display_tz
        && old.display_offset_min == clock.display_offset_min
        && clock.display_tz.is_none();
    if only_shift {
        let delta = (clock.shift_s - old.shift_s) * 1000;
        return Ok(c.execute(
            "UPDATE media_dates SET instant_utc = instant_utc + ?2
             WHERE group_id = ?1 AND instant_utc IS NOT NULL
               AND media_id NOT IN (SELECT media_id FROM media_time_placement)",
            params![group, delta],
        )?);
    }
    recompute_group(c, group, system)
}

/// Modifica annullabile delle correzioni di un gruppo.
pub fn change_clock(
    c: &Connection,
    group: i64,
    clock: &GroupClock,
    description: &str,
    system: &SystemOffset,
) -> StoreResult<usize> {
    let previous = group_clock(c, group)?;
    let n = set_clock(c, group, clock, system)?;
    crate::bulk::record(
        c,
        description,
        &crate::bulk::UndoAction::GroupClock { group, previous },
    )?;
    Ok(n)
}

/// Ora locale mostrata per un gruppo: fuso IANA oppure scostamento fisso
/// (`None` per entrambi = quella dichiarata nei file).
pub fn set_display(
    c: &Connection,
    group: i64,
    tz: Option<String>,
    offset_min: Option<i32>,
    system: &SystemOffset,
) -> StoreResult<usize> {
    let mut clock = group_clock(c, group)?;
    clock.display_tz = tz.filter(|t| !t.trim().is_empty());
    clock.display_offset_min = if clock.display_tz.is_some() {
        None
    } else {
        offset_min
    };
    let label = clock
        .display_tz
        .clone()
        .or(clock.display_offset_min.map(offset_text))
        .unwrap_or_else(|| "dichiarata".into());
    change_clock(
        c,
        group,
        &clock,
        &format!("Ora locale del gruppo: {label}"),
        system,
    )
}

/// Scostamento dell'orologio del gruppo, in secondi (valore assoluto, non incremento).
pub fn set_shift(
    c: &Connection,
    group: i64,
    shift_s: i64,
    system: &SystemOffset,
) -> StoreResult<usize> {
    let mut clock = group_clock(c, group)?;
    clock.shift_s = shift_s;
    change_clock(
        c,
        group,
        &clock,
        &format!("Scostamento del gruppo: {}", shift_text(shift_s)),
        system,
    )
}

/// `+2 min 10 s`, `-1 h 3 s`.
pub fn shift_text(s: i64) -> String {
    let sign = if s < 0 { '-' } else { '+' };
    let a = s.abs();
    let (h, m, sec) = (a / 3600, a % 3600 / 60, a % 60);
    let mut parts = Vec::new();
    if h > 0 {
        parts.push(format!("{h} h"));
    }
    if m > 0 {
        parts.push(format!("{m} min"));
    }
    if sec > 0 || parts.is_empty() {
        parts.push(format!("{sec} s"));
    }
    format!("{sign}{}", parts.join(" "))
}

/// Istante corrente e gruppo di un file.
pub fn instant_of(c: &Connection, id: MediaId) -> StoreResult<(i64, i64)> {
    c.query_row(
        "SELECT instant_utc, group_id FROM media_dates WHERE media_id = ?1 AND instant_utc IS NOT NULL",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound(format!("istante del file {id}")))
}

/// Proposta di allineamento: lo scostamento (assoluto) da dare al gruppo di
/// `other` perché il suo istante coincida con quello di `reference`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Alignment {
    pub group: i64,
    pub current_shift_s: i64,
    pub proposed_shift_s: i64,
    /// Differenza applicata (proposto − attuale).
    pub delta_s: i64,
}

pub fn propose_alignment(
    c: &Connection,
    reference: MediaId,
    other: MediaId,
) -> StoreResult<Alignment> {
    let (a, ga) = instant_of(c, reference)?;
    let (b, gb) = instant_of(c, other)?;
    if ga == gb {
        return Err(StoreError::Invalid(
            "le due foto appartengono allo stesso gruppo di sorgente".into(),
        ));
    }
    let current = group_clock(c, gb)?.shift_s;
    let delta = ((a - b) as f64 / 1000.0).round() as i64;
    Ok(Alignment {
        group: gb,
        current_shift_s: current,
        proposed_shift_s: current + delta,
        delta_s: delta,
    })
}

/// Riga di anteprima: ora mostrata prima e dopo una correzione.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ClockPreview {
    pub media_id: MediaId,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Anteprima delle prime `limit` foto del gruppo con correzioni diverse.
pub fn preview_clock(
    c: &Connection,
    group: i64,
    clock: &GroupClock,
    limit: usize,
    system: &SystemOffset,
) -> StoreResult<Vec<ClockPreview>> {
    let mut q = c.prepare(
        "SELECT media_id, captured_at, declared_offset_min, instant_utc, local_offset_min FROM media_dates
         WHERE group_id = ?1 AND captured_at IS NOT NULL ORDER BY instant_utc, media_id LIMIT ?2",
    )?;
    let rows = q
        .query_map(params![group, limit as i64], |r| {
            Ok((
                r.get::<_, MediaId>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i32>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, Option<i32>>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .map(|(id, captured, declared, utc, local)| ClockPreview {
            media_id: id,
            before: utc.zip(local).and_then(|(u, l)| local_text(u, l)),
            after: compute(&captured, declared, clock, system)
                .and_then(|i| local_text(i.utc_ms, i.local_offset_min)),
        })
        .collect())
}

// ------------------------------------------------------- foto senza data

/// Imposta (o toglie, con `None`) la collocazione manuale e ricalcola il file.
pub fn set_placement(
    c: &Connection,
    id: MediaId,
    placement: Option<(i64, Option<i32>)>,
    system: &SystemOffset,
) -> StoreResult<()> {
    match placement {
        Some((utc, local)) => {
            c.execute(
                "INSERT INTO media_time_placement(media_id, instant_utc, local_offset_min) VALUES (?1, ?2, ?3)
                 ON CONFLICT(media_id) DO UPDATE SET instant_utc = excluded.instant_utc,
                   local_offset_min = excluded.local_offset_min",
                params![id, utc, local],
            )?;
        }
        None => {
            c.execute("DELETE FROM media_time_placement WHERE media_id = ?1", [id])?;
        }
    }
    refresh_media(c, id, system)
}

/// Colloca una foto senza data subito dopo `after`: istante a metà tra `after`
/// e la foto datata successiva (o +1 s se non ce n'è), fuso locale di `after`.
pub fn place_after(
    c: &Connection,
    id: MediaId,
    after: MediaId,
    system: &SystemOffset,
) -> StoreResult<i64> {
    if id == after {
        return Err(StoreError::Invalid(
            "una foto non si colloca dopo se stessa".into(),
        ));
    }
    let (prev, local): (i64, Option<i32>) = c
        .query_row(
            "SELECT instant_utc, local_offset_min FROM media_dates WHERE media_id = ?1 AND instant_utc IS NOT NULL",
            [after],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound(format!("istante del file {after}")))?;
    let next: Option<i64> = c.query_row(
        "SELECT MIN(d.instant_utc) FROM media_dates d
             WHERE d.instant_utc > ?1 AND d.media_id <> ?2
               AND (d.captured_source <> 'file_system'
                    OR d.media_id IN (SELECT media_id FROM media_time_placement))",
        params![prev, id],
        |r| r.get(0),
    )?;
    let utc = match next {
        Some(n) => prev + (n - prev) / 2,
        None => prev + 1000,
    };
    let previous: Option<(i64, Option<i32>)> = c
        .query_row(
            "SELECT instant_utc, local_offset_min FROM media_time_placement WHERE media_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    set_placement(c, id, Some((utc, local)), system)?;
    crate::bulk::record(
        c,
        "Colloca foto senza data",
        &crate::bulk::UndoAction::Placement {
            media_id: id,
            previous,
        },
    )?;
    Ok(utc)
}

// ------------------------------------------------------- suggerimento di fuso dal GPS

/// Finestra entro cui due scatti di sorgenti diverse si considerano contemporanei.
pub const SUGGEST_WINDOW_MS: i64 = 30 * 60 * 1000;

/// Proposta di ora locale per un gruppo, ricavata dalle foto con GPS di altri
/// gruppi scattate negli stessi istanti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TzSuggestion {
    pub group: i64,
    pub device: Option<String>,
    /// Fuso proposto (IANA) e scostamento corrispondente.
    pub tz: Option<String>,
    pub offset_min: i32,
    /// Ora locale attuale del gruppo.
    pub current_offset_min: i32,
    /// Scatti del gruppo che hanno foto con GPS contemporanee in quel fuso.
    pub count: i64,
}

fn ignore_key(group: i64) -> String {
    format!("tz_suggestion_ignored.{group}")
}

/// Nasconde il suggerimento `offset_min` per il gruppo (finché non cambia).
pub fn ignore_suggestion(c: &Connection, group: i64, offset_min: i32) -> StoreResult<()> {
    c.execute(
        "INSERT INTO settings_kv(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![ignore_key(group), offset_min.to_string()],
    )?;
    Ok(())
}

/// Suggerimenti di fuso: per ogni gruppo si prendono le foto con GPS di altri
/// gruppi entro ±30 minuti dai suoi scatti; se la maggioranza indica un fuso
/// diverso dall'ora locale del gruppo, lo si propone con il numero di scatti.
/// `tz_at` dà il fuso IANA di una coordinata (città più vicina).
pub fn suggest_timezones(
    c: &Connection,
    ids: Option<&[MediaId]>,
    tz_at: &dyn Fn(f64, f64) -> Option<String>,
) -> StoreResult<Vec<TzSuggestion>> {
    let scope = ids.map(serde_json::to_string).transpose()?;
    let mut q = c.prepare(
        "SELECT d.media_id, d.group_id, d.instant_utc, d.local_offset_min, e.gps_lat, e.gps_lon
         FROM media_dates d LEFT JOIN exif e ON e.media_id = d.media_id
         WHERE d.instant_utc IS NOT NULL AND d.group_id IS NOT NULL
           AND d.captured_source <> 'file_system'
           AND (?1 IS NULL OR d.media_id IN (SELECT value FROM json_each(?1)))
         ORDER BY d.instant_utc",
    )?;
    struct Shot {
        group: i64,
        utc: i64,
        local: i32,
        gps: Option<(f64, f64)>,
    }
    let shots: Vec<Shot> = q
        .query_map([scope], |r| {
            let lat: Option<f64> = r.get(4)?;
            let lon: Option<f64> = r.get(5)?;
            Ok(Shot {
                group: r.get(1)?,
                utc: r.get(2)?,
                local: r.get::<_, Option<i32>>(3)?.unwrap_or(0),
                gps: lat.zip(lon),
            })
        })?
        .collect::<Result<_, _>>()?;
    // Fuso di ogni foto con GPS (IANA → scostamento in quell'istante).
    let mut cache: std::collections::HashMap<(i64, i64), Option<String>> = Default::default();
    let gps: Vec<(usize, String, i32)> = shots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let (lat, lon) = s.gps?;
            let key = ((lat * 100.0) as i64, (lon * 100.0) as i64);
            let tz = cache
                .entry(key)
                .or_insert_with(|| tz_at(lat, lon))
                .clone()?;
            let off = tz_offset_at_utc(&tz, s.utc)?;
            Some((i, tz, off))
        })
        .collect();
    let mut out = Vec::new();
    let groups: std::collections::BTreeSet<i64> = shots.iter().map(|s| s.group).collect();
    for g in groups {
        // offset → (scatti del gruppo che lo sostengono, conteggio per nome IANA)
        let mut votes: std::collections::BTreeMap<
            i32,
            (i64, std::collections::BTreeMap<String, i64>),
        > = Default::default();
        let mut current = None;
        for s in shots.iter().filter(|s| s.group == g) {
            current.get_or_insert(s.local);
            let lo = gps.partition_point(|(i, _, _)| shots[*i].utc < s.utc - SUGGEST_WINDOW_MS);
            let mut local: std::collections::BTreeMap<i32, (i64, String)> = Default::default();
            for (i, tz, off) in gps[lo..]
                .iter()
                .take_while(|(i, _, _)| shots[*i].utc <= s.utc + SUGGEST_WINDOW_MS)
            {
                if shots[*i].group == g {
                    continue;
                }
                let e = local.entry(*off).or_insert((0, tz.clone()));
                e.0 += 1;
            }
            // Maggioranza tra le foto contemporanee di questo scatto.
            if let Some((off, (_, tz))) = local.into_iter().max_by_key(|(_, (n, _))| *n) {
                if off != s.local {
                    let v = votes.entry(off).or_default();
                    v.0 += 1;
                    *v.1.entry(tz).or_default() += 1;
                }
            }
        }
        let Some((offset, (count, names))) = votes.into_iter().max_by_key(|(_, (n, _))| *n) else {
            continue;
        };
        let ignored: Option<String> = c
            .query_row(
                "SELECT value FROM settings_kv WHERE key = ?1",
                [ignore_key(g)],
                |r| r.get(0),
            )
            .optional()?;
        if ignored.and_then(|v| v.parse::<i32>().ok()) == Some(offset) {
            continue;
        }
        let device: Option<String> =
            c.query_row("SELECT device FROM source_group WHERE id = ?1", [g], |r| {
                r.get(0)
            })?;
        out.push(TzSuggestion {
            group: g,
            device,
            tz: names.into_iter().max_by_key(|(_, n)| *n).map(|(t, _)| t),
            offset_min: offset,
            current_offset_min: current.unwrap_or(0),
            count,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rome() -> GroupClock {
        GroupClock {
            display_tz: Some("Europe/Rome".into()),
            ..Default::default()
        }
    }
    fn utc(s: &str) -> i64 {
        parse_local(s).unwrap().and_utc().timestamp_millis()
    }
    fn fixed(min: i32) -> impl Fn(NaiveDateTime) -> i32 {
        move |_| min
    }

    #[test]
    fn declared_offset_is_not_presumed() {
        let i = compute(
            "2026-08-19 10:43:00",
            Some(-360),
            &GroupClock::default(),
            &fixed(60),
        )
        .unwrap();
        assert_eq!(i.utc_ms, utc("2026-08-19 16:43:00"));
        assert!(!i.presumed);
        assert_eq!(i.local_offset_min, -360);
        assert_eq!(
            local_text(i.utc_ms, i.local_offset_min).as_deref(),
            Some("2026-08-19 10:43:00")
        );
    }

    #[test]
    fn group_timezone_follows_daylight_saving_and_is_presumed() {
        let summer = compute("2026-08-19 10:43:00", None, &rome(), &fixed(0)).unwrap();
        assert_eq!(summer.utc_ms, utc("2026-08-19 08:43:00"));
        assert!(summer.presumed);
        assert_eq!(summer.local_offset_min, 120);
        let winter = compute("2026-01-10 10:43:00", None, &rome(), &fixed(0)).unwrap();
        assert_eq!(winter.utc_ms, utc("2026-01-10 09:43:00"));
        assert_eq!(winter.local_offset_min, 60);
        // 29 marzo 2026, 02:30 non esiste a Roma: vale il fuso prima del salto.
        let gap = compute("2026-03-29 02:30:00", None, &rome(), &fixed(0)).unwrap();
        assert_eq!(gap.utc_ms, utc("2026-03-29 01:30:00"));
        // 25 ottobre 2026, 02:30 c'è due volte: la prima (ancora ora legale).
        let twice = compute("2026-10-25 02:30:00", None, &rome(), &fixed(0)).unwrap();
        assert_eq!(twice.utc_ms, utc("2026-10-25 00:30:00"));
    }

    #[test]
    fn system_timezone_when_nothing_else_is_known() {
        let i = compute(
            "2026-08-19 10:43:00",
            None,
            &GroupClock::default(),
            &fixed(120),
        )
        .unwrap();
        assert_eq!(i.utc_ms, utc("2026-08-19 08:43:00"));
        assert!(i.presumed);
        assert_eq!(i.local_offset_min, 120);
    }

    #[test]
    fn display_timezone_changes_shown_time_not_instant_of_declared_photos() {
        let g = GroupClock {
            display_offset_min: Some(-420),
            ..Default::default()
        };
        let before = compute(
            "2026-08-19 10:43:00",
            Some(120),
            &GroupClock::default(),
            &fixed(0),
        )
        .unwrap();
        let after = compute("2026-08-19 10:43:00", Some(120), &g, &fixed(0)).unwrap();
        assert_eq!(before.utc_ms, after.utc_ms);
        assert_eq!(
            local_text(after.utc_ms, after.local_offset_min).as_deref(),
            Some("2026-08-19 01:43:00")
        );
    }

    #[test]
    fn shift_moves_the_instant() {
        let g = GroupClock {
            shift_s: 130,
            ..Default::default()
        };
        let i = compute("2026-08-19 10:41:50", Some(-360), &g, &fixed(0)).unwrap();
        assert_eq!(
            local_text(i.utc_ms, i.local_offset_min).as_deref(),
            Some("2026-08-19 10:44:00")
        );
    }

    #[test]
    fn offsets_and_bad_input() {
        assert_eq!(offset_text(-360), "-06:00");
        assert_eq!(offset_text(345), "+05:45");
        assert!(compute("19/08/2026", None, &GroupClock::default(), &fixed(0)).is_none());
        assert!(
            tz_offset_at_local("Marte/Olympus", parse_local("2026-01-01 00:00:00").unwrap())
                .is_none()
        );
        assert!(valid_tz("America/Denver"));
    }
}
