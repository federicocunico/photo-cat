//! Sviluppo dei RAF con la fotocamera (porting di `variants/` di FujiView, commit a798987).
//!
//! - `plan`, `estimate`, `types`: piano delle conversioni, stime di spazio e tempo, tipi per la UI;
//!   si compilano e si testano su ogni piattaforma.
//! - `diagnostics`, `service`: verifica della fotocamera e thread che la possiede; su Windows
//!   parlano con la fotocamera reale, altrove solo con il simulatore (`fake-camera`).
//! - `runner`, `job`: l'esecuzione, convertita in un job di `pc-jobs` (D6).
//! - `prefs`: i preset scelti l'ultima volta.

pub mod diagnostics;
pub mod estimate;
pub mod job;
pub mod plan;
pub mod prefs;
pub mod runner;
pub mod service;
pub mod types;
