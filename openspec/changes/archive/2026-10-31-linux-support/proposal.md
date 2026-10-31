# Proposal

## Why

photo-cat è dichiarata cross-platform, ma in pratica funziona solo su Windows: su Linux `make dev` apre una finestra che non viene mai disegnata, perché all'avvio l'app prova a caricare `onnxruntime.dll` e un bug di `ort` 2.0.0-rc.12 trasforma il fallimento in un blocco permanente del thread principale. Anche superato quel punto, runtime AI, decodifica HEIC, apertura delle cartelle, messaggi sui collegamenti, CI e release presuppongono Windows. Questo change rende Linux (x86_64) una piattaforma supportata per sviluppo, uso e rilascio.

## What Changes

- **Avvio robusto**: il caricamento del runtime ONNX non può più bloccare l'app. Una libreria mancante, non caricabile o di versione insufficiente disattiva le analisi AI con un errore diagnostico, e l'interfaccia parte comunque. Vale su tutte le piattaforme, Windows compreso.
- **Runtime AI per Linux**: ONNX Runtime CPU per `linux-x64` distribuito negli asset come su Windows, con percorsi scelti per piattaforma (sviluppo e build). Su Linux l'AI gira su CPU; l'accelerazione GPU viene valutata nel design (CUDA, WebGPU/Vulkan, MIGraphX) e resta fuori dal primo rilascio, a meno che una verifica a tempo non dia esito positivo.
- **HEIC su Linux**: decodifica tramite la `libheif` di sistema, se presente; altrimenti si applica il comportamento già previsto (solo metadati, con avviso).
- **Integrazione col desktop**: "Apri cartella log" usa il file manager di sistema (`xdg-open`), e anche macOS è coperto (`open`). Il messaggio d'errore per sistemi non supportati diventa comprensibile.
- **Organizzazione file**: su Linux i symlink non richiedono privilegi. Quando falliscono, il motivo indicato riguarda la destinazione (per esempio un filesystem FAT/exFAT), non la "Modalità sviluppatore di Windows".
- **Build e strumenti**: configurazione Tauri specifica per Linux (risorse `linux-x64`, nessun bundle NSIS/MSI) e target `make` per la build Linux. I warning e gli errori di clippy emersi su Linux vengono eliminati, così `make test` passa.
- **CI e release**: job Ubuntu in CI (fmt, clippy, test Rust, build). Le release su tag allegano anche il pacchetto `photo-cat_<versione>_amd64.deb` (Debian/Ubuntu) e l'archivio `photo-cat-<versione>-linux-x64.tar.gz` (binario e risorse, per le altre distro), ciascuno con checksum SHA-256. AppImage e rpm esclusi per ora.

## Capabilities

### New Capabilities

<!-- Nessuna: i comportamenti rientrano nelle capability esistenti. -->

### Modified Capabilities

Le capability sono definite nel change `photo-catalog-foundation`, non ancora archiviato in `openspec/specs/`. Le delta aggiungono requisiti (ADDED) senza modificare quelli esistenti.

- `model-management`: il runtime ONNX è risolto per piattaforma e un suo errore non blocca mai l'avvio; provider di esecuzione su Linux.
- `media-ingestion`: decodifica HEIC su Linux tramite libheif di sistema.
- `diagnostics-logging`: apertura della cartella dei log con il file manager della piattaforma.
- `file-organization`: diagnosi dei collegamenti simbolici indipendente da Windows.
- `desktop-shell`: avvio su Linux, CI Linux, pacchetto deb e archivio Linux nelle release.

## Impact

- **Codice**: `crates/pc-models` (runtime, verifica preliminare della libreria; già presente nel working tree del branch), `app/src-tauri` (`models.rs`, `lib.rs`, `diagnostics.rs`, `commands.rs`), `crates/pc-ingest/src/heif.rs`, `crates/pc-organize/src/organize.rs`.
- **Configurazione**: nuovo `app/src-tauri/tauri.linux.conf.json`, `Makefile`, `.github/workflows/ci.yml` e `release.yml`.
- **Asset**: nuova cartella `assets/runtime/linux-x64/` con `libonnxruntime.so` 1.24.4 (CPU, MIT) e voce corrispondente in `assets/README.md` e nelle licenze.
- **Dipendenze**: `libloading` 0.9 diretta in `pc-models` (già usata da `ort`). Su Linux per compilare servono i pacchetti di sistema WebKitGTK 4.1, per eseguire `libwebkit2gtk-4.1` e, facoltativamente, `libheif1`.
- **Compatibilità**: nessun cambiamento di comportamento su Windows, salvo che una DLL ONNX corrotta o troppo vecchia ora produce un errore invece di bloccare l'app.
