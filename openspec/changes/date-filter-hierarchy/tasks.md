# Tasks

## 1. Backend: mesi e giorni nel riepilogo dei filtri

- [x] 1.1 In `crates/pc-store/src/query.rs` aggiungere la funzione che ricava dal filtro il percorso della data scelta (anno, mese `AAAA-MM`, giorno `AAAA-MM-GG`) secondo design §1, con test unitari per anno intero, mese intero (anche febbraio bisestile), giorno e intervallo non riconosciuto; verificare con `cargo test -p pc-store`
- [x] 1.2 Aggiungere a `Facets` i campi `months: Option<FacetList>` e `days: Option<FacetList>`, calcolati con il filtro senza la scelta della data più l'intervallo dell'anno o del mese scelto, in ordine cronologico, con la voce scelta presente anche a 0 foto; aggiornare `FacetKey::Year::selected` per usare il percorso; verificare con nuovi test in `crates/pc-store/tests/facets.rs` (mesi e giorni con conteggi, altri mesi visibili con un altro filtro attivo, ora locale con fuso +02:00 che sposta la foto al 1° giugno, foto senza data escluse, nessun mese senza anno scelto)
- [x] 1.3 Rigenerare `app/ui/src/bindings.ts` con `cargo test -p photo-cat --test bindings` e verificare che `npm run typecheck` (in `app/ui`) passi

## 2. UI: lista gerarchica delle date

- [x] 2.1 In `app/ui/src/modules/library/facets.ts` aggiungere `datePath` (percorso scelto dall'intervallo) e `toggleDate` (anno/mese/giorno secondo design §3) e aggiornare `selectedOf`/`toggleIn`/`clearIn` per la lista Anno; verificare con un nuovo test vitest `facets.test.ts` su tutte le transizioni (scelta, risalita, cambio di anno e di mese)
- [x] 2.2 Creare `DateFacet.tsx` (anni come oggi, mesi rientrati sotto l'anno scelto, giorni sotto il mese scelto, nomi con `Intl.DateTimeFormat` nella lingua dell'interfaccia) e usarlo in `Sidebar.tsx` al posto di `FacetList` per gli anni; verificare con un test in `LibraryPage.test.tsx` che scegliendo anno e mese il filtro inviato a `libraryQuery` contenga l'intervallo del mese e che compaiano i giorni
- [x] 2.3 Aggiornare `chips.ts` per mostrare il livello scelto ("2023", "maggio 2023", "14 maggio 2023") nella lingua dell'interfaccia e aggiungere le stringhe i18n it/en necessarie; verificare con un test vitest sui chip in italiano e inglese
- [x] 2.4 Aggiungere al README, nella sezione della Libreria, una riga sul filtro per data anno → mese → giorno e verificare che il testo corrisponda al comportamento

## 3. Verifica

- [x] 3.1 Eseguire `cargo test` (workspace), `npm test` in `app/ui` e `make test-linux` (Docker) e correggere ogni errore
- [x] 3.2 Verificare sull'app reale (tauri dev con cartella dati di prova) che scegliendo un anno, un mese e un giorno la griglia e i conteggi siano corretti e che il chip mostri la data nella lingua scelta

## Workflow follow-up

- Aprire la PR che cita la issue #12, fare il merge su main dopo la CI.
- Archiviare la change e sincronizzare la spec `catalog-library`.
