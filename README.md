# PhotoCat

Gestore desktop locale per catalogare fotografie: libreria unica, cataloghi tematici, operazioni bulk, catalogazione automatica con reti neurali eseguite interamente sul proprio computer.

Stack: backend Rust (workspace `crates/pc-*`) + shell Tauri 2 (`app/src-tauri`) + UI React/TypeScript (`app/ui`).
Piano e requisiti: `openspec/changes/photo-catalog-foundation/`.

## Uso dell'app

**Primo avvio.** Tre passi, ognuno saltabile con "Lo faccio dopo": lingua e tema, la prima cartella di foto (scelta o trascinata; i file non vengono spostati né modificati), le funzioni AI con un riepilogo del computer in parole e il download dei modelli in background. Alla fine si apre la Libreria, dove le foto compaiono man mano che l'importazione procede, insieme alla lista **Primi passi** (importare una cartella, scaricare i modelli, dare un nome a una persona, creare un catalogo, usare un set di tag), che si spunta da sola anche se l'azione si fa da un'altra parte. Aiuto › "Rivedi l'introduzione" la ripropone senza cambiare nulla finché non si conferma un passo.

**Navigazione.** A sinistra i gruppi **Sfoglia** (Libreria, Sequenza), **Organizza** (Cataloghi, Persone, Tag) e **Importa ed esporta** (Importa, Esporta), e in fondo **Attività** e **Impostazioni**. Modelli AI e Diagnostica sono sezioni di Impostazioni. La barra superiore ha la ricerca dei comandi, l'indicatore delle attività in corso e il menu **Aiuto** (scorciatoie, introduzione, primi passi, suggerimenti, guida, segnala un problema, informazioni). La prima volta che si apre Persone, Cataloghi, Tag o Sequenza compare un breve suggerimento non bloccante.

**Scorciatoie principali** (l'elenco completo con `?` o Aiuto › Scorciatoie da tastiera):

| Tasto | Azione |
|---|---|
| Ctrl+K | palette dei comandi: moduli, azioni, persone, tag, cataloghi, luoghi |
| Ctrl+1…7 | apre i moduli nell'ordine della navigazione; Ctrl+, apre Impostazioni |
| Ctrl+B | riduce o espande la navigazione |
| Ctrl+Z | annulla l'ultima operazione annullabile |
| ? | foglio delle scorciatoie |
| 1–9, 0 | in Libreria e Sequenza, aggiunge il tag del set attivo alle foto selezionate (0 toglie i tag del set) |

**Impostazioni.** Sezioni Generale, Libreria e importazione, Riconoscimento AI, Modelli AI e sistema, Applicazioni esterne, Diagnostica e Avanzate, con una ricerca che porta al campo. Le preferenze si salvano appena modificate ("Salvato" accanto al campo). Soglie, tassonomia e percorso dei dati cambiano i risultati della libreria: restano in sospeso nella barra **Ripristina / Applica**, con l'anteprima dell'effetto. La sensibilità si regola su una scala "Più risultati ↔ Più precisi"; i valori numerici sono in Avanzate.

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

## Aggiornare e tornare a una versione precedente

**Dove stanno i dati.** Libreria e preferenze sono nella cartella utente dell'app, separata dalla cartella del programma:

| Contenuto | Windows | Linux |
|---|---|---|
| Cartella dati: `catalog.db`, `config.toml` (impostazioni, tema, lingua), miniature, modelli, log, `backups/` | `%APPDATA%\photocat\photo-cat\data` (o il percorso scelto nelle impostazioni) | `~/.local/share/photo-cat` |
| Cartella dell'app: `app.db` (identità, set di tag), `ui-prefs.json` (layout, ordinamento, miniature, pannelli) | `%APPDATA%\photocat\photo-cat\config` | `~/.config/photo-cat` |

**Installare sopra una versione esistente** (anche senza disinstallare) conserva tutto: l'installer sostituisce solo il programma. **Disinstallare e reinstallare** conserva comunque libreria, impostazioni e preferenze, anche spuntando "Elimina i dati dell'applicazione": quella casella cancella solo il profilo dell'interfaccia (`%LOCALAPPDATA%\com.photocat.app`), e al primo avvio le preferenze dell'interfaccia vengono ripristinate da `ui-prefs.json`. Per rimuovere davvero tutto, dopo la disinstallazione cancellare a mano la cartella `%APPDATA%\photocat` (Windows) oppure `~/.local/share/photo-cat` e `~/.config/photo-cat` (Linux).

All'avvio la nuova versione aggiorna il catalogo (`catalog.db`) se è di una versione precedente. Prima di farlo ne salva una copia in `backups/` dentro la cartella dati (`catalog-v<schema>-<data>.db`), insieme a una copia della configurazione (`config-v<schema>-<data>.toml`). L'archivio dell'app (`app.db`, identità e set di tag) è nuovo dalla 0.2.0 e viene creato al primo avvio; le persone con nome del catalogo diventano identità.

Una versione precedente non apre un catalogo aggiornato: lo segnala e non lo modifica. Non legge nemmeno la configurazione nuova, quindi usa i valori predefiniti, compresa la cartella dati. Per tornare a una versione precedente:

1. chiudere photo-cat;
2. nella cartella dati, sostituire `catalog.db` con la copia `catalog-v…db` più recente di `backups/`, rinominandola in `catalog.db`;
3. sostituire `config.toml` con la copia `config-v…toml` più recente;
4. installare la versione precedente.

Le modifiche fatte dopo l'aggiornamento non sono nella copia. Le voci di catalogazione manuali (senza descrizione) non sono supportate dalle versioni precedenti alla 0.2.0 (vedi "Voci di catalogazione").

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
Impostazioni › **Modelli AI e sistema** permette di verificare, installare un file locale,
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

## Tag, Persone e sequenza

- **Tag**: elenco dei tag della libreria con conteggi, rinomina, unione, eliminazione (annullabili) e colore; **set di tag** con tasti rapidi 1–9, eventualmente esclusivi. Il set attivo assegna i tag da tastiera in Libreria e Sequenza (0 rimuove i tag del set).
- **Persone**: un solo modulo (Identità non è più una voce separata nella navigazione), a tre colonne (elenco, volti, scheda), con le sezioni **Da rivedere** (gruppi automatici, con suggerimenti di unione), **Persone** (identità decise dall'utente: nome, nome breve per i file, alias, gruppo, soglia di somiglianza, volti di riferimento) e **Ignorati**. Un gruppo diventa persona solo quando gli si dà un nome con "Chi è?"; i volti di riferimento fanno riconoscere le stesse persone anche in una libreria nuova. Esportazione e importazione delle identità in un file JSON, con o senza volti.
  - **Ignorare**: non toglie foto dalla libreria né file dal disco. La persona sparisce da Da rivedere, dai filtri, dall'Inspector e dai suggerimenti; i nuovi volti simili vi confluiscono. Si ripristina dalla sezione Ignorati.
  - **Qualità dei volti**: ogni volto ha nitidezza del ritaglio, frontalità e dimensione (oltre alla confidenza del rilevatore). I volti sotto soglia sono scartati dal raggruppamento e dai suggerimenti, ma restano consultabili e recuperabili ("Volti scartati per qualità"). La nitidezza minima si regola con `ai.thresholds.face_min_sharpness` in Impostazioni › Riconoscimento AI (scala) o › Avanzate (valore).
  - **Apri in Persone**: dall'Inspector (persona, volto non riconosciuto o scartato), dai filtri e dalla Sequenza apre direttamente il gruppo o la persona di quel volto, con il volto evidenziato e un avviso per tornare alla foto.
- **Sequenza**: le foto di un ambito (libreria, catalogo, selezione) in ordine di istante reale, divise in sessioni. Per ogni gruppo di sorgente (dispositivo + fuso dichiarato) si possono impostare l'ora locale mostrata e uno scostamento dell'orologio, allineare due foto dello stesso momento e accettare i suggerimenti di fuso ricavati dal GPS; le foto senza data si collocano trascinandole. Le correzioni stanno nel catalogo: gli originali non vengono mai modificati.
- **Export della sequenza**: copie rinominate con un modello di nome (`{seq:04d}_{date}_{time}_{tag}` e altri token), cartelle `sequenza/` e `per-tag/<Tag>/` (hard link quando possibile), `manifest.csv` e due zip non compressi. Il JPEG è ricodificato a qualità 97 con crominanza 4:4:4, verificato con SSIM/PSNR e ricodificato fino a 100 se serve; profilo ICC, risoluzione e orientamento vengono preservati e i metadati corretti scritti nelle copie.

### Archivio dell'app (`app.db`)

Identità e set di tag non stanno in `catalog.db` ma in un archivio dell'app che **non segue** la cartella dati scelta nelle impostazioni, così sopravvivono a cambi di libreria:

| Sistema | Percorso |
|---|---|
| Windows | `%APPDATA%\photocat\photo-cat\config\app.db` |
| Linux | `~/.config/photo-cat/app.db` |
| `PHOTO_CAT_DATA_DIR` impostata | `<radice>/app/app.db` (installazioni portabili e prove) |

`app.db` contiene **dati biometrici**: i volti di riferimento delle identità (ritaglio allineato 112×112 ed embedding, mai le foto originali). Restano sul computer; il modulo Persone mostra dove si trova l'archivio, "Elimina tutti i dati dei volti" li cancella insieme ai volti della libreria (le identità restano con nome e alias) e l'esportazione delle identità permette di escluderli. Il pacchetto diagnostico riporta di `app.db` solo versione e conteggi.

## Voci di catalogazione

Le catalogazioni (tipo soggetto, interno/esterno, ambiente e i gruppi aggiunti dall'utente) hanno due tipi di voci:

- **automatiche**: hanno una descrizione in inglese per il riconoscimento (es. `a photo of boats`) e l'analisi CLIP le assegna da sola;
- **manuali**: senza descrizione. L'analisi non le assegna mai; le assegna l'utente, e compaiono comunque nei filtri, nell'Inspector e nei cataloghi smart.

Creare una voce:

- **durante la correzione**: nell'Inspector o nell'azione bulk "Imposta catalogazione", ultima opzione "Nuova voce…". Si scrive il nome e Invio conferma (Esc annulla). La voce nasce manuale e viene assegnata subito alla foto o alla selezione; l'azione bulk si annulla con "Annulla", e la voce resta. Un nome già presente nel gruppo, anche con maiuscole o accenti diversi, riusa la voce esistente;
- **in Impostazioni › Riconoscimento AI › Tassonomia**: "Aggiungi voce" in un gruppo e "Aggiungi gruppo" (es. "Evento", "Progetto"). La descrizione per il riconoscimento è facoltativa: scriverla rende la voce automatica. "Elimina" chiede conferma con il numero di foto che perdono la voce; l'eliminazione si annulla con "Annulla" nella barra bulk.

**Quando serve la rianalisi.** Aggiungere, rinominare, disattivare o eliminare voci e gruppi manuali non rende nessun file "da rianalizzare". Lo fanno solo le modifiche che cambiano l'analisi automatica: descrizioni, voci automatiche attivate, disattivate o eliminate, soglie. Impostazioni lo segnala prima di applicare. Con "Applica" le catalogazioni automatiche vengono ricalcolate dagli embedding già salvati, senza rielaborare le immagini.

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

Le impostazioni stanno in `config.toml`, nella cartella dati dell'app (Impostazioni › Generale › Percorso dati alternativo): Windows `%APPDATA%\photocat\photo-cat\data`, Linux `~/.local/share/photo-cat`. Si modificano dalla schermata Impostazioni; gli esempi sotto servono per configurazioni preparate a mano.

### Correzione del luogo

- **Una foto**: nell'Inspector il campo Luogo suggerisce i luoghi già presenti in libreria e le località del dataset geografico offline. Il luogo scelto diventa una correzione **manuale**: le analisi successive non lo modificano. "Ripristina automatico" torna al luogo dedotto da GPS o cartella.
- **Più foto**: azione bulk "Imposta luogo" (o "Ripristina luogo automatico") sulla selezione, oppure dal menu del tasto destro sulla griglia. Si annulla con "Annulla".
- **Un luogo intero**: nel facet Luogo, ✎ accanto al nome rinomina il luogo per tutte le foto che lo hanno (es. "D7000" → "Giappone"). Se il nuovo nome esiste già, i due luoghi si uniscono.
- **Elenco completo dei filtri**: ogni lista di filtro (luoghi, tag, catalogazioni, anni, formati) mostra le voci principali e "Mostra tutti (N)" con ricerca. ⧉ la apre in un pannello flottante, con ricerca e ordinamento per numero di foto o alfabetico, che resta aperto mentre si lavora sulla griglia e ricorda posizione e dimensione. I conteggi di una lista ignorano la sua selezione: scelto un luogo, gli altri restano selezionabili.
- **Filtro per data**: la lista Anno è una gerarchia. Scelto un anno compaiono sotto di esso i suoi mesi, scelto un mese i suoi giorni, ognuno con il numero di foto; mese e giorno valgono solo dentro l'anno scelto. Togliere la spunta risale di un livello (giorno → mese → anno → nessun filtro); il chip mostra la data scelta (es. "14 maggio 2023") e rimuoverlo toglie il filtro per data. Le date usano l'ora locale corretta di ogni foto; le foto senza data non compaiono.
- **Una cartella**: le sorgenti nella barra laterale si espandono nell'albero delle cartelle (con il numero di foto, sottocartelle incluse). Scegliendo una cartella la libreria si filtra, con o senza sottocartelle; "Seleziona tutto (filtro)" seleziona tutte le sue foto per un'azione bulk.

Nella deduzione dal percorso le cartelle dei dispositivi vengono saltate: marca o modello della fotocamera della foto (EXIF, es. `D7000` per una Nikon D7000) e nomi comuni come `Telefono`, `iPhone`, `WhatsApp`. Per `Viaggio Giappone\D7000\DSC_0001.NEF` il luogo è quindi "Viaggio Giappone". Altri nomi da saltare si aggiungono in Impostazioni › Libreria e importazione › "Cartelle da ignorare per il luogo"; appena salvate l'app propone, con un avviso, di ricalcolare i luoghi dedotti dalle cartelle (quelli manuali e da GPS non cambiano).

### Soglie AI

| Soglia | Predefinito | Effetto |
|---|---:|---|
| `ai.thresholds.person_detection` | 0,4 | confidenza minima di una persona rilevata (conteggio persone) |
| `ai.thresholds.face_detection` | 0,8 | confidenza minima di un volto usato per riconoscere le persone |
| `ai.thresholds.face_similarity` | 0,363 | somiglianza minima tra due volti della stessa persona (raggruppamento e associazione ai nomi; una soglia propria dell'identità ha la precedenza) |
| `ai.thresholds.face_min_sharpness` | 0,35 | nitidezza minima del ritaglio di un volto (i volti sotto soglia sono scartati dal raggruppamento, recuperabili in Persone) |
| soglia di gruppo (tassonomia) | 0,25 | punteggio minimo delle catalogazioni CLIP del gruppo |
| soglia di categoria (tassonomia) | quella del gruppo | punteggio minimo di una singola categoria |

Valori più alti danno meno catalogazioni automatiche, più sicure. La barra "Applica" delle Impostazioni mostra prima, con "Anteprima dell'effetto", l'effetto ("N foto cambiano catalogazioni", conteggi delle persone, volti esclusi, stima delle persone che cambiano raggruppamento; per la nitidezza minima, quanti volti verrebbero scartati). Con "Applica" parte l'attività **retune**, che ricalcola i risultati già salvati senza rieseguire i modelli: le catalogazioni dagli embedding CLIP, i conteggi dai riquadri delle persone, i raggruppamenti dai volti e la qualità dei volti. Le correzioni manuali restano. Le analisi conservano i risultati da 0,25 (persone) e 0,6 (volti) in su: si può scendere fino a questi valori senza rianalizzare. Le foto analizzate con versioni precedenti hanno come minimo 0,4 e 0,8; scendendo sotto, l'app lo segnala e propone la rianalisi dei soli file interessati. La soglia globale `ai.confidence_threshold` è solo il valore proposto ai gruppi nuovi e non modifica le soglie dei gruppi esistenti.

```toml
[ai]
confidence_threshold = 0.25
ignored_folder_names = ["Selezionate", "Da stampare"]

[ai.thresholds]
person_detection = 0.4
face_detection = 0.8
face_similarity = 0.363
face_min_sharpness = 0.35
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

## Foto simili

L'analisi raggruppa le foto quasi identiche (raffiche, scatti ripetuti, piccole variazioni della stessa inquadratura). Per ogni foto cerca le candidate più vicine nell'embedding CLIP, quelle dello stesso dispositivo scattate entro 10 secondi e le copie esatte, poi conferma la somiglianza confrontando le miniature (SSIM). Tutto avviene in locale e nessuna funzione dei gruppi cancella o modifica file sul disco.

- **Raggruppa foto simili** (barra della griglia, anche nei cataloghi): ogni gruppo diventa una sola miniatura, la foto principale, con il numero di foto; il numero apre il gruppo sul posto. Conteggi, filtri e "Seleziona tutto" restano sulle singole foto; un clic su un gruppo chiuso seleziona tutte le sue foto presenti nei risultati.
- **Foto principale**: proposta in base a nitidezza ed esposizione; dall'Inspector (sezione **Foto simili**) si sceglie un'altra foto, e la scelta resta anche dopo nuove analisi.
- **Correzioni** dall'Inspector: *Togli dal gruppo*, *Sciogli gruppo*; le correzioni valgono anche per le analisi successive.
- **Aiuto alla scelta**: *Solo foto con simili* (barra della griglia) e *Seleziona tutte tranne la principale* (barra delle azioni), da combinare con "Escludi dal catalogo", i tag o "Rimuovi dalla libreria".
- **Soglia** in Impostazioni › Riconoscimento AI o › Avanzate (*Somiglianza dei gruppi di foto simili*, predefinita 0,80): l'anteprima indica quanti gruppi si formerebbero, e con "Applica" i gruppi vengono ricalcolati senza rianalizzare le foto.

L'analisi dei gruppi fa parte dell'analisi AI ed è incrementale: dopo un import vengono confrontate solo le foto nuove. Per una libreria analizzata prima di questa funzione, Impostazioni › Riconoscimento AI propone **Analizza i gruppi**.

## Librerie grandi

L'app è pensata per librerie da 100.000 foto e oltre:

- la griglia tiene in memoria solo le pagine vicine alla vista (al massimo 5 da 300 foto); scorrendo lontano le altre vengono scaricate e ricaricate quando si torna indietro;
- "Seleziona tutto" e "Inverti" non caricano l'elenco delle foto: la selezione è "tutti i risultati del filtro" più le eccezioni (Ctrl+clic), e le azioni in blocco si applicano solo alle foto presenti al momento della selezione;
- conteggi, filtri e miniature non bloccano mai la finestra: una richiesta superata da un filtro più recente viene interrotta;
- gli errori per file arrivano a gruppi: Impostazioni › Diagnostica › **Riepilogo per codice** mostra, per esempio, "DECODE_FAILED — 1.240 file"; aprendo la riga si vede l'elenco dei file con il motivo (file danneggiato o incompleto, variante non supportata, RAW senza anteprima, immagine oltre i limiti);
- immagini non JPEG oltre 100 megapixel non vengono decodificate (miniatura assente, errore "immagine oltre i limiti"); per i JPEG che il decodificatore principale non legge si usa un decodificatore di riserva;
- se la memoria del sistema diventa critica, decodifica e analisi si fermano ("in pausa per memoria" nella schermata Job) e riprendono da sole; se l'interfaccia supera 1,5 GB, l'import rallenta e l'interfaccia libera le cache.

### Dopo una chiusura anomala

Al riavvio un avviso indica che la sessione precedente non si è chiusa regolarmente, con l'ora dell'ultima attività e la memoria in quel momento. Se si chiude solo l'interfaccia (processo WebView), l'app la ricarica e i job continuano; dopo tre chiusure in pochi minuti i job vanno in pausa e compare una finestra con il percorso dei log.

Dove guardare (cartella dati, vedi Impostazioni › Generale):

| File | Contenuto |
|---|---|
| `logs/photo-cat.<data>.log` | log dell'app (JSON, una riga per evento) |
| `logs/session.last-abnormal.json` | ultima sessione non chiusa: ora, memoria, job attivi |
| `logs/crash/native.log`, `logs/crash/native-*.dmp` | abort nativi (eccezioni, segnali) e minidump (Windows) |
| `%LOCALAPPDATA%\com.photocat.app\EBWebView\Crashpad\reports\*.dmp` | dump di crash di WebView2 (Windows) |

Impostazioni › Diagnostica › **Esporta pacchetto** (anche da Aiuto › Segnala un problema) include questi file (al massimo 3 dump per tipo e 20 MB in tutto, solo quelli successivi all'ultima chiusura regolare).

### Test di carico

`scripts/loadtest/` contiene la prova ripetibile a 100.000 foto, che non modifica mai la cartella delle foto:

```powershell
# libreria di prova con collegamenti fisici alle foto di D:\Foto (in sola lettura)
powershell -ExecutionPolicy Bypass -File scripts\loadtest\make-library.ps1 -Source D:\Foto -Target D:\photo-cat-loadtest
# app con cartella dati separata, poi aggiungere la sorgente D:\photo-cat-loadtest\library
$env:PHOTO_CAT_DATA_DIR = 'D:\photo-cat-loadtest\data'; .\photo-cat.exe
# memoria dell'app e della WebView ogni 5 s in un CSV (long task: livello di log "debug")
powershell -ExecutionPolicy Bypass -File scripts\loadtest\monitor.ps1 -DataDir D:\photo-cat-loadtest\data
# pulizia (solo D:\photo-cat-loadtest)
powershell -ExecutionPolicy Bypass -File scripts\loadtest\clean.ps1
```

Su Linux: `scripts/loadtest/make-library.sh SORGENTE DESTINAZIONE 100000 2000` (e `--clean`). Gli script verificano prima e dopo che numero e date dei file della sorgente siano invariati. Per classificare le decodifiche fallite di una cartella senza importarla: `cargo run --release -p pc-ingest --features heif --example classify_decode -- D:\Foto`.

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

### Esclusioni dai cataloghi

Da un catalogo manuale o smart puoi escludere le foto dalla barra delle azioni,
dal menu contestuale, dall'Inspector o premendo Canc sulla selezione nella griglia.
L'operazione non modifica i file e non rimuove le foto dalla libreria; l'avviso
mostra Annulla per otto secondi.

Apri **Escluse (N)** nell'intestazione del catalogo per rivedere le foto, ordinate
per data di esclusione, e reinserirle singolarmente o in gruppo (anche con Invio).
Negli smart vengono mostrate inizialmente solo le escluse che soddisfano ancora
le regole: l'interruttore permette di vederle tutte. Le esclusioni persistono
anche se cambiano i metadati o viene ripetuta l'analisi. Riaggiungere una foto a
un catalogo manuale cancella la sua esclusione. I conteggi e le regole di
appartenenza ad altri cataloghi rispettano le esclusioni.

### Cestino, Duplicati e Qualità insufficiente

Tre **cataloghi di sistema**, creati automaticamente e non eliminabili né rinominabili.
Le foto che contengono spariscono da Libreria, cataloghi, filtri e conteggi, e compaiono
solo aprendo quel catalogo; da lì **Ripristina** le riporta indietro. I file sul disco
non vengono mai spostati né cancellati.

- **Cestino**: "Sposta nel cestino" dalla barra delle azioni (Altro) o Canc sulla
  selezione fuori da un catalogo. L'avviso permette di annullare.
- **Duplicati**: "Metti da parte i duplicati" (scheda del catalogo o palette): per ogni
  file presente più volte con lo stesso contenuto resta in Libreria la copia importata
  per prima, le altre vanno in "Duplicati".
- **Qualità insufficiente**: "Controlla la qualità" avvia un'attività che misura la
  nitidezza sulle miniature e la risoluzione, e sposta qui le foto molto sfocate o sotto
  0,3 megapixel. Ogni foto viene misurata una volta sola, quindi una foto ripristinata
  non ci torna al controllo successivo.

La 0.3.0 aggiorna il catalogo allo schema 10 (cataloghi di sistema e misure di qualità);
la copia di sicurezza in `backups/` permette di tornare alla versione precedente.
