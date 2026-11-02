# Design

## Context

Stato attuale su `main`:

- **Componenti**: in `app/ui/src/components` ci sono solo `icons.tsx`, `shell/` e `floating/`. L'unico spinner è l'icona `Icon.Spinner` (`icons.tsx:117`). Nessun uso di `useIsFetching`/`useIsMutating`.
- **Query**: le pagine leggono `query.data ?? []` e non distinguono "in caricamento" da "vuoto". Ad esempio `Sidebar.tsx:100,180,238`, `ImportPage.tsx:372`, `JobsPage.tsx:387`, `DiagnosticsPage.tsx:182`, `WhoIs.tsx:37`, `PlaceEditor.tsx:31`, e nessuna gestione in Inspector, Tag, Persone, Identità, Cataloghi, Modelli. Eccezioni: `FacetValuesList.tsx:92,126` (testo "caricamento"), `SettingsPage.tsx:65`, `Onboarding.tsx:69`, `Grid.tsx:390-392` (solo pagina successiva). `CatalogsPage.tsx:70` mostra 0 durante il caricamento dell'anteprima.
- **Mutazioni**: BulkBar, CategoryPicker, RenamePlaceDialog, Diagnostica, WhoIs, Import, Organizza, Persone e Impostazioni usano un flag `busy` locale che disabilita i pulsanti, senza mostrare uno spinner. Tag (158, 263, 477, 555, 623), Identità (256, 383), Inspector (111, 126, 153), Cataloghi (77), Sequenza (667-692) e GridContextMenu (187) non hanno protezione dal doppio invio.
- **TopBar**: `JobPill` (`TopBar.tsx:44-68`) disegna lo spinner anche senza job (riga 59).
- **Avvio**: `.setup()` in `app/src-tauri/src/lib.rs:244` esegue in modo sincrono il backup di `config.toml`, l'apertura e la migrazione del catalogo, l'archivio dell'app e il caricamento del runtime ONNX (con attesa fino a 30 s, `lib.rs:285-295`), e poi chiama `app.manage(AppState)`. I comandi ricevono `State<AppState>`. In UI, `App.tsx:63-80` parte con `onboarding=false` e decide dopo, quindi la Libreria compare prima dell'onboarding.

## Goals / Non-Goals

**Goals:**
- Una sola libreria di componenti e un solo schema (caricamento / vuoto / errore / aggiornamento) usati da tutti i moduli.
- Nessun doppio invio di comandi.
- Finestra visibile subito all'avvio con le fasi di inizializzazione.

**Non-Goals:**
- Rendere più veloci le query: è compito di `large-library-stability`. Qui si rende visibile l'attesa.
- Progresso percentuale delle singole richieste IPC: c'è solo l'indicazione "in corso". I job hanno già la loro percentuale.

## Decisions

### 1. Componenti `components/feedback/`
- `Spinner` (`size`, `label` per gli screen reader) basato su `Icon.Spinner`, con `prefers-reduced-motion` che sostituisce la rotazione con una pulsazione d'opacità.
- `Skeleton` (riga, blocco, miniatura) con i colori dei token (`--pc-raised`, `--pc-panel`).
- `QueryView<T>({ query, empty, skeleton, children })`: mostra `skeleton` mentre `query.isPending`, il messaggio d'errore con "Riprova" (`query.refetch`) se `isError`, `empty` se il risultato è vuoto (array vuoto o funzione `isEmpty`), altrimenti `children(data)`. È l'unico modo previsto per rendere un elenco.
- `RefreshBar`: barra sottile animata in cima a un pannello quando `isFetching && !isPending`. Il contenuto resta visibile, attenuato solo nella griglia.
- `BusyButton`: incapsula un `onClick` asincrono, ignora i clic mentre la promessa è pendente, mostra lo spinner nel pulsante mantenendo testo e larghezza, imposta `aria-busy`. Sostituisce i flag `busy` locali.
- `Toast` e `useToast()`: avvisi non modali (`role="status"`, chiusura automatica configurabile, azione opzionale come "Annulla"), impilati in basso a destra sopra la barra di stato.

### 2. Indicatore delle richieste in corso
`lib/ipc.ts` avvolge `commands` con un proxy che incrementa e decrementa un contatore in uno store zustand (`store/pending.ts`). Il `TopBar` mostra un piccolo indicatore se il contatore è > 0 da più di 300 ms (ritardo per evitare sfarfallii), con un tooltip sul numero di richieste. Sono esclusi i comandi di polling leggero (stato dei job, RAM) tramite un elenco esplicito. `JobPill` anima lo spinner solo con job attivi.
*Alternativa scartata*: `useIsFetching` di react-query — non vede i comandi chiamati fuori dalle query (azioni, `startBulk` ecc.).

### 3. Griglia
- Primo caricamento: righe di `Skeleton` miniatura che riempiono la vista.
- Conteggio: `total.isPending` → "in calcolo…", mai il numero di elementi già caricati.
- Cambio di filtro: `placeholderData: keepPreviousData` e la griglia attenuata con `RefreshBar` finché arriva la prima pagina dei nuovi risultati.

### 4. Avvio con schermata di caricamento
- `index.html` contiene una schermata di avvio statica (logo, fase, nessun JS esterno) visibile prima che React parta. React la sostituisce con `BootScreen` quando monta.
- Lato Rust, `setup` gestisce subito uno stato `Boot` (fase corrente, eventuale errore) e avvia l'inizializzazione pesante (backup e migrazione, archivio dell'app, runtime ONNX, governor, job manager) su un thread. Al termine viene registrato `AppState` (`app.manage` è chiamabile anche dopo `setup`) ed emesso l'evento `boot://ready`, mentre ogni fase emette `boot://phase`.
- I comandi che usano `State<AppState>` prima della fine dell'avvio non devono andare in panic. L'UI non li chiama finché non riceve `ready`: `App.tsx` resta su `BootScreen` finché `boot_status()` (comando che legge solo `Boot`) non risponde `ready`, poi legge configurazione e onboarding e solo allora sceglie tra onboarding e Libreria.
- Errore bloccante: `BootScreen` mostra il messaggio, il percorso dei log e i comandi "Riprova" (riavvia l'inizializzazione) e "Scegli un'altra cartella dati".
- Il primo task di questa parte è una verifica rapida che `app.manage` dopo `setup` e i comandi con `State` funzionino con la versione di Tauri in uso. Se non è possibile, il ripiego è registrare subito un `Arc<OnceLock<AppState>>` e accedere allo stato con una funzione che restituisce l'errore `NOT_READY` finché non è pronto.

### 5. Applicazione ai moduli
Ogni modulo passa a `QueryView`, `RefreshBar` e `BusyButton`. L'elenco dei punti da toccare è nel Context; ogni task di gruppo cita i file. I test usano `ipcMock` con risposte ritardate per verificare i tre stati.

## Risks / Trade-offs

- [Inizializzazione asincrona che introduce race con comandi chiamati presto (eventi, protocolli `thumb://` e `face://`)] → i protocolli rispondono 503 finché lo stato non è pronto e la UI non carica immagini prima di `ready`; test di avvio con catalogo da migrare.
- [Proxy su `commands` che cambia i tipi generati] → il proxy preserva le firme (`typeof commands`); i test esistenti continuano a usare `ipcMock`.
- [Molti file toccati, conflitti con le altre change] → modifiche per modulo in task separati e piccoli; `large-library-stability` tocca `Grid.tsx` e `PeoplePage.tsx`, `people-faces-workspace` riscrive Persone/Identità: i loro componenti nuovi usano direttamente `QueryView` e `BusyButton` se questa change è già unita.

## Migration Plan

Nessun dato coinvolto. Rollback: revert.

## Open Questions resolved

- Tauri 2.12.5: `tests/late_state.rs` uses the real IPC dispatcher with MockRuntime. A command requiring managed State rejects before registration; registering it on a worker after setup makes the same command succeed. The executable test passes on Linux. The primary route (late AppState registration) is therefore used; no OnceLock wrapper is needed.
- `boot::open_stores` is shared by startup and a migration test against a real schema-v1 database in a temporary directory. It verifies opening, migration and archive phases, schema upgrade and catalog/config backups.
- The logger is retained through retries using Arc<Diagnostics>. If the selected data directory cannot hold logs, boot errors point to the temporary boot log directory. The catalog and app archive are initialized on the startup worker.

## Decisione sulle verifiche (2026-11-02)

L'utente ha autorizzato la conclusione con CI e archive dopo il blocco di una prova GUI interattiva completa (come per `catalog-exclusions`). Il flusso interattivo del task 5.2 viene differito e resta documentato in tasks.md; i requisiti funzionali rimangono invariati. Sono previsti i test automatici Rust/UI (Boot, QueryView, BusyButton, Toast, griglia) e `make test-linux`; l'avvio con migrazione in cartella temporanea è coperto dal test `boot::tests::phases_cover_migration_and_app_archive_with_a_real_old_catalog`.
