# Tasks

## 1. Ancora della selezione con Shift+clic

- [x] 1.1 Test (rossi) per `Grid.tsx`:
  - clic su 2, Shift+clic su 5, Shift+clic su 8 → 2…8;
  - poi Shift+clic su 4 → 2…4;
  - clic su 2, Ctrl+clic su 6, Ctrl+Shift+clic su 9 → {2, 6…9};
  - Maiusc+frecce dopo un clic usano la stessa ancora.
- [x] 1.2 Aggiungere `anchor` a `useLibrary`:
  - impostata da clic semplice e Ctrl+clic, letta da Shift+clic e dalla tastiera;
  - rimuovere `rangeAnchor` locale in `Grid.tsx`;
  - azzerare l'ancora quando cambia il filtro o la selezione viene svuotata.
- [x] 1.3 `FaceGrid.tsx`: aggiornare `anchor` solo senza Shift, con test "primo, Shift terzo, Shift sesto → 1…6".

## 2. Backend: Cestino che toglie dai cataloghi

- [x] 2.1 Test in `crates/pc-store` per `move_to_trash(c, ids)`:
  - toglie le foto dai cataloghi manuali non di sistema e le mette nel Cestino;
  - lascia stare i cataloghi smart;
  - l'annullamento rimette appartenenze e Cestino com'erano.
- [x] 2.2 Implementare `bulk::move_to_trash` in `crates/pc-store/src/bulk.rs`, con un'unica voce `UndoAction::Group { CatalogAdded{trash}, CatalogRemoved… }`.
- [x] 2.3 Test per `trash_impact` (conteggi per catalogo manuale, sia per `ids` sia per `filter` con `except`), poi implementarlo con una query aggregata.
- [x] 2.4 Aggiungere `BulkAction::MoveToTrash` in `crates/pc-jobs/src/bulk.rs`:
  - verificare che "Annulla operazione" annulli l'intera operazione a blocchi, come per `add_catalog`;
  - test del job.
- [x] 2.5 Comando Tauri `trash_impact(target)` in `app/src-tauri/src/catalogs.rs`, registrato in `lib.rs`; rigenerare `app/ui/src/bindings.ts` (`cargo test -p photo-cat --test bindings`).

## 3. UI: "Sposta nel cestino" con conferma

- [x] 3.1 Helper condiviso `moveToTrash(target, count)`:
  - chiama `trashImpact`;
  - senza cataloghi avvia subito `move_to_trash`, altrimenti `ConfirmDialog` con l'elenco "Mare — 3 foto";
  - nel dialogo il focus iniziale è sul pulsante di conferma, così Invio conferma: nuova opzione di `ConfirmDialog` (es. `focusConfirm`), mentre il comportamento predefinito resta il focus su "Annulla";
  - avviso con "Annulla operazione".
- [x] 3.2 Usare l'helper nella barra delle azioni (`SelectionBar.tsx`) e per il tasto Canc (`Grid.tsx`) al posto di `add_catalog` sul Cestino; testi it/en.
- [x] 3.3 Test:
  - nessun dialogo senza cataloghi;
  - dialogo con l'elenco;
  - "Annulla" del dialogo non avvia nulla;
  - Invio nel dialogo conferma;
  - Canc passa dallo stesso percorso.

## 4. Trascinamento sui cataloghi

- [x] 4.1 Modulo `app/ui/src/components/dnd/photoDrag.ts`:
  - negozio del trascinamento (payload `Target` + conteggio, posizione, bersaglio);
  - `DragGhost` ("N foto");
  - hook `useDropTarget` con attributo `data-drop-catalog`;
  - Esc annulla.
- [x] 4.2 `Grid.tsx`, avvio dal `pointerdown` su una miniatura dopo 5 px:
  - payload = selezione se la miniatura è selezionata, altrimenti solo quella miniatura, senza cambiare la selezione;
  - ignorare il `click` che segue un trascinamento;
  - il rettangolo resta invariato.
- [x] 4.3 `Sidebar.tsx`, bersagli di rilascio:
  - cataloghi manuali → `add_catalog` con avviso e annullamento;
  - Cestino → helper `moveToTrash`;
  - smart e altri cataloghi di sistema non accettano (cursore `not-allowed`, nessuna evidenziazione).
- [x] 4.4 Test (eventi `pointer*` in jsdom):
  - selezione intera;
  - miniatura non selezionata;
  - selezione per filtro;
  - catalogo smart rifiutato;
  - Esc;
  - clic sotto i 5 px resta un clic.

## 5. Barra laterale orientata ai cataloghi

- [x] 5.1 Ordine delle sezioni in `Sidebar.tsx`: Cataloghi, Filtri, Cartelle.
  - Nei cataloghi: quelli dell'utente in ordine alfabetico, "Nuovo catalogo", separatore, quelli di sistema.
- [x] 5.2 "Nuovo catalogo":
  - campo del nome sul posto (Invio crea, Esc annulla, errore per nome duplicato accanto al campo);
  - come bersaglio di rilascio crea il catalogo e aggiunge le foto trascinate.
- [x] 5.3 Cartelle chiuse per impostazione predefinita (`collapsedSections` predefinito `["library.sources"]` in `store/layout.ts`) e "Gestisci cartelle" che apre Importa.
- [x] 5.4 Test di ordine, creazione sul posto, rilascio su "Nuovo catalogo", stato predefinito.

## 6. Menu contestuale "Aggiungi a catalogo ▸"

- [x] 6.1 Sottomenu in `GridContextMenu.tsx` sullo schema di "Apri con":
  - `SearchPicker` con i cataloghi manuali non di sistema in ordine alfabetico, più "Nuovo catalogo…" (dialogo con il nome);
  - agisce su `menu.target`.
- [x] 6.2 Test:
  - aggiunta di 20 foto selezionate;
  - ricerca;
  - smart e di sistema esclusi;
  - "Nuovo catalogo…" crea e aggiunge;
  - navigazione da tastiera.

## 7. Verifica

- [x] 7.1 Test UI completi (`npx vitest run`), `tsc --noEmit`, eslint e prettier sui file toccati.
- [x] 7.2 `cargo test` dei crate toccati, `cargo clippy --all-targets`, `cargo fmt --check`.
- [x] 7.3 Verifica manuale nel browser con dati simulati: Shift+clic ripetuti, trascinamento su catalogo, Cestino, "Nuovo catalogo", menu contestuale, ordine della barra laterale.
- [x] 7.4 Passi CI Linux: `make test-linux` (Docker) dopo aver messo in stage i file nuovi; correggere ogni errore.
