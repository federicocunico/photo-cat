# Proposal

## Why

I gruppi di foto simili oggi dipendono dal modello CLIP: senza modelli installati, o finché l'analisi AI non è terminata, nessuna foto viene raggruppata. Anche gli scatti a raffica, i casi più evidenti, devono aspettare l'analisi semantica. Con librerie oltre le 10.000 foto serve un segnale economico, calcolato all'import e confrontabile in pochi istanti. Il candidato naturale è un'impronta percettiva (perceptual hash) confrontata con la distanza di Hamming.

## What Changes

- **Impronta percettiva all'import**: per ogni foto viene calcolata un'impronta di 64 bit dalla miniatura già in memoria. Il calcolo non usa modelli AI né accessi aggiuntivi agli originali.
- **Retrocompatibilità**: un'attività una tantum calcola l'impronta per le foto già importate, leggendo le miniature in cache. Le foto senza miniatura ricevono l'impronta quando la miniatura viene generata.
- **Nuove fonti di candidati per i gruppi**, accanto a vicini semantici CLIP e copie esatte:
  - **impronte vicine**: distanza di Hamming bassa (XOR + conteggio dei bit), indipendentemente dalla data;
  - **vicinanza temporale**: scatti a pochi secondi l'uno dall'altro dello stesso dispositivo (o della stessa cartella, se il dispositivo è ignoto) con impronta compatibile, anche senza CLIP.
- La conferma resta strutturale (SSIM sulle miniature). L'impronta scarta subito le coppie palesemente diverse e riduce i confronti SSIM.
- **Gruppi senza modelli AI**: l'analisi dei gruppi parte dopo ogni import anche se i modelli non sono installati. CLIP, quando c'è, aggiunge candidati ma non è più necessario.
- I gruppi vengono ricalcolati una volta con i nuovi segnali. Le correzioni dell'utente restano valide: foto principale scelta, foto tolte, gruppi uniti o sciolti.

La visualizzazione raggruppata, l'espansione e la scelta della principale esistono già e non cambiano.

## Capabilities

### New Capabilities

Nessuna.

### Modified Capabilities

- `similar-photos`: il rilevamento non richiede più CLIP e usa impronta percettiva e vicinanza temporale. L'analisi parte dopo l'import. Nuovi requisiti per il calcolo dell'impronta (import e foto già presenti) e per la velocità del confronto.

## Impact

- **Rust**:
  - schema v11: colonna `media_item.phash`;
  - calcolo dell'impronta in `pc-ingest` (fase miniature dell'import) e nuovo job di recupero `phash_backfill`;
  - in `pc-ai/src/similar`: generatore di candidati per Hamming e per tempo, `pending` senza obbligo di embedding, `params_key` aggiornato;
  - accodamento del job `similar` al termine dell'import.
- **UI**: nessuna nuova schermata. Il conteggio "foto da raggruppare" e l'avvio manuale funzionano anche senza modelli.
- **Dipendenze**: nessuna nuova crate. DCT e Hamming sono implementati nel progetto.
- **Prestazioni**: un ricalcolo completo una tantum dei gruppi dopo l'aggiornamento (solo SSIM sui candidati, nessuna inferenza).
