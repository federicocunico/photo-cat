# Design

## Context

Repository vuoto: esistono solo `docs/Applicazione per catalogare le fotografie.md` (idea + studio di fattibilità) e la configurazione OpenSpec. Motivazioni e ambito in `proposal.md`; requisiti nelle spec di questo change (`specs/*/spec.md`).

Vincoli emersi con l'utente:
- **Stack**: Tauri 2 (backend Rust) + React/TypeScript.
- **Piattaforma**: cross-platform, Windows primario (sviluppo e test principali su Windows 11).
- **AI**: tutto in locale, CPU di default; volti inclusi già in questa versione.
- **RAM**: verifica delle risorse all'avvio e coordinamento delle chiamate in base al budget.
- **Osservabilità**: raccolta errori, log e percentuali di avanzamento per fase sono requisiti di prima classe.
- **UI**: mockup prodotti con il Design artifact → **https://claude.ai/artifact/3YWUuBaVhtupzLfNjtbNy3** (Libreria, Importa, Job e progresso, Cataloghi, Persone, Modelli AI e sistema, Diagnostica, Esporta e organizza, Primo avvio). I mockup sono il riferimento visivo per layout, token di colore e tipografia.

## Goals / Non-Goals

**Goals:**
- Architettura a crate separati, con un core testabile senza GUI, così che in futuro si possano aggiungere una CLI, nuovi tipi di media e nuovi modelli senza toccare la shell.
- Pipeline concorrente con back-pressure e controllo esplicito della memoria, stabile su macchine con 8 GB di RAM.
- Un unico meccanismo trasversale per progresso, errori e log usato da tutte le fasi.
- Modelli con licenze compatibili con un uso eventualmente commerciale del prodotto.

**Non-Goals:**
- Editing delle immagini, sviluppo RAW, sincronizzazione cloud, multiutente.
- Video e documenti (il modello dati li prevede, l'implementazione no).
- CLI dedicata (il core la rende possibile; non è in questo change).
- Ricerca semantica in linguaggio naturale ("foto al tramonto con cane"): gli embedding CLIP vengono salvati per abilitarla in un change futuro.
- Auto-update dell'applicazione (solo dei modelli).

## Decisions

### 1. Struttura del repository: workspace Cargo + frontend Vite

```
photo-cat/
├─ Cargo.toml                 (workspace)
├─ crates/
│  ├─ pc-core/        dominio, tipi, errori (codici), config, i18n keys
│  ├─ pc-store/       SQLite: schema, migrazioni, repository, query filtri/FTS
│  ├─ pc-diag/        tracing setup, log su file, registro errori, panic hook, bundle
│  ├─ pc-governor/    rilevamento risorse, budget, permessi di memoria, monitor
│  ├─ pc-jobs/        motore job/fasi, progresso, cancellazione, persistenza
│  ├─ pc-ingest/      sorgenti, scansione, metadati, EXIF, hash, miniature, watcher
│  ├─ pc-models/      manifest, download, verifica, self-test, sessioni ONNX
│  ├─ pc-ai/          classificazione CLIP, persone, volti, clustering, geo
│  └─ pc-organize/    export CSV/JSON, organizzazione cartelle, undo
├─ app/
│  ├─ src-tauri/      shell Tauri: comandi IPC, eventi, protocollo thumb://
│  └─ ui/             React + TS (Vite)
└─ assets/            manifest modelli, immagini self-test, gazetteer
```
I crate `pc-*` non dipendono da Tauri. *Alternativa scartata*: crate unico dentro `src-tauri` — più veloce all'inizio ma lega il dominio alla GUI e impedisce CLI/test headless.

### 2. Persistenza: SQLite (rusqlite, bundled) con un unico writer
- WAL, `synchronous=NORMAL`, migrazioni versionate (`refinery` o tabella `schema_version` propria) eseguite all'avvio con backup automatico prima della migrazione.
- **Un solo thread writer** riceve comandi da un canale e scrive in transazioni a batch (es. 200 righe o 250 ms): evita `SQLITE_BUSY` con molti worker e massimizza il throughput. Letture da un pool di connessioni read-only.
- Schema principale: `media_item` (id, kind, source_id, path, name, size, mtime, quick_hash, content_hash, status: ok/missing/duplicate), `media_tech` (formato, w, h, dpi_x, dpi_y, bit_depth, color_space), `media_dates` (+ fonte), `exif` (colonne principali + JSON grezzo), `location` (lat, lon, nome, fonte), `label` (media_id, group, value, score, origin auto/manual, model_ver, taxonomy_ver), `embedding` (media_id, model_ver, vector BLOB), `face` (media_id, bbox, quality, embedding BLOB, person_id, rejected_person_ids), `person`, `tag`/`media_tag`, `catalog` (manual/smart, regole JSON), `catalog_item`, `source`, `job`, `job_phase`, `item_stage` (stato per file e fase → ripresa), `error_log`, `organize_op`/`organize_entry`, `undo_log`.
- FTS5 su nome, percorso, luogo, persone, tag. I cataloghi smart sono regole JSON compilate in SQL parametrizzato (nessuna concatenazione di stringhe utente).
- `kind` + tabelle satellite per tipo rendono semplice aggiungere video/documenti. *Alternative*: `sqlx` (async, ma qui il lavoro è CPU-bound e l'API sync con writer dedicato è più semplice); DB embedded KV (sled/redb) scartati per la ricerca a filtri combinati.

### 3. Motore job: pipeline a stadi con canali limitati
- `tokio` per orchestrazione/IO e `rayon` (o `spawn_blocking` con pool dimensionato) per decodifica e inferenza.
- Ogni job è un grafo lineare di **stadi** collegati da canali **bounded** (back-pressure naturale): `scan → metadata/hash → thumbnail → ai:clip → ai:persons → ai:faces → writer`. Il numero di worker per stadio viene dal governor.
- Progresso: contatori atomici per fase (`done`, `failed`, `total`, `total_known`); un ticker a 100 ms calcola %, velocità (media mobile esponenziale) ed ETA ed emette l'evento Tauri `job://progress` — da cui il limite ~10 aggiornamenti/s della spec.
- Percentuale complessiva = media pesata delle fasi con pesi per costo medio (es. AI e volti pesano più della scansione), ricalibrati con i tempi misurati.
- Cancellazione/pausa: `CancellationToken` + flag di pausa controllati tra un elemento e l'altro; nessun elemento viene interrotto a metà scrittura.
- Ripresa: `item_stage` registra per file e fase lo stato `pending/done/failed`; riprendere un job = rilanciare gli stadi sui soli `pending`. Le analisi sono idempotenti (upsert per `media_id + group + model_ver`).
- *Alternativa scartata*: un unico thread pool "tutto in uno" senza stadi — non permette di dare budget diversi a decodifica e inferenza né di serializzare le fasi AI quando la RAM non basta.

### 4. Governor delle risorse
- **Rilevamento** con `sysinfo` (RAM totale/disponibile, core logici/fisici, RSS del processo) e sonda dei provider ONNX disponibili.
- **Budget** = `min(limite_utente, frazione_profilo × RAM_disponibile_all_avvio, RAM_totale − riserva_OS)` con frazioni indicative Leggero 25%, Bilanciato 40%, Prestazioni 60% e riserva minima 2 GB. Valori in config, tarati nei test.
- **Permessi di memoria**: semaforo pesato in MB. Ogni operazione chiede un permesso pari al costo stimato e lo rilascia a fine lavoro:
  - decodifica: `w × h × 4 × 1,5` byte (dimensioni lette dall'header prima di decodificare);
  - inferenza: memoria del modello è "residente" (vedi sotto) + `batch × costo_input`;
  - miniature: costo della decodifica ridotta.
- **Decodifica ridotta**: per JPEG si usa lo scaling DCT (1/2, 1/4, 1/8) del decoder per ottenere direttamente l'immagine ≥ alla dimensione richiesta (miniatura 512 px, input modelli 224/640 px): riduce di 10–60× la memoria per le foto ad alta risoluzione. Un'unica decodifica per file alimenta miniatura e input dei modelli.
- **Residenza modelli**: un `ModelPool` carica/scarica sessioni ONNX contando il loro costo (da manifest, poi corretto con misura RSS reale dopo il caricamento). Se i modelli AI non entrano insieme, lo scheduler passa in modalità **"model-major"**: esegue tutta la classificazione sulla coda, scarica il modello, poi carica quello dei volti.
- Sessioni ONNX con `intra_op_threads` derivati dai core e dal numero di worker (no oversubscription), arena di memoria limitata/disattivata su CPU.
- **Monitor** ogni 1 s: se la RAM disponibile di sistema scende sotto la soglia (es. 10% o 1 GB) riduce i permessi disponibili; sotto la soglia critica mette in pausa le fasi AI. Ogni transizione è loggata. Il valore è pubblicato alla UI ogni 1–2 s.

### 5. Inferenza: ONNX Runtime via `ort`, CPU default
- `ort` 2.x con caricamento dinamico della libreria `onnxruntime` distribuita con l'app (build con DirectML su Windows; CPU su macOS/Linux, CoreML/CUDA opzionali). *Alternativa*: Candle — 100% Rust ma più lento su CPU per questi modelli e con meno modelli pronti; resta un'opzione dietro il trait `InferenceBackend` per il futuro.
- Pre/post-processing in Rust (`fast_image_resize`, `ndarray`).

### 6. Scelta dei modelli (licenze permissive)
| Ruolo | Modello predefinito | Note |
|---|---|---|
| Classificazione zero-shot + embedding | CLIP ViT-B/32 (OpenAI/OpenCLIP), encoder immagine e testo ONNX, variante quantizzata int8 se supera il self-test | MIT; embedding salvati per la ricerca semantica futura |
| Rilevamento persone | YOLOX-S o RT-DETR-R18 (Apache-2.0), solo classe "person" | **YOLOv8/11 di Ultralytics è AGPL-3.0**: scartato come predefinito per non vincolare la licenza del prodotto |
| Rilevamento volti | YuNet (OpenCV Zoo, MIT) | leggero, landmark per l'allineamento |
| Embedding volti | SFace (OpenCV Zoo, Apache-2.0) | *Alternativa*: InsightFace ArcFace/SCRFD — più accurati ma pesi con licenza **non commerciale**; ammessi solo come opzione dell'utente nel manifest |
| Geocodifica inversa | GeoNames `cities1000`/`cities15000` (CC-BY 4.0) come asset scaricabile | k-d tree in memoria (~20–50 MB) |

- **Distribuzione (decisione con l'utente)**: modelli e runtime provengono solo da repository pubblici stabili (Hugging Face, release GitHub ufficiali, OpenCV Zoo, GeoNames) e si usano varianti con ogni file < 100 MB (limite di GitHub): CLIP ViT-B/32 quantizzato int8 (Xenova), YOLOX-tiny, YuNet, SFace, GeoNames `cities15000`, più `onnxruntime.dll` + `DirectML.dll`. I file sono versionati nel repository in `assets/models/` e inclusi nell'installer; il manifest conserva URL pubblici e SHA-256 e il download all'avvio interviene solo per file mancanti o corrotti.
- **Testi della tassonomia**: gli embedding testuali (con prompt-ensemble, es. "una foto di {etichetta}") sono calcolati una volta, salvati in DB con la `taxonomy_ver`, e ricalcolati solo quando la tassonomia cambia → l'encoder testuale è caricato solo in quel momento.
- Classificazione = softmax sulle similarità coseno per ciascun gruppo della tassonomia; soglie per gruppo configurabili.
- Tutti i modelli sono dietro il manifest: sostituirli non richiede modifiche al codice, solo un nuovo manifest con pre/post-processing dichiarato (tipo di modello noto).

### 7. Volti: assegnazione incrementale + clustering periodico
- Per ogni volto valido: allineamento con i 5 landmark → embedding SFace normalizzato.
- **Assegnazione online**: confronto con i centroidi delle persone con nome; sopra soglia → assegnato (rispettando i `rejected_person_ids`).
- **Clustering** dei volti non assegnati alla fine del job: Chinese Whispers / agglomerativo su grafo dei k vicini con soglia coseno, vincoli *cannot-link* dalle correzioni utente. Con molti volti, indice ANN (`hnsw_rs` o `instant-distance`) per i vicini.
- Le correzioni utente sono vincoli persistenti, mai dati derivati: il ri-clustering non può violarle.
- "Elimina dati volti": cancellazione di `face`, `person`, ritagli in cache e `VACUUM`.

### 8. Ingestione e metadati
- Scansione con `walkdir`/`jwalk` (parallelo), filtri glob (`globset`). Watcher con `notify` + debounce.
- Metadati tecnici dall'header (`image` + `imagesize`; DPI da JFIF/EXIF/PNG pHYs/TIFF); EXIF con `kamadak-exif` (JPEG/TIFF/HEIC container). HEIC: decodifica tramite `libheif-rs` come feature opzionale (licenza LGPL della libreria → linking dinamico); senza feature → solo metadati. RAW: metadati + anteprima JPEG incorporata quando presente.
- Hash: `quick_hash` (dimensione + blake3 dei primi/ultimi 64 KB) per rilevare spostamenti in modo economico; `content_hash` blake3 completo calcolato in streaming per i duplicati esatti.
- Miniature: JPEG/WebP 512 px nella cache `thumbs/<hash[0..2]>/<hash>.webp`, servite alla UI con un protocollo custom `thumb://` (niente base64 via IPC).
- Luogo da directory: si scorrono i segmenti del percorso dal più profondo, scartando anni/date/nomi generici (`DCIM`, `Foto`, `Camera`…) e cercando corrispondenze nel gazetteer; altrimenti si propone il segmento più significativo come luogo con fonte "directory".

### 9. Diagnostica: `tracing` come unica fonte
- `tracing` + `tracing-subscriber` con tre layer: (1) file JSON con `tracing-appender` a rotazione giornaliera + limite dimensione/conservazione; (2) layer che trasforma eventi `WARN/ERROR` con campi `job_id`, `phase`, `path`, `code` in righe di `error_log` (via writer DB, a batch); (3) broadcast verso la UI per la coda dei log.
- Livello modificabile a runtime con `reload::Layer`.
- Errori tipizzati per crate (`thiserror`) che convergono in `pc_core::AppError { code, message_key, detail, retriable }`; `message_key` → testo localizzato nella UI, `detail` → dettaglio tecnico.
- Panic hook che logga e scrive un marker `last_session_crashed`; al riavvio la UI mostra l'avviso.
- Pacchetto diagnostico: zip con log recenti, export `error_log`, info sistema/governor, versioni app/manifest, stato job. Nessuna immagine o dato dei volti per costruzione (lista di inclusione esplicita).

### 10. Export e organizzazione
- Export in streaming dalla query (CSV con crate `csv` + BOM UTF-8 opzionale per Excel; JSON con array in streaming) su file temporaneo nella destinazione e `rename` atomico finale. Multi-valore: in CSV unito con `|` (configurabile) e in JSON array. Campo `schema_version`.
- Organizzazione: anteprima = calcolo dell'albero in memoria senza IO. Verifica preliminare scrivendo un symlink/hard link di prova nella destinazione (rileva Developer Mode su Windows e volumi diversi). Ogni elemento creato è registrato in `organize_entry` → l'undo rimuove solo quelli, verificando che puntino ancora all'origine registrata. Nomi sanificati (caratteri riservati, `CON`/`NUL`…, lunghezza ≤ 255, path lunghi Windows con prefisso `\\?\`).

### 11. Frontend
- React 18 + TypeScript + Vite; **Zustand** per lo stato UI, **TanStack Query** per i dati dal backend, **TanStack Virtual** per la griglia (100k elementi), **Radix UI** primitives per accessibilità, **i18next** (it/en).
- Design token in CSS custom properties derivati dai mockup (tema scuro predefinito + chiaro).
- IPC tipizzato: comandi Tauri con tipi TypeScript generati (`tauri-specta`), così il contratto Rust↔TS non diverge. Eventi: `job://progress`, `job://state`, `resources://tick`, `log://tail`, `models://status`.
- La griglia chiede pagine di id per filtro (keyset pagination) e i dettagli solo per gli elementi visibili.

### 12. Dati dell'app e configurazione
- Directory dati via `directories` (es. `%APPDATA%\photo-cat\` su Windows): `catalog.db`, `thumbs/`, `models/`, `logs/`, `config.toml`. Percorso spostabile dalle impostazioni.
- Config in TOML con valori di default compilati; validazione all'avvio.

### 13. Test
- Unit test per ogni crate core; test di integrazione su una cartella fixture di immagini (EXIF variati, orientamenti, corrotti, duplicati, HEIC).
- Test dei modelli dietro feature `model-tests` (richiedono i modelli scaricati), riusando le stesse immagini del self-test.
- Test del governor con budget artificialmente bassi per verificare ammissione, modalità model-major e pausa.
- Frontend: Vitest + Testing Library sui componenti; smoke test E2E con WebDriver Tauri in un change successivo.
- Verifiche complete e test dei modelli prevalentemente in locale. GitHub Actions su `federicocunico/photo-cat` esegue una pipeline rapida Windows con cache per compilare EXE/setup, senza matrici Linux/macOS né suite pesanti a ogni push.
- Installazione pulita e verifica end-to-end in Windows Sandbox con 8 GB. La sorgente fotografica fornita dall'utente è non cancellabile e mappata in sola lettura; DB, miniature, log, export e pulizia dei test risiedono solo in directory di output separate.

### 14. CI e pubblicazione degli installer
- Una build Windows con cache produce l'eseguibile e il setup NSIS; il bundle MSI resta verificabile localmente. Le suite Rust, UI e modelli vengono eseguite localmente prima del push.
- Un tag `v<versione>` attiva la creazione automatica della GitHub Release con installer EXE e checksum SHA-256. La versione del tag deve corrispondere a quella dell'app; i push ordinari producono artefatti di build senza pubblicare release.
- Il workflow riceve permessi di scrittura dei contenuti solo per la pubblicazione della release. Nessuna fotografia dell'utente è inclusa negli asset, negli artefatti CI o nel repository.

## Risks / Trade-offs

- [Accuratezza CLIP zero-shot su categorie fini (es. "teatro" vs "palazzetto")] → prompt-ensemble, soglie per gruppo, correzione manuale che prevale; in futuro classificatori lineari leggeri addestrati sulle correzioni dell'utente.
- [Modelli permissivi per volti meno accurati di InsightFace] → manifest sostituibile; InsightFace selezionabile dall'utente con avviso di licenza.
- [Stime di memoria imprecise → superamento budget] → stime conservative, correzione con RSS misurato dopo il caricamento dei modelli, monitor di pressione che riduce/mette in pausa.
- [Distribuzione di onnxruntime (dimensione, DLL su Windows, provider GPU)] → libreria caricata dinamicamente dalla cartella dell'app, fallback CPU e self-test all'avvio.
- [Symlink su Windows senza Developer Mode] → verifica preliminare + proposta di hard link/copia (spec `file-organization`).
- [Contesa SQLite con molti worker] → writer unico a batch; letture separate.
- [Dati biometrici] → solo locale, disattivabili, cancellabili, mai nel pacchetto diagnostico.
- [HEIC/RAW e licenze delle librerie native] → feature opzionali e linking dinamico; senza di esse si degrada a soli metadati con avviso.
- [Ampiezza del primo change] → tasks ordinati per milestone verticali utilizzabili (import+libreria → job/diag → AI → volti → export/organizzazione), ognuna testabile da sola.

## Migration Plan

Progetto nuovo: nessuna migrazione dati. Il sistema di migrazioni dello schema SQLite è introdotto da subito (versione 1) con backup automatico del DB prima di ogni migrazione futura. Distribuzione con il bundler Tauri (NSIS/MSI su Windows, dmg/AppImage negli altri sistemi); rollback = reinstallazione della versione precedente (il backup del DB copre il downgrade).

## Open Questions

- Scelta finale tra YOLOX e RT-DETR per le persone: si decide in base al self-test di velocità/precisione su CPU durante l'implementazione (entrambi Apache-2.0, stessa interfaccia).
- URL definitivi e mirror dei modelli (Hugging Face vs hosting proprio): definibili nel manifest senza cambiare il design.
- Firma del codice dell'installer Windows: da decidere prima della prima distribuzione pubblica.
