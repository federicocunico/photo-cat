# Design

## Context

- **Navigazione**: `app/ui/src/modules/registry.ts` (`NAV_GROUPS`, `MODULES` con Ctrl+N assegnati in ordine), `store/layout.ts` (`ModuleId`, elenco `known`, `Destination`), `ReadyApp.tsx` (switch dei moduli), `commands/catalog.ts` (testo "Ctrl+1…7"), `components/shell/shell.test.tsx` (verifica dei gruppi).
- **Dati live**: `store/live.ts` riceve per eventi push `JobSnapshot`, risorse, errori. Non fa polling.
- **Rianalisi esistente**:
  - `models.rs::reanalyze(Target)` risolve gli id e accoda `analysis` con `{"ids"}`;
  - il job `analysis` accetta `ids | sourceId | niente` e salta le foto già alla versione corrente (`analysis::pending`);
  - `retune.rs::prepare_reanalysis` è il precedente per azzerare i marcatori.
- **Selezioni**: `Target::{Ids, Filter{filter,max_id,except}}` con `resolve` a blocchi. La UI non riceve mai gli elenchi completi di id.
- **Facet**: `query::facets` e `facet_values` danno già i conteggi per voce su un `LibraryFilter`. Non esistono istogrammi delle confidenze né aggregati di copertura.
- **Etichette**: `analysis::classification` e `relabel` riscrivono le etichette `auto` per gruppo, saltando i gruppi con etichette manuali. Non esiste un rifiuto per singola voce.
- **Pulizia annullabile**: le azioni bulk (`pc-jobs/src/bulk.rs`, `undo_bulk`) sono il precedente.
- **Badge della griglia**: `query.rs` calcola già `AnalysisState` (non analizzata / in coda / analizzata) per la classificazione. La Panoramica deve usare la stessa definizione.
- **Dipendenze UI**: nessuna libreria di grafici. Ci sono React 18, Radix, TanStack Query e zustand.

## Goals / Non-Goals

**Goals:**
- Un'unica definizione di "stato di una fase", usata da Panoramica, filtro della Libreria, anteprima della Rianalisi e badge della griglia.
- Query aggregate sotto i 300 ms su 100.000 foto, per stare nel requisito di 1 s con il rendering.
- Nessuna nuova dipendenza npm o Rust.

**Non-Goals:**
- Modificare soglie o tassonomia dal modulo AI (restano in Impostazioni; il modulo collega la sezione giusta).
- Pulizia di volti e persone: c'è già il modulo Persone. Qui si puliscono solo le etichette.
- Mappa (change `photo-map`).

## Decisions

### D1. Stato per fase in `pc-store::coverage`
Un enum `Stage { Classification, Persons, Faces, Similar, Phash }`. Per ogni fase:

| Fase | "Analizzata" | "Corrente" |
|---|---|---|
| Classificazione | riga `analysis_version` `clip` | stessi `model_ver` e `taxonomy_ver` dei parametri |
| Persone | `media_people` | `model_ver` = chiave del rilevatore |
| Volti | marcatore della fase volti in `analysis_version` (da confermare nel codice di `save_faces_tuned`; se manca, la migrazione v12 lo aggiunge) | versione dell'analisi dei volti |
| Simili | `similar_done` | `params` = `params_key` |
| Impronte | `phash IS NOT NULL` | sempre corrente |

Le chiavi correnti le calcola lo strato app (che conosce i modelli installati) e le passa alle query in un `StageKeys`. Senza modello installato una fase ha chiave `None`: le foto sono "mai analizzate" o "da aggiornare" ma non "correnti", e la Rianalisi la mostra come non disponibile.

"In coda" = non corrente **e** coperta da un job attivo della fase. Job: `analysis` (target `ids`/`sourceId`/libreria/`scope`), `similar`, `phash_backfill`, `import` (solo per le sue foto). La copertura si calcola caricando gli id target dei job attivi in una tabella temporanea. Il badge della griglia viene riscritto su questa funzione per la classificazione.

- *Alternativa*: contatori incrementali mantenuti dai job. Scartata: si disallineano a ogni correzione o migrazione. I COUNT sugli indici esistenti (`analysis_version` PK `(media_id, stage)`, indice `analysis_stage_version`) bastano.

### D2. `LibraryFilter.analysis` e `LibraryFilter.label_score`
- `analysis: Option<{ stage, state: current|stale|never }>`, compilato in `compile_filter` con gli stessi predicati di D1. Le chiavi correnti arrivano dal comando Tauri, che le aggiunge prima di eseguire la query.
- `label_score: Option<{ grp, value, min, max }>` per "Mostra in Libreria" da un intervallo dell'istogramma (solo etichette `auto`).

Entrambi appaiono come chip nei filtri attivi.

### D3. Rianalisi: parametri `scope`, `stages`, `force` del job `analysis`
- I parametri del job diventano `{ scope, stages?, force? }`, con `scope` = `library | {catalogIds:[..]} | {ids:[..]}`. Gli id di un catalogo vengono risolti all'inizio del job, così i parametri restano piccoli anche per cataloghi da 50.000 foto. Selezione corrente e risultati filtrati vengono risolti in id dal comando, come fa oggi `reanalyze`.
- Restano compatibili i vecchi `{ids}` e `{sourceId}`.
- `stages` limita le passate (`clip`, `persons`, `faces`, `similar`; `phash` → accoda `phash_backfill` con gli id).
- `force: true` azzera prima i marcatori di versione delle fasi scelte per quegli id (`analysis_version`, `media_people.model_ver`, `similar_done`, `phash`), sul modello di `prepare_reanalysis`. Le scritture esistenti rispettano già le correzioni manuali; il rifiuto (D5) completa la garanzia.
- L'anteprima `ai_reprocess_preview(scope, stages, force)` restituisce i conteggi per fase con le query di D1, senza creare il job.

### D4. Aggregati per i grafici in `pc-store::insights`
- **Distribuzione**: riuso di `query::facets`/`facet_values` sul filtro di ambito (nessuna query nuova).
- **Istogramma**: `SELECT min(CAST(score*20 AS INT),19), count(*) FROM label JOIN <filtro> WHERE grp=? AND value=? AND origin='auto' GROUP BY 1`. Venti classi da 0,05; la soglia corrente viene dalla tassonomia.
- **Copertura per mese**: `GROUP BY substr(captured_at,1,7)` con conteggio delle foto totali e analizzate (classificazione corrente). Le foto senza data stanno in una barra "senza data".
- **Simili**: numero di gruppi, foto raggruppate, `similar::pending`, classi di dimensione (2, 3–5, 6–10, >10). Ogni classe corrisponde a un filtro `similar_size`.

### D5. Rifiuto delle etichette: tabella `label_rejection` (schema v12)
`label_rejection(media_id, grp, value, created_at, PRIMARY KEY(media_id, grp, value))`.
- `classification` e `relabel` scartano i valori rifiutati prima di inserire.
- L'assegnazione manuale della stessa voce cancella il rifiuto.
- L'azione bulk `reject_auto_labels { grp, value, max_score?, target }` cancella le etichette `auto` e inserisce i rifiuti. Lo stato per l'annullamento (le righe `label` cancellate) va nel registro bulk esistente. `undo_bulk` le ripristina e toglie i rifiuti.

Se la change `perceptual-hash-similarity` non è stata applicata, questa migrazione diventa la v11: il numero si decide all'applicazione.

### D6. Grafici SVG fatti a mano in `app/ui/src/components/charts/`
`BarChart` (orizzontale, ordinato, clic su barra), `Histogram` (selezione di un intervallo trascinando, linea di soglia), `StackedBar` (copertura della Panoramica, segmenti cliccabili), `ColumnSeries` (copertura mensile). Colori dai token del tema, tooltip con Radix, accessibilità da tastiera sulle barre. L'implementazione segue la skill `dataviz`.
- *Alternative*: recharts o visx (≥ 100 KB, stile da riallineare); uPlot (ottimo per serie temporali, meno per barre interattive). Con quattro forme semplici il codice proprio è più piccolo e coerente con il resto dell'interfaccia.

### D7. Aggiornamento live
`useAiOverview(scope)` (TanStack Query) invalida la query quando arriva un `JobSnapshot` di un tipo AI (`analysis`, `similar`, `phash_backfill`, `retune`, `import`), con throttling a 2 s e un refresh finale al completamento. Job attivi, modelli e memoria arrivano già dallo store `live`.

### D8. Modulo e navigazione
- Nuovo gruppo `ai` in `NAV_GROUPS` tra `organize` e `transfer`, con il modulo `ai` (icona lucide `Sparkles`), così Ctrl+6 = AI, Ctrl+7 = Importa, Ctrl+8 = Esporta.
- `ModuleId` e `known` aggiornati; `Destination { module: "ai", section }`.
- Sezioni in `app/ui/src/modules/ai/` (`OverviewSection`, `ReprocessSection`, `ResultsSection`, `CleanupSection`, `SimilarSection`), con sotto-navigazione come in Impostazioni e sezione ricordata in `layout`.
- Comandi della palette: "AI: Panoramica", "AI: Rianalisi…", "AI: Pulisci etichette…".
- L'azione "Rianalizza" della barra di selezione apre Rianalisi con ambito "selezione corrente".

## Risks / Trade-offs

- **[Rischio]** I conteggi "in coda" dipendono dai target dei job attivi: un `analysis` sull'intera libreria rende "in coda" tutte le foto non correnti, comprese quelle che il job salterà per errori.
  → Accettato: le foto con errori tornano "mai analizzate" a fine job.
- **[Rischio]** Le query di copertura con molti job attivi o cataloghi grandi superano il budget.
  → Test di prestazioni su 100.000 foto sintetiche; tabella temporanea indicizzata degli id target.
- **[Rischio]** La rinumerazione delle scorciatoie (Esporta passa da Ctrl+7 a Ctrl+8) spiazza chi è abituato.
  → Accettato e documentato nel foglio delle scorciatoie. La sequenza resta l'ordine visivo.
- **[Trade-off]** "Forza" su volti e persone può cambiare i gruppi di persone automatici. Le assegnazioni manuali restano. L'anteprima lo segnala con una nota.

## Migration Plan

Schema v12 (`label_rejection`; eventuale marcatore della fase volti). Nessun dato da convertire. I job `analysis` in coda con i vecchi parametri restano validi.

## Open Questions

- Nome definitivo e icona del gruppo di navigazione ("AI" oppure "Intelligenza"): deciso in fase di UI, la spec usa "AI".
