# Design

## Context

- Schema (`crates/pc-store/src/schema_v1.sql:139-151`): `catalog(id, name, kind 'manual'|'smart', rules JSON, created_at)` e `catalog_item(catalog_id, media_id, PK)` con `ON DELETE CASCADE` su entrambi.
- Contenuto (`crates/pc-store/src/query.rs:260-275`): manuale = `m.id IN (SELECT media_id FROM catalog_item WHERE catalog_id = ?)`; smart = SQL compilato dalle regole (`rules::compile`), che ignora `catalog_item`. Le regole possono riferirsi all'appartenenza ad altri cataloghi (`rules.rs:238`).
- Rimozione oggi: `BulkOp::RemoveCatalog` (`crates/pc-jobs/src/bulk.rs:30,86`) → `bulk::remove_from_catalog` (`pc-store/src/bulk.rs:641`) → `DELETE FROM catalog_item`, con undo `UndoAction::CatalogRemoved` (`bulk.rs:937`). In UI è l'azione `remove_catalog` della BulkBar con conferma (`BulkBar.tsx:76,85`). Né `add_items` né `remove_items` controllano il tipo del catalogo: sui cataloghi smart l'azione non ha effetto visibile.
- Lo schema è alla v5 (`migrate.rs`, `MIGRATIONS` con un file `schema_vN.sql` per versione).

## Goals / Non-Goals

**Goals:**
- Esclusioni tracciate e reversibili per entrambi i tipi di catalogo, con una vista delle escluse.
- Esclusione veloce (tasto Canc, avviso con Annulla) senza dialoghi bloccanti, perché l'operazione non distrugge nulla.

**Non-Goals:**
- Inclusioni manuali nei cataloghi smart (foto aggiunte a mano oltre le regole).
- Cancellare file dal disco o rimuoverli dalla libreria: non cambia nulla rispetto a oggi.
- Esclusioni globali dalla libreria (es. "nascondi ovunque").

## Decisions

### 1. Tabella `catalog_exclusion` (migrazione v6)
```sql
CREATE TABLE catalog_exclusion (
    catalog_id  INTEGER NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    media_id    INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    excluded_at INTEGER NOT NULL,
    PRIMARY KEY (catalog_id, media_id)
);
CREATE INDEX catalog_exclusion_media ON catalog_exclusion(media_id);
```
Per i cataloghi manuali escludere = `DELETE FROM catalog_item` + `INSERT OR REPLACE INTO catalog_exclusion`, nella stessa transazione. Reinserire = l'inverso. Così il contenuto del manuale resta "le righe di `catalog_item`" e la query non cambia. Per gli smart escludere = solo `INSERT` in `catalog_exclusion`.
*Alternativa scartata*: una colonna `excluded` in `catalog_item` — non serve agli smart, che non usano `catalog_item`, e obbligherebbe a modificare tutte le query dei manuali.

### 2. Query
- Contenuto smart: `(<sql delle regole>) AND m.id NOT IN (SELECT media_id FROM catalog_exclusion WHERE catalog_id = ?)`.
- Vista "Escluse": nuovo campo `LibraryFilter.excluded_from: Option<i64>`, alternativo a `catalog_id`. Restituisce `m.id IN (SELECT media_id FROM catalog_exclusion WHERE catalog_id = ?)`. Per gli smart, con `matching_only = true` (predefinito), aggiunge `AND (<sql delle regole>)`. Usando il filtro della libreria, la vista eredita griglia, ordinamento, facet e selezione senza componenti nuovi. L'ordinamento predefinito della vista è per `excluded_at` decrescente (nuova `SortKey::ExcludedAt`, valida solo con `excluded_from`).
- Appartenenza nelle regole (`rules.rs:238`): la condizione "Catalogo = X" usa la stessa funzione che produce il contenuto di X, quindi le esclusioni valgono automaticamente. Va aggiunto un test, perché oggi la funzione legge `catalog_item` direttamente per i manuali e non conosce le esclusioni degli smart.
- Conteggi: `list_catalogs` restituisce `items` (senza escluse) e `excluded`. Per gli smart, `excluded` conta le escluse che oggi soddisfano le regole.

### 3. Azioni e undo
- `BulkAction::ExcludeFromCatalog { catalog }` e `BulkAction::RestoreToCatalog { catalog }` sostituiscono `RemoveCatalog`. Il nome serializzato `remove_catalog` resta accettato come alias di `ExcludeFromCatalog`, così un `undo_log` o un job in coda non si rompe.
- `AddCatalog` su un manuale cancella anche l'eventuale esclusione. Su uno smart viene rifiutato con `invalid_input` ("i cataloghi smart si modificano con le regole o le esclusioni"); oggi è silenziosamente inefficace.
- Nuove `UndoAction::Excluded { catalog_id, ids, manual_rows }` e `UndoAction::Restored { … }`. Per i manuali conservano quali righe di `catalog_item` sono state tolte, così l'annullamento le ricrea esattamente.
- Le azioni non sono più `destructive()`: nessuna conferma, ma esito e "Annulla" come le altre azioni sui metadati.

### 4. UI
- **Intestazione del catalogo** in Libreria (quando `filter.catalogId` è impostato): nome, conteggio, "Escluse (N)" come pulsante che passa alla vista escluse. Nella vista escluse c'è un banner "Stai vedendo le foto escluse da «X»", con "Torna al catalogo", e per gli smart l'interruttore "Mostra anche quelle che non soddisfano più le regole".
- **Azioni**: "Escludi dal catalogo" nella BulkBar, nel `GridContextMenu` e nell'Inspector (sezione cataloghi, accanto a ciascun catalogo della foto); "Reinserisci nel catalogo" nella vista escluse, che diventa l'azione principale della BulkBar lì.
- **Tasto Canc**: in `Grid.tsx`, solo con `catalogId` impostato e non nella vista escluse, esegue l'esclusione della selezione. Fuori da un catalogo non fa nulla. Nella vista escluse, Canc non fa nulla e Invio reinserisce.
- **Avviso con Annulla**: un toast non modale per 8 secondi, "6 foto escluse da «Viaggio Toscana» · Annulla", che chiama `undo_bulk`. Se nell'app non c'è ancora un sistema di toast, va creato in `components/` (piccolo, accessibile, `role="status"`); può servire anche a `loading-feedback`.

## Risks / Trade-offs

- [Conflitto di numero di migrazione con altre change in corso (es. `date-filter-hierarchy`)] → il file è `schema_v6.sql` solo se nessun'altra v6 è già in `main` al momento del merge; altrimenti si rinumera. Test di migrazione da v5.
- [Escluse orfane negli smart: una foto esclusa che non soddisfa più le regole resta in tabella per sempre] → è voluto, perché l'esclusione resti valida se la foto torna a soddisfarle. La vista con "mostra tutte" permette di ripulirle reinserendole; costo trascurabile.
- [Il tasto Canc viene associato per abitudine a "elimina file"] → testo dell'avviso esplicito ("escluse dal catalogo, i file non sono stati toccati") e Annulla immediato; mai nessuna azione su disco.

## Migration Plan

Migrazione v6 automatica con il backup del catalogo già previsto prima delle migrazioni. Nessun dato esistente cambia: le rimozioni fatte prima di questa versione non compaiono tra le escluse, perché non erano tracciate. Rollback: una versione precedente rifiuta un catalogo v6 (comportamento esistente); serve il backup automatico pre-migrazione.
