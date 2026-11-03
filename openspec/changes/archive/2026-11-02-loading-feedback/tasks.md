# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Coordinamento**: `large-library-stability` (sposta comandi fuori dal thread principale, limita le liste, tocca `Grid.tsx` e `PeoplePage.tsx`) e `people-faces-workspace` (riscrive Persone e Identità) sono in preparazione su branch propri. Controllare cosa è già unito in `main`. Se `people-faces-workspace` è già unito, applicare questa change al suo nuovo modulo invece che a `PeoplePage.tsx` e `IdentitiesPage.tsx`.
> - **`Toast`**: `catalog-exclusions` ne ha bisogno. Se è già stato creato lì, riusarlo ed estenderlo invece di duplicarlo.
> - **Avvio asincrono**: il task 4.1 è una verifica tecnica. Se il ripiego descritto nel design non basta, fermarsi e chiedere prima di ristrutturare `AppState`.

## 1. Componenti comuni

- [x] 1.1 Creare `components/feedback/` con `Spinner`, `Skeleton`, `QueryView`, `RefreshBar`, `BusyButton`, `Toast`/`useToast` (design §1), con stili dai token e `prefers-reduced-motion`. Verifica: test in `components/feedback/*.test.tsx` per i tre stati di `QueryView` (incluso "Riprova" che richiama `refetch`), `BusyButton` che ignora il secondo clic e imposta `aria-busy`, e `Toast` con azione e chiusura automatica.
- [x] 1.2 Proxy su `commands` con lo store `store/pending.ts`, indicatore nel `TopBar` dopo 300 ms, esclusione dei comandi di polling, e `JobPill` animato solo con job attivi (design §2). Verifica: test in `shell.test.tsx` per "Richiesta lenta" (con `ipcMock` ritardato) e "Nessun job attivo".

## 2. Libreria

- [x] 2.1 Griglia: segnaposto al primo caricamento, conteggio "in calcolo…", risultati precedenti attenuati con `RefreshBar` al cambio di filtro (design §3). Verifica: test in `LibraryPage.test.tsx` per "Primo caricamento della griglia" e "Aggiornamento in background".
- [x] 2.2 Sidebar (sorgenti, cartelle, cataloghi, filtri) e Inspector con `QueryView`; azioni dell'Inspector, BulkBar, `GridContextMenu`, `CategoryPicker`, `PlaceEditor` e `RenamePlaceDialog` con `BusyButton`. Verifica: test che durante il caricamento non compare "nessuna sorgente" e che un doppio clic su un'azione dell'Inspector invia un solo comando.

## 3. Altri moduli

- [x] 3.1 Import, Job e Diagnostica: `QueryView`, `RefreshBar` e `BusyButton`. Verifica: test per "Elenco delle sorgenti in caricamento" e per l'errore con "Riprova".
- [x] 3.2 Cataloghi (inclusa l'anteprima del conteggio che oggi mostra 0), Tag e Sequenza: `QueryView` e `BusyButton` sulle azioni senza protezione elencate nel design. Verifica: test per "Errore di caricamento" nei Cataloghi e per "Doppio clic" su un'azione dei Tag.
- [x] 3.3 Persone e Identità (o il nuovo modulo, se `people-faces-workspace` è unito), Modelli, Organizza e Impostazioni. Verifica: test per "Elenco vuoto" in Persone e "Doppio clic su Salva" nell'editor delle identità.
- [x] 3.4 Avvisi di esito con `Toast` per le operazioni oltre un secondo (bulk, unione tag, salvataggi). Verifica: test per "Esito di un'operazione lenta".

## 4. Avvio

- [x] 4.1 Verifica tecnica: `app.manage` dopo `setup` e comandi con `State<AppState>` chiamati prima della registrazione, con la versione di Tauri in uso; annotare l'esito in Open Questions del design e scegliere tra la soluzione principale e il ripiego (design §4). Verifica: un test o un prototipo in un ramo locale che dimostra il comportamento, descritto nel design.
- [x] 4.2 Stato `Boot` con fasi ed eventi `boot://phase`/`boot://ready`, inizializzazione su thread, protocolli `thumb://`/`face://` che rispondono 503 prima di `ready`, comando `boot_status`. Verifica: test Rust dell'avanzamento delle fasi con un catalogo da migrare in una cartella temporanea, e avvio manuale dell'app con un catalogo grande.
- [x] 4.3 Schermata statica in `index.html`, `BootScreen` in React con fasi ed errore ("Riprova", "Scegli un'altra cartella dati"), e scelta tra onboarding e Libreria solo dopo `ready` e lettura della configurazione. Verifica: test in `App.test.tsx` per "Primo avvio" (nessun render della Libreria prima dell'onboarding) ed "Errore all'avvio".

## 5. Verifica d'insieme

- [x] 5.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde (`make test-linux` con `SKIP_PACKAGE=1` su Windows).
- [x] 5.3 Allineare le delta e verificare che `openspec validate loading-feedback --strict` passi.

## Verifica interattiva differita (decisione del 2026-11-02)

La prova precedentemente prevista dal task 5.2 resta da eseguire:

5.2 Prova sull'app reale con una cartella dati separata (`PHOTO_CAT_DATA_DIR`) e una libreria grande: avvio con migrazione, navigazione in tutti i moduli con rete di IPC lenta (sotto carico di import), doppi clic sulle azioni. Verifica: nessuno stato vuoto mostrato durante un caricamento, nessun doppio invio, nessun errore nuovo in Diagnostica.

L'utente ha richiesto di concludere con CI/validate/archive differendo la prova GUI interattiva (stesso modello di `catalog-exclusions`). Non viene dichiarata superata.

Copertura automatica già presente: stati QueryView/BusyButton/Toast, griglia (primo caricamento e refresh), BootScreen (`App.test.tsx`), fasi di migrazione Rust (`boot::tests`), protocolli `thumb://`/`face://` con 503 prima di `ready`.

## Workflow follow-up

- Coordinare il merge con `people-faces-workspace` e `similar-photo-stacks` (LLS e catalog-exclusions già in main).
- Archiviare il change dopo la revisione.
- Eseguire la prova interattiva 5.2 quando disponibile una sessione GUI.
