# Tasks

> Esecuzione: ogni task numerato va a un subagente **Sonnet**. I task "Revisione Opus" chiudono una macroparte e vanno a un subagente **Opus**. Le issue vanno citate (`#NN`) nei commit e nella PR senza chiuderle.

## 1. Schema e dati di libreria

- [x] 1.1 Migrazione v14 (D2, D3, D6, D7):
  - `media_item.rating` e `color_trait`, con indici;
  - `media_dates.manual_at` e `manual_precision`, con `DateSource::Manual`;
  - `media_item.missing_since`, valorizzato all'ora della migrazione per i mancanti esistenti;
  - `error_log.archived_at`, con indice parziale.

  Test di migrazione: le righe esistenti sono conservate e i default sono neutri.
- [x] 1.2 Bulk e annulla per valutazione e colore (`UndoAction::{SetRating, SetColorTrait}`), comandi Tauri e binding. Test Rust:
  - valori fuori intervallo rifiutati;
  - l'annulla ripristina il valore per id;
  - 100.000 id sopra la soglia passano come job.
- [x] 1.3 Data manuale (D3): `set_manual_date(ids, value, precision)`, `clear_manual_date(ids)`, ricalcolo di `instant_utc` che ignora `shift_s` e annulla. Test Rust per gli scenari di `capture-timeline`:
  - "giugno 1984";
  - solo anno ordinato al 1° gennaio;
  - ripristino;
  - uscita dalle foto senza data;
  - scostamento di gruppo che non tocca la data manuale.
- [x] 1.4 Filtri e regole smart (`LibraryFilter.rating`, `color_trait`) in `compile_filter`, facet con conteggi, nodo "Anno senza mese" nel filtro per data. Test Rust su "almeno 4", "Non indicato", combinazione con un altro filtro e catalogo smart aggiornato dopo un bulk.
- [x] 1.5 Revisione Opus della parte 1: correttezza dell'istante reale con date manuali, costo delle query sui nuovi indici, completezza dell'annulla.

## 2. Valutazione, colore e data nell'interfaccia (#41, #43)

- [x] 2.1 Inspector: stelle cliccabili, selettore "Colore", editor "Data di scatto" con precisione giorno/mese/anno e "Ripristina data del file", con la fonte mostrata. Test vitest delle tre modifiche e del ripristino.
- [x] 2.2 Menu contestuale della griglia: "Valutazione ▸", "Colore ▸" (con "Non indicato") e "Imposta data di scatto…" sulla selezione. Alt+1–5 e Alt+0 in Libreria e Sequenza, senza toccare i tasti 1–9 dei tag. Stelle sulle miniature. Test vitest:
  - Alt+5 su 3 foto;
  - il tasto 2 assegna ancora il tag;
  - avviso con "Annulla".
- [x] 2.3 Facet "Valutazione" e "Colore", chip dei filtri e campi "Valutazione"/"Colore" nell'editor delle regole, con frase di riepilogo. i18n it/en e glossario. Test vitest dei chip e dell'editor.

## 3. Selezione ovunque (#39)

- [x] 3.1 Estrarre `useGridSelection` (D1) da `useLibrary`, portarvi la Libreria e i cataloghi senza cambiarne il comportamento. I test esistenti della selezione devono restare verdi.
- [x] 3.2 Comandi `faceSelection` e `sequenceSelection` con snapshot e test Rust (400 volti con 120 caricati).
- [x] 3.3 Collegare FaceGrid, Sequenza, catalogo di sistema, escluse e sezioni AI. Voci "Seleziona tutto"/"Inverti selezione" in barra e menu contestuale. Scorciatoie Ctrl/Cmd+A, Ctrl+Maiusc+I ed Esc registrate nella shell e ignorate nei campi di testo. Test vitest per ogni vista e per il campo di ricerca.
- [x] 3.4 Revisione Opus della parte 3: coerenza della selezione tra viste, prestazioni con 100.000 elementi, accessibilità delle scorciatoie.

## 4. Persone: dal volto alla foto e ordinamento (#40, #42, #47)

- [x] 4.1 Menu contestuale di FaceGrid (D5) con "Apri la foto", "Mostra in Libreria", "Apri con ▸" e "Mostra nella cartella", con media deduplicati. Doppio clic e Invio aprono il visore con il volto evidenziato. Test vitest:
  - 4 volti di 3 foto aprono 3 file;
  - foto mancante mostra un avviso;
  - il ritorno alla stessa posizione della griglia.
- [x] 4.2 `FaceSort::CapturedAsc`, query `group_face_years` e ordinamento per `instant_utc`, con i volti senza data in fondo (D4). Test Rust: cursore stabile a pagine, date manuali rispettate, conteggi per anno.
- [x] 4.3 Selettore di ordinamento nella griglia, intestazioni per anno con conteggi, "Senza data" e preferenza salvata in `ui_prefs`. Test vitest: scelta ricordata e selezione conservata al cambio di ordinamento.

## 5. Import, errori e monitoraggio (#44, #45, #46)

- [x] 5.1 Import: `missing_since` valorizzato quando un file diventa mancante, `import.ack.<source>` e comando `acknowledgeSourceWarnings` (D6). Test Rust: "5 mancanti" ignorati, poi 2 nuovi mancanti contano 2; il riepilogo sostituito non porta errori vecchi.
- [x] 5.2 Pagina Import: avvisi limitati all'ultimo aggiornamento, pulsante "Ignora", menu Ricollega/Rimuovi finché ci sono mancanti. Test vitest degli scenari di `media-ingestion`.
- [x] 5.3 Errori archiviati (D7): `archive_errors` e `unarchive_errors`, `ErrorQuery.archived`, `errorCount` che conta solo le voci non archiviate. In Diagnostica: azioni "Archivia" su voce, gruppo e filtro, filtro "Archiviati" e "Ripristina". Test Rust e vitest: indicatore globale a zero, nuovo errore conteggiato, log intatto.
- [x] 5.4 Monitoraggio (D8): etichetta "Aggiorna automaticamente", tooltip, voce della guida, stato per sorgente (attivo · ultimo alle HH:MM / non raggiungibile) e `last_auto_update_at`. Test vitest del tooltip e degli stati; test Rust dello stato "non raggiungibile".
- [x] 5.5 Revisione Opus delle parti 2, 4 e 5: aderenza alle spec delle issue e testi in italiano chiari per un utente non tecnico.

## 6. Verifica

- [x] 6.1 `cargo test` del workspace, typecheck, lint e test UI, poi `make test-linux` con i nuovi file in stage. Correggere ogni errore.
- [ ] 6.2 Prova manuale nell'app (skill `run`) su una libreria con scansioni:
  - data "solo anno";
  - Persone ordinate per anno;
  - "Ignora" sui mancanti;
  - archiviazione degli errori;
  - Ctrl+A in Persone e in Sequenza.

## Workflow follow-up

- PR verso `main` che chiude #39–#47, merge dopo la revisione, poi archiviazione della change.
