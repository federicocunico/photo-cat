# Proposal

## Why

Un utente che ha provato la versione attuale (`main`) ha segnalato due limiti concreti. Il primo: il luogo dedotto dal nome della cartella sbaglia con le strutture tipiche di un viaggio (`Viaggio Giappone\D7000`, `Viaggio Giappone\Telefono`): le foto finiscono catalogate con luogo "D7000" e oggi **non c'è modo di correggere il luogo**, né su una foto né su un gruppo, né di agire su una cartella intera. Il secondo: manca un collegamento con l'applicazione di fotoritocco/gestione che l'utente usa già; vorrebbe aprirvi una foto con doppio clic o con "Apri con" dal tasto destro.

In più, l'automatismo è poco governabile: la soglia di CLIP si regola solo per gruppo (e la soglia globale sovrascrive quelle dei gruppi), mentre le soglie di YOLO (persone), del rilevatore di volti e del raggruppamento sono costanti nel codice. Chi trova troppe o troppo poche catalogazioni automatiche non ha leve per correggere il comportamento.

## What Changes

- **Correzione manuale del luogo**: il luogo diventa modificabile nell'Inspector (singola foto) e tramite una nuova azione bulk "Imposta luogo" (selezione, risultati del filtro, cartella). Anche "Ripristina luogo automatico". Il luogo manuale prevale sulle analisi successive (già previsto dallo store, ora raggiungibile dall'utente). Annullabile come le altre azioni bulk.
- **Rinomina di un luogo**: dal facet "Luogo" della libreria, "Rinomina…" cambia in un colpo il nome del luogo per tutte le foto che lo hanno (es. "D7000" → "Giappone"), marcandolo come correzione manuale; con conteggio e annullamento.
- **Filtro e selezione per cartella**: le sorgenti nella sidebar si espandono nell'albero delle sottocartelle; scegliere una cartella filtra la libreria (con o senza sottocartelle), così da poter "Seleziona tutto" e applicare un'azione bulk all'intera cartella.
- **Luogo da cartella più intelligente**: nella deduzione del luogo dal percorso si scartano anche i segmenti che corrispondono a marca/modello della fotocamera della foto (EXIF) e a nomi di dispositivo comuni (telefono, iPhone, Nikon, D7000…), risalendo alla cartella superiore; in impostazioni l'utente può aggiungere nomi di cartella da ignorare.
- **Applicazioni esterne ("Apri con")**: in impostazioni si configurano una o più applicazioni esterne (nome, eseguibile, argomenti opzionali) e quale usare al doppio clic. Il doppio clic su una miniatura apre la foto con l'applicazione scelta (o con l'app predefinita del sistema); nuovo menu contestuale della griglia con "Apri con ▸", "Apri con l'app predefinita" e "Mostra nella cartella", anche su selezioni multiple.
- **Taratura dell'automatismo**: nuova sezione "Soglie AI" nelle impostazioni con:
  - soglia CLIP per gruppo e, opzionalmente, per singola categoria della tassonomia;
  - soglia di confidenza del rilevamento persone (YOLO);
  - soglia di confidenza del rilevamento volti e dimensione minima (già esistente);
  - soglia di somiglianza per il raggruppamento dei volti in persone e per l'associazione automatica ai nomi;
  - anteprima dell'effetto ("con queste soglie: N foto cambiano") e "Ripristina predefiniti".
  Le nuove soglie si applicano **senza ri-eseguire i modelli** quando possibile (rivalutazione da embedding e punteggi già salvati); abbassare una soglia sotto il minimo con cui i risultati sono stati salvati propone la rianalisi.
- La soglia globale "Soglia di confidenza" attuale diventa il valore predefinito proposto ai gruppi e non sovrascrive più in silenzio le soglie personalizzate.

## Capabilities

### New Capabilities

- `external-apps`: configurazione di applicazioni esterne e apertura dei file della libreria con esse (doppio clic, "Apri con", app predefinita del sistema, mostra nella cartella).

### Modified Capabilities

- `ai-classification`: soglie per categoria e rivalutazione senza rianalisi ("Classificazione zero-shot con confidenza"); soglia configurabile del rilevamento persone; deduzione del luogo da directory che ignora cartelle di dispositivo e nomi configurati ("Geolocalizzazione"); nuovo requisito sul pannello di taratura delle soglie.
- `face-recognition`: soglia di rilevamento volti configurabile ("Rilevamento volti"); soglia di somiglianza configurabile per raggruppamento e associazione automatica ("Raggruppamento in persone", "Identificazione delle persone").
- `bulk-operations`: azioni "Imposta luogo" e "Ripristina luogo automatico" ("Azioni bulk disponibili"); nuovo requisito "Rinomina di un luogo".
- `catalog-library`: filtro per cartella ("Ricerca e filtri"); nuovo requisito "Albero delle cartelle".
- `desktop-shell`: luogo modificabile nell'Inspector ("Pannello di ispezione"); impostazioni estese con soglie AI, cartelle da ignorare e applicazioni esterne ("Impostazioni"); nuovo requisito "Menu contestuale della griglia".

## Impact

- **Codice Rust**: `pc-core/src/config.rs` (nuove sezioni `ai.thresholds`, `ai.ignored_folder_names`, `external_apps`), `pc-ai` (`geo.rs` per i segmenti di dispositivo, `classify.rs` per le soglie per categoria, `analyze.rs`/`faces.rs`/`cluster.rs`/`face_analysis.rs` per leggere le soglie dalla configurazione, nuovo job di rivalutazione), `pc-ai/src/taxonomy.rs` (soglia opzionale per categoria), `pc-store` (`bulk.rs`: impostazione/rinomina del luogo con undo; `query.rs`: filtro per cartella e albero delle cartelle), `pc-jobs/src/bulk.rs` (nuove azioni), `app/src-tauri` (comandi per luogo, cartelle, apertura con app esterna, anteprima soglie; `settings.rs` non sovrascrive più le soglie dei gruppi).
- **UI**: `Inspector.tsx` (luogo modificabile), `BulkBar.tsx` (imposta/ripristina luogo), `Sidebar.tsx` (albero cartelle, rinomina luogo dal facet), `Grid.tsx` (doppio clic, menu contestuale), `SettingsPage.tsx` (sezioni "Soglie AI", "Applicazioni esterne", cartelle ignorate), i18n it/en.
- **Dati**: nessuna migrazione di `catalog.db`; le soglie per categoria stanno nella tassonomia (JSON già salvato), il resto in `config.toml`. Cambiare le soglie incrementa la versione della tassonomia come oggi.
- **Dipendenze**: nessuna nuova obbligatoria (apertura via `std::process::Command`, già usato per i log); eventuale `tauri-plugin-opener` da valutare nel design.
- **Coordinamento**: il change in corso `tags-identities-sequence` tocca anch'esso Inspector, BulkBar, Impostazioni, i18n e introduce una soglia di somiglianza per identità; questo change non introduce migrazioni di schema per non collidere con la sua v5, e la soglia di raggruppamento qui definita è il valore di libreria su cui le soglie per identità si sovrappongono.
