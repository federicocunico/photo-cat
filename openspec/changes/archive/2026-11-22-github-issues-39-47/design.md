# Design

## Context

Lo stato attuale rilevante:

- **Selezione**: in Libreria passa da `useLibrary` (`selectAll`/`invertSelection` con una `SelectionBase` calcolata dal backend tramite `librarySelection(filter)`) ed è esposta da `useSelectItems`. Le altre griglie hanno ciascuna una selezione locale costruita con `clickSelect`/`rectangleSelect`, senza "seleziona tutto": `FaceGrid` in Persone, `SequencePage`, le sezioni del modulo AI.
- **Volti**: `people::group_faces` ha già `FaceSort::{QualityDesc, CapturedDesc}` con cursore per pagina, ma la UI usa solo la qualità. `FaceGrid` non ha un menu contestuale verso la foto. `external-apps` offre già "Apri con" e "Mostra nella cartella" per gli id dei media.
- **Date**: `media_dates.captured_at` è l'ora letta dal file e `timeline.rs` ne ricava `instant_utc`, materializzato e indicizzato, applicando le correzioni di gruppo. Ordinamento, filtro per data, Sequenza e regole smart leggono `instant_utc` o `captured_at`.
- **Import**: il riepilogo dell'ultimo aggiornamento è salvato in `kv` (`import.last_update.<source>`) e quindi sostituito a ogni esecuzione. L'avviso "N mancanti" della cartella è invece il conteggio vivo dei file `missing` e resta finché esistono file mancanti: è il problema di #45. Il contatore errori globale (`errorCount`) conta tutto il registro errori.
- **Monitoraggio**: `pc-ingest::watch::SourceWatcher` funziona già. L'issue #46 è un problema di comprensione, non un difetto.
- **Tasti**: 1–9 e 0 sono già usati per il tagging del set attivo (`useTagHotkeys`).

## Goals / Non-Goals

**Goals:**
- un solo meccanismo di selezione riusabile da tutte le griglie, scalabile fino a 100.000 elementi;
- valutazione, colore e data manuale come dati di libreria di prima classe: filtro, regole smart, bulk, annulla;
- avvisi dell'import e contatore errori che riflettono ciò che l'utente non ha ancora visto.

**Non-Goals:**
- classificazione automatica del colore (esclusa su indicazione dell'utente);
- scrittura di valutazione o data nei file (XMP/EXIF): gli originali restano in sola lettura;
- modifica dei pixel per bianco e nero o seppia;
- monitoraggio delle cartelle ad app chiusa.

## Decisions

### D1. Selezione: base dal backend più eccezioni, estesa a tutte le griglie
Il modello `SelectionBase` (insieme definito da filtro e snapshot, più elementi aggiunti e tolti) viene estratto in un hook generico `useGridSelection(scope)`. Ogni vista fornisce:

- l'ordine visibile per Shift+clic;
- una funzione backend che restituisce la base dell'insieme completo.

Le basi per vista sono:

- **Libreria, cataloghi, catalogo di sistema, escluse**: `librarySelection(filter)`, già esistente.
- **Volti**: nuovo `faceSelection(groupRef, discardedOnly)`, che restituisce lo snapshot degli id dei volti.
- **Sequenza**: `sequenceSelection(scope)`.
- **AI**: i risultati e i simili riusano il filtro della Libreria che già producono.

Le scorciatoie (Ctrl/Cmd+A, Ctrl+Maiusc+I, Esc) sono registrate una volta nella shell e inoltrate alla griglia attiva; vengono ignorate se `document.activeElement` è un campo modificabile.

*Alternativa scartata:* caricare tutti gli id nella UI. Supera la soglia di 100.000 elementi della spec e duplica la logica già validata in Libreria.

### D2. Valutazione e colore come colonne di `media_item`
La migrazione v14 aggiunge:

- `rating INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5)`;
- `color_trait TEXT NULL CHECK (color_trait IN ('bw','color','faded','sepia'))`;
- gli indici `media_rating` e `media_color_trait`.

Il bulk usa `with_undo_group` con nuove varianti `UndoAction::{SetRating, SetColorTrait}` che salvano i valori precedenti per id. I filtri e le regole smart estendono `LibraryFilter`/`compile_filter` con `rating: {op, value}` e `color_trait: Vec<Option<ColorTrait>>`, dove `None` significa "Non indicato".

*Alternativa scartata:* modellare il colore come tag di un set esclusivo "Colore". Sarebbe più rapido da fare, ma l'utente potrebbe rinominare o eliminare i valori, quindi niente filtro stabile né traduzione. Si scarta anche di tenere i valori in una tabella separata: sono un valore per foto, sempre letto nei filtri, e una colonna evita join.

### D3. Data manuale con precisione, integrata nell'istante reale
In `media_dates` si aggiungono `manual_at TEXT NULL` (`AAAA-MM-GG HH:MM:SS`, completata al primo istante del periodo) e `manual_precision TEXT NULL CHECK (IN ('day','month','year'))`. `DateSource` riceve la variante `Manual`.

`timeline.rs` calcola `instant_utc` così:

- con `manual_at` presente, `instant_utc = manual_at` interpretata nel fuso di sistema (o del gruppo), senza applicare `shift_s` del gruppo, come richiede lo scenario "Correzioni di gruppo";
- altrimenti la formula resta quella attuale.

Il filtro per data gerarchico usa la precisione per non collocare una data "solo anno" sotto un mese: le foto con precisione `year` compaiono nel nodo anno come "Anno senza mese".

Il ripristino azzera le due colonne e ricalcola l'istante. L'annulla segue lo stesso schema (`UndoAction::SetManualDate`).

*Alternativa scartata:* sovrascrivere `captured_at`. Si perderebbe la data letta dal file e non si potrebbe ripristinarla.

### D4. Volti: ordinamento per data ascendente e intestazioni per anno
Si aggiunge `FaceSort::CapturedAsc` con un cursore simmetrico a `CapturedDesc`. Entrambi ordinano per `instant_utc` (quindi rispettano le date manuali), mettono in fondo i volti senza data e usano `face.id` come spareggio. Le intestazioni per anno sono calcolate nella UI dai volti caricati. I conteggi per anno arrivano da una query leggera `group_face_years(group)` (`strftime('%Y', …)`, `count`), così sono corretti anche per le pagine non ancora caricate. La scelta è salvata in `ui_prefs` (`people.faceSort`).

### D5. Menu contestuale dei volti
`FaceGrid` riusa il componente `Menu` del `GridContextMenu`. Le voci "Apri con ▸" e "Mostra nella cartella" ricevono gli id dei media distinti della selezione di volti, deduplicati. "Apri la foto" apre il `viewer` esistente con un parametro `highlightFace`. "Mostra in Libreria" naviga con `Destination` alla Libreria, con il filtro di cartella della foto e `focusMedia`, riusando il meccanismo di `openPersonPhotos`.

### D6. Avvisi dell'import: soglia di riconoscimento per cartella
Per ogni sorgente si salva in `kv` `import.ack.<source>` = `{missing_ids_max, failed_job_id}` al momento di "Ignora". Le regole di visualizzazione:

- l'avviso dei mancanti mostra il numero di file `missing` diventati tali dopo la soglia, cioè con `missing_since` posteriore; se la colonna non esiste viene aggiunta nella migrazione v14 e valorizzata dall'import quando un file passa a `missing`;
- l'avviso errori usa solo il conteggio `failed` del riepilogo dell'ultimo aggiornamento, già sostituito a ogni esecuzione, e sparisce se il job riconosciuto coincide con l'ultimo.

Il menu della cartella continua a offrire "Ricollega" e "Rimuovi i mancanti" finché il conteggio vivo è maggiore di 0.

### D7. Errori archiviati
La migrazione v14 aggiunge a `error_log` `archived_at TEXT NULL` e un indice parziale sulle voci non archiviate. `ErrorQuery` riceve `archived: Option<bool>`, con predefinito `false`. Nuovi comandi:

- `archive_errors(query | ids)`;
- `unarchive_errors(ids)`.

`errorCount` (eventi live e conteggio iniziale) conta solo le voci non archiviate. Il file di log non viene toccato.

### D8. Monitoraggio spiegato
Cambiano le etichette i18n ("Aggiorna automaticamente") e si aggiungono un tooltip e una voce in `in-app-guidance`. `restart_watcher` pubblica uno stato per sorgente (`active`, `unreachable`, `error`) e `last_auto_update_at`, salvato in `kv` quando un import parte dal watcher. La UI lo legge con `sourceWatchStatus()`.

## Risks / Trade-offs

- **[Rischio]** Ctrl+A globale può entrare in conflitto con i campi di testo o con il visore. → Le scorciatoie vengono ignorate nei campi modificabili e nei dialoghi aperti, con test vitest dedicati.
- **[Rischio]** Ricalcolare `instant_utc` per date manuali in bulk su molte foto. → Si usa lo stesso ricalcolo a blocchi delle correzioni di gruppo, con job sopra la soglia "Esecuzione come job per selezioni grandi".
- **[Trade-off]** Una data con precisione "anno" viene ordinata al 1° gennaio, quindi prima delle foto precise dello stesso anno. È una scelta deterministica e documentata nello scenario della spec.
- **[Rischio]** Senza una colonna `missing_since` storica, i file già mancanti prima della migrazione non hanno data. → La migrazione la imposta all'ora della migrazione, così il primo "Ignora" li copre tutti.

## Migration Plan

La migrazione v14 è solo additiva: colonne con default, nessuna riscrittura dei dati esistenti, test di migrazione come per v11–v13. Per tornare indietro basta ignorare le colonne nuove, che hanno valori predefiniti neutri. Nessun file originale viene modificato.
