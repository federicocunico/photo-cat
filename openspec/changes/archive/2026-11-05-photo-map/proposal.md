# Proposal

## Why

La libreria sa già dove sono state scattate molte foto: dalle coordinate GPS, con il nome della città da un dizionario geografico offline, o dal nome della cartella ("Sardegna", "Viaggio Giappone"). Questi luoghi però si vedono solo come un elenco di nomi nei filtri. Una mappa del mondo permette di vedere a colpo d'occhio dove sono le foto e i cataloghi e di aprire le foto di una zona. Fa anche emergere i luoghi che non è stato possibile collocare.

## What Changes

- **Sezione "Mappa" nel modulo AI**: mappa del mondo **offline**, con confini dei paesi inclusi nell'app e nessuna richiesta di rete. Si sposta e si ingrandisce fino al livello regionale.
- **Foto raggruppate per zona**: cerchi proporzionali al numero di foto, ricalcolati a ogni livello di zoom. Al passaggio del puntatore compare l'anteprima con alcune miniature; con un clic si apre la Libreria filtrata su quella zona.
- **Due precisioni**:
  - punti precisi dalle coordinate GPS;
  - punti approssimati per i luoghi ricavati da cartelle o assegnati a mano, collocati con il dizionario geografico offline quando il nome corrisponde a una città.

  I punti approssimati sono distinguibili e si possono nascondere.
- **Ambito**: tutta la libreria o un catalogo. Un elenco laterale mostra i cataloghi presenti nell'area visibile, con il numero di foto.
- **Luoghi senza coordinate**: elenco dei nomi di luogo non collocabili, con il numero di foto. L'utente può posizionarli con un clic sulla mappa, e la posizione vale per tutte le foto con quel luogo.
- **Filtro della Libreria per area geografica**, usato dalla mappa.

## Capabilities

### New Capabilities

- `photo-map`: mappa offline delle foto per posizione, con raggruppamento per zona, ambito per catalogo, luoghi approssimati e collocazione manuale dei luoghi.

### Modified Capabilities

- `catalog-library`: nuovo requisito "Filtro per area geografica".

## Impact

- **Rust**:
  - schema: tabella `place_geo` (nome del luogo → coordinate, fonte `gazetteer`/`manual`) e indice su `location(lat, lon)`;
  - geocodifica dei nomi dei luoghi con `Gazetteer::by_name` (job leggero, rieseguito quando cambiano i luoghi);
  - query di aggregazione per celle e comandi Tauri `map_cells`, `map_places_unplaced`, `map_place_set`;
  - `LibraryFilter.geo_bbox`.
- **UI**: sezione Mappa in `app/ui/src/modules/ai/`, rendering su canvas con proiezione Web Mercator, asset dei confini (Natural Earth 1:50m, pubblico dominio) in `app/ui/public/`, i18n.
- **Dipendenze tra change**: richiede `ai-workspace` (il modulo AI che ospita la sezione).
- **Privacy**: nessuna coordinata lascia il computer, coerentemente con "Esecuzione esclusivamente locale".
