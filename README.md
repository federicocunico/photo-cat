# PhotoCat

Gestore desktop locale per catalogare fotografie: libreria unica, cataloghi tematici, operazioni bulk, catalogazione automatica con reti neurali eseguite interamente sul proprio computer.

Stack: backend Rust (workspace `crates/pc-*`) + shell Tauri 2 (`app/src-tauri`) + UI React/TypeScript (`app/ui`).
Piano e requisiti: `openspec/changes/photo-catalog-foundation/`.

## Uso dell'app

**Primo avvio.** Tre passi, ognuno saltabile con "Lo faccio dopo": lingua e tema, la prima cartella di foto (scelta o trascinata; i file non vengono spostati né modificati), le funzioni AI con un riepilogo del computer in parole e il download dei modelli in background. Alla fine si apre la Libreria, dove le foto compaiono man mano che l'importazione procede, insieme alla lista **Primi passi** (importare una cartella, scaricare i modelli, dare un nome a una persona, creare un catalogo, usare un set di tag), che si spunta da sola anche se l'azione si fa da un'altra parte. Aiuto › "Rivedi l'introduzione" la ripropone senza cambiare nulla finché non si conferma un passo.

**Navigazione.** A sinistra i gruppi **Sfoglia** (Libreria, Sequenza), **Organizza** (Cataloghi, Persone, Tag, Analisi AI) e **Importa ed esporta** (Importa, Esporta, Varianti RAW), e in fondo **Attività** e **Impostazioni**. Modelli AI e Diagnostica sono sezioni di Impostazioni. Ogni modulo ha in alto la stessa intestazione (titolo, schede e comandi della pagina) e scorre solo il suo contenuto. La prima volta che si apre Persone, Cataloghi, Tag o Sequenza compare un breve suggerimento in linea, che non copre i comandi.

**Barra dei menu.** Accanto al nome dell'app ci sono i menu **File** (importare, esportare, aprire una cartella di RAW o un progetto FujiView, la cartella dei dati, Impostazioni, Esci), **Modifica** (annulla, selezione, tag del set attivo, valutazione, cataloghi, Cestino), **Vista** (navigazione compatta, pannelli, Inspector, miniature, foto simili, mappa, tema, schermo intero), **Vai** (i moduli e la ricerca dei comandi), **Strumenti** (rianalisi, duplicati, qualità, modelli AI, diagnostica) e **Aiuto** (guida, scorciatoie, primi passi, introduzione, suggerimenti, segnala un problema, copia le informazioni di sistema, Informazioni su photo-cat). Le voci hanno lo stesso nome e la stessa scorciatoia della ricerca dei comandi; una voce che ora non si può usare resta visibile, disattivata, e dice perché. Alt (premuto e rilasciato da solo) o F10 portano il focus sul primo menu: frecce per spostarsi, Invio per scegliere, Esc per tornare dove si era, la lettera sottolineata per aprire un menu. Su macOS gli stessi menu sono nella barra del sistema. **Informazioni su photo-cat** mostra versione, build (commit, data, configurazione), sistema, motore della finestra, acceleratore AI, autori, licenza e le licenze di tutti i componenti di terze parti, con una ricerca; "Copia informazioni" mette negli appunti i dati per una segnalazione, senza il percorso dei dati. Le stesse informazioni sono in Impostazioni › Informazioni.

**Scorciatoie principali** (l'elenco completo con `?` o Aiuto › Scorciatoie da tastiera, e accanto alle voci dei menu):

| Tasto | Azione |
|---|---|
| Ctrl+K | palette dei comandi: moduli, azioni, persone, tag, cataloghi, luoghi |
| Ctrl+1…9 | apre i moduli nell'ordine della navigazione (Ctrl+9: Varianti RAW); Ctrl+, apre Impostazioni |
| Ctrl+B | riduce o espande la navigazione |
| Ctrl+Z | annulla l'ultima operazione annullabile |
| Ctrl+O, Ctrl+E | importa una cartella, esporta |
| Alt, F10 | porta il focus sulla barra dei menu |
| F11 | schermo intero |
| ? | foglio delle scorciatoie |
| 1–9, 0 | in Libreria e Sequenza, aggiunge il tag del set attivo alle foto selezionate (0 toglie i tag del set) |
| Maiusc+1…9, Invio, 1–5, 0, Spazio | in Varianti RAW, sceglie la variante migliore e dà le stelle (vedi la sezione dedicata) |

**Selezione** (come in un gestore di file, uguale in Libreria, cataloghi, Sequenza, Persone e Cataloghi):

| Gesto | Effetto |
|---|---|
| Clic | seleziona solo quell'elemento |
| Ctrl+clic, Maiusc+clic, Ctrl+Maiusc+clic | aggiunge o toglie, seleziona l'intervallo dall'ancora, aggiunge l'intervallo |
| Clic su uno spazio vuoto, Esc | deseleziona |
| Frecce, Home/Fine, Pag↑/Pag↓ | spostano il focus e selezionano l'elemento raggiunto |
| Maiusc+frecce | estendono la selezione; Ctrl+frecce spostano solo il focus, Ctrl+Spazio aggiunge o toglie l'elemento con il focus |
| Ctrl+A, Ctrl+Maiusc+I | seleziona tutto (anche gli elementi non ancora caricati), inverte la selezione |
| Clic destro | su un elemento selezionato agisce su tutta la selezione; su uno non selezionato seleziona solo quello |

Con più raccolte nella stessa schermata (in Persone: elenco dei gruppi e volti) le scorciatoie agiscono sul riquadro dell'ultimo clic, segnato da un bordo superiore.

**Impostazioni.** Sezioni Generale, Libreria e importazione, Riconoscimento AI, Modelli AI e sistema, Applicazioni esterne, Diagnostica e Avanzate, con una ricerca che porta al campo. Le preferenze si salvano appena modificate ("Salvato" accanto al campo). Soglie, tassonomia e percorso dei dati cambiano i risultati della libreria: restano in sospeso nella barra **Ripristina / Applica**, con l'anteprima dell'effetto. La sensibilità si regola su una scala "Più risultati ↔ Più precisi"; i valori numerici sono in Avanzate.

## Prerequisiti

- Rust stable ≥ 1.90 (`rustup`), con `rustfmt` e `clippy`
- Node.js ≥ 20 e npm
- Windows: Microsoft Edge WebView2 Runtime (preinstallato su Windows 11) e Build Tools di Visual Studio (C++)
  - lo sviluppo dei RAF dalla fotocamera (modulo Varianti RAW) esiste solo su Windows; il resto del modulo funziona su ogni sistema
- Linux (Debian/Ubuntu): `sudo apt install build-essential libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`
  - per le foto HEIC: `libheif1` ≥ 1.20 e `libheif-plugin-libde265` (senza, gli HEIC vengono importati con i soli metadati)
- macOS (Apple Silicon): Xcode Command Line Tools
  - per le foto HEIC: `brew install libheif` (senza, gli HEIC vengono importati con i soli metadati)

## Sviluppo

Scorciatoie (GNU Make: su Windows da cmd o Git Bash, su Linux o macOS dal terminale):

| Comando | Cosa fa |
|---|---|
| `make dev` (o `make run`) | avvio locale in debug: Vite + finestra Tauri con ricarica |
| `make build` | test di build: setup NSIS in `target/release/bundle/nsis` su Windows; `.deb` e `.tar.gz` con SHA-256 in `target/release/dist` su Linux; `.dmg` con SHA-256 in `target/release/dist` su macOS Apple Silicon (solo locale, non in CI) |
| `make test` | fmt, clippy, test Rust e UI (i test non girano in CI) |
| `make test-linux` | da Windows o macOS, con Docker: ripete in un container Ubuntu 22.04 i passi della CI Linux (build UI, fmt, clippy, test Rust, `.deb` e `.tar.gz`) sullo stato corrente del repository, file nuovi compresi se in stage; `SKIP_PACKAGE=1` salta i pacchetti |
| `make release` | crea e pubblica il tag `v<versione>` di `tauri.conf.json`: la GitHub Action pubblica la release con installer Windows, pacchetti Linux e SHA-256 |
| `make publish-mac` | solo su macOS Apple Silicon: allega il DMG e lo SHA-256 di `target/release/dist` alla release GitHub `v<versione>` già esistente (se manca il DMG lo ricompila; `SKIP_BUILD=1` lo evita e fallisce). Serve `gh` autenticato |

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

In entrambi i casi la decodifica HEIC usa la libheif di sistema (vedi Prerequisiti). L'AI gira su CPU: il runtime ONNX per Linux è incluso. Lo sviluppo dei RAF dalla fotocamera (Varianti RAW) è solo per Windows.

Verifica del download: `sha256sum -c <file>.sha256` nella cartella del file.

## Installazione su macOS (Apple Silicon)

La build locale (`make build` su un Mac arm64) produce in `target/release/dist/`:

- `photo-cat_<versione>_aarch64.dmg` — si apre il disco e si trascina photo-cat in Applicazioni.
- `photo-cat_<versione>_aarch64.dmg.sha256` — verifica con `shasum -a 256 -c photo-cat_<versione>_aarch64.dmg.sha256`.

Per pubblicarli sulla release GitHub della stessa versione (dopo `make release` / CI Windows+Linux): `make publish-mac`. Non passa da CI.

Lo sviluppo dei RAF dalla fotocamera non è disponibile su macOS (né su Linux): il resto del modulo Varianti RAW funziona.

Il DMG usa una firma ad-hoc (non Developer ID / notarizzazione). Al primo avvio macOS chiede di autorizzare un’app di uno sviluppatore non identificato: tasto destro › Apri, oppure Impostazioni di sistema › Privacy e sicurezza › Apri comunque. Se compare «è danneggiata», è una build senza firma del bundle: rifare `make build` / `make publish-mac` con la versione corrente.

La decodifica HEIC usa la libheif di sistema (vedi Prerequisiti). Il runtime ONNX per macOS arm64 è incluso; su Apple Silicon è selezionabile anche CoreML.

## Aggiornare e tornare a una versione precedente

**Dove stanno i dati.** Libreria e preferenze sono nella cartella utente dell'app, separata dalla cartella del programma:

| Contenuto | Windows | Linux | macOS |
|---|---|---|---|
| Cartella dati: `catalog.db`, `config.toml` (impostazioni, tema, lingua), miniature, modelli, log, `backups/` | `%APPDATA%\photocat\photo-cat\data` (o il percorso scelto nelle impostazioni) | `~/.local/share/photo-cat` | `~/Library/Application Support/com.photocat.photo-cat` |
| Cartella dell'app: `app.db` (identità, set di tag), `ui-prefs.json` (layout, ordinamento, miniature, pannelli) | `%APPDATA%\photocat\photo-cat\config` | `~/.config/photo-cat` | `~/Library/Preferences/com.photocat.photo-cat` |

**Installare sopra una versione esistente** (anche senza disinstallare) conserva tutto: l'installer sostituisce solo il programma. **Disinstallare e reinstallare** conserva comunque libreria, impostazioni e preferenze, anche spuntando "Elimina i dati dell'applicazione": quella casella cancella solo il profilo dell'interfaccia (`%LOCALAPPDATA%\com.photocat.app`), e al primo avvio le preferenze dell'interfaccia vengono ripristinate da `ui-prefs.json`. Per rimuovere davvero tutto, dopo la disinstallazione cancellare a mano la cartella `%APPDATA%\photocat` (Windows), oppure `~/.local/share/photo-cat` e `~/.config/photo-cat` (Linux), oppure `~/Library/Application Support/com.photocat.photo-cat` e `~/Library/Preferences/com.photocat.photo-cat` (macOS).

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

I modelli e il runtime ONNX per Windows, Linux e macOS Apple Silicon sono inclusi in `assets/`, nell'installer e nei pacchetti;
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
- **Persone**: un solo modulo a tre colonne (elenco, volti, scheda). Le schede **Da rivedere** (gruppi automatici, cioè "persone senza nome"), **Persone** (con nome) e **Ignorati** stanno nell'intestazione della pagina, insieme a "Nuova persona". Un gruppo diventa persona solo quando gli si dà un nome con "Chi è?"; i volti di riferimento fanno riconoscere le stesse persone anche in una libreria nuova.
  - **Elenco**: ricerca e menu "Ordina e filtra" (ordine, poche foto, cerchia). Un clic apre la voce, Ctrl/Maiusc+clic o la casella della riga la aggiungono alla selezione; con due o più voci selezionate, in fondo all'elenco compaiono "Unisci a…" e "Ignora".
  - **Volti**: sopra la griglia ci sono solo "Ordina" e "Vista". Con dei volti selezionati compaiono "Sposta in…", "Non è questa persona" e "Altro" (nuovo gruppo, copertina, rimozione). "Sposta in…" e "Unisci a…" elencano le persone (in ordine alfabetico) e poi i gruppi senza nome, con la copertina. Spostamenti e unioni tra gruppi avvengono subito e si annullano dall'avviso (Ctrl+Z); l'unione di persone con nome mostra prima un'anteprima.
  - **Scheda del gruppo**: se il gruppo somiglia a una persona o a un altro gruppo, in cima compare la domanda "È Giulia?" o "È la stessa persona?", con le due copertine, la somiglianza in parole e Sì/No.
  - **Scheda della persona**: copertina, "Mostra in Libreria" e il menu ⋯ (Unisci a…, Esporta, Elimina persona…). I dettagli (nome, nome nei file esportati, cerchia, altri nomi, riconoscimento come scala "Più risultati ↔ Più precisi") si salvano appena modificati. I volti nella libreria e i volti di riferimento sono due schede della colonna dei volti.
  - **Esportazione e importazione** delle persone in un file JSON, con o senza volti di riferimento (tutte da Persone, una sola dal suo menu ⋯).
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
| macOS | `~/Library/Preferences/com.photocat.photo-cat/app.db` |
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

Le impostazioni stanno in `config.toml`, nella cartella dati dell'app (Impostazioni › Generale › Percorso dati alternativo): Windows `%APPDATA%\photocat\photo-cat\data`, Linux `~/.local/share/photo-cat`, macOS `~/Library/Application Support/com.photocat.photo-cat`. Si modificano dalla schermata Impostazioni; gli esempi sotto servono per configurazioni preparate a mano.

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

## Varianti RAW (Fujifilm X-T5)

Il modulo **Varianti RAW** (per la Fujifilm X-T5, come dice il sottotitolo della pagina) sta nel gruppo "Importa ed esporta" (Ctrl+9; anche dalla ricerca dei comandi, Ctrl+K). Dopo uno shooting con la X-T5:

1. apre la cartella dei RAF **senza importarla** in libreria;
2. mette a confronto le varianti di simulazione pellicola (le ricette C1–C7 della fotocamera e "Come scattato") di ogni scatto;
3. permette di scegliere la variante migliore e dare da 0 a 5 stelle;
4. esporta le scelte in un **nuovo catalogo**.

La guida in app (Aiuto › Guida, o il pulsante **Guida** nella pagina del modulo) ha la stessa panoramica.

### Cartelle riconosciute

| Forma | Contenuto |
|---|---|
| Solo RAF | file `.RAF` al primo livello; le varianti si possono generare dalla fotocamera (solo Windows) |
| RAF con varianti | file `.RAF` e, accanto, `FujiView varianti/<preset>/` con i JPEG già sviluppati |
| Una sottocartella per preset | ogni sottocartella è un preset e contiene JPEG o TIFF con gli stessi nomi dei file (l'esportazione di FujiView) |

La cartella con più scatti diversi fa da guida all'elenco degli scatti, in ordine naturale (`DSCF2` prima di `DSCF10`). Una variante mancante in uno scatto compare come "Variante non disponibile", senza togliere lo scatto. Una cartella senza foto da rivedere viene rifiutata con un messaggio che spiega le forme supportate.

Uno scatto senza varianti mostra l'**Anteprima RAW**, l'anteprima contenuta nel RAF: serve per confrontare e valutare, ma **non si può scegliere come variante migliore né esportare**. Va prima sviluppato.

### Sviluppo dalla fotocamera (solo Windows)

"Sviluppa dalla fotocamera…" invia i RAF alla X-T5 collegata con il cavo USB e le fa produrre un JPEG per ogni ricetta scelta: i banchi personalizzati **C1–C7** (con il nome che hanno nella fotocamera) e **Come scattato**, cioè le impostazioni registrate nel RAF. Si sviluppano tutti gli scatti della cartella o solo quelli selezionati nell'elenco.

Preparare la fotocamera:

1. nel menu `MENU > SET UP > CONNECTION SETTING > USB MODE` scegliere **USB RAW CONV./BACKUP RESTORE** (il nome in italiano può cambiare con il firmware);
2. disattivare lo spegnimento automatico (`AUTO POWER OFF`), così la fotocamera non va in standby durante un lavoro lungo, e tenere la batteria carica;
3. chiudere FUJIFILM X RAW Studio e ogni altro programma che usa la fotocamera: ne può parlare uno solo alla volta;
4. usare RAF scattati con lo **stesso corpo macchina** collegato: gli altri vengono saltati.

La finestra controlla la fotocamera ("Ripeti verifica" dopo ogni correzione) e, prima di partire, mostra quante conversioni servono, lo spazio su disco necessario contro quello libero (con poco spazio lo sviluppo non parte) e un tempo stimato. Valori indicativi su una X-T5 (firmware 4.31) con RAF non compressi da 87 MB: circa 2,3 s per caricare ogni RAF, poi 1,5–2,5 s per ogni variante (in media 2,4 s su 5 RAF × 8 ricette); ogni JPEG a piena risoluzione pesa circa 26 MB.

- **Cartella di output**: predefinita `FujiView varianti/` accanto ai RAF, con una sottocartella per ricetta (`<ricetta>/DSCF0001.jpg`, `Come scattato/DSCF0001.jpg`); si può cambiare. Le ricette scelte vengono ricordate per la volta dopo.
- **Come attività**: lo sviluppo gira in Attività (si può chiudere la finestra e continuare a usare l'app), con **Pausa**, **Riprendi** e **Annulla**. Si mette in pausa da solo se finisce lo spazio o la fotocamera si scollega o si spegne, e riprende da dove era arrivato.
- **Varianti già presenti**: sono saltate, quindi un lavoro interrotto non si rifà da capo; "Sovrascrivi le varianti esistenti" le rigenera.
- **Sicurezza**: i RAF non vengono mai modificati e ogni JPEG compare solo quando è completo. A fine lavoro i nuovi pannelli sono già nel confronto.

Su Linux e macOS la voce è disattivata con "Disponibile solo su Windows"; tutto il resto del modulo funziona.

### Confronto e scorciatoie

Ogni preset ha un pannello (fino a 9, la griglia si adatta). Rotellina per ingrandire, trascinamento per spostare, doppio clic per adattare: **zoom e spostamento sono sincronizzati** in tutti i pannelli. La barra a destra mostra metadati e ricetta Fujifilm della variante (simulazione pellicola, gamma dinamica, bilanciamento del bianco e altro) e permette di **nascondere** e **riordinare** i preset; "Mostra tutti" li riporta tutti.

| Tasto | Azione |
|---|---|
| Maiusc+1…9 | sceglie la variante del pannello corrispondente |
| ← → ↑ ↓ | sposta il pannello a fuoco |
| Invio | sceglie la variante del pannello a fuoco |
| 1–5, 0 (anche Alt+1–5, Alt+0) | dà da 1 a 5 stelle allo scatto; 0 le toglie |
| Spazio, Maiusc+Spazio | scatto successivo, precedente |
| ↑ ↓ Pag↑ Pag↓ Home Fine | nell'elenco degli scatti, passano da uno scatto all'altro |
| Ctrl+A, Ctrl+Maiusc+I | nell'elenco degli scatti, selezionano tutto o invertono la selezione |

Scegliere una variante non fa avanzare da solo. Le scorciatoie sono ignorate mentre si scrive in un campo di testo, e Ctrl+1…9 continuano ad aprire i moduli. L'elenco completo è nel foglio delle scorciatoie (`?`).

### Sessioni

Scelte, stelle, preset nascosti e ordine dei pannelli e ultimo scatto visto si salvano **da soli**, senza un pulsante Salva, in una sessione per cartella nei dati dell'app (cartella `raw-review/`): **mai nella cartella delle foto**. Ogni scrittura è atomica e tiene una copia `.bak` usata se il file si rovina. Riaprendo la cartella si riparte da dove si era; nuovi preset (per esempio dopo uno sviluppo) non toccano le scelte già fatte. Le cartelle usate di recente compaiono in **Sessioni recenti**, dove si possono anche togliere dall'elenco (la sessione salvata non viene cancellata). **Apri progetto FujiView (.fujiview)…** apre i progetti dell'app FujiView; le sessioni di FujiView non vengono migrate da sole.

Le anteprime si tengono in una cache su disco, con indice SQLite, per aprirsi subito, e le prossime foto vengono caricate in anticipo. Il limite (20 GB se vuoto) si imposta in Impostazioni › Avanzate › **Cache delle anteprime di Varianti RAW (GB)**, minimo 1 GB; superato il limite si eliminano le anteprime usate meno di recente, e "Svuota la cache" le elimina tutte. Le foto non vengono mai toccate.

### Esportare in un nuovo catalogo

"Esporta in un nuovo catalogo…" copia in una cartella a scelta le varianti scelte e le mette in un catalogo della libreria. Il riepilogo del dialogo dice quante foto verranno esportate e quante no.

- **Quali foto**: solo scatti con una variante scelta, con **valutazione minima** (conteggi per stelle nel dialogo) e, volendo, solo alcuni **preset**.
- **Solo JPEG**: si copiano byte per byte (nessuna ricodifica), senza modificare i metadati. **RAF e TIFF non vengono mai copiati**: gli scatti scelti su un TIFF sono indicati come "non JPEG" tra gli esclusi, e quelli senza variante sviluppata come "senza scelta".
- **Nomi**: si possono mantenere gli originali o rinominare con un modello, con anteprima dei nuovi nomi. I token sono quelli dell'export della Sequenza (`{seq:04d}`, `{date}`, `{time}`, `{device}`, `{orig}`) più `{preset}`, il nome del preset della variante scelta. I numeri seguono l'ordine di scatto e, a parità di nome, si aggiunge un suffisso (-2, -3).
- **Destinazione**: una cartella fuori da quella della revisione, dove non si scrive mai. Se non fa ancora parte della libreria viene **aggiunta come sorgente**; se ne fa parte, la sorgente viene **aggiornata**; il dialogo lo dice prima. Un file già presente nella cartella è lasciato com'è, salvo "Sovrascrivi i file già presenti".
- **Catalogo**: nuovo con il nome indicato, oppure, se esiste già un catalogo manuale con quel nome, "Aggiungi le foto al catalogo esistente" o "Cambia nome" (i cataloghi Smart non accettano foto a mano).
- **Valutazioni e tag**: le stelle date in revisione passano alle foto in libreria. Uno scatto con **0 stelle non azzera** una valutazione che la foto ha già in libreria. Con "Aggiungi il preset come tag" ogni foto riceve come tag il nome del preset scelto, nel set "Preset Fujifilm".
- **Come attività**: l'esportazione (copia, importazione e catalogo) è un'unica attività in Attività, che si può annullare. Se si annulla, i file già copiati restano nella destinazione ma il catalogo non viene creato. A fine lavoro il riepilogo ha "Apri il catalogo".

## Foto simili

L'analisi raggruppa le foto quasi identiche (raffiche, scatti ripetuti, piccole variazioni della stessa inquadratura). Per ogni foto cerca le candidate in quattro modi:
- un'**impronta percettiva** di 64 bit, calcolata all'import dalla miniatura e confrontata con la distanza di Hamming;
- le foto dello stesso dispositivo (o della stessa cartella) scattate entro 10 secondi;
- i vicini nell'embedding CLIP, se i modelli AI sono installati;
- le copie esatte.

Poi conferma la somiglianza confrontando le miniature (SSIM). I gruppi si formano anche senza modelli AI. Tutto avviene in locale e nessuna funzione dei gruppi cancella o modifica file sul disco.

- **Raggruppa foto simili** (barra della griglia, anche nei cataloghi): ogni gruppo diventa una sola miniatura, la foto principale, con il numero di foto; il numero apre il gruppo sul posto. Conteggi, filtri e "Seleziona tutto" restano sulle singole foto; un clic su un gruppo chiuso seleziona tutte le sue foto presenti nei risultati.
- **Foto principale**: proposta in base a nitidezza ed esposizione; dall'Inspector (sezione **Foto simili**) si sceglie un'altra foto, e la scelta resta anche dopo nuove analisi.
- **Correzioni** dall'Inspector: *Togli dal gruppo*, *Sciogli gruppo*; le correzioni valgono anche per le analisi successive.
- **Aiuto alla scelta**: *Solo foto con simili* (barra della griglia) e *Seleziona tutte tranne la principale* (barra delle azioni), da combinare con "Escludi dal catalogo", i tag o "Rimuovi dalla libreria".
- **Soglia** in Impostazioni › Riconoscimento AI o › Avanzate (*Somiglianza dei gruppi di foto simili*, predefinita 0,80): l'anteprima indica quanti gruppi si formerebbero, e con "Applica" i gruppi vengono ricalcolati senza rianalizzare le foto.

L'analisi dei gruppi è incrementale e parte da sola dopo ogni import: vengono confrontate solo le foto nuove. Per le foto importate con una versione precedente, all'avvio un'attività una tantum calcola le impronte dalle miniature in cache e poi ricalcola i gruppi, mantenendo le correzioni dell'utente.

## Modulo AI

Il modulo **Analisi AI** (Ctrl+6) controlla lo stato delle analisi; soglie e modelli restano in Impostazioni.

- **Panoramica**: per ogni fase (classificazione, persone, volti, gruppi di simili, impronte) quante foto sono analizzate, da aggiornare, in coda o mai analizzate. Si aggiorna durante le attività e ogni conteggio apre la Libreria filtrata.
- **Rianalisi**: tutta la libreria, uno o più cataloghi, la selezione o i risultati filtrati della Libreria. Si scelgono le fasi e la modalità (solo mancanti, oppure "forza"), con un'anteprima delle foto da elaborare. Le correzioni manuali restano sempre valide.
- **Risultati**: distribuzione delle voci, istogramma delle confidenze con la soglia corrente e copertura per mese. Barre e intervalli portano alla Libreria o alla Pulizia.
- **Pulizia**: toglie un'etichetta automatica da un insieme di foto, anche solo sotto una confidenza. L'etichetta tolta non viene riassegnata dalle analisi successive, e l'operazione si può annullare.
- **Simili**: gruppi, foto in attesa, dimensioni dei gruppi e avvio dell'analisi.
- **Mappa**: mappa del mondo offline, senza richieste di rete, con le foto raggruppate per zona.
  - Le posizioni precise vengono dal GPS; quelle approssimate dai luoghi delle cartelle, collocati con il dizionario geografico offline.
  - Il clic su un gruppo apre in Libreria esattamente quelle foto.
  - Si può limitare la mappa a un catalogo.
  - I luoghi senza coordinate si posizionano a mano con un clic.

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

- **Cestino**: "Sposta nel cestino" dalla barra delle azioni (Altro), Canc sulla
  selezione fuori da un catalogo, oppure trascinando le foto sul Cestino. Le foto escono
  anche dai cataloghi manuali: se ne fanno parte, un dialogo elenca i cataloghi
  interessati (Invio conferma). L'avviso permette di annullare, cataloghi compresi;
  "Ripristina" dal Cestino riporta le foto in Libreria senza i cataloghi.
- **Duplicati**: "Metti da parte i duplicati" (scheda del catalogo o palette): per ogni
  file presente più volte con lo stesso contenuto resta in Libreria la copia importata
  per prima, le altre vanno in "Duplicati".
- **Qualità insufficiente**: "Controlla la qualità" avvia un'attività che misura la
  nitidezza sulle miniature e la risoluzione, e sposta qui le foto molto sfocate o sotto
  0,3 megapixel. Ogni foto viene misurata una volta sola, quindi una foto ripristinata
  non ci torna al controllo successivo.

La 0.3.1:
- **Organizzazione per cataloghi**: trascinare le foto dalla griglia sui cataloghi della barra laterale (anche su "Nuovo catalogo"); "Aggiungi a catalogo ▸" nel menu contestuale; barra laterale con cataloghi in alto e cartelle in fondo.
- **Correzioni**:
  - Maiusc+clic ripetuti mantengono l'ancora dell'intervallo;
  - in Persone la griglia dei volti scorre;
  - "Riassegna a…" elenca le persone con nome e le identità senza foto nella libreria.

La 0.3.0 aggiorna il catalogo allo schema 10 (cataloghi di sistema e misure di qualità);
la copia di sicurezza in `backups/` permette di tornare alla versione precedente.
