# PhotoCat

Gestore desktop locale per catalogare fotografie: libreria unica, cataloghi tematici, operazioni bulk, catalogazione automatica con reti neurali eseguite interamente sul proprio computer.

Stack: backend Rust (workspace `crates/pc-*`) + shell Tauri 2 (`app/src-tauri`) + UI React/TypeScript (`app/ui`).
Piano e requisiti: `openspec/changes/photo-catalog-foundation/`.

## Prerequisiti

- Rust stable ≥ 1.90 (`rustup`), con `rustfmt` e `clippy`
- Node.js ≥ 20 e npm
- Windows: Microsoft Edge WebView2 Runtime (preinstallato su Windows 11) e Build Tools di Visual Studio (C++)
- Linux (Debian/Ubuntu): `sudo apt install build-essential libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`
  - per le foto HEIC: `libheif1` ≥ 1.20 e `libheif-plugin-libde265` (senza, gli HEIC vengono importati con i soli metadati)
- macOS: Xcode Command Line Tools

## Sviluppo

Scorciatoie (GNU Make: su Windows da cmd o Git Bash, su Linux dal terminale):

| Comando | Cosa fa |
|---|---|
| `make dev` (o `make run`) | avvio locale in debug: Vite + finestra Tauri con ricarica |
| `make build` | test di build come la CI: setup NSIS in `target/release/bundle/nsis` su Windows, `.deb` e `.tar.gz` con SHA-256 in `target/release/dist` su Linux |
| `make test` | fmt, clippy, test Rust e UI (i test non girano in CI) |
| `make release` | crea e pubblica il tag `v<versione>` di `tauri.conf.json`: la GitHub Action pubblica la release con installer Windows, pacchetti Linux e SHA-256 |

Per una nuova versione aggiornare prima `version` in `Cargo.toml` e in `app/src-tauri/tauri.conf.json` (devono coincidere: la pipeline lo verifica).

Comandi equivalenti:

```sh
# dipendenze frontend (una volta)
npm ci --prefix app/ui

# app in modalità sviluppo (avvia Vite su :1420 e la finestra Tauri)
npm --prefix app/ui run tauri dev

# build del frontend (necessaria prima di `cargo build` del crate photo-cat)
npm --prefix app/ui run build

# workspace Rust
cargo build --workspace
cargo test --workspace
```

## Installazione su Linux

Le release includono, per Linux x86_64:

- `photo-cat_<versione>_amd64.deb` per Debian/Ubuntu: `sudo apt install ./photo-cat_<versione>_amd64.deb` installa anche le dipendenze e aggiunge photo-cat al menu delle applicazioni (categoria Fotografia). Si rimuove con `sudo apt remove photo-cat`; i dati in `~/.local/share/photo-cat` restano.
- `photo-cat-<versione>-linux-x64.tar.gz` per le altre distro: si estrae dove si vuole e si avvia `bin/photo-cat`. Servono WebKitGTK 4.1 e GTK 3 del sistema (su Debian/Ubuntu `libwebkit2gtk-4.1-0`) e glibc ≥ 2.35.

In entrambi i casi la decodifica HEIC usa la libheif di sistema (vedi Prerequisiti). L'AI gira su CPU: il runtime ONNX per Linux è incluso.

Verifica del download: `sha256sum -c <file>.sha256` nella cartella del file.

## Qualità

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix app/ui run lint
npm --prefix app/ui run typecheck
npm --prefix app/ui run format:check
npm --prefix app/ui test
```

## Modelli inclusi e verifica

I modelli e il runtime ONNX per Windows e Linux sono inclusi in `assets/`, nell'installer e nei pacchetti;
fonti e licenze sono documentate in [assets/README.md](assets/README.md).
All'avvio la verifica e l'eventuale ripristino/download avvengono in background.
La schermata **Modelli AI** permette di verificare, installare un file locale,
eseguire i self-test e scegliere provider e profilo di memoria. Il provider
predefinito è CPU; gli acceleratori disponibili sono selezionabili e, in caso
di errore di inizializzazione, si torna alla CPU registrando il motivo.

Su Windows e su Linux, per eseguire anche i test con i pesi e le immagini reali inclusi:

```sh
cargo test -p pc-models -p pc-ai --features model-tests
# riepilogo dei self-test e dei tempi medi
cargo run -p pc-ai --example selftest_all -- cpu
```

I test ordinari usano server HTTP locali e file temporanei per verificare
ripresa dei download, mirror, file corrotti e annullamento. I test
`model-tests` aggiungono inferenza reale, fallback dei provider, gestione
delle sessioni sotto budget e ripristino/self-test come job completo.

### Rilevamento persone: scelta del modello

Il predefinito è **YOLOX-Tiny**. Benchmark locale Windows del 2026-10-30,
ONNX Runtime CPU con 4 thread, arena disattivata, 2 warm-up e 5 esecuzioni
sul ritratto NASA incluso (4 persone); il paesaggio incluso controlla il caso
senza persone. I tempi includono preprocessing e postprocessing, senza caricare
il modello. Questi due riferimenti verificano la pipeline, non misurano
l'accuratezza generale su un dataset fotografico.

| Modello | File | Media CPU | RSS del processo dopo inferenza | Persone gruppo / paesaggio |
|---|---:|---:|---:|---:|
| YOLOX-Tiny FP32, input 416 | 20.2 MB | 491 ms | 65 MB (baseline 11 MB) | 4 / 0 |
| RT-DETR-R18 FP32, input 640 | 82.6 MB | 990 ms | 127 MB (baseline 10 MB) | 4 / 0 |

YOLOX-Tiny usa meno memoria e risulta circa due volte più veloce in questa
verifica CPU: è quindi preferito per il budget delle macchine da 8 GB.
RT-DETR è stato confrontato con la [conversione ONNX di RT-DETR-R18](https://huggingface.co/onnx-community/rtdetr_r18vd),
SHA-256 `11843b02455cc24009aed24d4c40db721b1093be5ccd6bbe7b9c441abb1d0558`.
Il file del confronto resta in `target/model-bench/` e non è distribuito.
Per riprodurre il confronto con i medesimi file:

```sh
cargo run -p pc-ai --example benchmark_detector -- yolox
cargo run -p pc-ai --example benchmark_detector -- rtdetr target/model-bench/rtdetr_r18vd.onnx
```

## Tag, identità e sequenza

- **Tag**: elenco dei tag della libreria con conteggi, rinomina, unione, eliminazione (annullabili) e colore; **set di tag** con tasti rapidi 1–9, eventualmente esclusivi. Il set attivo assegna i tag da tastiera in Libreria e Sequenza (0 rimuove i tag del set).
- **Identità**: persone note all'app (nome, nome breve per i file, alias, gruppo, soglia di somiglianza, volti di riferimento). In ogni libreria i gruppi di volti si collegano alle identità dal campo "Chi è?" di Persone; i volti di riferimento fanno riconoscere le stesse persone anche in una libreria nuova. Esportazione e importazione in un file JSON, con o senza volti.
- **Sequenza**: le foto di un ambito (libreria, catalogo, selezione) in ordine di istante reale, divise in sessioni. Per ogni gruppo di sorgente (dispositivo + fuso dichiarato) si possono impostare l'ora locale mostrata e uno scostamento dell'orologio, allineare due foto dello stesso momento e accettare i suggerimenti di fuso ricavati dal GPS; le foto senza data si collocano trascinandole. Le correzioni stanno nel catalogo: gli originali non vengono mai modificati.
- **Export della sequenza**: copie rinominate con un modello di nome (`{seq:04d}_{date}_{time}_{tag}` e altri token), cartelle `sequenza/` e `per-tag/<Tag>/` (hard link quando possibile), `manifest.csv` e due zip non compressi. Il JPEG è ricodificato a qualità 97 con crominanza 4:4:4, verificato con SSIM/PSNR e ricodificato fino a 100 se serve; profilo ICC, risoluzione e orientamento vengono preservati e i metadati corretti scritti nelle copie.

### Archivio dell'app (`app.db`)

Identità e set di tag non stanno in `catalog.db` ma in un archivio dell'app che **non segue** la cartella dati scelta nelle impostazioni, così sopravvivono a cambi di libreria:

| Sistema | Percorso |
|---|---|
| Windows | `%APPDATA%\photocat\photo-cat\config\app.db` |
| Linux | `~/.config/photo-cat/app.db` |
| `PHOTO_CAT_DATA_DIR` impostata | `<radice>/app/app.db` (installazioni portabili e prove) |

`app.db` contiene **dati biometrici**: i volti di riferimento delle identità (ritaglio allineato 112×112 ed embedding, mai le foto originali). Restano sul computer; la schermata Identità mostra dove si trova l'archivio, "Elimina tutti i dati dei volti" li cancella insieme ai volti della libreria (le identità restano con nome e alias) e l'esportazione delle identità permette di escluderli. Il pacchetto diagnostico riporta di `app.db` solo versione e conteggi.

## Struttura

| Percorso | Contenuto |
|---|---|
| `crates/pc-core` | tipi di dominio, errori con codici, configurazione |
| `crates/pc-store` | database SQLite (catalogo e archivio dell'app `app.db`), migrazioni, query, istante reale e sequenza |
| `crates/pc-diag` | log, registro errori, panic hook, pacchetto diagnostico |
| `crates/pc-governor` | rilevamento RAM/CPU e budget di memoria |
| `crates/pc-jobs` | motore dei job a fasi e progresso |
| `crates/pc-ingest` | scansione, metadati, EXIF (fuso e dispositivo), hash, miniature, decodifica a piena risoluzione |
| `crates/pc-models` | manifest, download e verifica dei modelli |
| `crates/pc-ai` | classificazione, persone, volti e volti di riferimento, geolocalizzazione e fuso dal GPS |
| `crates/pc-organize` | export CSV/JSON, organizzazione in cartelle, export della sequenza |
| `app/src-tauri` | shell desktop Tauri |
| `app/ui` | interfaccia React |
