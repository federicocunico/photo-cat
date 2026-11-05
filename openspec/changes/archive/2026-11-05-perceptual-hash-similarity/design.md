# Design

## Context

Stato attuale (`crates/pc-ai/src/similar/`, `crates/pc-store/src/similar.rs`):

- **Candidati**, in `run()` (`similar/mod.rs`):
  - vicini semantici CLIP (cos ≥ 0,92, HNSW oltre 2.000 foto);
  - raffiche: stesso `media_dates.device`, `instant_utc` entro ±10 s, gate CLIP cos ≥ 0,80;
  - copie esatte per `content_hash`.
- **Conferma**: SSIM su miniatura 512 px ridotta a 128 px. Gli archi ≥ 0,6 vanno in `similar_edge`, così un cambio di soglia rifà solo l'union-find.
- **`similar::pending`** richiede un embedding CLIP, quindi senza modelli niente gruppi.
- **Avvio**: l'analisi gira come ultima fase del job `analysis`, oppure con il job manuale `similar` (`start_similar_analysis`).
- **`params_key`** = `v1;k12;cos0.92;burst0.8@10000;<model>`. Cambiarlo rimette tutto in pending, e le correzioni (`similar_link`, `similar_dissolved`, `cover_manual`) sopravvivono.
- **Import** (`crates/pc-ingest/src/import.rs`): fasi meta → miniature → stadi extra → scrittura. In `stage_thumbnail` la miniatura 512 px (`fit(&oriented, THUMB_SIZE)`) è in memoria. Se la miniatura esiste già e non serve l'immagine decodificata, la funzione esce prima di decodificare.
- **Modello di recupero**: `crates/pc-ingest/src/backfill.rs` (`queue_once` + marcatore in `settings_kv`, blocchi da 500 con checkpoint).
- **Schema** alla versione 10 (`migrate.rs`, `schema_v1..v10.sql`).

## Goals / Non-Goals

**Goals:**
- Gruppi disponibili subito dopo l'import, senza CLIP.
- Costo per foto trascurabile all'import (< 1 ms per l'impronta).
- Ricerca candidati per impronta in tempo lineare con costante minima.

**Non-Goals:**
- Nessuna nuova interfaccia per i gruppi: vista raggruppata, espansione e scelta della principale restano come sono.
- Niente impronte per video o per frame diversi dal primo.
- Niente indice persistente delle impronte su disco: la scansione in memoria basta (vedi decisioni).

## Decisions

### D1. pHash DCT a 64 bit, implementato nel progetto
Passi: luminanza → ridimensionamento 32×32 (filtro Triangle) dalla miniatura 512 px → DCT 2D separabile con matrice dei coseni 32×32 precalcolata → blocco 8×8 in alto a sinistra senza la componente continua → bit = coefficiente > mediana. Tutto in un modulo `pc-ingest/src/phash.rs` (o `pc-core` se serve anche a `pc-ai`), con test.

- *Alternative*: dHash (più rapido ma meno robusto a variazioni di esposizione); crate `image_hasher` (dipendenza in più, API generica e più lenta, versioni di `image` da allineare).
- La DCT 32×32 sono due prodotti matrice 32×32×32 (~65k moltiplicazioni): trascurabile.

### D2. Archiviazione: `media_item.phash INTEGER NULL` (schema v11)
I 64 bit vengono salvati come `i64` reinterpretato (`u64 as i64`). `NULL` = non ancora calcolata. Nessun indice: le letture sono scansioni complete.

- *Alternativa*: tabella separata `media_phash(media_id, algo, hash)` con versione dell'algoritmo. Scartata: una sola variante dell'algoritmo, e la colonna evita un join in `pending`/`facts`. Se l'algoritmo cambiasse, una nuova migrazione azzererebbe la colonna.

### D3. Calcolo all'import e ricalcolo al cambio di contenuto
`ImportItem` riceve `phash: Option<u64>`, calcolato in `stage_thumbnail` **sempre dal JPEG della miniatura in cache**, anche quando la miniatura è appena stata generata. Così import e recupero (D4) producono la stessa impronta per lo stesso file: tra la miniatura in memoria e quella ricodificata si sono misurati fino a 12 bit di differenza su un gradiente. Il costo è una decodifica di un JPEG da 512 px per ogni miniatura generata. I file invariati vengono saltati prima, quindi il ricalcolo riguarda solo i file toccati. La scrittura avviene in `stage_write` accanto a `set_thumb`, con NULL se il calcolo fallisce, così un cambio di `content_hash` non lascia un'impronta vecchia.

### D4. Recupero `phash_backfill` sul modello di `time_backfill`
Job `phash_backfill`, `pending` = foto `status='ok'` con `thumb` e senza `phash`. Blocchi da 500 letti da `ThumbCache`, checkpoint, `inc_done`. L'`UPDATE` è condizionato a `thumb` invariato e `phash IS NULL`, per non sovrascrivere un import concorrente. Le miniature mancanti o illeggibili contano come errori per file non bloccanti.

Accodamento all'avvio (lib.rs, accanto a `time_backfill`): il job viene accodato se ci sono foto in pending, il marcatore `phash_backfill_done` non c'è e nessun job `phash_backfill` è in coda, in corso, in pausa o interrotto. Il marcatore si scrive **solo al completamento** del job, non all'accodamento: un recupero annullato o interrotto riparte alla riapertura. Gli errori per file non causano accodamenti a ogni avvio, perché il marcatore viene scritto anche se alcuni file falliscono. Al termine il job accoda `similar` se ci sono foto in attesa.

### D5. Nuove fonti di candidati in `similar::run`
Costanti, non configurabili dall'utente:
- `HAMMING_NEAR = 10`: candidato globale, indipendente da data e cartella.
- `HAMMING_BURST = 22`: candidato temporale. Stesso dispositivo entro ±10 s (`BURST_WINDOW_MS` attuale) oppure, con dispositivo ignoto, stessa cartella e `instant_utc` entro ±10 s.
- `HAMMING_REJECT = 40`: ogni candidata semantica o temporale con distanza > 40 viene scartata prima della SSIM. Non si applica mai alle copie esatte (`content_hash`). Si è partiti da 32, ma 32 è circa la mediana tra foto non correlate (~32 ± 4) e scarterebbe veri duplicati poco testurizzati (cielo, nebbia: 24–28 bit e rumorosi).
- `HAMMING_NEAR_MAX = 32`: la ricerca globale tiene al massimo i 32 vicini più prossimi per foto. Tutte le immagini uniformi hanno impronta 0 e sarebbero candidate a coppie, con un costo O(k²) in SSIM.

Misure (revisione delle parti 1–2): copie ricompresse o con esposizione +10% a 0–4 bit; scene sintetiche non correlate mai sotto 18 (5° percentile 22, mediana 30).

Il gate CLIP delle raffiche (cos ≥ 0,80) diventa "cos ≥ 0,80 **oppure** Hamming ≤ 22". Senza embedding vale solo l'Hamming.

Ricerca globale: all'inizio del run, `Vec<(u64 hash, MediaId)>` di tutte le foto con impronta (800 KB per 100k). Per ogni foto in pending, scansione lineare `(h ^ x).count_ones() <= HAMMING_NEAR`. La scansione sta in una funzione `#[target_feature(enable = "popcnt")]` scelta a runtime con `is_x86_feature_detected!`, con fallback portabile (aarch64 ha `cnt` nativo). Stima: 200 × 100k = 2·10⁷ confronti, pochi ms. Ricalcolo completo: 100k × 100k / 2 ≈ 5·10⁹, pochi secondi, dominati comunque dalla SSIM.

- *Alternativa*: multi-index hashing a bande. Garantisce il recupero solo fino a distanza = numero di bande − 1, quindi servono ≥ 11 bande da ~6 bit con bucket enormi: più complesso e non più veloce a queste dimensioni.
- *Alternativa*: BK-tree. Scartata per memoria e complessità; la scansione è cache-friendly e vettorizzabile.

### D6. `pending` senza obbligo di CLIP
`similar::pending` richiede `phash IS NOT NULL` **oppure** un embedding (le foto senza impronta ma con CLIP continuano a funzionare). `facts()` restituisce anche `phash`.

`params_key` diventa `v2;k12;cos0.92;burst0.8@10000;h10/22/40;<model|none>`, con `none` senza modello CLIP. Quando CLIP viene installato più tardi, il cambio di chiave rimette in pending le foto, che ricevono i vicini semantici.

### D7. Avvio dopo l'import
Al termine del job `import`, se `similar::pending` > 0 e non è già in coda o in corso un job `similar` o `analysis` (che include la fase dei gruppi), viene accodato un job `similar`. Il job `analysis` continua ad avere la sua fase dei gruppi. Il job `similar` autonomo non richiede più `Role::ClassificationImage`.

## Risks / Trade-offs

- **[Rischio]** Il ricalcolo completo una tantum dopo l'aggiornamento (cambio di `params_key`) rilegge tutte le miniature per la SSIM.
  → Riprendibile, a bassa priorità, gestito dal governor. Le correzioni dell'utente sono preservate dalle tabelle esistenti.
- **[Rischio]** Falsi positivi su immagini uniformi (cielo, pagine bianche): impronte simili tra contenuti diversi.
  → La SSIM resta il giudice finale. Test con due immagini uniformi diverse.
- **[Rischio]** Le foto con poco dettaglio (cielo, nebbia, gradienti) hanno impronte instabili: su ricompressione si sono misurati 24–28 bit. È intrinseco al pHash, perché con pochi coefficienti forti il confronto con la mediana è deciso dal rumore.
  → Il richiamo per impronta di queste foto è basso. Restano le copie esatte, le raffiche (soglia 22) e CLIP. I test dei candidati usano scene testurizzate.
- **[Rischio]** Foto ruotate o specchiate non vengono trovate per impronta.
  → Accettato: le miniature sono già orientate secondo l'EXIF. CLIP, se presente, copre i casi residui.
- **[Trade-off]** Le soglie Hamming sono costanti e non configurabili, per non moltiplicare le impostazioni. La soglia SSIM resta l'unica soglia utente.

## Migration Plan

1. La migrazione v11 aggiunge la colonna (backup automatico già previsto prima di migrare).
2. Al primo avvio, `phash_backfill` calcola le impronte, poi accoda `similar`. Il cambio di `params_key` fa ricalcolare i gruppi.
3. Rollback: una versione precedente ignora la colonna, ma lo schema v11 non è riapribile da versioni precedenti (comportamento già in vigore per ogni migrazione).
