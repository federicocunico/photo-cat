# Tasks

Riferimento visivo: `mockups/Tags.html`, `mockups/Identities.html`, `mockups/Sequence.html`, `mockups/SequenceExport.html` (aprire nel browser; vedi `mockups/README.md`). Ogni gruppo lascia `make test` verde (fmt, clippy, test Rust e UI) e aggiorna le stringhe i18n in `it.ts` ed `en.ts` per le parti di UI che tocca.

## 1. Archivio dell'app (`app.db`)

- [x] 1.1 Aggiungere `AppPaths::app_store()` (design §1): cartella di configurazione della piattaforma, indipendente da `data_dir`; con `PHOTO_CAT_DATA_DIR` impostata, `<radice>/app/`. Verifica: unit test in `pc-core` che con `data_dir` reindirizzato il percorso di `app.db` non cambia, e che con `PHOTO_CAT_DATA_DIR` sta sotto la radice indicata.
- [x] 1.2 Creare il modulo `pc-store` per `app.db` con migrazioni versionate proprie e lo schema v1 (identità, alias, volti di riferimento, set di tag, voci dei set, colori dei tag, `app_kv`) e il set "Base" iniziale. Verifica: test che aprono un `app.db` vuoto, ne controllano versione e tabelle e verificano che il set "Base" esista una sola volta dopo due aperture.
- [x] 1.3 Aprire `app.db` all'avvio in `app/src-tauri` (stato condiviso accanto allo `Store` della libreria) e registrarlo nel bundle diagnostico senza i blob dei volti. Verifica: `make dev` crea `app.db` nella cartella attesa; il test esistente del bundle diagnostico passa e il bundle non contiene la tabella dei volti di riferimento.

## 2. Gestione dei tag e set

- [x] 2.1 In `pc-store/src/bulk.rs`: rinomina, unione ed eliminazione di un tag con voce in `undo_log`, e parametro `exclusive_siblings` in `add_tag` (design §3). Verifica: unit test per rinomina, rinomina su nome esistente (errore che segnala l'unione), unione con file in comune, eliminazione, sostituzione nel set esclusivo e `undo_last` per ciascuno.
- [x] 2.2 In `app.db`: CRUD dei set (crea, rinomina, duplica, elimina, riordina, esclusivo, limite 9), colori dei tag, set attivo; propagazione di rinomina e unione dei tag ai set. Verifica: unit test inclusi limite dei 9 tag, eliminazione set che non tocca i tag e rilevamento dei file in conflitto quando un set diventa esclusivo.
- [x] 2.3 Comandi Tauri `tags.rs` (elenco con conteggi e set, rinomina, unione, eliminazione, colore, set, set attivo, conflitti) con tipi `specta` e binding rigenerati. Verifica: `cargo test -p photo-cat` e `npm --prefix app/ui run typecheck` passano con i nuovi binding.
- [x] 2.4 Modulo UI `tags` secondo `mockups/Tags.html` (colonna set, dettaglio set con tasti e opzione esclusivo, tutti i tag, pannello del tag con rinomina, colore, unione, eliminazione con conferma), voce in `registry.ts` e `ModuleId`. Verifica: test Vitest per rinomina, unione con conferma, eliminazione con conteggio e creazione di un set; navigazione da tastiera e `aria-label` sui pulsanti a icona.
- [x] 2.5 Hook `useTagHotkeys` (1–9, 0) in Libreria, chip colorati dei tag in griglia e Inspector, e indicatore del set attivo nella barra di stato. Verifica: test Vitest che il tasto 3 invia l'azione bulk sulla selezione, che lo 0 rimuove i tag del set attivo e che con il fuoco nella ricerca nessuna azione parte.

## 3. Identità

- [x] 3.1 CRUD delle identità in `app.db` (nome univoco, nome breve predefinito, alias, gruppo, soglia, unione) e volti di riferimento (ritaglio allineato + embedding per versione di modello). Verifica: unit test su nome duplicato, nome breve predefinito, ricerca per alias e unione che sposta alias e riferimenti.
- [x] 3.2 Migrazione `catalog.db`: colonna `person.identity_uid` in un nuovo `schema_v5.sql` (che il task 5.1 estende prima del rilascio) e migrazione una tantum delle persone con nome verso identità con marcatore in `settings_kv`. Verifica: test su un DB v4 con due persone con nome che dopo l'apertura sono collegate a due identità, e che una seconda apertura non crea duplicati.
- [x] 3.3 Estendere `named_match` e il job dei volti (design §2): confronto con i riferimenti delle identità con soglia propria, creazione di una persona collegata in una libreria nuova, ricalcolo degli embedding dei riferimenti al cambio di modello; `people::clear` cancella anche i riferimenti. Verifica: unit test con embedding sintetici per match su identità non collegata, rispetto di "non è questa persona" e cancellazione dei riferimenti; `cargo test -p pc-ai --features model-tests` passa sulle immagini di self-test.
- [x] 3.4 Esportazione e importazione delle identità (con o senza volti) con proposta di unione per nomi uguali; scegliere e documentare nel design il formato del file (Open Questions). Verifica: test di round-trip export → import su un `app.db` vuoto, e import su un archivio con nome già presente che non crea duplicati.
- [x] 3.5 Comandi Tauri `identities.rs` e aggiunta di un riferimento da file immagine (rilevamento volti; scelta del volto se più di uno). Verifica: test del comando con l'immagine `assets/selftest/group_3_faces.jpg` che restituisce tre volti tra cui scegliere.
- [x] 3.6 Modulo UI `identities` secondo `mockups/Identities.html` (elenco con filtro per gruppo, pannello di modifica, volti di riferimento, soglia, stato del collegamento, importa/esporta, posizione di `app.db`) e in `PeoplePage` selettore di identità nel campo "Chi è?". Verifica: test Vitest per creazione, nome duplicato, collegamento di un gruppo senza nome a un'identità e suggerimenti per alias.

## 4. Fuso e dispositivo all'importazione

- [x] 4.1 In `pc-ingest/src/metadata.rs` leggere `OffsetTimeOriginal`/`OffsetTimeDigitized`/`OffsetTime` e dispositivo normalizzato (marca + modello). Verifica: unit test con EXIF sintetici (offset presente, assente, malformato) e sulle immagini di self-test.
- [x] 4.2 Rigenerare `assets/models/geo/cities15000.tsv` con la colonna del fuso IANA, aggiornare SHA-256 nel manifest, formato in `assets/README.md`, e `Gazetteer::timezone_at`. Verifica: test che `timezone_at` per le coordinate di Firenze restituisce `Europe/Rome` e per Moab `America/Denver`; la verifica dei modelli all'avvio passa.

## 5. Istante reale e correzioni orarie

- [x] 5.1 `schema_v5.sql` per `catalog.db` (colonne di `media_dates`, `source_group`, `media_time_placement`, indici) e calcolo di `instant_utc` all'importazione (design §4) con `chrono-tz`. Verifica: test di migrazione da v4 con backup creato; unit test del calcolo con offset dichiarato, con fuso del gruppo (incluso il cambio dell'ora legale) e con fuso di sistema marcato presunto.
- [x] 5.2 Job `time_backfill` per le librerie esistenti, accodato una volta, che rilegge solo l'EXIF. Verifica: test su una libreria v4 che dopo il job ha `instant_utc` per ogni foto e nessuna miniatura o analisi rigenerata.
- [x] 5.3 Operazioni sui gruppi: ora locale, scostamento al secondo, allineamento di due foto, collocazione delle foto senza data, tutte con `undo_log` e ricalcolo in blocco. Verifica: unit test per ciascuna, incluso l'annullamento, e test di prestazioni che un cambio di scostamento su 100.000 righe sintetiche richiede meno di 1 s.
- [x] 5.4 Suggerimento di fuso dal GPS (design §5). Verifica: test con due gruppi sintetici (X-T5 a +02:00 senza GPS, iPhone a -07:00 con GPS negli stessi istanti) che produce il suggerimento -07:00 con il conteggio corretto, e nessun suggerimento se i fusi coincidono.
- [x] 5.5 Ordinamento della libreria per istante reale e filtro date sull'ora locale corretta; il CSV/JSON di catalog-export continua a esportare `captured_at` invariato. Verifica: test di query in `pc-store` e test esistenti di `export.rs` invariati.

## 6. Vista Sequenza

- [x] 6.1 Query di sequenza per ambito (libreria, catalogo, selezione) con sessioni (`LAG`, soglia in `app_kv`), dispositivo, tag del set attivo e foto senza data; comandi Tauri `timeline.rs`. Verifica: unit test sulle sessioni con soglia predefinita e modificata, e test che la prima pagina su 100.000 righe arriva entro 1 s.
- [x] 6.2 Modulo UI `sequence` secondo `mockups/Sequence.html`: barra con ambito e tasti del set attivo, banner del suggerimento, sessioni con griglia virtualizzata e striscia del dispositivo, area "Senza data" con trascinamento, pannello "Sorgenti e orari" con anteprima e allineamento, tasti 1–9/0, A, Spazio, Maiusc+clic. Verifica: test Vitest per applicazione dello scostamento con anteprima, accettazione del suggerimento, selezione di una sessione con A e collocazione di una foto senza data; alternativa da tastiera al trascinamento (menu "Colloca dopo…").

## 7. Export della sequenza

- [x] 7.1 Parser e motore del modello di nome (token, `{seq:0Nd}`, token vuoto che elimina il separatore, sanificazione Windows/Linux, suffissi deterministici). Verifica: unit test che coprono ogni token e gli scenari di `specs/sequence-export` (incluso token sconosciuto).
- [x] 7.2 Decodifica pubblica a piena risoluzione in `pc-ingest` (pixel, profilo ICC, orientamento) per JPEG, PNG, TIFF, HEIC e RAF. Verifica: test che un PNG con profilo ICC restituisce lo stesso profilo e che una foto con orientamento 6 viene ruotata.
- [x] 7.3 Codifica JPEG con `jpeg-encoder` (qualità, 4:4:4, ICC), metriche SSIM/PSNR e ciclo di ricodifica fino a 100 (design §7). Verifica: unit test delle metriche su immagini sintetiche note (identiche: SSIM 1; rumore noto: PSNR atteso) e test che una soglia irraggiungibile termina a qualità 100 con esito segnalato.
- [x] 7.4 Metadati delle copie: EXIF (data e fuso corretti, fotocamera, obiettivo, GPS, orientamento 1) e XMP (tag e identità). Verifica: test di round-trip che rilegge la copia con il lettore di `pc-ingest` e trova data, fuso, GPS, parole chiave e persone; gli originali hanno hash e mtime invariati.
- [x] 7.5 Job `sequence_export`: verifica preliminare (spazio, hard link, destinazione già usata), cartella temporanea nella destinazione, `sequenza/`, `per-tag/` con hard link o copie, `manifest.csv`, zip non compressi con ZIP64, spostamento finale e pulizia in annullamento/errore. Verifica: test di integrazione su una sequenza di 10 immagini di test che controllano struttura, conteggi, contenuto del manifest e zip leggibili; test di annullamento che non lascia nulla nella destinazione.
- [x] 7.6 Comandi Tauri e vista di export secondo `mockups/SequenceExport.html` (modello con chip dei token e anteprima live, formato e qualità, struttura, destinazione, esito della verifica preliminare, avviso foto senza tag, avvio come job). Verifica: test Vitest per anteprima che si aggiorna mentre si scrive, token sconosciuto che disabilita l'avvio e spazio insufficiente che blocca l'avvio.

## 8. Integrazione

- [ ] 8.1 Prova manuale end-to-end con foto reali di almeno due dispositivi (fotocamera e telefono, almeno uno con fuso dichiarato) e alcune foto senza data: importazione, correzione con allineamento, tagging da tastiera con set esclusivo, collegamento di un'identità, export con tutte le opzioni. Verifica: nomi e ordine nel risultato corrispondono alla sequenza vista in app; `manifest.csv` si apre in un foglio di calcolo; nessun originale modificato (hash invariati).
- [ ] 8.2 Prova di persistenza delle identità e dei set: cambiare `data_dir` verso una libreria vuota e importare foto delle stesse persone. Verifica: identità e set presenti, volti riconosciuti e collegati senza nominarli di nuovo.
- [ ] 8.3 Aggiornare `README.md` (nuovi moduli, posizione di `app.db`, dati biometrici) e verificare la build Linux e Windows con `make build`. Verifica: `make build` termina con successo su entrambe le piattaforme o in CI.

## Workflow follow-up

- Archiviare il change dopo la revisione, sincronizzando le spec in `openspec/specs/`.
