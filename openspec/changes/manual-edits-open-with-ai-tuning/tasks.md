# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Conflitti con `tags-identities-sequence`**: quel change modifica gli stessi file (`Inspector.tsx`, `BulkBar.tsx`, `SettingsPage.tsx`, i18n it/en, `pc-core/src/config.rs`). Prima di iniziare verificare se è già stato unito in `main`; se sì, fare il rebase di questo branch e partire da lì. Se no, tenere le modifiche nei componenti separati indicati nel design (`PlaceEditor`, `ThresholdsSection`, `ExternalAppsSection`, `GridContextMenu`), con poche righe nei file condivisi e chiavi i18n sotto namespace nuovi (`place.*`, `thresholds.*`, `externalApps.*`).
> - **Nessuna migrazione di schema**: questo change non deve toccare `pc-store/src/migrate.rs`, perché la v5 appartiene a `tags-identities-sequence`. Se un task sembra richiederla, fermarsi e chiedere.
> - **Nuova dipendenza**: per "Apri con" e "Mostra nella cartella" si aggiunge `tauri-plugin-opener` (design §8, task 6.1). Va registrato anche in `tauri.linux.conf.json` e nelle capability, tenendo allineata la configurazione Linux.
> - **Nomi di dispositivo da ignorare**: la lista del design §6 è un punto di partenza conservativo. Va tarata sulle cartelle reali dell'utente che ha dato il feedback (struttura `Viaggio\{Telefono, D7000, altra Nikon}`): prima di chiudere il task 4.1 chiedere all'utente esempi dei nomi delle sue cartelle.
> - **Soglie dell'altro change**: `tags-identities-sequence` introduce una soglia di somiglianza per singola identità. La `face_similarity` di questo change è il valore della libreria; quella per identità, quando esiste, ha la precedenza. Non implementarla qui.

## 1. Configurazione e tassonomia

- [x] 1.1 Estendere `pc-core/src/config.rs`: `AiConfig.thresholds { person_detection, face_detection, face_similarity }` (predefiniti 0,4 / 0,8 / 0,363), `AiConfig.ignored_folder_names`, `Config.external_apps: Vec<ExternalApp { id, name, path, args }>`, `UiConfig.double_click` (`system` | `none` | id app), tutti con `#[serde(default)]`; `validate()` rifiuta soglie fuori 0–1, id duplicati e `double_click` verso app inesistenti (design §2). Verifica: unit test in `config.rs` su un `config.toml` vecchio che si carica con i predefiniti, su soglia 1,5 rifiutata e su `double_click` orfano rifiutato; `cargo test -p pc-core` passa.
- [x] 1.2 Aggiungere `Category.threshold: Option<f32>` in `pc-ai/src/taxonomy.rs` con validazione 0–1 e applicarla in `classify::labels` (soglia categoria, altrimenti gruppo). Verifica: estendere `confidence_is_per_group_and_respects_thresholds` con lo scenario "Soglia per categoria" della spec e un test che una tassonomia JSON senza il campo si carica; `cargo test -p pc-ai classify taxonomy` passa.
- [x] 1.3 In `app/src-tauri/src/settings.rs` eliminare la copia di `confidence_threshold` su tutti i gruppi; il valore resta solo come predefinito dei gruppi nuovi. Rigenerare `bindings.ts`. Verifica: test (Rust o `SettingsPage.test.tsx` con `ipcMock`) che salvare un'impostazione qualsiasi lascia invariate le soglie dei gruppi (scenario "Soglie di gruppo preservate").

## 2. Soglie dei modelli dalla configurazione

- [x] 2.1 Sostituire `PERSON_THRESHOLD`, la soglia volti `0.8` in `analyze.rs` e gli usi di `faces::SAME_PERSON` in `cluster.rs`, `face_analysis.rs`, `analyze.rs` con i valori di `config.ai.thresholds` (`SAME_PERSON` resta solo come predefinito e per il self-test). Verifica: `cargo test -p pc-ai` passa, incluso un test di cluster che con soglia più alta separa due volti che con quella predefinita erano uniti.
- [x] 2.2 Salvare i risultati di persone e volti con i pavimenti fissi (persone ≥ 0,25, volti ≥ 0,6) applicando la soglia configurata in conteggio/raggruppamento, e registrare il pavimento usato nella versione del risultato (design §3). Verifica: test in `pc-ai` che una foto con riquadri 0,9 / 0,7 / 0,45 / 0,3 dà conteggio 3 con soglia 0,4 e 2 con soglia 0,6 senza rianalisi; test in `pc-store` che i volti sotto soglia non entrano nei nodi del cluster.

## 3. Rivalutazione e anteprima delle soglie

- [x] 3.1 Implementare il job `retune` in `pc-ai` con le fasi CLIP (da `embedding` + cache testuale, rispettando le etichette manuali e marcando la nuova versione della tassonomia), persone (da `media_people.boxes`) e volti (filtro + rilancio del cluster), eseguite solo per le soglie cambiate; annullabile e riprendibile come gli altri job. Verifica: test di integrazione in `crates/pc-ai/tests/` che, su un DB con embedding sintetici, alzare una soglia rimuove l'etichetta attesa senza caricare il modello immagine, lascia intatta una correzione manuale (scenario "Correzione manuale preservata") e azzera `stale_count`.
- [x] 3.2 Comando `preview_thresholds` (design §4) che restituisce foto con catalogazioni cambiate, foto con conteggio persone cambiato, volti esclusi e stima delle persone interessate, senza scrivere. Verifica: test Rust che confronta l'anteprima con l'esito effettivo del `retune` sullo stesso DB (numeri di catalogazioni e conteggi uguali).
- [x] 3.3 Rilevare quando una soglia scende sotto il pavimento con cui sono stati salvati i risultati e restituire il numero di file da rianalizzare. Verifica: test che con risultati salvati a 0,8 e nuova soglia volti 0,7 il comando riporta i file interessati (scenario "Soglia sotto il minimo salvato").
- [x] 3.4 Collegare `save_settings` e il salvataggio della tassonomia all'avvio del `retune` quando cambiano soglie. Verifica: test con `ipcMock` / test Rust che il salvataggio di una soglia crea un job `retune` visibile nella schermata Job.

## 4. Luogo: deduzione, correzione, rinomina

- [ ] 4.1 Estendere `geo::place_from_path` con `PlaceHints` (marca/modello EXIF, nomi di dispositivo, `ignored_folder_names`) e passare gli EXIF dalla fase di analisi (design §6). Verifica: test in `geo.rs` per gli scenari "Sottocartella del dispositivo", "Sottocartella del telefono", "Nome di cartella ignorato dall'utente" e che `Toscana\Siena` dia ancora "Siena".
- [x] 4.2 In `pc-store/src/bulk.rs` aggiungere `set_place(ids, Option<PlaceInput>)` (manuale o ripristino automatico con ricalcolo da GPS/directory), `UndoAction::LocationReplaced` e aggiornamento di `media_fts`. Verifica: test in `crates/pc-store/tests/` per impostazione, ripristino, "Annulla luogo" e ricerca testuale sul nuovo nome.
- [x] 4.3 Aggiungere `BulkAction::SetPlace` in `pc-jobs/src/bulk.rs` e il comando `rename_place(from, to)` (un solo gruppo di undo, unione con luogo esistente). Verifica: test per gli scenari "Luogo in bulk", "Rinomina da facet", "Unione con luogo esistente" e "Annulla rinomina"; esito riusciti/falliti riportato come le altre azioni bulk.
- [x] 4.4 Comando di suggerimento luoghi (luoghi in libreria + prefisso nel gazetteer) e ricalcolo dei luoghi con fonte `directory` quando cambiano `ignored_folder_names`. Verifica: test Rust che i luoghi manuali e GPS non vengono toccati dal ricalcolo.
- [x] 4.5 UI: componente `PlaceEditor` nell'Inspector con autocompletamento e "Ripristina automatico"; azioni `set_place` / `reset_place` in `BulkBar`; "Rinomina…" sul facet Luogo con dialog di conferma e conteggio; chiavi i18n it/en sotto `place.*`. Verifica: test in `Inspector.test.tsx`, `BulkBar.test.tsx` e `LibraryPage.test.tsx` con `ipcMock`; `npm --prefix app/ui test` passa.

## 5. Filtro e albero delle cartelle

- [x] 5.1 Aggiungere `LibraryFilter.folder { path, recursive }` in `pc-store/src/query.rs` (prefisso con escape, confronto case-insensitive solo su Windows) e `list_folders(source_id, parent)` un livello alla volta con conteggi (design §7). Verifica: test in `crates/pc-store/tests/query.rs` per "Filtro per cartella con sottocartelle", "Filtro per cartella esatta", percorsi con `%`/`_` nel nome e conteggi dell'albero; controllo con `EXPLAIN QUERY PLAN` che il filtro usa l'indice sul percorso.
- [x] 5.2 UI: sorgenti espandibili nella `Sidebar` con albero lazy e interruttore "includi sottocartelle"; il filtro cartella compare tra i filtri attivi ed è rimovibile; "Seleziona tutto" lavora sul filtro. Verifica: test in `LibraryPage.test.tsx` per espansione, filtro e scenario "Seleziona tutto in una cartella"; prova manuale su una libreria reale con struttura viaggio/dispositivo.

## 6. Applicazioni esterne e menu contestuale

- [x] 6.1 Aggiungere `tauri-plugin-opener` (Cargo, capability, `tauri.conf.json` e `tauri.linux.conf.json`) e il comando `open_with(ids, System | App(id) | Reveal)`: argomenti senza shell con sostituzione di `{file}`, `.app` su macOS via `open -a`, controllo dei file mancanti, limite di 100 file con conferma, errori in diagnostica (design §8). Verifica: unit test sulla costruzione degli argomenti (percorsi con spazi, `&`, virgolette, assenza di `{file}`, più file) e sul rifiuto dei file mancanti; prova manuale su Windows con un'app reale e su Linux con `xdg-open`.
- [x] 6.2 Validare in `save_settings` che l'eseguibile di ogni app esista (file, o directory `.app` su macOS). Verifica: test per lo scenario "Eseguibile inesistente".
- [x] 6.3 UI: `onDoubleClick` in `Grid.tsx` secondo `ui.double_click`; componente `GridContextMenu` (Apri con ▸, app predefinita, Mostra nella cartella, Imposta luogo…, azioni bulk comuni) con regole di selezione del clic destro, apertura da tasto Menu/Maiusc+F10 e navigazione con frecce; chiavi i18n sotto `externalApps.*`. Verifica: test con `ipcMock` per gli scenari "Clic destro su miniatura non selezionata", "Clic destro sulla selezione", "Apertura da tastiera" e "Doppio clic con applicazione configurata".

## 7. Impostazioni

- [x] 7.1 Sezione "Soglie AI" in `SettingsPage` (`ThresholdsSection`): persone, volti, dimensione minima, somiglianza; tabella gruppi/categorie con soglia ereditata visibile; anteprima da `preview_thresholds`; avviso e proposta di rianalisi sotto il pavimento; "Ripristina predefiniti". Verifica: test in `SettingsPage.test.tsx` per anteprima, valore non valido rifiutato e ripristino dei predefiniti.
- [x] 7.2 Sezione "Applicazioni esterne" (`ExternalAppsSection`): aggiunta con file picker di `tauri-plugin-dialog`, argomenti, eliminazione, scelta dell'azione del doppio clic; campo "Cartelle da ignorare per il luogo" con proposta di ricalcolo. Verifica: test in `SettingsPage.test.tsx` per aggiunta, errore di eseguibile inesistente e scelta del doppio clic.
- [ ] 7.3 Documentare nel `README.md` le soglie AI (significato, predefiniti, effetto senza rianalisi), la configurazione di "Apri con" con esempi per Windows/macOS/Linux e la correzione del luogo. Verifica: gli esempi di configurazione si caricano senza errori in un `config.toml` di prova.

## 8. Verifica d'insieme

- [x] 8.1 Eseguire `make test` (fmt, clippy, test Rust e UI) su Windows e Linux. Verifica: tutto verde.
- [ ] 8.2 Prova end-to-end sull'app reale con una cartella `Viaggio\{Telefono,D7000}`: luogo dedotto "Viaggio", rinomina e impostazione in bulk via albero cartelle con annullamento, doppio clic e "Apri con" verso un'app configurata, modifica di una soglia CLIP con anteprima e rivalutazione senza rianalisi. Verifica: tutti i passi riusciti, nessun errore nuovo in Diagnostica.
- [x] 8.3 Allineare le delta di `specs/` a quanto implementato e verificare che `openspec validate manual-edits-open-with-ai-tuning --strict` passi.

## Workflow follow-up

- Coordinare il merge con `tags-identities-sequence` (file condivisi: Inspector, BulkBar, SettingsPage, i18n, `config.rs`); rebase di questo branch dopo il merge dell'altro change.
- Archiviare il change dopo la revisione.
