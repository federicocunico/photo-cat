# Design

## Context

Stato attuale rilevante (per il perché vedi proposal.md):

- **Selezione in Libreria** (`app/ui/src/modules/library/Grid.tsx`):
  - il clic su una miniatura chiama `clickSelect(order, explicit, focused, id, mods)`, con `focused` del negozio `useLibrary` come ancora;
  - `setSelection(next, id)` sposta `focused` sull'elemento cliccato, quindi dopo uno Shift+clic l'ancora diventa l'ultimo elemento;
  - esiste anche un `rangeAnchor` locale per la tastiera, ma il clic lo sovrascrive sempre.
- **Selezione dei volti**: la griglia di Persone (`modules/people/FaceGrid.tsx`) ha lo stesso schema con `anchor`.
- **Selezione rettangolare**: `pointerdown` sulla griglia solo fuori dalle miniature (`[role="option"]`), con `setPointerCapture`.
- **Trascinamento nell'app**: nessuno verso i cataloghi. Tauri 2 su Windows ha `dragDropEnabled` attivo per impostazione predefinita (non configurato in `tauri.conf.json`). Con quell'impostazione il trascinamento HTML5 dentro la WebView è poco affidabile.
- **Cestino**: è il catalogo di sistema `trash`. "Sposta nel cestino" usa l'azione bulk `add_catalog { catalog: trash }` da `SelectionBar.tsx` e dal tasto Canc in `Grid.tsx`. Le foto restano in `catalog_item` dei cataloghi manuali: le nasconde solo il filtro sui cataloghi di sistema.
- **Annullamento**: il registro (`crates/pc-store/src/bulk.rs`, `record_undo` e `UndoAction`) ha già `CatalogAdded`, `CatalogRemoved` e `Group`. `catalog_snapshot` prepara lo stato precedente.
- **Riepilogo dei cataloghi persi**: per la rimozione di una sorgente esiste già `CatalogLoss { name, items }` (`crates/pc-store/src/sources.rs`).
- **Barra laterale** (`Sidebar.tsx`): sezioni `library.sources`, `library.catalogs`, `library.filters` in quest'ordine. `collapsedSections` è persistito in `store/layout.ts` e vale `[]` per impostazione predefinita.
- **Menu contestuale** (`GridContextMenu.tsx`): ha già un sottomenu ("Apri con") con lo stesso schema.

## Goals / Non-Goals

**Goals:**
- un solo modo di trascinare nella griglia della Libreria (anche con un catalogo aperto), senza dipendere dal drag-and-drop della WebView;
- "Sposta nel cestino" atomico per la selezione intera, compresa la selezione per filtro, con un solo "Annulla operazione";
- nessuna migrazione dei dati.

**Non-Goals:**
- trascinare le foto fuori dall'app (in Esplora risorse) o file dall'esterno nella griglia;
- riordinare i cataloghi o annidarli;
- svuotare il Cestino o eliminare i file dal disco;
- cambiare il modulo Cataloghi (resta la vista di gestione completa).

## Decisions

### 1. Ancora della selezione separata dal focus
- **Decisione**: un'ancora esplicita (`anchor` in `useLibrary`, accanto a `focused`):
  - il clic semplice e il Ctrl+clic la impostano sull'elemento;
  - Shift+clic e Ctrl+Shift+clic la leggono senza modificarla;
  - la tastiera (Maiusc+frecce) usa la stessa ancora, al posto del `rangeAnchor` locale.
- In Persone: `FaceGrid` aggiorna `anchor` solo se non è premuto Shift.
- **Alternativa scartata**: continuare a usare `focused` come ancora e ripristinarla dopo lo Shift+clic. Avrebbe ancora due fonti di verità per la tastiera e per il mouse.
- **Griglia a finestra**: se l'ancora non è tra gli elementi caricati, Shift+clic resta un clic semplice (come oggi). L'ancora è sempre un elemento visto di recente, quindi il caso è raro.

### 2. Trascinamento con eventi del puntatore, non HTML5
- **Decisione**: un piccolo modulo condiviso `components/dnd/photoDrag.ts`:
  - un negozio zustand: `payload`, posizione, bersaglio sotto il puntatore;
  - un componente `DragGhost`: etichetta "N foto" vicino al puntatore;
  - un hook `useDropTarget(accepts, onDrop)` per le righe della barra laterale.
- **Avvio**:
  - parte da `pointerdown` su una miniatura dopo uno spostamento di 5 px, con `setPointerCapture`;
  - il bersaglio si trova con `document.elementFromPoint` e l'attributo `data-drop-catalog`;
  - Esc annulla;
  - dopo un trascinamento il `click` successivo viene ignorato.
- **Payload**:
  - un `Target` (`ids` o `filter` come `selTarget`) con il conteggio;
  - selezione se la miniatura è selezionata, altrimenti `{ kind: "ids", ids: [id] }`, senza cambiare la selezione.
- **Perché**:
  - funziona allo stesso modo su Windows (anche con `dragDropEnabled`), macOS e Linux;
  - non serve toccare `tauri.conf.json`: la conferma per il drop di cartelle in Importa resta invariata;
  - si controlla l'aspetto ed è testabile in jsdom con eventi `pointer*`.
- **Alternativa scartata**: HTML5 DnD con `dragDropEnabled: false`.
  - Disattiverebbe il drop nativo di cartelle usato da Importa.
  - L'anteprima di trascinamento di WebView2 non si personalizza bene.
- **Convivenza col rettangolo**: il rettangolo parte solo fuori dalle miniature, il trascinamento solo sopra. Restano indipendenti.

### 3. Bersagli di rilascio
- **Righe dei cataloghi manuali** dell'utente: azione bulk `add_catalog` esistente, con avviso e annullamento (`bulk.start`).
- **Riga "Cestino"**: lo stesso percorso di "Sposta nel cestino" (decisione 4), compresa la conferma.
- **Cataloghi smart, "Duplicati" e "Qualità insufficiente"**: non accettano. Nessuna evidenziazione e cursore `not-allowed` sulla riga.
- **Riga "Nuovo catalogo"**:
  - accetta il rilascio e apre il campo del nome sul posto, ricordando il payload;
  - Invio crea il catalogo con `save_catalog`, poi `add_catalog`;
  - Esc scarta;
  - nome duplicato: errore `save_catalog` mostrato accanto al campo.

### 4. "Sposta nel cestino" che toglie dai cataloghi
- **Nuova azione bulk `MoveToTrash`** (`crates/pc-jobs/src/bulk.rs`), che per ogni blocco di id:
  1. legge le appartenenze ai cataloghi manuali non di sistema;
  2. le cancella;
  3. aggiunge al Cestino;
  4. registra un'unica voce `UndoAction::Group { CatalogAdded{trash}, CatalogRemoved{catalog, ids}… }`.
- L'annullamento esistente di `Group` ripristina tutto, senza nuovi tipi di annullamento.
  - Da verificare in implementazione: come il job a blocchi raggruppa le voci (oggi `add_catalog` registra una voce per blocco). `MoveToTrash` segue lo stesso schema, così "Annulla operazione" annulla l'intera operazione come per le altre azioni bulk.
- **Nuovo comando di sola lettura** `trash_impact(target) -> Vec<CatalogLoss>`:
  - conta per catalogo manuale non di sistema le foto del `Target` che vi appartengono;
  - risolve `filter` e `except` come le azioni bulk;
  - una query aggregata con `GROUP BY catalog_id`, senza caricare gli id nell'interfaccia.
- **Interfaccia** (barra delle azioni, tasto Canc, rilascio sul Cestino), un unico helper `moveToTrash(target, count)`:
  - chiama `trash_impact`;
  - se l'elenco è vuoto avvia subito il job, altrimenti apre `ConfirmDialog` con l'elenco "Mare — 3 foto";
  - avvia `start_bulk(move_to_trash)` con avviso e annullamento.
- **Alternativa scartata**: togliere le appartenenze dentro `add_catalog` quando il catalogo è il Cestino. Nasconde un effetto distruttivo in un'azione che altrove è additiva, e cambierebbe il comportamento dei chiamanti esistenti.
- **"Ripristina" dal Cestino**: non cambia (`restore_from_system`). Le appartenenze non ci sono più, quindi le foto tornano senza cataloghi, come richiesto dallo spec.

### 5. Barra laterale orientata ai cataloghi
- **Ordine**: `library.catalogs`, `library.filters`, `library.sources` (rinominata "Cartelle" nella UI se non lo è già).
- **Dentro la sezione cataloghi**:
  - cataloghi dell'utente in ordine alfabetico (manuali e smart, smart con la sua icona);
  - riga "Nuovo catalogo";
  - separatore, poi i cataloghi di sistema.
- **Sezione cartelle chiusa per impostazione predefinita**:
  - `collapsedSections` predefinito `["library.sources"]`;
  - chi ha già preferenze salvate mantiene le proprie (nessuna migrazione forzata: non si richiude una sezione che l'utente teneva aperta);
  - in fondo alla sezione, "Gestisci cartelle" apre `openModule({ module: "import" })`.

### 6. "Aggiungi a catalogo ▸" nel menu contestuale
- Sottomenu sullo schema di "Apri con", con dentro il `SearchPicker` esistente: ricerca, elenco scorrevole con `max-height` già definito, navigazione da tastiera.
- Contenuto: cataloghi manuali non di sistema in ordine alfabetico, più "Nuovo catalogo…".
- "Nuovo catalogo…" chiede il nome in un piccolo dialogo (riusa `Dialog`), poi crea il catalogo e aggiunge le foto.
- Il bersaglio è quello del menu (`menu.target`): stesso comportamento di clic destro e selezione già specificato.

## Risks / Trade-offs

- **Trascinamento con il mouse su griglie grandi**: `elementFromPoint` a ogni `pointermove`. → Basta per poche decine di righe nella barra laterale; aggiornamenti limitati a uno per frame (`requestAnimationFrame`).
- **Click fantasma dopo il trascinamento**: il rilascio sopra una miniatura genera un `click`. → Flag "appena trascinato" consumato dal primo `click`, con test.
- **Conferma che arriva tardi su selezioni grandi**: `trash_impact` su 100.000 foto per filtro. → Query aggregata indicizzata su `catalog_item(media_id)`, eseguita fuori dal thread della UI; il pulsante mostra lo stato in corso.
- **Comportamento diverso dal passato per il Cestino (BREAKING)**: chi contava sul ritorno nei cataloghi dopo "Ripristina" lo perde. → La conferma lo dice prima; "Annulla operazione" ripristina tutto subito dopo.
- **Utenti con la sezione cartelle già aperta**: vedono il nuovo ordine ma la sezione resta aperta. → Accettato: rispetta la loro scelta salvata.

## Migration Plan

- Nessuna migrazione dello schema.
- **Rilascio**: binding rigenerati per `move_to_trash` e `trash_impact`.
- **Ritorno indietro**: basta ripristinare il codice. Le foto già nel Cestino senza cataloghi restano valide anche nella versione precedente.
