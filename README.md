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
| `make test-linux` | da Windows o macOS, con Docker: ripete in un container Ubuntu 22.04 i passi della CI Linux (build UI, fmt, clippy, test Rust, `.deb` e `.tar.gz`) sullo stato corrente del repository, file nuovi compresi se in stage; `SKIP_PACKAGE=1` salta i pacchetti |
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

## Voci di catalogazione

Le catalogazioni (tipo soggetto, interno/esterno, ambiente e i gruppi aggiunti dall'utente) hanno due tipi di voci:

- **automatiche**: hanno una descrizione in inglese per il riconoscimento (es. `a photo of boats`) e l'analisi CLIP le assegna da sola;
- **manuali**: senza descrizione. L'analisi non le assegna mai; le assegna l'utente, e compaiono comunque nei filtri, nell'Inspector e nei cataloghi smart.

Creare una voce:

- **durante la correzione**: nell'Inspector o nell'azione bulk "Imposta catalogazione", ultima opzione "Nuova voce…". Si scrive il nome e Invio conferma (Esc annulla). La voce nasce manuale e viene assegnata subito alla foto o alla selezione; l'azione bulk si annulla con "Annulla", e la voce resta. Un nome già presente nel gruppo, anche con maiuscole o accenti diversi, riusa la voce esistente;
- **in Impostazioni › Tassonomia**: "Aggiungi voce" in un gruppo e "Aggiungi gruppo" (es. "Evento", "Progetto"). La descrizione per il riconoscimento è facoltativa: scriverla rende la voce automatica. "Elimina" chiede conferma con il numero di foto che perdono la voce; l'eliminazione si annulla con "Annulla" nella barra bulk.

**Quando serve la rianalisi.** Aggiungere, rinominare, disattivare o eliminare voci e gruppi manuali non rende nessun file "da rianalizzare". Lo fanno solo le modifiche che cambiano l'analisi automatica: descrizioni, voci automatiche attivate, disattivate o eliminate, soglie. Impostazioni lo segnala prima del salvataggio. Al salvataggio le catalogazioni automatiche vengono ricalcolate dagli embedding già salvati, senza rielaborare le immagini.

**Tornare a una versione precedente.** Le versioni precedenti rifiutano le tassonomie con voci senza descrizione. Prima di installarne una vanno eliminati le voci e i gruppi manuali, oppure va aggiunta una descrizione a ogni voce manuale.

## Gestione delle importazioni

Ogni sorgente ha un menu "…" nella barra laterale della Libreria (e gli stessi comandi nella pagina Import). Le azioni non toccano mai i file sul disco.

- **Aggiorna**: rilegge la sorgente. Al termine il riepilogo "Ultimo aggiornamento" (pagina Import e schermata Job) indica i file nuovi, modificati, spostati o rinominati (riconosciuti dall'impronta del contenuto), mancanti e tornati presenti; un clic su un gruppo apre la Libreria solo su quei file. Un file con contenuto cambiato (es. ritagliato con un altro programma) riceve una nuova miniatura e torna "da analizzare": l'analisi automatica viene rifatta, mentre correzioni manuali, tag, cataloghi, luogo manuale e volti assegnati a mano restano. I volti assegnati a mano potrebbero non corrispondere più all'immagine ritoccata: vanno ricontrollati in Persone.
- **Aggiorna cartella…**: aggiorna solo una sottocartella della sorgente (anche con ↻ accanto alla cartella nell'albero). Solo i file dentro quella cartella possono diventare mancanti.
- **Ricollega mancanti…**: dopo aver spostato o rinominato una cartella fuori dal programma, si indica la nuova posizione. Il percorso precedente è proposto (cartella comune dei mancanti). I file si ritrovano per percorso relativo e poi per impronta del contenuto, con tutte le catalogazioni; le copie identiche ambigue restano mancanti. Se la nuova cartella non è in nessuna sorgente l'app propone di aggiungerla come sorgente o, se tutta la sorgente è stata ritrovata, di spostare lì la sorgente.
- **Rimuovi mancanti…**: toglie dalla libreria, dopo una conferma con il numero di file, tutti i mancanti della sorgente.
- **Rimuovi dalla libreria…**: prima della conferma mostra quante foto verranno tolte, quante hanno correzioni manuali o tag, i cataloghi manuali che perderanno foto, le persone con nome che resteranno senza foto e i job in corso sulla sorgente. Dopo la conferma i job vengono annullati, poi si cancellano dal catalogo le foto con miniature, embedding e volti; i gruppi di volti senza nome rimasti vuoti spariscono, le persone con nome restano (con 0 foto). La rimozione non si annulla: reimportando la cartella le foto tornano, ma senza correzioni manuali.

Anche "Rimuovi dalla libreria" sulla selezione cancella le miniature non più usate. Al primo avvio di questa versione l'app ripulisce una sola volta, in background, le miniature rimaste da rimozioni precedenti.

## Luogo, cartelle, "Apri con" e soglie AI

Le impostazioni stanno in `config.toml`, nella cartella dati dell'app (Impostazioni → Percorso dati): Windows `%APPDATA%\photocat\photo-cat\data`, Linux `~/.local/share/photo-cat`. Si modificano dalla schermata Impostazioni; gli esempi sotto servono per configurazioni preparate a mano.

### Correzione del luogo

- **Una foto**: nell'Inspector il campo Luogo suggerisce i luoghi già presenti in libreria e le località del dataset geografico offline. Il luogo scelto diventa una correzione **manuale**: le analisi successive non lo modificano. "Ripristina automatico" torna al luogo dedotto da GPS o cartella.
- **Più foto**: azione bulk "Imposta luogo" (o "Ripristina luogo automatico") sulla selezione, oppure dal menu del tasto destro sulla griglia. Si annulla con "Annulla".
- **Un luogo intero**: nel facet Luogo, ✎ accanto al nome rinomina il luogo per tutte le foto che lo hanno (es. "D7000" → "Giappone"). Se il nuovo nome esiste già, i due luoghi si uniscono.
- **Elenco completo dei filtri**: ogni lista di filtro (luoghi, tag, catalogazioni, anni, formati) mostra le voci principali e "Mostra tutti (N)" con ricerca. ⧉ la apre in un pannello flottante, con ricerca e ordinamento per numero di foto o alfabetico, che resta aperto mentre si lavora sulla griglia e ricorda posizione e dimensione. I conteggi di una lista ignorano la sua selezione: scelto un luogo, gli altri restano selezionabili.
- **Una cartella**: le sorgenti nella barra laterale si espandono nell'albero delle cartelle (con il numero di foto, sottocartelle incluse). Scegliendo una cartella la libreria si filtra, con o senza sottocartelle; "Seleziona tutto (filtro)" seleziona tutte le sue foto per un'azione bulk.

Nella deduzione dal percorso le cartelle dei dispositivi vengono saltate: marca o modello della fotocamera della foto (EXIF, es. `D7000` per una Nikon D7000) e nomi comuni come `Telefono`, `iPhone`, `WhatsApp`. Per `Viaggio Giappone\D7000\DSC_0001.NEF` il luogo è quindi "Viaggio Giappone". Altri nomi da saltare si aggiungono in Impostazioni → "Cartelle da ignorare per il luogo"; dopo il salvataggio l'app propone di ricalcolare i luoghi dedotti dalle cartelle (quelli manuali e da GPS non cambiano).

### Soglie AI

| Soglia | Predefinito | Effetto |
|---|---:|---|
| `ai.thresholds.person_detection` | 0,4 | confidenza minima di una persona rilevata (conteggio persone) |
| `ai.thresholds.face_detection` | 0,8 | confidenza minima di un volto usato per riconoscere le persone |
| `ai.thresholds.face_similarity` | 0,363 | somiglianza minima tra due volti della stessa persona (raggruppamento e associazione ai nomi; una soglia propria dell'identità ha la precedenza) |
| soglia di gruppo (tassonomia) | 0,25 | punteggio minimo delle catalogazioni CLIP del gruppo |
| soglia di categoria (tassonomia) | quella del gruppo | punteggio minimo di una singola categoria |

Valori più alti danno meno catalogazioni automatiche, più sicure. La sezione "Soglie AI" mostra prima l'effetto ("N foto cambiano catalogazioni", conteggi delle persone, volti esclusi, stima delle persone che cambiano raggruppamento). Al salvataggio parte il job **retune**, che ricalcola i risultati già salvati senza rieseguire i modelli: le catalogazioni dagli embedding CLIP, i conteggi dai riquadri delle persone, i raggruppamenti dai volti. Le correzioni manuali restano. Le analisi conservano i risultati da 0,25 (persone) e 0,6 (volti) in su: si può scendere fino a questi valori senza rianalizzare. Le foto analizzate con versioni precedenti hanno come minimo 0,4 e 0,8; scendendo sotto, l'app lo segnala e propone la rianalisi dei soli file interessati. La soglia globale `ai.confidence_threshold` è solo il valore proposto ai gruppi nuovi e non modifica le soglie dei gruppi esistenti.

```toml
[ai]
confidence_threshold = 0.25
ignored_folder_names = ["Selezionate", "Da stampare"]

[ai.thresholds]
person_detection = 0.4
face_detection = 0.8
face_similarity = 0.363
```

### Applicazioni esterne ("Apri con")

Il doppio clic su una miniatura apre la foto con l'app predefinita del sistema (`double_click = "system"`), con un'applicazione configurata (il suo `id`) o non fa nulla (`"none"`). Il tasto destro (o il tasto Menu / Maiusc+F10) apre il menu della griglia: "Apri con ▸", "Apri con l'app predefinita", "Mostra nella cartella", "Imposta luogo…" e altre azioni, sulla selezione o sulla sola miniatura cliccata. Negli argomenti `{file}` è sostituito dal percorso di ogni foto, come argomento separato; senza `{file}` i percorsi vengono aggiunti in fondo. Le applicazioni sono avviate senza shell intermedia, quindi spazi, `&` e virgolette nei percorsi non sono un problema. Con più di 100 file l'app chiede conferma. Gli eseguibili devono esistere al salvataggio.

Windows (nei percorsi TOML con `\` usare apici singoli):

```toml
[ui]
double_click = "lightroom"

[[external_apps]]
id = "lightroom"
name = "Lightroom Classic"
path = 'C:\Program Files\Adobe\Adobe Lightroom Classic\Lightroom.exe'
args = ["{file}"]

[[external_apps]]
id = "xnview"
name = "XnView MP"
path = 'C:\Program Files\XnViewMP\xnviewmp.exe'
args = []
```

macOS (un bundle `.app` viene avviato con `open -a`):

```toml
[[external_apps]]
id = "affinity"
name = "Affinity Photo"
path = "/Applications/Affinity Photo 2.app"
args = []
```

Linux:

```toml
[ui]
double_click = "gimp"

[[external_apps]]
id = "gimp"
name = "GIMP"
path = "/usr/bin/gimp"
args = ["{file}"]

[[external_apps]]
id = "darktable"
name = "darktable"
path = "/usr/bin/darktable"
args = ["--library", ":memory:", "{file}"]
```

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
