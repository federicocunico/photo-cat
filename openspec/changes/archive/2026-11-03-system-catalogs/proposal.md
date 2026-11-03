# Proposal

## Why

Tre richieste degli utenti chiedono di togliere foto dalla vista principale senza cancellarle dal disco:

- **#24 Eliminare una fotografia**: una cancellazione "logica", con le foto spostate in un catalogo "Cestino" di sistema, non eliminabile.
- **#29 Eliminare i duplicati**: le copie identiche messe in un catalogo "Duplicati" non eliminabile, e tolte dalla finestra principale.
- **#20 Foto molto sgranate o sfocate**: spostate automaticamente in un catalogo "Qualità insufficiente".

Oggi l'unico modo per togliere una foto dalla vista è "Rimuovi dalla libreria", che cancella anche tag, cataloghi e volti e non si può annullare.

## What Changes

- **Cataloghi di sistema**: "Cestino", "Duplicati" e "Qualità insufficiente", creati automaticamente, non eliminabili né rinominabili. Sono cataloghi manuali con un ruolo di sistema.
- Le foto in un catalogo di sistema **non compaiono** in Libreria, nei cataloghi, nei filtri e nei conteggi, salvo quando si apre proprio quel catalogo. I file sul disco non vengono toccati.
- **Ripristina**: dal catalogo di sistema aperto, le foto selezionate tornano nella libreria (annullabile).
- **Sposta nel cestino** sulla selezione (barra delle azioni, Canc fuori da un catalogo), annullabile.
- **Metti da parte i duplicati**: per ogni gruppo di file con lo stesso contenuto resta in libreria la prima copia importata; le altre vanno in "Duplicati" (annullabile).
- **Controlla la qualità**: un'attività misura sulle miniature la nitidezza (varianza del laplaciano) e la risoluzione, e sposta in "Qualità insufficiente" le foto molto sfocate o sotto 0,3 megapixel.

Nessun file sul disco viene spostato o cancellato.

## Capabilities

### Modified Capabilities

- `catalog-library`: nuovi requisiti "Cataloghi di sistema", "Cestino", "Duplicati messi da parte", "Foto di qualità insufficiente"; "Canc fuori da un catalogo" sposta nel cestino.

## Impact

- **Rust**: schema v10 (`catalog.system`, `media_quality`), filtro comune (`compile_filter`), protezione di eliminazione e rinomina, comandi `collect_duplicates` e job `photo_quality`, ripristino da un catalogo di sistema.
- **UI**: cataloghi di sistema nella barra laterale e in Cataloghi (senza Elimina/Rinomina), "Sposta nel cestino", "Ripristina", "Metti da parte i duplicati", "Controlla la qualità".
- **Test**: Rust (filtro, migrazione, duplicati, qualità) e vitest (barra delle azioni, Canc).
