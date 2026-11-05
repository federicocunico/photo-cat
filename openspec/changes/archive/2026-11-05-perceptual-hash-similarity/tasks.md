# Tasks

> Esecuzione: ogni task numerato va a un subagente **Sonnet**. I task "Revisione Opus" chiudono una macroparte e vanno a un subagente **Opus**, che rilegge il diff della parte rispetto a spec e design.

## 1. Impronta e schema

- [x] 1.1 Modulo `phash` (D1): `phash(&GrayImage|&RgbImage) -> u64` con DCT 32×32 precalcolata e `hamming(a, b) -> u32`. Test unitari:
  - stessa immagine → 0;
  - ridimensionata a metà + JPEG 70 → ≤ 6;
  - due immagini diverse → > 20;
  - due immagini uniformi di colore diverso: risultato documentato.
- [x] 1.2 Scansione Hamming veloce (D5): `near(hashes: &[u64], query: u64, max: u32, out: &mut Vec<usize>)` con variante `popcnt` scelta a runtime e fallback portabile. Test di equivalenza tra le due varianti. Micro-benchmark in test `#[ignore]`: 200 query su 100.000 hash < 1 s in release.
- [x] 1.3 Migrazione `schema_v11.sql`: `media_item.phash INTEGER NULL`, registrata in `MIGRATIONS`. Test di migrazione da v10 con dati esistenti (colonna nulla, nessuna perdita).

## 2. Calcolo all'import e recupero

- [x] 2.1 Import (D3): `ImportItem.phash`, calcolo in `stage_thumbnail` sia con miniatura generata sia con miniatura in cache, scrittura in `stage_write`. Azzeramento al cambio di `content_hash`. Test in `crates/pc-ingest/tests`: dopo l'import ogni foto ha `phash`; un file ritoccato e riaggiornato ha `phash` diverso.
- [x] 2.2 Job `phash_backfill` (D4) sul modello di `backfill.rs`, con `queue_once` all'avvio in `app/src-tauri/src/lib.rs`. Test: 1.200 foto senza impronta → tutte calcolate; interruzione a metà e ripresa calcola solo le mancanti; il marcatore impedisce un secondo accodamento.
- [x] 2.3 Revisione Opus della parte 1–2: correttezza della DCT, robustezza dei test di stabilità, rami di `stage_thumbnail`, ripresa del backfill.

## 3. Candidati e analisi dei gruppi

- [x] 3.1 `similar::pending` e `facts()` (D6): foto con impronta **o** embedding; `facts` restituisce `phash`. Nuovo `params_key` v2 con `none` senza CLIP. Test di store: una foto con sola impronta è in pending; il cambio di chiave rimette in pending.
- [x] 3.2 Candidati in `similar::run` (D5):
  - globali per Hamming ≤ 10;
  - temporali per Hamming ≤ 22 (stesso dispositivo, oppure stessa cartella se il dispositivo è ignoto, entro ±10 s);
  - gate delle raffiche "CLIP ≥ 0,80 o Hamming ≤ 22";
  - scarto prima della SSIM oltre 40, mai per le copie esatte;
  - al massimo 32 vicini globali per foto.

  Test in `crates/pc-ai/tests` con immagini sintetiche, senza modelli:
  - raffica di 5 → un gruppo;
  - due scatti a 2 s con soggetti diversi → nessun gruppo;
  - copia ridimensionata senza data in un'altra cartella → stesso gruppo;
  - correzioni dell'utente (rimozione, scioglimento, principale manuale) rispettate dopo il ricalcolo.
- [x] 3.3 Job `similar` senza CLIP e accodamento dopo l'import (D7): marcatore `phash_backfill_done` scritto solo al completamento e riaccodamento all'avvio se ci sono foto in pending e nessun job `phash_backfill` attivo o interrotto (D4); il job autonomo non richiede `Role::ClassificationImage`. Al termine di `import` accoda `similar` se ci sono foto in pending e nessun job `similar`/`analysis` attivo. `phash_backfill` accoda `similar` al termine. Test: import senza modelli → gruppi presenti al termine dei job; nessun doppio accodamento.
- [x] 3.4 Prestazioni: test `#[ignore]` su libreria sintetica di 100.000 impronte, dove la ricerca dei candidati per 200 foto nuove resta < 1 s. Annotare i tempi misurati nel commento del test.
- [x] 3.5 UI: etichette i18n it/en del job `phash_backfill` e della fase `impronte` (`lib/jobs.ts`, `keys.test.ts`); verificare che "Foto da raggruppare" e "Avvia analisi dei gruppi" (`similar_pending`, `start_similar_analysis`) funzionino senza modelli. Togliere eventuali testi o guardie UI che richiedono CLIP. Test vitest aggiornati.
- [x] 3.6 Revisione Opus della parte 3: integrazione con union-find e correzioni esistenti, regressioni sui gruppi CLIP, costo del ricalcolo completo.

## 4. Verifica

- [x] 4.1 `cargo test` del workspace, typecheck, lint e test UI, poi `make test-linux` (Docker) con i nuovi file in stage. Correggere ogni errore.
- [x] 4.2 Prova su una libreria reale (o `scripts/loadtest`): avvio con libreria esistente → backfill → gruppi ricalcolati. Annotare tempi e numero di gruppi prima e dopo.

## Workflow follow-up

- Archiviare la change dopo la revisione.
