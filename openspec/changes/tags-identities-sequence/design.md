# Design

## Context

Stato attuale rilevante (vedi `proposal.md` per le motivazioni):

- **Tag**: tabelle `tag(id, name COLLATE NOCASE)` e `media_tag` in `catalog.db`; si creano solo con `bulk::add_tag`/`remove_tag` (`crates/pc-store/src/bulk.rs`), con annullamento tramite `undo_log`. Nessuna UI dedicata: si scrivono a mano in `Inspector.tsx` e `BulkBar.tsx`.
- **Persone**: `person`, `face`, `person_centroid`, `face_exclusion` in `catalog.db`. L'associazione automatica usa `people::named_match` sui centroidi delle persone con nome (`crates/pc-store/src/people.rs`). Tutto è legato alla libreria.
- **Cartelle**: `AppPaths` (`crates/pc-core/src/paths.rs`) mette DB, miniature, modelli, log e `config.toml` sotto un'unica radice dati, che l'utente può reindirizzare con `data_dir`. Non esiste oggi un posto "dell'app" che sopravviva al cambio di radice.
- **Date**: `media_dates.captured_at` è testo `AAAA-MM-GG HH:MM:SS` in ora locale senza fuso; `pc-ingest/src/metadata.rs` legge `DateTimeOriginal`/`DateTimeDigitized` ma non `OffsetTime*`. Marca e modello sono in `exif.camera_make/model`.
- **Export**: `pc-organize/src/export.rs` produce CSV/JSON come job (`pc-jobs`), con scrittura atomica; `organize.rs` sa già scegliere fra symlink, hard link e copia (`decide()`).
- **Geo**: `assets/models/geo/cities15000.tsv` è derivato da GeoNames ma ha scartato la colonna del fuso IANA.
- **Mockup**: `mockups/*.html` (statici, apribili nel browser) e `mockups/src/*.dc.html` (sorgenti del canvas). Sono il riferimento visivo per i task UI.

## Goals / Non-Goals

**Goals:**
- Un unico archivio dell'app (`app.db`) per identità e set di tag, fuori dalla radice dati reindirizzabile.
- Istante reale materializzato e indicizzato in `catalog.db`, ricalcolabile in blocco quando cambia una correzione di gruppo.
- Export di sequenza come nuovo job in `pc-organize`, riusando job, governor, scrittura atomica e scelta del collegamento già esistenti.

**Non-Goals:**
- Scrivere metadati o rinominare gli **originali**.
- Sincronizzare `app.db` tra computer (solo esporta/importa manuale).
- Formati di export diversi da JPEG ricodificato e copia dell'originale (niente HEIC/AVIF/WebP in uscita).
- Riconoscimento di identità nei video o tipi di media diversi dalle foto.
- Riordino manuale arbitrario della sequenza (drag di foto datate): l'ordine deriva sempre dall'istante; solo le foto senza data si collocano a mano.

## Decisions

### 1. Archivio dell'app: `app.db` nella cartella di configurazione

Nuovo DB SQLite `app.db` in `ProjectDirs::config_dir()` (Windows `%APPDATA%\photocat\photo-cat\config`, Linux `~/.config/photo-cat`), esposto da `AppPaths::app_store()`. Questo percorso **non** segue `data_dir`: è il motivo per cui sopravvive al cambio di libreria. `PHOTO_CAT_DATA_DIR` (installazioni portabili e test) lo colloca invece sotto `<radice>/app/`, così i test restano isolati.

Contenuto (migrazioni proprie, stesso meccanismo `PRAGMA user_version` di `migrate.rs`, in un modulo `pc-store/src/app_store/`):
- `identity(uid TEXT PK, name UNIQUE NOCASE, short_name, grp, threshold REAL, created_at)`, `identity_alias(uid, alias)`.
- `identity_face(id, uid, crop BLOB, model_ver, embedding BLOB, source TEXT)`: il ritaglio allineato 112×112 usato da SFace è conservato per poter ricalcolare l'embedding quando cambia la versione del modello; nessuna foto originale.
- `tag_set(id, name UNIQUE, exclusive, is_base, position)`, `tag_set_item(set_id, position 1..9, tag_name NOCASE)`, `tag_style(tag_name NOCASE PK, color)`.
- `app_kv` per il set attivo e preferenze del modulo Sequenza (soglia sessioni, ultimo modello di nome).

Collegamento con la libreria: colonna `person.identity_uid TEXT NULL` in `catalog.db` (UUID, non id intero, perché i due DB evolvono separatamente). Il nome mostrato di una persona collegata è letto da `app.db`; `person.name` resta come copia per ricerca/filtri e viene riallineato quando l'identità cambia nome.

*Alternative scartate*: identità in `config.toml` (blob biometrici e relazioni non adatti a TOML); identità in `catalog.db` con export automatico (non soddisfa "permanente per l'app"); un file per identità (complica unione e importazione).

### 2. Riconoscimento con le identità

`named_match` viene esteso: oltre ai centroidi delle persone con nome della libreria confronta i centroidi dei volti di riferimento di ogni identità (per la stessa `model_ver`), usando la soglia dell'identità. Se vince un'identità non ancora collegata in questa libreria, si crea una persona collegata ad essa. Le esclusioni (`face.rejected`, "non è questa persona") restano per persona e valgono anche per l'identità collegata. Con un nuovo modello, gli embedding dei riferimenti si ricalcolano dai ritagli nel job di analisi volti.

"Elimina tutti i dati dei volti" (`people::clear`) cancella anche `identity_face` in `app.db`, lasciando le identità.

### 3. Tag: entità nella libreria, set e colori nell'app

I tag restano in `catalog.db` (le assegnazioni sono della libreria). Set, posizioni e colori stanno in `app.db` e referenziano i tag **per nome** (senza distinzione di maiuscole): così il set Base e gli altri set valgono in ogni libreria, e un tag del set inesistente in libreria viene creato al primo uso. Rinomina e unione aggiornano sia `catalog.db` sia i riferimenti in `app.db`; l'annullamento ripristina entrambi (voce `undo_log` con payload che include le righe dei set coinvolti).

Esclusività: `bulk::add_tag` riceve un parametro `exclusive_siblings: &[String]` calcolato dal livello Tauri a partire da `app.db`; nella stessa transazione e nello stesso gruppo di annullamento rimuove gli altri tag del set dai file. `pc-store` resta così ignaro dei set.

Hotkey: un hook UI condiviso (`useTagHotkeys`) in Libreria e Sequenza, ignorato quando il fuoco è in `input`, `textarea`, `select` o `contenteditable`; invia un'unica azione bulk sulla selezione corrente (inclusa la selezione "tutti i risultati" già supportata).

### 4. Istante reale materializzato

Migrazione `schema_v5.sql` di `catalog.db`:
- `media_dates` + `declared_offset_min INTEGER NULL`, `device TEXT NULL` (marca + modello normalizzati), `instant_utc INTEGER NULL` (ms, già corretto), `instant_presumed INTEGER NOT NULL DEFAULT 0`, `local_offset_min INTEGER NULL` (fuso dell'ora mostrata). Indice su `instant_utc`.
- `source_group(id, device, declared_offset_min, display_tz TEXT NULL, display_offset_min INTEGER NULL, shift_s INTEGER NOT NULL DEFAULT 0, UNIQUE(device, declared_offset_min))` e `media_dates.group_id`.
- `media_time_placement(media_id PK, instant_utc, local_offset_min)` per le foto senza data collocate a mano.

Calcolo: `instant_utc = parse(captured_at) − offset + shift_s`, dove `offset` è quello dichiarato, altrimenti quello di `display_tz` del gruppo alla data dello scatto (`chrono-tz`), altrimenti il fuso di sistema (e `instant_presumed = 1`). Una modifica a un gruppo esegue un `UPDATE` in blocco sui suoi membri: con l'indice per `group_id` è una singola istruzione anche su 100.000 file. Le correzioni passano da `undo_log`.

`captured_at` non cambia significato (resta l'ora letta dal file), quindi export CSV, filtri esistenti e cataloghi smart non si rompono; i filtri per data passano a usare l'ora locale corretta derivata da `instant_utc + local_offset_min`.

*Alternativa scartata*: calcolare l'istante a ogni query. Ordinare 100.000 righe con fusi IANA in SQL non rispetta il requisito di 1 s sui filtri.

### 5. Fuso dal GPS

Si rigenera `cities15000.tsv` aggiungendo la colonna del fuso IANA (presente nel dump GeoNames originale) e si aggiorna lo SHA-256 nel manifest. `Gazetteer` espone `timezone_at(lat, lon)` sulla città più vicina. Il suggerimento: per ogni gruppo, si prendono le foto con GPS di altri gruppi entro ±30 minuti dagli scatti del gruppo; se la maggioranza indica un offset diverso dall'ora locale del gruppo, si propone quel fuso con il numero di scatti coinvolti.

*Alternativa scartata*: crate con poligoni dei fusi (`tzf-rs`): più preciso ai confini, ma diversi MB in più e un secondo dataset da mantenere; la città più vicina è sufficiente per un suggerimento che l'utente conferma.

### 6. Allineamento e collocazione

Allineamento: l'utente sceglie foto A (gruppo riferimento) e B; si propone `shift_s(B) += instant(A) − instant(B)`, mostrato in anteprima prima di applicarlo. Collocazione di una foto senza data: istante = punto medio tra la foto precedente e la successiva (o +1 s se manca la successiva), offset locale della precedente; scritto in `media_time_placement`, che ha precedenza sul calcolo del gruppo.

Sessioni: calcolate in query con `LAG(instant_utc)` e soglia da `app_kv` (predefinita 2 ore); luogo della sessione = luogo più frequente.

### 7. Export della sequenza (`pc-organize/src/sequence.rs`)

Nuovo job `sequence_export` con fasi: verifica preliminare → codifica (parallela entro il budget del governor) → per-tag → manifest → zip.

- **Nomi**: parser del modello con token tipizzati e test unitari; `{seq:0Nd}` con N 1–9; un token vuoto elimina anche il separatore (`_`, `-`, spazio, `.`) che lo precede; sanificazione per Windows e Linux (caratteri riservati, nomi riservati come `CON`, punti finali); suffisso `-2`, `-3` deterministico in ordine di sequenza.
- **Atomicità**: si scrive tutto in `<dest>/.photo-cat-export-<job>/`; al termine si spostano `sequenza/`, `per-tag/`, `manifest.csv` e gli zip nella destinazione con rinomine sullo stesso volume. In annullamento o errore la cartella temporanea viene rimossa. Se la destinazione contiene già uno di questi nomi, la verifica preliminare blocca l'avvio.
- **Decodifica**: si riusa la decodifica a piena risoluzione di `pc-ingest` (inclusi HEIC via libheif e RAF), esposta come funzione pubblica che restituisce pixel RGB(A) a 8 o 16 bit, profilo ICC e orientamento.
- **Codifica**: crate `jpeg-encoder` (Rust puro) perché permette crominanza 4:4:4 (`SamplingFactor::F_1_1`), incorporamento ICC e segmenti APP arbitrari. *Scartato* l'encoder di `image`, che non espone il campionamento della crominanza né i segmenti APP.
- **Verifica**: SSIM sulla luminanza (finestre 8×8, a piena risoluzione) e PSNR su RGB implementati in un modulo dedicato con test su immagini sintetiche; soglie predefinite SSIM 0,985 e PSNR 42 dB, configurabili. Se non superate: qualità +1 fino a 100. *Scartato* `image-compare`: dipendenze pesanti per due metriche semplici.
- **Metadati**: blocco EXIF costruito con il writer di `kamadak-exif` (già dipendenza di `pc-ingest`): `DateTimeOriginal` e `OffsetTimeOriginal` corretti, `Make`, `Model`, `LensModel`, GPS, `Orientation = 1` dopo aver applicato la rotazione ai pixel. Pacchetto XMP generato come testo: `dc:subject` per i tag del set, `Iptc4xmpExt:PersonInImage` per le identità.
- **Per-tag**: hard link verso i file in `sequenza/` quando `organize::decide()` lo consente sul volume di destinazione, altrimenti copie (la stima dello spazio ne tiene conto).
- **Zip**: crate `zip` con `CompressionMethod::Stored` e ZIP64 abilitato (gli archivi superano facilmente 4 GB).
- **Spazio**: stima = Σ(dimensione stimata del JPEG: pixel × fattore per qualità calibrato in test) × (1 + zip) + copie per-tag se senza hard link; confronto con lo spazio libero del volume.

### 8. IPC e UI

Nuovi comandi Tauri in `app/src-tauri/src/` (`tags.rs`, `identities.rs`, `timeline.rs`, `sequence.rs`), tipi esportati con `specta` come gli esistenti. Nuovi moduli UI `tags`, `identities`, `sequence` (export come vista interna di `sequence`), voci in `modules/registry.ts` e `ModuleId`. La griglia della Sequenza riusa la virtualizzazione di `library/Grid.tsx`. `PeoplePage` sostituisce il campo nome libero con un selettore di identità (crea se assente). I mockup in `mockups/` fissano layout, testi e stati; i colori arrivano dai token esistenti (`theme/tokens.ts`), aggiungendo solo la palette dei tag e i colori per dispositivo.

## Risks / Trade-offs

- [Due DB che possono divergere (identità rinominata mentre una libreria è chiusa)] → `person.name` viene riallineato all'apertura della libreria; il riferimento è sempre l'UUID.
- [`app.db` contiene dati biometrici fuori dalla radice dati, che l'utente potrebbe non aspettarsi] → la schermata Identità indica dove si trova l'archivio, "Elimina tutti i dati dei volti" lo include, l'export delle identità permette di escludere i volti.
- [Fuso dalla città più vicina sbagliato vicino ai confini] → è solo un suggerimento da confermare, con il numero di scatti mostrato.
- [Verifica SSIM a piena risoluzione lenta su 900 foto da 40 MP] → parallelismo controllato dal governor; la verifica è disattivabile; misurare durante i task e documentare i tempi.
- [Writer EXIF di `kamadak-exif` marcato sperimentale] → test di round-trip (scrittura e rilettura con lo stesso crate e con il lettore di `pc-ingest`); in caso di limiti, ripiegare su un blocco TIFF minimale scritto a mano per i soli tag elencati.
- [Hard link e rinomina finale falliscono su FAT/exFAT o tra volumi] → la cartella temporanea è dentro la destinazione (stesso volume); per-tag ricade su copie con avviso nella verifica preliminare.
- [Migrazione v5 su librerie grandi] → le nuove colonne si riempiono con un job in background (`time_backfill`) che rilegge solo l'EXIF; finché non è completo, le foto non ancora elaborate usano il fuso di sistema e risultano presunte.

## Migration Plan

1. `catalog.db` → v5 con backup automatico (meccanismo esistente). Nessun dato rimosso.
2. Al primo avvio si crea `app.db` con il set "Base" (tag predefiniti: Preferite, Da stampare, Da scartare, Famiglia, Viaggi, Lavoro — modificabili).
3. All'apertura di una libreria, se `settings_kv` non ha il marcatore `identities_migrated`, ogni persona con nome viene collegata a un'identità con lo stesso nome (creata se assente); i centroidi non vengono copiati come riferimenti, l'utente sceglie i volti di riferimento.
4. Job `time_backfill` accodato una volta per libreria per leggere fuso e dispositivo e calcolare `instant_utc`.
5. Rollback: una versione precedente rifiuta un `catalog.db` v5 (comportamento esistente di `migrate`); si ripristina il backup creato al punto 1. `app.db` viene semplicemente ignorato dalle versioni precedenti.

## Open Questions

- Elenco definitivo dei tag del set "Base" predefinito: modificabile dall'utente, si può decidere in implementazione.
- Formato del file di esportazione delle identità (JSON con blob in base64 o archivio zip con ritagli): non cambia i requisiti, si decide nel task relativo.
- Fattore di stima della dimensione JPEG per qualità: da calibrare sulle immagini di test durante l'implementazione.
