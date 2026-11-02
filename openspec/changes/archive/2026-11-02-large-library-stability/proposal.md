# Proposal

## Why

Un utente che importava una libreria da 80–100.000 foto ha visto l'app chiudersi ("qualche errore su WebView") senza alcuna traccia nei log, e durante l'import compaiono molti errori `DECODE_FAILED`.

Sulla macchina di sviluppo c'è un dump di crash di WebView2 del 2 novembre 2026 alle 09:23 (`%LOCALAPPDATA%\com.photocat.app\EBWebView\Crashpad\reports\*.dmp`, `ProcessType=browser`, `msedgewebview2.exe` 156.0.4314.58, photo-cat 0.2.0). Il log dell'app di quell'ora mostra:
- memoria di sistema critica (fino a 925 MB liberi, con le fasi AI messe in pausa più volte);
- nessuna riga sul crash;
- dopo il riavvio, nessun avviso di sessione chiusa in modo anomalo.

L'analisi del codice (dettagli in `design.md`) individua le cause probabili:

1. **Crash invisibili**: viene catturato solo il panic Rust. La morte del processo WebView2, un abort nativo o una chiusura per memoria non lasciano traccia: non c'è un marcatore di sessione in corso e non c'è gestione degli eventi di WebView2.
2. **Memoria della WebView senza limiti**:
   - la griglia conserva tutte le pagine già scorse;
   - "Seleziona tutto" carica 100.000 id nella selezione;
   - "Tutti i volti" carica l'intera tabella dei volti (decine di MB) e la ricarica ogni 5 secondi;
   - la cache delle immagini non ha limiti.
3. **Thread principale bloccato**: conteggi, facet, "Seleziona tutto" e il protocollo delle miniature lavorano in modo sincrono sul thread che ospita la WebView.
4. **Valanga di eventi**: ogni errore per file diventa un evento IPC e un aggiornamento dell'interfaccia, senza raggruppamento.
5. **Decodifiche pesanti**: PNG, TIFF, WebP, BMP e GIF vengono decodificati a piena risoluzione senza limiti, con una copia in più. Il governor stima per difetto e non conta né la WebView né SQLite.

## What Changes

- **Crash sempre registrati**:
  - marcatore di sessione in corso cancellato all'uscita regolare;
  - gestione della terminazione del processo WebView2, con registrazione, tentativo di ricaricare l'interfaccia e salvataggio dello stato dei job;
  - registrazione degli abort nativi;
  - al riavvio, avviso con causa, ora e collegamento al pacchetto diagnostico, che include anche i dump di WebView2 e la memoria al momento del crash.
- **Interfaccia a memoria limitata**:
  - la griglia tiene in memoria solo le pagine vicine alla vista;
  - "Seleziona tutto" e "Seleziona tutte tranne…" diventano una selezione per filtro più eccezioni, non una lista di id;
  - le liste di persone e volti sono paginate e si aggiornano su evento invece che ogni 5 secondi;
  - limiti alla cache delle immagini.
- **Thread principale libero**: tutti i comandi che leggono il catalogo e i protocolli `thumb://` e `face://` lavorano fuori dal thread principale.
- **Errori raggruppati**: durante un job gli errori per file vengono aggregati per codice e inviati a lotti; la diagnostica mostra "1.240 file con DECODE_FAILED", con l'elenco consultabile.
- **Import robusto a 100.000 foto**:
  - decodifica ridotta anche per i formati non JPEG quando possibile, con limiti di dimensione;
  - permessi del governor che coprono le copie reali;
  - concorrenza adattata durante il job;
  - checkpoint periodico del WAL;
  - il governor tiene conto della memoria del processo e della WebView.
- **`DECODE_FAILED` spiegati**:
  - classificazione delle cause (file troncato, JPEG CMYK o con codifica non supportata, anteprima RAW assente…);
  - decodificatori di riserva per le varianti JPEG più comuni;
  - messaggi che dicono se il file è danneggiato o non supportato.
- **Verifica a 100.000 foto**: una prova ripetibile che importa 100.000 file senza crash, generati come collegamenti alle foto di `D:\Foto` in una cartella di test separata, senza mai modificare né cancellare nulla in `D:\Foto`.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `diagnostics-logging`:
  - "Cattura dei crash" (WebView2, abort nativi, sessione non chiusa, dump nel pacchetto diagnostico);
  - nuovo requisito "Errori ripetuti raggruppati".
- `desktop-shell`:
  - "Griglia di miniature performante" (memoria limitata, reattività con 100.000 foto);
  - nuovo requisito "Recupero dalla chiusura dell'interfaccia".
- `bulk-operations`: "Selezione multipla" (selezione per filtro con eccezioni).
- `resource-governor`: "Adattamento alla pressione di memoria" (memoria del processo e della WebView).
- `media-ingestion`:
  - "Formati supportati" (cause di decodifica fallita e decodificatori di riserva);
  - nuovo requisito "Import di librerie grandi".

## Impact

- **Rust**:
  - `pc-diag` (marcatore di sessione, abort nativi, dump di WebView2 nel pacchetto, aggregazione degli errori);
  - `app/src-tauri/src/lib.rs` (gestione degli eventi della WebView e del ciclo di vita dell'app, comandi `async`/`spawn_blocking`, protocollo `thumb://` asincrono);
  - `events.rs` (errori a lotti);
  - `pc-ingest/src/thumb.rs` (decodifica ridotta e limiti, decodificatori di riserva);
  - `pc-governor` (memoria del processo e dei processi figli WebView2, concorrenza dinamica);
  - `pc-store` (checkpoint del WAL, query della selezione per filtro);
  - comandi bulk che accettano una selezione per filtro.
- **UI**:
  - `Grid.tsx` (`maxPages`, selezione per insiemi);
  - store della selezione;
  - `BulkBar.tsx`, `PeoplePage.tsx` (paginazione, niente polling);
  - `store/live.ts` (aggiornamenti aggregati);
  - avviso del crash precedente.
- **Dipendenze**: forse un decodificatore JPEG di riserva (es. `zune-jpeg`), da valutare nel design; nessuna di rete.
- **Test**: nuovo test di carico ripetibile (script e istruzioni), che crea la cartella di test con collegamenti fisici o simbolici e un catalogo separato.
