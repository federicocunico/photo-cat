# Tasks

> Esecuzione: ogni task numerato va a un subagente **Sonnet**. I task "Revisione Opus" chiudono una macroparte e vanno a un subagente **Opus**. Prerequisito: change `github-issues-39-47` applicata (valutazione 0–5 e `useGridSelection`). Sorgente del porting: `D:\Github\fujiview` al commit `a798987`.

## 1. Crate e scansione

- [x] 1.1 Creare `crates/pc-raw-review` nel workspace (D1):
  - dipendenze `natord`, `blake3`, `kamadak-exif`, `image`, `rusqlite` 0.32;
  - `windows` solo su `cfg(windows)`;
  - feature `simd-jpeg` (turbojpeg) e `fake-camera`.

  Verifica: `cargo check -p pc-raw-review` su Windows e nel container Linux.
- [x] 1.2 Porting di `scan_library_impl` in `scan.rs` con le tre forme di cartella e la pseudo-variante "Anteprima RAW" (D2). Porting dei test di FujiView più nuovi test:
  - solo RAF;
  - RAF con `FujiView varianti/`;
  - variante mancante;
  - duplicato con avviso;
  - cartella vuota con errore spiegato;
  - ordine naturale;
  - "Anteprima RAW" rifiutata come variante migliore (`set_winner`).
- [x] 1.3 Porting di `metadata.rs` (EXIF e ricetta MakerNote). I test di FujiView devono passare invariati.
- [x] 1.4 `session.rs` (D4): schema v2, lettura di v1 e dei file `.fujiview`, `merge`, scrittura atomica con `.bak`, indice delle sessioni recenti in `<app data>/raw-review/`. Test Rust:
  - riapertura con 80 scelte;
  - nuovo preset che non tocca le scelte;
  - `.bak` usato se il file è corrotto;
  - nessun file scritto nella cartella delle foto.

## 2. Anteprime

- [x] 2.1 `preview.rs` (D3): decodifica ridotta (turbojpeg con `simd-jpeg`, altrimenti `image`), orientamento EXIF, cache con indice SQLite e LRU con limite configurabile, coalescenza delle richieste doppie. Test:
  - anteprima limitata;
  - eviction oltre il limite;
  - chiave invalidata quando il file cambia.

  Benchmark `#[ignore]` a 2048 px.
- [x] 2.2 Protocollo `review://` in `app/src-tauri` con controllo che la chiave appartenga a una sessione aperta, e precaricamento a code separate (anteprime interattive, scatti vicini +4/−2, miniature). Test Rust del rifiuto di chiavi estranee.
- [x] 2.3 Revisione Opus delle parti 1 e 2: fedeltà del porting, nessuna scrittura nelle cartelle sorgente, prestazioni.

  Per i task successivi:
  - 3.2: aprire con `session::open_folder` / `session::open_fujiview` (sessione salvata o `.fujiview`, cartella delle varianti, merge e salvataggio) e poi `ReviewState::register(&opened.index)`; dopo uno sviluppo (5.3) `ReviewState::refresh` mantiene l'id della sessione.
  - 3.3: l'"Anteprima RAW" (`raw-preview`) è un pannello di `panelPresets` nelle cartelle con RAF; i testi dei metadati passano da `modules/raw-review/metaText.ts` (`valueKey` con fallback al testo).
  - 4.2: una scelta su un preset assente dall'indice resta salvata (disco delle varianti scollegato): l'export deve verificare che la variante esista.
  - 5.1: sostituire nel test di `metadata.rs` la tabella D192 locale con `camera::banks::film_simulation_name` (TODO nel codice).

## 3. Modulo UI di revisione

- [x] 3.1 Registrare il modulo "Varianti RAW (Fujifilm X-T5)" nel gruppo "Importa ed esporta" con Ctrl+9, `Destination` sulla sessione e comando di palette. i18n it/en. Test vitest: navigazione e scenari "Scorciatoia della revisione RAW" e "Gruppi della navigazione".
- [x] 3.2 Pagina iniziale: scelta della cartella, sessioni recenti, apertura di `.fujiview` e messaggio per le cartelle non supportate. Comandi Tauri `raw_review_open`, `raw_review_recent` e `raw_review_save` con binding. Test vitest con comandi simulati.
- [x] 3.3 Porting di `project.ts` ed `export.ts` con i loro test. `ComparePanels` con griglia adattiva, zoom e spostamento sincronizzati, preset nascosti e ordinati, e stato "variante non disponibile". `InfoSidebar` con ricetta. Test vitest della sincronizzazione e della visibilità dei preset.
- [x] 3.4 `ShotList` virtualizzata con selezione multipla (`useGridSelection`). Scorciatoie del modulo (D5): Maiusc+1–9, Invio, 1–5/0, Alt+1–5/Alt+0, Spazio/Maiusc+Spazio, ignorate nei campi di testo. Salvataggio automatico con debounce. Test vitest:
  - Maiusc+2 e poi 4 non fanno avanzare;
  - Ctrl+3 apre Cataloghi;
  - la sessione viene salvata.
- [x] 3.5 Revisione Opus della parte 3: aderenza alla spec `raw-review`, accessibilità da tastiera, fluidità con 160 scatti × 8 preset.

## 4. Export in un nuovo catalogo

- [x] 4.1 Token `{preset}` e contesto ridotto nel motore dei nomi di `pc-organize::sequence::naming`; il token viene rifiutato fuori dalla revisione. Test Rust:
  - numerazione per data di scatto ("Dolomiti_0001…");
  - suffisso per i nomi uguali;
  - modello non valido segnalato.
- [x] 4.2 Job `raw_review_export` (D7):
  - fase di copia (porting di `export_winners`) con la whitelist JPEG, con test di hash identico, file saltati, nessun file parziale, nessun `.RAF` nella destinazione e TIFF esclusi;
  - fase di import (sorgente nuova o aggiornamento della cartella);
  - fase del catalogo (nuovo o esistente, valutazioni, tag del set "Preset Fujifilm"), con annullamento unico.

  Test Rust degli scenari di `raw-review-export`: destinazione già nella libreria, file saltati nel catalogo, errore su un file, annullamento dopo la copia.
- [x] 4.3 `ExportToCatalogDialog`:
  - conteggi per valutazione;
  - conteggio degli esclusi perché "non JPEG";
  - filtro per preset;
  - nome del catalogo con controllo di unicità ("Aggiungi all'esistente" / "Cambia nome");
  - modello dei nomi con anteprima;
  - "Sovrascrivi" e "Aggiungi il preset come tag";
  - avviso sulla sorgente che verrà aggiunta o aggiornata;
  - riepilogo con "Apri il catalogo".

  Test vitest dei conteggi, del conflitto di nome e del riepilogo.
- [x] 4.4 Revisione Opus della parte 4: atomicità della copia, coerenza tra file copiati e catalogo, annullamento.

## 5. Sviluppo dalla fotocamera (Windows)

- [x] 5.1 Porting di `camera/` (WPD, PTP, fuji, profile, banks) sotto `cfg(windows)` e di `sim.rs` dietro `fake-camera`. Test di `banks` e `profile` (nomi delle cartelle `C3 Neg-Hi- 2`, banco senza nome) eseguiti su tutte le piattaforme.
- [x] 5.2 Porting di `variants/` (plan, estimate, diagnostics) e conversione del runner in `JobPlan` di `pc-jobs` (D6): journal per `<raf>|<preset>`, pausa per disco o fotocamera, errori nel registro con fase `sviluppo_raw`. Test con il simulatore:
  - ripresa che salta 400 conversioni;
  - annullamento senza file parziali;
  - RAF di un altro corpo saltato;
  - spazio insufficiente che blocca l'avvio;
  - RAF con hash invariato.
- [x] 5.3 `DevelopDialog` (tutti gli scatti o i selezionati, preset ricordati, stima di spazio e tempo, cartella di output, "Sovrascrivi") e `CameraCheck`. Su Linux e macOS la voce è disattivata con "Disponibile solo su Windows". Al termine la sessione si aggiorna da sola con i nuovi pannelli. Test vitest con comandi simulati, incluso il caso non Windows.
- [x] 5.4 Revisione Opus della parte 5: sicurezza dei file, gestione della sessione esclusiva, messaggi d'errore comprensibili.

## 6. Verifica

- [ ] 6.1 `cargo test` del workspace (con `fake-camera` nei test), typecheck, lint e test UI, poi `make test-linux` con i nuovi file in stage. Correggere ogni errore.
- [ ] 6.2 Documentare il modulo nella guida in app e nel README (forme di cartella, sviluppo solo su Windows, export in catalogo). Verificare che la guida si apra dal modulo.
- [ ] 6.3 Prova manuale nell'app (skill `run`) su Windows con una X-T5 reale: apertura di una cartella RAF, sviluppo di 5 RAF × 3 preset, confronto, valutazione, export in "Prova Dolomiti" e verifica del catalogo, delle stelle e dei tag in Libreria. Annotare i tempi.

## Workflow follow-up

- PR verso `main` dopo il merge di `github-issues-39-47`, merge dopo la revisione, poi archiviazione della change.
