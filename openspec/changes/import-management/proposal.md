# Proposal

## Why

Le issue #7 ("Eliminare importazione") e #8 ("Aggiornare il contenuto correlato ad una importazione") chiedono di poter eliminare un'importazione fatta per errore e di aggiornarla dopo aver modificato il contenuto delle cartelle. Il sistema ha già "Rimuovi" e "Riscansiona" per ogni sorgente, ma l'utente non li ha trovati o non gli sono bastati. Dal codice di `main` risulta che:

- le due azioni esistono solo nella pagina Import, e "Riscansiona" non richiama l'idea di "aggiornare";
- la rimozione cancella le righe dal catalogo ma lascia su disco le miniature e non ferma un import o un'analisi in corso sulla stessa sorgente;
- l'aggiornamento lavora solo sull'intera sorgente e non dice cosa è cambiato (nuovi, modificati, spostati, mancanti);
- i file scomparsi restano "mancanti" per sempre: la spec `catalog-library` prevede di poterli ricollegare o rimuovere, ma non esiste un comando per ricollegarli né un'azione per rimuoverli tutti insieme.

## What Changes

- **Importazioni raggiungibili dalla Libreria**: ogni sorgente nella barra laterale ha un menu con "Aggiorna", "Aggiorna cartella…" (sottocartella), "Rimuovi dalla libreria…" e "Apri in Import". Nella pagina Import "Riscansiona" diventa "Aggiorna".
- **Rimozione completa e sicura**: conferma con riepilogo (foto, foto con correzioni manuali, cataloghi manuali e persone coinvolte). Prima della rimozione si annullano i job attivi sulla sorgente. Poi si cancellano anche miniature e dati derivati (embedding, volti, ritagli). Le persone che restano senza volti vengono rimosse se non hanno nome. I file sul disco non vengono mai toccati.
- **Aggiornamento con riepilogo**: al termine l'utente vede quanti file sono nuovi, modificati, spostati o rinominati (riconosciuti dall'impronta del contenuto), mancanti o tornati presenti, e accede direttamente a ciascun gruppo nella libreria.
- **Aggiornamento di una cartella**: si può aggiornare solo una sottocartella della sorgente. I file mancanti vengono marcati solo dentro quella cartella.
- **Gestione dei mancanti**: dopo un aggiornamento con file mancanti l'utente può:
  - "Ricollegare" i file indicando la nuova posizione di una cartella: il sistema li ritrova per percorso relativo o per impronta del contenuto e conserva tutte le catalogazioni;
  - rimuoverli dalla libreria con conferma;
  - lasciarli come sono.
- **File modificati rianalizzati**: un file che ha cambiato contenuto (non solo data) viene rianalizzato. Le correzioni manuali, i tag e i cataloghi restano.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `media-ingestion`: "Sorgenti di import" (rimozione completa, sicura e accessibile dalla Libreria) e "Scansione incrementale" (riepilogo, aggiornamento di una cartella, rianalisi dei file modificati); nuovi requisiti "Riepilogo dell'aggiornamento" e "Aggiornamento di una cartella".
- `catalog-library`: "Stato dei file sul disco" (ricollegamento per cartella e rimozione dei mancanti in blocco, con comandi concreti).

## Impact

- **Codice Rust**:
  - `pc-store/src/repo.rs`: `source::delete` con riepilogo e raccolta dei percorsi delle miniature; `media::delete` che pulisce i dati derivati; ricollegamento per prefisso o per impronta;
  - `pc-ingest/src/import.rs`: parametro `folder`, `mark_missing` limitato all'ambito, riepilogo con categorie, rianalisi del contenuto modificato;
  - `pc-jobs`: annullamento dei job per sorgente;
  - `app/src-tauri/src/library.rs`: comandi `source_removal_summary`, `remove_source` rivisto, `relink_missing`, `remove_missing`.
- **UI**: `Sidebar.tsx` (menu della sorgente), `ImportPage.tsx` ("Aggiorna", riepilogo dell'aggiornamento, rimozione con riepilogo al posto di `window.confirm`), schermata Job (riepilogo dell'import con collegamenti alla libreria), i18n it/en.
- **Dati**: nessuna migrazione di schema; le miniature orfane già presenti vengono ripulite una volta all'avvio successivo.
- **Coordinamento**: `manual-edits-open-with-ai-tuning` aggiunge un albero delle cartelle sotto le sorgenti in `Sidebar.tsx`; "Aggiorna cartella" va esposto anche da lì quando entrambi sono uniti. `facet-lists-expand` tocca la stessa `Sidebar.tsx`.
