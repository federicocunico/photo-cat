# Design

## Context

- **Duplicati esatti**: `media_item.content_hash` (`schema_v1.sql:24`, indice `:32`), filtro `LibraryFilter.duplicates_only` (`query.rs`), `repo::duplicates_of` per l'Inspector. Nessun hash percettivo, nessun concetto di gruppo o raffica (`media_dates.group_id` della v5 è il gruppo di sorgente per gli orari, non c'entra).
- **Dati disponibili per foto**:
  - embedding CLIP 512-d nella tabella `embedding(media_id, model_ver, vector)`;
  - miniature JPEG a 512 px di lato lungo in `<dati>/thumbs` (`pc-ingest/src/thumb.rs:18`);
  - istante di scatto `media_dates.instant_utc`, `device` (v5, indicizzato).
- **Infrastruttura**:
  - fasi dell'analisi in `pc-ai/src/analyze.rs` (`clip`, `persons`, `faces`, `cluster`), con il cluster come fase finale a livello di libreria;
  - `hnsw_rs` già usato da `cluster.rs` per i volti;
  - job `retune` per riapplicare soglie senza rieseguire i modelli;
  - governor della memoria in `pc-governor`.

## Goals / Non-Goals

**Goals:**
- Gruppi di foto quasi identiche con costo quasi lineare nel numero di foto, incrementale, entro il budget di memoria.
- Correzioni manuali stabili, come per i volti.
- Visualizzazione raggruppata che non cambia la semantica di conteggi e filtri.

**Non-Goals:**
- Scelta automatica e cancellazione delle foto "peggiori": il sistema propone una principale, l'utente decide.
- Somiglianza "tematica" (stesso luogo, stesso evento): servono già i cataloghi e i filtri esistenti.
- Video.

## Decisions

### 1. Candidati, poi conferma con SSIM
Confrontare con SSIM ogni coppia della libreria è impossibile (5·10⁹ coppie a 100k foto). La pipeline per ogni foto da elaborare è:
1. **Candidati semantici**: i k=12 vicini nell'indice HNSW degli embedding CLIP, con coseno ≥ 0,92 (fisso, interno: serve solo a filtrare).
2. **Candidati temporali**: le foto dello stesso `device` entro ±10 s di `instant_utc` (raffiche). Si aggiungono anche se il coseno è più basso, purché ≥ 0,80.
3. **Duplicati esatti**: stesso `content_hash`, sempre nello stesso gruppo senza ulteriori verifiche.
4. **Conferma**: SSIM su miniature ridotte a 128 px di lato lungo, in scala di grigi, allineate per proporzioni (le coppie con proporzioni diverse oltre il 5% sono scartate; la rotazione EXIF è già applicata alle miniature). La coppia è un arco se `ssim ≥ soglia` (predefinita 0,80).
5. **Gruppi**: componenti connesse degli archi (union-find), con i vincoli manuali (§3). Per evitare catene (A≈B≈C con A e C diversi), un componente sopra 50 foto viene diviso con un clustering per collegamento completo sulla matrice SSIM interna.

*Alternativa scartata*: hash percettivo (dHash/pHash) come unico criterio — veloce, ma fragile su piccoli spostamenti di inquadratura e cambi d'esposizione delle raffiche. Resta un'ottimizzazione possibile per scartare in anticipo candidati evidentemente diversi.

### 2. Archiviazione (migrazione di schema)
```sql
CREATE TABLE similar_group (id INTEGER PRIMARY KEY, cover_media_id INTEGER, cover_manual INTEGER NOT NULL DEFAULT 0, params TEXT NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE similar_member (media_id INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE, group_id INTEGER NOT NULL REFERENCES similar_group(id) ON DELETE CASCADE, score REAL NOT NULL);
CREATE INDEX similar_member_group ON similar_member(group_id);
CREATE TABLE similar_link (a INTEGER NOT NULL, b INTEGER NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('must','cannot')), PRIMARY KEY (a, b)); -- a < b
CREATE TABLE similar_edge (a INTEGER NOT NULL, b INTEGER NOT NULL, ssim REAL NOT NULL, PRIMARY KEY (a, b)); -- archi confermati, per il retune
CREATE TABLE similar_done (media_id INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE, params TEXT NOT NULL);
```
`similar_edge` conserva le coppie candidate con il loro SSIM, inclusi i valori sotto soglia fino a un pavimento di 0,6. Così cambiare la soglia ricalcola i gruppi con union-find in memoria, senza riaprire le miniature. `similar_done` marca le foto elaborate con i parametri del calcolo, per renderlo incrementale e riprendibile.

### 3. Correzioni manuali
- "Togli dal gruppo" → `cannot` tra la foto e ogni altro membro.
- "Raggruppa come simili" → `must` tra le foto selezionate (catena).
- "Sciogli gruppo" → `cannot` tra tutte le coppie del gruppo. Per gruppi grandi si salva un vincolo per gruppo: tabella `similar_dissolved(media_id, token)`, dove le foto con lo stesso token non si uniscono.
- Le `must` si applicano prima delle componenti connesse, e le `cannot` tolgono gli archi relativi. Un conflitto (A must B, B must C, A cannot C) dà precedenza alla correzione più recente.
- `cover_manual` protegge la principale scelta dall'utente finché la foto resta nel gruppo.

### 4. Principale proposta e nitidezza
Punteggio = nitidezza (varianza del laplaciano sulla miniatura a 512 px in scala di grigi, normalizzata nel gruppo) × 0,7 + esposizione (penalità per percentuale di pixel saturati o chiusi) × 0,3. La funzione di nitidezza va in `pc-ai/src/quality.rs` ed è condivisa con `people-faces-workspace`, che la usa sui ritagli dei volti. Chi la implementa per primo la crea, l'altro la riusa.

### 5. Job e integrazione con l'analisi
- Nuova fase `similar` dopo `clip` nel job di analisi, a livello di libreria (come `cluster`). Elabora le foto senza riga in `similar_done` con i parametri correnti; per ciascuna calcola candidati e SSIM, aggiorna `similar_edge`, poi ricalcola i gruppi solo dei componenti toccati.
- L'indice HNSW è costruito in memoria all'inizio della fase (512-d × 100k ≈ 200 MB come f32; si riduce a f16 o si costruisce per blocchi di `model_ver` se il governor non concede il budget). In alternativa, per import piccoli (< 2.000 foto nuove), si confrontano le nuove solo con le foto nella finestra temporale più i vicini calcolati con scansione esatta per blocchi. Scelta guidata dal budget del governor.
- Cambio di soglia: il job `retune` riceve `similar: true` e ricalcola i gruppi da `similar_edge`; l'anteprima conta i gruppi con la nuova soglia allo stesso modo.
- Configurazione: `ai.thresholds.similar_photos` (predefinito 0,80).

### 6. Query e UI
- `LibraryFilter.similar_only: bool`. La griglia chiede le pagine con `group_similar: bool`: il backend restituisce un elemento per gruppo (la principale, o la migliore tra i membri presenti nei risultati) con `group_id` e `group_count` calcolati sui risultati filtrati, più gli elementi singoli. Il conteggio dei risultati resta sulle singole foto (`library_count` invariato).
- Espansione: comando `similar_group_members(group_id, filter)`, con le foto del gruppo inserite nella griglia subito dopo la miniatura del gruppo.
- Selezione di una miniatura chiusa = tutti i membri presenti nei risultati.
- "Seleziona tutte tranne la principale": comando `similar_non_covers(filter | ids)` che restituisce gli id (con i limiti di `large-library-stability` sulla selezione).
- Inspector: sezione "Foto simili" con le miniature del gruppo, "Imposta come principale", "Togli dal gruppo".
- Opzione nella barra della griglia, persistita come le altre preferenze di layout.

## Risks / Trade-offs

- [Memoria dell'indice HNSW a 100k foto] → budget richiesto al governor, con ripiego a blocchi (§5); test con 100k embedding sintetici.
- [Costo SSIM: circa 12–20 confronti per foto, ciascuno su 128 px] → circa 1–2 ms a confronto, quindi qualche minuto per 100k foto su CPU. Fase annullabile e riprendibile; le miniature in scala di grigi a 128 px si tengono in una cache LRU limitata.
- [Raffiche con soggetto in movimento (sport) sotto soglia SSIM] → la finestra temporale abbassa il requisito di coseno ma non quello SSIM. L'utente può unire a mano, oppure abbassare la soglia.
- [Conflitto con `catalog-exclusions` sul numero di migrazione e con `people-faces-workspace` su `quality.rs` e sul requisito "Soglie dell'automatismo configurabili"] → vedi note in `tasks.md`.
- [Griglia raggruppata e paginazione per cursore] → il raggruppamento si fa in SQL con `ROW_NUMBER() OVER (PARTITION BY group_id …)` sul risultato filtrato, mantenendo la paginazione per cursore sulla chiave d'ordinamento della foto rappresentante. Test di prestazioni con 100k foto.

## Migration Plan

Migrazione di schema automatica con backup. Le librerie esistenti non hanno gruppi finché la fase `similar` non gira: al primo avvio viene proposta l'analisi dei gruppi come job (non automatica, per non partire a sorpresa su librerie enormi); le analisi successive la includono. Rollback: revert; con una versione precedente il catalogo migrato non si apre (serve il backup pre-migrazione).

## Open Questions

- Valori predefiniti di soglia SSIM, coseno minimo e finestra temporale: da tarare su `D:\Foto` (in sola lettura, con una cartella dati separata) senza cambiare l'approccio.
