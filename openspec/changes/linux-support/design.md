# Design

## Context

Motivazione e scope sono nella proposal; il comportamento richiesto è nelle delta di `specs/`. Qui interessa lo stato attuale del codice:

- **Runtime ONNX.** In debug [models.rs](../../../app/src-tauri/src/models.rs) punta sempre a `assets/runtime/win-x64/onnxruntime.dll`, e lo stesso fa `pc_models::runtime::dev_dll_path`. In release si usa `runtime/onnxruntime.dll` nella cartella risorse. Gli asset contengono solo binari Windows.
- **Deadlock di `ort`.** In `ort` 2.0.0-rc.12 con `load-dynamic`, ogni errore dentro `load_dylib_from_path` (dlopen fallito, simbolo `OrtGetApiBase` assente, versione minore insufficiente) costruisce un `ort::Error`. Il costruttore chiama `ort::api()`, che rientra nello stesso `OnceLock` in inizializzazione e resta fermo per sempre. Lo stack catturato con gdb su questa macchina lo conferma, con chiamata da `lib.rs:193` nella `setup` di Tauri, sul thread principale. Il branch contiene già una prima correzione non committata: `probe_library` in `pc-models/src/runtime.rs` ripete i tre controlli prima di chiamare `ort::init_from`.
- **HEIF.** Su piattaforme diverse da Windows, `pc_ingest::heif::library_path()` usa già `libheif.so.1` (o `PHOTOCAT_HEIF_LIBRARY`), e `lib.rs` imposta il percorso degli asset solo su Windows. I simboli C usati sono quelli di libheif 1.23.7, la versione distribuita su Windows.
- **Organizzazione file.** `developer_mode()` restituisce `false` su tutto ciò che non è Windows, e `decide()` lo usa per scegliere il messaggio `symlink_privilege` che cita Windows.
- **Risorse Tauri.** `tauri.conf.json` mappa `assets/runtime/win-x64/` su `runtime/` e produce bundle NSIS/MSI. Su Linux, per un binario non in `target/`, Tauri 2 risolve `resource_dir()` come `<exe>/../lib/photo-cat` se esiste, altrimenti come `$APPDIR/usr/lib/photo-cat` o `/usr/lib/photo-cat` (`tauri-utils` 2.10.2, `platform::resource_dir`).
- **CI e release.** Girano solo su `windows-latest`. Per scelta la CI è veloce e non esegue i test.
- **Clippy.** Con il toolchain stabile attuale (1.97), clippy fallisce già su `main` per `unnecessary_sort_by` in `commands.rs:61`.

## Goals / Non-Goals

**Goals:**
- Un solo punto che decide nome cartella e nome file delle librerie native per piattaforma, usato da app, test e HEIF.
- Nessun percorso di errore nel caricamento delle librerie native può bloccare il thread principale.
- Archivio Linux con la stessa struttura delle risorse che Tauri si aspetta, generato da uno script unico usato sia da `make` sia dalla release.

**Non-Goals:**
- Pacchetti AppImage e rpm, e pubblicazione in un repository apt (il `.deb` si scarica dalla release).
- Architetture diverse da x86_64 (aarch64) e macOS: il codice resta compatibile dove costa poco (`open` su macOS), ma non è verificato.
- Accelerazione GPU su Linux in questo change, salvo l'esito della verifica a tempo descritta sotto.
- Aggiornare `ort` a una versione che corregga il deadlock a monte: resta bloccata a `=2.0.0-rc.12`.

## Decisions

### 1. Verifica preliminare della libreria ONNX invece di fidarsi di `ort`
Prima di `ort::init_from`, `probe_library` apre la libreria con `libloading` 0.9 (la stessa versione di `ort`), risolve `OrtGetApiBase`, legge la versione e la confronta con `ort::MINOR_VERSION`. Qualsiasi errore diventa un `ErrorCode::ModelUnavailable` senza passare da `ort`. La libreria aperta per la verifica viene rilasciata subito: il caricamento successivo da parte di `ort` riapre lo stesso file.

- *Alternativa:* scegliere solo il percorso giusto per piattaforma. Basta per Linux, ma una DLL corrotta o vecchia su Windows bloccherebbe ancora l'app.
- *Alternativa:* inizializzare l'AI fuori dal thread principale. Il blocco resterebbe, solo meno visibile: il thread AI rimarrebbe appeso e `INIT` non verrebbe mai completato.
- *Alternativa:* patchare `ort` (fork o `[patch]`). Costa più manutenzione, e il problema è ristretto a un solo punto d'ingresso.

### 2. Percorsi delle librerie native per piattaforma in un solo modulo
Un modulo `pc_core::platform` (o equivalente in `pc-models`) espone il nome della cartella asset (`win-x64`, `linux-x64`) e il nome file del runtime ONNX (`onnxruntime.dll`, `libonnxruntime.so`). `models::asset_paths`, `runtime::dev_dll_path` (rinominato `dev_runtime_path`) e i test lo usano al posto delle stringhe fisse. Il campo `AssetPaths::runtime_dll` diventa `runtime_lib`. Su piattaforme senza asset (per esempio macOS) il percorso calcolato semplicemente non esiste, e la richiesta "Runtime assente per la piattaforma" è coperta dal controllo `is_file()` già presente.

### 3. Runtime ONNX CPU ufficiale per Linux negli asset
In `assets/runtime/linux-x64/libonnxruntime.so` si mette il `libonnxruntime.so.1.24.4` estratto dal pacchetto ufficiale `onnxruntime-linux-x64-1.24.4.tgz`, la stessa versione delle DLL Windows, con il SONAME originale. È sotto il limite di 100 MB per file di GitHub. Le licenze vanno in `assets/licenses/` come per Windows. Una configurazione `tauri.linux.conf.json`, unita automaticamente da Tauri su Linux, mappa `assets/runtime/linux-x64/` su `runtime/` e sostituisce i target di bundle con `deb` (vedi decisione 5). In release `asset_paths` usa quindi `runtime/libonnxruntime.so`.

### 4. HEIC con libheif di sistema
Su Linux non si distribuisce libheif: le distro la forniscono (`libheif1`, con il plugin libde265 per HEVC), e ridistribuire una LGPL compilata comporterebbe gli stessi obblighi di sorgenti già gestiti per Windows. Il caricamento resta quello attuale (`libheif.so.1`, sovrascrivibile con `PHOTOCAT_HEIF_LIBRARY`). Se manca un simbolo richiesto, perché la versione di sistema è più vecchia della 1.23.7, si ricade nel comportamento "solo metadati", con un avviso che indica la versione minima. La versione minima reale si misura sui simboli effettivamente usati da `heif.rs`.

### 5. Pacchetto deb e archivio dalla stessa build
`tauri build --bundles deb` produce `target/release/bundle/deb/photo-cat_<versione>_amd64.deb` (installa in `/usr/bin/photo-cat` e `/usr/lib/photo-cat/`, con file `.desktop` e icone) e, nella cartella `bundle/deb/<pacchetto>/data/usr/`, lo staging con `bin/photo-cat` e `lib/photo-cat/<risorse>`, cioè la struttura che `resource_dir()` riconosce come `<exe>/../lib/photo-cat`. Uno script `scripts/package-linux.sh` copia il `.deb`, comprime lo staging in `photo-cat-<versione>-linux-x64.tar.gz` e scrive un `.sha256` con fine riga LF per ciascuno. `make build` su Linux e il job Linux della release chiamano lo stesso script.

Le dipendenze del `.deb` le calcola Tauri (WebKitGTK 4.1, GTK 3). `bundle.linux.deb.depends` in `tauri.linux.conf.json` aggiunge `libheif1` come dipendenza consigliata, se il bundler lo consente; altrimenti la si documenta nel README. Le icone esistenti (`32x32`, `128x128`, `128x128@2x`) bastano per il file `.desktop`.

- *Alternativa:* `--no-bundle` e copia manuale delle risorse per l'archivio. Duplicherebbe la mappa `resources` di `tauri.conf.json`, con il rischio che le due copie divergano.
- *Alternativa:* solo `.deb`, senza archivio. L'archivio non costa nulla in più e copre le distro non Debian finché manca un AppImage.

### 6. CI e release su `ubuntu-22.04`
`ubuntu-22.04` ha glibc 2.35 e WebKitGTK 4.1, quindi il binario gira anche su distro meno recenti rispetto a una build su 24.04. I pacchetti di sistema sono quelli documentati da Tauri 2: `libwebkit2gtk-4.1-dev`, `build-essential`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`. Il job CI Linux esegue `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace` (senza `model-tests`) e la build dell'archivio, con `rust-cache` a chiave separata da quella Windows. Rispetto al principio "test prevalentemente in locale", fanno eccezione i soli test Rust su Linux: sono veloci e sono l'unica protezione contro le regressioni specifiche della piattaforma. Nella release si aggiunge un job `build-linux` accanto al job Windows `build`, che resta invariato (i passi sono troppo diversi per una matrice leggibile); il controllo tag/versione resta nel job Windows, e `publish` attende entrambi e unisce gli artefatti `release-assets*`.

### 7. Messaggi e diagnosi specifici della piattaforma
- **Collegamenti.** `developer_mode` passa da `bool` a `Option<bool>` (`None` fuori da Windows). `decide()` usa `symlink_privilege` solo se il valore è `Some(false)`; negli altri casi usa `symlink_denied`, che già descrive la destinazione. Il campo esposto alla UI resta compatibile: si serializza `false` quando il valore è `None`.
- **Cartella dei log.** `open_logs` sceglie `explorer.exe`, `open` o `xdg-open` (già nel working tree). Se il comando non parte, l'errore restituito contiene il percorso della cartella, come richiesto dalla spec.

### 8. GPU su Linux: valutazione e decisione
Opzioni con ONNX Runtime 1.24 (macchina di sviluppo: Intel UHD 630 più AMD Radeon Pro WX 5100, cioè Polaris/gfx803):

| Provider | Hardware | Distribuzione | Valutazione |
|---|---|---|---|
| CUDA | Solo NVIDIA | Pacchetto ufficiale `linux-x64-gpu`; richiede CUDA 12 e cuDNN 9 di sistema | Provider di centinaia di MB, oltre il limite di 100 MB per file del repository: servirebbe un download on demand. Non verificabile su questa macchina. |
| WebGPU (Dawn → Vulkan) | AMD, Intel e NVIDIA via Vulkan | Plugin EP (ABI introdotta in ORT 1.24), pacchetti `onnxruntime-ep-webgpu`; serve `libvulkan.so.1` | L'unica opzione multi-vendor e l'unica utile su questa macchina. Da verificare se `ort` rc.12 permette di registrare un plugin EP e se i modelli quantizzati int8 (CLIP) girano su WebGPU. |
| MIGraphX / AMDGPU (plugin AMD) | AMD con ROCm | Plugin EP; richiede lo stack ROCm | ROCm recente non supporta gfx803. Escluso. ROCm EP è deprecato. |
| OpenVINO | Intel CPU/iGPU | Build personalizzata | Guadagno modesto su UHD 630. Escluso. |

**Decisione:** questo change consegna la CPU. Un task a tempo (al massimo una giornata) verifica WebGPU su questa macchina e ne documenta l'esito nel design; se è positivo, l'integrazione diventa un change separato, perché tocca il manifest dei modelli, il flag `gpu_ok` e il download on demand. Il codice di selezione del provider resta invariato: DirectML e CoreML risultano già non supportati su Linux tramite `supported_by_platform()`.

## Risks / Trade-offs

- [`probe_library` duplica la logica interna di `ort` e potrebbe divergere a un aggiornamento] → `ort` è bloccato a `=2.0.0-rc.12`; un commento nel codice e una voce nel checklist di aggiornamento ricordano di ricontrollare il deadlock quando si alza la versione.
- [Doppio caricamento della libreria ONNX, una volta per la verifica e una per `ort`] → Il costo è trascurabile (un dlopen in più all'avvio) e non lascia stato: la verifica scarica la libreria prima di `ort`.
- [libheif di sistema più vecchia della 1.23.7, senza alcuni simboli] → Si ricade sui soli metadati con un avviso esplicito; la versione minima va documentata nel README.
- [L'archivio dipende da WebKitGTK 4.1 e glibc ≥ 2.35 del sistema] → Le dipendenze di esecuzione vanno nel README e nelle note di release; la build su 22.04 allarga la compatibilità.
- [Tempo di CI in più per il job Linux] → Cache separata; il job gira in parallelo a quello Windows.
- [Il repository cresce di circa 20–25 MB per `libonnxruntime.so`] → È coerente con la scelta già fatta per le DLL Windows di tenere gli asset nel repository.

## Migration Plan

Nessuna migrazione di dati: su Linux la cartella dati è `~/.local/share/photo-cat`, ottenuta da `directories::ProjectDirs`, e resta invariata. Una configurazione copiata da Windows con provider DirectML ripiega sulla CPU (vedi spec `model-management`). Per il rollback basta togliere il job Linux dalla release; il build Windows non dipende da nulla di quanto aggiunto per Linux.

## Open Questions

- ~~Versione minima di libheif~~ Risolta (task 3.2): `heif.rs` usa 14 simboli, tutti presenti nella libheif 1.21.2 di Ubuntu 26.04, dove il test di decodifica completo passa. Il più recente è `heif_image_get_plane_readonly2`, che si fa risalire alla 1.20 per via indiretta (OpenImageIO usa la variante `get_plane2` da libheif 1.20.0); il dato non è confermato dalle release notes. Minimo dichiarato: 1.20. Ne segue che Ubuntu 24.04 (1.17) e 22.04 (1.12) ricadono sui soli metadati, con il suggerimento di installazione. Non è stato verificato su una 24.04 reale.
- Esito della verifica WebGPU: non cambia questo change, eventualmente ne apre uno nuovo.
