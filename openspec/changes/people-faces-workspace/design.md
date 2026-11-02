# Design

## Context

Stato attuale su `main`:

- **Dati**:
  - `person(id, name, centroid, created_at, identity_uid)` (`schema_v1.sql:107`, v5); nessun flag "ignorato";
  - `face(id, media_id, bbox, embedding 128 f32, model_ver, quality, person_id, assigned_by 'auto'|'manual', rejected JSON)` (`schema_v1.sql:114`). `quality` è il solo punteggio del rilevatore (`face_analysis.rs:215`): niente nitidezza, dimensione o posa;
  - `face_exclusion(media_id, bbox)` per i volti rimossi (`schema_v3.sql:1`), `person_centroid(person_id, model_ver, vector)` (`schema_v3.sql:7`);
  - in `app.db`: `identity`, `identity_alias`, `identity_face` (ritaglio PNG 112×112 ed embedding).
- **Comandi**:
  - Persone: `list_people`, `face_count`, `person_faces(Option)` senza paginazione, `correct_person(Rename|Merge|Separate|Assign|Reject|Remove)`, `purge_faces` (`app/src-tauri/src/people.rs`);
  - Identità: `list/save/delete/merge_identities`, riferimenti, `link_person_identity` (unisce se l'identità è già collegata, `pc-store/src/people.rs:795`), `suggest_identities` per il "Chi è?", esporta/importa (`identities.rs`).
- **Analisi**:
  - `faces::detect_usable` filtra per lato minore e confidenza (`faces.rs:99-104`);
  - il ritaglio allineato 112×112 viene calcolato per l'embedding (`face_analysis.rs`), poi scartato;
  - il raggruppamento (`cluster.rs`) usa Chinese Whispers su un grafo kNN (HNSW sopra 64 volti), protegge persone con nome, volti manuali e rifiuti, e prima associa i volti a persone con nome e riferimenti delle identità (`named_match_refs`);
  - non esistono suggerimenti persona↔persona.
- **UI**:
  - `PeoplePage.tsx` (214 righe): griglia di schede, dettaglio con `<select>` per il bersaglio, `refetchInterval: 5000` su elenco e volti;
  - `IdentitiesPage.tsx` (756 righe) con editor, riferimenti e `Transfer`, e `WhoIs.tsx` (173 righe);
  - "Apri in Persone" (`IdentitiesPage.tsx:333-343`) chiama solo `setModule("people")`; `store/layout.ts` ha solo il precedente di `openDiagnostics(filter)` per aprire un modulo con un parametro.
- `FaceCrop` usa la miniatura come segnaposto e il protocollo `face://` per il ritaglio.

## Goals / Non-Goals

**Goals:**
- Un unico flusso di revisione: dai gruppi automatici alle persone decise dall'utente, con anti-doppioni, ignora, qualità e navigazione diretta.
- Scala a 100.000 volti senza caricare tutto e senza polling.
- Nessuna perdita delle correzioni e delle identità esistenti.

**Non-Goals:**
- Cambiare i modelli di rilevamento o di embedding.
- Volti nei video.
- Riconoscimento di animali.

## Decisions

### 1. Modello: gruppi, persone, ignorati
Si riusa `person` come "gruppo": un gruppo con `identity_uid` è una **persona**, uno senza è un gruppo **da rivedere**. `person.name` resta per compatibilità ma il nome mostrato è sempre quello dell'identità. Nuova colonna `person.ignored_at INTEGER NULL`. Le query di Libreria (filtro Persone, Inspector, facet, conteggi, FTS) escludono i gruppi ignorati. L'analisi tratta i gruppi ignorati come persone "protette": i volti nuovi simili vi vengono assegnati (come per le persone con nome), così non riemergono.

La regola "solo l'utente crea persone" è già vera per le identità, perché vengono create solo dal "Chi è?" o dalla schermata Identità. Va rimossa ogni creazione implicita di identità. Fa eccezione la migrazione delle persone con nome già esistente (requisito "Migrazione delle persone con nome"), che resta. Il riconoscimento in una libreria nuova continua a collegare automaticamente i volti alle identità esistenti tramite i riferimenti.

### 2. Qualità dei volti
In `face_analysis.rs`, sul ritaglio allineato 112×112 già calcolato:
- **nitidezza**: varianza del laplaciano sul canale di luminanza, normalizzata in 0–1 con una curva tarata (funzione in `pc-ai/src/quality.rs`, condivisa con `similar-photo-stacks`);
- **frontalità**: dai 5 punti di riferimento (occhi, naso, bocca), simmetria orizzontale del naso rispetto agli occhi e rapporto distanza occhi / larghezza del riquadro, in 0–1;
- **dimensione**: lato minore in pixel della sorgente (già calcolata);
- **qualità combinata**: `quality_score = min(1, conf^0.5 · sharp^0.8 · front^0.5 · size_factor)`, dove `size_factor` satura a 1 sopra 96 px.

Si salvano `face.sharpness`, `face.frontality`, `face.size_px` e `face.quality_score`; `quality` (confidenza) resta. Un volto è **scartato** se `conf < face_detection` oppure `sharpness < face_min_sharpness` (nuova soglia, predefinita 0,35) oppure `size_px < face_min_size_px`. Il motivo si deriva dai valori e non si salva. I volti scartati non entrano nel grafo del raggruppamento né nei centroidi, salvo `assigned_by = 'manual'` o recupero esplicito (nuova colonna `face.quality_override INTEGER NOT NULL DEFAULT 0`).

Per i volti già presenti un job di manutenzione `face_quality` ricalcola nitidezza e dimensione dai ritagli: si riusa `face_crop::jpeg` (stesso allineamento ricavato dal riquadro), che decodifica la foto in modo ridotto senza eseguire modelli. La frontalità per i volti vecchi è sconosciuta (`NULL` = neutra, 1).

### 3. Suggerimenti di unione
Nuovo calcolo `merge_suggestions` dopo ogni raggruppamento: kNN (HNSW) sui centroidi dei gruppi non ignorati. Si propone una coppia quando il coseno tra centroidi è ≥ `face_similarity × 0,8` e almeno 3 coppie di volti tra i due gruppi superano `face_similarity`. Le coppie sono salvate in `person_suggestion(a, b, score, created_at)`. Ne sono escluse quelle in `person_not_same(a, b)`, scritta da "Sono persone diverse", che il raggruppamento usa anche come vincolo per non unire i due gruppi (estensione dell'attuale meccanismo `rejected`, a livello di gruppo). Unendo o separando gruppi si aggiornano i riferimenti nelle due tabelle.

### 4. Comandi (paginati)
- `people_sections()`: conteggi di "Da rivedere", "Persone", "Ignorati", volti singoli e scartati.
- `list_groups(section, query, sort, after, limit)`: righe con copertina (miglior `quality_score`), volti, foto, suggerimento di nome (top-1 di `suggest_identities` con punteggio) e suggerimento di unione.
- `group_faces(group | Singles | Discarded(group?), sort, after, limit)`: cursore su `(quality_score DESC, id)`.
- `correct_person` esteso con `MergeMany { from: Vec, to }`, `Move { faces, to }`, `NotSame { a, b }`, `Ignore { group }`, `Restore { group }`, `Recover { faces }`, `SetCover { group, face }`.
- `locate_face(face_id | media_id+person?)` → `{ section, group_id, index }`: la posizione del volto nell'ordinamento predefinito, così la UI apre la pagina giusta.
- Evento `faces://changed` alla fine delle fasi `faces`/`cluster` e dopo ogni correzione, al posto del polling.
- Tutti i comandi sono `async` con `spawn_blocking`, in linea con `large-library-stability`.

### 5. Interfaccia
Riferimento visivo: `mockups/People.html` (vedi `mockups/README.md` su cosa è vincolante).
- **Struttura**: `modules/people/PeopleModule.tsx` con tre colonne:
  - `GroupList`, virtualizzato con sezioni a schede;
  - `FaceGrid`, virtualizzato a pagine, con selezione multipla (clic, Ctrl, Maiusc, rettangolo), barra delle azioni e banner del suggerimento di unione;
  - `PersonCard`: "Chi è?" (riuso di `WhoIs`), copertina, unione, ignora; per le persone, i campi dell'editor delle identità (estratto da `IdentitiesPage.tsx` in `IdentityEditor`), con riferimenti, esporta/importa (`Transfer`).

  Sotto 1.100 px la scheda diventa un pannello a scomparsa.
- **Dialoghi**: unione (con scelta del nome se entrambi ne hanno uno), conferma di "Rimuovi volto" (l'unica azione distruttiva sui dati dei volti), "Sono persone diverse" senza conferma ma annullabile.
- **Navigazione**: `store/layout.ts` aggiunge `peopleTarget` e `openPeople(target)` (sul modello di `openDiagnostics`). `target` è `{ person } | { face } | { media, person? }`. `PeopleModule` risolve con `locate_face`, seleziona la sezione e il gruppo, scorre fino all'indice ed evidenzia il volto (bordo `--pc-warn`), con un avviso "Torna alla foto" che riporta in Libreria. L'Inspector mostra i volti della foto come chip (persona, "volto non riconosciuto", "scartato"), ciascuno con "Apri in Persone". I chip delle persone nei filtri e nella Sequenza usano lo stesso comando.
- **Registry**: la voce `identities` viene rimossa da `registry.ts`. `layout.ts` migra `module: "identities"` a `"people"` con sezione "Persone" (versione dello store persistito incrementata).
- **Tastiera**: come da mockup (↑/↓, Spazio, N, M, X, U, I, Ctrl+Z), gestita a livello di modulo e disattivata quando il focus è in un campo di testo.
- **Stati**: caricamento, vuoto ed errore per ogni colonna. Usa `QueryView`/`BusyButton` se `loading-feedback` è unito, altrimenti segnaposto locali da sostituire poi.

### 6. Impostazioni
"Nitidezza minima dei volti" (`ai.thresholds.face_min_sharpness`) nella sezione Soglie AI, con anteprima del numero di volti scartati e rivalutazione senza rianalisi, tramite il job `retune` esteso con `faces_quality: true`.

## Risks / Trade-offs

- [Taratura della nitidezza: la varianza del laplaciano dipende da illuminazione e risoluzione del ritaglio] → normalizzazione sul ritaglio allineato a dimensione fissa, curva tarata su `D:\Foto` (in sola lettura, cartella dati separata) e soglia regolabile dall'utente; i volti scartati restano recuperabili.
- [Ignorare un gruppo che in realtà contiene più persone] → il gruppo ignorato resta ispezionabile e ripristinabile; "Sposta in…" funziona anche dalla sezione Ignorati.
- [Migrazione della UI: le identità perdono la loro schermata dedicata] → tutte le funzioni dell'editor attuale (alias, gruppo, soglia, riferimenti, unione, esporta/importa) restano nella scheda della persona; i test esistenti di `IdentitiesPage.test.tsx` vengono portati sul nuovo modulo.
- [Conflitti]
  - con `large-library-stability` (paginazione di `person_faces`, eventi): questo change riscrive quelle parti, e chi arriva per secondo si allinea;
  - con `similar-photo-stacks`: `quality.rs` e lo stesso requisito sulle soglie in `ai-classification`;
  - con `catalog-exclusions`: numero di migrazione;
  - con `loading-feedback`: componenti di stato.
- [Suggerimenti di unione rumorosi con molti gruppi piccoli] → soglia minima di 3 volti per gruppo e limite di 1 suggerimento per gruppo nell'elenco, ordinato per punteggio.

## Migration Plan

Migrazione di schema automatica (con backup):
- nuove colonne in `face` e `person`;
- nuove tabelle `person_not_same` e `person_suggestion`;
- indici per la paginazione: `face(person_id, quality_score DESC, id)` e `face(quality_score)`.

Dopo la migrazione parte una volta, in background, il job `face_quality` (annullabile e riprendibile). Fino al suo completamento i volti esistenti sono trattati come "qualità sconosciuta = non scartati", così il raggruppamento non cambia. Le persone con nome già collegate a identità diventano "Persone"; quelle con nome e senza identità sono già state migrate dalla v5. I gruppi senza nome diventano "Da rivedere". Rollback: una versione precedente non apre il catalogo migrato (serve il backup pre-migrazione); `app.db` non cambia schema.

## Open Questions

- Curva di normalizzazione della nitidezza e soglia predefinita (0,35): da tarare sui volti di `D:\Foto` senza cambiare l'approccio.
