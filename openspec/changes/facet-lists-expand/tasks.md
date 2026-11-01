# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Conflitti con `manual-edits-open-with-ai-tuning`** (branch `feat/user-feedback-tuning`): quel change aggiunge "Rinomina…" sul filtro Luogo e l'albero delle cartelle nella stessa `Sidebar.tsx`. Prima di iniziare controllare se è già unito in `main` e, se sì, fare il rebase. In ogni caso `FacetList` e `FacetPanel` devono accettare un'azione per riga opzionale, così la rinomina del luogo funziona anche dal pannello flottante; chi arriva per secondo collega le due cose.
> - **Nessuna migrazione di schema** e nessuna nuova dipendenza: il pannello flottante si fa con Pointer Events, l'elenco con `@tanstack/react-virtual` già presente.
> - **Finestra nativa del sistema operativo fuori ambito**: la richiesta di finestra "flottante" è coperta dal pannello in-app (design, Decisione 4). Se l'utente chiede esplicitamente di poterla portare su un secondo monitor, fermarsi e proporre un change separato.

## 1. Backend dei facet

- [x] 1.1 Spostare `fold` da `pc-ai/src/geo.rs` a `pc-core` (estendendo la rimozione dei segni diacritici almeno a maiuscole accentate, `ç`, `ñ`) e farla usare da `geo.rs`. Verifica: i test esistenti di `geo.rs` passano e un nuovo unit test in `pc-core` copre "Città" → "citta" e "ÉLITE" → "elite".
- [ ] 1.2 Calcolare in `query::facets` ogni lista con il filtro privato della propria selezione (design, Decisione 1), riusando il CTE di base per le liste senza selezione. Verifica: test in `crates/pc-store/tests/query.rs` per gli scenari "Selezione di più luoghi" e "Altri filtri applicati", e per un gruppo di catalogazione selezionato che non riduce gli altri valori dello stesso gruppo.
- [ ] 1.3 Estendere `Facets` con `FacetList { values, total }`, al massimo 50 valori per lista più le voci selezionate non comprese, e rimuovere i `LIMIT 100`. Verifica: test con 340 luoghi che controlla `total = 340`, 50 valori restituiti e la presenza di un luogo selezionato con poche foto.
- [ ] 1.4 Aggiungere `query::facet_values(filter, FacetKey, search, sort, offset, limit) -> FacetPage` con ricerca tramite `fold` e ordinamento per conteggio o nome (design, Decisione 2), e il comando IPC `library_facet_values` in `app/src-tauri/src/library.rs`; rigenerare `bindings.ts`. Verifica: test per "Ricerca nell'elenco" ("sie" trova "Siena" e "SIENA"), "Ordinamento alfabetico", paginazione senza duplicati né buchi, e un test di prestazioni accanto a quelli esistenti in `tests/query.rs` (100.000 foto, 5.000 tag: prima pagina entro 1 s in release).

## 2. Lista compatta nella barra laterale

- [ ] 2.1 Sostituire `CheckList` con `FacetList` in `Sidebar.tsx`: prime 8 voci più quelle selezionate, "Mostra tutti (N)" con espansione sul posto, ricerca con debounce, elenco virtualizzato a pagine e "Mostra meno"; azione per riga opzionale; chiavi i18n it/en sotto `library.facets.*`. Verifica: test in `LibraryPage.test.tsx` con `ipcMock` per gli scenari "Molte località", "Voce selezionata sempre visibile" e per la ricerca nell'elenco espanso; `npm --prefix app/ui test` passa.
- [ ] 2.2 Usare il nuovo `total` per mostrare "Mostra tutti" solo quando servono e adeguare `ipcMock.ts` alla nuova forma di `Facets`. Verifica: i test esistenti della libreria passano senza modifiche al loro significato.

## 3. Pannello flottante

- [ ] 3.1 Creare `components/floating/FloatingPanel.tsx` e lo store persistito `store/floating.ts` (posizione, dimensione, aperto, ordine di sovrapposizione), con trascinamento, ridimensionamento, vincolo dentro la finestra, Esc e restituzione del focus (design, Decisione 4). Verifica: test in `components/floating/FloatingPanel.test.tsx` per trascinamento con Pointer Events simulati, vincolo dopo un resize della finestra, primo piano al clic, Esc con ritorno del focus e persistenza dopo un nuovo montaggio.
- [ ] 3.2 Creare `FacetPanel` (ricerca, ordinamento, numero di voci, elenco virtualizzato con caselle, "Deseleziona tutto") collegato a `useLibrary`, e il comando "Apri in finestra" su ogni lista della barra laterale; i pannelli si nascondono fuori dal modulo Libreria. Verifica: test con `ipcMock` per gli scenari "Elenco luoghi in un pannello flottante", "Aggiornamento col filtro" e "Più pannelli"; selezionare una voce nel pannello aggiorna griglia e barra laterale.
- [ ] 3.3 Per le catalogazioni AI, ordinamento alfabetico sull'etichetta tradotta caricando tutte le voci del gruppo in una pagina. Verifica: test che in inglese "Animal" precede "Art work" nell'ordine alfabetico del gruppo "Tipo soggetto".

## 4. Verifica d'insieme

- [ ] 4.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde.
- [ ] 4.2 Prova sull'app reale con una libreria con centinaia di luoghi: espansione sul posto, ricerca, pannello flottante dei luoghi e dei tag aperti insieme mentre si selezionano foto, selezione di più luoghi, riapertura dopo un riavvio nella stessa posizione. Verifica: tutti i passi riusciti, nessun errore nuovo in Diagnostica.
- [ ] 4.3 Allineare le delta di `specs/` a quanto implementato e verificare che `openspec validate facet-lists-expand --strict` passi.

## Workflow follow-up

- Coordinare il merge con `manual-edits-open-with-ai-tuning` (`Sidebar.tsx`).
- Archiviare il change dopo la revisione.
