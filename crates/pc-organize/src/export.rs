use crate::Scope;
use pc_core::{AppError, AppResult};
use pc_jobs::{JobPlan, PhaseSpec, RunSummary};
use pc_store::{detail::MediaDetail, Store};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    io::{BufWriter, Write},
    path::PathBuf,
};
pub const JOB_KIND: &str = "export";
pub const FIELDS: &[&str] = &[
    "path",
    "name",
    "format",
    "width",
    "height",
    "dpi_x",
    "dpi_y",
    "bit_depth",
    "size",
    "captured_at",
    "mtime",
    "location",
    "location_source",
    "people_present",
    "people_count",
    "people",
    "subject",
    "location_type",
    "environment",
    "tags",
    "catalogs",
    "labels",
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Csv,
    Json,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub scope: Scope,
    pub destination: PathBuf,
    pub format: Format,
    pub separator: char,
    pub bom: bool,
    pub fields: Vec<String>,
}
pub type Options = ExportOptions;
fn record(d: MediaDetail, fields: &[String]) -> Map<String, Value> {
    let label = |group: &str| {
        d.labels
            .iter()
            .filter(|l| l.group == group)
            .map(|l| l.value.clone())
            .collect::<Vec<_>>()
    };
    let all = json!({"path":d.item.path,"name":d.item.name,"format":d.item.ext,"width":d.tech.as_ref().and_then(|t|t.width),"height":d.tech.as_ref().and_then(|t|t.height),"dpi_x":d.tech.as_ref().and_then(|t|t.dpi_x),"dpi_y":d.tech.as_ref().and_then(|t|t.dpi_y),"bit_depth":d.tech.as_ref().and_then(|t|t.bit_depth),"size":d.item.size,"captured_at":d.dates.as_ref().and_then(|v|v.captured_at.as_ref()),"mtime":d.item.mtime,"location":d.location.as_ref().and_then(|l|l.name.as_ref()),"location_source":d.location.as_ref().map(|l|&l.source),"people_present":d.people_count.map(|n|n>0),"people_count":d.people_count,"people":d.people,"subject":label("subject"),"location_type":label("location"),"environment":label("environment"),"tags":d.tags,"catalogs":d.catalogs,"labels":d.labels});
    fields.iter().map(|f| (f.clone(), all[f].clone())).collect()
}
fn csv_cell(w: &mut impl Write, value: &str, separator: char) -> std::io::Result<()> {
    if value.contains([separator, '"', '\n', '\r']) {
        write!(w, "\"{}\"", value.replace('"', "\"\""))
    } else {
        write!(w, "{value}")
    }
}
pub fn plan(store: Store, params: &Value) -> AppResult<JobPlan> {
    let opts: Options = serde_json::from_value(params.clone())?;
    validate(&opts)?;
    Ok(JobPlan {
        phases: vec![PhaseSpec::new("esportazione", 1.0)],
        min_memory_mb: 16,
        run: Box::new(move |ctx| {
            let ids = crate::ids(&store, &opts.scope)?;
            let p = ctx.progress.phase("esportazione");
            p.set_total(ids.len() as i64);
            p.start();
            let dest = crate::paths::writable_destination(&store, &opts.destination)?;
            let parent = dest
                .parent()
                .ok_or_else(|| AppError::invalid_input("cartella destinazione mancante"))?;
            // Il file temporaneo resta nella destinazione: rename atomico, nessun parziale.
            let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
            {
                let mut out = BufWriter::new(tmp.as_file_mut());
                match opts.format {
                    Format::Json => write!(
                        out,
                        "{{\"schema_version\":1,\"generated_at\":{},\"records\":[",
                        json!(chrono::Utc::now().to_rfc3339())
                    )?,
                    Format::Csv => {
                        if opts.bom {
                            out.write_all(b"\xef\xbb\xbf")?;
                        }
                        write!(out, "schema_version")?;
                        for f in &opts.fields {
                            write!(out, "{}", opts.separator)?;
                            csv_cell(&mut out, f, opts.separator)?;
                        }
                        writeln!(out)?;
                    }
                }
                for (index, id) in ids.into_iter().enumerate() {
                    if !ctx.control.checkpoint() {
                        return Ok(RunSummary {
                            cancelled: true,
                            failed: 0,
                        });
                    }
                    let d = store.read(|c| pc_store::detail::media_detail(c, id))?;
                    let row = record(d, &opts.fields);
                    match opts.format {
                        Format::Json => {
                            if index > 0 {
                                write!(out, ",")?;
                            }
                            serde_json::to_writer(&mut out, &row)?;
                        }
                        Format::Csv => {
                            write!(out, "1")?;
                            for f in &opts.fields {
                                write!(out, "{}", opts.separator)?;
                                let v = &row[f];
                                let cell = match v {
                                    Value::Null => String::new(),
                                    Value::String(s) => s.clone(),
                                    _ => v.to_string(),
                                };
                                csv_cell(&mut out, &cell, opts.separator)?;
                            }
                            writeln!(out)?;
                        }
                    }
                    p.inc_done(1);
                }
                if matches!(opts.format, Format::Json) {
                    write!(out, "]}}")?;
                }
                out.flush()?;
            }
            if !ctx.control.checkpoint() {
                return Ok(RunSummary {
                    cancelled: true,
                    failed: 0,
                });
            }
            tmp.as_file().sync_all()?;
            tmp.persist_noclobber(&dest)
                .map_err(|e| AppError::from(e.error))?;
            p.finish();
            Ok(RunSummary {
                cancelled: false,
                failed: 0,
            })
        }),
    })
}
fn validate(o: &Options) -> AppResult<()> {
    if o.fields.is_empty() || o.fields.iter().any(|s| !FIELDS.contains(&s.as_str())) {
        return Err(AppError::invalid_input("campi export non validi"));
    }
    if o.separator.is_control() || matches!(o.separator, '"') {
        return Err(AppError::invalid_input("separatore CSV non valido"));
    }
    Ok(())
}
