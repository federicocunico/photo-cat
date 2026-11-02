# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Mai toccare i file sul disco né rimuovere foto dalla libreria**: escludere e reinserire agiscono solo sulle tabelle dei cataloghi. Ogni test deve verificarlo.
> - **Migrazione v6**: prima di iniziare controllare in `crates/pc-store/src/migrate.rs` su `main` quale sia l'ultima versione. Se nel frattempo un'altra change (es. `date-filter-hierarchy`) ha introdotto la v6, usare la versione successiva. Se la conflittualità non è risolvibile con una semplice rinumerazione, fermarsi e chiedere.
> - **Compatibilità**: il nome `remove_catalog` delle azioni bulk deve restare accettato (alias), per non rompere l'`undo_log` esistente.
> - **Toast**: se nel frattempo `loading-feedback` ha creato un componente di notifiche, riusarlo invece di crearne un altro.

## 1. Dati e query

- [x] 1.1 Migrazione con la tabella `catalog_exclusion` e il suo indice (design §1). Verifica: test di migrazione da un catalogo v5 con cataloghi e righe di `catalog_item` che restano invariate, e `cargo test -p pc-store migrate` passa.
- [x] 1.2 Funzioni di store `exclude(catalog, ids)` e `restore(catalog, ids)` con il comportamento diverso per manuali e smart, e `add_items` che cancella l'esclusione sui manuali e rifiuta gli smart. Verifica: test in `crates/pc-store/tests/` per gli scenari "Esclusione da un catalogo manuale", "Riaggiunta di una foto esclusa" e "Esclusione da un catalogo smart"; nessuna riga di `media_item` cancellata.
- [x] 1.3 Esclusioni nel contenuto smart, `LibraryFilter.excluded_from` con `matching_only`, `SortKey::ExcludedAt`, e appartenenza a catalogo nelle regole che rispetta le esclusioni (design §2). Verifica: test per "Esclusione che sopravvive al cambio dei dati", "Appartenenza in una regola smart" e per la vista escluse con e senza `matching_only`.
- [x] 1.4 `list_catalogs` con `items` ed `excluded`. Verifica: test per lo scenario "Conteggi" su un manuale e su uno smart.

## 2. Azioni bulk e undo

- [x] 2.1 `BulkAction::ExcludeFromCatalog` / `RestoreToCatalog` con alias `remove_catalog`, `UndoAction::Excluded` / `Restored` che ricreano esattamente le righe dei manuali, e azioni non più distruttive (design §3); rigenerare `bindings.ts`. Verifica: test in `pc-jobs` e `pc-store` per "Esclusione in bulk da un catalogo smart" con annullamento, per il ripristino esatto di un manuale dopo Annulla, e per la deserializzazione di un vecchio `remove_catalog`.

## 3. Interfaccia

- [x] 3.1 Intestazione del catalogo in Libreria con conteggio, "Escluse (N)", vista escluse con banner, "Torna al catalogo" e interruttore per gli smart (design §4); chiavi i18n it/en. Verifica: test con `ipcMock` in `LibraryPage.test.tsx` per "Revisione e reinserimento".
- [x] 3.2 "Escludi dal catalogo" in BulkBar, `GridContextMenu` e Inspector; "Reinserisci nel catalogo" nella vista escluse; tolta la conferma bloccante. Verifica: test in `BulkBar.test.tsx` e `Inspector.test.tsx`.
- [x] 3.3 Tasto Canc in `Grid.tsx` (solo dentro un catalogo, non nella vista escluse) e avviso non modale con "Annulla" (`role="status"`). Verifica: test per "Esclusione rapida con annullamento" e "Canc fuori da un catalogo".

## 4. Verifica d'insieme

- [x] 4.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde.
- [ ] 4.2 Prova sull'app reale con una copia della libreria (mai sulla cartella `D:\Foto` in scrittura): escludere da un manuale e da uno smart, annullare, rivedere le escluse, reinserire, rianalizzare e controllare che le esclusioni smart reggano. Verifica: tutti i passi riusciti, file su disco intatti, nessun errore nuovo in Diagnostica.
- [x] 4.3 Aggiornare il `README.md` (esclusioni dai cataloghi), allineare le delta e verificare che `openspec validate catalog-exclusions --strict` passi.

## Workflow follow-up

- Coordinare il numero di migrazione con le altre change in corso.
- Archiviare il change dopo la revisione.
