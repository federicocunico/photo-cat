# Proposal

## Why

Un utente con molte località segnala che le liste dei filtri della libreria (luoghi, tag, catalogazioni AI, anni, formati) mostrano solo una parte delle voci: per trovare un luogo deve prima restringere la selezione. Oggi la barra laterale mostra al massimo 12 valori per filtro e il backend restituisce al massimo 100 luoghi, tag o persone; le voci oltre questi limiti non sono raggiungibili, nemmeno se già selezionate.

## What Changes

- Ogni lista di filtro nella barra laterale mostra le voci principali e un comando **"Mostra tutti (N)"** che la espande sul posto, con un campo di ricerca, e "Mostra meno" per richiuderla.
- Ogni lista ha un comando **"Apri in finestra"** che la mostra in un **pannello flottante** non modale: trascinabile, ridimensionabile, con ricerca, ordinamento (per numero di foto o alfabetico), elenco completo scorrevole e selezione multipla. Il pannello resta aperto mentre si lavora sulla griglia e si aggiorna quando cambia il filtro; se ne possono aprire più di uno (es. Luoghi e Tag), e posizione e dimensione vengono ricordate.
- Le voci selezionate restano **sempre visibili** nella lista compatta, anche se non sono tra le principali.
- Nessun limite fisso al numero di valori raggiungibili: il backend fornisce il totale delle voci per ogni filtro e un elenco completo paginato e ricercabile.
- I conteggi di una lista vengono calcolati **ignorando la selezione della lista stessa** (mantenendo gli altri filtri). Così, dopo aver scelto un luogo, gli altri luoghi restano visibili e selezionabili invece di sparire. Si applica a luoghi, tag, catalogazioni AI per gruppo e formati; per gli anni, che sono una scelta singola, il comportamento è lo stesso.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `catalog-library`: nuovi requisiti "Elenco completo dei valori dei filtri" e "Conteggi dei filtri indipendenti dalla propria selezione"; "Ricerca e filtri" resta invariato.
- `desktop-shell`: nuovo requisito "Pannelli flottanti" (comportamento generale delle finestre flottanti nell'app).

## Impact

- **Codice Rust**: `pc-store/src/query.rs` (calcolo dei facet escludendo la propria selezione, totale per facet, rimozione dei `LIMIT 100`, nuova funzione di elenco paginato e ricercabile dei valori di un facet), `app/src-tauri/src/library.rs` (nuovo comando IPC), `bindings.ts` rigenerato.
- **UI**: `app/ui/src/modules/library/Sidebar.tsx` (`CheckList` con espansione, ricerca e voci selezionate sempre visibili), nuovo componente generico di pannello flottante in `app/ui/src/components/`, nuovo pannello dei valori di un filtro, stato dei pannelli aperti in `store/layout.ts`, i18n it/en.
- **Dipendenze**: nessuna nuova (`@tanstack/react-virtual` è già presente per la griglia).
- **Dati**: nessuna migrazione.
- **Coordinamento**: il branch `feat/user-feedback-tuning` (change `manual-edits-open-with-ai-tuning`) aggiunge "Rinomina…" sul filtro Luogo e l'albero cartelle nella stessa `Sidebar.tsx`. La rinomina va resa disponibile anche dalle righe del pannello flottante: chi implementa per secondo integra l'altra.
