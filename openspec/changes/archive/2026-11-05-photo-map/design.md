# Design

## Context

- **`location`** (`schema_v1.sql`): `media_id`, `name`, `country`, `lat`, `lon`, `source` (`gps|directory|manual`). Lat/lon esistono solo per `gps`. Le coordinate grezze sono anche in `exif.gps_lat/gps_lon`.
- **`pc_ai::geo`**: gazetteer GeoNames cities15000 (asset opzionale gestito dai modelli), k-d tree e `by_name`. `place_from_path` ricava i luoghi dalle cartelle.
- **Facet dei luoghi**: solo per nome, nessuna coordinata.
- **Policy**: "Esecuzione esclusivamente locale" (spec `ai-classification`). La CSP è `null`, ma le tile remote rivelerebbero le zone visitate: si escludono.
- **UI**: nessuna libreria per mappe. La sezione vive nel modulo AI (change `ai-workspace`).

## Goals / Non-Goals

**Goals:**
- Mappa leggera, offline, senza dipendenze nuove.
- Aggregazione lato Rust, così la UI riceve al massimo qualche centinaio di gruppi.

**Non-Goals:**
- Strade, rilievi, ricerca di indirizzi, livelli di zoom urbani.
- Modificare le coordinate GPS delle singole foto.
- Gestione dell'antimeridiano oltre il taglio a ±180°.

## Decisions

### D1. Mappa di base: confini Natural Earth su canvas
Asset `app/ui/public/geo/countries-50m.json`: TopoJSON semplificato Natural Earth 1:50m, pubblico dominio, < 1 MB, decodificato una volta. Rendering su `<canvas>` con proiezione Web Mercator (lat limitata a ±85°), pan con trascinamento, zoom con rotellina e pulsanti da 1× a 64×. Ridisegno con `requestAnimationFrame`. Lo strato dei gruppi è un secondo canvas, con hit-test sui cerchi.

- *Alternativa*: MapLibre GL con tile vettoriali incluse (MBTiles/PMTiles). Molto più ricco, ma pesa centinaia di MB per uno zoom utile, con dipendenza WebGL e ~800 KB di JS. Sproporzionato per una vista d'insieme.
- *Alternativa*: Leaflet con tile remote OpenStreetMap. Scartata per privacy e funzionamento offline.

### D2. Coordinate dei luoghi: tabella `place_geo`
`place_geo(name PRIMARY KEY, lat, lon, country, source 'gazetteer'|'manual', updated_at)`.

- **Job `place_geo`**: per i nomi distinti di `location` con `source IN ('directory','manual')` e senza riga, prova `Gazetteer::by_name`. Gira all'avvio (se il gazetteer è installato), dopo gli import e dopo una rinomina dei luoghi.
- **Posizione manuale**: sovrascrive con `source='manual'`. Rimuovere una posizione manuale fa tornare all'eventuale risultato del gazetteer.
- **Coordinate di una foto**: GPS se presente (`location.source='gps'` con lat/lon); altrimenti `place_geo` del suo nome (approssimata).

*Alternativa*: scrivere lat/lon direttamente in `location`. Scartata: mescola precise e approssimate, e la posizione manuale di un luogo andrebbe propagata a migliaia di righe.

### D3. Aggregazione a griglia per zoom: `map_cells(filter, bbox, zoom, approx)`
Una cella misura `360 / (2^zoom · 8)` gradi; la latitudine viene convertita in y Mercator prima di dividere, così le celle sono quadrate sullo schermo. SQL:
1. CTE del filtro di ambito (`compile_filter`);
2. coordinate effettive (GPS o `place_geo`);
3. `WHERE` sul bbox;
4. `GROUP BY` cella.

Per ogni cella restituisce conteggio, baricentro medio, foto approssimate, luogo prevalente (`max(count)` per nome) e fino a 4 `media_id` di anteprima. Indice `location_geo(lat, lon)`. Le celle confinanti vengono fuse lato client solo per evitare sovrapposizioni visive.

`map_catalogs(filter, bbox)` conta le foto per catalogo nell'area (esclusi i cataloghi di sistema). `map_places_unplaced(filter)` restituisce i nomi senza `place_geo`, con il conteggio.

- *Alternativa*: supercluster lato client sui punti. Richiede di inviare 100.000 punti alla UI: memoria e trasferimento superiori, ed esclude il filtro SQL.

### D4. `LibraryFilter.geo_bbox`
`geo_bbox: Option<{ south, west, north, east, approx: bool, label: String }>`. Compilato con le stesse coordinate effettive di D3. `label` (luogo prevalente) serve solo per il chip.

### D5. Interazione
- **Debounce**: 150 ms dopo la fine di pan e zoom, poi `map_cells`, con cancellazione delle richieste superate (TanStack Query con chiave arrotondata).
- **Tooltip**: miniature dal servizio miniature esistente.
- **Clic**: `Destination { module: "library", filter: { geo_bbox } }`.
- **"Posiziona"**: modalità puntatore a croce, poi conferma con `map_place_set(name, lat, lon)`. Il menu di un luogo manuale offre "Rimuovi posizione".

## Risks / Trade-offs

- **[Rischio]** `by_name` è ambiguo (più città con lo stesso nome).
  → Si sceglie la più popolosa, come già fa il gazetteer. La posizione resta approssimata e correggibile a mano.
- **[Rischio]** Gazetteer non installato: nessun luogo da cartella viene collocato automaticamente.
  → L'elenco dei luoghi senza coordinate lo indica, con un collegamento a Impostazioni › Modelli.
- **[Trade-off]** Niente dettaglio sotto il livello regionale: per le foto della stessa città basta il clic sul gruppo e la Libreria.

## Migration Plan

Nuova migrazione (numero successivo all'ultima applicata): `place_geo` e indice `location_geo`. Job `place_geo` accodato una volta all'avvio con il marcatore in `settings_kv`. Nessuna conversione di dati esistenti.
