# Proposal

## Why

Dentro un catalogo l'utente vuole togliere le foto che non gli interessano, senza toccarle sul disco. Vuole anche poter rivedere quelle tolte e rimetterle. Oggi:

- dai **cataloghi manuali** si può già rimuovere una selezione (azione bulk "Rimuovi dal catalogo", annullabile solo come ultima operazione). Una volta rimossa, però, la foto sparisce senza traccia: non c'è un elenco delle foto tolte da cui reinserirle;
- dai **cataloghi smart** non si può togliere nulla. Il contenuto viene dalle regole, e l'azione bulk scrive righe in `catalog_item` che la query del catalogo smart non legge, quindi non ha alcun effetto visibile.

## What Changes

- **Escludi dal catalogo**: su una o più foto di un catalogo (manuale o smart), "Escludi dal catalogo" le toglie dal catalogo. Si fa dalla barra bulk, dal menu contestuale della griglia, dall'Inspector e con il tasto Canc quando si sta guardando un catalogo. Non c'è una conferma bloccante: compare un avviso con "Annulla". I file sul disco e la libreria non vengono mai toccati.
- **Foto escluse**: ogni catalogo ha una vista "Escluse (N)", raggiungibile dall'intestazione del catalogo, che mostra le foto escluse con data di esclusione. Da lì si possono **reinserire** singolarmente o in blocco.
- Nei **cataloghi smart** un'esclusione vale anche se la foto continua a soddisfare le regole. Se la foto smette di soddisfarle, resta comunque nella lista delle escluse, così l'esclusione rimane valida se torna a soddisfarle. La vista "Escluse" mostra solo quelle che oggi soddisferebbero le regole, con un interruttore per vederle tutte.
- Nei **cataloghi manuali** "Rimuovi dal catalogo" diventa "Escludi dal catalogo" con lo stesso effetto visibile più la tracciabilità. Aggiungere di nuovo una foto esclusa la reinserisce e cancella l'esclusione.
- Il conteggio di un catalogo nella barra laterale non include le escluse; l'intestazione mostra anche quante sono le escluse.
- Le regole smart che fanno riferimento all'appartenenza a un altro catalogo tengono conto delle esclusioni di quel catalogo.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `catalog-library`: "Cataloghi manuali" (rimozione tracciata e reversibile), "Cataloghi smart" (esclusioni); nuovo requisito "Foto escluse da un catalogo".
- `bulk-operations`: "Azioni bulk disponibili" (escludere da un catalogo e reinserire al posto di "rimuovere da cataloghi manuali").

## Impact

- **Codice Rust**: `pc-store` (nuova tabella `catalog_exclusion` con migrazione di schema v6, `query.rs` per escludere le escluse dai cataloghi e mostrarle nella vista dedicata, `rules.rs` per l'appartenenza a catalogo, `bulk.rs` con nuove azioni e undo), `pc-jobs/src/bulk.rs` (`ExcludeFromCatalog`, `RestoreToCatalog` al posto di `RemoveCatalog`), `app/src-tauri/src/catalogs.rs` (conteggi delle escluse).
- **UI**: intestazione del catalogo in Libreria (conteggio escluse, vista "Escluse"), `BulkBar.tsx`, `GridContextMenu`, `Inspector.tsx`, tasto Canc in `Grid.tsx`, avviso con "Annulla", i18n it/en.
- **Dati**: migrazione v6; le righe esistenti di `catalog_item` restano invariate.
- **Coordinamento**: altre change in corso possono introdurre a loro volta una migrazione v6 (vedi note in `tasks.md`); chi arriva per secondo rinumera.
