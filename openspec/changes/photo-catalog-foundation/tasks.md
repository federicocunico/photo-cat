# Tasks

Ordine per milestone verticali (vedi design.md, Risks): ogni gruppo lascia l'app compilabile e i propri test verdi. Riferimento visivo per tutta la UI: mockup https://claude.ai/artifact/3YWUuBaVhtupzLfNjtbNy3.

## 1. Scaffolding del workspace

- [x] 1.1 Creare il workspace Cargo con i crate `pc-core`, `pc-store`, `pc-diag`, `pc-governor`, `pc-jobs`, `pc-ingest`, `pc-models`, `pc-ai`, `pc-organize` e verificare che `cargo build --workspace` passi
- [x] 1.2 Inizializzare `app/src-tauri` (Tauri 2) e `app/ui` (React + TS + Vite) e verificare che `cargo tauri dev` apra una finestra vuota su Windows
- [x] 1.3 Configurare lint e formattazione (`rustfmt`, `clippy -D warnings`, ESLint, Prettier, `tsc --noEmit`) e verificare che girino puliti
- [x] 1.4 Aggiungere CI veloce (GitHub Actions) nel repository `federicocunico/photo-cat`: compilazione EXE/setup Windows con cache, test eseguiti prevalentemente in locale, e verificare una pipeline verde
- [x] 1.5 Scrivere `README.md` con prerequisiti (Rust, Node, WebView2) e comandi di sviluppo, verificando che i comandi documentati funzionino da un clone pulito

## 2. Core, configurazione e diagnostica

- [x] 2.1 Implementare in `pc-core` `AppError { code, message_key, detail, retriable }` e il catalogo dei codici errore, con unit test sulla conversione dagli errori dei crate
- [x] 2.2 Implementare directory dati (`directories`) e `config.toml` con default e validazione; unit test su config mancante, parziale e invalida
- [x] 2.3 Implementare in `pc-diag` il setup `tracing` con log JSON su file a rotazione giornaliera + limite dimensione/conservazione; test che verifica rotazione e pulizia
- [x] 2.4 Aggiungere il livello di log modificabile a runtime (`reload::Layer`); test che un evento debug compare solo dopo il cambio di livello
- [x] 2.5 Implementare il panic hook con marker `last_session_crashed` e lettura al riavvio; test che simula un panic e verifica log + marker
- [x] 2.6 Implementare il layer che inoltra WARN/ERROR con `job_id/phase/path/code` a un sink (poi collegato a `error_log` nel gruppo 3) e al broadcast UI; unit test con sink in memoria

## 3. Persistenza (pc-store)

- [x] 3.1 Definire lo schema v1 (tabelle in design.md §2) con sistema di migrazioni e backup pre-migrazione; test che crea un DB vuoto, migra e verifica le tabelle
- [x] 3.2 Implementare il writer unico a batch (canale + transazioni a soglia righe/tempo) e il pool di letture; test di concorrenza con 8 produttori senza `SQLITE_BUSY`
- [x] 3.3 Implementare i repository per media, tech, date, exif, location, tag, catalog, source; unit test CRUD
- [x] 3.4 Implementare `error_log` e collegare il sink del gruppo 2.6; test che un `warn!` con campi file/fase produce una riga persistita
- [x] 3.5 Implementare la query di filtro (facet + intervallo date + formato + tag) con keyset pagination e FTS5; test su 100.000 righe sintetiche che verifica risultati e tempo < 1 s per la prima pagina
- [x] 3.6 Implementare i cataloghi smart (regole JSON AND/OR → SQL parametrizzato) con validazione delle regole; test su regole combinate e su input malevoli
- [x] 3.7 Implementare `undo_log` per le modifiche bulk a tag/cataloghi/catalogazioni; test annulla dell'ultima operazione

## 4. Governor delle risorse (pc-governor)

- [x] 4.1 Implementare il rilevamento di RAM totale/disponibile, core logici/fisici e RSS del processo con `sysinfo`; test che i valori sono plausibili e loggati all'avvio
- [x] 4.2 Implementare il calcolo del budget (profili Leggero/Bilanciato/Prestazioni, limite manuale, riserva OS); unit test su matrice di RAM 4/8/16/32 GB
- [x] 4.3 Implementare il semaforo pesato in MB con stima dei costi (decodifica da header, inferenza, miniature); test che con budget 2 GB il consumo stimato concorrente non supera mai 2 GB
- [x] 4.4 Implementare il monitor periodico con soglie di pressione/critica che riduce i permessi e segnala pausa AI; test con sorgente RAM simulata
- [x] 4.5 Esporre lo snapshot risorse (usato/budget/sistema/modelli residenti) come evento `resources://tick`; test sul payload e sulla frequenza

## 5. Motore job e progresso (pc-jobs)

- [x] 5.1 Implementare job/fasi/stadi con canali bounded, worker per stadio dal governor e `CancellationToken`; test con stadi finti su 10.000 elementi
- [x] 5.2 Implementare contatori di progresso, % per fase e complessiva pesata, totale indeterminato, velocità EMA ed ETA, ticker a 100 ms; unit test sui calcoli e sul limite di frequenza
- [x] 5.3 Implementare pausa/ripresa/annulla con stato finale coerente (`Completato`, `Completato con errori`, `Annullato`, `Interrotto`); test che l'annullamento conserva i risultati completati
- [x] 5.4 Persistire job, fasi e `item_stage` e implementare la ripresa al riavvio solo dei pending; test che simula kill a metà job e ripresa
- [x] 5.5 Implementare la coda dei job con ammissione in base al budget; test con due job concorrenti
- [x] 5.6 Loggare inizio/fine/durata/conteggi/% finale di ogni fase; test che verifica le righe di log

## 6. Shell UI e design system

- [x] 6.1 Tradurre i token dei mockup (colori, tipografia Source Sans 3 / JetBrains Mono, spaziature) in CSS custom properties per tema scuro e chiaro; verificare contrasto ≥ 4,5:1 con un test automatico sui token
- [x] 6.2 Implementare il layout a pannelli (barra superiore, nav moduli, pannelli ridimensionabili/richiudibili persistiti, barra di stato) come da mockup "Libreria"; test di componente sulla persistenza del layout
- [x] 6.3 Configurare i18next (it/en) e `tauri-specta` per i tipi IPC generati; verificare che `tsc` fallisca se un comando Rust cambia firma
- [x] 6.4 Implementare gli indicatori globali (pill job %, chip RAM, errori recenti) collegati agli eventi `job://progress` e `resources://tick`; test con eventi simulati
- [x] 6.5 Implementare la schermata Job (mockup "Job e progresso") con fasi, barre, ETA, pausa/annulla e pannello risorse; verificare manualmente con un job finto e test di componente
- [x] 6.6 Implementare la schermata Diagnostica (mockup "Diagnostica") con filtri, dettaglio, coda log e messaggi localizzati; test di componente sui filtri

## 7. Ingestione e libreria (prima milestone utilizzabile)

- [x] 7.1 Implementare sorgenti (aggiunta/rimozione, ricorsiva, formati, pattern ignorati) e scansione parallela; test su cartella fixture con pattern
- [x] 7.2 Implementare estrazione caratteristiche tecniche (formato, w/h, DPI, bit, colore, peso) per JPEG/PNG/TIFF/WebP/BMP/GIF; test sulle fixture con valori attesi
- [x] 7.3 Implementare EXIF e date con fonte (DateTimeOriginal → CreateDate → file system) e gestione EXIF corrotto come avviso; test su fixture con EXIF assente/corrotto
- [x] 7.4 Implementare `quick_hash` + `content_hash` blake3, rilevamento duplicati e riconoscimento file spostati; test su fixture con copie e spostamenti
- [x] 7.5 Implementare la decodifica ridotta (scaling DCT) con permesso di memoria e miniature 512 px orientate in cache, servite via protocollo `thumb://`; test di orientamento e misura del picco di memoria su un JPEG 50 MP
- [x] 7.6 Implementare la scansione incrementale e lo stato "mancante"; test rescan senza modifiche (0 rielaborati) e con file cancellato
- [x] 7.7 Implementare HEIC (feature `heif`) e RAW (metadati + anteprima incorporata) con degrado a soli metadati e avviso; test con e senza feature
- [x] 7.8 Implementare il watcher `notify` con debounce per sorgenti monitorate; test che copia un file e verifica l'import automatico
- [x] 7.9 Verificare che nessuna fase modifichi gli originali: test che confronta hash e mtime prima/dopo un import completo
- [x] 7.10 Implementare la schermata Importa (mockup "Importa") e il job di import end-to-end; verifica manuale: import di 1.000 foto con progresso per fase e libreria popolata
- [x] 7.11 Implementare la griglia virtualizzata, filtri/facet, ricerca e pannello di ispezione (mockup "Libreria"); test di componente e verifica manuale dello scorrimento su 100.000 elementi sintetici

## 8. Gestione modelli (pc-models)

- [x] 8.1 Definire il formato del manifest (nome, versione, ruolo, URL, mirror, dimensione, SHA-256, memoria stimata, pre/post-processing) e il manifest iniziale con CLIP ViT-B/32, rilevatore persone, YuNet, SFace, gazetteer GeoNames; test di parsing e validazione
- [x] 8.2 Implementare il downloader (Range resume, file `.part`, SHA-256, retry con backoff, mirror) come job con progresso; test con server HTTP locale che interrompe la connessione e serve un file corrotto
- [x] 8.3 Implementare verifica all'avvio e auto-download in background non bloccante, più installazione da file locale; test su modello mancante, corrotto e file locale con hash errato
- [x] 8.4 Integrare `ort` con caricamento dinamico di onnxruntime distribuito con l'app, rilevamento provider (CPU/DirectML/CUDA/CoreML) e fallback CPU loggato; test che forza il fallimento del provider GPU
- [x] 8.5 Implementare il `ModelPool` (carica/scarica sessioni con costo da manifest corretto da RSS misurato) integrato col governor; test con budget che non contiene due modelli
- [x] 8.6 Implementare il self-test con immagini di riferimento in `assets/` e tolleranze, con tempo medio di inferenza; test che passa con il modello corretto e fallisce con un modello alterato (feature `model-tests`)
- [x] 8.7 Implementare la schermata Modelli AI e sistema (mockup) con stato, download %, verifica, test e scelta provider/profilo; test di componente sugli stati

## 9. Catalogazione AI (pc-ai)

- [x] 9.1 Definire la tassonomia predefinita (gruppi e voci della spec) versionata e modificabile dall'utente; unit test su aggiunta/disattivazione e incremento di `taxonomy_ver`
- [x] 9.2 Implementare il calcolo e la cache degli embedding testuali con prompt-ensemble (encoder testo caricato solo al cambio tassonomia); test che la cache viene invalidata al cambio versione
- [x] 9.3 Implementare lo stadio CLIP immagine (preprocessing, batch dal governor, embedding salvato) e la classificazione per gruppo con soglie; test `model-tests` su immagini di riferimento (paesaggio montano → Esterno natura/Montagna)
- [x] 9.4 Implementare lo stadio rilevamento persone (presenza, conteggio, bbox); test `model-tests` su foto con 0 e 4 persone; documentare nel README la scelta YOLOX vs RT-DETR dopo il benchmark CPU
- [x] 9.5 Implementare la geolocalizzazione (k-d tree GeoNames per GPS, euristica da directory) con fonte; unit test su coordinate di Firenze e su `D:\Foto\2023\Toscana\Siena`
- [x] 9.6 Garantire priorità delle correzioni manuali, versionamento risultati (model_ver/taxonomy_ver) e conteggio file da rianalizzare; test di rianalisi dopo correzione
- [x] 9.7 Implementare lo scheduling "model-major" quando i modelli non entrano insieme nel budget; test di integrazione con budget ridotto che verifica l'ordine di carico/scarico
- [x] 9.8 Collegare gli stadi AI al job di import e alla rianalisi; verifica manuale: import con AI su 1.000 foto con RAM dell'app entro il budget mostrato

## 10. Riconoscimento volti

- [x] 10.1 Implementare rilevamento YuNet con soglie di dimensione/qualità e allineamento a 5 landmark; test `model-tests` su foto con 3 volti e con volto troppo piccolo
- [x] 10.2 Implementare embedding SFace e salvataggio dei volti; test di similarità stessa persona vs persone diverse sulle immagini di riferimento
- [x] 10.3 Implementare assegnazione online alle persone con nome (rispettando i rifiuti) e clustering a fine job (Chinese Whispers su kNN/ANN con vincoli cannot-link); unit test su embedding sintetici con correzioni
- [x] 10.4 Implementare le operazioni utente: rinomina, unisci, separa, "non è questa persona", rimuovi volto; test che le correzioni sopravvivono al ri-clustering
- [x] 10.5 Implementare disattivazione del riconoscimento (conteggio persone mantenuto) ed "Elimina tutti i dati dei volti" con VACUUM; test che non restano volti/embedding/ritagli
- [ ] 10.6 Implementare la schermata Persone (mockup "Persone") con cluster, dettaglio e correzioni; test di componente e verifica manuale su una libreria di prova

## 11. Cataloghi e operazioni bulk

- [x] 11.1 Implementare la schermata Cataloghi (mockup) con cataloghi manuali e editor di regole smart con anteprima conteggio; test di componente sull'editor
- [x] 11.2 Implementare la selezione multipla (clic, Ctrl/Shift, rettangolo, seleziona tutto sui risultati filtrati, inverti) con contatore; test di componente
- [x] 11.3 Implementare le azioni bulk (tag, cataloghi, correzione campo, assegna persona, rianalizza, rimuovi dalla libreria) con conferma per quelle distruttive e job sopra soglia; test di integrazione su 5.000 elementi
- [x] 11.4 Implementare "Annulla" dell'ultima azione bulk ed esito riusciti/falliti con collegamento alla diagnostica; test annulla tag su 200 file

## 12. Export e organizzazione cartelle (pc-organize)

- [x] 12.1 Implementare l'export CSV (separatore configurabile, BOM opzionale) e JSON in streaming con `schema_version`, ambito e campi selezionabili, scrittura atomica; test su campi multi-valore, accenti e annullamento senza file parziali
- [x] 12.2 Implementare il calcolo dell'albero di organizzazione (gerarchia multi-livello, "Senza <valore>", sanificazione nomi, suffissi per conflitti) come anteprima senza IO; unit test su caratteri riservati, `CON`/`NUL` e due `IMG_0001.JPG`
- [x] 12.3 Implementare la verifica preliminare del metodo (symlink/hard link di prova, rilevamento Developer Mode e volumi diversi) con proposta alternativa; test su Windows con e senza privilegi symlink
- [x] 12.4 Implementare l'esecuzione come job (symlink/hard link/copia, path lunghi `\\?\`), registro `organize_entry` e annullamento sicuro; test che l'undo rimuove solo gli elementi creati e lascia intatti gli originali
- [x] 12.5 Implementare la schermata Esporta e organizza (mockup) con anteprima albero, conteggi e spazio richiesto; test di componente

## 13. Primo avvio, impostazioni e pacchetto diagnostico

- [ ] 13.1 Implementare l'onboarding (mockup "Primo avvio"): benvenuto → verifica sistema → download modelli → prima sorgente, con passi rimandabili; verifica manuale su profilo utente pulito
- [x] 13.2 Implementare la schermata Impostazioni (non presente nei mockup: seguire lo stile della schermata "Modelli AI e sistema") (profilo/limite RAM, provider, soglie, tassonomia, volti on/off, livello log, tema, lingua, percorso dati); test che il cambio profilo modifica il budget dei job successivi
- [x] 13.3 Implementare "Esporta pacchetto diagnostica" (zip con lista di inclusione esplicita), "Apri cartella log" e "Riprova file falliti"; test che lo zip non contiene immagini né dati volti
- [ ] 13.4 Implementare l'avviso di chiusura anomala al riavvio con collegamento alla diagnostica; verifica manuale con panic forzato in build di debug

## 14. Integrazione e rilascio

- [ ] 14.1 Test end-to-end su una libreria di 10.000 foto reali in Windows Sandbox con 8 GB e sorgente reale mappata in sola lettura: import + AI + volti completati, RAM dell'app sempre entro il budget, errori per file registrati e job ripreso dopo kill; nessuna modifica, cancellazione o spostamento degli originali
- [ ] 14.2 Verificare l'esecuzione offline completa con modelli già presenti (nessuna richiesta di rete durante l'analisi, controllo con monitor di rete)
- [ ] 14.3 Configurare il bundle Tauri (NSIS/MSI con onnxruntime incluso) e verificare installazione e primo avvio in Windows Sandbox pulita
- [ ] 14.4 Eseguire `openspec validate photo-catalog-foundation --strict` e verificare che ogni requisito delle spec sia coperto da almeno un test o una verifica manuale documentata
- [x] 14.5 Creare la pipeline di rilascio automatico su GitHub: su tag di versione `v*` compilare il setup Windows, creare la GitHub Release e allegare l'installer EXE con checksum; verificare la creazione e gli asset della release

## Workflow follow-up

- Archiviare il change dopo la review (`/opsx:archive`), verificando che le spec principali in `openspec/specs/` siano state create.
