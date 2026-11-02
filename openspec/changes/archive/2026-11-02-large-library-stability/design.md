# Design

## Context

### Evidenza dal crash
Le prove sono state raccolte in sola lettura sulla macchina di sviluppo il 2 novembre 2026.

- **Dump di WebView2**: `%LOCALAPPDATA%\com.photocat.app\EBWebView\Crashpad\reports\e8195465-….dmp` delle 09:23 locali. `watson_metadata` riporta `ProcessType=browser`, `ModuleName=msedge_elf.dll`, `WV=photo-cat.exe`, `WV_V=0.2.0`.
- **Log dell'app** (`%APPDATA%\photocat\photo-cat\data\logs\photo-cat.2026-11-02.0.log`):
  - dalle 08:20Z il governor segnala più volte pressione e memoria critica, con un minimo di 925 MB liberi e le fasi AI in pausa;
  - l'ultima riga prima del riavvio è delle 08:25:53Z;
  - il riavvio delle 08:36Z non registra alcuna sessione anomala.
- La libreria locale è piccola (360 foto). I `DECODE_FAILED` nel suo `error_log` vengono dal job dimostrativo ("simulato"), quindi le cause reali dei `DECODE_FAILED` dell'utente vanno ricavate dalla prova su `D:\Foto` (task 1.3).

### Codice (`main`)
- **Crash**:
  - `pc-diag/src/crash.rs:25-57` installa un hook di panic che scrive `last_session_crashed`, letto al riavvio (`lib.rs:218-224`);
  - nessun marcatore di "sessione in corso";
  - nessuna gestione di `RunEvent`, degli eventi di finestra o di `ProcessFailed` di WebView2: `.run(tauri::generate_context!())` in `lib.rs:521` è senza callback.
- **Payload proporzionali alla libreria**:
  - `library_ids` (`library.rs:253`, ~0,7 MB a 100k), copiato nello store della selezione (`BulkBar.tsx:63-65`);
  - `person_faces(None)` senza LIMIT (`pc-store/src/people.rs:147`, 15–30 MB a 100k volti), con `refetchInterval: 5000` (`PeoplePage.tsx:60-66`), e lo stesso polling su `list_people`;
  - `sequence_ids` (`timeline.rs:147`).
- **Griglia**:
  - `useInfiniteQuery` senza `maxPages` (`Grid.tsx:226-232`), che a ogni pagina ricostruisce `flatMap` e `order` (`:238-239`);
  - `selected.includes(id)` lineare (`:280`);
  - miniature servite da `thumb://` sincrono con `fs::read` (`library.rs:340-366`) e `Cache-Control: immutable`.
- **Thread principale**: `library_page`, `library_count`, `library_facets`, `library_ids`, `media_detail` e `list_catalogs` sono `fn` sincrone (`library.rs:205-282`), quindi in Tauri 2 girano sul thread principale.
- **Eventi**:
  - `job://progress` ogni 100 ms per job, con lo snapshot completo (`events.rs:24-26`, `manager.rs:20-21`), e l'UI copia l'intera mappa dei job a ogni evento (`live.ts:32`);
  - `ERROR_RECORDED` è inviato per singolo errore, senza raggruppamento (`events.rs:59-61`), e ciascuno produce un aggiornamento dello store (`live.ts:35-39`) e un inserimento nel DB.
- **Decodifica**:
  - JPEG ridotto con `jpeg_decoder` `scale` (`thumb.rs:88`);
  - PNG, TIFF, WebP, BMP e GIF decodificati a piena risoluzione con `no_limits()` più la copia di `to_rgb8()` (`thumb.rs:137-160`), con un permesso che non copre la copia;
  - per i RAW l'anteprima incorporata viene estratta due volte (`import.rs:193` e `decode_reduced`);
  - i worker sono fissati all'avvio (`lib.rs:436-437`), con 2 job concorrenti (`lib.rs:351`);
  - il governor lascia sempre passare una richiesta se nulla è in uso (`governor.rs:144`) e conta solo i buffer Rust.
- **SQLite**: WAL senza checkpoint durante gli import (solo in `vacuum`, `db.rs:255`) e `temp_store=MEMORY` (`db.rs:64`).

## Goals / Non-Goals

**Goals:**
- Nessun crash invisibile: ogni chiusura anomala lascia traccia ed è segnalata al riavvio.
- Memoria dell'interfaccia limitata e indipendente dalla dimensione della libreria.
- Thread principale mai occupato da query o I/O.
- Import di 100.000 foto ripetibile e misurato, senza crash.
- Cause dei `DECODE_FAILED` comprensibili e ridotte con decodificatori di riserva.

**Non-Goals:**
- Supporto completo dei RAW (decodifica del sensore): restano le anteprime incorporate.
- Cambiare l'architettura a processo unico o il motore della WebView.
- Ottimizzare i tempi dell'analisi AI oltre quanto serve per stare nel budget.

## Decisions

### 1. Cattura dei crash
- **Marcatore di sessione**: all'avvio `pc-diag` scrive `session.lock` (PID, ora di avvio, versione) e lo aggiorna ogni 30 s con l'ora dell'ultima attività e un riepilogo della memoria (RSS dell'app, memoria dei processi WebView2, RAM libera). All'uscita regolare (`RunEvent::Exit`) lo cancella. Al riavvio, un `session.lock` presente significa sessione chiusa in modo anomalo: si registra nel log e si mostra l'avviso con l'ora dell'ultima attività e l'ultimo riepilogo della memoria. Si unisce all'attuale `last_session_crashed`, che resta per i panic.
- **WebView2**: su Windows ci si registra all'evento `ProcessFailed` del `CoreWebView2`, accessibile tramite `with_webview` di Tauri e `webview2-com`, già dipendenza. Si registrano tipo di processo, motivo, codice di uscita e memoria; poi:
  - per `BrowserProcessExited` o `RenderProcessExited` si ricarica la pagina (`Reload`) dopo aver emesso `app://ui-recovered`, e la UI mostra l'avviso al rimontaggio;
  - si contano le terminazioni: oltre 3 in 5 minuti si mettono in pausa i job e si mostra un dialogo nativo (`tauri-plugin-dialog`) con il percorso dei log.

  Su Linux (WebKitGTK) si usa il segnale `web-process-terminated` tramite `with_webview`; su macOS ci si limita al marcatore di sessione.
- **Abort nativi**: su Windows `SetUnhandledExceptionFilter` scrive una riga nel log (codice dell'eccezione) e un minidump minimo in `logs/crash/` tramite `MiniDumpWriteDump` (crate `windows`, già nell'albero tramite WebView2). Su Linux e macOS si gestisce `SIGABRT`/`SIGSEGV` con una scrittura sincrona minima nel log.
- **Pacchetto diagnostico** (`pc-diag/src/bundle.rs`): aggiunge i `.dmp` di `EBWebView\Crashpad\reports` più recenti dell'ultimo avvio regolare (al massimo 3, al massimo 20 MB), i minidump in `logs/crash/` e `session.lock` dell'ultima sessione anomala.

### 2. Interfaccia a memoria limitata
- **Griglia**: `useInfiniteQuery` con `maxPages: 5` e `getPreviousPageParam` (paginazione bidirezionale con cursori), così risalendo si ricaricano le pagine eliminate. `order`/`items` si calcolano solo sulle pagine presenti. `gcTime` breve (30 s) per le chiavi della libreria non attive.
- **Selezione come insieme**: lo store della selezione diventa `{ mode: "ids", ids: Set<number> } | { mode: "filter", filter, snapshot: number, except: Set<number> }`. "Seleziona tutto" passa a `mode: "filter"` senza chiamare `library_ids`; l'inversione scambia la modalità; `selected.has(id)` è O(1). `snapshot` è il valore massimo di `media_item.id` (o un contatore di versione) al momento della selezione, per lo scenario "Risultati cambiati dopo la selezione". I comandi bulk accettano `Target = Ids(Vec) | Filter { filter, max_id, except }` e risolvono gli id lato Rust, in blocchi.
- **Persone e volti**: `person_faces` diventa paginato (`after`, `limit`) e "tutti i volti" non è più un elenco unico (vedi `people-faces-workspace`). Il `refetchInterval` è sostituito da invalidazioni su evento (`faces://changed`, emesso alla fine delle fasi `faces`/`cluster` e dopo le correzioni).
- **Cache delle immagini**: `Cache-Control: max-age=86400` senza `immutable`, e sui comandi di WebView2 si chiama `TrySuspend`/`MemoryUsageTargetLevel=Low` quando il governor segnala pressione (API `ICoreWebView2_19`), se disponibile nel runtime installato.
- **Eventi**: `upsertJob` sostituisce solo l'elemento cambiato (immutabilità mirata), con progresso inviato al massimo ogni 250 ms per job.

### 3. Thread principale libero
Tutti i comandi che accedono al catalogo diventano `async` e usano `spawn_blocking`, con una regola di clippy o un test che elenca i comandi sincroni ammessi. `thumb://` passa a `register_asynchronous_uri_scheme_protocol` con lettura su un pool di I/O. Le query di conteggio e facet di un filtro superato vengono annullate: la UI passa un `request_id` e il backend scarta i risultati vecchi, con `sqlite3_interrupt` sulla connessione di lettura dedicata se la query è ancora in corso.

### 4. Errori raggruppati
`TauriDiagSink::on_error` accumula gli errori in un buffer e ogni 500 ms invia `errors://batch` con i conteggi per `(job_id, phase, code)` e gli ultimi 20 record. L'UI aggiorna lo store una volta per lotto. Il `DbErrorSink` scrive a lotti in un'unica transazione. La Diagnostica aggiunge la vista "per codice" (query `GROUP BY job_id, phase, code`) con l'elenco dei file paginato.

### 5. Decodifica e memoria dell'import
- PNG, WebP e TIFF: si legge prima l'intestazione. Oltre `MAX_PIXELS` (100 megapixel predefiniti) il file viene registrato come "immagine oltre i limiti" senza decodificarlo. Altrimenti si decodifica direttamente nel formato di destinazione, senza `to_rgb8` aggiuntivo (conversione per righe dove l'API lo consente), e il permesso del governor copre `w*h*bpp` più la copia RGB. Si usa `image::io::Limits` con `max_alloc` pari al permesso concesso.
- RAW: l'anteprima incorporata viene estratta una sola volta e passata nello stato dell'elemento.
- **Decodificatore di riserva per JPEG**: se `jpeg_decoder` fallisce con un errore di formato (CMYK/YCCK, codifica aritmetica, 12 bit), si riprova con `zune-jpeg` (Rust puro, licenza MIT/Apache/Zlib). Su Windows, in ultima istanza, si usa il decodificatore di sistema WIC tramite il crate `windows`, che copre anche alcuni HEIC/JPEG XR. La scelta definitiva dipende dalla classificazione del task 1.3.
- **Cause classificate**: un enum `DecodeCause { Truncated, Corrupt, UnsupportedVariant(String), NoRawPreview, TooLarge, Io }`, con messaggio e suggerimento propri, salvato nel `detail` dell'errore. Il codice resta `DECODE_FAILED` (o `UNSUPPORTED_FORMAT`) per compatibilità.
- **Concorrenza dinamica**: i worker di decodifica leggono la capacità del governor a ogni elemento invece di fissarla all'avvio. In pressione critica la fase miniature si sospende come le fasi AI.
- **Governor e processi**: il `PressureMonitor` misura anche l'RSS del processo e dei processi figli `msedgewebview2.exe` (via `sysinfo`, figli per PID padre). La quota dell'interfaccia è predefinita a 1,5 GB: se la supera, si riduce la capacità e si emette `app://trim-caches`, a cui l'UI risponde con `queryClient.clear()` delle query inattive.
- **SQLite**: `wal_autocheckpoint` predefinito più un `wal_checkpoint(PASSIVE)` ogni 5.000 elementi scritti durante l'import, e `temp_store=FILE` per le query della libreria (le tabelle temporanee grandi vanno su disco).

### 6. Prova a 100.000 foto
Lo script `scripts/loadtest/make-library.ps1` (con equivalente `.sh` per Linux):
- crea `D:\photo-cat-loadtest\library\` (mai dentro `D:\Foto`);
- popola la cartella con **collegamenti fisici** (`New-Item -ItemType HardLink`, stesso volume D:) ai file immagine di `D:\Foto`, ripetuti in sottocartelle `copy-NN\` fino a raggiungere 100.000 file;
- crea, solo per una parte dei file, varianti generate (ritaglio e ricodifica in una cartella `variants\`) così che non tutto sia un duplicato esatto.

`D:\Foto` è aperta solo in lettura. Lo script verifica all'inizio e alla fine che numero e date di modifica dei file in `D:\Foto` siano invariati e fallisce altrimenti. Lo script di pulizia cancella solo `D:\photo-cat-loadtest`, con un controllo esplicito che il percorso non sia dentro `D:\Foto`.

L'app si avvia con `PHOTO_CAT_DATA_DIR=D:\photo-cat-loadtest\data`. Un registratore (`scripts/loadtest/monitor.ps1`) campiona ogni 5 s l'RSS di `photo-cat.exe` e dei processi `msedgewebview2.exe` figli in un CSV. Criteri di accettazione:
- import e analisi completati senza chiusure anomale;
- RSS dell'app entro il budget più 1,5 GB della WebView;
- nessun blocco dell'interfaccia oltre 200 ms durante lo scorrimento (misurato con il profiler della WebView o un contatore di long task registrato dall'UI in modalità diagnostica).

## Risks / Trade-offs

- [API di WebView2 (`ProcessFailed`, `MemoryUsageTargetLevel`) non disponibili in tutte le versioni del runtime] → controllo della versione già presente (`webview_runtime.rs`) e degradazione: senza l'API resta il marcatore di sessione.
- [La selezione per filtro cambia la firma dei comandi bulk usati da altre change (`catalog-exclusions`, `similar-photo-stacks`)] → `Target::Ids` resta supportato; le altre change si adeguano al merge (note in `tasks.md`).
- [I collegamenti fisici producono molti duplicati esatti e falsano rilevamento dei duplicati e analisi] → le varianti generate e un secondo scenario con sole foto distinte (circa 20.000 da `D:\Foto`) separano i due aspetti; documentato nei risultati.
- [Gestori di segnali e eccezioni nativi difficili da testare] → test manuale con un comando nascosto di sviluppo che provoca un abort (solo in build di debug).
- [Comandi resi `async` che cambiano l'ordine di risposta] → `request_id` e scarto dei risultati superati; test UI con risposte fuori ordine.

## Migration Plan

Nessuna migrazione di schema. `session.lock` e i nuovi campi di configurazione hanno valori predefiniti. Rollback: revert; un `session.lock` rimasto viene ignorato dalle versioni precedenti.

## Open Questions

- Elenco definitivo dei decodificatori di riserva: dipende dalla classificazione dei `DECODE_FAILED` reali su `D:\Foto` (task 1.3), senza cambiare l'approccio.
- Quota di memoria predefinita della WebView (1,5 GB): da tarare con la prova a 100.000 foto.
