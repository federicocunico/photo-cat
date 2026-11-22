# Proposal

## Why

Le issue aperte su GitHub (#39–#47) raccolgono le richieste di chi usa photo-cat ogni giorno per riordinare un archivio di famiglia, con molte foto vecchie scansionate. I problemi sono di due tipi:

- **mancano strumenti di catalogazione**: data reale delle scansioni, importanza, aspetto del colore;
- **attriti nei flussi esistenti**: selezione, passaggio dal volto alla foto, avvisi di import che non si chiudono mai, una casella "Monitora" che non si capisce.

Questa change le chiude tutte insieme.

## What Changes

- **Seleziona tutto / Inverti selezione ovunque (#39)**: in ogni vista con una griglia a selezione multipla (Libreria, cataloghi, cataloghi di sistema, volti in Persone, Sequenza, risultati e simili del modulo AI). Disponibili dalla barra, dal menu contestuale e da tastiera (Ctrl+A, Ctrl+Maiusc+I).
- **Dal volto alla foto (#40, #42)**: clic destro su un volto in Persone con queste voci:
  - "Apri la foto" (visore a schermo intero);
  - "Mostra in Libreria", con la foto selezionata e in vista;
  - "Apri con ▸" (editor esterni configurati);
  - "Mostra nella cartella".

  Il doppio clic su un volto apre la foto.
- **Modifica dei dati (#41)**:
  - **valutazione 0–5 stelle**: Alt+1–5 e Alt+0 (i tasti 1–9 e 0 restano ai tag del set attivo), Inspector, menu contestuale, bulk, filtro e campo delle regole smart;
  - **data di scatto impostata a mano**, singola o in bulk, con precisione giorno, mese o solo anno, per negativi e stampe scansionate. Prevale sulla data letta dal file, è annullabile e non tocca gli originali. L'"anno di riferimento" è questa data con precisione "anno".
- **Caratteristica di colore (#43)**: valore impostato **solo a mano**, da menu contestuale su una o più foto o dall'Inspector. Valori: "Bianco e nero", "Colore", "Colore virato", "Seppia". Disponibile come filtro e campo delle regole smart. Nessuna analisi automatica e nessuna modifica ai pixel.
- **Avvisi dell'import (#44, #45)**:
  - la pagina Import mostra solo l'esito dell'**ultimo** aggiornamento di ogni cartella; gli avvisi delle esecuzioni precedenti spariscono dalla pagina ma restano nel registro errori e nel log;
  - ogni avviso (file mancanti, file con errori) si può chiudere con "Ignora": resta nascosto finché un aggiornamento successivo non trova novità;
  - in Diagnostica gli errori già visti si possono "archiviare", così l'indicatore globale non li conta più.
- **"Monitora" comprensibile (#46)**: etichetta esplicita "Aggiorna automaticamente" con spiegazione al passaggio del puntatore e nella guida. Sulla cartella è indicato lo stato del monitoraggio: attivo, ultimo aggiornamento automatico, oppure non disponibile.
- **Persone ordinate per anno (#47)**: i volti di una persona si possono ordinare per data di scatto (dal più vecchio o dal più recente), con intestazioni per anno. L'ordinamento attuale per qualità resta disponibile e la scelta viene ricordata.

## Capabilities

### New Capabilities

Nessuna.

### Modified Capabilities

- `bulk-operations`: "Selezione multipla" estesa a tutte le viste con griglia; "Azioni bulk disponibili" con valutazione, data di scatto e caratteristica di colore.
- `catalog-library`: nuovi requisiti "Valutazione delle foto" e "Caratteristica di colore"; "Ricerca e filtri" con i nuovi campi.
- `capture-timeline`: nuovo requisito "Data di scatto impostata a mano" (precisione giorno, mese o anno).
- `face-recognition`: nuovi requisiti "Dal volto alla foto" e "Ordinamento dei volti per data".
- `media-ingestion`: "Riepilogo dell'aggiornamento" limitato all'ultima esecuzione; nuovo requisito "Avvisi delle cartelle chiudibili"; "Monitoraggio delle sorgenti" spiegato e con stato visibile.
- `diagnostics-logging`: nuovo requisito "Errori archiviati".

## Impact

- **Schema** (nuova migrazione):
  - colonne `rating` (0–5) e `color_trait` su `media_item`, oppure tabella dedicata;
  - in `media_dates`, la data manuale con precisione: `manual_captured_at`, `manual_precision` e la fonte `manual` in `DateSource`;
  - `acknowledged_at` sugli avvisi di aggiornamento delle sorgenti;
  - `archived_at` nel registro errori.
- **Rust**:
  - `pc-store`: bulk, filtri, regole smart, query volti con ordinamento per data, sorgenti ed errorlog;
  - comandi Tauri e binding `specta`;
  - storico annullabile (`bulk-operations` "Annullamento delle modifiche ai metadati").
- **UI**:
  - modulo `selection` condiviso tra le griglie;
  - `FaceGrid` (menu contestuale, ordinamento);
  - Inspector, `GridContextMenu`, `ImportPage`, Diagnostica, editor delle regole;
  - i18n it/en e glossario.
- **Compatibilità**: le librerie esistenti partono con valutazione 0, nessun colore e nessuna data manuale. Nessun file originale viene modificato.
