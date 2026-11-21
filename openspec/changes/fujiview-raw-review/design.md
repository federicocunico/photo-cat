# Design

## Context

FujiView (`D:\Github\fujiview`, v0.2.7) è un'app Tauri 2 + React a sé stante:

- **Backend** (`src-tauri/src`), circa 6.000 righe:
  - `lib.rs`: scansione delle cartelle per preset, anteprime turbojpeg con cache SQLite LRU da 20 GB, export `export_winners` con copia atomica e rinomina `<prefisso>_NNNN` per ora di scatto;
  - `metadata.rs`: EXIF e ricetta dai MakerNote Fujifilm;
  - `persist.rs`: progetto JSON per cartella in `%APPDATA%`, scrittura atomica con `.bak`;
  - `camera/` (WPD, PTP, profilo, banchi, simulatore) e `variants/` (piano, stima, runner, servizio, diagnostica): conversione dalla fotocamera, solo Windows.
- **Frontend** (`src/`): `App.tsx` (confronto), `ExportDialog`, `VariantGenerator`, `InfoSidebar`, `CameraDiagnostics`, e la logica pura in `project.ts`, `export.ts` e `variants.ts`, con test vitest.
- **Limiti attuali**:
  - la scansione ignora i RAF e conosce solo il layout per preset;
  - Ctrl+1–9 scelgono la variante, ma in photo-cat sono le scorciatoie dei moduli;
  - il job delle varianti ha un proprio stato e propri eventi, fuori dalle attività di photo-cat.

photo-cat offre già i pezzi riusabili:

- `pc-ingest`: import di una cartella e aggiornamento di una sottocartella, con estrazione dell'anteprima dai RAW;
- `pc-organize::sequence::naming`: modello dei nomi con token e anteprima;
- `pc-store`: cataloghi manuali, tag e set esclusivi, e la valutazione introdotta da `github-issues-39-47`;
- `pc-jobs`: attività a fasi persistenti, con pausa, ripresa e journal;
- protocolli URI personalizzati (`thumb://`, `face://`) per servire le immagini alla webview.

## Goals / Non-Goals

**Goals:**
- portare FujiView in photo-cat senza perdere prestazioni né la sicurezza sui file (RAF intatti, scritture atomiche);
- una sola infrastruttura di attività, errori, i18n e stile;
- un percorso unico apri → (sviluppa) → confronta → esporta → catalogo.

**Non-Goals:**
- sviluppo RAW software (demosaicizzazione) senza fotocamera;
- supporto di fotocamere non Fujifilm o di RAW non RAF per lo sviluppo in camera;
- sincronizzare in futuro photo-cat e il FujiView standalone: dopo il porting il codice evolve solo in photo-cat;
- copia dei RAF o di file non JPEG nella destinazione dell'export: esclusa per scelta di prodotto, senza opzione.

## Decisions

### D1. Crate `pc-raw-review` con porting, non dipendenza
I moduli di FujiView si copiano in `crates/pc-raw-review/src/`:

- `scan.rs`, `preview.rs`, `metadata.rs`, `session.rs` (ex `persist.rs` + `project.ts` lato dati), `export.rs`;
- `camera/` e `variants/` sotto `#[cfg(windows)]`, con `sim.rs` disponibile ovunque dietro la feature `fake-camera`, che sostituisce `FUJIVIEW_FAKE_CAMERA`.

Si tolgono le dipendenze da `tauri::AppHandle`: i percorsi dei dati arrivano da `pc_core::paths::AppPaths`. Così il crate si testa con `cargo test` anche nel container Linux, dove `camera/` reale è esclusa e il simulatore è attivo.

*Alternativa scartata:* dipendere dal crate di fujiview via path o git. Richiede prima di separare fujiview dalla sua app Tauri, e lascia due progetti da tenere allineati (`rusqlite` 0.37 contro 0.32, toolchain turbojpeg).

### D2. Scansione con tre forme di cartella
`scan(root)` restituisce `ReviewIndex { root, layout, presets, guide_preset, shots, warnings }` con `layout ∈ {RafOnly, RafWithVariants{output_dir}, Presets}`. Le regole:

- **RAF al primo livello**: ogni RAF è uno scatto. I preset arrivano dalle sottocartelle di `FujiView varianti/`, o dalla cartella di output salvata nella sessione se l'utente l'ha cambiata.
- **Nessun RAF, sottocartelle con JPEG o TIFF**: comportamento FujiView invariato, con il preset più numeroso come guida.
- **Abbinamento e ordine**: per nome base in minuscolo, ordine naturale (`natord`).

Ogni scatto RAF ha sempre la pseudo-variante `raw-preview` ("Anteprima RAW"), estratta con la funzione di `pc-ingest` già usata per le miniature RAW. `raw-preview` si può valutare ma non scegliere come migliore: la UI e il backend (`set_winner`) la rifiutano.

### D3. Anteprime: protocollo `review://` e cache dedicata
Le anteprime si servono con un nuovo protocollo URI `review://<chiave>`, come `thumb://`, invece di `protocol-asset` di FujiView: niente ampliamento dello scope degli asset Tauri, e gli stessi controlli d'accesso dei protocolli esistenti.

La cache sta in `<app data>/raw-review/previews/` con un indice SQLite proprio (rusqlite 0.32), separato dal database della libreria. Il limite predefinito è 20 GB, configurabile in Impostazioni.

Per la decodifica, turbojpeg è dietro la feature `simd-jpeg`, attiva nelle build di release Windows e macOS. Senza la feature si usa il crate `image` con riduzione in decodifica dove possibile. Un benchmark `#[ignore]` (porting di `benchmark_real_preview_pipeline`) verifica i 150 ms della spec con `simd-jpeg`.

*Alternativa scartata:* rendere obbligatorio turbojpeg ovunque. Servirebbero cmake e nasm nell'immagine Docker di CI e nelle build Linux.

### D4. Sessione di revisione
`session.rs` porta lo schema `Project` di FujiView (`schemaVersion: 1`) con i campi aggiunti `layout` e `variantsOutputDir`, e diventa la versione 2. Al caricamento:

- un file v1 (anche `.fujiview`) viene letto e aggiornato in memoria;
- `merge` (porting di `mergeProject`) aggiunge scatti e preset nuovi;
- il salvataggio è atomico con `.bak` (porting di `write_json_atomic`).

Le sessioni stanno in `<app data>/raw-review/sessions/<blake3(root canonico)>.json`. Un indice `recent.json` elenca le sessioni recenti. Il salvataggio automatico ha un debounce di 500 ms lato UI.

### D5. Scorciatoie
- La scelta della variante passa da Ctrl+1–9 a **Maiusc+1–9**, oppure clic sul pannello e Invio, perché Ctrl+1–9 restano ai moduli. Ctrl+9 apre il modulo stesso.
- La valutazione usa 1–5 e 0, come in FujiView, più Alt+1–5 e Alt+0 per coerenza con la Libreria: in questo modulo non ci sono tag da tastiera, quindi nessun conflitto.
- Spazio e Maiusc+Spazio restano per scorrere gli scatti.

Le scorciatoie del modulo si registrano solo quando il modulo è attivo e il focus non è in un campo di testo.

### D6. Sviluppo dalla fotocamera come attività `pc-jobs`
`variants::runner` e `variants::service` diventano un `JobPlan` con una fase per conversione:

- chiave del journal: `<raf>|<preset folder>`;
- la ripresa e il salto delle varianti presenti usano la verifica di completezza già fatta da `plan.rs`;
- la pausa per disco pieno o fotocamera scollegata usa la pausa delle attività, con un motivo che la UI mostra.

Il servizio della fotocamera, con la sessione esclusiva, resta un singleton nello stato Tauri, e il job lo acquisisce per tutta la sua durata. Gli errori per RAF vanno nel registro errori con fase `sviluppo_raw` e codici `RAW_CAMERA_REJECTED`, `CAMERA_DISCONNECTED` e simili, tradotti in i18n.

La parte dei messaggi viene rivista: stato e progresso passano dagli eventi delle attività, non più dagli eventi `variant-progress` di FujiView.

### D7. Export in catalogo come attività a tre fasi
`raw_review_export` crea un job con tre fasi:

1. **copia**: porting di `export_winners`, con la whitelist `jpg`/`jpeg` al posto di `image_extension` (che accetta anche i TIFF), file temporaneo, rinomina, backup del file sovrascritto e ripristino in caso di errore;
2. **import**: se la destinazione è dentro una sorgente, aggiornamento della cartella (`media-ingestion` "Aggiornamento di una cartella"); altrimenti nuova sorgente, con `recursive=false` e `watch=false`, seguita da import;
3. **catalogo**: risoluzione dei media per percorso, creazione del catalogo manuale o aggiunta a uno esistente, valutazione in bulk, tag del set esclusivo "Preset Fujifilm" se l'opzione è attiva.

La fase 3 è una transazione unica con un'unica voce di annullamento: "Annulla" toglie valutazioni, tag e catalogo, ma non i file né la sorgente.

I nomi vengono da `pc-organize::sequence::naming` con un contesto ridotto: data originale, dispositivo da EXIF, nome originale, e il nuovo token `{preset}`, aggiunto al motore e rifiutato fuori dalla revisione RAW. L'ordine di `{seq}` segue la chiave di FujiView (`DateTimeOriginal` con i sottosecondi, poi nome naturale).

Il conflitto sul nome del catalogo si risolve prima della fase 1 con `catalog_by_name`. Lo scenario "File saltati" è coperto perché la fase 3 risolve tutti i percorsi di destinazione attesi, non solo quelli copiati.

*Alternativa scartata:* creare il catalogo dagli id restituiti dall'import. I file saltati perché già presenti non verrebbero restituiti.

### D8. UI del modulo
`app/ui/src/modules/raw-review/` contiene:

- `RawReviewPage` (vuoto, recenti, apertura);
- `ComparePanels` (porting di `App.tsx`, griglia `panelGrid`, zoom e spostamento sincronizzati in un unico stato di trasformazione);
- `ShotList` (barra laterale virtualizzata, con selezione multipla tramite `useGridSelection` di `github-issues-39-47` per "Sviluppa selezionati");
- `InfoSidebar`, `DevelopDialog` (ex `VariantGenerator`), `CameraCheck` (ex `CameraDiagnostics`) e `ExportToCatalogDialog` (ex `ExportDialog` più nome del catalogo, modello dei nomi e opzione preset come tag).

La logica pura (`project.ts`, `export.ts`, `variants.ts`) si porta con i suoi test vitest. Testi in i18n it/en tramite chiavi, nessuna stringa nel codice, voce nella guida in app.

## Risks / Trade-offs

- **[Rischio]** Il codice WPD/PTP non si può testare in CI. → Il simulatore (`fake-camera`) copre piano, ripresa, annullamento ed errori. Una prova manuale con la X-T5 reale è un task di verifica esplicito.
- **[Rischio]** Senza `simd-jpeg` le anteprime su Linux sono più lente. → La spec di 150 ms vale per gli scatti precaricati. Il precaricamento resta, e il benchmark documenta i tempi delle due strade.
- **[Rischio]** L'aggiunta automatica di una sorgente può sorprendere. → Il dialogo di export mostra prima dell'avvio "La cartella verrà aggiunta alla libreria" oppure "Verrà aggiornata la cartella … della libreria".
- **[Trade-off]** Il token `{preset}` esiste solo nella revisione RAW, quindi il motore dei nomi ha un contesto opzionale. È accettato per non duplicare il motore.
- **[Rischio]** Crate grande da portare in una volta. → Il porting procede per parti con test a ogni passo: scansione e sessioni, anteprime, export, fotocamera.

## Migration Plan

- Nessuna migrazione dello schema della libreria: valutazione e tag arrivano da `github-issues-39-47`.
- I dati del modulo stanno in `<app data>/raw-review/` e si possono cancellare senza effetti sulla libreria.
- Le sessioni FujiView esistenti si aprono come file `.fujiview`. Non c'è un'importazione in blocco dalla cartella `%APPDATA%\com.fujiview.desktop`.
- Per tornare indietro basta togliere il modulo dalla navigazione: la libreria non dipende da `pc-raw-review`.
