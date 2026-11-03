# Design

## Context

Richieste degli utenti #20, #24, #29. La libreria ha già cataloghi manuali (`catalog`, `catalog_item`), il rilevamento dei duplicati per impronta del contenuto (`media_item.content_hash`, filtro `duplicatesOnly`) e una misura di nitidezza per i volti (`pc-ai::quality`, varianza del laplaciano). Il filtro comune `query::compile_filter` è usato da griglia, conteggi, facet, selezione per filtro, azioni bulk e cataloghi smart.

## Decisions

### D1. Un ruolo di sistema sui cataloghi manuali
Schema v10: `catalog.system TEXT NULL` (`trash` | `duplicates` | `low_quality`), indice unico parziale, e i tre cataloghi creati dalla migrazione. Restano cataloghi manuali: aggiunta, undo e conteggi riusano tutto il codice esistente. `Catalog` espone `system` nei binding; la UI mostra il nome tradotto in base al ruolo.

**Perché non un flag su `media_item`**: tre stati diversi con lo stesso comportamento; il catalogo dà vista, conteggio e undo gratis.

### D2. Nascondere nel filtro comune
`compile_filter` aggiunge `m.id NOT IN (SELECT media_id FROM catalog_item WHERE catalog_id IN (SELECT id FROM catalog WHERE system IS NOT NULL))`, salvo quando `catalog_id` è proprio un catalogo di sistema. Una sola condizione copre tutte le viste della Libreria; la sottoquery usa la chiave primaria di `catalog_item`.

Fuori ambito: Sequenza, Persone e gli export leggono altre query. Le foto nascoste restano visibili lì; da rivedere in una change successiva se serve.

### D3. Protezione e ripristino
`catalog::delete` e `catalog::rename` rifiutano i cataloghi di sistema. "Ripristina" toglie le righe da `catalog_item` (non usa le esclusioni), come nuova azione bulk `remove_from_system` annullabile. "Escludi" non è offerto sui cataloghi di sistema.

### D4. Duplicati
`bulk::collect_duplicates`: per ogni `content_hash` con più file non nascosti, tiene il `min(id)` (prima importazione) e aggiunge gli altri a "Duplicati" con `add_to_catalog` (annullabile come le altre aggiunte).

### D5. Qualità
Job `photo_quality` (pc-ai): per le foto senza riga in `media_quality`, decodifica la miniatura, calcola la nitidezza normalizzata con `quality::laplacian_variance_cropped` (stessa scala di `sharpness`) e legge la risoluzione da `media_tech`. Salva la misura; sposta in "Qualità insufficiente" le foto con nitidezza < 0,12 o meno di 0,3 MP. Le foto già misurate non sono riesaminate, quindi un ripristino dell'utente resta valido.

**Soglie prudenti**: si vuole togliere solo ciò che è chiaramente inutilizzabile; le misure salvate permettono di tararle in futuro.

## Risks

- Foto spostate per errore: tutto è annullabile e ripristinabile, nessun file toccato.
- Prestazioni del filtro: sottoquery su un insieme piccolo e indicizzato.
