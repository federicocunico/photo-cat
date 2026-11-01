# Design

## Context

Stato attuale rilevante (branch `main`, vedi proposal.md per la motivazione):

- **Luogo**: `pc-ai/src/geo.rs::place_from_path` sceglie il segmento più profondo non generico (o il primo che corrisponde a una città). `pc-store::repo::meta::set_location` supporta già `LocationSource::Manual` e lo protegge dalle sovrascritture automatiche, ma nessun comando IPC lo usa: dall'interfaccia il luogo non si modifica. Il facet `places` e il filtro `LibraryFilter.places` esistono.
- **Bulk**: `pc-jobs/src/bulk.rs::BulkAction` (tag, cataloghi, `SetLabel`, `AssignPerson`, `RemoveLibrary`) con undo in `pc-store/src/bulk.rs::UndoAction` raggruppabile via `with_undo_group`.
- **Filtri per cartella**: assenti; la sidebar elenca solo le sorgenti (`listSources`).
- **Apertura file**: unico precedente è `open_logs` in `app/src-tauri/src/diagnostics.rs` (`explorer.exe`/`open`/`xdg-open` via `std::process::Command`). La griglia (`Grid.tsx`) gestisce solo il clic singolo; nessun menu contestuale.
- **Soglie**:
  - CLIP: `Group.threshold` nella tassonomia (JSON salvato nel DB, versionato); softmax per gruppo in `classify.rs::labels`. `save_settings` sovrascrive le soglie di tutti i gruppi quando cambia `ai.confidence_threshold`.
  - Persone (YOLO): `PERSON_THRESHOLD = 0.4` costante in `analyze.rs`; `media_people.boxes` salva i riquadri **con punteggio**.
  - Volti: soglia del rilevatore `0.8` costante in `analyze.rs`; dimensione minima già in `ai.face_min_size_px`; somiglianza `faces::SAME_PERSON = 0.363` costante usata da `cluster.rs`, `face_analysis.rs`, `analyze.rs`.
  - Gli embedding CLIP (512-d) sono salvati in `embedding`; gli embedding dei volti e la loro qualità sono salvati con i volti.
- Il change in corso `tags-identities-sequence` porta lo schema a v5 e tocca Inspector, BulkBar, Impostazioni, i18n, persone/identità.

## Goals / Non-Goals

**Goals:**
- Rendere raggiungibili dall'utente le correzioni del luogo (singola, bulk, rinomina, per cartella) riusando la protezione `manual` già presente.
- Ridurre alla fonte l'errore "luogo = nome del dispositivo".
- Aprire i file con applicazioni esterne in modo sicuro e multipiattaforma (Windows, macOS, Linux).
- Spostare tutte le soglie dell'automatismo in configurazione e applicarle senza ri-eseguire i modelli quando i dati salvati lo permettono.

**Non-Goals:**
- Nessuna modifica dei file originali (né scrittura del luogo negli EXIF).
- Nessun visualizzatore interno a schermo intero (il doppio clic delega a un'app esterna).
- Nessuna gerarchia di luoghi (paese/regione/città) né geocodifica inversa online.
- Nessuna soglia per singola identità: appartiene a `tags-identities-sequence`.
- Nessuna migrazione dello schema di `catalog.db`.

## Decisions

### 1. Nessuna migrazione di schema
Le soglie per categoria vanno in `taxonomy::Category.threshold: Option<f32>` (JSON della tassonomia, `#[serde(default)]`), le altre soglie, i nomi di cartella ignorati e le applicazioni esterne in `config.toml`. Il filtro per cartella usa il percorso già presente in `media.path`.
*Alternativa scartata*: tabella `thresholds` dedicata — richiederebbe una migrazione che collide con la v5 dell'altro change.

### 2. Struttura della configurazione
```toml
[ai]
confidence_threshold = 0.25          # predefinito proposto ai nuovi gruppi
ignored_folder_names = ["Selezionate"]
[ai.thresholds]
person_detection = 0.4
face_detection   = 0.8
face_similarity  = 0.363

[[external_apps]]
id = "lightroom"; name = "Lightroom"; path = "C:\\...\\Lightroom.exe"; args = ["{file}"]
[ui]
double_click = "system"              # "system" | "none" | id di external_apps
```
Tutti i campi nuovi hanno `#[serde(default)]`, quindi i `config.toml` esistenti restano validi. `Config::validate` controlla gli intervalli 0–1, l'unicità degli `id` e che `double_click` punti a un'app esistente. Rischio downgrade: `deny_unknown_fields` fa rifiutare il file a una versione precedente → vedi Rischi.

`save_settings` smette di copiare `confidence_threshold` in tutti i gruppi: le soglie dei gruppi si modificano solo dall'editor delle soglie/tassonomia. `confidence_threshold` resta come valore iniziale dei gruppi nuovi.

### 3. Rivalutazione delle soglie senza modelli ("retune")
Nuovo job `retune` in `pc-ai` (stesso schema dei job esistenti, idempotente e annullabile) con tre fasi indipendenti, eseguite solo se la soglia corrispondente è cambiata:
- **CLIP**: per ogni media con embedding della versione corrente del modello, ricalcola `classify::labels` usando i vettori dei concetti dalla cache testuale (`text_cache`); se la cache non è disponibile per la versione della tassonomia viene ricalcolata (solo encoder testuale, poche centinaia di prompt). Le etichette con origine manuale non si toccano (stessa logica della `Correzione manuale prevale`). La versione della tassonomia viene incrementata come oggi, così `stale_count` resta coerente; i media rivalutati vengono marcati con la nuova versione per non risultare "da rianalizzare".
- **Persone**: ricalcola `count` filtrando `media_people.boxes` per punteggio.
- **Volti**: i volti sotto la nuova soglia di rilevamento sono esclusi dal raggruppamento (flag/filtro in lettura, non cancellati) e il job di cluster esistente viene rilanciato con la nuova `face_similarity`.

Il minimo salvato: per persone e volti il rilevatore scarta sotto la soglia al momento dell'analisi. Per rendere possibile abbassare la soglia senza rianalisi, l'analisi salva d'ora in poi i risultati con un **pavimento fisso** (persone ≥ 0,25, volti ≥ 0,6) e applica la soglia configurata in lettura/conteggio. I risultati analizzati prima di questo change hanno come minimo la vecchia costante (0,4 / 0,8): se l'utente scende sotto, la UI avvisa e propone "Rianalizza" (scenario "Soglia sotto il minimo salvato"). Il pavimento usato viene registrato nella versione del risultato (`model_ver`/`stage` già esistenti, suffisso del parametro) per sapere quali file servono.

*Alternativa scartata*: applicare le soglie solo alle analisi future — l'utente non vedrebbe effetto sulla libreria esistente, contraddicendo l'obiettivo di "tunare".

### 4. Anteprima dell'effetto
Comando `preview_thresholds(new) -> ThresholdImpact { media_labels_changed, media_people_changed, faces_excluded, clusters_estimate }`, eseguito in `spawn_blocking` sugli stessi dati del retune ma senza scrivere. Per il raggruppamento il conteggio esatto richiederebbe di rieseguire il clustering: si mostra il numero di coppie volto–persona sotto la nuova soglia come stima, etichettata come tale. Su librerie molto grandi l'anteprima CLIP lavora per blocchi e la UI mostra un indicatore; l'utente può salvare senza attendere.

### 5. Luogo: comandi e undo
- `pc-store::bulk::set_place(c, ids, Option<PlaceInput>)`: `Some` scrive `source = manual` (nome, eventualmente lat/lon/paese se scelto dal gazetteer); `None` = "ripristina automatico", cioè cancella la riga manuale e ricalcola subito il luogo da GPS/directory (operazione senza modelli: riusa `geo::nearest` e `place_from_path`).
- Nuova `UndoAction::LocationReplaced { ids, previous: Vec<LocationRow> }` (riga completa o assenza), inserita nel gruppo di undo come le altre.
- `BulkAction::SetPlace { place: Option<PlaceInput> }` in `pc-jobs` → passa per lo stesso piano dei job bulk (soglia `JOB_THRESHOLD`, esito riusciti/falliti).
- **Rinomina**: comando `rename_place(from, to)` che risolve gli id con `location.name = from` e delega a `set_place`; un'unica voce di undo. Conteggio mostrato con il facet già esistente.
- Dopo ogni modifica si aggiorna `media_fts` (campo `place`) come fa già `fts::refresh`.

### 6. Luogo da cartella: segmenti di dispositivo
`place_from_path` riceve un contesto `PlaceHints { camera_make, camera_model, ignored: &[String] }`. Un segmento è scartato se, dopo `fold`:
- è generico (lista attuale);
- coincide con la marca EXIF, con il modello EXIF o con il modello privato della marca (`NIKON D7000` → `d7000`), ignorando spazi, trattini e maiuscole;
- è in una lista di nomi di dispositivo (`telefono`, `phone`, `cellulare`, `smartphone`, `iphone`, `android`, `nikon`, `canon`, `sony`, `fuji`, `fujifilm`, `olympus`, `panasonic`, `gopro`, `drone`, `dji`, `whatsapp`, `screenshots`, …);
- è tra `ai.ignored_folder_names`.
La lista dei dispositivi resta conservativa per non scartare luoghi veri; il controllo sul modello EXIF copre i casi specifici. Cambiare `ignored_folder_names` propone di ricalcolare i luoghi con fonte `directory` (non quelli manuali o GPS) tramite la stessa routine di "ripristina automatico".

### 7. Filtro e albero cartelle
- `LibraryFilter.folder: Option<FolderFilter { path, recursive }>`. Ricorsivo: prefisso `m.path LIKE <path><sep>% ESCAPE '\'` (con escape di `%`/`_`). Non ricorsivo: stesso prefisso e nessun separatore nel resto del percorso (`instr(substr(m.path, length(<path>)+2), <sep>) = 0`). Su Windows il confronto è case-insensitive, su macOS/Linux no.
- `list_folders(source_id, parent: Option<String>) -> Vec<FolderNode { path, name, items, has_children }>`: un livello alla volta, raggruppando per il segmento successivo al prefisso. Valutare un indice su `media(path)` se l'EXPLAIN mostra scansioni complete (l'indice univoco sul percorso esiste già per "una sola volta per percorso").
- I cataloghi smart non ricevono la regola "cartella" in questo change (resta possibile via `text`/`path` già esistenti).

### 8. Apertura con applicazioni esterne
- Comando `open_with(ids: Vec<MediaId>, app: OpenTarget)` con `OpenTarget = System | App(id) | Reveal`.
- `App(id)`: `Command::new(path)` con gli argomenti di configurazione; `{file}` viene sostituito da ogni percorso come argomento separato. Se gli argomenti non contengono `{file}`, i percorsi vengono accodati. Con più file: un solo processo con tutti i percorsi (comportamento atteso da Lightroom/Photoshop/XnView); limite di 100 file per invocazione, oltre si chiede conferma.
- `System` e `Reveal`: si adotta `tauri-plugin-opener` (plugin ufficiale Tauri 2), che apre con l'app predefinita senza shell intermedia (`ShellExecuteW` su Windows, `open` su macOS, `xdg-open`/portal su Linux) e gestisce "mostra nella cartella" con il file evidenziato su Windows/macOS e via `org.freedesktop.FileManager1` su Linux. `std::process::Command` resta solo per le app configurate. *Alternativa scartata*: estendere il codice di `open_logs` — più codice specifico per piattaforma e nessun "reveal" con selezione del file.
- macOS: un percorso `.app` viene lanciato con `open -a <app> --args …`.
- Nessuna shell intermedia (`cmd /c`, `sh -c`): i percorsi non vengono mai interpretati (requisito "Apertura sicura").
- Prima dell'avvio si verifica `status != missing` e l'esistenza del file; gli errori finiscono in diagnostica con `code = "IO"` come per `open_logs`.
- La validazione dell'eseguibile avviene in `save_settings` (esiste ed è un file; su macOS può essere una directory `.app`).

### 9. UI
- **Inspector**: campo luogo con autocompletamento (facet `places` + `gazetteer.by_name`/prefisso), "Ripristina automatico" se la fonte è manuale.
- **BulkBar**: azioni `set_place` / `reset_place`.
- **Sidebar**: sorgenti espandibili con albero cartelle (lazy), interruttore "includi sottocartelle"; voce "Rinomina…" sul facet Luogo con dialog di conferma che riporta il conteggio.
- **Grid**: `onDoubleClick` → `open_with(focused, config.ui.double_click)`; `onContextMenu` → menu React (non menu nativo, per coerenza di stile e testabilità) con focus trap, tasto Menu/Maiusc+F10.
- **Settings**: sezione "Soglie AI" (slider + campo numerico per persone, volti, somiglianza; tabella gruppi/categorie con soglia ereditata mostrata in grigio), box anteprima, "Ripristina predefiniti"; sezione "Applicazioni esterne" (aggiungi con file picker di `tauri-plugin-dialog`, già presente; argomenti; scelta del doppio clic); campo "Cartelle da ignorare per il luogo".

### 10. Scelte fatte in implementazione
- **Pavimento nella versione**: il pavimento è il suffisso `|floor=<valore>` di `analysis_version.model_ver` (stadi `persons` e `faces`). `pending` e `stale_count` considerano uguali `<modello>` e `<modello>|floor=…`: le librerie esistenti non diventano "da rianalizzare". Senza suffisso il minimo è la vecchia costante (0,4 / 0,8).
- **Rianalisi sotto il pavimento**: il comando `reanalyze_below_floor` toglie la versione dello stadio ai soli file salvati con un minimo più alto della soglia in vigore e li passa al job di analisi (il normale "Rianalizza" salterebbe file già aggiornati).
- **Cache dei concetti testuali**: è valida finché non cambiano prompt e categorie attive (`Taxonomy::prompt_signature`); versione, etichette e soglie non la invalidano, così il retune CLIP non carica l'encoder di testo.
- **Retune CLIP**: rivaluta tutti i media con embedding del modello immagine corrente, a blocchi di 500, saltando quelli già marcati con la versione corrente della tassonomia (ripresa). Nessuna scrittura degli embedding.
- **Volti sotto soglia**: restano salvati; quelli con persona automatica la perdono e non entrano nei nodi del clustering. Le assegnazioni manuali restano.
- **Rinomina del luogo**: cambia solo nome e fonte (`manual`); coordinate e paese di ogni foto restano.
- **Nomi di dispositivo**: dalla lista del §6 sono esclusi "fuji" e "olympus" (possono essere luoghi, es. "Monte Fuji"); per quelle fotocamere vale il confronto con la marca EXIF. Una cartella è scartata se una sua parola è un nome di dispositivo (es. "altra Nikon", "iPhone di Giulia").
- **Filtro per cartella**: intervallo sull'indice univoco di `media_item.path` (`path >= <cartella>\` e `path < <cartella>]`); su Windows l'intervallo va dalla forma maiuscola a quella minuscola del prefisso, seguito da un confronto `NOCASE` esatto.
- **`tauri-plugin-opener`**: usato solo dal backend (comando `open_with`), quindi nessun permesso nuovo per la webview e nessuna voce in `capabilities` o nei `tauri*.conf.json`. Su Linux "Mostra nella cartella" passa da `org.freedesktop.FileManager1` (gestito dal plugin).
- **Impostazioni**: le soglie dei gruppi si modificano solo nella sezione "Soglie AI" (tolte dall'editor della tassonomia); "Applica soglie" salva configurazione e tassonomia e avvia il retune. Un unico pulsante "Salva" per le altre impostazioni.

## Risks / Trade-offs

- [Conflitti di merge con `tags-identities-sequence` (Inspector, BulkBar, Settings, i18n, `config.rs`)] → nessuna migrazione di schema; modifiche UI in componenti separati (`PlaceEditor`, `ThresholdsSection`, `ExternalAppsSection`, `GridContextMenu`) montati con poche righe nei file condivisi; chiavi i18n sotto namespace nuovi (`place.*`, `thresholds.*`, `externalApps.*`).
- [Downgrade dell'app con `config.toml` contenente campi nuovi → rifiutato per `deny_unknown_fields`] → accettato (comportamento già esistente per ogni nuovo campo); documentato nelle note di rilascio.
- [Pavimenti di salvataggio più bassi aumentano i riquadri salvati] → impatto trascurabile (pochi riquadri in più per foto, JSON compatto); i volti sotto soglia non vengono raggruppati né mostrati, solo conservati.
- [Rivalutazione CLIP su 100.000 foto] → solo prodotto scalare 512-d × N concetti per foto, a blocchi nel governor; job annullabile e riprendibile.
- [Euristica dei dispositivi che scarta un luogo vero (es. cartella "Canon" per una città)] → lista conservativa, il controllo per modello vale solo se coincide con l'EXIF, l'utente può correggere manualmente o rinominare.
- [App esterne che non accettano più file insieme] → limite per invocazione e documentazione degli argomenti; l'utente può configurare argomenti diversi.
- [Il clustering cambia dopo un retune e "Senza nome N" cambia numerazione] → persone con nome e correzioni manuali sono protette come oggi; i cluster senza nome sono per natura instabili.

## Migration Plan

- Nessuna migrazione dati. Al primo avvio i nuovi campi di configurazione assumono i predefiniti, equivalenti alle costanti attuali: il comportamento non cambia finché l'utente non tocca le soglie.
- Le analisi successive al rilascio salvano con i nuovi pavimenti; le librerie esistenti funzionano senza rianalisi, con l'unico limite di non poter scendere sotto le vecchie soglie senza rianalizzare.
- Rollback: revert del change; i luoghi manuali scritti restano validi (la sorgente `manual` esiste già), i campi extra di `config.toml` vanno rimossi a mano per una versione precedente.

## Open Questions

- Elenco definitivo dei nomi di dispositivo da scartare: da tarare sulle cartelle reali dell'utente che ha segnalato il problema (non cambia approccio né task).
- Se `tauri-plugin-opener` su Linux non riesce a evidenziare il file in "Mostra nella cartella" con alcuni file manager, ripiegare sull'apertura della sola cartella (già coperto dalla formulazione "dove il sistema lo consente").
