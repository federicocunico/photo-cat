# Tasks

> Esecuzione: ogni task numerato va a un subagente **Sonnet**. I task "Revisione Opus" chiudono una macroparte e vanno a un subagente **Opus**. Prerequisito: change `ai-workspace` applicata.

## 1. Dati geografici (backend)

- [x] 1.1 Migrazione: `place_geo` e indice `location_geo(lat, lon)` (D2, D3). Test di migrazione.
- [x] 1.2 Job `place_geo` (D2): geocodifica dei nomi `directory`/`manual` con `Gazetteer::by_name`. Accodamento all'avvio (marcatore), dopo gli import e dopo una rinomina dei luoghi. Senza gazetteer non fa nulla. Test:
  - "Lisbona" → coordinate;
  - nome sconosciuto → nessuna riga;
  - la posizione manuale non viene sovrascritta.
- [x] 1.3 `map_cells`, `map_catalogs`, `map_places_unplaced`, `map_place_set`/`map_place_clear` in `pc-store` (D3) e comandi Tauri con binding. Test:
  - la somma delle celle è invariata tra due zoom;
  - le foto GPS ignorano `place_geo`;
  - le approssimate si escludono con `approx=false`;
  - conteggio dei cataloghi nell'area.
- [x] 1.4 `LibraryFilter.geo_bbox` (D4) in `compile_filter` e chip "Area: <luogo>". Test Rust (pagina e conteggio coerenti con la cella) e vitest (chip rimovibile).
- [x] 1.5 Test di prestazioni `#[ignore]` con 100.000 foto con posizione: `map_cells` < 150 ms a zoom mondiale e regionale in release.
- [x] 1.6 Revisione Opus della parte 1: correttezza della proiezione e delle celle, precedenza GPS / approssimata, costo delle query.

## 2. Sezione Mappa (interfaccia)

- [x] 2.1 Asset Natural Earth 1:50m semplificato in `app/ui/public/geo/` con nota di licenza (pubblico dominio) e decodificatore TopoJSON minimo. Test vitest del decodificatore su un frammento.
- [x] 2.2 Componente `WorldMap` su canvas (D1): proiezione Web Mercator, pan, zoom (rotellina, pulsanti, tastiera), ridisegno con rAF. Test vitest delle funzioni di proiezione e dei limiti di zoom.
- [x] 2.3 Strato dei gruppi (D3, D5): richieste con debounce e cancellazione, cerchi proporzionali, stile distinto per i gruppi approssimati, tooltip con miniature, clic verso la Libreria con `geo_bbox`, interruttore "Mostra posizioni approssimate". Test vitest con comandi simulati.
- [x] 2.4 Pannello laterale: ambito (libreria o catalogo), cataloghi nell'area visibile (clic limita l'ambito), luoghi senza coordinate con "Posiziona" e "Rimuovi posizione". Test vitest del flusso di posizionamento.
- [x] 2.5 Sezione "Mappa" nel modulo AI con `Destination` e comando di palette "AI: Mappa". i18n it/en. Test vitest della navigazione.
- [x] 2.6 Verifica dell'assenza di richieste di rete: test che nessun `fetch` verso host esterni parta dalla sezione (stub di `fetch` e controllo degli URL degli asset).
- [x] 2.7 Revisione Opus della parte 2: aderenza alle spec, fluidità di pan e zoom, accessibilità.

## 3. Verifica

- [x] 3.1 `cargo test` del workspace, typecheck, lint e test UI, poi `make test-linux` con i nuovi file in stage. Correggere ogni errore.
- [ ] 3.2 Prova manuale nell'app (skill `run`) su una libreria con GPS e cartelle con nomi di città. Annotare i tempi di apertura e aggiornamento. **Non eseguita**: la prova visiva è stata saltata su indicazione dell'utente per la release 0.4.0. Coperta da test UI completi, CI Linux e prova del backend nell'app su un catalogo reale da 5.000 foto.

## Workflow follow-up

- Archiviare la change dopo la revisione.
