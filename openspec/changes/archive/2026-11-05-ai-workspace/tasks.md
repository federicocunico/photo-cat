# Tasks

> Esecuzione: ogni task numerato va a un subagente **Sonnet**. I task "Revisione Opus" chiudono una macroparte e vanno a un subagente **Opus**. Prerequisito: change `perceptual-hash-similarity` applicata (fase "impronte").

## 1. Stato per fase e filtri (backend)

- [x] 1.1 Verificare come vengono marcate le fasi volti e persone (`save_faces_tuned`, `media_people.model_ver`). Se la fase volti non ha un marcatore per foto, aggiungerlo nella migrazione v12 (D1). Test di migrazione.
- [x] 1.2 Modulo `pc-store::coverage` (D1): `Stage`, `StageKeys`, predicati SQL corrente / da aggiornare / mai analizzata per fase, `coverage(filter, keys, active_jobs)` con il conteggio "in coda". Test su libreria sintetica che coprono:
  - lo scenario 6.500 / 500 / 0 / 3.000;
  - la foto analizzata senza etichette;
  - un job attivo con target `ids`.
- [x] 1.3 Riscrivere `AnalysisState` della griglia (`query.rs`) sulla stessa definizione per la classificazione. I test esistenti di `analysis_state_follows_processing_not_labels` restano verdi.
- [x] 1.4 `LibraryFilter.analysis` e `LibraryFilter.label_score` (D2) in `compile_filter`, con binding specta e chip dei filtri attivi in UI. Test Rust su pagina, conteggio e "Seleziona tutto"; test vitest dei chip.
- [x] 1.5 Test di prestazioni `#[ignore]`: `coverage` su 100.000 foto < 300 ms in release.

## 2. Rianalisi e rifiuto delle etichette (backend)

- [x] 2.1 Job `analysis` con `{scope, stages, force}` (D3), compatibile con `{ids}`/`{sourceId}`. Il catalogo viene risolto all'inizio del job; `force` azzera i marcatori delle fasi scelte. Test:
  - due cataloghi, solo mancanti;
  - forza su 40 id per persone e volti, che non tocca le altre fasi;
  - soggetto manuale preservato con forza.
- [x] 2.2 Comandi Tauri `ai_overview`, `ai_reprocess_preview`, `ai_reprocess` (con `Target` o catalogIds o libreria), registrati in `lib.rs` e nei binding. Test dei comandi dove esistono test dei comandi.
- [x] 2.3 Schema v12 `label_rejection` (D5): `classification` e `relabel` scartano i rifiutati, l'assegnazione manuale toglie il rifiuto. Test dei tre scenari di "Etichette automatiche rifiutate".
- [x] 2.4 Azione bulk annullabile `reject_auto_labels {grp, value, max_score?, target}` con anteprima del conteggio, comandi `ai_reject_preview`/`ai_reject_labels`. Test: rimozione sotto soglia in un catalogo; annullamento ripristina etichette e rifiuti.
- [x] 2.5 Aggregati `pc-store::insights` (D4): istogramma delle confidenze, copertura per mese, statistiche dei gruppi con filtro `similar_size`. Comandi Tauri relativi. Test con dati sintetici; test `#[ignore]` < 300 ms su 100.000 foto.
- [x] 2.6 Revisione Opus delle parti 1–2: coerenza delle definizioni di stato, garanzie sulle correzioni manuali con `force`, annullamento, costo delle query.

## 3. Modulo AI (interfaccia)

- [x] 3.1 Navigazione (D8):
  - gruppo e modulo `ai` in `registry.ts`, `ModuleId`/`known`/`Destination` in `layout.ts`, `ReadyApp.tsx`;
  - icona, i18n it/en (`common.ts`, `it.ts`, `en.ts`, `screens.ts`), "Ctrl+1…8" in `commands/catalog.ts`.

  Test: `shell.test.tsx` aggiornato con i gruppi; Ctrl+6 apre AI e Ctrl+3 apre Cataloghi.
- [x] 3.2 Componenti grafici SVG (D6) in `components/charts/`: `BarChart`, `Histogram` con selezione di intervallo e linea di soglia, `StackedBar`, `ColumnSeries`. Seguire la skill `dataviz`. Test vitest: rendering, tooltip, clic e selezione da tastiera, stato vuoto.
- [x] 3.3 Sezione Panoramica: copertura per fase con `StackedBar`, job AI attivi, modelli e memoria dallo store `live`, aggiornamento live (D7). I segmenti aprono la Libreria con `filter.analysis`. Test vitest: conteggi visualizzati, invalidazione su `JobSnapshot` con throttling, navigazione verso la Libreria.
- [x] 3.4 Sezione Rianalisi: scelta dell'ambito (libreria, cataloghi multipli, selezione corrente, filtro corrente), fasi, modalità, anteprima per fase, avvio. L'azione "Rianalizza" della barra di selezione apre questa sezione con l'ambito precompilato. Test vitest: ambito vuoto disattiva l'avvio; i parametri inviati corrispondono alle scelte.
- [x] 3.5 Sezione Risultati: ambito, distribuzione per gruppo (clic → Libreria filtrata), istogramma della voce con soglia (intervallo → "Mostra in Libreria" o "Pulisci…"), copertura mensile. Stato vuoto con collegamento a Rianalisi. Test vitest.
- [x] 3.6 Sezione Pulizia: voce, ambito, confidenza massima opzionale, anteprima, conferma, avviso con "Annulla operazione". Test vitest.
- [x] 3.7 Sezione Simili: statistiche, distribuzione delle dimensioni (clic → Libreria raggruppata con `similar_size`), avvio dell'analisi in attesa. Test vitest.
- [x] 3.8 Palette dei comandi: "AI: Panoramica", "AI: Rianalisi…", "AI: Pulisci etichette…". Test.
- [x] 3.9 Revisione Opus della parte 3: aderenza alle spec, coerenza visiva e accessibilità dei grafici, flussi Panoramica → Libreria → ritorno.

## 4. Verifica

- [x] 4.1 `cargo test` del workspace, typecheck, lint e test UI, poi `make test-linux` con i nuovi file in stage. Correggere ogni errore.
- [ ] 4.2 Prova manuale nell'app (skill `run`) su una libreria reale. Verificare: **Non eseguita**: la prova visiva è stata saltata su indicazione dell'utente per la release 0.4.0. Coperta da test UI completi, CI Linux e prova del backend nell'app su un catalogo reale da 5.000 foto.
  - Panoramica live durante un'analisi;
  - rianalisi di un catalogo;
  - pulizia e annullamento;
  - tempi di apertura entro 1 s.

## Workflow follow-up

- Archiviare la change dopo la revisione; applicare poi `photo-map`.
