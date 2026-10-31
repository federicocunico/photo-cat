# Proposal

## Why

Chi possiede migliaia di fotografie sparse in cartelle non ha un modo locale, privato e veloce per catalogarle automaticamente (date, luogo, persone, tipo di soggetto, interno/esterno) e riorganizzarle per l'interesse del momento. `docs/Applicazione per catalogare le fotografie.md` descrive l'idea e uno studio di fattibilità: questo change crea da zero il prodotto **photo-cat**, un gestore desktop in Rust con AI 100% locale, ponendo fondamenta pensate per crescere (altri tipi di media, nuovi modelli, nuove catalogazioni).

## What Changes

- Nuova applicazione desktop **Tauri 2 (backend Rust) + React/TypeScript**, cross-platform con Windows come piattaforma primaria.
- UI pulita in stile "Adobe-like" (Lightroom/Bridge): tema scuro, pannelli laterali, griglia di miniature, pannello metadati, ma semplice e intuitiva. I mockup delle schermate sono prodotti con il Design artifact e referenziati da `design.md`.
- **Libreria + cataloghi**: tutti i file aggiunti confluiscono in una libreria unica; l'utente crea **cataloghi tematici** (manuali o "smart" basati su filtri) e opera **in bulk** su selezioni di file.
- **Ingestione**: scansione ricorsiva di cartelle, estrazione caratteristiche tecniche (formato, dimensioni, DPI, profondità bit, peso) ed EXIF (date, fotocamera, GPS), hash per deduplicazione, rilevamento file modificati/spostati/mancanti.
- **Catalogazione AI locale** (ONNX Runtime, CPU di default, GPU opzionale): classificazione zero-shot CLIP (tipo soggetto, interno/esterno, ambiente), rilevamento e conteggio persone (YOLO), **riconoscimento volti** (detection + embedding + clustering + assegnazione nome alle persone).
- **Geolocalizzazione** da GPS EXIF, con fallback sul nome della directory.
- **Gestione modelli**: manifest dei modelli, auto-download all'avvio, verifica integrità (SHA-256), self-test di funzionamento, schermata dedicata.
- **Governor delle risorse**: rilevamento RAM (totale/disponibile) e CPU all'avvio e in esecuzione; budget di memoria che decide parallelismo, batch size e quali modelli possono stare in memoria contemporaneamente.
- **Job e progresso**: pipeline concorrente (scansione → metadati → miniature → AI → scrittura DB) con percentuali per fase e complessive, ETA, pausa/annulla/ripresa.
- **Diagnostica**: log strutturati su file con rotazione, raccolta centralizzata degli errori per file/fase, consultabile ed esportabile dalla UI.
- **Export e organizzazione**: file strutturato (CSV/JSON) con path, nome, caratteristiche tecniche e catalogazioni; creazione di cartelle organizzate per catalogazione tramite symlink/hard link (copia come fallback), senza toccare gli originali.

## Capabilities

### New Capabilities

- `catalog-library`: libreria unica dei media, cataloghi tematici (manuali e smart), tag, ricerca e filtri.
- `media-ingestion`: sorgenti (cartelle monitorate), scansione, metadati tecnici ed EXIF, miniature, hash, rilevamento modifiche/duplicati.
- `ai-classification`: catalogazioni automatiche via CLIP zero-shot e rilevamento persone, tassonomia configurabile, geolocalizzazione.
- `face-recognition`: rilevamento volti, embedding, clustering in persone, assegnazione/correzione dell'identità da parte dell'utente.
- `model-management`: manifest, auto-download, verifica integrità, self-test e selezione del provider di esecuzione (CPU/GPU).
- `resource-governor`: rilevamento RAM/CPU e budget di memoria che regola concorrenza, batch e residenza dei modelli.
- `job-orchestration`: pipeline concorrente a fasi, progresso percentuale per fase, ETA, pausa/annulla/ripresa, persistenza dei job.
- `diagnostics-logging`: log su file con rotazione, registro errori strutturato per file/fase, vista e export diagnostica.
- `bulk-operations`: azioni su selezioni multiple (tag, assegnazione a cataloghi, rianalisi, rimozione dalla libreria, correzione campi).
- `catalog-export`: generazione del file strutturato di catalogazione (CSV/JSON) per libreria, catalogo o selezione.
- `file-organization`: materializzazione di cartelle organizzate per catalogazione tramite symlink/hard link/copia, con anteprima e annullamento.
- `desktop-shell`: struttura dell'applicazione desktop, navigazione, layout a pannelli, temi, impostazioni e onboarding al primo avvio.

### Modified Capabilities

<!-- Nessuna: progetto nuovo, non esistono spec. -->

## Impact

- **Codice**: nuovo repository applicativo (workspace Cargo con crate core separati dalla shell Tauri, frontend React/TS con Vite). Nessun codice esistente.
- **Dipendenze principali**: `tauri` 2, `ort` (ONNX Runtime), `image`, `kamadak-exif`, `rusqlite` (SQLite, bundled), `tokio`, `rayon`, `sysinfo`, `tracing` + `tracing-appender`, `reqwest`, `sha2`, `blake3`; frontend React, TypeScript, TanStack (Query/Virtual), Zustand.
- **Rete**: unica connessione esterna è il download dei modelli (e opzionalmente del runtime ONNX) da URL configurati nel manifest; nessuna foto lascia la macchina.
- **Disco**: dati app in directory utente standard (DB, cache miniature, modelli ~ 0,5–1 GB, log).
- **Sistema**: symlink su Windows richiedono Developer Mode o privilegi; il design prevede fallback a hard link/copia.
- **Privacy**: gli embedding dei volti sono dati biometrici e restano esclusivamente in locale, cancellabili dall'utente.
- **CI e rilascio**: pipeline Windows veloce con cache sul repository `federicocunico/photo-cat`; test principalmente locali, installer verificato in Windows Sandbox e GitHub Release automatica dell'EXE su tag di versione. Le foto reali usate per la verifica restano in sola lettura fuori dal repository e dagli artefatti pubblicati.
