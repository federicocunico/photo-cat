# Proposal

## Why

Dopo uno shooting con la Fujifilm l'utente oggi alterna due applicazioni:

1. **FujiView** (`D:\Github\fujiview`): fa sviluppare i RAF dalla fotocamera con le ricette C1–C7, confronta le varianti, sceglie la migliore di ogni scatto e la esporta rinominata;
2. **photo-cat**: importa a mano la cartella esportata e ci crea un catalogo.

Portare FujiView dentro photo-cat come modulo elimina il passaggio manuale. La revisione lavora su una cartella di RAF **senza importarla**, e il risultato dell'export diventa direttamente un nuovo catalogo con il nome scelto, con valutazioni e preset già impostati.

## What Changes

- **Nuovo modulo "Varianti RAW (Fujifilm X-T5)"** (porting di FujiView), nel gruppo "Importa ed esporta" con scorciatoia Ctrl+9:
  - **apre** una cartella senza aggiungerla alla libreria e riconosce tre forme:
    - cartella di RAF;
    - cartella di RAF con le varianti già generate (`FujiView varianti/`);
    - cartella con una sottocartella per preset (layout FujiView classico);
  - a ogni cartella aperta corrisponde una **sessione di revisione** salvata nei dati dell'app, mai nelle cartelle delle foto: scelte, valutazioni, preset nascosti, ultimo scatto.
- **Sviluppo su richiesta** (solo Windows): genera dalla fotocamera collegata via USB le varianti "Come scattato" e C1–C7 per tutti gli scatti o solo per quelli scelti.
  - Gira come attività di photo-cat, con pausa e ripresa, salta le varianti già presenti e controlla lo spazio su disco.
  - I RAF non vengono mai modificati.
  - Su Linux e macOS la voce non è disponibile e l'interfaccia ne indica il motivo.
- **Confronto**:
  - un pannello per preset, fino a 9, con zoom e spostamento sincronizzati e metadati e ricetta Fujifilm di ogni variante;
  - scelta della variante migliore e valutazione 0–5, da tastiera e da mouse;
  - uno scatto senza varianti si confronta e si valuta sull'anteprima incorporata nel RAF, ma per esportarlo va prima sviluppato.
- **Esporta in un nuovo catalogo**:
  - **filtri**: valutazione minima, solo gli scatti con una scelta, preset;
  - **destinazione**: una cartella a scelta;
  - **rinomina**: con il modello di nomi già usato dall'export della Sequenza; predefinito `<nome>_{seq:04d}` in ordine di scatto;
  - **copia** byte per byte dei soli JPEG scelti: RAF e altri formati non finiscono mai nella destinazione;
  - **ultimo passo**: import della destinazione in libreria e creazione di un **catalogo manuale con il nome indicato**. Le foto ricevono la valutazione data in revisione e, se l'opzione è attiva, il preset come tag del set "Preset Fujifilm".

## Capabilities

### New Capabilities

- `raw-review`: apertura di una cartella di RAF o varianti senza import, sessione di revisione persistente, confronto delle varianti, scelta e valutazione.
- `raw-review-export`: export delle scelte con filtri e rinomina e creazione del nuovo catalogo in libreria.
- `camera-connection`: rilevamento della fotocamera Fujifilm in USB RAW CONV./BACKUP RESTORE su Windows, sessione esclusiva, disconnessioni ed errori. Porting dalla change FujiView `add-camera-preset-conversion`.
- `camera-presets`: lettura dei banchi C1–C7, scelta dei preset, nomi delle cartelle, fedeltà della ricetta. Porting.
- `variant-generation`: conversione RAF → JPEG per preset come attività ripresa e idempotente. Porting adattato alle attività di photo-cat.

### Modified Capabilities

- `desktop-shell`: "Moduli di navigazione" con il nuovo modulo "Varianti RAW (Fujifilm X-T5)" e la scorciatoia Ctrl+9.

## Impact

- **Rust**:
  - nuovo crate `crates/pc-raw-review`, porting di `fujiview/src-tauri/src`:
    - scansione;
    - metadati e ricetta Fujifilm;
    - anteprime con cache;
    - `camera/` (WPD/PTP, solo `cfg(windows)`);
    - `variants/`;
    - export;
  - attività di generazione in `pc-jobs`;
  - comandi Tauri in `app/src-tauri/src/raw_review.rs` con binding `specta`;
  - export e import riusano `pc-organize` (modello dei nomi della Sequenza), `pc-ingest` (import di una cartella) e `pc-store` (catalogo manuale, valutazione, tag).
- **Dipendenze**:
  - `windows` (WPD) solo su Windows;
  - `natord`;
  - `turbojpeg` opzionale dietro una feature, con fallback sul crate `image` già usato;
  - `rusqlite` allineato alla 0.32 del workspace.
- **UI**: modulo `app/ui/src/modules/raw-review/` (porting di `App.tsx`, `ExportDialog`, `VariantGenerator`, `InfoSidebar`, `CameraDiagnostics` nei componenti e negli stili di photo-cat), i18n it/en, voce della guida.
- **Dipendenze tra change**: richiede `github-issues-39-47` per la valutazione 0–5 delle foto in libreria.
- **FujiView standalone**: resta un repository separato e non è una dipendenza. Questa change non migra le sue sessioni `.fujiview` esistenti, che si possono però aprire (vedi spec `raw-review`).
