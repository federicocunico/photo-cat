# Proposal

## Why

Raffiche, scatti ripetuti e piccole variazioni della stessa inquadratura riempiono la libreria di foto quasi identiche: rendono lenta la consultazione e la scelta delle foto migliori. Oggi photo-cat riconosce solo i **duplicati esatti** (stessa impronta del contenuto, filtro "solo duplicati"); non c'è un hash percettivo né un concetto di gruppo o raffica. L'utente chiede di raggruppare le foto molto simili, citando SSIM come misura.

## What Changes

- **Gruppi di foto simili**: un'analisi locale raggruppa le foto molto simili usando i dati già disponibili (embedding CLIP, istante di scatto, dispositivo) e una verifica di somiglianza strutturale (SSIM) sulle miniature, che conferma solo le coppie davvero quasi identiche.
- **Foto principale del gruppo**: per ogni gruppo il sistema propone la foto principale (la più nitida e meglio esposta) e l'utente può sceglierne un'altra.
- **Griglia raggruppata**: l'opzione "Raggruppa foto simili" mostra ogni gruppo come una sola miniatura (la principale) con il numero di foto; il gruppo si espande sul posto. È disponibile in Libreria, nei cataloghi e nella Sequenza.
- **Correzioni**: l'utente può togliere foto da un gruppo, unire foto in un gruppo e sciogliere un gruppo; le correzioni sono rispettate dalle analisi successive.
- **Aiuto alla scelta**: filtro "Solo foto con simili", e azione "Seleziona tutte tranne la principale" per escluderle da un catalogo, taggarle o rimuoverle dalla libreria. Nessuna foto viene mai cancellata dal disco.
- **Soglia regolabile** nelle impostazioni AI (quanto devono essere simili), con anteprima del numero di gruppi, come le altre soglie.
- L'analisi è incrementale: le foto nuove vengono confrontate solo con le candidate vicine, non con tutta la libreria, così da funzionare anche con 100.000 foto.

## Capabilities

### New Capabilities

- `similar-photos`: rilevamento dei gruppi di foto quasi identiche, foto principale, correzioni manuali, visualizzazione raggruppata e azioni di scelta.

### Modified Capabilities

- `ai-classification`: "Soglie dell'automatismo configurabili" include la soglia di somiglianza dei gruppi.

## Impact

- **Codice Rust**:
  - nuovo modulo `pc-ai/src/similar.rs`, con indicizzazione dei candidati (HNSW su embedding CLIP, `hnsw_rs` già presente), finestre temporali per dispositivo e verifica SSIM;
  - fase di job `similar` dopo l'analisi CLIP;
  - `pc-store`: nuove tabelle `similar_group`, `similar_member` e `similar_link` (vincoli manuali), migrazione di schema, e filtri e raggruppamento in `query.rs`;
  - metrica di nitidezza dalle miniature, condivisa con `people-faces-workspace`;
  - comandi IPC per correzioni e scelta della principale.
- **UI**: `Grid.tsx` (miniatura di gruppo con numero ed espansione), opzione nella barra della griglia, Inspector (gruppo della foto), filtro nella barra laterale, azione bulk "Seleziona tutte tranne la principale", sezione soglie in Impostazioni, i18n it/en.
- **Dati**: migrazione di schema (numero da coordinare con `catalog-exclusions`).
- **Prestazioni**: calcolo SSIM sulle miniature in scala di grigi ridotte (circa 128 px); nessuna nuova dipendenza obbligatoria (SSIM implementato in Rust, poche decine di righe).
